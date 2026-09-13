//! Gopher64 native JSON input profiles, not Mupen64Plus/libretro mappings.
//!
//! Functional contract pinned to gopher64/gopher64
//! 0bb9fbba638f5cebe3b8a3c1c245abcfec8b0132 (v1.1.36-12):
//! - `src/ui/config.rs` serializes `config.json`; `InputProfile.inputs` is
//!   nineteen pairs of optional externally tagged `InputItem` values.
//! - `src/ui/input_profile.rs` fixes indices 0..17 as D-pad right/left/down/up,
//!   Start, Z, B, A, C right/left/down/up, R, L and stick right/left/down/up;
//!   index 18 is the hotkey activator.
//! - `src/ui/input.rs` opens the exact path stored in `controller_assignment`,
//!   chooses SDL3 gamepad input when `dinput` is false, and selects one named
//!   profile per port through `input_profile_binding`.
//! - `src/ui.rs` uses `$XDG_CONFIG_HOME/gopher64/config.json` separately from
//!   `$XDG_DATA_HOME/gopher64/{saves,states}`. A private config home therefore
//!   leaves native saves and states in their ordinary persistent data root.
//! - a `portable.txt` beside the executable overrides XDG roots, so portable
//!   mode is rejected rather than touching the user's live configuration.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

#[cfg(target_os = "linux")]
use crate::controller_pcsx2::sdl::{AxisRange, Input as SdlInput};

pub(crate) const PROFILE_ID: &str = "gopher64:standalone-n64";

/// N64 target control -> Gopher64's `InputProfile.inputs` index.
pub(crate) const CONTROLS: [(&str, usize); 18] = [
    ("right", 0),
    ("left", 1),
    ("down", 2),
    ("up", 3),
    ("start", 4),
    ("z", 5),
    ("b", 6),
    ("a", 7),
    ("c_right", 8),
    ("c_left", 9),
    ("c_down", 10),
    ("c_up", 11),
    ("r", 12),
    ("l", 13),
    ("stick_right", 14),
    ("stick_left", 15),
    ("stick_down", 16),
    ("stick_up", 17),
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum MappedInput {
    Button(u8),
    Axis { index: u8, positive: bool },
}

/// The pinned Gopher64 input schema.  `Config::new` first tries the complete
/// config and then falls back to deserializing just `input`; either path must
/// accept the patched input or Gopher64 silently starts with defaults.
#[allow(dead_code)]
#[derive(Deserialize)]
struct Gopher64InputButton {
    id: i32,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Gopher64InputAxis {
    id: i32,
    axis: i16,
    initial_state: i16,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Gopher64InputHat {
    id: i32,
    direction: u8,
}

#[allow(dead_code)]
#[derive(Deserialize)]
enum Gopher64InputItem {
    Key(Gopher64InputButton),
    ControllerButton(Gopher64InputButton),
    ControllerAxis(Gopher64InputAxis),
    JoystickButton(Gopher64InputButton),
    JoystickHat(Gopher64InputHat),
    JoystickAxis(Gopher64InputAxis),
}

#[derive(Deserialize)]
struct Gopher64InputProfile {
    inputs: [[Option<Gopher64InputItem>; 2]; 19],
    dinput: bool,
    deadzone: i32,
}

#[derive(Deserialize)]
struct Gopher64Input {
    input_profiles: BTreeMap<String, Gopher64InputProfile>,
    input_profile_binding: [String; 4],
    controller_assignment: [Option<String>; 4],
    controller_enabled: [bool; 4],
    transfer_pak: [bool; 4],
    gb_rom_path: [String; 4],
    gb_ram_path: [String; 4],
    emulate_vru: bool,
}

impl MappedInput {
    fn json(self) -> serde_json::Value {
        match self {
            Self::Button(id) => serde_json::json!({"ControllerButton":{"id":id}}),
            Self::Axis { index, positive } => serde_json::json!({
                "ControllerAxis":{"id":index,"axis":if positive { 1 } else { -1 },"initial_state":0}
            }),
        }
    }

    #[cfg(target_os = "linux")]
    fn from_sdl(input: SdlInput) -> Result<Self> {
        match input {
            SdlInput::GamepadButton { index } => Ok(Self::Button(index)),
            SdlInput::GamepadAxis { index, range } => Ok(Self::Axis {
                index,
                positive: match range {
                    AxisRange::Positive => true,
                    AxisRange::Negative => false,
                    AxisRange::Full => {
                        anyhow::bail!("Gopher64 directional profile entries need one SDL axis half")
                    }
                },
            }),
            _ => anyhow::bail!(
                "Gopher64's gamepad profile cannot consume an unmapped raw SDL control"
            ),
        }
    }
}

fn profile_json(mapped: &BTreeMap<String, MappedInput>) -> Result<serde_json::Value> {
    ensure!(
        mapped.len() == CONTROLS.len(),
        "Gopher64 needs every N64 gameplay control"
    );
    ensure!(
        mapped
            .keys()
            .all(|key| CONTROLS.iter().any(|(control, _)| key == control)),
        "Gopher64 mapping contains an unknown N64 control"
    );
    let mut inputs = vec![serde_json::json!([null, null]); 19];
    let mut used = BTreeSet::new();
    for (control, index) in CONTROLS {
        let value = *mapped
            .get(control)
            .with_context(|| format!("Gopher64 control {control} is absent"))?;
        ensure!(
            used.insert(value),
            "Gopher64 cannot assign one SDL output to multiple N64 controls"
        );
        match value {
            MappedInput::Button(id) => {
                ensure!(id < 26, "Gopher64 SDL gamepad button is out of range")
            }
            MappedInput::Axis { index, .. } => {
                ensure!(index < 6, "Gopher64 SDL gamepad axis is out of range")
            }
        }
        inputs[index] = serde_json::json!([value.json(), null]);
    }
    let axis = |name: &str| match mapped.get(name) {
        Some(MappedInput::Axis { index, positive }) => Ok((*index, *positive)),
        _ => anyhow::bail!("Gopher64 {name} needs a proportional SDL gamepad axis"),
    };
    let left = axis("stick_left")?;
    let right = axis("stick_right")?;
    let up = axis("stick_up")?;
    let down = axis("stick_down")?;
    ensure!(
        left.0 == right.0 && left.1 != right.1,
        "Gopher64 stick left/right must be opposite halves of one SDL axis"
    );
    ensure!(
        up.0 == down.0 && up.1 != down.1 && up.0 != left.0,
        "Gopher64 stick up/down must be opposite halves of a distinct SDL axis"
    );
    Ok(serde_json::json!({"inputs":inputs,"dinput":false,"deadzone":5}))
}

fn patched_config(
    baseline: &[u8],
    profiles: &[(u8, String, serde_json::Value)],
) -> Result<Vec<u8>> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "Gopher64 config is too large"
    );
    let mut root: serde_json::Value =
        serde_json::from_slice(baseline).context("Parsing Gopher64 config.json")?;
    let root_object = root
        .as_object_mut()
        .context("Gopher64 config root must be a JSON object")?;
    let input = root_object
        .entry("input")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("Gopher64 input config must be a JSON object")?;
    let saved_profiles = input
        .entry("input_profiles")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("Gopher64 input_profiles must be a JSON object")?;

    let mut bindings = ["default", "default", "default", "default"].map(str::to_owned);
    let mut assignments: [Option<String>; 4] = std::array::from_fn(|_| None);
    let mut enabled = [false; 4];
    for (port, path, profile) in profiles {
        ensure!((1..=4).contains(port), "Gopher64 port is out of range");
        let index = usize::from(*port - 1);
        let name = format!("Lunchbox Player {port}");
        ensure!(
            assignments[index].replace(path.clone()).is_none(),
            "Gopher64 port is duplicated"
        );
        bindings[index].clone_from(&name);
        enabled[index] = true;
        saved_profiles.insert(name, profile.clone());
    }
    input.insert(
        "input_profile_binding".into(),
        serde_json::to_value(bindings)?,
    );
    input.insert(
        "controller_assignment".into(),
        serde_json::to_value(assignments)?,
    );
    input.insert("controller_enabled".into(), serde_json::to_value(enabled)?);
    input
        .entry("transfer_pak")
        .or_insert_with(|| serde_json::json!([false, false, false, false]));
    input
        .entry("gb_rom_path")
        .or_insert_with(|| serde_json::json!(["", "", "", ""]));
    input
        .entry("gb_ram_path")
        .or_insert_with(|| serde_json::json!(["", "", "", ""]));
    input
        .entry("emulate_vru")
        .or_insert(serde_json::Value::Bool(false));
    serde_json::from_value::<Gopher64Input>(serde_json::Value::Object(input.clone()))
        .context("Patched Gopher64 input config does not match the pinned schema")?;
    serde_json::to_vec_pretty(&root).context("Serializing private Gopher64 config")
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Player {
        pub player: u8,
        pub controller_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        /// Existing user config copied and patched inside the private XDG root.
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Gopher64 needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, Component::ParentDir)),
                    "Gopher64 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str()) == Some("config.json"),
                "Gopher64 config_path must name config.json"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Gopher64 needs a trusted executable SHA-256"
            );
            ensure!(
                (1..=4).contains(&self.players.len()),
                "Gopher64 supports one to four controller ports"
            );
            let mut ports = BTreeSet::new();
            let mut controllers = BTreeSet::new();
            for player in &self.players {
                ensure!(
                    (1..=4).contains(&player.player)
                        && ports.insert(player.player)
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Gopher64 players and physical controllers must be distinct"
                );
            }
            Ok(())
        }

        pub(crate) fn review(
            &self,
            calibrations: &HashMap<String, Calibration>,
        ) -> Result<serde_json::Value> {
            self.validate()?;
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing native Gopher64 profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Gopher64 controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux",
                    "Gopher64 native mapping requires Linux calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "Gopher64 needs native calibration for every N64 gameplay control"
                );
                players.push(serde_json::json!({
                    "port":player.player,"controller_id":player.controller_id,
                    "source_layout":calibration.layout,"target_layout":profile.target_layout,
                    "mapping":mapping
                }));
            }
            Ok(serde_json::json!({
                "players":players,"launch_ready":false,"launch_integration":"partial",
                "detail":"Gopher64 JSON dispatch is connected but runtime-untested. Launch copies the declared config into a private XDG_CONFIG_HOME, patches only controller profiles/assignments, and leaves XDG_DATA_HOME untouched so native saves and states remain persistent. The target SDL3 runtime resolves measured Linux controls. Transfer Pak/VRU configuration is preserved from the copied config but not authored by this adapter; portable mode is rejected."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Gopher64 saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Gopher64 emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub(crate) mod session {
    use super::*;
    use crate::{
        controller_bizhawk_guard::InputTopology,
        controller_catalog::Calibration,
        controller_native_process::{cancelled, capture},
        controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{Snapshot, file_hash, linux_classic::AxisEndpoints};
    use std::{process::Command, sync::atomic::AtomicBool};

    fn observe(
        setup: &settings::SavedSetup,
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl-library")
            .arg(&setup.sdl_library)
            .arg("--hint")
            .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
        for path in paths {
            command.arg("--bindings-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid Gopher64 SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "Gopher64 helper inspected a different SDL runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot, bindings: bool) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        let object = value
            .as_object_mut()
            .context("Invalid Gopher64 snapshot shape")?;
        object.remove("warnings");
        if let Some(report) = object
            .get_mut("player_probe")
            .and_then(serde_json::Value::as_object_mut)
        {
            report.remove("events_processed");
        }
        if !bindings
            && let Some(devices) = object
                .get_mut("devices")
                .and_then(serde_json::Value::as_array_mut)
        {
            for device in devices {
                let object = device
                    .as_object_mut()
                    .context("Invalid Gopher64 device shape")?;
                object.remove("resolved");
                object.remove("linux_classic");
            }
        }
        Ok(value)
    }

    fn mapped_profile(
        calibration: &Calibration,
        device: &lunchbox_controller_probe::Device,
    ) -> Result<serde_json::Value> {
        ensure!(
            device.is_gamepad,
            "Gopher64 needs an SDL3-recognized gamepad"
        );
        let gamepad = device
            .resolved
            .as_ref()
            .context("Gopher64 SDL3 resolved bindings are absent")?;
        let physical = device
            .linux_classic
            .as_ref()
            .context("Gopher64 classic Linux control map is absent")?;
        physical.validate_counts(gamepad)?;
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing native Gopher64 profile")?;
        let mut mapped = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let binding = row
                .input
                .as_ref()
                .context("Gopher64 gameplay control is not calibrated")?;
            let native = binding
                .native
                .as_ref()
                .context("Gopher64 requires measured native controls")?;
            let endpoints = binding.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let raw = physical.digital_input(native.code, endpoints)?;
            let translated = if row.target_id.starts_with("stick_") {
                let lunchbox_controller_probe::duckstation::DigitalInput::Axis {
                    index,
                    released,
                    pressed,
                } = raw
                else {
                    anyhow::bail!(
                        "Gopher64 analog-stick directions require proportional physical axes"
                    );
                };
                crate::controller_pcsx2::physical::analog(
                    gamepad,
                    true,
                    lunchbox_controller_probe::duckstation::AnalogInput {
                        index,
                        released,
                        extent: pressed,
                    },
                )?
            } else {
                crate::controller_pcsx2::physical::digital(gamepad, true, raw)?
            };
            ensure!(
                CONTROLS
                    .iter()
                    .any(|(control, _)| *control == row.target_id),
                "Gopher64 target is outside the N64 contract"
            );
            ensure!(
                mapped
                    .insert(row.target_id, MappedInput::from_sdl(translated)?)
                    .is_none(),
                "Gopher64 target appears twice"
            );
        }
        profile_json(&mapped)
    }

    fn copy_companion(source_dir: &Path, target_dir: &Path, name: &str) -> Result<Option<PathBuf>> {
        let source = source_dir.join(name);
        if !source.exists() {
            return Ok(None);
        }
        ensure!(source.is_file(), "Gopher64 config companion must be a file");
        let bytes = std::fs::read(&source)?;
        ensure!(
            bytes.len() <= 1024 * 1024,
            "Gopher64 config companion is too large"
        );
        let target = target_dir.join(name);
        std::fs::write(&target, bytes)?;
        Ok(Some(target))
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) config_home: PathBuf,
        runtime_paths: Vec<String>,
        topology: InputTopology,
        snapshot: Snapshot,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            calibrations: &HashMap<String, Calibration>,
            inventory: &[ControllerDevice],
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "Gopher64 physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = observe(setup, &[], cancel)?;
            let mut runtime_paths = Vec::new();
            for path in &selected {
                runtime_paths.push(
                    topology.resolve_runtime_path(
                        path,
                        initial
                            .devices
                            .iter()
                            .filter_map(|device| device.path.as_deref()),
                    )?,
                );
            }
            ensure!(
                runtime_paths.iter().collect::<BTreeSet<_>>().len() == runtime_paths.len(),
                "Gopher64 players resolved to the same SDL device"
            );
            let snapshot = observe(setup, &runtime_paths, cancel)?;
            ensure!(
                comparable(&initial, false)? == comparable(&snapshot, false)?,
                "Gopher64 SDL routing changed during binding capture"
            );
            let mut profiles = Vec::new();
            for (player, path) in setup.players.iter().zip(&runtime_paths) {
                profiles.push((
                    player.player,
                    path.clone(),
                    mapped_profile(
                        calibrations
                            .get(&player.controller_id)
                            .context("Gopher64 calibration disappeared")?,
                        snapshot.device_at_path(path)?,
                    )?,
                ));
            }
            let baseline = std::fs::read(&setup.config_path)
                .context("Reading the declared Gopher64 config.json")?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-gopher64-")
                .tempdir()?;
            let config_home = directory.path().join("config-home");
            let target_dir = config_home.join("gopher64");
            std::fs::create_dir_all(&target_dir)?;
            let private_config = target_dir.join("config.json");
            std::fs::write(&private_config, patched_config(&baseline, &profiles)?)?;
            let source_dir = setup
                .config_path
                .parent()
                .context("Gopher64 config path has no parent")?;
            let mut companions = Vec::new();
            for name in ["cheats.json", "retroachievements.json"] {
                if let Some(path) = copy_companion(source_dir, &target_dir, name)? {
                    companions.push(path);
                }
            }

            let mut hashes = BTreeMap::new();
            for path in [
                &setup.probe_program,
                &setup.sdl_library,
                &setup.content,
                &setup.config_path,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            for path in companions {
                hashes.insert(path.clone(), file_hash(&path)?);
            }
            let session = Self {
                directory,
                config_home,
                runtime_paths,
                topology,
                snapshot,
                setup: setup.clone(),
                hashes,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            ensure!(
                self.directory.path().is_dir(),
                "Gopher64 session disappeared"
            );
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "Gopher64 launch input changed"
                );
            }
            let current = observe(&self.setup, &self.runtime_paths, cancel)?;
            ensure!(
                comparable(&current, true)? == comparable(&self.snapshot, true)?,
                "Gopher64 SDL routing or resolved bindings changed before launch"
            );
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) mod native_command {
    use super::*;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::cancelled,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::sync::atomic::AtomicBool;

    pub(crate) struct NativeSession {
        pub(crate) inputs: session::PreparedSession,
        executable: PathBuf,
        setup: settings::SavedSetup,
        pub(crate) plan: LaunchPlan,
    }

    impl NativeSession {
        pub(crate) fn check_health(&self) -> Result<()> {
            self.inputs.check_health()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                file_hash(&self.executable)? == self.setup.executable_sha256,
                "Gopher64 executable differs from the saved trusted runtime"
            );
            self.inputs.verify(cancel)
        }

        pub(crate) fn spawn(
            &mut self,
            plan: &LaunchPlan,
            cancel: &AtomicBool,
        ) -> Result<std::process::Child> {
            ensure!(
                plan == &self.plan,
                "Gopher64 launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
        }
    }

    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<NativeSession> {
        cancelled(cancel)?;
        setup.validate()?;
        let EmulatorExecutable::Native(executable) = &option.executable else {
            anyhow::bail!("Gopher64 calibrated launch requires native Linux, not Wine/Flatpak");
        };
        ensure!(
            setup.emulator_id == option.emulator_id && original.environment.is_empty(),
            "Gopher64 identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Gopher64 launch executable differs from selection"
        );
        ensure!(
            original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
            "Gopher64 calibrated launch currently requires exactly the saved game argument"
        );
        ensure!(
            !executable
                .parent()
                .context("Gopher64 executable has no parent")?
                .join("portable.txt")
                .exists(),
            "Gopher64 portable mode overrides XDG_CONFIG_HOME and is not safely supported"
        );
        ensure!(
            file_hash(&executable)? == setup.executable_sha256,
            "Gopher64 executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.environment.push((
            "XDG_CONFIG_HOME".into(),
            inputs.config_home.as_os_str().to_owned(),
        ));
        plan.environment
            .push(("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into()));
        let session = NativeSession {
            inputs,
            executable,
            setup: setup.clone(),
            plan,
        };
        session.verify(cancel)?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_profile() -> BTreeMap<String, MappedInput> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(n, (control, _))| {
                let input = match *control {
                    "stick_left" => MappedInput::Axis {
                        index: 0,
                        positive: false,
                    },
                    "stick_right" => MappedInput::Axis {
                        index: 0,
                        positive: true,
                    },
                    "stick_up" => MappedInput::Axis {
                        index: 1,
                        positive: false,
                    },
                    "stick_down" => MappedInput::Axis {
                        index: 1,
                        positive: true,
                    },
                    _ => MappedInput::Button(n as u8 + 2),
                };
                ((*control).to_owned(), input)
            })
            .collect()
    }

    #[test]
    fn renders_pinned_profile_order_and_axis_schema() {
        let profile = profile_json(&complete_profile()).unwrap();
        serde_json::from_value::<Gopher64InputProfile>(profile.clone()).unwrap();
        let inputs = profile["inputs"].as_array().unwrap();
        assert_eq!(inputs.len(), 19);
        assert_eq!(inputs[7][0]["ControllerButton"]["id"], 9);
        assert_eq!(inputs[15][0]["ControllerAxis"]["id"], 0);
        assert_eq!(inputs[15][0]["ControllerAxis"]["axis"], -1);
        assert!(inputs[18][0].is_null());
        assert_eq!(profile["dinput"], false);
        assert_eq!(profile["deadzone"], 5);
    }

    #[test]
    fn private_patch_preserves_unowned_config_and_transfer_pak() {
        let mut custom = profile_json(&complete_profile()).unwrap();
        custom["future"] = serde_json::json!(1);
        let baseline = serde_json::to_vec(&serde_json::json!({
          "input":{"input_profiles":{"custom":custom},
            "input_profile_binding":["custom","custom","custom","custom"],
            "controller_assignment":["old",null,null,null],
            "controller_enabled":[true,false,false,false],
            "transfer_pak":[true,false,false,false],"gb_rom_path":["gb", "", "", ""],
            "gb_ram_path":["ram", "", "", ""],"emulate_vru":true,"future_input":7},
          "video":{"future_video":8},"future_root":9
        }))
        .unwrap();
        let players = vec![(
            1,
            "/dev/input/js0".into(),
            profile_json(&complete_profile()).unwrap(),
        )];
        let value: serde_json::Value =
            serde_json::from_slice(&patched_config(&baseline, &players).unwrap()).unwrap();
        assert_eq!(value["future_root"], 9);
        assert_eq!(value["video"]["future_video"], 8);
        assert_eq!(value["input"]["future_input"], 7);
        assert_eq!(value["input"]["transfer_pak"][0], true);
        assert_eq!(value["input"]["gb_rom_path"][0], "gb");
        assert_eq!(value["input"]["controller_assignment"][0], "/dev/input/js0");
        assert_eq!(
            value["input"]["input_profile_binding"][0],
            "Lunchbox Player 1"
        );
        assert_eq!(value["input"]["input_profiles"]["custom"]["future"], 1);
    }

    #[test]
    fn rejects_duplicate_or_mismatched_stick_outputs() {
        let mut duplicate = complete_profile();
        duplicate.insert("b".into(), duplicate["a"]);
        assert!(profile_json(&duplicate).is_err());
        let mut mismatched = complete_profile();
        mismatched.insert(
            "stick_right".into(),
            MappedInput::Axis {
                index: 2,
                positive: true,
            },
        );
        assert!(profile_json(&mismatched).is_err());
    }

    #[test]
    fn rejects_unknown_controls_and_invalid_sdl_indices() {
        let mut unknown = complete_profile();
        unknown.remove("a");
        unknown.insert("not_n64".into(), MappedInput::Button(0));
        assert!(profile_json(&unknown).is_err());

        let mut button = complete_profile();
        button.insert("a".into(), MappedInput::Button(26));
        assert!(profile_json(&button).is_err());

        let mut axis = complete_profile();
        axis.insert(
            "stick_left".into(),
            MappedInput::Axis {
                index: 6,
                positive: false,
            },
        );
        assert!(profile_json(&axis).is_err());
    }

    #[test]
    fn rejects_malformed_preserved_input_fields() {
        let result = patched_config(br#"{"input":{"transfer_pak":"not-an-array"}}"#, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn setup_validation_rejects_traversal_and_duplicate_players() {
        let base = settings::SavedSetup {
            emulator_id: "gopher64".into(),
            content: "/games/mario.n64".into(),
            config_path: "/home/user/.config/gopher64/config.json".into(),
            probe_program: "/opt/lunchbox-probe".into(),
            sdl_library: "/usr/lib/libSDL3.so".into(),
            executable_sha256: "a".repeat(64),
            players: vec![settings::Player {
                player: 1,
                controller_id: "pad-1".into(),
            }],
        };
        assert!(base.validate().is_ok());

        let mut traversal = base.clone();
        traversal.content = "/games/../secret.n64".into();
        assert!(traversal.validate().is_err());

        let mut duplicate = base;
        duplicate.players.push(settings::Player {
            player: 1,
            controller_id: "pad-2".into(),
        });
        assert!(duplicate.validate().is_err());
    }
}
