//! RMG native Linux input-plugin profiles, not Mupen64Plus/libretro bindings.
//!
//! Functional contract pinned to Rosalie241/RMG
//! 3e8b366be91ea96329db0567b038a31785f33468 (v0.9.0):
//! - `Source/RMG-Input/main.cpp` loads exactly four profile sections,
//!   `Rosalie's Mupen GUI - Input Plugin Profile 0` through `3`.  This is the
//!   source's zero-based slot index; the UI labels these as players 1 through
//!   4.  `DeviceType=4` opens an SDL joystick and compares DeviceName,
//!   DevicePath and DeviceSerial exactly.
//! - `Source/RMG-Input/common.hpp` defines raw joystick input types: button is
//!   2, axis is 3 and hat is 4.  Axis ExtraData is positive when non-zero and
//!   negative when zero; SDL hat masks are up=1, right=2, down=4, left=8.
//! - `load_inputmapping_settings` consumes semicolon-separated Name, InputType,
//!   Data and ExtraData lists.  The writer emits one measured raw joystick
//!   item per N64 control and leaves GUI/hotkey settings untouched.
//! - `Source/RMG-Core/Core.cpp` starts Mupen64Plus with the config directory
//!   under `$XDG_CONFIG_HOME/RMG`, then force-sets SaveSRAMPath and
//!   SaveStatePath from the independent XDG data root.  The Linux session
//!   therefore isolates only XDG_CONFIG_HOME and deliberately leaves
//!   XDG_DATA_HOME unchanged.
//!
//! The Linux session requires the same trusted SDL3 library and probe hashes
//! at preparation and launch.  SDL's classic Linux backend is selected because
//! the probe's verified `/dev/input/js*` map is the native RMG joystick
//! coordinate system.  Portable mode and virtual controllers are rejected.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

pub(crate) const SOURCE_COMMIT: &str = "3e8b366be91ea96329db0567b038a31785f33468";
pub(crate) const PROFILE_ID: &str = "rmg:standalone-n64";
pub(crate) const DIGITAL_THRESHOLD: i32 = 16_383; // SDL_AXIS_PEAK / 2

/// N64 target control -> RMG InputMapping field prefix.
pub(crate) const CONTROLS: [(&str, &str); 18] = [
    ("a", "A"),
    ("b", "B"),
    ("start", "Start"),
    ("up", "DpadUp"),
    ("down", "DpadDown"),
    ("left", "DpadLeft"),
    ("right", "DpadRight"),
    ("c_up", "CButtonUp"),
    ("c_down", "CButtonDown"),
    ("c_left", "CButtonLeft"),
    ("c_right", "CButtonRight"),
    ("l", "LeftTrigger"),
    ("r", "RightTrigger"),
    ("z", "ZTrigger"),
    ("stick_up", "AnalogStickUp"),
    ("stick_down", "AnalogStickDown"),
    ("stick_left", "AnalogStickLeft"),
    ("stick_right", "AnalogStickRight"),
];

/// One raw SDL joystick item in RMG's InputType/Data/ExtraData vocabulary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
    Hat { index: u32, direction: u8 },
}

impl Binding {
    fn fields(self) -> (i32, u32, u32) {
        match self {
            Self::Button(index) => (2, index, 0),
            Self::Axis { index, positive } => (3, index, u32::from(positive)),
            Self::Hat { index, direction } => (4, index, u32::from(direction)),
        }
    }

    fn validate(self) -> Result<()> {
        match self {
            Self::Button(index) => ensure!(
                index <= i32::MAX as u32,
                "RMG button index overflows its integer setting"
            ),
            Self::Axis { index, positive: _ } => ensure!(
                index <= i32::MAX as u32,
                "RMG axis index overflows its integer setting"
            ),
            Self::Hat { index, direction } => {
                ensure!(
                    index <= i32::MAX as u32,
                    "RMG hat index overflows its integer setting"
                );
                ensure!(
                    matches!(direction, 1 | 2 | 4 | 8),
                    "RMG hat direction must be a cardinal SDL mask"
                );
            }
        }
        Ok(())
    }
}

fn safe_string(value: &str, what: &str) -> Result<()> {
    ensure!(
        !value.is_empty() || what == "SDL serial",
        "RMG {what} cannot be empty"
    );
    ensure!(value.len() <= 4096, "RMG {what} is too long");
    ensure!(
        !value
            .chars()
            .any(|ch| ch == '"' || ch == '\r' || ch == '\n' || ch.is_control()),
        "RMG {what} cannot contain quotes or control characters"
    );
    ensure!(
        !value.contains(';'),
        "RMG {what} cannot contain the list separator"
    );
    Ok(())
}

fn quoted(value: &str, what: &str) -> Result<String> {
    safe_string(value, what)?;
    Ok(format!("\"{value}\""))
}

#[derive(Clone, Debug)]
pub(crate) struct PortConfig {
    /// One-based Lunchbox player number; RMG's source section is `port - 1`.
    pub port: u8,
    pub device_name: String,
    pub device_path: String,
    pub device_serial: String,
    pub bindings: BTreeMap<String, Binding>,
}

fn section_name(index: usize) -> String {
    format!("Rosalie's Mupen GUI - Input Plugin Profile {index}")
}

fn is_removed_section(name: &str) -> bool {
    (0..4).any(|index| {
        let prefix = format!(
            "{} {index} Game ",
            "Rosalie's Mupen GUI - Input Plugin Profile"
        );
        name.starts_with(&prefix)
    })
}

fn profile_text(index: usize, config: Option<&PortConfig>) -> Result<String> {
    let section = section_name(index);
    let mut out = format!("[{section}]\n\n");
    let Some(config) = config else {
        out.push_str("PluggedIn = False\nDeviceType = 0\nDeviceName = \"\"\nDevicePath = \"\"\nDeviceSerial = \"\"\n\n");
        return Ok(out);
    };
    ensure!(config.port as usize == index + 1, "RMG port/index mismatch");
    safe_string(&config.device_name, "SDL device name")?;
    safe_string(&config.device_path, "SDL device path")?;
    safe_string(&config.device_serial, "SDL serial")?;
    ensure!(
        config.device_path.starts_with("/dev/input/js"),
        "RMG native Linux requires an exact /dev/input/js* path"
    );
    ensure!(!config.bindings.is_empty(), "RMG port has no bindings");
    for (control, _) in &config.bindings {
        ensure!(
            CONTROLS.iter().any(|(id, _)| *id == control),
            "Unknown RMG N64 control {control}"
        );
    }
    ensure!(
        config.bindings.len() == CONTROLS.len(),
        "RMG requires all N64 gameplay controls"
    );
    let mut used = BTreeSet::new();
    for binding in config.bindings.values() {
        ensure!(used.insert(*binding), "RMG controls share one native input");
    }
    out.push_str("PluggedIn = True\nDeviceType = 4\n");
    out.push_str(&format!(
        "DeviceName = {}\n",
        quoted(&config.device_name, "SDL device name")?
    ));
    out.push_str(&format!(
        "DevicePath = {}\n",
        quoted(&config.device_path, "SDL device path")?
    ));
    out.push_str(&format!(
        "DeviceSerial = {}\n",
        quoted(&config.device_serial, "SDL serial")?
    ));
    // Mapping-only launch defaults. Pak/Transfer Pak authoring is deliberately
    // outside this adapter and the private profile starts with no pak.
    out.push_str(
        "Deadzone = 0\nSensitivity = 100\nPak = 3\nGameboyRom = \"\"\nGameboySave = \"\"\n",
    );
    for (control, field) in CONTROLS {
        let binding = *config
            .bindings
            .get(control)
            .with_context(|| format!("RMG control {control} is absent"))?;
        binding.validate()?;
        let (kind, data, extra) = binding.fields();
        out.push_str(&format!("{field}_Name = \"{control}\"\n{field}_InputType = {kind}\n{field}_Data = {data}\n{field}_ExtraData = {extra}\n"));
    }
    out.push('\n');
    Ok(out)
}

/// Replace only the four base controller sections in a copied config. Game
/// override sections are removed because the current ROM identity is not
/// available to Lunchbox's launcher; retaining them would let RMG silently
/// replace a measured mapping. All unrelated GUI/core/plugin sections remain.
pub(crate) fn patched_config(baseline: &[u8], ports: &[PortConfig]) -> Result<Vec<u8>> {
    ensure!(baseline.len() <= 4 * 1024 * 1024, "RMG config is too large");
    let baseline = std::str::from_utf8(baseline).context("RMG config is not UTF-8")?;
    ensure!(ports.len() <= 4, "RMG supports four controller slots");
    let mut by_index = [None, None, None, None];
    let mut seen = BTreeSet::new();
    for port in ports {
        ensure!(
            (1..=4).contains(&port.port) && seen.insert(port.port),
            "RMG port is invalid or duplicated"
        );
        by_index[usize::from(port.port - 1)] = Some(port);
    }
    let generated = (0..4)
        .map(|i| profile_text(i, by_index[i]))
        .collect::<Result<Vec<_>>>()?;

    let mut out = String::new();
    let mut skip_current = false;
    let mut replaced = [false; 4];
    for line in baseline.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']).trim();
        let header = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'));
        if let Some(name) = header {
            if let Some(index) = (0..4).find(|i| name == section_name(*i)) {
                out.push_str(&generated[index]);
                replaced[index] = true;
                skip_current = true;
                continue;
            }
            if is_removed_section(name) {
                skip_current = true;
                continue;
            }
            skip_current = false;
        }
        if skip_current {
            continue;
        }
        out.push_str(line);
    }
    for (index, text) in generated.iter().enumerate() {
        if !replaced[index] {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(text);
        }
    }
    Ok(out.into_bytes())
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
        /// Existing RMG config copied and patched inside private XDG_CONFIG_HOME.
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
                "RMG needs an emulator identity"
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
                    "RMG setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str())
                    == Some("mupen64plus.cfg"),
                "RMG config_path must name mupen64plus.cfg"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "RMG needs a trusted executable SHA-256"
            );
            ensure!(
                (1..=4).contains(&self.players.len()),
                "RMG supports one to four controller ports"
            );
            let mut ports = BTreeSet::new();
            let mut devices = BTreeSet::new();
            for player in &self.players {
                ensure!(
                    (1..=4).contains(&player.player)
                        && ports.insert(player.player)
                        && !player.controller_id.trim().is_empty()
                        && devices.insert(&player.controller_id),
                    "RMG players and physical controllers must be distinct"
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
                .context("Missing native RMG profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("RMG controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux",
                    "RMG native mapping requires Linux calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.len() == CONTROLS.len()
                        && mapping.rows.iter().all(|row| row.physical_id.is_some()
                            && row
                                .input
                                .as_ref()
                                .is_some_and(|input| input.native.is_some())),
                    "RMG needs native calibration for every N64 gameplay control"
                );
                players.push(serde_json::json!({"port":player.player,"controller_id":player.controller_id,"source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}));
            }
            Ok(
                serde_json::json!({"players":players,"launch_ready":false,"launch_integration":"partial","detail":"RMG raw SDL3 joystick profiles are source-backed but runtime-untested. Launch copies mupen64plus.cfg under private XDG_CONFIG_HOME/RMG, removes per-game input overrides, patches source-accurate zero-based profile sections, and leaves XDG_DATA_HOME untouched so Save/Game and Save/State remain persistent. Device identity, SDL classic numbering, probe hash and executable hash are rechecked before launch. Paks, Transfer Pak/Game Boy paths, GUI hotkeys, optional PIF and 64DD media are not authored."}),
            )
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many RMG saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate RMG emulator/content setup"
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
            command.arg("--match-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid RMG SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "RMG helper inspected a different SDL runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot, bindings: bool) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        let object = value
            .as_object_mut()
            .context("Invalid RMG snapshot shape")?;
        object.remove("warnings");
        if !bindings
            && let Some(devices) = object
                .get_mut("devices")
                .and_then(serde_json::Value::as_array_mut)
        {
            for device in devices {
                let device = device.as_object_mut().context("Invalid RMG device shape")?;
                device.remove("resolved");
                device.remove("linux_classic");
            }
        }
        Ok(value)
    }

    fn mapped_profile(
        calibration: &Calibration,
        physical: &lunchbox_controller_probe::linux_classic::ClassicMap,
    ) -> Result<BTreeMap<String, Binding>> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing native RMG profile")?;
        let mut mapped = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("RMG gameplay control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("RMG requires measured native controls")?;
            let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let binding = match physical.digital_input(native.code, measured)? {
                lunchbox_controller_probe::duckstation::DigitalInput::Button(index) => {
                    Binding::Button(index)
                }
                lunchbox_controller_probe::duckstation::DigitalInput::Hat { index, direction } => {
                    Binding::Hat { index, direction }
                }
                lunchbox_controller_probe::duckstation::DigitalInput::Axis {
                    index,
                    released,
                    pressed,
                } => {
                    ensure!(
                        i32::from(released).abs() < DIGITAL_THRESHOLD,
                        "RMG axis rest would be read as a pressed digital input"
                    );
                    let pressed = i32::from(pressed);
                    ensure!(
                        pressed.abs() >= DIGITAL_THRESHOLD,
                        "RMG axis travel does not cross SDL's digital threshold"
                    );
                    Binding::Axis {
                        index,
                        positive: pressed > 0,
                    }
                }
            };
            ensure!(
                mapped.insert(row.target_id, binding).is_none(),
                "RMG target appears twice"
            );
        }
        ensure!(
            mapped.len() == CONTROLS.len()
                && CONTROLS.iter().all(|(id, _)| mapped.contains_key(*id)),
            "RMG mapping is incomplete"
        );
        let mut used = BTreeSet::new();
        for binding in mapped.values() {
            ensure!(used.insert(*binding), "RMG controls share one native input");
        }
        for (negative, positive) in [("stick_left", "stick_right"), ("stick_up", "stick_down")] {
            let (
                Binding::Axis {
                    index: a,
                    positive: pa,
                },
                Binding::Axis {
                    index: b,
                    positive: pb,
                },
            ) = (mapped[negative], mapped[positive])
            else {
                anyhow::bail!("RMG analog stick directions require proportional axes")
            };
            ensure!(
                a == b && pa != pb,
                "RMG analog stick directions must be opposite halves of one axis"
            );
        }
        Ok(mapped)
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) config_home: PathBuf,
        runtime_paths: Vec<String>,
        topology: InputTopology,
        snapshot: Snapshot,
        classic_maps: Vec<lunchbox_controller_probe::linux_classic::ClassicMap>,
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
                    "RMG physical controller is missing or ambiguous"
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
                "RMG players resolved to the same SDL device"
            );
            let snapshot = observe(setup, &runtime_paths, cancel)?;
            ensure!(
                comparable(&initial, false)? == comparable(&snapshot, false)?,
                "RMG SDL routing changed during binding capture"
            );
            let mut ports = Vec::new();
            let mut classic_maps = Vec::new();
            for (player, path) in setup.players.iter().zip(&runtime_paths) {
                let device = snapshot.device_at_path(path)?;
                ensure!(
                    path.starts_with("/dev/input/js"),
                    "RMG native Linux requires SDL's exact /dev/input/js* path"
                );
                let physical = lunchbox_controller_probe::linux_classic::read(Path::new(path))?;
                classic_maps.push(physical.clone());
                ports.push(PortConfig {
                    port: player.player,
                    device_name: device
                        .name
                        .clone()
                        .context("RMG SDL device has no stable name")?,
                    device_path: path.clone(),
                    device_serial: String::new(),
                    bindings: mapped_profile(
                        calibrations
                            .get(&player.controller_id)
                            .context("RMG calibration disappeared")?,
                        &physical,
                    )?,
                });
            }
            let baseline = std::fs::read(&setup.config_path)
                .context("Reading declared RMG mupen64plus.cfg")?;
            let directory = tempfile::Builder::new().prefix("lunchbox-rmg-").tempdir()?;
            let config_home = directory.path().join("config-home");
            let target_dir = config_home.join("RMG");
            std::fs::create_dir_all(&target_dir)?;
            let private_config = target_dir.join("mupen64plus.cfg");
            std::fs::write(&private_config, patched_config(&baseline, &ports)?)?;
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
            let session = Self {
                directory,
                config_home,
                runtime_paths,
                topology,
                snapshot,
                classic_maps,
                setup: setup.clone(),
                hashes,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            ensure!(self.directory.path().is_dir(), "RMG session disappeared");
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "RMG launch input changed: {}",
                    path.display()
                );
            }
            let current = observe(&self.setup, &self.runtime_paths, cancel)?;
            ensure!(
                comparable(&current, true)? == comparable(&self.snapshot, true)?,
                "RMG SDL routing or resolved bindings changed before launch"
            );
            for (path, expected) in self.runtime_paths.iter().zip(&self.classic_maps) {
                ensure!(
                    lunchbox_controller_probe::linux_classic::read(Path::new(path))? == *expected,
                    "RMG SDL classic joystick map changed before launch"
                );
            }
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
                "RMG executable differs from the saved trusted runtime"
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
                "RMG launch plan changed after preparation"
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
            anyhow::bail!("RMG calibrated launch requires native Linux, not Wine/Flatpak")
        };
        ensure!(
            setup.emulator_id == option.emulator_id && original.environment.is_empty(),
            "RMG identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "RMG launch executable differs from selection"
        );
        ensure!(
            original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
            "RMG calibrated launch requires exactly the saved ROM argument"
        );
        let parent = executable
            .parent()
            .context("RMG executable has no parent")?;
        ensure!(
            !parent.join("portable.txt").exists()
                && !parent.join("Config/mupen64plus.cfg").exists(),
            "RMG portable mode overrides XDG roots and is not safely supported"
        );
        ensure!(
            file_hash(&executable)? == setup.executable_sha256,
            "RMG executable differs from the saved trusted runtime"
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
    #[test]
    fn source_zero_based_profiles_and_raw_mapping_fields_are_exact() {
        let mut bindings = BTreeMap::new();
        for (index, (control, _)) in CONTROLS.iter().enumerate() {
            bindings.insert((*control).to_owned(), Binding::Button(index as u32));
        }
        bindings.insert(
            "stick_left".into(),
            Binding::Axis {
                index: 0,
                positive: false,
            },
        );
        bindings.insert(
            "stick_right".into(),
            Binding::Axis {
                index: 0,
                positive: true,
            },
        );
        bindings.insert(
            "stick_up".into(),
            Binding::Axis {
                index: 1,
                positive: false,
            },
        );
        bindings.insert(
            "stick_down".into(),
            Binding::Axis {
                index: 1,
                positive: true,
            },
        );
        let port = PortConfig {
            port: 1,
            device_name: "Pad".into(),
            device_path: "/dev/input/js0".into(),
            device_serial: String::new(),
            bindings,
        };
        let text = patched_config(b"[Core]\nSaveSRAMPath = \"/persistent\"\n[Rosalie's Mupen GUI - Input Plugin Profile 0 Game abc]\nUseGameProfile = True\n", &[port]).unwrap();
        let text = String::from_utf8(text).unwrap();
        assert!(text.contains("[Rosalie's Mupen GUI - Input Plugin Profile 0]\n"));
        assert!(text.contains("A_InputType = 2\nA_Data = 0\n"));
        assert!(text.contains(
            "AnalogStickLeft_InputType = 3\nAnalogStickLeft_Data = 0\nAnalogStickLeft_ExtraData = 0\n"
        ));
        assert!(!text.contains("Profile 0 Game abc"));
        assert!(text.contains("SaveSRAMPath = \"/persistent\""));
        assert!(text.contains("[Rosalie's Mupen GUI - Input Plugin Profile 3]\n"));
    }
}
