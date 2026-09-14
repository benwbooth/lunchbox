//! Tsugaru standalone-native game-port profile writer.
//!
//! Pinned source: captainys/TOWNSEMU commit
//! `e27adde120fe06150f3d57a6dc7cba66b5f43a32`. Game ports select physical
//! joysticks by index: `-GAMEPORT0 PHYS0` reads `/dev/input/js0` through the
//! Linux joydev backend (`ysgamepad_linux.c`), with buttons 0/1 as run A/B,
//! hats driving directions, and the last two axes driving analog directions.
//! The launch profile is a `Tsugaru_CUI`-shaped text file (`townsprofile.cpp`)
//! selecting the ROM directory, CMOS image, CD image, and both game ports.
//! Content is the CD image path passed as `-CD`.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "e27adde120fe06150f3d57a6dc7cba66b5f43a32";
pub(crate) const PROFILE_ID: &str = "tsugaru:standalone-towns-pad";

/// Layout target ids covered by the native profile: the two run buttons and
/// four directions. FM Towns pads expose no start/select to the guest port.
pub(crate) const ROUTES: [(&str, &str); 6] = [
    ("a", "Run A"),
    ("b", "Run B"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
];

/// Render the launch profile: ROM directory, CMOS, CD image, and both game
/// ports on the physical joystick. Port 1 stays on keyboard so a second
/// unmeasured pad can never hijack player one.
pub(crate) fn launch_profile(rom_dir: &str, cmos: &str, cd: &str, phys: u32) -> Result<String> {
    ensure!(phys <= 7, "Tsugaru physical joystick is out of range");
    for (label, value) in [
        ("ROM directory", rom_dir),
        ("CMOS image", cmos),
        ("CD image", cd),
    ] {
        ensure!(
            !value.is_empty() && value.len() <= 1024 && !value.contains(['\0', '\n', '\r']),
            "Tsugaru {label} is invalid"
        );
    }
    Ok(format!(
        "ROMDIR {rom_dir}\nCMOS {cmos}\nCD {cd}\nGAMEPORT 0 PHYS{phys}\nGAMEPORT 1 KEY\n"
    ))
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
        /// FM Towns ROM directory (machine ROM assets).
        pub rom_dir: PathBuf,
        /// CMOS settings image.
        pub cmos: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Tsugaru setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Tsugaru setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.rom_dir.is_absolute()
                    && !self
                        .rom_dir
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                "Tsugaru ROM directory must be absolute without parent traversal"
            );
            ensure!(
                self.cmos.is_absolute()
                    && !self
                        .cmos
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                "Tsugaru CMOS image must be absolute without parent traversal"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Tsugaru setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Tsugaru native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Tsugaru supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Tsugaru player needs a saved controller identity"
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
                .context("Missing Tsugaru native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Tsugaru controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Tsugaru mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Tsugaru needs native calibration for every pad control"
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
                "detail": "Native Linux launch passes explicit ROM/CMOS/CD/game-port flags with PHYS0 for port 0, then rechecks the exact joydev routes. Only raw buttons and hats on /dev/input/js0 are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Tsugaru saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Tsugaru emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_gameport_profile() {
        let text = launch_profile("/roms/towns", "/saves/CMOS.BIN", "/games/game.cue", 0).unwrap();
        assert!(text.contains("ROMDIR /roms/towns\n"));
        assert!(text.contains("GAMEPORT 0 PHYS0\n"));
        assert!(text.contains("GAMEPORT 1 KEY\n"));
        assert!(launch_profile("/r", "/c", "/g", 8).is_err());
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
            serde_json::from_slice(&output).context("Invalid Tsugaru SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Tsugaru helper inspected a different SDL2 runtime"
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
                "Tsugaru content must be a direct regular CD image with canonical ancestry"
            );
            ensure!(
                setup.rom_dir.is_dir() && setup.rom_dir.canonicalize()? == setup.rom_dir,
                "Tsugaru ROM directory must be a canonical directory"
            );
            ensure!(
                fs::symlink_metadata(&setup.cmos)?.file_type().is_file(),
                "Tsugaru CMOS image must be a regular file"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Tsugaru physical controller is missing or ambiguous"
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
            // The joydev backend opens /dev/input/js0 for PHYS0; the pad
            // must be SDL index 0 so both numberings agree.
            ensure!(
                device.device_index == 0,
                "Tsugaru PHYS0 needs SDL index 0; selected pad is index {}",
                device.device_index
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Tsugaru calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Tsugaru native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("Tsugaru SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("Tsugaru SDL control counts are missing")?,
            )?;
            // PHYS0 reads raw buttons 0/1 and hats; analog axes drive
            // directions through the last-two-axes rule, so axis halves
            // are refused with a redirect to hats or buttons.
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "Tsugaru target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("Tsugaru pad control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("Tsugaru requires measured native controls")?;
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
                    DigitalInput::Axis { .. } => {
                        anyhow::bail!(
                            "Tsugaru PHYS0 directions need hats or buttons; analog axes follow the last-two-axes rule"
                        )
                    }
                };
                ensure!(
                    released,
                    "Release the Tsugaru controls before launch preparation"
                );
                if matches!(row.target_id.as_str(), "a" | "b") {
                    ensure!(
                        matches!(translated, DigitalInput::Button(_)),
                        "Tsugaru run buttons need raw buttons"
                    );
                }
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-tsugaru-")
                .tempdir()?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.cmos,
                &setup.rom_dir,
                &setup.probe_program,
                &setup.sdl_library,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                physical_path,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Tsugaru launch input changed");
            }
            self.setup
                .rom_dir
                .canonicalize()?
                .eq(&self.setup.rom_dir)
                .then_some(())
                .context("Tsugaru ROM directory moved")?;
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "Tsugaru SDL index 0 moved before launch"
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
                "Tsugaru executable differs from the saved trusted runtime"
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
                "Tsugaru launch plan changed after preparation"
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
                    "Tsugaru exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Tsugaru did not open the selected joystick before timeout"
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
            anyhow::bail!("Tsugaru calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Tsugaru")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Tsugaru identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Tsugaru launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Tsugaru calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Tsugaru executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // The ROM directory is positional; the profile values ride as
        // explicit flags so no other file is touched. The CMOS image is
        // passed through so BIOS settings persist in place.
        plan.arguments = vec![
            setup.rom_dir.as_os_str().to_owned(),
            std::ffi::OsString::from("-CMOS"),
            setup.cmos.as_os_str().to_owned(),
            std::ffi::OsString::from("-CD"),
            setup.content.as_os_str().to_owned(),
            std::ffi::OsString::from("-GAMEPORT0"),
            std::ffi::OsString::from("PHYS0"),
            std::ffi::OsString::from("-GAMEPORT1"),
            std::ffi::OsString::from("KEY"),
        ];
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
