//! Uzem standalone-native joystick-settings writer.
//!
//! Pinned source: Uzebox/uzebox commit
//! `abf5125847e68a6b7c4432f7849cd5baf717bba5`, `tools/uzem/avr8.h` and
//! `avr8.cpp`. The `joystick-settings` file beside the working directory
//! holds per stick (two max, opened as SDL indices 0/1) eight
//! `{u8 button, u8 bit}` pairs in remap order
//! [START, SELECT, A, B, X, Y, LSh, RSh] with SNES bit values, followed by
//! eight `{i32 axis, u8 bits}` records where axes 0/1 carry the direction
//! pair (even index = left/right, odd = up/down) and the rest are
//! `JOY_AXIS_UNUSED` (-1). Hats need no mapping. `init_joysticks` opens
//! `SDL_JoystickOpen(i)`, so callers must prove the pads hold SDL slots 0
//! and 1 for the exact child immediately before launch.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "abf5125847e68a6b7c4432f7849cd5baf717bba5";
pub(crate) const PROFILE_ID: &str = "uzem:standalone-snes";
pub(crate) const SETTINGS_FILE: &str = "joystick-settings";

/// Remap order with SNES bit values from `avr8.h`.
pub(crate) const BUTTON_SLOTS: [(u8, &str); 8] = [
    (3, "start"),
    (2, "select"),
    (8, "a"),
    (0, "b"),
    (9, "x"),
    (1, "y"),
    (10, "l"),
    (11, "r"),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 12] = [
    ("b", "B"),
    ("a", "A"),
    ("y", "Y"),
    ("x", "X"),
    ("l", "L"),
    ("r", "R"),
    ("select", "Select"),
    ("start", "Start"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
];

/// One stick's mapping: eight raw SDL button indices in remap order plus
/// the shared direction-axis pair (even = left/right, odd = up/down).
#[derive(Clone, Copy)]
pub(crate) struct StickMapping {
    pub buttons: [u8; 8],
    pub axis_x: u8,
    pub axis_y: u8,
}

/// Render the exact binary `joystick-settings` file: per stick, eight
/// `{button, bit}` byte pairs then eight little-endian `{i32 axis, u8
/// bits}` records with three padding bytes each (matching the source
/// struct layout on little-endian hosts). Unused axes are -1.
pub(crate) fn joystick_settings(sticks: &[StickMapping]) -> Result<Vec<u8>> {
    ensure!(
        !sticks.is_empty() && sticks.len() <= 2,
        "Uzem supports one or two joystick sticks"
    );
    let mut out = Vec::with_capacity(160);
    for stick in sticks {
        ensure!(
            stick.buttons.len() == BUTTON_SLOTS.len(),
            "Uzem stick needs every remap-order button"
        );
        for (index, button) in stick.buttons.iter().enumerate() {
            out.push(*button);
            out.push(BUTTON_SLOTS[index].0);
        }
        for (position, axis) in [stick.axis_x, stick.axis_y].into_iter().enumerate() {
            out.extend_from_slice(&(axis as i32).to_le_bytes());
            out.push(0);
            out.extend_from_slice(&[0, 0, 0]);
            let _ = position;
        }
        for _ in 2..8 {
            out.extend_from_slice(&(-1i32).to_le_bytes());
            out.push(0);
            out.extend_from_slice(&[0, 0, 0]);
        }
    }
    ensure!(out.len() == sticks.len() * 80, "Uzem record size differs");
    Ok(out)
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
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Uzem setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Uzem setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Uzem setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Uzem native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "Uzem setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Uzem players must be distinct, contiguous, and start at player one"
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
                .context("Missing Uzem native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Uzem controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                    "Uzem mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "Uzem needs native calibration for every SNES control"
                );
                players.push(serde_json::json!({
                    "player": player.player,
                    "controller_id": player.controller_id,
                    "source_layout": calibration.layout,
                    "target_layout": profile.target_layout,
                    "mapping": mapping,
                }));
            }
            Ok(serde_json::json!({
                "profile_id": PROFILE_ID,
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch writes a private joystick-settings binary, then rechecks the exact SDL routes. Only SNES pads on SDL slots 0/1 are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Uzem saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Uzem emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_layout_matches_source_structs() {
        let bytes = joystick_settings(&[StickMapping {
            buttons: [0, 1, 2, 3, 4, 5, 6, 7],
            axis_x: 0,
            axis_y: 1,
        }])
        .unwrap();
        assert_eq!(bytes.len(), 80);
        assert_eq!(&bytes[0..4], &[0, 3, 1, 2]);
        assert_eq!(&bytes[14..16], &[7, 11]);
        assert_eq!(&bytes[16..20], &[0, 0, 0, 0]);
        assert_eq!(&bytes[24..32], &[1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bytes[32..40], &[255, 255, 255, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn rejects_bad_stick_counts() {
        let stick = StickMapping {
            buttons: [0; 8],
            axis_x: 0,
            axis_y: 1,
        };
        assert!(joystick_settings(&[]).is_err());
        assert!(joystick_settings(&[stick, stick, stick]).is_err());
    }
}

mod session {
    use super::*;
    #[cfg(target_os = "linux")]
    use crate::controller_bizhawk_guard::InputTopology;
    #[cfg(not(target_os = "linux"))]
    use crate::controller_native_platform as platform;
    use crate::{
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
            serde_json::from_slice(&output).context("Invalid Uzem SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Uzem helper inspected a different SDL2 runtime"
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

    fn stick_mapping(calibration: &Calibration, device: &Device) -> Result<StickMapping> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing Uzem native profile")?;
        let physical = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("Uzem SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("Uzem SDL control counts are missing")?,
        )?;
        let mut buttons = BTreeMap::new();
        let mut axes: BTreeMap<u32, (bool, bool)> = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            ensure!(
                ROUTES.iter().any(|(target, _)| *target == row.target_id),
                "Uzem target {} outside contract",
                row.target_id
            );
            let input = row
                .input
                .as_ref()
                .context("Uzem SNES control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Uzem requires measured native controls")?;
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
                "Release the Uzem controls before launch preparation"
            );
            // Axes must pair into direction halves; hats ride free and need
            // no mapping. Buttons land on their own raw indices.
            match translated {
                DigitalInput::Button(index) => {
                    let index = u8::try_from(index).context("Uzem button index is too large")?;
                    ensure!(
                        buttons.insert(row.target_id, index).is_none(),
                        "Uzem control appears twice"
                    );
                }
                DigitalInput::Axis { index, .. } => {
                    let index = u32::try_from(index).context("Uzem axis index is too large")?;
                    let entry = axes.entry(index).or_insert((false, false));
                    if native.direction > 0 {
                        ensure!(!entry.1, "Uzem axis direction appears twice");
                        entry.1 = true;
                    } else {
                        ensure!(!entry.0, "Uzem axis direction appears twice");
                        entry.0 = true;
                    }
                }
                DigitalInput::Hat { .. } => {}
            }
        }
        // Direction halves must resolve to exactly one shared X and one
        // shared Y axis; each pair faces opposite polarity by construction
        // of the calibration endpoints.
        let mut axis_x = None;
        let mut axis_y = None;
        for (index, (neg, pos)) in &axes {
            ensure!(*neg && *pos, "Uzem axis {index} needs both halves");
            if axis_x.is_none() {
                axis_x = Some(*index);
            } else if axis_y.is_none() {
                ensure!(*index != axis_x.unwrap(), "Uzem axes must differ");
                axis_y = Some(*index);
            } else {
                anyhow::bail!("Uzem supports one direction axis pair per stick");
            }
        }
        let (axis_x, axis_y) = match (axis_x, axis_y) {
            (Some(x), Some(y)) => (x, y),
            _ => anyhow::bail!("Uzem directions need two shared analog axes"),
        };
        ensure!(
            u8::try_from(axis_x).is_ok() && u8::try_from(axis_y).is_ok(),
            "Uzem axis index is too large"
        );
        // Remap-order buttons: START, SELECT, A, B, X, Y, L, R.
        // Directions ride the shared axes; only action buttons consume raw
        // indices here.
        let mut action = |target: &str| {
            buttons
                .remove(target)
                .with_context(|| format!("Uzem button {target} is not calibrated"))
        };
        let ordered = [
            action("start")?,
            action("select")?,
            action("a")?,
            action("b")?,
            action("x")?,
            action("y")?,
            action("l")?,
            action("r")?,
        ];
        Ok(StickMapping {
            buttons: ordered,
            axis_x: axis_x as u8,
            axis_y: axis_y as u8,
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        physical_paths: Vec<String>,
        device_indices: Vec<u32>,
        #[cfg(target_os = "linux")]
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
                "Uzem content must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "Uzem physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            #[cfg(target_os = "linux")]
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            let mut physical_paths = Vec::new();
            let mut sticks = Vec::new();
            let mut device_indices = Vec::new();
            for (player, selected_path) in setup.players.iter().zip(&selected) {
                // Linux resolves through the sysfs topology; other hosts
                // match the SDL device-interface path and require uniqueness.
                #[cfg(target_os = "linux")]
                let path = topology.resolve_runtime_path(
                    selected_path,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                #[cfg(not(target_os = "linux"))]
                let path = {
                    let selected_string = selected_path.to_string_lossy().into_owned();
                    let candidates = initial
                        .devices
                        .iter()
                        .filter(|device| device.path.as_deref() == Some(selected_string.as_str()))
                        .collect::<Vec<_>>();
                    ensure!(
                        candidates.len() == 1,
                        "Uzem physical controller is missing or ambiguous in SDL"
                    );
                    selected_string
                };
                ensure!(
                    !physical_paths.contains(&path),
                    "Uzem players share a controller"
                );
                let captured = observe(setup, Some(&path), cancel)?;
                initial.ensure_same_routing(&routing(captured.clone()))?;
                #[cfg(target_os = "linux")]
                topology.verify()?;
                let device = captured.device_at_path(&path)?;
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_device_path(
                    &captured.devices,
                    &path,
                    device.device_index,
                )?;
                // SDL_JoystickOpen(i) for i in 0..1: the pads must hold
                // slots 0 and 1 in order.
                ensure!(
                    device.device_index == u32::from(player.player) - 1,
                    "Uzem player {} needs SDL slot {}, found {}",
                    player.player,
                    player.player - 1,
                    device.device_index
                );
                sticks.push(stick_mapping(
                    calibrations
                        .get(&player.controller_id)
                        .context("Uzem calibration disappeared")?,
                    device,
                )?);
                device_indices.push(device.device_index);
                physical_paths.push(path);
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-uzem-")
                .tempdir()?;
            // joystick-settings resolves in the working directory; the
            // launch layer runs there so only the private file is visible.
            let settings_path = directory.path().join(SETTINGS_FILE);
            fs::write(&settings_path, joystick_settings(&sticks)?)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &settings_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                physical_paths,
                device_indices,
                #[cfg(target_os = "linux")]
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
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Uzem launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for ((index, path), expected) in self
                .physical_paths
                .iter()
                .enumerate()
                .zip(&self.device_indices)
            {
                let captured = observe(&self.setup, Some(path), cancel)?;
                self.initial
                    .ensure_same_routing(&routing(captured.clone()))?;
                let device = captured.device_at_path(path)?;
                ensure!(
                    device.device_index == index as u32 && device.device_index == *expected,
                    "Uzem SDL slot order moved"
                );
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_device_path(&captured.devices, path, *expected)?;
                #[cfg(target_os = "linux")]
                self.topology.verify()?;
            }
            #[cfg(target_os = "linux")]
            {
                return self.topology.verify();
            }
            #[cfg(not(target_os = "linux"))]
            {
                return Ok(());
            }
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            #[cfg(target_os = "linux")]
            return self.topology.verify();
            #[cfg(not(target_os = "linux"))]
            return self.verify_health_probe();
        }

        #[cfg(not(target_os = "linux"))]
        fn verify_health_probe(&self) -> Result<()> {
            let fresh = routing(observe(&self.setup, None, &AtomicBool::new(false))?);
            self.initial.ensure_same_routing(&fresh)?;
            for (path, index) in self.physical_paths.iter().zip(&self.device_indices) {
                platform::require_unique_device_path(&fresh.devices, path, *index)?;
            }
            Ok(())
        }
    }
}

pub(crate) mod native_command {
    use super::*;
    use crate::controller_native_process::cancelled;
    #[cfg(target_os = "linux")]
    use crate::controller_native_process::native_pid;
    use crate::{
        controller_catalog::Calibration,
        controller_native_platform as platform,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{
        collections::HashMap,
        path::PathBuf,
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
                "Uzem executable differs from the saved trusted runtime"
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
                "Uzem launch plan changed after preparation"
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
                    "Uzem exited before controller handoff"
                );
                // Linux walks the launch tree (bubblewrap monitors); other
                // hosts check the direct child, which they spawn directly.
                #[cfg(target_os = "linux")]
                let owned = native_pid(child.id(), &self.executable)?
                    .is_some_and(|pid| self.ready(pid).unwrap_or(false));
                #[cfg(not(target_os = "linux"))]
                let owned = platform::child_exe_matches(child.id(), &self.executable)?
                    && self.ready(child.id())?;
                if owned {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Uzem did not open the selected SDL controllers before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        fn ready(&self, pid: u32) -> Result<bool> {
            // Linux proves the child mapped the exact SDL library. Other
            // hosts pin the executable plus a fresh device re-probe; the
            // weaker guarantee is explicit here and in the launch text.
            if cfg!(target_os = "linux") {
                return platform::child_maps_library(pid, &self.setup.sdl_library);
            }
            if !platform::child_exe_matches(pid, &self.executable)? {
                return Ok(false);
            }
            self.inputs.check_health()?;
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
            anyhow::bail!("Uzem calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Uzem")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Uzem identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Uzem launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Uzem calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Uzem executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // joystick-settings resolves in the working directory; running there
        // selects the private file. The ROM keeps its default positional slot.
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
