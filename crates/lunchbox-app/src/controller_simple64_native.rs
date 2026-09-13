//! simple64 native `simple64-input-qt` SDL2 controller profiles.
//!
//! Functional contract pinned to simple64/simple64
//! d8c969c7b932e3d76e6a25549d76348839dbaefd:
//! - `simple64-input-qt/main.cpp` opens `input-profiles.ini` and
//!   `input-settings.ini` below `ConfigGetUserConfigPath()`. Profiles use
//!   comma-separated `index,type,direction` values: type 1 is an SDL
//!   GameController button and type 2 is an SDL GameController axis.
//! - the same file fixes the N64 controls as A, B, Z, Start, L, R, DPadL/R/U/D,
//!   CLeft/Right/Up/Down and AxisLeft/Right/Up/Down. `Controller1` through
//!   `Controller4` select `Profile`, `Gamepad` and `Pak`.
//! - `simple64-gui/mainwindow.cpp` reads `simple64-gui.ini` beside the
//!   executable and passes its `configDirPath` to `CoreStartup`; that override
//!   changes config/input roots only. Mupen64Plus data and save roots remain
//!   `$XDG_DATA_HOME/mupen64plus/save` (or the platform default).
//!
//! Runtime status: source-backed and launch-isolated, but runtime-unverified.
//! This adapter never edits the user's application-adjacent GUI INI or core
//! INI files.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

pub(crate) const PROFILE_ID: &str = "simple64:standalone-n64";

/// Lunchbox N64 control -> simple64-input-qt profile key.
pub(crate) const CONTROLS: [(&str, &str); 18] = [
    ("a", "A"),
    ("b", "B"),
    ("z", "Z"),
    ("start", "Start"),
    ("l", "L"),
    ("r", "R"),
    ("left", "DPadL"),
    ("right", "DPadR"),
    ("up", "DPadU"),
    ("down", "DPadD"),
    ("c_left", "CLeft"),
    ("c_right", "CRight"),
    ("c_up", "CUp"),
    ("c_down", "CDown"),
    ("stick_left", "AxisLeft"),
    ("stick_right", "AxisRight"),
    ("stick_up", "AxisUp"),
    ("stick_down", "AxisDown"),
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
}

impl Binding {
    fn value(self) -> String {
        match self {
            Self::Button(index) => format!("{index},1"),
            Self::Axis { index, positive } => {
                format!("{index},2,{}", if positive { 1 } else { -1 })
            }
        }
    }
}

/// Render a QSettings-compatible profile section. The profile intentionally
/// contains no keyboard/raw-joystick fallback: simple64 must use SDL2's
/// logical GameController path selected by `Gamepad` below.
pub(crate) fn profile_ini(name: &str, bindings: &BTreeMap<String, Binding>) -> Result<String> {
    ensure!(
        !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "Invalid simple64 profile name"
    );
    ensure!(
        bindings.len() == CONTROLS.len(),
        "simple64 needs every N64 gameplay control"
    );
    let mut out = format!("[{name}]\n");
    let mut used = BTreeSet::new();
    for (target, key) in CONTROLS {
        let binding = *bindings
            .get(target)
            .with_context(|| format!("simple64 control {target} is absent"))?;
        ensure!(
            used.insert(binding),
            "simple64 profile reuses one SDL output"
        );
        out.push_str(key);
        out.push('=');
        out.push_str(&binding.value());
        out.push('\n');
    }
    out.push_str("Deadzone=5\nSensitivity=100\n");
    Ok(out)
}

fn patch_ini(baseline: &[u8], sections: &BTreeSet<String>, additions: &str) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "simple64 INI is too large"
    );
    let text = String::from_utf8(baseline.to_vec()).context("simple64 INI is not UTF-8")?;
    let mut kept = Vec::new();
    let mut section = None::<String>;
    for line in text.lines() {
        if section.is_none() && line.starts_with("version=") {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
            section = Some(name.to_owned());
        }
        if section.as_ref().is_none_or(|name| !sections.contains(name)) {
            kept.push(line);
        }
    }
    let mut out = String::from("version=2\n");
    out.push_str(&kept.join("\n"));
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(additions);
    Ok(out)
}

pub(crate) fn input_profiles_ini(
    baseline: &[u8],
    bindings: &[(String, BTreeMap<String, Binding>)],
) -> Result<String> {
    let names = bindings.iter().map(|(name, _)| name.clone()).collect();
    let mut additions = String::new();
    for (name, map) in bindings {
        additions.push_str(&profile_ini(name, map)?);
    }
    patch_ini(baseline, &names, &additions)
}

pub(crate) fn input_settings_ini(
    baseline: &[u8],
    assignments: &[(u8, String, String)],
) -> Result<String> {
    ensure!(
        !assignments.is_empty() && assignments.len() <= 4,
        "simple64 supports one to four controller ports"
    );
    let mut seen = BTreeSet::new();
    let mut sections = BTreeSet::new();
    let mut additions = String::new();
    for (port, profile, gamepad) in assignments {
        ensure!(
            (1..=4).contains(port) && seen.insert(*port),
            "simple64 controller port is duplicated or out of range"
        );
        ensure!(
            !profile.is_empty() && !gamepad.is_empty(),
            "simple64 controller assignment is incomplete"
        );
        let section = format!("Controller{port}");
        sections.insert(section.clone());
        additions.push_str(&format!(
            "[{section}]\nProfile={profile}\nGamepad={gamepad}\nPak=Memory\n"
        ));
    }
    for port in 1..=4 {
        if seen.contains(&port) {
            continue;
        }
        let section = format!("Controller{port}");
        sections.insert(section.clone());
        additions.push_str(&format!(
            "[{section}]\nProfile=Auto\nGamepad=None\nPak=None\n"
        ));
    }
    patch_ini(baseline, &sections, &additions)
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
        /// Existing Mupen64Plus config directory containing mupen64plus.cfg.
        pub config_dir: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "simple64 needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_dir,
                &self.probe_program,
                &self.sdl_library,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, Component::ParentDir)),
                    "simple64 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_dir.is_dir(),
                "simple64 config_dir is not a directory"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit()),
                "simple64 needs a trusted executable SHA-256"
            );
            ensure!(
                (1..=4).contains(&self.players.len()),
                "simple64 supports one to four controller ports"
            );
            let mut ports = BTreeSet::new();
            let mut controllers = BTreeSet::new();
            for player in &self.players {
                ensure!(
                    (1..=4).contains(&player.player)
                        && ports.insert(player.player)
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "simple64 players and physical controllers must be distinct"
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
                .context("Missing native simple64 profile")?;
            let players = self.players.iter().map(|player| {
                let calibration = calibrations.get(&player.controller_id).context("simple64 controller has no saved calibration")?;
                ensure!(calibration.os == "linux", "simple64 native mapping requires Linux calibration");
                Ok(serde_json::json!({"port":player.player,"controller_id":player.controller_id,"target_layout":profile.target_layout,"mapping":calibration.plan_profile(profile)?}))
            }).collect::<Result<Vec<_>>>()?;
            Ok(
                serde_json::json!({"players":players,"launch_ready":false,"launch_integration":"partial","detail":"simple64 native SDL2 input profiles are source-backed and launch-isolated but runtime-unverified. The adapter stages the executable directory, writes private input-profiles.ini/input-settings.ini and a private GUI configDirPath, while leaving XDG_DATA_HOME/mupen64plus/save untouched. Keyboard, raw joystick, VRU, transfer-pak behavior and 64DD IPL selection are outside this contract."}),
            )
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many simple64 saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate simple64 emulator/content setup"
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
    use lunchbox_controller_probe::{
        duckstation::DigitalInput,
        file_hash,
        sdl2::{Device, Snapshot},
        sdl2_mapping::{self, Input as LogicalInput},
        sdl2_physical::PhysicalMap,
    };
    use std::{fs, os::unix::fs::symlink, process::Command, sync::atomic::AtomicBool};

    fn observe(
        setup: &settings::SavedSetup,
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&setup.sdl_library);
        for path in paths {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid simple64 SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "simple64 helper inspected a different SDL2 runtime"
        );
        Ok(snapshot)
    }

    fn routing(mut snapshot: Snapshot) -> Snapshot {
        for device in &mut snapshot.devices {
            device.controls = None;
            device.linux_classic = None;
            device.linux_evdev = None;
            device.sampled_state = None;
        }
        snapshot
    }

    fn logical_binding(
        device: &Device,
        native: u32,
        endpoints: Option<lunchbox_controller_probe::linux_classic::AxisEndpoints>,
    ) -> Result<Binding> {
        ensure!(
            device.is_game_controller,
            "simple64 requires an SDL2 GameController"
        );
        let map = PhysicalMap::from_device(device)?;
        let raw = map.digital_input(native, endpoints)?;
        let mapping = sdl2_mapping::parse(
            device
                .mapping
                .as_deref()
                .context("simple64 SDL2 mapping is absent")?,
        )?;
        let button_index = |name: &str| match name {
            "a" => Some(0),
            "b" => Some(1),
            "x" => Some(2),
            "y" => Some(3),
            "back" => Some(4),
            "guide" => Some(5),
            "start" => Some(6),
            "leftstick" => Some(7),
            "rightstick" => Some(8),
            "leftshoulder" => Some(9),
            "rightshoulder" => Some(10),
            "dpup" => Some(11),
            "dpdown" => Some(12),
            "dpleft" => Some(13),
            "dpright" => Some(14),
            _ => None,
        };
        let axis_index = |name: &str| match name {
            "leftx" => Some(0),
            "lefty" => Some(1),
            "rightx" => Some(2),
            "righty" => Some(3),
            "lefttrigger" => Some(4),
            "righttrigger" => Some(5),
            _ => None,
        };
        let mut candidates = BTreeSet::new();
        for entry in &mapping {
            let values = match (&raw, &entry.input) {
                (DigitalInput::Button(index), LogicalInput::Button(other)) if index == other => {
                    Some((0, 1))
                }
                (
                    DigitalInput::Hat { index, direction },
                    LogicalInput::Hat { index: other, mask },
                ) if index == other && *direction & *mask != 0 => Some((0, i32::from(*direction))),
                (
                    DigitalInput::Axis {
                        index,
                        released,
                        pressed,
                    },
                    LogicalInput::Axis { index: other, .. },
                ) if index == other => Some((i32::from(*released), i32::from(*pressed))),
                _ => None,
            };
            let Some((released, pressed)) = values else {
                continue;
            };
            let one = std::slice::from_ref(entry);
            let before = sdl2_mapping::output_value(one, &entry.output, |_| Ok(released))?;
            let after = sdl2_mapping::output_value(one, &entry.output, |_| Ok(pressed))?;
            let binding = if entry.output_range.is_some() {
                // The profile's five-percent deadzone will treat this as centered.
                if before.abs() > 1_638 || after.abs() <= 1_638 {
                    continue;
                }
                Binding::Axis {
                    index: axis_index(&entry.output)
                        .context("simple64 SDL2 axis output is unknown")?,
                    positive: after > 0,
                }
            } else {
                if before != 0 || after == 0 {
                    continue;
                }
                Binding::Button(
                    button_index(&entry.output)
                        .context("simple64 SDL2 button output is unknown")?,
                )
            };
            candidates.insert(binding);
        }
        ensure!(
            candidates.len() == 1,
            "simple64 calibrated control has no unique SDL2 logical output"
        );
        Ok(*candidates.first().expect("one candidate checked"))
    }

    fn calibrated_profile(
        calibration: &Calibration,
        device: &Device,
    ) -> Result<BTreeMap<String, Binding>> {
        ensure!(
            device.is_game_controller,
            "simple64 needs an SDL2-recognized gamepad"
        );
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == PROFILE_ID)
            .context("Missing native simple64 profile")?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            CONTROLS
                .iter()
                .find(|(target, _)| *target == row.target_id)
                .with_context(|| {
                    format!(
                        "simple64 target {} is outside the N64 contract",
                        row.target_id
                    )
                })?;
            let input = row
                .input
                .as_ref()
                .context("simple64 control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("simple64 requires measured native controls")?;
            let endpoints = input.axis.as_ref().map(|axis| {
                lunchbox_controller_probe::linux_classic::AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                }
            });
            let binding = logical_binding(device, native.code, endpoints)?;
            ensure!(
                result
                    .insert((*row.target_id).to_owned(), binding)
                    .is_none(),
                "simple64 target appears twice"
            );
        }
        ensure!(
            result.len() == CONTROLS.len(),
            "simple64 needs every N64 gameplay control"
        );
        Ok(result)
    }

    fn stage_app(source: &Path, target: &Path, executable: &Path) -> Result<PathBuf> {
        ensure!(
            source.is_dir(),
            "simple64 executable parent is not a directory"
        );
        fs::create_dir_all(target)?;
        let name = executable
            .file_name()
            .context("simple64 executable has no filename")?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let from = entry.path();
            let to = target.join(entry.file_name());
            if from == executable {
                fs::copy(&from, &to)?;
                continue;
            }
            if entry.file_name() == "simple64-gui.ini" {
                continue;
            }
            symlink(&from, &to)?;
        }
        Ok(target.join(name))
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) staged_executable: PathBuf,
        topology: InputTopology,
        runtime_paths: Vec<String>,
        snapshot: Snapshot,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            executable: &Path,
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
                    "simple64 physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, &[], cancel)?);
            let runtime_paths = selected
                .iter()
                .map(|path| {
                    topology.resolve_runtime_path(
                        path,
                        initial
                            .devices
                            .iter()
                            .filter_map(|device| device.path.as_deref()),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                runtime_paths.iter().collect::<BTreeSet<_>>().len() == runtime_paths.len(),
                "simple64 players resolved to the same SDL2 device"
            );
            let snapshot = observe(setup, &runtime_paths, cancel)?;
            let mut profile_bindings = Vec::new();
            let mut assignments = Vec::new();
            for (player, runtime_path) in setup.players.iter().zip(&runtime_paths) {
                let selected_device = snapshot.device_at_path(runtime_path)?;
                let profile = calibrated_profile(
                    calibrations
                        .get(&player.controller_id)
                        .context("simple64 calibration disappeared")?,
                    selected_device,
                )?;
                let profile_name = format!("Lunchbox-Player-{}", player.player);
                let gamepad = format!(
                    "{}:{}",
                    selected_device.device_index,
                    selected_device
                        .name
                        .as_deref()
                        .context("simple64 SDL2 gamepad has no name")?
                );
                ensure!(
                    !gamepad.contains(['\n', '\r'])
                        && selected_device
                            .name
                            .as_deref()
                            .is_some_and(|name| !name.contains(':')),
                    "simple64 SDL2 gamepad name cannot contain a colon or newline"
                );
                profile_bindings.push((profile_name.clone(), profile));
                assignments.push((player.player, profile_name, gamepad));
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-simple64-")
                .tempdir()?;
            let core_dir = directory.path().join("core-config");
            fs::create_dir_all(&core_dir)?;
            for name in [
                "mupen64plus.cfg",
                "input-profiles.ini",
                "input-settings.ini",
            ] {
                let source = setup.config_dir.join(name);
                if source.is_file() {
                    fs::copy(source, core_dir.join(name))?;
                }
            }
            let profiles_base = fs::read(core_dir.join("input-profiles.ini")).unwrap_or_default();
            let settings_base = fs::read(core_dir.join("input-settings.ini")).unwrap_or_default();
            fs::write(
                core_dir.join("input-profiles.ini"),
                input_profiles_ini(&profiles_base, &profile_bindings)?,
            )?;
            fs::write(
                core_dir.join("input-settings.ini"),
                input_settings_ini(&settings_base, &assignments)?,
            )?;
            let app_dir = directory.path().join("app");
            let staged_executable = stage_app(
                executable
                    .parent()
                    .context("simple64 executable has no parent")?,
                &app_dir,
                executable,
            )?;
            fs::write(
                app_dir.join("simple64-gui.ini"),
                format!(
                    "[General]\nversion=2\nconfigDirPath={}\n",
                    core_dir.display()
                ),
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.probe_program,
                &setup.sdl_library,
                &setup.content,
                executable,
                &staged_executable,
                &app_dir.join("simple64-gui.ini"),
                &core_dir.join("input-profiles.ini"),
                &core_dir.join("input-settings.ini"),
            ] {
                hashes.insert(path.to_path_buf(), file_hash(path)?);
            }
            let session = Self {
                directory,
                staged_executable,
                topology,
                runtime_paths,
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
                "simple64 session disappeared"
            );
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "simple64 launch input changed"
                );
            }
            let fresh = observe(&self.setup, &self.runtime_paths, cancel)?;
            ensure!(
                routing(fresh.clone())
                    .devices
                    .iter()
                    .map(|d| (&d.device_index, &d.path, &d.mapping, &d.is_game_controller))
                    .collect::<Vec<_>>()
                    == routing(self.snapshot.clone())
                        .devices
                        .iter()
                        .map(|d| (&d.device_index, &d.path, &d.mapping, &d.is_game_controller))
                        .collect::<Vec<_>>(),
                "simple64 SDL2 routing changed before launch"
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
    use super::settings::SavedSetup;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::cancelled,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use anyhow::{Result, ensure};
    use lunchbox_controller_probe::file_hash;
    use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};

    pub(crate) struct NativeSession {
        pub(crate) inputs: super::session::PreparedSession,
        pub(crate) executable: PathBuf,
        pub(crate) setup: SavedSetup,
        pub(crate) plan: LaunchPlan,
    }
    impl NativeSession {
        pub(crate) fn check_health(&self) -> Result<()> {
            self.inputs.check_health()
        }
        pub(crate) fn spawn(
            &mut self,
            plan: &LaunchPlan,
            cancel: &AtomicBool,
        ) -> Result<std::process::Child> {
            ensure!(
                plan == &self.plan,
                "simple64 launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
        }
        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                file_hash(&self.executable)? == self.setup.executable_sha256,
                "simple64 executable differs from saved trusted runtime"
            );
            self.inputs.verify(cancel)
        }
    }
    pub(crate) fn prepare(
        setup: &SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<NativeSession> {
        cancelled(cancel)?;
        setup.validate()?;
        let EmulatorExecutable::Native(executable) = &option.executable else {
            anyhow::bail!("simple64 calibrated launch requires native Linux, not Wine/Flatpak")
        };
        ensure!(
            setup.emulator_id == option.emulator_id && original.environment.is_empty(),
            "simple64 identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?
                && original.arguments.len() == 1
                && original.arguments[0] == setup.content.as_os_str(),
            "simple64 launch executable or content differs from selection"
        );
        ensure!(
            file_hash(&executable)? == setup.executable_sha256,
            "simple64 executable differs from saved trusted runtime"
        );
        let inputs = super::session::PreparedSession::prepare(
            setup,
            &executable,
            calibrations,
            inventory,
            cancel,
        )?;
        let mut plan = original.clone();
        plan.program = inputs.staged_executable.clone();
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
    fn profile_uses_simple64_qsettings_values() {
        let map = CONTROLS
            .iter()
            .enumerate()
            .map(|(i, (target, _))| {
                (
                    (*target).to_owned(),
                    if i < 14 {
                        Binding::Button(i as u32)
                    } else {
                        Binding::Axis {
                            index: ((i - 14) / 2) as u32,
                            positive: i % 2 == 1,
                        }
                    },
                )
            })
            .collect();
        let text = profile_ini("Lunchbox-Player-1", &map).unwrap();
        assert!(
            text.contains("A=0,1\n")
                && text.contains("AxisLeft=0,2,-1\n")
                && text.ends_with("Sensitivity=100\n")
        );
    }
    #[test]
    fn settings_assign_exactly_four_ports() {
        let text = input_settings_ini(
            b"version=1\n[Controller1]\nGamepad=Auto\n",
            &[(1, "Lunchbox-Player-1".into(), "0:Pad".into())],
        )
        .unwrap();
        assert_eq!(text.matches("version=").count(), 1);
        assert!(text.starts_with("version=2\n"));
        assert!(
            text.contains("[Controller1]\nProfile=Lunchbox-Player-1\nGamepad=0:Pad\nPak=Memory\n")
        );
        assert!(text.contains("[Controller2]\nProfile=Auto\nGamepad=None\nPak=None\n"));
    }
}
