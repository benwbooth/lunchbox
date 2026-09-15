//! ADAMEm SDL standalone-native joystick writer.
//!
//! Pinned source: geoffdaddy/adamem_sdl commit
//! `e7553218b2367ce431ad8e1bafc504f9bdf5df96`. The SDL frontend opens
//! joysticks by index (`SDL_JoystickOpen(i)`, sticks 0/1 only), reads axes
//! as directions (axis 0 -> X, any other -> Y) and raw buttons 0..29 through
//! a remappable table (`AdamemSDL.c`). The table lives in `adamem.joy`
//! beside the executable (`ProgramPath`, derived from `argv[0]`), with
//! `raw,cv` lines where `f` is fire and `a` is aim. Content is the cartridge
//! path passed positionally. The session symlinks the trusted executable
//! into its own directory so `ProgramPath` resolves to the private
//! `adamem.joy`; the symlink canonicalizes to the saved executable so its
//! hash still pins the runtime.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "e7553218b2367ce431ad8e1bafc504f9bdf5df96";
pub(crate) const PROFILE_ID: &str = "adamem:standalone-coleco-stick";
pub(crate) const JOY_FILE: &str = "adamem.joy";

/// Layout target ids covered by the native profile: four directions plus
/// fire and aim on stick 0. Keypad/shift buttons stay on the user's own
/// configuration.
pub(crate) const ROUTES: [(&str, &str); 6] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "Fire"),
    ("b", "Aim"),
];

/// Render an `adamem.joy` table: `raw,cv` lines with `f` for the fire
/// button and `a` for the aim button.
pub(crate) fn joy_file(fire: u32, aim: u32) -> Result<String> {
    ensure!(fire < 30, "ADAMEm SDL fire button is out of range");
    ensure!(aim < 30, "ADAMEm SDL aim button is out of range");
    ensure!(fire != aim, "ADAMEm SDL fire and aim need distinct buttons");
    Ok(format!("{fire},f\n{aim},a\n"))
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
                "ADAMEm setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "ADAMEm setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "ADAMEm setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing ADAMEm native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "ADAMEm supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "ADAMEm player needs a saved controller identity"
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
                .context("Missing ADAMEm native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("ADAMEm controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "ADAMEm mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "ADAMEm needs native calibration for every stick control"
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
                "detail": "Native launch stages a private adamem.joy with fire/aim buttons, then rechecks the exact SDL routes. Only stick 0 with axes is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many ADAMEm saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate ADAMEm emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_fire_and_aim_table() {
        assert_eq!(joy_file(0, 1).unwrap(), "0,f\n1,a\n");
        assert!(joy_file(0, 0).is_err());
        assert!(joy_file(30, 1).is_err());
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
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    #[cfg(target_os = "windows")]
    use std::os::windows::fs::symlink_file;
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
            serde_json::from_slice(&output).context("Invalid ADAMEm SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "ADAMEm helper inspected a different SDL2 runtime"
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
        executable: PathBuf,
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
            executable: &std::path::Path,
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            ensure!(
                fs::symlink_metadata(&setup.content)?.file_type().is_file()
                    && setup.content.canonicalize()? == setup.content,
                "ADAMEm content must be a direct regular cartridge with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "ADAMEm physical controller is missing or ambiguous"
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
                    "adamem physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // SDL_JoystickOpen(i) opens SDL slots; only sticks 0/1 are read.
            ensure!(
                device.device_index == 0,
                "ADAMEm stick 0 needs SDL index 0; selected pad is index {}",
                device.device_index
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("ADAMEm calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing ADAMEm native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("ADAMEm SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("ADAMEm SDL control counts are missing")?,
            )?;
            // Axes drive directions (axis 0 -> X, the rest -> Y); buttons
            // land on raw indices through adamem.joy. Hats have no handler
            // and are refused.
            let mut axes: BTreeMap<String, (u32, bool)> = BTreeMap::new();
            let mut buttons: BTreeMap<String, u32> = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "ADAMEm target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("ADAMEm stick control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("ADAMEm requires measured native controls")?;
                let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                });
                let translated = physical.digital_input(native.code, measured)?;
                let released = match translated {
                    DigitalInput::Button(index) => state.buttons.get(&index) == Some(&false),
                    DigitalInput::Hat { .. } => {
                        anyhow::bail!("ADAMEm SDL has no hat handler; map directions to axes")
                    }
                    DigitalInput::Axis {
                        index, released, ..
                    } => state.axes.get(&index) == Some(&released),
                };
                ensure!(
                    released,
                    "Release the ADAMEm controls before launch preparation"
                );
                match translated {
                    DigitalInput::Button(index) => {
                        ensure!(
                            buttons
                                .insert(
                                    row.target_id,
                                    u32::try_from(index)
                                        .context("ADAMEm button index is too large")?
                                )
                                .is_none(),
                            "ADAMEm control appears twice"
                        );
                    }
                    DigitalInput::Axis { index, .. } => {
                        ensure!(
                            axes.insert(
                                row.target_id.clone(),
                                (
                                    u32::try_from(index)
                                        .context("ADAMEm axis index is too large")?,
                                    native.direction > 0
                                )
                            )
                            .is_none(),
                            "ADAMEm stick direction appears twice"
                        );
                    }
                    DigitalInput::Hat { .. } => {
                        anyhow::bail!("ADAMEm SDL has no hat handler; map directions to axes")
                    }
                }
            }
            let axis_of = |negative: &str, positive: &str| {
                let (neg_index, neg_dir) = axes
                    .get(negative)
                    .with_context(|| format!("ADAMEm direction {negative} is not calibrated"))?;
                let (pos_index, pos_dir) = axes
                    .get(positive)
                    .with_context(|| format!("ADAMEm direction {positive} is not calibrated"))?;
                ensure!(
                    !neg_dir && *pos_dir,
                    "ADAMEm axis halves must face opposite polarity"
                );
                // Axis 0 drives X, any other axis drives Y.
                if negative == "left" {
                    ensure!(
                        *neg_index == *pos_index && *neg_index == 0,
                        "ADAMEm X needs axis 0"
                    );
                } else {
                    ensure!(
                        *neg_index == *pos_index && *neg_index != 0,
                        "ADAMEm Y needs a non-zero axis"
                    );
                }
                Ok::<u32, anyhow::Error>(*neg_index)
            };
            let _ = (axis_of("left", "right")?, axis_of("up", "down")?);
            ensure!(
                buttons.len() == 2 && buttons.contains_key("a") && buttons.contains_key("b"),
                "ADAMEm stick needs fire (a) and aim (b) buttons"
            );
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-adamem-")
                .tempdir()?;
            fs::write(
                directory.path().join(JOY_FILE),
                joy_file(buttons["a"], buttons["b"])?,
            )?;
            // ProgramPath derives from argv[0], so the trusted executable
            // is linked here; its canonical target still hashes to the
            // saved runtime while adamem.joy resolves beside the link.
            // Unix symlinks; Windows tries a file symlink and falls back
            // to a copy (same hashed bytes) without developer-mode rights.
            let executable_link = directory.path().join("adamem");
            #[cfg(unix)]
            symlink(executable, &executable_link)?;
            #[cfg(target_os = "windows")]
            if symlink_file(executable, &executable_link).is_err() {
                fs::copy(executable, &executable_link)?;
            }
            ensure!(
                executable_link.canonicalize()? == executable.canonicalize()?,
                "ADAMEm executable link escapes the trusted runtime"
            );
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &directory.path().join(JOY_FILE),
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            hashes.insert(executable.canonicalize()?, file_hash(executable)?);
            let prepared = Self {
                directory,
                executable: executable_link,
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

        pub(crate) fn executable(&self) -> &std::path::Path {
            &self.executable
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "ADAMEm launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "ADAMEm SDL index 0 moved before launch"
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
                "ADAMEm executable differs from the saved trusted runtime"
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
                "ADAMEm launch plan changed after preparation"
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
                    "ADAMEm exited before controller handoff"
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
                    "ADAMEm did not open the selected SDL controller before timeout"
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
            anyhow::bail!("ADAMEm calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("ADAMEm SDL")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "ADAMEm identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "ADAMEm launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "ADAMEm calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "ADAMEm executable differs from the saved trusted runtime"
        );
        let inputs =
            session::PreparedSession::prepare(setup, calibrations, inventory, &executable, cancel)?;
        let mut plan = original.clone();
        // argv[0] selects ProgramPath; the session symlink keeps the
        // canonical executable while adamem.joy resolves beside the link.
        plan.program = inputs.executable().to_path_buf();
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
