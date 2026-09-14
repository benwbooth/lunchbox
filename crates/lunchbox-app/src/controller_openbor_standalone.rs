//! OpenBOR standalone-native packed-settings writer.
//!
//! Pinned source: `DCurrent/openbor` at
//! `9d81480f8481fbb9e76b0b5f2a5dfa408376761a`. The Linux/SDL build persists
//! `#pragma pack(1)` `s_savedata` in `./Saves/<pak-stem>.cfg` (or
//! `Saves/default.cfg`): `compatibleversion` (`0x00033749`), display/audio
//! settings, `keys[4][13]` player key codes, and `joyrumble[4]`. Player 1
//! defaults are SDL scancodes; extra players default to `CONTROL_NONE`.
//! Joystick codes address SDL joystick slots directly: `JOYBUTTON(i,btn)` =
//! `1 + i*64 + btn`, `JOYAXIS(i,axis,dir)` = button-count + `2*axis + dir`,
//! hats at button-count + `2*axes + 4*hat + dir` (`control.h`, `control.c`).
//! `SDL_JoystickOpen(i)` opens slots 0..3, so the session proves the pad
//! holds slot 0 for player 1. Content is the `.pak` path passed as argv[1];
//! the per-pak config stem derives from its filename (`getPakName`).

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "9d81480f8481fbb9e76b0b5f2a5dfa408376761a";
pub(crate) const PROFILE_ID: &str = "openbor:standalone-brawler";
pub(crate) const COMPATIBLE_VERSION: u32 = 0x00033749;
pub(crate) const JOY_MAX_INPUTS: i32 = 64;
pub(crate) const JOY_LIST_FIRST: i32 = 600;
pub(crate) const CONTROL_NONE: i32 = (600 + 1) + (64 * 99);

/// Player action order matching `e_key_id` (SDID_* indices 0..12).
pub(crate) const ACTIONS: [(&str, usize); 12] = [
    ("up", 0),
    ("down", 1),
    ("left", 2),
    ("right", 3),
    ("button1", 4),
    ("button2", 5),
    ("button3", 6),
    ("button4", 7),
    ("button5", 8),
    ("button6", 9),
    ("start", 10),
    ("select", 11),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 12] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("button1", "Attack"),
    ("button2", "Attack 2"),
    ("button3", "Attack 3"),
    ("button4", "Attack 4"),
    ("button5", "Jump"),
    ("button6", "Special"),
    ("start", "Start"),
    ("select", "Screenshot"),
];

/// SDL scancodes for the player-1 keyboard defaults (`control.h`). These
/// seed slots the session does not rebind; they are SDL constants, not
/// invented mappings.
pub(crate) const DEFAULT_SCANCODES: [i32; 12] = [
    82, // up
    81, // down
    80, // left
    79, // right
    4,  // a: attack
    22, // s: attack2
    29, // z: attack3
    27, // x: attack4
    7,  // d: jump
    9,  // f: special
    40, // return: start
    69, // f12: screenshot
];

/// Joystick key code for slot `slot`, raw SDL button `button`. Persisted
/// codes carry the `JOY_LIST_FIRST` base (`control_scankey` returns
/// `JOY_LIST_FIRST + lastjoy`); the runtime strips it back off.
pub(crate) fn joy_button(slot: u32, button: u32, num_buttons: u32) -> Result<i32> {
    ensure!(slot <= 3, "OpenBOR joystick slot is out of range");
    ensure!(
        button < num_buttons,
        "OpenBOR button is outside the measured pad"
    );
    Ok(JOY_LIST_FIRST + 1 + (slot as i32) * JOY_MAX_INPUTS + button as i32)
}

/// Joystick key code for a hat direction. `hat` and `dir` follow the source
/// `hatfirst` formula: buttons, then `2*axes`, then `4*hat + dir`.
pub(crate) fn joy_hat(
    slot: u32,
    hat: u32,
    dir: u32,
    num_buttons: u32,
    num_axes: u32,
    num_hats: u32,
) -> Result<i32> {
    ensure!(slot <= 3, "OpenBOR joystick slot is out of range");
    ensure!(hat < num_hats, "OpenBOR hat is outside the measured pad");
    ensure!(dir <= 3, "OpenBOR hat direction is out of range");
    Ok(JOY_LIST_FIRST
        + 1
        + (slot as i32) * JOY_MAX_INPUTS
        + num_buttons as i32
        + 2 * num_axes as i32
        + 4 * hat as i32
        + dir as i32)
}

/// Joystick key code for an axis half: `axisfirst + dir` with `dir` 0 for
/// negative and 1 for positive.
pub(crate) fn joy_axis(
    slot: u32,
    axis: u32,
    positive: bool,
    num_buttons: u32,
    num_axes: u32,
) -> Result<i32> {
    ensure!(slot <= 3, "OpenBOR joystick slot is out of range");
    ensure!(axis < num_axes, "OpenBOR axis is outside the measured pad");
    Ok(JOY_LIST_FIRST
        + 1
        + (slot as i32) * JOY_MAX_INPUTS
        + num_buttons as i32
        + 2 * axis as i32
        + i32::from(positive))
}

/// Offsets into the packed `s_savedata` for the fields the session writes.
/// Derived from the pinned header with `#pragma pack(1)`: 9 leading ints
/// (36 bytes), `keys[4][13]` at 36, `joyrumble[4]` at 36+208=244.
pub(crate) const KEYS_OFFSET: usize = 36;
pub(crate) const JOYRUMBLE_OFFSET: usize = 244;
pub(crate) const MIN_SIZE: usize = 260;

/// Render the session `Saves/<pak-stem>.cfg`: the user's real config bytes
/// with `compatibleversion` pinned, player-1 `keys[0]` replaced by the
/// measured joystick codes, and players 2-4 cleared to `CONTROL_NONE` (the
/// source's own `clearbuttons` behavior for non-player-1 slots).
pub(crate) fn session_config(baseline: &[u8], player1: &[i32; 12]) -> Result<Vec<u8>> {
    ensure!(
        baseline.len() >= MIN_SIZE,
        "OpenBOR config is smaller than the pinned settings header"
    );
    ensure!(baseline.len() <= 64 * 1024, "OpenBOR config is too large");
    let mut out = baseline.to_vec();
    out[0..4].copy_from_slice(&COMPATIBLE_VERSION.to_le_bytes());
    for (index, code) in player1.iter().enumerate() {
        let at = KEYS_OFFSET + index * 4;
        out[at..at + 4].copy_from_slice(&code.to_le_bytes());
    }
    for player in 1..4 {
        for index in 0..12 {
            let at = KEYS_OFFSET + (player * 13 + index) * 4;
            out[at..at + 4].copy_from_slice(&CONTROL_NONE.to_le_bytes());
        }
        let at = JOYRUMBLE_OFFSET + player * 4;
        out[at..at + 4].copy_from_slice(&0i32.to_le_bytes());
    }
    Ok(out)
}

/// Config stem for content: the filename with any extension replaced by
/// `.cfg`, matching `getPakName(name, 4)`.
pub(crate) fn config_stem(content: &std::path::Path) -> Result<String> {
    let stem = content
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("OpenBOR content filename is not UTF-8")?;
    ensure!(
        !stem.is_empty() && stem.len() <= 128 && !stem.contains(['/', '\\', '\0']),
        "OpenBOR content stem is invalid"
    );
    Ok(format!("{stem}.cfg"))
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use std::{collections::HashMap, path::PathBuf};

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
        /// The user's real per-pak `.cfg`; player-1 keys are replaced while
        /// display/audio settings and the version stamp survive.
        pub config_source: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "OpenBOR setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_source,
                &self.probe_program,
                &self.sdl_library,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "OpenBOR setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                config_stem(&self.content).is_ok(),
                "OpenBOR content needs a valid pak stem"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "OpenBOR setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing OpenBOR native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "OpenBOR supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "OpenBOR player needs a saved controller identity"
            );
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
                .context("Missing OpenBOR native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("OpenBOR controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "OpenBOR mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "OpenBOR needs native calibration for every brawler control"
            );
            Ok(serde_json::json!({
                "profile_id": PROFILE_ID,
                "player": player.player,
                "controller_id": player.controller_id,
                "source_layout": calibration.layout,
                "target_layout": profile.target_layout,
                "mapping": mapping,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch stages a session Saves/<pak>.cfg with player-1 joystick codes, then rechecks the exact SDL2 routes. Only the single P1 brawler deck on joystick slot 0 is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many OpenBOR saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate OpenBOR emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_codes_follow_source_formulas() {
        assert_eq!(joy_button(0, 0, 16).unwrap(), 601);
        assert_eq!(joy_button(0, 9, 16).unwrap(), 610);
        // Axis halves sit after all buttons: 601 + 10 + 2*2 + 1.
        assert_eq!(joy_axis(0, 2, true, 10, 4).unwrap(), 616);
        // Hat directions sit after buttons and axes.
        assert_eq!(joy_hat(0, 0, 2, 10, 4, 1).unwrap(), 621);
    }

    #[test]
    fn session_config_pins_version_and_player_keys() {
        let baseline = vec![0u8; 300];
        let player1 = [601i32; 12];
        let out = session_config(&baseline, &player1).unwrap();
        assert_eq!(&out[0..4], &COMPATIBLE_VERSION.to_le_bytes());
        assert_eq!(&out[36..40], &601i32.to_le_bytes());
        assert_eq!(
            &out[36 + 13 * 4..36 + 13 * 4 + 4],
            &CONTROL_NONE.to_le_bytes()
        );
        assert_eq!(out.len(), 300);
        assert!(session_config(&[0u8; 100], &player1).is_err());
    }

    #[test]
    fn config_stem_replaces_any_extension() {
        assert_eq!(
            config_stem(std::path::Path::new("/games/foo.pak")).unwrap(),
            "foo.cfg"
        );
        assert_eq!(
            config_stem(std::path::Path::new("/games/foo.PAK")).unwrap(),
            "foo.cfg"
        );
    }
}

#[cfg(target_os = "linux")]
mod session {
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
        linux_classic::AxisEndpoints,
        sdl2::{Device, Snapshot},
        sdl2_physical::PhysicalMap,
    };
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
        path::PathBuf,
        process::Command,
        sync::atomic::AtomicBool,
    };

    fn observe(
        setup: &settings::SavedSetup,
        path: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&setup.sdl_library);
        if let Some(path) = path {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid OpenBOR SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "OpenBOR helper inspected a different SDL2 runtime"
        );
        Ok(snapshot)
    }

    fn routing(mut snapshot: Snapshot) -> Snapshot {
        for device in &mut snapshot.devices {
            device.controls = None;
            device.linux_classic = None;
            device.linux_evdev = None;
            device.sampled_state = None;
            device.mapping = None;
        }
        snapshot
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        config_name: String,
        physical_path: String,
        topology: InputTopology,
        initial: Snapshot,
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
            ensure!(
                fs::symlink_metadata(&setup.content)?.file_type().is_file()
                    && setup.content.canonicalize()? == setup.content,
                "OpenBOR content must be a direct regular .pak file with canonical ancestry"
            );
            ensure!(
                fs::symlink_metadata(&setup.config_source)?
                    .file_type()
                    .is_file(),
                "OpenBOR config source must be a regular file"
            );
            let config_name = config_stem(&setup.content)?;
            ensure!(
                setup
                    .config_source
                    .file_name()
                    .and_then(|name| name.to_str())
                    == Some(config_name.as_str()),
                "OpenBOR config source must be the content's per-pak .cfg"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "OpenBOR physical controller is missing or ambiguous"
            );
            let selected = found[0].device_path.clone();
            let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
            let initial = routing(observe(setup, None, cancel)?);
            let physical_path = topology.resolve_runtime_path(
                &selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // SDL_JoystickOpen(i): player 1 uses slot 0.
            ensure!(
                device.device_index == 0,
                "OpenBOR player 1 needs SDL slot 0; selected pad is index {}",
                device.device_index
            );
            let counts = device
                .controls
                .as_ref()
                .context("OpenBOR SDL control counts are missing")?;
            let num_buttons = counts.buttons;
            let num_axes = counts.axes;
            let num_hats = counts.hats;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("OpenBOR calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing OpenBOR native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("OpenBOR SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("OpenBOR SDL control counts are missing")?,
            )?;
            let mut player1 = DEFAULT_SCANCODES;
            for row in calibration.plan_profile(profile)?.rows {
                let (_, index) = ACTIONS
                    .iter()
                    .find(|(target, _)| *target == row.target_id)
                    .context("OpenBOR target is outside the brawler profile")?;
                let input = row
                    .input
                    .as_ref()
                    .context("OpenBOR brawler control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("OpenBOR requires measured native controls")?;
                let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                });
                let translated = physical.digital_input(native.code, measured)?;
                let released = match translated {
                    DigitalInput::Button(index) => state.buttons.get(&index) == Some(&false),
                    DigitalInput::Hat { index, direction } => state
                        .hats
                        .get(&index)
                        .is_some_and(|mask| mask & direction == 0),
                    DigitalInput::Axis {
                        index, released, ..
                    } => state.axes.get(&index) == Some(&released),
                };
                ensure!(
                    released,
                    "Release the OpenBOR controls before launch preparation"
                );
                // Key codes address this pad's own slot with its measured
                // counts; hats and axes use the source's own layout.
                player1[*index] = match translated {
                    DigitalInput::Button(index) => joy_button(0, index as u32, num_buttons)?,
                    DigitalInput::Hat { index, direction } => {
                        let dir = match direction {
                            0x01 => 0,
                            0x02 => 1,
                            0x04 => 2,
                            0x08 => 3,
                            _ => anyhow::bail!("OpenBOR hat direction is not cardinal"),
                        };
                        joy_hat(0, index as u32, dir, num_buttons, num_axes, num_hats)?
                    }
                    DigitalInput::Axis { index, .. } => {
                        joy_axis(0, index as u32, native.direction > 0, num_buttons, num_axes)?
                    }
                };
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-openbor-")
                .tempdir()?;
            // Saves/ resolves under ./, so running here fully isolates the
            // per-pak config while the .pak itself stays in place.
            let saves_dir = directory.path().join("Saves");
            fs::create_dir(&saves_dir)?;
            let config_path = saves_dir.join(&config_name);
            fs::write(
                &config_path,
                session_config(&fs::read(&setup.config_source)?, &player1)?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.config_source,
                &setup.probe_program,
                &setup.sdl_library,
                &config_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                config_name,
                physical_path,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn directory(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "OpenBOR launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "OpenBOR SDL slot 0 moved before launch"
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
        controller_native_process::{cancelled, native_pid},
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };

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
                file_hash(&self.executable)?.eq_ignore_ascii_case(&self.setup.executable_sha256),
                "OpenBOR executable differs from the saved trusted runtime"
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
                "OpenBOR launch plan changed after preparation"
            );
            self.verify(cancel)?;
            let mut child = crate::emulator::spawn_launch_plan(plan)?;
            if let Err(error) = self.confirm(&mut child, cancel) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            Ok(child)
        }

        fn confirm(&self, child: &mut std::process::Child, cancel: &AtomicBool) -> Result<()> {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                cancelled(cancel)?;
                ensure!(
                    child.try_wait()?.is_none(),
                    "OpenBOR exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "OpenBOR did not open the selected SDL controller before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        fn ready(&self, pid: u32) -> Result<bool> {
            let expected_sdl = self.setup.sdl_library.canonicalize()?;
            let maps = std::fs::read_to_string(format!("/proc/{pid}/maps"))?;
            if !maps.lines().any(|line| {
                let path = line
                    .split_whitespace()
                    .skip(5)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .replace("\\040", " ");
                Path::new(&path) == expected_sdl
            }) {
                return Ok(false);
            }
            Ok(true)
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
            anyhow::bail!("OpenBOR calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("OpenBOR")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "OpenBOR identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "OpenBOR launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "OpenBOR calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "OpenBOR executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // Saves/ resolves under ./, so running in the session directory
        // selects the private per-pak config; the .pak keeps its slot.
        plan.current_directory = inputs.directory().to_path_buf();
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
