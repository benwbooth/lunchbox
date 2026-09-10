//! Selected-device relative mouse capture. This is not a gamepad axis transform
//! or an emulator routing policy. Deltas are physical counts, never positions.
pub use super::relative_settings::RelativeMotionCalibration;
use super::{GamepadReader, input_bits, open_identified_event, read_gamepad_event};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One complete kernel report. Button entries are complete selected state;
/// delta entries are only movement since the preceding SYN_REPORT.
#[derive(Clone, Debug)]
pub struct RelativePacket {
    pub deltas: BTreeMap<u16, i64>,
    pub buttons: BTreeMap<u16, bool>,
}

struct RelativeMotionTransform {
    calibration: RelativeMotionCalibration,
    remainder: [i128; 2],
}

impl RelativeMotionTransform {
    fn apply(&mut self, packet: &mut RelativePacket) -> Result<()> {
        let mut remainder = self.remainder;
        let mut deltas = packet.deltas.clone();
        if self.calibration.swap_xy {
            let x = deltas.remove(&0);
            let y = deltas.remove(&1);
            if let Some(x) = x {
                deltas.insert(1, x);
            }
            if let Some(y) = y {
                deltas.insert(0, y);
            }
        }
        for (axis, percent, inverted) in [
            (0u16, self.calibration.x_percent, self.calibration.invert_x),
            (1u16, self.calibration.y_percent, self.calibration.invert_y),
        ] {
            let Some(delta) = deltas.get_mut(&axis) else {
                continue;
            };
            let numerator =
                i128::from(*delta) * i128::from(percent) * if inverted { -1 } else { 1 }
                    + remainder[axis as usize];
            *delta = i64::try_from(numerator / 100)
                .map_err(|_| anyhow::anyhow!("Calibrated relative movement exceeds count range"))?;
            remainder[axis as usize] = numerator % 100;
        }
        // Carry signed fractional counts between complete reports. Wheel
        // detents and button transitions pass through without scaling.
        packet.deltas = deltas;
        self.remainder = remainder;
        Ok(())
    }
}

/// A nonblocking source bound to the opened evdev identity. Non-grabbing by
/// default; a saved session may explicitly request exclusive source capture.
/// REL_X/REL_Y, conventional REL_HWHEEL/REL_WHEEL detents and BTN_MOUSE
/// buttons can be selected. High-resolution wheel units, keyboard, touch and
/// absolute axes need separate contracts; they are not folded into detents.
pub struct RelativeReader {
    file: Option<std::fs::File>,
    axes: BTreeSet<u16>,
    buttons: BTreeMap<u16, bool>,
    pending: BTreeMap<u16, i64>,
    initializing: bool,
    failed: bool,
}

impl RelativeReader {
    pub fn open(
        path: &Path,
        expected_input_identity: &Path,
        axes: impl IntoIterator<Item = u16>,
        buttons: impl IntoIterator<Item = u16>,
    ) -> Result<Self> {
        let mut selected_axes = BTreeSet::new();
        for code in axes {
            ensure!(
                matches!(code, 0 | 1 | 6 | 8),
                "Only relative X/Y motion and conventional horizontal/vertical wheel detents are supported"
            );
            ensure!(selected_axes.insert(code), "Duplicate relative axis");
        }
        let mut selected_buttons = BTreeMap::new();
        for code in buttons {
            ensure!(
                (0x110..=0x117).contains(&code),
                "Only mouse buttons may be captured"
            );
            ensure!(
                selected_buttons.insert(code, false).is_none(),
                "Duplicate mouse button"
            );
        }
        ensure!(
            !selected_axes.is_empty() || !selected_buttons.is_empty(),
            "No relative mouse controls selected"
        );
        let file = open_identified_event(path, expected_input_identity)?;
        let supported_axes = input_bits(&file, 0x22)?; // EVIOCGBIT(EV_REL)
        let supported_buttons = input_bits(&file, 0x21)?; // EVIOCGBIT(EV_KEY)
        for code in &selected_axes {
            ensure!(
                GamepadReader::bit(&supported_axes, *code),
                "Selected relative axis is absent on this device"
            );
        }
        for code in selected_buttons.keys() {
            ensure!(
                GamepadReader::bit(&supported_buttons, *code),
                "Selected mouse button is absent on this device"
            );
        }
        Ok(Self {
            file: Some(file),
            axes: selected_axes,
            buttons: selected_buttons,
            pending: BTreeMap::new(),
            initializing: true,
            failed: false,
        })
    }

    /// Grab this exact open source, never a device rediscovered by name. The
    /// entire event node is grabbed, including controls not selected for output.
    /// The caller must have obtained explicit opt-in and retain another way to
    /// stop the session. Closing our sole descriptor releases the kernel grab.
    fn grab_exclusive(&mut self) -> Result<()> {
        use std::os::fd::AsRawFd;
        ensure!(!self.failed, "Cannot grab a stopped relative source");
        let file = self
            .file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Relative source is closed"))?;
        // Linux UAPI EVIOCGRAB = _IOW('E', 0x90, int); the argument is a
        // nonzero scalar, not a pointer to an integer (evdev_do_ioctl).
        ensure!(
            unsafe {
                libc::ioctl(
                    file.as_raw_fd(),
                    0x40044590 as libc::c_ulong,
                    1 as libc::c_int,
                )
            } == 0,
            "Exclusive relative source capture failed: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }

    /// Permanently stop this source and produce button releases. A transport
    /// owner must publish these releases on any error and before teardown.
    pub fn neutral(&mut self) -> RelativePacket {
        self.failed = true;
        // Also releases any EVIOCGRAB immediately, even when the stopped
        // session object remains owned by a launch worker.
        self.file.take();
        self.pending.clear();
        self.buttons.values_mut().for_each(|held| *held = false);
        RelativePacket {
            deltas: BTreeMap::new(),
            buttons: self.buttons.clone(),
        }
    }

    /// Bounded drain, preserving individual kernel packet order. The caller
    /// owns accumulation between frontend polls; never retain a delta as a
    /// held axis value. Initialization discards backlog before key snapshot.
    /// SYN_DROPPED fails the session: relative movement cannot be recovered
    /// from a kernel state snapshot. No partial batch escapes on failure.
    pub fn poll(&mut self) -> Result<Vec<RelativePacket>> {
        ensure!(!self.failed, "Relative input failed; open a new session");
        let result = self.poll_inner();
        if result.is_err() {
            self.neutral();
        }
        result
    }

    fn poll_inner(&mut self) -> Result<Vec<RelativePacket>> {
        let mut packets = Vec::new();
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Relative source is closed"))?;
        for _ in 0..256 {
            let Some(event) = read_gamepad_event(file)? else {
                if self.initializing {
                    let keys = input_bits(file, 0x18)?; // EVIOCGKEY
                    for (code, held) in &mut self.buttons {
                        *held = GamepadReader::bit(&keys, *code);
                    }
                    self.initializing = false;
                    packets.push(RelativePacket {
                        deltas: BTreeMap::new(),
                        buttons: self.buttons.clone(),
                    });
                }
                break;
            };
            ensure!(
                !(event.kind == 0 && event.code == 3),
                "Relative input synchronization lost; start a new session"
            );
            if self.initializing {
                continue;
            }
            match (event.kind, event.code) {
                (2, code) if self.axes.contains(&code) => {
                    let delta = self.pending.entry(code).or_default();
                    *delta = delta.checked_add(i64::from(event.value)).ok_or_else(|| {
                        anyhow::anyhow!("Relative input delta accumulation overflow")
                    })?;
                }
                (1, code) if self.buttons.contains_key(&code) => {
                    ensure!((0..=2).contains(&event.value), "Invalid mouse button value");
                    self.buttons.insert(code, event.value != 0);
                }
                (0, 0) => packets.push(RelativePacket {
                    deltas: std::mem::take(&mut self.pending),
                    buttons: self.buttons.clone(),
                }),
                _ => {}
            }
        }
        Ok(packets)
    }
}

/// Launch-owned virtual mouse. Creation is explicit and requires existing
/// /dev/uinput access. The kernel identity is obtained from the owned descriptor,
/// not from a display-name search or an assumed RetroArch mouse index.
pub struct VirtualRelativeMouse {
    file: std::fs::File,
    axes: BTreeSet<u16>,
    buttons: BTreeSet<u16>,
    system_name: String,
    created: bool,
}

/// Exact owned endpoint, not a frontend mouse index. A caller must revalidate
/// identity when opening it and must not infer RetroArch order from eventN.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelativeMouseEndpoint {
    pub event_node: std::path::PathBuf,
    pub input_identity: std::path::PathBuf,
}

impl VirtualRelativeMouse {
    pub fn create(reader: &RelativeReader) -> Result<Self> {
        Self::create_configured(reader, false, reader.buttons.keys().copied().collect())
    }

    fn create_configured(
        reader: &RelativeReader,
        swap_xy: bool,
        buttons: BTreeSet<u16>,
    ) -> Result<Self> {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_MOUSE: AtomicU64 = AtomicU64::new(0);
        ensure!(
            !reader.failed,
            "Cannot publish a failed relative input session"
        );
        let file = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open("/dev/uinput")
            .map_err(|error| anyhow::anyhow!(
                "Relative mouse output requires writable /dev/uinput: {error}. No permissions were changed."
            ))?;
        let mut mouse = Self {
            file,
            axes: reader
                .axes
                .iter()
                .map(|axis| {
                    if swap_xy && *axis <= 1 {
                        1 - *axis
                    } else {
                        *axis
                    }
                })
                .collect(),
            buttons,
            system_name: String::new(),
            created: false,
        };
        mouse.capability(0x40045564, 1)?; // UI_SET_EVBIT(EV_KEY)
        mouse.capability(0x40045564, 2)?; // UI_SET_EVBIT(EV_REL)
        // Standard mouse classification requires X/Y and BTN_LEFT even when
        // the selected source only supplies a subset. Unselected capabilities
        // stay neutral; they are not admitted by publish().
        mouse.capability(0x40045566, 0)?; // UI_SET_RELBIT(REL_X)
        mouse.capability(0x40045566, 1)?; // UI_SET_RELBIT(REL_Y)
        for code in &mouse.axes {
            mouse.capability(0x40045566, i32::from(*code))?;
        }
        mouse.capability(0x40045565, 0x110)?; // UI_SET_KEYBIT(BTN_LEFT)
        for code in &mouse.buttons {
            mouse.capability(0x40045565, i32::from(*code))?;
        }
        let name = format!(
            "Lunchbox session mouse {}-{}",
            std::process::id(),
            NEXT_MOUSE.fetch_add(1, Ordering::Relaxed)
        );
        ensure!(name.len() < 80, "Virtual mouse name exceeds kernel limit");
        let mut setup = [0u8; 92]; // uinput_setup
        setup[..2].copy_from_slice(&0x06u16.to_ne_bytes()); // BUS_VIRTUAL
        setup[8..8 + name.len()].copy_from_slice(name.as_bytes());
        ensure!(
            unsafe {
                libc::ioctl(
                    mouse.file.as_raw_fd(),
                    0x405c5503 as libc::c_ulong,
                    setup.as_ptr(),
                )
            } >= 0,
            "Cannot configure virtual mouse: {}",
            std::io::Error::last_os_error()
        );
        ensure!(
            unsafe { libc::ioctl(mouse.file.as_raw_fd(), 0x5501 as libc::c_ulong) } >= 0,
            "Cannot create virtual mouse: {}",
            std::io::Error::last_os_error()
        );
        mouse.created = true;
        let mut identity = [0u8; 128];
        let request = 0x8000_0000u32 | (128 << 16) | (0x55 << 8) | 44; // UI_GET_SYSNAME
        ensure!(
            unsafe {
                libc::ioctl(
                    mouse.file.as_raw_fd(),
                    request as libc::c_ulong,
                    identity.as_mut_ptr(),
                )
            } >= 0,
            "Cannot identify virtual mouse: {}",
            std::io::Error::last_os_error()
        );
        let end = identity
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| anyhow::anyhow!("Unterminated virtual mouse identity"))?;
        let identity = std::str::from_utf8(&identity[..end])?;
        ensure!(
            identity.strip_prefix("input").is_some_and(
                |suffix| !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
            ),
            "Invalid virtual mouse identity"
        );
        mouse.system_name = identity.to_owned();
        mouse.publish(&mouse.neutral_packet())?;
        Ok(mouse)
    }

    fn capability(&self, request: u32, value: i32) -> Result<()> {
        use std::os::fd::AsRawFd;
        ensure!(
            unsafe {
                libc::ioctl(
                    self.file.as_raw_fd(),
                    request as libc::c_ulong,
                    value as libc::c_int,
                )
            } >= 0,
            "Cannot declare virtual mouse capability: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }

    pub fn system_name(&self) -> &str {
        &self.system_name
    }

    /// Resolve the owned inputN to its evdev child. None means sysfs/devtmpfs
    /// publication is not ready yet; no waiting, name guessing or fallback to
    /// another mouse occurs. A launch owner can retry within its own deadline.
    pub fn endpoint(&self) -> Result<Option<RelativeMouseEndpoint>> {
        use std::os::unix::fs::FileTypeExt;
        ensure!(self.created, "Virtual mouse is no longer available");
        let input_path = Path::new("/sys/devices/virtual/input").join(&self.system_name);
        let input_identity = match input_path.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut event_name = None;
        for entry in std::fs::read_dir(&input_identity)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !name.strip_prefix("event").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
            }) {
                continue;
            }
            ensure!(
                event_name.is_none(),
                "Owned virtual mouse has multiple evdev endpoints"
            );
            event_name = Some(name.to_owned());
        }
        let Some(event_name) = event_name else {
            return Ok(None);
        };
        let event_node = Path::new("/dev/input").join(&event_name);
        let metadata = match std::fs::symlink_metadata(&event_node) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        ensure!(
            metadata.file_type().is_char_device(),
            "Virtual mouse endpoint is not an evdev node"
        );
        // Opening verifies the actual fd's dev_t/sysfs identity and checks that
        // the path still names the same node. Do not rely on child name alone.
        let _verified = open_identified_event(&event_node, &input_identity)?;
        Ok(Some(RelativeMouseEndpoint {
            event_node,
            input_identity,
        }))
    }

    fn neutral_packet(&self) -> RelativePacket {
        RelativePacket {
            deltas: BTreeMap::new(),
            buttons: self.buttons.iter().map(|code| (*code, false)).collect(),
        }
    }

    /// Validate the complete packet before writing, without clipping counts or
    /// reinterpreting them as absolute positions. Errors destroy this output.
    pub fn publish(&mut self, packet: &RelativePacket) -> Result<()> {
        ensure!(self.created, "Virtual mouse is no longer available");
        let result = self.publish_inner(packet);
        if result.is_err() {
            self.shutdown();
        }
        result
    }

    fn publish_inner(&mut self, packet: &RelativePacket) -> Result<()> {
        use std::io::Write;
        ensure!(
            packet
                .buttons
                .keys()
                .copied()
                .eq(self.buttons.iter().copied()),
            "Relative packet must contain exactly the selected mouse buttons"
        );
        let mut events = Vec::with_capacity(packet.deltas.len() + packet.buttons.len() + 2);
        for (code, delta) in &packet.deltas {
            ensure!(self.axes.contains(code), "Undeclared relative mouse axis");
            let delta = i32::try_from(*delta)
                .map_err(|_| anyhow::anyhow!("Relative mouse packet exceeds kernel delta range"))?;
            if delta != 0 {
                if matches!(*code, 6 | 8) {
                    // The pinned RetroArch udev backend only recognizes
                    // conventional wheel values +1/-1, ignoring larger
                    // totals. Preserve detent count in the emitted stream;
                    // frontend polling may still coalesce its boolean flags.
                    let detents = delta.unsigned_abs() as usize;
                    ensure!(
                        detents <= 4096 && events.len() + detents * 2 <= 8192,
                        "Relative wheel report exceeds bounded detent publication"
                    );
                    for _ in 0..detents {
                        events.push((2u16, *code, delta.signum()));
                        events.push((0u16, 0u16, 0)); // one detent per SYN_REPORT
                    }
                } else {
                    events.push((2u16, *code, delta));
                }
            }
        }
        for (code, held) in &packet.buttons {
            events.push((1u16, *code, i32::from(*held)));
        }
        if !self.buttons.contains(&0x110) {
            events.push((1u16, 0x110, 0));
        }
        events.push((0u16, 0u16, 0)); // SYN_REPORT
        let mut bytes = Vec::with_capacity(events.len() * 24);
        for (kind, code, value) in events {
            bytes.extend_from_slice(&[0u8; 16]); // Linux 64-bit timeval
            bytes.extend_from_slice(&kind.to_ne_bytes());
            bytes.extend_from_slice(&code.to_ne_bytes());
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        self.file.write_all(&bytes)?;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        use std::os::fd::AsRawFd;
        if self.created {
            let _ = self.publish_inner(&self.neutral_packet());
            unsafe {
                libc::ioctl(self.file.as_raw_fd(), 0x5502 as libc::c_ulong);
            }
            self.created = false;
        }
    }
}

impl Drop for VirtualRelativeMouse {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Explicitly owned capture-to-output session. This does not choose a frontend
/// port, run a worker, or suppress the desktop's physical/virtual pointer input.
/// A launch integration must resolve that policy before starting publication.
pub struct RelativeMouseSession {
    reader: RelativeReader,
    output: VirtualRelativeMouse,
    motion: RelativeMotionTransform,
    button_map: BTreeMap<u16, u16>,
    stopped: bool,
}

impl RelativeMouseSession {
    pub fn open_saved(settings: &super::relative_settings::RelativeDeviceSettings) -> Result<Self> {
        settings.validate()?;
        let mut session = Self::open_mapped(
            &settings.event_path,
            &settings.input_identity,
            settings.axes.iter().copied(),
            settings.buttons.iter().copied(),
            settings.motion,
        )?;
        if settings.exclusive_source {
            session.reader.grab_exclusive()?;
        }
        Ok(session)
    }

    pub fn open(
        path: &Path,
        expected_input_identity: &Path,
        axes: impl IntoIterator<Item = u16>,
        buttons: impl IntoIterator<Item = u16>,
    ) -> Result<Self> {
        Self::open_calibrated(
            path,
            expected_input_identity,
            axes,
            buttons,
            RelativeMotionCalibration::default(),
        )
    }

    pub fn open_calibrated(
        path: &Path,
        expected_input_identity: &Path,
        axes: impl IntoIterator<Item = u16>,
        buttons: impl IntoIterator<Item = u16>,
        calibration: RelativeMotionCalibration,
    ) -> Result<Self> {
        Self::open_mapped(
            path,
            expected_input_identity,
            axes,
            buttons.into_iter().map(|button| (button, button)),
            calibration,
        )
    }

    /// Explicit physical-to-output mouse-button mapping. Every selected source
    /// has one distinct output; this does not synthesize chords or keyboard keys.
    pub fn open_mapped(
        path: &Path,
        expected_input_identity: &Path,
        axes: impl IntoIterator<Item = u16>,
        buttons: impl IntoIterator<Item = (u16, u16)>,
        calibration: RelativeMotionCalibration,
    ) -> Result<Self> {
        calibration.validate()?;
        let mut button_map = BTreeMap::new();
        let mut outputs = BTreeSet::new();
        for (source, target) in buttons {
            ensure!(
                (0x110..=0x117).contains(&source) && (0x110..=0x117).contains(&target),
                "Relative button mapping requires mouse-button identities"
            );
            ensure!(
                button_map.insert(source, target).is_none() && outputs.insert(target),
                "Relative button mapping requires distinct sources and outputs"
            );
        }
        let reader = RelativeReader::open(
            path,
            expected_input_identity,
            axes,
            button_map.keys().copied(),
        )?;
        let output =
            VirtualRelativeMouse::create_configured(&reader, calibration.swap_xy, outputs)?;
        Ok(Self {
            reader,
            output,
            motion: RelativeMotionTransform {
                calibration,
                remainder: [0; 2],
            },
            button_map,
            stopped: false,
        })
    }

    pub fn system_name(&self) -> &str {
        self.output.system_name()
    }

    pub fn endpoint(&self) -> Result<Option<RelativeMouseEndpoint>> {
        ensure!(!self.stopped, "Relative mouse session is stopped");
        self.output.endpoint()
    }

    /// Forward complete kernel reports once, with a bounded read batch. The
    /// owning launch worker controls cadence and must stop on the first error.
    pub fn pump(&mut self) -> Result<usize> {
        ensure!(!self.stopped, "Relative mouse session is stopped");
        let result = self.pump_inner();
        if result.is_err() {
            self.shutdown();
        }
        result
    }

    fn pump_inner(&mut self) -> Result<usize> {
        let packets = self.reader.poll()?;
        let count = packets.len();
        for mut packet in packets {
            self.motion.apply(&mut packet)?;
            ensure!(
                packet.buttons.keys().eq(self.button_map.keys()),
                "Relative packet button identities changed during the session"
            );
            packet.buttons = packet
                .buttons
                .into_iter()
                .map(|(source, held)| (self.button_map[&source], held))
                .collect();
            self.output.publish(&packet)?;
        }
        Ok(count)
    }

    pub fn shutdown(&mut self) {
        if !self.stopped {
            self.reader.neutral();
            let neutral = self.output.neutral_packet();
            self.motion.remainder = [0; 2];
            let _ = self.output.publish(&neutral);
            self.output.shutdown();
            self.stopped = true;
        }
    }
}

impl Drop for RelativeMouseSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Own a selected set of player-relative transports as one launch lifetime.
/// Player keys are caller identities, never RetroArch mouse indices. Endpoint
/// readiness must still be followed by exact frontend routing before launch.
pub struct RelativeMouseGroup {
    sessions: BTreeMap<u8, RelativeMouseSession>,
    stopped: bool,
}

impl RelativeMouseGroup {
    /// Validate the complete selection before opening hardware. If any open
    /// fails, already-created sessions drop and release their owned outputs.
    pub fn open_saved(
        players: &BTreeMap<u8, super::relative_settings::RelativeDeviceSettings>,
    ) -> Result<Self> {
        ensure!(!players.is_empty(), "No relative devices selected");
        let devices: Vec<_> = players.values().cloned().collect();
        super::relative_settings::validate_devices(&devices)?;
        let mut sessions = BTreeMap::new();
        for (player, settings) in players {
            sessions.insert(*player, RelativeMouseSession::open_saved(settings)?);
        }
        Ok(Self {
            sessions,
            stopped: false,
        })
    }

    /// Return the complete endpoint set only when every owned device is ready.
    /// This performs no waiting and supplies no guessed frontend index. The
    /// caller owns a bounded readiness deadline and must shut down on expiry.
    pub fn endpoints(&mut self) -> Result<Option<BTreeMap<u8, RelativeMouseEndpoint>>> {
        ensure!(!self.stopped, "Relative mouse group is stopped");
        let result = (|| {
            let mut endpoints = BTreeMap::new();
            let mut pending = false;
            for (player, session) in &self.sessions {
                match session.endpoint()? {
                    Some(endpoint) => {
                        endpoints.insert(*player, endpoint);
                    }
                    None => pending = true,
                }
            }
            Ok(if pending { None } else { Some(endpoints) })
        })();
        if result.is_err() {
            self.shutdown();
        }
        result
    }

    /// Resolve one frontend startup against the complete owned endpoint set.
    /// Errors stop every member; successful resolution does not forward input.
    pub fn resolve_frontend_routes(
        &mut self,
        startup_log: &str,
        topology: &mut super::relative_topology::RelativeTopologyGuard,
    ) -> Result<BTreeMap<u8, u32>> {
        // Subscribe before frontend startup. The caller must establish this
        // log's executable/startup provenance and apply/verify these routes
        // before forwarding. This method does not start a bridge.
        let result = (|| {
            topology.verify()?;
            let endpoints = self.endpoints()?.ok_or_else(|| {
                anyhow::anyhow!("Relative endpoints are not ready for route resolution")
            })?;
            let paths = endpoints
                .iter()
                .map(|(player, endpoint)| (*player, endpoint.event_node.clone()))
                .collect();
            let routes = super::relative_frontend::relative_player_indices(startup_log, &paths)?;
            ensure!(
                self.endpoints()?.as_ref() == Some(&endpoints),
                "Owned relative endpoints changed during route resolution"
            );
            topology.verify()?;
            Ok(routes)
        })();
        if result.is_err() {
            self.shutdown();
        }
        result
    }

    /// Produce a private routing fragment from revalidated owned endpoints.
    /// Applying this fragment requires another check against the actual game
    /// startup; configuration generation is not an applied-route receipt.
    pub fn frontend_configuration(
        &mut self,
        startup_log: &str,
        topology: &mut super::relative_topology::RelativeTopologyGuard,
    ) -> Result<String> {
        let result = self
            .resolve_frontend_routes(startup_log, topology)
            .and_then(|routes| super::relative_frontend::relative_player_config(&routes));
        if result.is_err() {
            self.shutdown();
        }
        result
    }

    /// One bounded poll per source. A failure stops every member, preventing
    /// other players from retaining held virtual buttons after a partial fault.
    /// Publications before a fault cannot be rolled back.
    pub fn pump(&mut self) -> Result<BTreeMap<u8, usize>> {
        ensure!(!self.stopped, "Relative mouse group is stopped");
        let result = self
            .sessions
            .iter_mut()
            .map(|(player, session)| session.pump().map(|count| (*player, count)))
            .collect::<Result<BTreeMap<_, _>>>();
        if result.is_err() {
            self.shutdown();
        }
        result
    }

    pub fn shutdown(&mut self) {
        for session in self.sessions.values_mut() {
            session.shutdown();
        }
        self.stopped = true;
    }
}

impl Drop for RelativeMouseGroup {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Background owner for a completely prepared relative-device group. This is
/// transport only: callers must establish exact frontend routes and explicitly
/// stop on focus loss, cancellation or emulator exit. Drop joins the worker.
pub struct RelativeMouseBridge {
    stop: std::sync::mpsc::Sender<()>,
    worker: Option<std::thread::JoinHandle<()>>,
    failure: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    routing_confirmed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl RelativeMouseBridge {
    /// Retain the fresh child's command pipes for initial and ongoing routing
    /// checks. The owner establishes executable/startup-log identity and retains
    /// the Child for termination on failure, focus loss or cancellation.
    /// No input is forwarded before the first matching effective-state reply.
    pub fn start_with_command_channel(
        mut group: RelativeMouseGroup,
        mut topology: super::relative_topology::RelativeTopologyGuard,
        mut commands: super::relative_command::RelativeCommandChannel,
    ) -> Result<Self> {
        ensure!(
            group
                .sessions
                .values()
                .all(|session| session.reader.initializing),
            "Relative handshake requires an unstarted source group"
        );
        let endpoints = group.endpoints()?.ok_or_else(|| {
            anyhow::anyhow!("Relative endpoints are not ready for background forwarding")
        })?;
        topology.verify()?;
        commands.begin_query(std::time::Duration::from_secs(2))?;
        let (stop, stopping) = std::sync::mpsc::channel();
        let failure = std::sync::Arc::new(std::sync::Mutex::new(None));
        let worker_failure = failure.clone();
        let routing_confirmed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_confirmed = routing_confirmed.clone();
        let worker = std::thread::Builder::new()
            .name("controller-relative-bridge".to_owned())
            .spawn(move || {
                let result: Result<()> = (|| {
                    let mut confirmed = false;
                    let mut resolved = None;
                    let mut query_pending = true;
                    let mut next_query = std::time::Instant::now();
                    let initial_focus_deadline = next_query + std::time::Duration::from_secs(2);
                    loop {
                        match stopping.try_recv() {
                            Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                return Ok(());
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => {}
                        }
                        topology.verify()?;
                        ensure!(confirmed || std::time::Instant::now() < initial_focus_deadline,
                            "Frontend did not establish focused routing before the startup deadline");
                        if let Some(effective) = commands.poll()? {
                            ensure!(query_pending, "Unexpected relative routing receipt");
                            if resolved.is_none() {
                                ensure!(effective.routing.is_disabled(),
                                    "Relative frontend did not start with all mouse ports disabled");
                                let log = commands.seal_startup_log()?;
                                resolved = Some(group.resolve_frontend_routes(&log, &mut topology)?);
                                commands.begin_route_assignment(resolved.as_ref()
                                    .ok_or_else(|| anyhow::anyhow!("Missing owned frontend routes"))?,
                                    std::time::Duration::from_millis(250))?;
                                // Setter replies with effective state. Keep the
                                // pending flag and never forward on send alone.
                                continue;
                            }
                            effective.routing.validate_routes(resolved.as_ref()
                                .ok_or_else(|| anyhow::anyhow!("Missing owned frontend routes"))?)?;
                            ensure!(!confirmed || effective.focused,
                                "Frontend lost focus; relative capture stopped and requires a new session");
                            ensure!(
                                group.endpoints()?.as_ref() == Some(&endpoints),
                                "Owned endpoints changed during the routing handshake"
                            );
                            topology.verify()?;
                            query_pending = false;
                            confirmed = effective.focused;
                            next_query =
                                std::time::Instant::now() + std::time::Duration::from_millis(100);
                            worker_confirmed.store(confirmed, std::sync::atomic::Ordering::Release);
                        }
                        if !query_pending && std::time::Instant::now() >= next_query {
                            commands.begin_query(std::time::Duration::from_millis(250))?;
                            query_pending = true;
                        }
                        // The first reader poll discards all startup backlog.
                        // Later queries monitor routes without dropping motion;
                        // their cadence is not an atomic frontend routing lock.
                        if confirmed {
                            group.pump()?;
                        }
                        // A stop wakes this wait immediately. The regular poll
                        // uses the same cadence as the existing gamepad bridge.
                        match stopping.recv_timeout(std::time::Duration::from_millis(2)) {
                            Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                                return Ok(());
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        }
                    }
                })();
                worker_confirmed.store(false, std::sync::atomic::Ordering::Release);
                group.shutdown();
                if let Ok(mut failure) = worker_failure.lock() {
                    *failure = result.err().map(|error| format!("{error:#}"));
                }
            })?;
        Ok(Self {
            stop,
            worker: Some(worker),
            failure,
            routing_confirmed,
        })
    }

    /// Initial route confirmation only, not game readiness or a live atomic
    /// guarantee. Owners must also check health and retain focus/exit handling.
    pub fn routing_confirmed(&self) -> Result<bool> {
        self.check_health()?;
        Ok(self
            .routing_confirmed
            .load(std::sync::atomic::Ordering::Acquire))
    }

    pub fn check_health(&self) -> Result<()> {
        let failure = self
            .failure
            .lock()
            .map_err(|_| anyhow::anyhow!("Relative worker health lock poisoned"))?;
        if let Some(error) = failure.as_ref() {
            anyhow::bail!("Relative controller transport failed: {error}");
        }
        ensure!(
            self.worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished()),
            "Relative controller transport is stopped"
        );
        Ok(())
    }

    /// Wait for source closure and virtual-output destruction before returning.
    /// A joining owner can surface worker failures; Drop performs best-effort
    /// cleanup when unwinding or abandoning launch preparation.
    pub fn shutdown(&mut self) -> Result<()> {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("Relative controller worker panicked"))?;
        }
        let failure = self
            .failure
            .lock()
            .map_err(|_| anyhow::anyhow!("Relative worker health lock poisoned"))?;
        if let Some(error) = failure.as_ref() {
            anyhow::bail!("Relative controller transport failed: {error}");
        }
        Ok(())
    }
}

impl Drop for RelativeMouseBridge {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
