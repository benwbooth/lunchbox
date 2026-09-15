//! PokeMini standalone-native pokemini.cfg joystick writer.
//!
//! Pinned source: SourceForge pokemini/code
//! `15cc97ff70d6d9d749287ac55cb68198708564f3`. `source/CommandLine.c` reads
//! `joyenabled`, `joyid` (0-15), `joyaxis_dpad`, `joyhats_dpad`, and
//! `joybutton_menu/a/b/c/up/down/left/right/power/shock` (-1 through 32)
//! from `pokemini.cfg` beside the executable; `source/Joystick.c` maps SDL
//! buttons directly, axes 0/1 and hats as dpad when enabled. The config is
//! resolved by chdir-ing to the executable directory, so launch uses a
//! symlink sandbox whose argv[0] selects the private directory.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "15cc97ff70d6d9d749287ac55cb68198708564f3";
pub(crate) const PROFILE_ID: &str = "pokemini:standalone-pokemini";
pub(crate) const ARTIFACT_SHA256: &str =
    "19de65332abe6d203c7065a70764202f28c974fa851b67cf0fe689a5d5c1f17e";

/// Config keys in source order with the calibrated layout target each feeds.
/// `power`/`shake`/`menu` have no layout counterparts and stay at source
/// defaults; they are not invented from gameplay controls.
pub(crate) const JOYBUTTON_KEYS: [(&str, Option<&str>); 10] = [
    ("joybutton_menu", None),
    ("joybutton_a", Some("a")),
    ("joybutton_b", Some("b")),
    ("joybutton_c", Some("c")),
    ("joybutton_up", Some("up")),
    ("joybutton_down", Some("down")),
    ("joybutton_left", Some("left")),
    ("joybutton_right", Some("right")),
    ("joybutton_power", None),
    ("joybutton_shock", None),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 9] = [
    ("b", "B"),
    ("a", "A"),
    ("c", "C"),
    ("shake", "Shake"),
    ("select", "Select"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
];

/// Render the joystick block of `pokemini.cfg`. Values are raw SDL button
/// indices (-1 unassigns); directions additionally work through axes 0/1 and
/// hats without any button assignment.
pub(crate) fn joystick_config(joyid: u8, buttons: &BTreeMap<String, i16>) -> Result<String> {
    ensure!(joyid <= 15, "PokeMini joystick ID must be 0 through 15");
    let mut out = String::from("joyenabled = 1\n");
    out.push_str(&format!("joyid = {joyid}\n"));
    out.push_str("joyaxis_dpad = 1\njoyhats_dpad = 1\n");
    for (key, target) in JOYBUTTON_KEYS {
        let value = match target {
            Some(target) => *buttons
                .get(target)
                .with_context(|| format!("PokeMini control {target} is not calibrated"))?,
            // Menu/power/shake stay disabled: no layout control feeds them
            // and inventing gameplay bindings would mis-map the emulator UI.
            None => -1,
        };
        ensure!(
            (-1..=32).contains(&value),
            "PokeMini joystick button is out of range"
        );
        out.push_str(&format!("{key} = {value}\n"));
    }
    Ok(out)
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use anyhow::Context;
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
                "PokeMini setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "PokeMini setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "PokeMini setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing PokeMini native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "PokeMini supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "PokeMini player needs a saved controller identity"
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
                .context("Missing PokeMini native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("PokeMini controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "PokeMini mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "PokeMini needs native calibration for every Mini control"
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
                "detail": "Native Linux launch runs a symlink sandbox with a private pokemini.cfg, then rechecks the exact SDL routes. Only raw-button mappings on SDL index 0 are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many PokeMini saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate PokeMini emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buttons() -> BTreeMap<String, i16> {
        [
            ("a", 1),
            ("b", 2),
            ("c", 7),
            ("up", 10),
            ("down", 11),
            ("left", 4),
            ("right", 5),
            ("shake", 6),
            ("select", 3),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect()
    }

    #[test]
    fn renders_joystick_block_with_unassigned_menu() {
        let text = joystick_config(0, &buttons()).unwrap();
        assert!(text.contains("joyenabled = 1\n"));
        assert!(text.contains("joyid = 0\n"));
        assert!(text.contains("joybutton_a = 1\n"));
        assert!(text.contains("joybutton_menu = -1\n"));
        assert!(text.contains("joybutton_power = -1\n"));
    }

    #[test]
    fn rejects_bad_ids_and_missing_controls() {
        assert!(joystick_config(16, &buttons()).is_err());
        let mut map = buttons();
        map.remove("a");
        assert!(joystick_config(0, &map).is_err());
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
        path::{Path, PathBuf},
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
            serde_json::from_slice(&output).context("Invalid PokeMini SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "PokeMini helper inspected a different SDL2 runtime"
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

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        physical_path: String,
        device_index: u32,
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
                "PokeMini content must be a direct regular file with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "PokeMini physical controller is missing or ambiguous"
            );
            let selected = found[0].device_path.clone();
            let initial = routing(observe(setup, None, cancel)?);
            // Linux pins kernel input identity through the sysfs topology.
            // Other hosts pin the SDL device-interface path plus index and
            // re-probe it; names and GUIDs are never identity.
            #[cfg(target_os = "linux")]
            let (physical_path, topology) = {
                let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
                let physical_path = topology.resolve_runtime_path(
                    &selected,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                topology.verify()?;
                (physical_path, topology)
            };
            #[cfg(not(target_os = "linux"))]
            let physical_path = {
                let selected_string = selected.to_string_lossy().into_owned();
                let candidates = initial
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(selected_string.as_str()))
                    .collect::<Vec<_>>();
                ensure!(
                    candidates.len() == 1,
                    "pokemini physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // joyid is a fixed SDL index; the pad must hold slot 0.
            ensure!(
                device.device_index == 0,
                "PokeMini opens SDL joystick 0; selected pad is index {}",
                device.device_index
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("PokeMini calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing PokeMini native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("PokeMini SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("PokeMini SDL control counts are missing")?,
            )?;
            // Only raw buttons feed joybutton slots; axes and hats drive
            // directions automatically through joyaxis_dpad/joyhats_dpad.
            let mut buttons = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "PokeMini target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("PokeMini Mini control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("PokeMini requires measured native controls")?;
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
                    "Release the PokeMini controls before launch preparation"
                );
                if matches!(row.target_id.as_str(), "a" | "b" | "c" | "shake" | "select") {
                    let DigitalInput::Button(index) = translated else {
                        anyhow::bail!(
                            "PokeMini action buttons need raw buttons; use axes or hats only for directions"
                        )
                    };
                    let index =
                        u16::try_from(index).context("PokeMini button index is too large")?;
                    ensure!(index <= 32, "PokeMini button is out of range");
                    ensure!(
                        buttons.insert(row.target_id, index as i16).is_none(),
                        "PokeMini control appears twice"
                    );
                }
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-pokemini-")
                .tempdir()?;
            fs::write(
                directory.path().join("pokemini.cfg"),
                joystick_config(0, &buttons)?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &directory.path().join("pokemini.cfg"),
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                physical_path,
                device_index: device.device_index,
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
                ensure!(file_hash(path)? == *hash, "PokeMini launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "PokeMini SDL index 0 moved before launch"
            );
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(
                &captured.devices,
                &self.physical_path,
                self.device_index,
            )?;
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
            // No sysfs exists here; health is a fresh same-routing probe
            // that still sees the pinned path at the pinned index.
            let fresh = routing(observe(&self.setup, None, &AtomicBool::new(false))?);
            self.initial.ensure_same_routing(&fresh)?;
            platform::require_unique_device_path(
                &fresh.devices,
                &self.physical_path,
                self.device_index,
            )?;
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
        os::unix::fs::symlink,
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
                "PokeMini executable differs from the saved trusted runtime"
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
                "PokeMini launch plan changed after preparation"
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
                    "PokeMini exited before controller handoff"
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
                    "PokeMini did not open the selected SDL controller before timeout"
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
            anyhow::bail!("PokeMini calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("PokeMini")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "PokeMini identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "PokeMini launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "PokeMini calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "PokeMini executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        // The config resolves beside argv[0]: link the trusted executable
        // into the session directory so its directory selects the private
        // pokemini.cfg. The link target is the hashed executable itself.
        let link = inputs.directory().join("pokemini");
        ensure!(
            !link.symlink_metadata().is_ok(),
            "PokeMini sandbox link already exists"
        );
        symlink(&executable, &link)?;
        ensure!(
            link.canonicalize()? == executable,
            "PokeMini sandbox link points elsewhere"
        );
        let mut plan = original.clone();
        plan.program = link;
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
