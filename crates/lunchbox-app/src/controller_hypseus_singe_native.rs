//! Hypseus Singe SDL_Gamepad input-file writer.
//!
//! Pinned upstream: DirtBagXon/hypseus-singe `a16e2521ee3233ef20c44e562008c47f4b4293df`.
//! The native parser consumes `hypinput_gamepad.ini` (or an alternate `.ini`
//! selected with `-keymapfile`) from the Hypseus home directory.  This module
//! patches only the four source-defined gamepad columns in `[KEYBOARD]` lines;
//! SDL device identity and `gamecontrollerdb.txt` mappings remain runtime
//! responsibilities.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, PathBuf},
};

pub(crate) const SOURCE_COMMIT: &str = "a16e2521ee3233ef20c44e562008c47f4b4293df";
pub(crate) const PROFILE_ID: &str = "hypseus-singe:sdl-gamepad-ini-v1";

const SWITCHES: [&str; 22] = [
    "KEY_UP",
    "KEY_LEFT",
    "KEY_DOWN",
    "KEY_RIGHT",
    "KEY_START1",
    "KEY_START2",
    "KEY_BUTTON1",
    "KEY_BUTTON2",
    "KEY_BUTTON3",
    "KEY_COIN1",
    "KEY_COIN2",
    "KEY_SKILL1",
    "KEY_SKILL2",
    "KEY_SKILL3",
    "KEY_SERVICE",
    "KEY_TEST",
    "KEY_RESET",
    "KEY_SCREENSHOT",
    "KEY_QUIT",
    "KEY_PAUSE",
    "KEY_CONSOLE",
    "KEY_TILT",
];

const BUTTONS: [&str; 18] = [
    "0",
    "BUTTON_A",
    "BUTTON_B",
    "BUTTON_X",
    "BUTTON_Y",
    "BUTTON_BACK",
    "BUTTON_GUIDE",
    "BUTTON_START",
    "BUTTON_LEFTSTICK",
    "BUTTON_RIGHTSTICK",
    "BUTTON_LEFTSHOULDER",
    "BUTTON_RIGHTSHOULDER",
    "BUTTON_DPAD_UP",
    "BUTTON_DPAD_DOWN",
    "BUTTON_DPAD_LEFT",
    "BUTTON_DPAD_RIGHT",
    "AXIS_TRIGGER_LEFT",
    "AXIS_TRIGGER_RIGHT",
];
const AXES: [&str; 9] = [
    "0",
    "AXIS_LEFT_UP",
    "AXIS_LEFT_DOWN",
    "AXIS_LEFT_LEFT",
    "AXIS_LEFT_RIGHT",
    "AXIS_RIGHT_UP",
    "AXIS_RIGHT_DOWN",
    "AXIS_RIGHT_LEFT",
    "AXIS_RIGHT_RIGHT",
];

/// Source-level SDL_Gamepad values. `0` means no binding, matching the sample
/// file and the parser's zero sentinel. The first and second keyboard columns
/// are intentionally left unchanged by the writer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GamepadMapping {
    pub switch: String,
    pub pad0_button: String,
    pub pad0_axis: String,
    pub pad1_button: String,
    pub pad1_axis: String,
}

fn known_switch(name: &str) -> bool {
    SWITCHES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn known_button(value: &str) -> bool {
    BUTTONS.contains(&value)
}

fn known_axis(value: &str) -> bool {
    AXES.contains(&value)
}

fn parse_line(line: &str) -> Option<(&str, &str, &str)> {
    let (key, rest) = line.split_once('=')?;
    let name = key.trim();
    if !known_switch(name) {
        return None;
    }
    let (values, comment) = rest.split_once('#').map_or((rest, ""), |(v, c)| (v, c));
    Some((name, values.trim(), comment))
}

fn validate_mappings(mappings: &[GamepadMapping]) -> Result<()> {
    ensure!(
        !mappings.is_empty() && mappings.len() <= SWITCHES.len(),
        "Hypseus needs one to {} mappings",
        SWITCHES.len()
    );
    let mut switches = BTreeSet::new();
    for mapping in mappings {
        ensure!(
            known_switch(&mapping.switch) && switches.insert(mapping.switch.to_ascii_uppercase()),
            "Unknown or duplicate Hypseus switch {}",
            mapping.switch
        );
        ensure!(
            known_button(&mapping.pad0_button),
            "Unknown SDL gamepad button {}",
            mapping.pad0_button
        );
        ensure!(
            known_axis(&mapping.pad0_axis),
            "Unknown SDL gamepad axis {}",
            mapping.pad0_axis
        );
        ensure!(
            known_button(&mapping.pad1_button),
            "Unknown SDL gamepad button {}",
            mapping.pad1_button
        );
        ensure!(
            known_axis(&mapping.pad1_axis),
            "Unknown SDL gamepad axis {}",
            mapping.pad1_axis
        );
    }
    Ok(())
}

/// Patches a complete source-generated gamepad INI. Every requested switch
/// must occur exactly once in `[KEYBOARD]`; malformed lines are rejected rather
/// than emitting a file Hypseus would partially ignore.
pub(crate) fn patch_gamepad_config(baseline: &[u8], mappings: &[GamepadMapping]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "Hypseus input config is too large"
    );
    let original = std::str::from_utf8(baseline).context("Hypseus input config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "Hypseus input config contains a NUL byte"
    );
    ensure!(
        original
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case("[KEYBOARD]")),
        "Hypseus input config is missing [KEYBOARD]"
    );
    validate_mappings(mappings)?;

    let mut output = String::with_capacity(original.len() + mappings.len() * 64);
    let mut in_keyboard = false;
    let mut seen = vec![false; mappings.len()];
    for line in original.split_inclusive('\n') {
        let (content, ending) = line.strip_suffix('\n').map_or((line, ""), |line| {
            line.strip_suffix('\r')
                .map_or((line, "\n"), |line| (line, "\r\n"))
        });
        let trimmed = content.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_keyboard = trimmed.eq_ignore_ascii_case("[KEYBOARD]");
        }
        if in_keyboard {
            if let Some((name, values, comment)) = parse_line(content) {
                if let Some(index) = mappings
                    .iter()
                    .position(|mapping| mapping.switch.eq_ignore_ascii_case(name))
                {
                    ensure!(!seen[index], "Duplicate Hypseus switch {}", name);
                    let tokens: Vec<&str> = values.split_whitespace().collect();
                    // The source parser requires the two keyboard values and
                    // the first gamepad-button value, then treats the first
                    // axis and both player-two values as optional. The
                    // official gamepad file contains both five- and six-value
                    // lines, so accept the full source grammar and normalize
                    // every patched line to all six values.
                    ensure!(
                        (3..=6).contains(&tokens.len()),
                        "Malformed Hypseus [KEYBOARD] line for {}",
                        name
                    );
                    let mapping = &mappings[index];
                    output.push_str(name);
                    output.push_str(" = ");
                    output.push_str(tokens[0]);
                    output.push(' ');
                    output.push_str(tokens[1]);
                    output.push(' ');
                    output.push_str(&mapping.pad0_button);
                    output.push(' ');
                    output.push_str(&mapping.pad0_axis);
                    output.push(' ');
                    output.push_str(&mapping.pad1_button);
                    output.push(' ');
                    output.push_str(&mapping.pad1_axis);
                    if !comment.is_empty() {
                        output.push_str(" #");
                        output.push_str(comment.trim_end());
                    }
                    output.push_str(ending);
                    seen[index] = true;
                    continue;
                }
            }
        }
        output.push_str(line);
    }
    ensure!(
        seen.iter().all(|value| *value),
        "Hypseus config is missing requested switches"
    );
    Ok(output)
}

pub(crate) mod settings {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Player {
        /// Hypseus gamepad column, zero or one.
        pub player: u8,
        pub controller_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        /// A complete executable-generated or official hypinput_gamepad.ini.
        pub config_path: PathBuf,
        /// Writable native NVRAM directory retained outside the private config home.
        pub ram_directory: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        /// Exact database Hypseus and the probe must both load.
        pub mapping_database: PathBuf,
        #[serde(default)]
        pub runtime_libraries: Vec<PathBuf>,
        pub executable_sha256: String,
        pub players: Vec<Player>,
        pub mappings: Vec<GamepadMapping>,
    }

    fn valid_path(path: &std::path::Path) -> bool {
        path.is_absolute()
            && !path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Hypseus needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.ram_directory,
                &self.probe_program,
                &self.sdl_library,
                &self.mapping_database,
            ]
            .into_iter()
            .chain(self.runtime_libraries.iter())
            {
                ensure!(
                    valid_path(path),
                    "Hypseus setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.content.is_file()
                    && self.config_path.is_file()
                    && self.ram_directory.is_dir()
                    && self.probe_program.is_file()
                    && self.sdl_library.is_file()
                    && self.mapping_database.is_file(),
                "Hypseus setup files or NVRAM directory are absent"
            );
            ensure!(
                self.content
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
                    && self
                        .config_path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("ini")),
                "Hypseus content must be a framefile and config_path must be an INI"
            );
            ensure!(
                self.runtime_libraries.len() <= 32,
                "Too many Hypseus runtime dependencies"
            );
            let runtime_paths = self.runtime_libraries.iter().collect::<BTreeSet<_>>();
            ensure!(
                runtime_paths.len() == self.runtime_libraries.len(),
                "Duplicate Hypseus runtime dependency"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Hypseus needs a trusted executable SHA-256"
            );
            ensure!(
                (1..=2).contains(&self.players.len()),
                "Hypseus supports one or two mapped gamepads"
            );
            let mut slots = BTreeSet::new();
            let mut controllers = BTreeSet::new();
            for player in &self.players {
                ensure!(
                    player.player <= 1
                        && slots.insert(player.player)
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Hypseus player slots and physical controllers must be distinct"
                );
            }
            ensure!(
                slots.iter().copied().eq(0..self.players.len() as u8),
                "Hypseus player slots must start at zero without gaps"
            );
            ensure!(
                self.players
                    .iter()
                    .enumerate()
                    .all(|(index, player)| player.player as usize == index),
                "Hypseus players must be ordered by their contiguous slot"
            );
            validate_mappings(&self.mappings)?;
            for switch in ["KEY_UP", "KEY_DOWN", "KEY_LEFT", "KEY_RIGHT", "KEY_BUTTON1"] {
                let mapping = self
                    .mappings
                    .iter()
                    .find(|mapping| mapping.switch.eq_ignore_ascii_case(switch))
                    .with_context(|| format!("Hypseus required mapping {switch} is absent"))?;
                ensure!(
                    mapping.pad0_button != "0" || mapping.pad0_axis != "0",
                    "Hypseus player zero has no {switch} binding"
                );
                if self.players.len() == 2 {
                    ensure!(
                        mapping.pad1_button != "0" || mapping.pad1_axis != "0",
                        "Hypseus player one has no {switch} binding"
                    );
                }
            }
            for (player, start, coin) in [
                (0, "KEY_START1", "KEY_COIN1"),
                (1, "KEY_START2", "KEY_COIN2"),
            ] {
                if player >= self.players.len() {
                    continue;
                }
                for switch in [start, coin] {
                    let mapping = self
                        .mappings
                        .iter()
                        .find(|mapping| mapping.switch.eq_ignore_ascii_case(switch))
                        .with_context(|| format!("Hypseus required mapping {switch} is absent"))?;
                    let (button, axis) = if player == 0 {
                        (&mapping.pad0_button, &mapping.pad0_axis)
                    } else {
                        (&mapping.pad1_button, &mapping.pad1_axis)
                    };
                    ensure!(
                        button != "0" || axis != "0",
                        "Hypseus player {player} has no {switch} binding"
                    );
                }
            }
            if self.players.len() == 1 {
                ensure!(
                    self.mappings
                        .iter()
                        .all(|mapping| mapping.pad1_button == "0" && mapping.pad1_axis == "0"),
                    "Hypseus player-one columns require a second selected controller"
                );
            }
            Ok(())
        }

        pub(crate) fn review(&self) -> Result<serde_json::Value> {
            self.validate()?;
            Ok(serde_json::json!({
                "players": [],
                "mapping_count": self.mappings.len(),
                "controller_slots": self.players.iter().map(|player| serde_json::json!({"player":player.player,"controller_id":player.controller_id})).collect::<Vec<_>>(),
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Hypseus Singe native Linux launch uses exact SDL3 Gamepad order, a private keymap/home, and a separate writable NVRAM directory. The pinned executable has accepted this parser/input shape through live SDL enumeration; gameplay input and save/load behavior remain unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Hypseus saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Hypseus emulator/content setup"
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
        controller_native_process::{cancelled, capture},
        controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{Snapshot, file_hash};
    use std::{
        fs,
        os::unix::{ffi::OsStrExt, fs::MetadataExt},
        process::Command,
        sync::atomic::AtomicBool,
    };

    type Routing = Vec<(String, String, Option<String>, Option<u16>, Option<String>)>;

    fn routing(snapshot: &Snapshot) -> Routing {
        snapshot
            .devices
            .iter()
            .map(|device| {
                (
                    device.path.clone().unwrap_or_default(),
                    device.guid.clone(),
                    device.gamepad_name.clone(),
                    device.gamepad_index,
                    device.mapping.clone(),
                )
            })
            .collect()
    }

    fn observe(
        setup: &settings::SavedSetup,
        runtime_paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl-library")
            .arg(&setup.sdl_library)
            .arg("--mapping-db")
            .arg(&setup.mapping_database);
        for library in &setup.runtime_libraries {
            command.arg("--runtime-library").arg(library);
        }
        for path in runtime_paths {
            command.arg("--match-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid Hypseus SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot.mapping_database_sha256.as_deref()
                    == Some(file_hash(&setup.mapping_database)?.as_str()),
            "Hypseus helper inspected different SDL3 inputs"
        );
        ensure!(
            snapshot.runtime_libraries.len() == setup.runtime_libraries.len(),
            "Hypseus helper loaded a different dependency set"
        );
        for (observed, expected) in snapshot
            .runtime_libraries
            .iter()
            .zip(&setup.runtime_libraries)
        {
            ensure!(
                observed.path.canonicalize()? == expected.canonicalize()?
                    && observed.sha256 == file_hash(expected)?,
                "Hypseus helper loaded a different runtime dependency"
            );
        }
        Ok(snapshot)
    }

    fn source_argument(path: &std::path::Path, name: &str) -> Result<()> {
        ensure!(
            path.as_os_str().as_bytes().len() <= 80,
            "Hypseus {name} exceeds its 80-byte source argument buffer"
        );
        Ok(())
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) home: PathBuf,
        pub(crate) keymap_name: String,
        pub(crate) ram_directory: PathBuf,
        pub(crate) gamepad_order: [u8; 8],
        pub(crate) mapping_database: PathBuf,
        runtime_paths: Vec<String>,
        topology: InputTopology,
        routing: Routing,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
        ram_identity: (u64, u64),
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            inventory: &[ControllerDevice],
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.validate()?;
            let mut selected = Vec::new();
            for player in &setup.players {
                let matches = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    matches.len() == 1 && !matches[0].is_virtual,
                    "Hypseus physical controller is missing or ambiguous"
                );
                selected.push(matches[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = observe(setup, &[], cancel)?;
            ensure!(
                initial
                    .devices
                    .iter()
                    .filter(|device| device.is_gamepad)
                    .count()
                    <= 8,
                "Hypseus source supports at most eight SDL gamepads"
            );
            let visible_paths = initial
                .devices
                .iter()
                .filter_map(|device| device.path.as_deref())
                .collect::<Vec<_>>();
            let runtime_paths = selected
                .iter()
                .map(|path| topology.resolve_runtime_path(path, visible_paths.iter().copied()))
                .collect::<Result<Vec<_>>>()?;
            let selected_snapshot = observe(setup, &runtime_paths, cancel)?;
            ensure!(
                routing(&initial) == routing(&selected_snapshot),
                "Hypseus SDL3 routing changed during capture"
            );
            let selected_indexes = runtime_paths
                .iter()
                .map(|path| {
                    selected_snapshot
                        .device_at_path(path)?
                        .gamepad_index
                        .context("Hypseus selected device has no SDL Gamepad index")
                })
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                selected_indexes.iter().all(|index| *index < 8)
                    && selected_indexes.iter().collect::<BTreeSet<_>>().len()
                        == selected_indexes.len(),
                "Hypseus selected Gamepad indexes are invalid or duplicated"
            );
            let order = selected_indexes
                .iter()
                .copied()
                .chain((0u16..8).filter(|index| !selected_indexes.contains(index)))
                .map(|index| u8::try_from(index).expect("Hypseus index is bounded"))
                .collect::<Vec<_>>();
            let gamepad_order: [u8; 8] = order
                .try_into()
                .expect("selected indexes plus remainder form a permutation");

            let directory = tempfile::Builder::new()
                .prefix("lunchbox-hypseus-")
                .tempdir()?;
            let home = directory.path().join("home");
            fs::create_dir_all(&home)?;
            let keymap_name = "lunchbox-gamepad.ini".to_owned();
            let keymap = home.join(&keymap_name);
            fs::write(
                &keymap,
                patch_gamepad_config(&fs::read(&setup.config_path)?, &setup.mappings)?,
            )?;
            let mapping_database = home.join("gamecontrollerdb.txt");
            fs::copy(&setup.mapping_database, &mapping_database)?;
            let ram_directory = setup.ram_directory.canonicalize()?;
            source_argument(&home, "private home")?;
            source_argument(&ram_directory, "NVRAM directory")?;
            let metadata = fs::metadata(&ram_directory)?;
            let ram_identity = (metadata.dev(), metadata.ino());
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.probe_program,
                &setup.sdl_library,
                &setup.mapping_database,
                &setup.config_path,
                &setup.content,
                &keymap,
                &mapping_database,
            ]
            .into_iter()
            .chain(setup.runtime_libraries.iter())
            {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                home,
                keymap_name,
                ram_directory,
                gamepad_order,
                mapping_database,
                runtime_paths,
                topology,
                routing: routing(&selected_snapshot),
                setup: setup.clone(),
                hashes,
                ram_identity,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir() && self.home.is_dir(),
                "Hypseus private home disappeared"
            );
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "Hypseus launch input changed"
                );
            }
            let ram = fs::metadata(&self.ram_directory)?;
            ensure!(
                (ram.dev(), ram.ino()) == self.ram_identity,
                "Hypseus NVRAM directory changed"
            );
            let current = observe(&self.setup, &self.runtime_paths, cancel)?;
            ensure!(
                routing(&current) == self.routing,
                "Hypseus SDL3 routing changed"
            );
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()?;
            let ram = fs::metadata(&self.ram_directory)?;
            ensure!(
                (ram.dev(), ram.ino()) == self.ram_identity,
                "Hypseus NVRAM directory changed"
            );
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) mod native_command {
    use super::*;
    use crate::{
        controller_native_process::cancelled,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use anyhow::bail;
    use lunchbox_controller_probe::file_hash;
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt, sync::atomic::AtomicBool};

    fn option(argument: &OsStr) -> Option<String> {
        argument.to_str().map(str::to_ascii_lowercase)
    }

    pub(super) fn validate_source_arguments(arguments: &[std::ffi::OsString]) -> Result<()> {
        for argument in arguments {
            ensure!(
                argument.as_os_str().as_bytes().len() <= 80,
                "Hypseus command-line token exceeds its 80-byte source buffer"
            );
        }
        Ok(())
    }

    fn overlay_arguments(
        original: &[std::ffi::OsString],
        inputs: &session::PreparedSession,
    ) -> Result<Vec<std::ffi::OsString>> {
        for forbidden in [
            "-ramdir",
            "-keymapfile",
            "-config",
            "-gamepad",
            "-gamepad_reorder",
        ] {
            ensure!(
                !original
                    .iter()
                    .any(|argument| option(argument).as_deref() == Some(forbidden)),
                "Hypseus launch already contains {forbidden}"
            );
        }
        let home_positions = original
            .iter()
            .enumerate()
            .filter(|(_, argument)| option(argument).as_deref() == Some("-homedir"))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        ensure!(
            home_positions.len() == 1 && home_positions[0] + 1 < original.len(),
            "Hypseus launch needs exactly one generated -homedir"
        );
        let mut arguments = original.to_vec();
        arguments[home_positions[0] + 1] = inputs.home.as_os_str().to_owned();
        arguments.extend([
            "-ramdir".into(),
            inputs.ram_directory.as_os_str().to_owned(),
            "-keymapfile".into(),
            inputs.keymap_name.clone().into(),
            "-gamepad_reorder".into(),
        ]);
        arguments.extend(
            inputs
                .gamepad_order
                .iter()
                .map(|index| index.to_string().into()),
        );
        validate_source_arguments(&arguments)?;
        Ok(arguments)
    }

    pub(crate) struct NativeSession {
        inputs: session::PreparedSession,
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
                "Hypseus executable differs from the trusted setup"
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
                "Hypseus launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
        }
    }

    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        inventory: &[ControllerDevice],
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<NativeSession> {
        cancelled(cancel)?;
        setup.validate()?;
        let EmulatorExecutable::Native(executable) = &option.executable else {
            bail!("Hypseus calibrated launch requires native Linux");
        };
        ensure!(
            setup.emulator_id == option.emulator_id && original.environment.is_empty(),
            "Hypseus identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?
                && original
                    .arguments
                    .iter()
                    .any(|argument| argument == setup.content.as_os_str()),
            "Hypseus executable or framefile differs from the saved setup"
        );
        ensure!(
            file_hash(&executable)? == setup.executable_sha256,
            "Hypseus executable differs from the trusted setup"
        );
        let inputs = session::PreparedSession::prepare(setup, inventory, cancel)?;
        let mut plan = original.clone();
        plan.arguments = overlay_arguments(&plan.arguments, &inputs)?;
        plan.environment.push((
            "SDL_GAMECONTROLLERCONFIG_FILE".into(),
            inputs.mapping_database.as_os_str().to_owned(),
        ));
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
    use std::fs;

    fn baseline() -> String {
        String::from(
            "[KEYBOARD]\r\nKEY_UP = SDLK_UP 0 BUTTON_DPAD_UP AXIS_LEFT_UP 0 0 # keep\r\nKEY_BUTTON1 = SDLK_LCTRL 0 BUTTON_A 0 0 0\r\n[MOUSE]\r\nMOUSE_BUTTON1 = KEY_BUTTON3\r\n",
        )
    }

    #[test]
    fn patches_gamepad_columns_and_preserves_comment() {
        let mappings = [GamepadMapping {
            switch: "KEY_UP".into(),
            pad0_button: "BUTTON_Y".into(),
            pad0_axis: "AXIS_RIGHT_UP".into(),
            pad1_button: "0".into(),
            pad1_axis: "0".into(),
        }];
        let result = patch_gamepad_config(baseline().as_bytes(), &mappings).unwrap();
        assert!(result.contains("KEY_UP = SDLK_UP 0 BUTTON_Y AXIS_RIGHT_UP 0 0 # keep\r\n"));
        assert!(result.contains("KEY_BUTTON1 = SDLK_LCTRL 0 BUTTON_A 0 0 0\r\n"));
    }

    #[test]
    fn rejects_unknown_values_and_missing_switch() {
        let mut mappings = vec![GamepadMapping {
            switch: "KEY_UP".into(),
            pad0_button: "BUTTON_NOPE".into(),
            pad0_axis: "0".into(),
            pad1_button: "0".into(),
            pad1_axis: "0".into(),
        }];
        assert!(patch_gamepad_config(baseline().as_bytes(), &mappings).is_err());
        mappings[0].pad0_button = "BUTTON_A".into();
        mappings[0].switch = "KEY_QUIT".into();
        assert!(patch_gamepad_config(baseline().as_bytes(), &mappings).is_err());
    }

    #[test]
    fn accepts_source_trigger_names_but_rejects_numeric_aliases() {
        let mut mappings = vec![GamepadMapping {
            switch: "KEY_BUTTON1".into(),
            pad0_button: "AXIS_TRIGGER_LEFT".into(),
            pad0_axis: "0".into(),
            pad1_button: "0".into(),
            pad1_axis: "0".into(),
        }];
        let result = patch_gamepad_config(baseline().as_bytes(), &mappings).unwrap();
        assert!(result.contains("KEY_BUTTON1 = SDLK_LCTRL 0 AXIS_TRIGGER_LEFT 0 0 0\r\n"));

        mappings[0].pad0_button = "1".into();
        assert!(patch_gamepad_config(baseline().as_bytes(), &mappings).is_err());
        mappings[0].pad0_button = "button_a".into();
        assert!(patch_gamepad_config(baseline().as_bytes(), &mappings).is_err());

        let short = baseline().replace(
            "KEY_BUTTON1 = SDLK_LCTRL 0 BUTTON_A 0 0 0",
            "KEY_BUTTON1 = SDLK_LCTRL 0",
        );
        mappings[0].pad0_button = "BUTTON_A".into();
        assert!(patch_gamepad_config(short.as_bytes(), &mappings).is_err());
    }

    #[test]
    fn accepts_official_five_value_lines_and_normalizes_them() {
        let official_shape = baseline().replace(
            "KEY_BUTTON1 = SDLK_LCTRL 0 BUTTON_A 0 0 0",
            "KEY_BUTTON1 = SDLK_LCTRL 0 BUTTON_A 0 0",
        );
        let mappings = [GamepadMapping {
            switch: "KEY_BUTTON1".into(),
            pad0_button: "BUTTON_Y".into(),
            pad0_axis: "0".into(),
            pad1_button: "BUTTON_B".into(),
            pad1_axis: "0".into(),
        }];
        let result = patch_gamepad_config(official_shape.as_bytes(), &mappings).unwrap();
        assert!(result.contains("KEY_BUTTON1 = SDLK_LCTRL 0 BUTTON_Y 0 BUTTON_B 0\r\n"));
    }

    fn required_mappings() -> Vec<GamepadMapping> {
        [
            ("KEY_UP", "0", "AXIS_LEFT_UP"),
            ("KEY_DOWN", "0", "AXIS_LEFT_DOWN"),
            ("KEY_LEFT", "0", "AXIS_LEFT_LEFT"),
            ("KEY_RIGHT", "0", "AXIS_LEFT_RIGHT"),
            ("KEY_BUTTON1", "BUTTON_A", "0"),
            ("KEY_START1", "BUTTON_START", "0"),
            ("KEY_COIN1", "BUTTON_BACK", "0"),
        ]
        .into_iter()
        .map(|(switch, button, axis)| GamepadMapping {
            switch: switch.into(),
            pad0_button: button.into(),
            pad0_axis: axis.into(),
            pad1_button: "0".into(),
            pad1_axis: "0".into(),
        })
        .collect()
    }

    #[test]
    fn saved_setup_requires_real_inputs_and_contiguous_players() {
        let temp = tempfile::tempdir().unwrap();
        let content = temp.path().join("game.txt");
        let config_path = temp.path().join("hypinput_gamepad.ini");
        let ram_directory = temp.path().join("ram");
        let probe_program = temp.path().join("probe");
        let sdl_library = temp.path().join("libSDL3.so");
        let mapping_database = temp.path().join("gamecontrollerdb.txt");
        for path in [
            &content,
            &config_path,
            &probe_program,
            &sdl_library,
            &mapping_database,
        ] {
            fs::write(path, b"fixture").unwrap();
        }
        fs::create_dir(&ram_directory).unwrap();
        let mut setup = settings::SavedSetup {
            emulator_id: "hypseus".into(),
            content,
            config_path,
            ram_directory,
            probe_program,
            sdl_library,
            mapping_database,
            runtime_libraries: Vec::new(),
            executable_sha256: "a".repeat(64),
            players: vec![settings::Player {
                player: 0,
                controller_id: "physical-pad".into(),
            }],
            mappings: required_mappings(),
        };
        setup.validate().unwrap();

        setup.players[0].player = 1;
        assert!(setup.validate().is_err());

        setup.players = vec![
            settings::Player {
                player: 0,
                controller_id: "physical-pad-zero".into(),
            },
            settings::Player {
                player: 1,
                controller_id: "physical-pad-one".into(),
            },
        ];
        for mapping in &mut setup.mappings {
            mapping.pad1_button = mapping.pad0_button.clone();
            mapping.pad1_axis = mapping.pad0_axis.clone();
        }
        setup.mappings.extend([
            GamepadMapping {
                switch: "KEY_START2".into(),
                pad0_button: "0".into(),
                pad0_axis: "0".into(),
                pad1_button: "BUTTON_START".into(),
                pad1_axis: "0".into(),
            },
            GamepadMapping {
                switch: "KEY_COIN2".into(),
                pad0_button: "0".into(),
                pad0_axis: "0".into(),
                pad1_button: "BUTTON_BACK".into(),
                pad1_axis: "0".into(),
            },
        ]);
        setup.validate().unwrap();
        setup.players.swap(0, 1);
        assert!(setup.validate().is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rejects_any_source_argument_longer_than_eighty_bytes() {
        use std::ffi::OsString;

        native_command::validate_source_arguments(&[OsString::from("x".repeat(80))]).unwrap();
        assert!(
            native_command::validate_source_arguments(&[OsString::from("x".repeat(81))]).is_err()
        );
    }
}
