//! ep128emu standalone-native joystick writer.
//!
//! Pinned source: istvan-v/ep128emu commit
//! `9054a7bcb7defb27e23390196c07152f387b3a37`. The SDL backend opens
//! `/dev/input/jsN` slots (`SDL_JoystickOpen(i)`), preferring capable pads
//! (4+ axes, 4+ buttons, 1+ hats) for the first of two slots, and emits
//! `keyCodeBase (0xC000)` joystick events: axes 1..8 as halves, buttons
//! 1..16, and two POV hats. The GUI maps those events through
//! `keyboard.XX.x` config keys to Enterprise matrix rows (`emucfg.cpp`),
//! with `convertKeyCode` masking to 7 bits. The ASCII config file (`-cfg`,
//! `key<TAB>value` lines in `cfg_db.cpp`) persists under
//! `$HOME/.ep128emu`, so launch overrides HOME with a session directory
//! holding a private config. Content is the snapshot path passed as
//! `-snapshot`.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "9054a7bcb7defb27e23390196c07152f387b3a37";
pub(crate) const PROFILE_ID: &str = "ep128emu:standalone-enterprise-joystick";
pub(crate) const KEYCODE_BASE: u32 = 0xC000;

/// Layout target ids covered by the native profile: the Enterprise joystick
/// directions, fire, and two extra buttons. The guest matrix rows come from
/// the user's own keyboard map; the session only stages joystick events.
pub(crate) const ROUTES: [(&str, &str); 7] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("fire", "Fire"),
    ("fire2", "Fire 2"),
    ("fire3", "Fire 3"),
];

/// Joystick event codes from `joystick.hpp`: axis halves, buttons, hats.
pub(crate) fn axis_code(axis: u32, positive: bool) -> Result<u32> {
    ensure!((1..=8).contains(&axis), "ep128emu axis is out of range");
    Ok(KEYCODE_BASE + (axis - 1) * 2 + u32::from(positive))
}

pub(crate) fn button_code(button: u32) -> Result<u32> {
    ensure!(
        (1..=16).contains(&button),
        "ep128emu button is out of range"
    );
    Ok(KEYCODE_BASE + 15 + button)
}

pub(crate) fn hat_code(hat: u32, direction: u8) -> Result<u32> {
    ensure!(hat <= 1, "ep128emu hat is out of range");
    let dir = match direction {
        0x02 => 0,
        0x01 => 1,
        0x08 => 2,
        0x04 => 3,
        _ => anyhow::bail!("ep128emu hat direction is not cardinal"),
    };
    Ok(KEYCODE_BASE + 32 + hat * 4 + dir)
}

/// Render the ASCII config patch: one `keyboard.<hi><lo>.0` TAB `value`
/// line per joystick event. The GUI merges file keys over the running
/// config, so unrelated settings survive.
pub(crate) fn keyboard_lines(events: &BTreeMap<u32, u32>) -> Result<String> {
    ensure!(
        !events.is_empty(),
        "ep128emu needs at least one joystick event"
    );
    let mut out = String::new();
    for (event, row) in events {
        ensure!(
            (KEYCODE_BASE..KEYCODE_BASE + 40).contains(event),
            "ep128emu event is outside the joystick range"
        );
        ensure!(*row <= 127, "ep128emu matrix row is out of range");
        out.push_str(&format!("keyboard.{event:02X}.0\t{row}\n"));
    }
    Ok(out)
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use std::{
        collections::{BTreeMap, HashMap},
        path::PathBuf,
    };

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
        /// Matrix rows per joystick event code, from the user's own
        /// keyboard map for the Enterprise title at hand.
        pub event_rows: BTreeMap<u32, u32>,
        /// The user's real ep128emu config; only `keyboard.*` joystick
        /// lines are patched, so ROM paths, media, and saves survive.
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
                "ep128emu setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "ep128emu setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "ep128emu setup needs a trusted executable SHA-256"
            );
            keyboard_lines(&self.event_rows).context("ep128emu event rows are invalid")?;
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing ep128emu native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "ep128emu supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "ep128emu player needs a saved controller identity"
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
                .context("Missing ep128emu native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("ep128emu controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "ep128emu mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "ep128emu needs native calibration for every joystick control"
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
                "detail": "Native launch stages a session .ep128emu config with joystick event rows, then rechecks the exact SDL routes. Only a capable pad in the first slot is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many ep128emu saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate ep128emu emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_codes_follow_joystick_header() {
        assert_eq!(axis_code(1, false).unwrap(), 0xC000);
        assert_eq!(axis_code(1, true).unwrap(), 0xC001);
        assert_eq!(axis_code(8, true).unwrap(), 0xC00F);
        assert_eq!(button_code(1).unwrap(), 0xC010);
        assert_eq!(button_code(16).unwrap(), 0xC01F);
        assert_eq!(hat_code(0, 0x01).unwrap(), 0xC021);
        assert_eq!(hat_code(1, 0x04).unwrap(), 0xC027);
        assert!(axis_code(9, false).is_err());
        assert!(button_code(17).is_err());
    }

    #[test]
    fn keyboard_lines_use_tab_separated_hex_keys() {
        let events = BTreeMap::from([(0xC000, 5), (0xC010, 6)]);
        let text = keyboard_lines(&events).unwrap();
        assert!(text.contains("keyboard.C000.0\t5\n"));
        assert!(text.contains("keyboard.C010.0\t6\n"));
        assert!(keyboard_lines(&BTreeMap::new()).is_err());
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
            serde_json::from_slice(&output).context("Invalid ep128emu SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "ep128emu helper inspected a different SDL2 runtime"
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
        home: PathBuf,
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
                "ep128emu content must be a direct regular snapshot with canonical ancestry"
            );
            ensure!(
                fs::symlink_metadata(&setup.config_source)?
                    .file_type()
                    .is_file(),
                "ep128emu config source must be a regular file"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "ep128emu physical controller is missing or ambiguous"
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
                    "ep128emu physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // The backend prefers capable pads (4+ axes, 4+ buttons, 1+
            // hat) for its first slot; the session proves the pad holds
            // SDL index 0 and meets that bar.
            ensure!(
                device.device_index == 0,
                "ep128emu first slot needs SDL index 0; selected pad is index {}",
                device.device_index
            );
            let counts = device
                .controls
                .as_ref()
                .context("ep128emu SDL control counts are missing")?;
            ensure!(
                counts.axes >= 4 && counts.buttons >= 4 && counts.hats >= 1,
                "ep128emu first slot needs 4+ axes, 4+ buttons, and 1+ hats"
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("ep128emu calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing ep128emu native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("ep128emu SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("ep128emu SDL control counts are missing")?,
            )?;
            // Every calibrated control must produce a joystick event, and
            // every event needs a staged matrix row from the user's own
            // keyboard map; unmapped events would silently do nothing.
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "ep128emu target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("ep128emu joystick control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("ep128emu requires measured native controls")?;
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
                    "Release the ep128emu controls before launch preparation"
                );
                let event = match translated {
                    DigitalInput::Button(index) => button_code(
                        u32::try_from(index).context("ep128emu button is too large")? + 1,
                    )?,
                    DigitalInput::Hat { index, direction } => hat_code(
                        u32::try_from(index).context("ep128emu hat is too large")?,
                        direction,
                    )?,
                    DigitalInput::Axis { index, .. } => axis_code(
                        u32::try_from(index).context("ep128emu axis is too large")? + 1,
                        native.direction > 0,
                    )?,
                };
                ensure!(
                    setup.event_rows.contains_key(&event),
                    "ep128emu event {event:#X} has no staged matrix row"
                );
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-ep128emu-")
                .tempdir()?;
            // $HOME/.ep128emu resolves under HOME; the launch layer points
            // it here so only the private config is visible.
            let home = directory.path().join("home");
            let dot = home.join(".ep128emu");
            fs::create_dir_all(&dot)?;
            // Windows user roots resolve under the session base as well.
            #[cfg(not(target_os = "linux"))]
            platform::prepare_user_dirs(&home)?;
            let baseline = fs::read(&setup.config_source)?;
            let mut config =
                String::from_utf8(baseline).context("ep128emu config source is not UTF-8")?;
            if !config.is_empty() && !config.ends_with('\n') {
                config.push('\n');
            }
            config.push_str(&keyboard_lines(&setup.event_rows)?);
            let config_path = dot.join("ep128emu.cfg");
            fs::write(&config_path, config)?;
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
                home,
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

        /// Private root for `HOME`; ep128emu appends `/.ep128emu`.
        pub(crate) fn home(&self) -> &std::path::Path {
            &self.home
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "ep128emu launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "ep128emu SDL index 0 moved before launch"
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
                "ep128emu executable differs from the saved trusted runtime"
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
                "ep128emu launch plan changed after preparation"
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
                    "ep128emu exited before controller handoff"
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
                    "ep128emu did not open the selected SDL controller before timeout"
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
            anyhow::bail!("ep128emu calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("ep128emu")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "ep128emu identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "ep128emu launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "ep128emu calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "ep128emu executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // $HOME/.ep128emu resolves under HOME; the snapshot keeps its
        // default -snapshot slot. Other hosts carry the session user roots
        // (USERPROFILE/APPDATA on Windows) over the same layout.
        #[cfg(target_os = "linux")]
        plan.environment.push((
            std::ffi::OsString::from("HOME"),
            inputs.home().as_os_str().to_owned(),
        ));
        #[cfg(not(target_os = "linux"))]
        {
            platform::prepare_user_dirs(inputs.home())?;
            for (key, value) in platform::session_user_env(inputs.home()) {
                plan.environment.push((key, value));
            }
        }
        plan.arguments = vec![
            std::ffi::OsString::from("-snapshot"),
            setup.content.as_os_str().to_owned(),
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
