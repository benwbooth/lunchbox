//! Session-owned, fixed-shape puNES target pad and calibrated input bridge.

use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Write},
    os::fd::AsRawFd,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, atomic::AtomicBool, mpsc},
    thread,
    time::{Duration, Instant},
};

use crate::{
    controller_axis::{GamepadControl, GamepadReader, normalized_gamepad_calibration},
    controller_catalog::{Calibration, EmulatorProfile},
};

const BUS_VIRTUAL: u16 = 0x06;
const VENDOR: u16 = 0x1209;
const VERSION: u16 = 0x0001;
const PRODUCT_BASE: u16 = 0x4c50;
const BUTTONS: [u16; 8] = [
    0x130, // BTN_A
    0x131, // BTN_B
    0x13a, // BTN_SELECT
    0x13b, // BTN_START
    0x220, // BTN_DPAD_UP
    0x221, // BTN_DPAD_DOWN
    0x222, // BTN_DPAD_LEFT
    0x223, // BTN_DPAD_RIGHT
];
const TARGETS: [&str; 8] = ["a", "b", "select", "start", "up", "down", "left", "right"];
const UDEV_DATA_LIMIT: u64 = 64 * 1024;

fn decimal(bytes: &[u8]) -> Option<u64> {
    (!bytes.is_empty() && bytes.iter().all(u8::is_ascii_digit))
        .then(|| {
            bytes.iter().try_fold(0u64, |value, byte| {
                value.checked_mul(10)?.checked_add(u64::from(byte - b'0'))
            })
        })
        .flatten()
}

fn udev_classifies_current_joystick(bytes: &[u8], created_after_usec: u64) -> bool {
    let lines = bytes.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    let initialized = lines
        .iter()
        .filter_map(|line| line.strip_prefix(b"I:").and_then(decimal))
        .collect::<Vec<_>>();
    initialized.len() == 1
        && initialized[0] >= created_after_usec
        && lines.iter().any(|line| *line == b"E:ID_INPUT_JOYSTICK=1")
}

fn monotonic_time_usec() -> Result<u64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    ensure!(
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) } == 0,
        "Cannot timestamp puNES target creation: {}",
        std::io::Error::last_os_error()
    );
    let seconds = u64::try_from(time.tv_sec)?;
    let nanoseconds = u64::try_from(time.tv_nsec)?;
    Ok(seconds
        .checked_mul(1_000_000)
        .and_then(|value| value.checked_add(nanoseconds / 1_000))
        .context("puNES target timestamp overflow")?)
}

fn udev_joystick_ready(rdev: u64, created_after_usec: u64) -> Result<bool> {
    let name = format!("c{}:{}", libc::major(rdev), libc::minor(rdev));
    for root in ["/run/udev/data", "/run/host/run/udev/data"] {
        let path = Path::new(root).join(&name);
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        ensure!(
            file.metadata()?.is_file(),
            "puNES target udev identity is not a regular file"
        );
        let mut bytes = Vec::new();
        file.take(UDEV_DATA_LIMIT + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= UDEV_DATA_LIMIT,
            "puNES target udev identity exceeds its size limit"
        );
        if udev_classifies_current_joystick(&bytes, created_after_usec) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn input_event_bytes(kind: u16, code: u16, value: i32) -> Vec<u8> {
    let mut bytes = vec![0; std::mem::size_of::<libc::timeval>()];
    bytes.extend_from_slice(&kind.to_ne_bytes());
    bytes.extend_from_slice(&code.to_ne_bytes());
    bytes.extend_from_slice(&value.to_ne_bytes());
    bytes
}

#[derive(Clone, Debug)]
struct Route {
    source: GamepadControl,
    released: i32,
    pressed: i32,
    output: u16,
}

impl Route {
    fn active(&self, value: i32) -> Result<bool> {
        match self.source {
            GamepadControl::Button(_) => {
                ensure!((0..=1).contains(&value), "Invalid normalized source button");
                Ok(value != 0)
            }
            GamepadControl::Axis(_) => {
                let span = i64::from(self.pressed) - i64::from(self.released);
                ensure!(span != 0, "puNES source axis has no measured travel");
                let projection = (i64::from(value) - i64::from(self.released)) * span;
                Ok(projection * 2 >= span * span)
            }
        }
    }
}

fn routes(
    calibration: &Calibration,
    profile: &EmulatorProfile,
) -> Result<(crate::controller_axis::GamepadFrame, Vec<Route>)> {
    ensure!(
        profile.target_layout == "nes",
        "puNES needs the NES target layout"
    );
    let plan = calibration.plan_profile(profile)?;
    ensure!(
        plan.rows.len() == TARGETS.len()
            && plan
                .rows
                .iter()
                .map(|row| row.target_id.as_str())
                .collect::<BTreeSet<_>>()
                == TARGETS.into_iter().collect(),
        "puNES needs all eight standard NES mappings"
    );
    let selected = plan
        .rows
        .iter()
        .map(|row| {
            row.physical_id
                .clone()
                .context("puNES target has no selected physical control")
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let mut subset = calibration.clone();
    subset.bindings.retain(|id, _| selected.contains(id));
    let (frame, translated) = normalized_gamepad_calibration(&subset)?;
    let translated_plan = translated.plan_profile(profile)?;
    let mut result = Vec::new();
    let mut outputs = BTreeSet::new();
    for row in translated_plan.rows {
        let output = BUTTONS[TARGETS
            .iter()
            .position(|target| *target == row.target_id)
            .context("Unknown puNES target control")?];
        ensure!(outputs.insert(output), "Duplicate puNES target control");
        let input = row.input.context("puNES target mapping is incomplete")?;
        let native = input
            .native
            .context("puNES mapping lacks physical input identity")?;
        let source = match native.code >> 16 {
            1 => GamepadControl::Button(native.code as u16),
            3 => GamepadControl::Axis(native.code as u16),
            _ => anyhow::bail!("puNES accepts only physical gamepad buttons and axes"),
        };
        let (released, pressed) = if matches!(source, GamepadControl::Axis(_)) {
            let axis = input.axis.context("puNES axis mapping lacks calibration")?;
            (axis.released, axis.pressed)
        } else {
            (0, 1)
        };
        result.push(Route {
            source,
            released,
            pressed,
            output,
        });
    }
    ensure!(result.len() == 8, "Incomplete puNES target routing");
    Ok((frame, result))
}

fn guid(bus: u16, vendor: u16, product: u16, version: u16) -> String {
    format!(
        "{{{:04X}{:04X}-{:04X}-{:04X}-{:04X}-{:04X}{:04X}{:04X}}}",
        bus.wrapping_sub(500),
        0u16.wrapping_sub(100),
        vendor,
        vendor.wrapping_sub(200),
        product,
        product.wrapping_sub(300),
        version,
        version.wrapping_sub(400)
    )
}

struct TargetPad {
    file: File,
    created: bool,
    system_name: String,
    name: String,
    player: u8,
    state: BTreeMap<u16, bool>,
    created_after_usec: u64,
}

struct SourceNode {
    path: PathBuf,
    input_identity: PathBuf,
    filesystem: u64,
    inode: u64,
    rdev: u64,
}

impl SourceNode {
    fn capture(path: &Path, input_identity: &Path) -> Result<Self> {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};

        let metadata = std::fs::symlink_metadata(path)?;
        ensure!(
            metadata.file_type().is_char_device(),
            "puNES source event node is not a character device"
        );
        let source = Self {
            path: path.to_path_buf(),
            input_identity: input_identity.to_path_buf(),
            filesystem: metadata.dev(),
            inode: metadata.ino(),
            rdev: metadata.rdev(),
        };
        source.verify()?;
        Ok(source)
    }

    fn verify(&self) -> Result<()> {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};

        let metadata = std::fs::symlink_metadata(&self.path)?;
        ensure!(
            metadata.file_type().is_char_device()
                && metadata.dev() == self.filesystem
                && metadata.ino() == self.inode
                && metadata.rdev() == self.rdev,
            "puNES physical source event node changed"
        );
        let identity = Path::new("/sys/dev/char")
            .join(format!(
                "{}:{}",
                libc::major(metadata.rdev()),
                libc::minor(metadata.rdev())
            ))
            .join("device")
            .canonicalize()?;
        ensure!(
            identity == self.input_identity,
            "puNES physical source identity changed"
        );
        Ok(())
    }
}

impl TargetPad {
    fn create(player: u8) -> Result<Self> {
        ensure!(
            (1..=2).contains(&player),
            "puNES target player must be one or two"
        );
        let file = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open("/dev/uinput")
            .map_err(|error| anyhow::anyhow!("puNES target requires writable /dev/uinput: {error}. No permissions were changed."))?;
        let mut pad = Self {
            file,
            created: false,
            system_name: String::new(),
            name: format!("Lunchbox puNES target P{player}"),
            player,
            state: BUTTONS.into_iter().map(|code| (code, false)).collect(),
            created_after_usec: 0,
        };
        pad.capability(0x40045564, 1)?; // UI_SET_EVBIT(EV_KEY)
        for code in BUTTONS {
            pad.capability(0x40045565, i32::from(code))?; // UI_SET_KEYBIT
        }
        let mut setup = [0u8; 92]; // uinput_setup
        setup[..2].copy_from_slice(&BUS_VIRTUAL.to_ne_bytes());
        setup[2..4].copy_from_slice(&VENDOR.to_ne_bytes());
        setup[4..6].copy_from_slice(&(PRODUCT_BASE + u16::from(player)).to_ne_bytes());
        setup[6..8].copy_from_slice(&VERSION.to_ne_bytes());
        ensure!(
            pad.name.len() < 80,
            "puNES target name exceeds kernel limit"
        );
        setup[8..8 + pad.name.len()].copy_from_slice(pad.name.as_bytes());
        pad.setup(0x405c5503, &setup)?; // UI_DEV_SETUP
        pad.created_after_usec = monotonic_time_usec()?;
        ensure!(
            unsafe { libc::ioctl(pad.file.as_raw_fd(), 0x5501 as libc::c_ulong) } >= 0,
            "Cannot create puNES target: {}",
            std::io::Error::last_os_error()
        );
        pad.created = true;
        let mut system_name = [0u8; 128];
        let request = 0x8000_0000u32 | (128 << 16) | (0x55 << 8) | 44;
        ensure!(
            unsafe {
                libc::ioctl(
                    pad.file.as_raw_fd(),
                    request as libc::c_ulong,
                    system_name.as_mut_ptr(),
                )
            } >= 0,
            "Cannot identify puNES target: {}",
            std::io::Error::last_os_error()
        );
        let end = system_name
            .iter()
            .position(|byte| *byte == 0)
            .context("Invalid puNES target identity")?;
        pad.system_name = std::str::from_utf8(&system_name[..end])?.to_owned();
        ensure!(
            pad.system_name.strip_prefix("input").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
            }),
            "Unexpected puNES target identity"
        );
        pad.publish(&BTreeMap::new())?;
        Ok(pad)
    }

    fn capability(&self, request: u32, value: i32) -> Result<()> {
        ensure!(
            unsafe {
                libc::ioctl(
                    self.file.as_raw_fd(),
                    request as libc::c_ulong,
                    value as libc::c_int,
                )
            } >= 0,
            "Cannot declare puNES target capability: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }

    fn setup(&self, request: u32, bytes: &[u8]) -> Result<()> {
        ensure!(
            unsafe {
                libc::ioctl(
                    self.file.as_raw_fd(),
                    request as libc::c_ulong,
                    bytes.as_ptr(),
                )
            } >= 0,
            "Cannot configure puNES target: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }

    fn publish(&mut self, updates: &BTreeMap<u16, bool>) -> Result<()> {
        ensure!(self.created, "puNES target is no longer available");
        let event_size = std::mem::size_of::<libc::timeval>() + 8;
        ensure!(
            event_size == std::mem::size_of::<libc::input_event>(),
            "Unsupported Linux input_event layout"
        );
        let mut bytes = Vec::with_capacity((updates.len() + 1) * event_size);
        for (&code, &pressed) in updates {
            ensure!(
                self.state.contains_key(&code),
                "Unknown puNES target button"
            );
            self.state.insert(code, pressed);
            bytes.extend_from_slice(&input_event_bytes(1, code, i32::from(pressed)));
        }
        bytes.extend_from_slice(&input_event_bytes(0, 0, 0)); // SYN_REPORT
        self.file.write_all(&bytes)?;
        Ok(())
    }

    fn publish_source(
        &mut self,
        routes: &[Route],
        frame: &BTreeMap<GamepadControl, i32>,
    ) -> Result<()> {
        let mut updates = BTreeMap::new();
        for route in routes {
            let Some(value) = frame.get(&route.source) else {
                continue;
            };
            let active = route.active(*value)?;
            if self.state.get(&route.output) != Some(&active) {
                updates.insert(route.output, active);
            }
        }
        if !updates.is_empty() {
            self.publish(&updates)?;
        }
        Ok(())
    }

    fn neutralize(&mut self) -> Result<()> {
        let updates: BTreeMap<u16, bool> = self
            .state
            .iter()
            .filter_map(|(&code, &pressed)| pressed.then_some((code, false)))
            .collect();
        if !updates.is_empty() {
            self.publish(&updates)?;
        }
        Ok(())
    }

    fn event_path(&self) -> Result<Option<PathBuf>> {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        let root = Path::new("/sys/class/input").join(&self.system_name);
        let entries = match std::fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut found = None;
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let Some(index) = name.to_str().and_then(|name| name.strip_prefix("event")) else {
                continue;
            };
            if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
                continue;
            }
            let path = Path::new("/dev/input").join(&name);
            let metadata = std::fs::metadata(&path)?;
            ensure!(
                metadata.file_type().is_char_device(),
                "puNES target event node is not a character device"
            );
            let expected = std::fs::read_to_string(entry.path().join("dev"))?;
            ensure!(
                expected.trim()
                    == format!(
                        "{}:{}",
                        libc::major(metadata.rdev()),
                        libc::minor(metadata.rdev())
                    ),
                "puNES target event identity changed"
            );
            if !udev_joystick_ready(metadata.rdev(), self.created_after_usec)? {
                continue;
            }
            ensure!(
                found.replace(path).is_none(),
                "puNES target has ambiguous event nodes"
            );
        }
        Ok(found)
    }

    fn guid(&self) -> String {
        guid(
            BUS_VIRTUAL,
            VENDOR,
            PRODUCT_BASE + u16::from(self.player),
            VERSION,
        )
    }

    fn shutdown(&mut self) {
        if self.created {
            let _ = self.neutralize();
            unsafe { libc::ioctl(self.file.as_raw_fd(), 0x5502 as libc::c_ulong) };
            self.created = false;
        }
    }
}

impl Drop for TargetPad {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) struct Bridge {
    stop: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
    failure: Arc<Mutex<Option<String>>>,
    event_path: PathBuf,
    guid: String,
    name: String,
    source: SourceNode,
}

impl Bridge {
    pub(crate) fn start(
        player: u8,
        calibration: &Calibration,
        profile: &EmulatorProfile,
        event_path: &Path,
        input_identity: &Path,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        ensure!(
            !cancel.load(std::sync::atomic::Ordering::Relaxed),
            "puNES bridge startup cancelled"
        );
        let (frame, routes) = routes(calibration, profile)?;
        let source = SourceNode::capture(event_path, input_identity)?;
        let mut reader = GamepadReader::open(event_path, frame, input_identity)?;
        // The audited Flatpak has devices=all, so its sandbox can still open
        // the physical node. Hold an exclusive grab on the same verified FD
        // that feeds the bridge before the target exists or puNES can launch.
        reader.grab_exclusive()?;
        let mut pad = TargetPad::create(player)?;
        source.verify()?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let target_event = loop {
            if let Some(path) = pad.event_path()? {
                break path;
            }
            ensure!(
                Instant::now() < deadline,
                "puNES target event node did not appear"
            );
            thread::sleep(Duration::from_millis(10));
        };
        let guid = pad.guid();
        let name = pad.name.clone();
        let (stop, stopping) = mpsc::channel();
        let (ready, readiness) = mpsc::sync_channel(1);
        let failure = Arc::new(Mutex::new(None));
        let worker_failure = failure.clone();
        let worker = thread::Builder::new()
            .name(format!("punes-controller-p{player}"))
            .spawn(move || {
                let result: Result<()> = (|| loop {
                    match stopping.try_recv() {
                        Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                        Err(mpsc::TryRecvError::Empty) => {}
                    }
                    for frame in reader.poll()? {
                        pad.publish_source(&routes, &frame)?;
                        let _ = ready.try_send(Ok(()));
                    }
                    match stopping.recv_timeout(Duration::from_millis(2)) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                })();
                let neutral = pad.neutralize();
                pad.shutdown();
                let error = result
                    .err()
                    .or_else(|| neutral.err())
                    .map(|error| format!("{error:#}"));
                let _ = ready.try_send(Err(error
                    .clone()
                    .unwrap_or_else(|| "puNES bridge stopped before synchronization".into())));
                if let Ok(mut failure) = worker_failure.lock() {
                    *failure = error;
                }
            })?;
        let bridge = Self {
            stop,
            worker: Some(worker),
            failure,
            event_path: target_event,
            guid,
            name,
            source,
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            ensure!(
                !cancel.load(std::sync::atomic::Ordering::Relaxed),
                "puNES bridge startup cancelled"
            );
            ensure!(
                Instant::now() < deadline,
                "puNES bridge did not synchronize"
            );
            match readiness.recv_timeout(Duration::from_millis(20)) {
                Ok(Ok(())) => break,
                Ok(Err(error)) => anyhow::bail!("puNES bridge startup failed: {error}"),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(error) => anyhow::bail!("puNES bridge readiness failed: {error}"),
            }
        }
        if let Err(error) = bridge.check_health() {
            drop(bridge);
            return Err(error);
        }
        Ok(bridge)
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        use std::os::unix::fs::FileTypeExt;

        if let Some(error) = self
            .failure
            .lock()
            .map_err(|_| anyhow::anyhow!("puNES bridge status unavailable"))?
            .as_ref()
        {
            anyhow::bail!("puNES bridge failed: {error}");
        }
        ensure!(
            self.worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished()),
            "puNES bridge stopped unexpectedly"
        );
        ensure!(
            self.event_path
                .metadata()
                .is_ok_and(|metadata| metadata.file_type().is_char_device()),
            "puNES target event node disappeared"
        );
        self.source.verify()?;
        Ok(())
    }

    pub(crate) fn event_path(&self) -> &Path {
        &self.event_path
    }

    pub(crate) fn guid(&self) -> &str {
        &self.guid
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_matches_punes_linux_word_encoding() {
        assert_eq!(
            guid(BUS_VIRTUAL, VENDOR, PRODUCT_BASE + 1, VERSION),
            "{FE12FF9C-1209-1141-4C51-4B250001FE71}"
        );
        assert_ne!(
            guid(BUS_VIRTUAL, VENDOR, PRODUCT_BASE + 1, VERSION),
            guid(BUS_VIRTUAL, VENDOR, PRODUCT_BASE + 2, VERSION)
        );
    }

    #[test]
    fn axis_route_uses_measured_half_travel() {
        let route = Route {
            source: GamepadControl::Axis(0),
            released: 100,
            pressed: -900,
            output: BUTTONS[4],
        };
        assert!(!route.active(100).unwrap());
        assert!(!route.active(-399).unwrap());
        assert!(route.active(-400).unwrap());
        assert!(route.active(-900).unwrap());
    }

    #[test]
    fn input_event_encoding_matches_native_linux_layout() {
        let bytes = input_event_bytes(1, 0x223, 1);
        let timeval = std::mem::size_of::<libc::timeval>();
        assert_eq!(bytes.len(), std::mem::size_of::<libc::input_event>());
        assert!(bytes[..timeval].iter().all(|byte| *byte == 0));
        assert_eq!(&bytes[timeval..timeval + 2], &1u16.to_ne_bytes());
        assert_eq!(&bytes[timeval + 2..timeval + 4], &0x223u16.to_ne_bytes());
        assert_eq!(&bytes[timeval + 4..], &1i32.to_ne_bytes());
    }

    #[test]
    fn udev_joystick_classification_is_current_and_exact() {
        assert!(udev_classifies_current_joystick(
            b"I:1234\nE:ID_INPUT=1\nE:ID_INPUT_JOYSTICK=1\n",
            1234,
        ));
        assert!(!udev_classifies_current_joystick(
            b"I:1233\nE:ID_INPUT_JOYSTICK=1\n",
            1234,
        ));
        assert!(!udev_classifies_current_joystick(
            b"I:1234\nE:ID_INPUT_JOYSTICK=0\nE:ID_INPUT_JOYSTICK_EXTRA=1\n",
            1234,
        ));
    }
}
