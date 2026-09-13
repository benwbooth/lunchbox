//! FBNeo libretro input identities, pinned to a251c76229f1637e433b93e29845039752771b6d.
//! Device selection is not a game layout: native descriptors must still resolve
//! the loaded driver's buttons, axes, macros and player count.
use anyhow::{Context, Result, ensure};
pub(crate) mod keyboard;

/// Run the helper's worker directly: the application owns timeout, cancellation
/// and its temporary tree. Nesting its CLI supervisor here would risk orphaning
/// a native child when a cancelled launch kills only the outer supervisor.
/// Call from a background launch task, never the UI thread. The supplied helper
/// and environment must belong to the selected trusted native runtime.
pub(crate) fn inspect_runtime(
    helper: &std::path::Path,
    environment: &[(std::ffi::OsString, std::ffi::OsString)],
    request: &lunchbox_controller_probe::content_inspection::Request,
    topology: &ControllerTopology,
    timeout: std::time::Duration,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<lunchbox_controller_probe::libretro_input::ContentControllerReport> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    request.validate()?;
    ensure!(
        helper.is_absolute() && helper.is_file(),
        "Resolve the trusted content-inspection helper first"
    );
    ensure!(
        (Duration::from_secs(1)..=Duration::from_secs(300)).contains(&timeout),
        "Content-inspection timeout must be between 1 and 300 seconds"
    );
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "FBNeo inspection cancelled"
    );
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-fbneo-inspection-")
        .tempdir()?;
    let root = directory.path().canonicalize()?;
    let request_path = root.join("request.json");
    let result_path = root.join("result.json");
    let temporary_root = root.join("temporary");
    std::fs::create_dir(&temporary_root)?;
    let request_bytes = serde_json::to_vec(request)?;
    ensure!(
        request_bytes.len() <= 1024 * 1024,
        "FBNeo inspection request exceeds its limit"
    );
    std::fs::write(&request_path, request_bytes)?;

    struct NativeWorker(std::process::Child);
    impl Drop for NativeWorker {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut worker = NativeWorker(
        Command::new(helper)
            .arg("--worker")
            .arg("--request")
            .arg(&request_path)
            .arg("--worker-result")
            .arg(&result_path)
            .envs(environment.iter().cloned())
            // Override temp roots after runtime variables: every staged input and
            // core save remains under the application-owned cleanup directory.
            .env("TMPDIR", &temporary_root)
            .env("TMP", &temporary_root)
            .env("TEMP", &temporary_root)
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Starting FBNeo native descriptor inspection")?,
    );
    let deadline = Instant::now() + timeout;
    let status = loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "FBNeo inspection cancelled"
        );
        if let Some(status) = worker.0.try_wait()? {
            break status;
        }
        ensure!(
            Instant::now() < deadline,
            "FBNeo native descriptor inspection timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    ensure!(
        status.success(),
        "FBNeo native descriptor worker failed: {status}"
    );
    let mut bytes = Vec::new();
    std::fs::File::open(&result_path)
        .context("Reading FBNeo inspection result")?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "FBNeo inspection result exceeds its limit"
    );
    #[derive(serde::Deserialize)]
    #[serde(tag = "status", deny_unknown_fields)]
    enum Outcome {
        Success { report: serde_json::Value },
        Failure { error: String },
    }
    let result = match serde_json::from_slice::<Outcome>(&bytes)? {
        Outcome::Success { report } => {
            parse_inspection_report(request, topology, &serde_json::to_vec(&report)?)?
        }
        Outcome::Failure { error } => anyhow::bail!(
            "FBNeo inspection failed: {}",
            error.chars().take(16384).collect::<String>()
        ),
    };
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "FBNeo inspection cancelled"
    );
    Ok(result)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputAddress {
    pub(crate) port: u32,
    pub(crate) device: u32,
    pub(crate) index: u32,
    pub(crate) id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectiveDescriptor {
    pub(crate) address: InputAddress,
    pub(crate) description: String,
}

/// Transport semantics, not a guess at an in-game action or physical control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InputEncoding {
    JoypadButton,
    KeyboardKey,
    SignedAxis,
    AnalogButtonPressure,
    RelativeAxis,
    AbsoluteCoordinate,
    Button,
    WheelEvent,
    OffscreenStatus,
    TouchCount,
    Unknown,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BindingPart {
    Digital,
    Negative,
    Positive,
    Pressure,
}

/// RetroArch 69a4f0ea input_config_bind_map fields. Coordinate, mouse,
/// keyboard and touch inputs need their own runtime adapters, not pad fields,
/// except Arcade Gun's explicit analog-channel absolute-coordinate route.
pub(crate) fn retroarch_field(address: &InputAddress, part: BindingPart) -> Result<&'static str> {
    const PAD: [&str; 16] = [
        "b", "y", "select", "start", "up", "down", "left", "right", "a", "x", "l", "r", "l2", "r2",
        "l3", "r3",
    ];
    match (address.device, address.index, address.id, part) {
        (1, 0, id @ 0..=15, BindingPart::Digital) | (5, 2, id @ 0..=15, BindingPart::Pressure) => {
            Ok(PAD[id as usize])
        }
        (5, index @ 0..=1, id @ 0..=1, BindingPart::Negative | BindingPart::Positive)
        | (1029, index @ 0, id @ 0..=1, BindingPart::Negative | BindingPart::Positive) => {
            let positive = usize::from(part == BindingPart::Positive);
            Ok([
                [["l_x_minus", "l_x_plus"], ["l_y_minus", "l_y_plus"]],
                [["r_x_minus", "r_x_plus"], ["r_y_minus", "r_y_plus"]],
            ][index as usize][id as usize][positive])
        }
        (4, 0, id, BindingPart::Digital) => Ok(match id {
            2 => "gun_trigger",
            3 => "gun_aux_a",
            4 => "gun_aux_b",
            // RetroArch input_driver_lightgun_id_convert maps legacy PAUSE
            // (5) and START (6) to RARCH_LIGHTGUN_START.
            5 | 6 => "gun_start",
            7 => "gun_select",
            8 => "gun_aux_c",
            9 => "gun_dpad_up",
            10 => "gun_dpad_down",
            11 => "gun_dpad_left",
            12 => "gun_dpad_right",
            16 => "gun_offscreen_shot",
            _ => anyhow::bail!("This lightgun address is not a bindable RetroArch button"),
        }),
        _ => anyhow::bail!("FBNeo address/part requires a different input adapter"),
    }
}

pub(crate) fn binding_parts(address: &InputAddress) -> &'static [BindingPart] {
    match input_encoding(address) {
        InputEncoding::JoypadButton => &[BindingPart::Digital],
        InputEncoding::AnalogButtonPressure => &[BindingPart::Pressure],
        InputEncoding::SignedAxis => &[BindingPart::Negative, BindingPart::Positive],
        InputEncoding::AbsoluteCoordinate if address.device == 1029 => {
            &[BindingPart::Negative, BindingPart::Positive]
        }
        InputEncoding::Button if address.device == 4 => &[BindingPart::Digital],
        _ => &[],
    }
}

/// Linux udev mouse requirements, not RetroPad bindings or a live route.
/// Wheel events need their own discrete-event conversion and are excluded.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub(crate) struct RelativeRequirement {
    pub event_type: u16,
    pub code: u16,
}

impl RelativeRequirement {
    pub(crate) fn matches_saved_device(
        self,
        device: &crate::controller_axis::relative_settings::RelativeDeviceSettings,
    ) -> bool {
        device
            .supports_output(self.event_type, self.code)
            .unwrap_or(false)
    }
}

pub(crate) fn relative_requirement(address: &InputAddress) -> Option<RelativeRequirement> {
    let (event_type, code) = match (address.device, address.index, address.id) {
        (2, 0, id @ 0..=1) => (2, id as u16),
        (2, 0, 2) => (1, 0x110),  // BTN_LEFT
        (2, 0, 3) => (1, 0x111),  // BTN_RIGHT
        (2, 0, 6) => (1, 0x112),  // BTN_MIDDLE
        (2, 0, 9) => (1, 0x113),  // BTN_SIDE / mouse button 4
        (2, 0, 10) => (1, 0x114), // BTN_EXTRA / mouse button 5
        _ => return None,
    };
    Some(RelativeRequirement { event_type, code })
}

/// Display identity only. Wheel controls remain visible even though their
/// conversion is not yet provided by the relative-device adapter.
pub(crate) fn mouse_destination_control(address: &InputAddress) -> Option<&'static str> {
    if address.device != 2 || address.index != 0 {
        return None;
    }
    Some(match address.id {
        0 => "mouse_x",
        1 => "mouse_y",
        2 => "mouse_left",
        3 => "mouse_right",
        4 => "wheel_up",
        5 => "wheel_down",
        6 => "mouse_middle",
        7 => "horizontal_wheel_up",
        8 => "horizontal_wheel_down",
        9 => "mouse_button4",
        10 => "mouse_button5",
        _ => return None,
    })
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct RelativePortReview {
    pub port: u32,
    pub required: Vec<InputAddress>,
    pub unsupported: Vec<InputAddress>,
    pub candidate_devices: Vec<std::path::PathBuf>,
    pub prepared_candidates: Vec<crate::controller_axis::relative_settings::RelativeDeviceSettings>,
}

/// Restrict one saved device to the inspected mouse addresses of one port.
/// The caller must still bind an explicitly selected candidate to fresh native
/// evidence and establish owned frontend routing before opening a transport.
pub(crate) fn prepare_relative_port(
    required: &[InputAddress],
    device: &crate::controller_axis::relative_settings::RelativeDeviceSettings,
) -> Result<crate::controller_axis::relative_settings::RelativeDeviceSettings> {
    use std::collections::BTreeSet;
    ensure!(
        !required.is_empty() && required.len() <= 4096,
        "FBNeo relative port needs a bounded nonempty address set"
    );
    let port = required[0].port;
    ensure!(
        (port as usize) < FRONTEND_PORTS && required.iter().all(|address| address.port == port),
        "FBNeo relative preparation requires one valid frontend port"
    );
    device.validate()?;
    let mut axes = BTreeSet::new();
    let mut buttons = BTreeSet::new();
    for address in required {
        let requirement = relative_requirement(address)
            .context("FBNeo address has no implemented physical-relative conversion")?;
        ensure!(
            device.supports_output(requirement.event_type, requirement.code)?,
            "Saved relative device cannot supply FBNeo mouse address {}:{}:{}:{}",
            address.port,
            address.device,
            address.index,
            address.id
        );
        match requirement.event_type {
            2 => {
                axes.insert(if device.motion.swap_xy {
                    requirement.code ^ 1
                } else {
                    requirement.code
                });
            }
            1 => {
                buttons.insert(requirement.code);
            }
            _ => anyhow::bail!("Unexpected FBNeo relative event type"),
        }
    }
    let mut prepared = device.clone();
    prepared.axes.retain(|axis| axes.contains(axis));
    prepared
        .buttons
        .retain(|(_, output)| buttons.contains(output));
    prepared.validate()?;
    Ok(prepared)
}

/// A candidate must cover the entire mouse part of one frontend port. This
/// does not select a device, assign its live mouse index, or claim launchability.
pub(crate) fn relative_port_reviews(
    targets: &[MappingTarget],
    devices: &[crate::controller_axis::relative_settings::RelativeDeviceSettings],
) -> Vec<RelativePortReview> {
    let ports: std::collections::BTreeSet<_> = targets
        .iter()
        .filter(|target| target.address.device == 2)
        .map(|target| target.address.port)
        .collect();
    ports
        .into_iter()
        .map(|port| {
            let required: Vec<_> = targets
                .iter()
                .filter(|target| target.address.port == port && target.address.device == 2)
                .map(|target| target.address.clone())
                .collect();
            let unsupported: Vec<_> = required
                .iter()
                .filter(|address| relative_requirement(address).is_none())
                .cloned()
                .collect();
            let prepared_candidates: Vec<_> = if unsupported.is_empty() {
                devices
                    .iter()
                    .filter_map(|device| prepare_relative_port(&required, device).ok())
                    .collect()
            } else {
                Vec::new()
            };
            let candidate_devices = prepared_candidates
                .iter()
                .map(|device| device.event_path.clone())
                .collect();
            RelativePortReview {
                port,
                required,
                unsupported,
                candidate_devices,
                prepared_candidates,
            }
        })
        .collect()
}

/// Geometry of the frontend channel, not an inferred game's cabinet button.
/// Reuse the exact binding translation used to render the launch configuration.
pub(crate) fn destination_control(
    address: &InputAddress,
    part: BindingPart,
) -> Option<&'static str> {
    let field = retroarch_field(address, part).ok()?;
    Some(match field {
        "b" => "b",
        "y" => "y",
        "select" => "select",
        "start" => "start",
        "up" => "up",
        "down" => "down",
        "left" => "left",
        "right" => "right",
        "a" => "a",
        "x" => "x",
        "l" => "l",
        "r" => "r",
        "l2" => "l2",
        "r2" => "r2",
        "l3" => "l3",
        "r3" => "r3",
        "l_x_minus" => "stick_left",
        "l_x_plus" => "stick_right",
        "l_y_minus" => "stick_up",
        "l_y_plus" => "stick_down",
        "r_x_minus" => "right_stick_left",
        "r_x_plus" => "right_stick_right",
        "r_y_minus" => "right_stick_up",
        "r_y_plus" => "right_stick_down",
        "gun_trigger" | "gun_offscreen_shot" | "gun_aux_a" | "gun_aux_b" | "gun_aux_c"
        | "gun_start" | "gun_select" | "gun_dpad_up" | "gun_dpad_down" | "gun_dpad_left"
        | "gun_dpad_right" => field,
        _ => return None,
    })
}

pub(crate) fn destination_layout(
    address: &InputAddress,
    part: BindingPart,
) -> Option<&'static str> {
    let control = destination_control(address, part)?;
    Some(if control.starts_with("gun_") {
        "fbneo-lightgun-buttons"
    } else {
        "fbneo-retropad-channels"
    })
}

/// These two native addresses read one RetroArch channel. Preserve their
/// distinct report identities while allowing a single physical assignment.
pub(crate) fn lightgun_start_alias(address: &InputAddress) -> Option<InputAddress> {
    match (address.device, address.index, address.id) {
        (4, 0, id @ (5 | 6)) => Some(InputAddress {
            id: if id == 5 { 6 } else { 5 },
            ..address.clone()
        }),
        _ => None,
    }
}

/// FBNeo advertises Arcade Gun axes as ANALOG, but queries its subclass ID.
/// RetroArch input_state masks the subclass back to ANALOG before dispatch.
pub(crate) fn retroarch_address_alias(address: &InputAddress) -> Option<InputAddress> {
    if let Some(alias) = lightgun_start_alias(address) {
        return Some(alias);
    }
    match (address.device, address.index, address.id) {
        (5 | 1029, 0, 0..=1) => Some(InputAddress {
            device: if address.device == 5 { 1029 } else { 5 },
            ..address.clone()
        }),
        _ => None,
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PhysicalBinding {
    pub(crate) target: InputAddress,
    pub(crate) part: BindingPart,
    pub(crate) input: crate::controller_catalog::NativeInput,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceBinding {
    pub(crate) target: InputAddress,
    pub(crate) part: BindingPart,
    /// Exact control ID in the selected physical calibration, never its label.
    pub(crate) source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeLaunchSettings {
    pub(crate) emulator_id: String,
    pub(crate) helper: std::path::PathBuf,
    pub(crate) core: std::path::PathBuf,
    pub(crate) core_sha256: String,
    pub(crate) content: std::path::PathBuf,
    pub(crate) content_sha256: String,
    pub(crate) library: String,
    pub(crate) dependencies: Vec<std::path::PathBuf>,
    pub(crate) system_files: Vec<std::path::PathBuf>,
    pub(crate) hardware: TopologyHardware,
    pub(crate) driver_players: usize,
    pub(crate) mahjong_keyboards: usize,
    /// Exact reviewed effective settings and input surface. Changes require a
    /// new review, even when the numeric output addresses happen to coincide.
    pub(crate) expected_options: std::collections::BTreeMap<String, String>,
    pub(crate) expected_descriptors: Vec<EffectiveDescriptor>,
    pub(crate) expected_queries: Vec<InputAddress>,
    pub(crate) expected_files: Vec<lunchbox_controller_probe::libretro_input::FirmwareIdentity>,
    pub(crate) players: Vec<SavedPlayer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) relative_sources: Vec<RelativeSourceSelection>,
    /// Explicit native-key to frontend-channel ownership. Empty preserves the
    /// ordinary gamepad path; native keyboard addresses are never rewritten.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) keyboard_bindings: Vec<keyboard::KeyboardBinding>,
    /// Explicit native frontend keyboard input, not a calibrated gamepad or an
    /// exclusively captured physical keyboard. No gamepad channel limit applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) keyboard_passthrough_port: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) absolute_sources: Vec<AbsoluteSourceSelection>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AbsoluteSourceSelection {
    pub port: u32,
    pub device: crate::controller_axis::absolute_settings::AbsoluteDeviceSettings,
}

pub(crate) fn arcade_aim_address(address: &InputAddress) -> bool {
    matches!(
        (address.device, address.index, address.id),
        (5 | 1029, 0, 0 | 1)
    )
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelativeSourceSelection {
    /// Zero-based inspected frontend port, not a host mouse-device index.
    pub(crate) port: u32,
    pub(crate) device: crate::controller_axis::relative_settings::RelativeDeviceSettings,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedPlayer {
    pub(crate) port: u32,
    pub(crate) device: u32,
    pub(crate) controller_id: String,
    pub(crate) assignments: Vec<SourceBinding>,
}

impl NativeLaunchSettings {
    /// Separate validated relative ownership from remaining gamepad/external
    /// requirements without changing the retained inspection evidence.
    pub(crate) fn gamepad_targets(&self) -> Result<Vec<MappingTarget>> {
        let prepared = self.prepared_relative_sources()?;
        let absolute = self.prepared_absolute_sources()?;
        let mut targets = self.keyboard_input_targets()?;
        targets.retain(|target| {
            !(target.address.device == 2
                && prepared.contains_key(&((target.address.port + 1) as u8)))
                && !(absolute.contains_key(&target.address.port)
                    && arcade_aim_address(&target.address))
        });
        if let Some((port, planned, _, _)) = self.keyboard_plan()? {
            targets.retain(|target| target.address.port != port || target.address.device == 2);
            targets.extend(
                planned
                    .into_iter()
                    .filter(|target| target.address.device != 2),
            );
        }
        Ok(targets)
    }

    pub(crate) fn prepared_absolute_sources(
        &self,
    ) -> Result<
        std::collections::BTreeMap<
            u32,
            crate::controller_axis::absolute_settings::AbsoluteDeviceSettings,
        >,
    > {
        ensure!(
            self.absolute_sources.len() <= FRONTEND_PORTS,
            "Too many FBNeo absolute sources"
        );
        let mut result = std::collections::BTreeMap::new();
        for source in &self.absolute_sources {
            ensure!(
                source.port < FRONTEND_PORTS as u32
                    && self
                        .players
                        .iter()
                        .any(|player| player.port == source.port && player.device == 1029),
                "Calibrated absolute aim requires the inspected Arcade Gun device"
            );
            for id in [0, 1] {
                ensure!(
                    self.expected_queries
                        .iter()
                        .any(|address| address.port == source.port
                            && address.device == 1029
                            && address.index == 0
                            && address.id == id),
                    "Arcade Gun did not query both native absolute aim coordinates"
                );
            }
            ensure!(
                self.players
                    .iter()
                    .filter(|player| player.port == source.port)
                    .flat_map(|player| &player.assignments)
                    .all(|assignment| !arcade_aim_address(&assignment.target)),
                "Remove gamepad axis assignments from aim owned by the absolute source"
            );
            source.device.validate()?;
            ensure!(
                result.insert(source.port, source.device.clone()).is_none(),
                "Duplicate absolute aim port"
            );
        }
        crate::controller_axis::absolute_settings::validate_devices(
            &result.values().cloned().collect::<Vec<_>>(),
        )?;
        Ok(result)
    }

    pub(crate) fn prepared_keyboard_passthrough(&self) -> Result<Option<u32>> {
        let Some(port) = self.keyboard_passthrough_port else {
            return Ok(None);
        };
        ensure!(
            port < FRONTEND_PORTS as u32
                && self
                    .players
                    .iter()
                    .any(|player| player.port == port && player.device == 3)
                && self
                    .players
                    .iter()
                    .all(|player| player.device != 3 || player.port == port),
            "Frontend keyboard passthrough requires one explicitly selected native keyboard port"
        );
        let targets =
            mapping_targets_from_records(&self.expected_descriptors, &self.expected_queries)?;
        let keys: Vec<_> = targets
            .iter()
            .filter(|target| target.address.device == 3)
            .collect();
        ensure!(
            !keys.is_empty()
                && keys.iter().all(|key| key.address.port == port
                    && key.address.index == 0
                    && keyboard::valid_key(key.address.id)),
            "Frontend keyboard passthrough requires supported keys on one inspected port"
        );
        ensure!(
            self.players
                .iter()
                .all(|player| player
                    .assignments
                    .iter()
                    .all(|assignment| assignment.target.device != 3
                        || self
                            .keyboard_bindings
                            .iter()
                            .any(|key| key.target == assignment.target))),
            "Calibrated keyboard assignments require explicit frontend channels"
        );
        Ok(Some(port))
    }

    /// Native inputs still needing calibrated transport. Keys absent from the
    /// frontend's physical key table stay required even in passthrough mode.
    pub(crate) fn keyboard_input_targets(&self) -> Result<Vec<MappingTarget>> {
        let port = self.prepared_keyboard_passthrough()?;
        let mut targets =
            mapping_targets_from_records(&self.expected_descriptors, &self.expected_queries)?;
        targets.retain(|target| {
            !(target.address.device == 3
                && port == Some(target.address.port)
                && keyboard::udev_passthrough_key(target.address.id)
                && !self
                    .keyboard_bindings
                    .iter()
                    .any(|key| key.target == target.address))
        });
        Ok(targets)
    }

    /// Ports fully supplied by explicit non-gamepad input paths. A native
    /// keyboard must not require a dummy connected/calibrated gamepad.
    pub(crate) fn gamepad_free_ports(&self) -> Result<std::collections::BTreeSet<u32>> {
        let native =
            mapping_targets_from_records(&self.expected_descriptors, &self.expected_queries)?;
        let remaining = self.gamepad_targets()?;
        Ok(self
            .players
            .iter()
            .filter(|player| {
                native
                    .iter()
                    .any(|target| target.address.port == player.port)
                    && !remaining
                        .iter()
                        .any(|target| target.address.port == player.port)
            })
            .map(|player| player.port)
            .collect())
    }

    pub(crate) fn keyboard_plan(
        &self,
    ) -> Result<Option<(u32, Vec<MappingTarget>, Vec<SourceBinding>, String)>> {
        let Some(first) = self.keyboard_bindings.first() else {
            return Ok(None);
        };
        let port = first.target.port;
        let player = self
            .players
            .iter()
            .find(|player| player.port == port)
            .ok_or_else(|| anyhow::anyhow!("Keyboard mapping has no selected player"))?;
        let targets = self.keyboard_input_targets()?;
        let devices = self
            .players
            .iter()
            .map(|player| (player.port, player.device))
            .collect();
        let (targets, assignments, remap) = keyboard::physical_plan(
            &targets,
            port,
            &self.keyboard_bindings,
            &player.assignments,
            &devices,
        )?;
        keyboard::relative_remap_path(&self.library)?;
        Ok(Some((port, targets, assignments, remap)))
    }

    pub(crate) fn physical_assignments(&self, player: &SavedPlayer) -> Result<Vec<SourceBinding>> {
        if let Some((port, _, assignments, _)) = self.keyboard_plan()? {
            if port == player.port {
                return Ok(assignments);
            }
        }
        Ok(player.assignments.clone())
    }

    /// Ports whose complete inspected input surface belongs to a prepared
    /// relative device. Empty or mixed/unknown surfaces never qualify.
    pub(crate) fn relative_only_ports(&self) -> Result<std::collections::BTreeSet<u32>> {
        let prepared = self.prepared_relative_sources()?;
        if prepared.is_empty() {
            return Ok(Default::default());
        }
        let targets =
            mapping_targets_from_records(&self.expected_descriptors, &self.expected_queries)?;
        Ok(prepared
            .keys()
            .map(|player| u32::from(*player) - 1)
            .filter(|port| {
                targets.iter().any(|target| target.address.port == *port)
                    && targets
                        .iter()
                        .filter(|target| target.address.port == *port)
                        .all(|target| relative_requirement(&target.address).is_some())
            })
            .collect())
    }

    /// Prepare selected ports only. Unselected mouse requirements remain
    /// unresolved; this does not certify complete game input coverage.
    pub(crate) fn prepared_relative_sources(
        &self,
    ) -> Result<
        std::collections::BTreeMap<
            u8,
            crate::controller_axis::relative_settings::RelativeDeviceSettings,
        >,
    > {
        ensure!(
            self.relative_sources.len() <= FRONTEND_PORTS,
            "Too many FBNeo relative source selections"
        );
        if self.relative_sources.is_empty() {
            return Ok(Default::default());
        }
        let targets =
            mapping_targets_from_records(&self.expected_descriptors, &self.expected_queries)?;
        let mut prepared = std::collections::BTreeMap::new();
        for source in &self.relative_sources {
            ensure!(
                self.players.iter().any(|player| player.port == source.port),
                "FBNeo relative source belongs to an unselected port"
            );
            let required: Vec<_> = targets
                .iter()
                .filter(|target| target.address.port == source.port && target.address.device == 2)
                .map(|target| target.address.clone())
                .collect();
            let device = prepare_relative_port(&required, &source.device)?;
            // The shared transport uses one-based player keys. Never derive
            // this key from event-node numbering or a frontend mouse index.
            let player = u8::try_from(source.port + 1)?;
            ensure!(
                prepared.insert(player, device).is_none(),
                "Duplicate FBNeo relative source port"
            );
        }
        crate::controller_axis::relative_settings::validate_devices(
            &prepared.values().cloned().collect::<Vec<_>>(),
        )?;
        Ok(prepared)
    }

    pub(crate) fn identity_key(&self) -> String {
        serde_json::to_string(&(&self.emulator_id, &self.core, &self.content))
            .expect("Validated FBNeo setup identity serializes")
    }
    pub(crate) fn validate(&self) -> Result<ControllerTopology> {
        ensure!(
            !self.emulator_id.is_empty() && self.emulator_id.len() <= 1024,
            "Invalid FBNeo emulator identity"
        );
        for hash in [&self.core_sha256, &self.content_sha256] {
            ensure!(
                hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
                "Invalid FBNeo saved SHA256"
            );
        }
        let topology =
            controller_topology(self.hardware, self.driver_players, self.mahjong_keyboards)?;
        ensure!(
            self.players.len() == topology.advertised.len(),
            "FBNeo saved setup must select every advertised port"
        );
        let mut ports = std::collections::BTreeSet::new();
        let mut controllers = std::collections::BTreeSet::new();
        let gamepad_free_ports = self.gamepad_free_ports()?;
        for player in &self.players {
            ensure!(
                ports.insert(player.port)
                    && (player.port as usize) < topology.advertised.len()
                    && topology.advertised[player.port as usize]
                        .iter()
                        .any(|device| device.libretro_id() == player.device),
                "Invalid or duplicate FBNeo saved port/device"
            );
            if player.controller_id.is_empty() {
                ensure!(
                    gamepad_free_ports.contains(&player.port) && player.assignments.is_empty(),
                    "FBNeo port without a gamepad needs complete prepared relative/keyboard input and no gamepad assignments"
                );
            } else {
                ensure!(
                    player.controller_id.len() <= 4096 && controllers.insert(&player.controller_id),
                    "Invalid or duplicate FBNeo physical controller selection"
                );
                ensure!(
                    !player.assignments.is_empty(),
                    "FBNeo gamepad has no source assignments"
                );
            }
            ensure!(
                player.assignments.len() <= 8192,
                "Invalid FBNeo assignment count"
            );
            let mut slots = std::collections::BTreeSet::new();
            for assignment in &player.assignments {
                ensure!(
                    assignment.target.port == player.port
                        && slots.insert((assignment.target.clone(), assignment.part))
                        && !assignment.source.is_empty()
                        && assignment.source.len() <= 256,
                    "Invalid FBNeo source assignment"
                );
                if assignment.target.device == 3 {
                    ensure!(
                        assignment.part == BindingPart::Digital
                            && self
                                .keyboard_bindings
                                .iter()
                                .any(|key| key.target == assignment.target),
                        "Native keyboard assignment needs an explicit keyboard channel"
                    );
                } else {
                    retroarch_field(&assignment.target, assignment.part)?;
                }
            }
        }
        ensure!(
            !self.expected_descriptors.is_empty()
                && !self.expected_queries.is_empty()
                && self.expected_queries.len() <= 4096,
            "Missing reviewed FBNeo input contract"
        );
        descriptor_groups(&self.expected_descriptors)?;
        let mut queries = std::collections::BTreeSet::new();
        for query in &self.expected_queries {
            ensure!(
                query.port < 16 && queries.insert(query),
                "Invalid reviewed FBNeo query address"
            );
        }
        self.prepared_relative_sources()?;
        self.keyboard_plan()?;
        lunchbox_controller_probe::libretro_options::OptionRegistry::new(
            self.expected_options.clone(),
        )?;
        ensure!(
            !self.expected_files.is_empty() && self.expected_files.len() <= 513,
            "Missing or oversized FBNeo reviewed file manifest"
        );
        let mut files = std::collections::BTreeSet::new();
        for file in &self.expected_files {
            ensure!(
                files.insert(&file.filename)
                    && file.filename.len() <= 32768
                    && !file.filename.chars().any(char::is_control)
                    && file.sha256.len() == 64
                    && file.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
                "Invalid reviewed FBNeo input file identity"
            );
        }
        // Reuse the helper's path/identity/option/device schema constraints.
        lunchbox_controller_probe::content_inspection::Request {
            schema_version: 1,
            core: self.core.clone(),
            core_sha256: self.core_sha256.clone(),
            core_name: self.library.clone(),
            content: self.content.clone(),
            content_dependencies: self.dependencies.clone(),
            system_files: self.system_files.clone(),
            options: Vec::new(),
            devices: self
                .players
                .iter()
                .map(
                    |p| lunchbox_controller_probe::content_inspection::DeviceSelection {
                        port: p.port,
                        device: p.device,
                    },
                )
                .collect(),
        }
        .validate()?;
        ensure!(
            self.helper.is_absolute(),
            "FBNeo helper must have an absolute path"
        );
        Ok(topology)
    }

    pub(crate) fn validate_review(
        &self,
        report: &lunchbox_controller_probe::libretro_input::ContentControllerReport,
    ) -> Result<()> {
        let file_map = |files: &[lunchbox_controller_probe::libretro_input::FirmwareIdentity]| {
            files
                .iter()
                .map(|file| {
                    (
                        file.filename.clone(),
                        (file.bytes, file.sha256.to_ascii_lowercase()),
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        ensure!(
            report.staged_files.len() == self.expected_files.len()
                && file_map(&report.staged_files) == file_map(&self.expected_files),
            "FBNeo content or dependency files changed since mapping review"
        );
        ensure!(
            report
                .core
                .core_sha256
                .eq_ignore_ascii_case(&self.core_sha256)
                && report.effective_options == self.expected_options,
            "FBNeo core/options changed since mapping review"
        );
        let observed = report
            .input_descriptors
            .iter()
            .cloned()
            .map(EffectiveDescriptor::from)
            .collect::<Vec<_>>();
        ensure!(
            descriptor_groups(&observed)? == descriptor_groups(&self.expected_descriptors)?,
            "FBNeo descriptors changed since mapping review"
        );
        let observed = report
            .input_queries
            .iter()
            .map(|q| InputAddress {
                port: q.port,
                device: q.device,
                index: q.index,
                id: q.id,
            })
            .collect::<std::collections::BTreeSet<_>>();
        ensure!(
            observed == self.expected_queries.iter().cloned().collect(),
            "FBNeo input queries changed since mapping review"
        );
        Ok(())
    }
}

/// Extra identities needed to associate a native report with application
/// settings. Driver topology remains explicit rather than inferred from labels.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionImportContext {
    pub(crate) emulator_id: String,
    pub(crate) helper: std::path::PathBuf,
    pub(crate) hardware: TopologyHardware,
    pub(crate) driver_players: usize,
    pub(crate) mahjong_keyboards: usize,
    #[serde(deserialize_with = "unique_import_controllers")]
    pub(crate) controllers: std::collections::BTreeMap<u32, String>,
}

fn unique_import_controllers<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<std::collections::BTreeMap<u32, String>, D::Error> {
    struct Ports;
    impl<'de> serde::de::Visitor<'de> for Ports {
        type Value = std::collections::BTreeMap<u32, String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("unique FBNeo port numbers mapped to physical controller IDs")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut ports = std::collections::BTreeMap::new();
            while let Some((port, controller)) = map.next_entry::<u32, String>()? {
                if port as usize >= FRONTEND_PORTS || ports.insert(port, controller).is_some() {
                    return Err(serde::de::Error::custom(
                        "Duplicate or out-of-range FBNeo import port",
                    ));
                }
            }
            Ok(ports)
        }
    }
    deserializer.deserialize_map(Ports)
}

/// Import on a background task: report validation rehashes actual source files.
/// Returned assignments are empty and deliberately cannot be saved/launched
/// until the user completes them. No core or input device is executed here.
pub(crate) fn import_inspection(
    request: &lunchbox_controller_probe::content_inspection::Request,
    report_bytes: &[u8],
    context: InspectionImportContext,
) -> Result<NativeLaunchSettings> {
    ensure!(
        !context.emulator_id.is_empty()
            && context.emulator_id.len() <= 1024
            && context.helper.is_absolute(),
        "Invalid FBNeo import application/runtime identity"
    );
    let topology = controller_topology(
        context.hardware,
        context.driver_players,
        context.mahjong_keyboards,
    )?;
    let report = parse_inspection_report(request, &topology, report_bytes)?;
    ensure!(
        context
            .controllers
            .keys()
            .eq(report.requested_devices.keys()),
        "Select an exact physical controller for every imported port"
    );
    let mut selected = std::collections::BTreeSet::new();
    for id in context.controllers.values() {
        ensure!(
            !id.is_empty() && id.len() <= 4096 && selected.insert(id),
            "Invalid or duplicate physical controller in FBNeo import"
        );
    }
    let primary_name = format!("content/{}", report.content_filename);
    let content_sha256 = report
        .staged_files
        .iter()
        .find(|file| file.filename == primary_name)
        .context("Imported FBNeo report has no primary content hash")?
        .sha256
        .clone();
    let players = report
        .requested_devices
        .iter()
        .map(|(&port, &device)| SavedPlayer {
            port,
            device,
            controller_id: context.controllers[&port].clone(),
            assignments: Vec::new(),
        })
        .collect();
    Ok(NativeLaunchSettings {
        emulator_id: context.emulator_id,
        helper: context.helper,
        core: request.core.canonicalize()?,
        core_sha256: report.core.core_sha256,
        content: request.content.canonicalize()?,
        content_sha256,
        library: report.core.core_name,
        dependencies: request
            .content_dependencies
            .iter()
            .map(|path| path.canonicalize())
            .collect::<std::io::Result<_>>()?,
        system_files: request
            .system_files
            .iter()
            .map(|path| path.canonicalize())
            .collect::<std::io::Result<_>>()?,
        hardware: context.hardware,
        driver_players: context.driver_players,
        mahjong_keyboards: context.mahjong_keyboards,
        expected_options: report.effective_options,
        expected_descriptors: report
            .input_descriptors
            .into_iter()
            .map(EffectiveDescriptor::from)
            .collect(),
        expected_queries: report
            .input_queries
            .into_iter()
            .map(|q| InputAddress {
                port: q.port,
                device: q.device,
                index: q.index,
                id: q.id,
            })
            .collect(),
        expected_files: report.staged_files,
        players,
        relative_sources: Vec::new(),
        keyboard_bindings: Vec::new(),
        keyboard_passthrough_port: None,
        absolute_sources: Vec::new(),
    })
}

#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct MappingTarget {
    pub(crate) id: String,
    pub(crate) address: InputAddress,
    pub(crate) encoding: InputEncoding,
    pub(crate) descriptions: Vec<String>,
    /// Analog pressure is labeled through a joypad descriptor in FBNeo.
    /// Keep the origin explicit rather than merging the two callback addresses.
    pub(crate) label_source: Option<InputAddress>,
    pub(crate) queried_during_idle_frame: bool,
}

fn input_encoding(address: &InputAddress) -> InputEncoding {
    use InputEncoding::*;
    match (address.device, address.index, address.id) {
        (1, 0, 0..=15) => JoypadButton,
        (3, 0, _) => KeyboardKey,
        (5, 0..=1, 0..=1) => SignedAxis,
        (5, 2, 0..=15) => AnalogButtonPressure,
        (2, 0, 0..=1) | (4, 0, 0..=1) => RelativeAxis,
        (2, 0, 2 | 3 | 6 | 9 | 10) => Button,
        (2, 0, 4 | 5 | 7 | 8) => WheelEvent,
        (4, 0, 13 | 14) | (6 | 262 | 1029, 0, 0..=1) => AbsoluteCoordinate,
        (4 | 6, 0, 15) => OffscreenStatus,
        (4, 0, 2..=12 | 16) | (6, 0, 2) => Button,
        (6, 0, 3) => TouchCount,
        _ => Unknown,
    }
}

/// Build the union of advertised and observed input addresses. Query-only
/// inputs remain visible without invented driver labels. Unqueried descriptors
/// remain visible without a claim that every execution path was exercised.
/// This does not establish completeness: touch-event labels and conditional
/// input paths still require native driver contracts.
pub(crate) fn mapping_targets(
    report: &lunchbox_controller_probe::libretro_input::ContentControllerReport,
) -> Result<Vec<MappingTarget>> {
    let descriptors = report
        .input_descriptors
        .iter()
        .cloned()
        .map(EffectiveDescriptor::from)
        .collect::<Vec<_>>();
    let queries = report
        .input_queries
        .iter()
        .map(|query| InputAddress {
            port: query.port,
            device: query.device,
            index: query.index,
            id: query.id,
        })
        .collect::<Vec<_>>();
    mapping_targets_from_records(&descriptors, &queries)
}

/// The reviewed settings editor shares target construction with live reports;
/// it must not fabricate a runtime report to preview stored input records.
pub(crate) fn mapping_targets_from_records(
    descriptors: &[EffectiveDescriptor],
    queries: &[InputAddress],
) -> Result<Vec<MappingTarget>> {
    let groups = descriptor_groups(descriptors)?;
    ensure!(
        queries.len() <= 4096,
        "FBNeo query snapshot exceeds its limit"
    );
    let mut queried = std::collections::BTreeSet::new();
    for query in queries {
        ensure!(query.port < 16, "FBNeo query port exceeds its limit");
        ensure!(
            queried.insert(InputAddress {
                port: query.port,
                device: query.device,
                index: query.index,
                id: query.id,
            }),
            "Duplicate FBNeo query address"
        );
    }
    let addresses = groups
        .keys()
        .cloned()
        .chain(queried.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    Ok(addresses
        .into_iter()
        .map(|address| {
            let mut descriptions = groups.get(&address).cloned().unwrap_or_default();
            let mut label_source = None;
            let encoding = input_encoding(&address);
            if descriptions.is_empty() && encoding == InputEncoding::AnalogButtonPressure {
                let source = InputAddress {
                    port: address.port,
                    device: 1,
                    index: 0,
                    id: address.id,
                };
                if let Some(labels) = groups.get(&source) {
                    descriptions = labels.clone();
                    label_source = Some(source);
                }
            }
            MappingTarget {
                id: format!(
                    "p{}_d{}_i{}_id{}",
                    address.port, address.device, address.index, address.id
                ),
                queried_during_idle_frame: queried.contains(&address),
                address,
                encoding,
                descriptions,
                label_source,
            }
        })
        .collect())
}

impl From<lunchbox_controller_probe::libretro_input::InputDescriptor> for EffectiveDescriptor {
    fn from(value: lunchbox_controller_probe::libretro_input::InputDescriptor) -> Self {
        Self {
            address: InputAddress {
                port: value.port,
                device: value.device,
                index: value.index,
                id: value.id,
            },
            description: value.description,
        }
    }
}

/// Decode the helper's input_descriptors array, not an entire report. The
/// caller must separately establish report/core/content/device provenance.
pub(crate) fn parse_descriptor_records(bytes: &[u8]) -> Result<Vec<EffectiveDescriptor>> {
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "FBNeo descriptor payload exceeds its byte limit"
    );
    let records: Vec<lunchbox_controller_probe::libretro_input::InputDescriptor> =
        serde_json::from_slice(bytes)?;
    let descriptors = records
        .into_iter()
        .map(EffectiveDescriptor::from)
        .collect::<Vec<_>>();
    descriptor_groups(&descriptors)?;
    Ok(descriptors)
}

/// Accept a complete inspection report only after request/file provenance and
/// the source-derived driver topology both agree. This still does not prove
/// successful native device activation or end-to-end physical input behavior.
pub(crate) fn parse_inspection_report(
    request: &lunchbox_controller_probe::content_inspection::Request,
    topology: &ControllerTopology,
    bytes: &[u8],
) -> Result<lunchbox_controller_probe::libretro_input::ContentControllerReport> {
    let report = request.parse_report(bytes)?;
    ensure!(
        report.requested_devices.len() == topology.advertised.len(),
        "FBNeo inspection must select every advertised port"
    );
    let requested = (0..topology.advertised.len())
        .map(|port| {
            report
                .requested_devices
                .get(&(port as u32))
                .copied()
                .ok_or_else(|| anyhow::anyhow!("FBNeo inspection is missing a port selection"))
        })
        .collect::<Result<Vec<_>>>()?;
    validate_advertised_selection(topology, &report.controller_choices, &requested)?;
    let descriptors = report
        .input_descriptors
        .iter()
        .cloned()
        .map(EffectiveDescriptor::from)
        .collect::<Vec<_>>();
    descriptor_groups(&descriptors)?;
    Ok(report)
}

/// Descriptors describe effective virtual inputs after native routing, unlike
/// BurnInputInfo's pre-routing labels. Preserve shared-address labels so the UI
/// cannot silently present simultaneous actions as independently assignable.
pub(crate) fn descriptor_groups(
    descriptors: &[EffectiveDescriptor],
) -> Result<std::collections::BTreeMap<InputAddress, Vec<String>>> {
    ensure!(
        descriptors.len() <= 4096,
        "FBNeo descriptor snapshot exceeds its entry limit"
    );
    let mut groups = std::collections::BTreeMap::<InputAddress, Vec<String>>::new();
    for descriptor in descriptors {
        ensure!(
            descriptor.address.port < 16,
            "FBNeo descriptor port exceeds frontend capacity"
        );
        ensure!(
            !descriptor.description.is_empty()
                && descriptor.description.len() <= 1024
                && !descriptor.description.chars().any(char::is_control),
            "FBNeo input descriptor has an invalid display label"
        );
        // Keep keyboard/mouse/gun/analog addresses distinct. No cast to a pad
        // ID, fuzzy label merge, or inferred device mode occurs at this layer.
        groups
            .entry(descriptor.address.clone())
            .or_default()
            .push(descriptor.description.clone());
    }
    Ok(groups)
}

pub(crate) const FRONTEND_PORTS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TopologyHardware {
    MsxOrSpectrum,
    Nes,
    Other,
}

pub(crate) struct ControllerTopology {
    pub(crate) player_ports: usize,
    pub(crate) native_controller_slots: usize,
    pub(crate) advertised: Vec<Vec<Device>>,
}

/// Check a captured native choice list against driver-derived expectations.
/// This checks advertised identity, not successful post-selection activation;
/// refreshed descriptors and core/content/options provenance remain required.
pub(crate) fn validate_advertised_selection(
    topology: &ControllerTopology,
    observed: &[Vec<lunchbox_controller_probe::libretro_input::ControllerChoice>],
    requested: &[u32],
) -> Result<()> {
    ensure!(
        observed.len() == topology.advertised.len()
            && observed.len() <= 16
            && requested.len() == observed.len(),
        "FBNeo advertised controller topology changed or selection is incomplete"
    );
    for (port, (choices, expected)) in observed.iter().zip(&topology.advertised).enumerate() {
        ensure!(
            choices.len() <= 64,
            "FBNeo advertised device list exceeds its limit"
        );
        let mut unique = std::collections::BTreeSet::new();
        for choice in choices {
            ensure!(
                !choice.description.is_empty()
                    && choice.description.len() <= 1024
                    && !choice.description.chars().any(char::is_control),
                "FBNeo advertised device has an invalid display label"
            );
            ensure!(
                unique.insert(choice.id),
                "FBNeo advertises duplicate device identities on port {}",
                port + 1
            );
        }
        let expected_ids = expected
            .iter()
            .map(|device| device.libretro_id())
            .collect::<Vec<_>>();
        let observed_ids = choices.iter().map(|choice| choice.id).collect::<Vec<_>>();
        ensure!(
            observed_ids == expected_ids,
            "FBNeo advertised choices differ from the selected driver contract on port {}",
            port + 1
        );
        ensure!(
            unique.contains(&requested[port]),
            "FBNeo requested device is not advertised on port {}",
            port + 1
        );
    }
    Ok(())
}

/// SetControllerInfo's advertised choices, separate from internal slot count.
/// In this revision the standard branch terminates at nMaxPlayers, hiding its
/// appended Mahjong keyboard entries. Do not infer working ports from allocation.
pub(crate) fn controller_topology(
    hardware: TopologyHardware,
    players: usize,
    mahjong_keyboards: usize,
) -> Result<ControllerTopology> {
    ensure!(
        (1..=FRONTEND_PORTS).contains(&players) && mahjong_keyboards <= FRONTEND_PORTS,
        "FBNeo native player/keyboard counts exceed supported limits"
    );
    if hardware == TopologyHardware::MsxOrSpectrum {
        return Ok(ControllerTopology {
            player_ports: players,
            native_controller_slots: 3,
            advertised: vec![
                vec![Device::Joystick],
                vec![Device::Joystick],
                vec![Device::Keyboard],
            ],
        });
    }
    let players = if hardware == TopologyHardware::Nes {
        players.max(2)
    } else {
        players
    };
    let choices = vec![
        Device::Classic,
        Device::Modern,
        Device::SixButtonPanel,
        Device::MouseBall,
        Device::FullMouse,
        Device::Pointer,
        Device::Touchscreen,
        Device::Lightgun,
        Device::ArcadeGun,
    ];
    Ok(ControllerTopology {
        player_ports: players,
        native_controller_slots: players + mahjong_keyboards,
        advertised: vec![choices; players],
    })
}

pub(crate) struct InputDescription<'a> {
    pub(crate) name: &'a str,
    pub(crate) info: &'a str,
}

/// AnalyzeGameLayout's attribution is intentionally asymmetric: a P2..P6
/// name wins, while a P1 name can be replaced by the info field's player.
pub(crate) fn metadata_player(input: &InputDescription<'_>) -> Option<usize> {
    let prefix = |value: &str, insensitive: bool| -> Option<usize> {
        let bytes = value.as_bytes();
        let first = *bytes.first()?;
        let number = *bytes.get(1)?;
        ((first == b'P' || (insensitive && first == b'p')) && (b'1'..=b'6').contains(&number))
            .then(|| usize::from(number - b'1'))
    };
    let name = prefix(input.name, false);
    let info = prefix(input.info, true);
    match name {
        Some(1..=5) => name,
        _ => info.or(name),
    }
}

pub(crate) struct LayoutAnalysis {
    pub(crate) player_axes: [usize; FRONTEND_PORTS],
    pub(crate) player_one_fire_inputs: usize,
    pub(crate) street_fighter: bool,
}

/// Inputs and the hardware/real-button facts must come from the selected
/// native driver. This is not title-based classification or a complete macro
/// inventory; specialized driver paths still need independent resolution.
pub(crate) fn analyze_standard_layout(
    inputs: &[InputDescription<'_>],
    cps2_hardware: bool,
    real_fire_buttons: usize,
) -> LayoutAnalysis {
    let mut analysis = LayoutAnalysis {
        player_axes: [0; FRONTEND_PORTS],
        player_one_fire_inputs: 0,
        street_fighter: false,
    };
    let mut punches = 0u8;
    let mut kicks = 0u8;
    for input in inputs {
        let Some(player) = metadata_player(input) else {
            continue;
        };
        let axis = match input.info.len() {
            9 => input.info.get(4..),
            12 => input.info.get(7..),
            _ => None,
        }
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case("-axis"));
        if axis {
            analysis.player_axes[player] += 1;
        }
        if player != 0 {
            continue;
        }
        if input
            .info
            .get(2..)
            .is_some_and(|info| info.starts_with(" fire"))
        {
            analysis.player_one_fire_inputs += 1;
        }
        let name = input.name.get(2..).unwrap_or_default();
        for (label, mask) in [(" Weak", 1), (" Medium", 2), (" Strong", 4)] {
            if name.eq_ignore_ascii_case(&format!("{label} Punch")) {
                punches |= mask;
            }
            if name.eq_ignore_ascii_case(&format!("{label} Kick")) {
                kicks |= mask;
            }
        }
    }
    analysis.street_fighter =
        (punches == 7 && kicks == 7) || (cps2_hardware && real_fire_buttons >= 5);
    analysis
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardInput {
    Joypad(u32),
    Analog { index: u32, axis: u32 },
}

/// Resolve only GameInpStandardOne's non-fire inputs after the native player
/// prefix is removed. Preserve case-sensitive prefix matching from the core.
pub(crate) fn standard_nonfire_input(
    info: &str,
    player_axis_count: usize,
) -> Option<StandardInput> {
    for (prefix, button) in [
        ("select", 2),
        ("coin", 2),
        ("start", 3),
        ("up", 4),
        ("down", 5),
        ("left", 6),
        ("right", 7),
    ] {
        if info.starts_with(prefix) {
            return Some(StandardInput::Joypad(button));
        }
    }
    if info.starts_with("x-axis") {
        Some(StandardInput::Analog { index: 0, axis: 0 })
    } else if info.starts_with("y-axis") {
        Some(StandardInput::Analog { index: 0, axis: 1 })
    } else if info.starts_with("z-axis") {
        Some(if player_axis_count == 1 {
            StandardInput::Analog { index: 0, axis: 0 }
        } else {
            StandardInput::Analog { index: 1, axis: 1 }
        })
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Device {
    Joystick,
    Keyboard,
    Lightgun,
    Pointer,
    Classic,
    SixButtonPanel,
    Modern,
    MouseBall,
    ArcadeGun,
    FullMouse,
    Touchscreen,
}

impl Device {
    /// Numbered FIRE01..FIRE10 fallback only. Driver-specific fighting-game,
    /// Neo Geo and other positional mappings take precedence in native setup.
    pub(crate) fn generic_fire_buttons(self) -> Result<[u32; 10]> {
        Ok(match self {
            // libretro joypad IDs: B=0, Y=1, A=8, X=9, L/R=10/11,
            // L2/R2=12/13. These are virtual outputs, not host button numbers.
            Self::Classic => [0, 8, 1, 9, 11, 10, 13, 12, 15, 14],
            Self::Modern => [0, 8, 1, 9, 13, 11, 12, 10, 15, 14],
            Self::SixButtonPanel => [1, 9, 10, 0, 8, 11, 13, 12, 15, 14],
            _ => anyhow::bail!("FBNeo peripheral button routing needs its own native contract"),
        })
    }

    /// GameInpStandardOne's fire branch. Inputs outside the selected native
    /// branch remain unmapped; never fill them using another branch's fallback.
    pub(crate) fn standard_fire_button(
        self,
        number: usize,
        neo_geo: bool,
        street_fighter_layout: bool,
    ) -> Result<Option<u32>> {
        let generic = self.generic_fire_buttons()?;
        let Some(index) = number.checked_sub(1) else {
            return Ok(None);
        };
        if neo_geo && self == Self::Modern {
            return Ok([1, 0, 9, 8].get(index).copied());
        }
        if street_fighter_layout {
            let (top, bottom) = self.positional_buttons()?;
            return Ok([top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]]
                .get(index)
                .copied());
        }
        Ok(generic.get(index).copied())
    }

    /// Native positional columns (top row, bottom row), independently of the
    /// numbered fire fallback. A driver decides when position is significant.
    pub(crate) fn positional_buttons(self) -> Result<([u32; 4], [u32; 4])> {
        Ok(match self {
            Self::Classic | Self::SixButtonPanel => ([1, 9, 10, 12], [0, 8, 11, 13]),
            Self::Modern => ([1, 9, 11, 10], [0, 8, 13, 12]),
            _ => anyhow::bail!("FBNeo positional button routing requires a pad device"),
        })
    }

    pub(crate) fn three_button_line(self) -> Result<[u32; 3]> {
        Ok(match self {
            Self::Classic | Self::Modern => [1, 0, 8],
            Self::SixButtonPanel => [1, 9, 10],
            _ => anyhow::bail!("FBNeo three-button line routing requires a pad device"),
        })
    }

    pub(crate) fn libretro_id(self) -> u32 {
        match self {
            Self::Joystick => 1,
            Self::Keyboard => 3,
            Self::Lightgun => 4,
            Self::Pointer => 6,
            Self::Classic => 5,
            Self::SixButtonPanel => 261,
            Self::Modern => 517,
            Self::MouseBall => 773,
            Self::ArcadeGun => 1029,
            Self::FullMouse => 514,
            Self::Touchscreen => 262,
        }
    }

    pub(crate) fn from_libretro_id(id: u32) -> Result<Self> {
        Ok(match id {
            1 => Self::Joystick,
            3 => Self::Keyboard,
            4 => Self::Lightgun,
            6 => Self::Pointer,
            5 => Self::Classic,
            261 => Self::SixButtonPanel,
            517 => Self::Modern,
            773 => Self::MouseBall,
            1029 => Self::ArcadeGun,
            514 => Self::FullMouse,
            262 => Self::Touchscreen,
            _ => anyhow::bail!("Unknown FBNeo fixed input-device identity: {id}"),
        })
    }
}

/// Coercion performed after a driver is active. Actual storage also requires
/// port < nMaxControllers; the inspection layer must check that separately.
pub(crate) fn loaded_device_id(requested: u32, port: usize, msx_or_spectrum: bool) -> u32 {
    if msx_or_spectrum {
        return match port {
            0 | 1 => 1,
            2 => 3,
            _ => requested,
        };
    }
    match requested {
        5 | 261 | 517 | 773 | 514 | 6 | 262 | 4 | 1029 => requested,
        _ => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fbneo_transport_fields_keep_device_and_part_contracts_distinct() {
        let joypad = InputAddress {
            port: 0,
            device: 1,
            index: 0,
            id: 8,
        };
        assert_eq!(retroarch_field(&joypad, BindingPart::Digital).unwrap(), "a");
        assert_eq!(binding_parts(&joypad), &[BindingPart::Digital]);
        let axis = InputAddress {
            port: 0,
            device: 5,
            index: 1,
            id: 0,
        };
        assert_eq!(
            binding_parts(&axis),
            &[BindingPart::Negative, BindingPart::Positive]
        );
        assert_eq!(loaded_device_id(999, 2, true), 3);
    }

    #[test]
    fn fbneo_unknown_transport_addresses_are_not_guessed() {
        let unknown = InputAddress {
            port: 0,
            device: 999,
            index: 0,
            id: 0,
        };
        assert!(retroarch_field(&unknown, BindingPart::Digital).is_err());
        assert!(binding_parts(&unknown).is_empty());
        assert_eq!(loaded_device_id(999, 0, false), 5);
    }
}
