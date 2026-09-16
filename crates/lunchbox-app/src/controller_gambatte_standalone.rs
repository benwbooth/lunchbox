//! Gambatte Qt standalone-native controller settings.
//!
//! The Qt frontend uses QSettings with organization `gambatte` and
//! application `gambatte_qt`. Its `input` group stores each event as a
//! packed SDL event id plus a value; the SDL frontend instead has only a
//! command-line `--input` mapping and no persistent profile.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "04e7ddf85ff23032cb7132f155c42c7d1857f474";

/// Order and labels used by `GambatteSource::createInputDialog`.
pub(crate) const CONTROLS: [(&str, &str); 8] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "A"),
    ("b", "B"),
    ("start", "Start"),
    ("select", "Select"),
];

const JOY_AXIS: u8 = 0x01;
const JOY_HAT: u8 = 0x02;
const JOY_BUTTON: u8 = 0x08;
const AXIS_POSITIVE: i32 = 1;
const AXIS_NEGATIVE: i32 = 2;
const HAT_UP: u8 = 0x01;
const HAT_RIGHT: u8 = 0x02;

/// One value accepted by the native Qt frontend's `InputBox`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Keyboard(u32),
    JoystickButton {
        device: u8,
        button: u8,
    },
    JoystickAxis {
        device: u8,
        axis: u8,
        positive: bool,
    },
    JoystickHat {
        device: u8,
        hat: u8,
        direction: u8,
    },
}

impl Binding {
    fn event(self) -> Result<(u32, i32)> {
        match self {
            Self::Keyboard(key) => {
                ensure!(key != 0, "Gambatte keyboard key cannot be zero");
                Ok((key, 0x7fff_ffff))
            }
            Self::JoystickButton { device, button } => {
                Ok((pack_event(JOY_BUTTON, device, button), 1))
            }
            Self::JoystickAxis {
                device,
                axis,
                positive,
            } => Ok((
                pack_event(JOY_AXIS, device, axis),
                if positive {
                    AXIS_POSITIVE
                } else {
                    AXIS_NEGATIVE
                },
            )),
            Self::JoystickHat {
                device,
                hat,
                direction,
            } => {
                ensure!(
                    direction != 0 && direction & !0x0f == 0,
                    "Gambatte hat direction is invalid"
                );
                Ok((pack_event(JOY_HAT, device, hat), i32::from(direction)))
            }
        }
    }
}

fn pack_event(kind: u8, device: u8, number: u8) -> u32 {
    u32::from(kind) | (u32::from(device) << 8) | (u32::from(number) << 16)
}

/// Render the `[input]` QSettings INI group used by `gambatte_qt`.
///
/// The second slot is explicitly cleared. `device` is SDL's runtime
/// joystick index, not a stable physical-device identifier.
pub(crate) fn input_ini(bindings: &BTreeMap<String, Binding>) -> Result<String> {
    ensure!(
        bindings.len() == CONTROLS.len()
            && CONTROLS
                .iter()
                .all(|(control, _)| bindings.contains_key(*control)),
        "Gambatte requires every Game Boy gameplay control"
    );

    let mut seen = BTreeSet::new();
    let mut out = String::from("[input]\n");
    for (control, label) in CONTROLS {
        let event = bindings[control].event()?;
        ensure!(
            seen.insert(event),
            "Gambatte mapping reuses one input event"
        );
        out.push_str(&format!(
            "Game{label}Key1={}\nGame{label}Value1={}\nGame{label}Key2=0\nGame{label}Value2=0\n",
            event.0, event.1
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controls() -> BTreeMap<String, Binding> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(i, (control, _))| {
                (
                    (*control).to_owned(),
                    Binding::JoystickButton {
                        device: 1,
                        button: i as u8,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn emits_qsettings_input_group_and_clears_alternates() {
        let text = input_ini(&controls()).unwrap();
        assert!(text.starts_with("[input]\n"));
        assert!(text.contains("GameUpKey1=264\nGameUpValue1=1\n"));
        assert!(text.contains("GameSelectKey2=0\nGameSelectValue2=0\n"));
    }

    #[test]
    fn emits_native_axis_and_hat_event_values() {
        let mut map = controls();
        map.insert(
            "up".to_owned(),
            Binding::JoystickAxis {
                device: 2,
                axis: 3,
                positive: false,
            },
        );
        map.insert(
            "down".to_owned(),
            Binding::JoystickHat {
                device: 2,
                hat: 1,
                direction: HAT_RIGHT | HAT_UP,
            },
        );
        let text = input_ini(&map).unwrap();
        assert!(text.contains("GameUpKey1=197121\nGameUpValue1=2\n"));
        assert!(text.contains("GameDownKey1=66050\nGameDownValue1=3\n"));
    }

    #[test]
    fn rejects_duplicate_gameplay_events() {
        let mut map = controls();
        map.insert(
            "b".to_owned(),
            Binding::JoystickButton {
                device: 1,
                button: 0,
            },
        );
        assert!(input_ini(&map).is_err());
    }

    #[test]
    fn keyboard_uses_inputbox_keyboard_sentinel() {
        let mut map = controls();
        map.insert("a".to_owned(), Binding::Keyboard(0x44));
        let text = input_ini(&map).unwrap();
        assert!(text.contains("GameAKey1=68\nGameAValue1=2147483647\n"));
    }
}

pub(crate) const PROFILE_ID: &str = "gambatte:standalone-gameboy";

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use anyhow::Context;
    use serde::{Deserialize, Serialize};
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
                "Gambatte setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Gambatte setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Gambatte setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Gambatte native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Gambatte supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Gambatte player needs a saved controller identity"
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
                .context("Missing Gambatte native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Gambatte controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "Gambatte mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Gambatte needs native calibration for every Game Boy control"
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
                "detail": "Native launch writes a private gambatte_qt.conf [input] group, then rechecks the exact SDL2 routes. Only the single Game Boy pad is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Gambatte saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Gambatte emulator/content setup"
            );
        }
        Ok(())
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
    use anyhow::Context;
    use lunchbox_controller_probe::{
        duckstation::DigitalInput,
        file_hash,
        linux_classic::AxisEndpoints,
        sdl2::{Device, Snapshot},
        sdl2_physical::PhysicalMap,
    };
    use std::{
        collections::HashMap,
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
            serde_json::from_slice(&output).context("Invalid Gambatte SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Gambatte helper inspected a different SDL2 runtime"
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

    fn mapped_bindings(
        calibration: &Calibration,
        device: &Device,
        device_index: u8,
    ) -> Result<BTreeMap<String, Binding>> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing Gambatte native profile")?;
        let physical = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("Gambatte SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("Gambatte SDL control counts are missing")?,
        )?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("Gambatte Game Boy control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Gambatte requires measured native controls")?;
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
                "Release the Gambatte controls before launch preparation"
            );
            let binding = match translated {
                DigitalInput::Button(index) => Binding::JoystickButton {
                    device: device_index,
                    button: u8::try_from(index).context("Gambatte button index is too large")?,
                },
                DigitalInput::Hat { index, direction } => Binding::JoystickHat {
                    device: device_index,
                    hat: u8::try_from(index).context("Gambatte hat index is too large")?,
                    direction,
                },
                DigitalInput::Axis { index, .. } => Binding::JoystickAxis {
                    device: device_index,
                    axis: u8::try_from(index).context("Gambatte axis index is too large")?,
                    positive: native.direction > 0,
                },
            };
            ensure!(
                result.insert(row.target_id, binding).is_none(),
                "Gambatte target control appears twice"
            );
        }
        ensure!(
            result.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|(control, _)| result.contains_key(*control)),
            "Gambatte native mapping is incomplete"
        );
        Ok(result)
    }

    pub(crate) struct PreparedSession {
        config_home: tempfile::TempDir,
        config_path: PathBuf,
        physical_path: String,
        device_index: u8,
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
                "Gambatte content must be a direct regular file with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Gambatte physical controller is missing or ambiguous"
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
                    "gambatte physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            let device_index = u8::try_from(device.device_index)
                .context("Gambatte SDL device index is too large")?;
            let bindings = mapped_bindings(
                calibrations
                    .get(&player.controller_id)
                    .context("Gambatte calibration disappeared")?,
                device,
                device_index,
            )?;
            let ini = input_ini(&bindings)?;
            let config_home = tempfile::Builder::new()
                .prefix("lunchbox-gambatte-config-")
                .tempdir()?;
            // QSettings with organization `gambatte` / application
            // `gambatte_qt` resolves this path under XDG_CONFIG_HOME on
            // Linux; absent keys fall back to frontend defaults.
            let config_dir = config_home.path().join("gambatte");
            fs::create_dir(&config_dir)?;
            let config_path = config_dir.join("gambatte_qt.conf");
            fs::write(&config_path, ini)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &config_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                config_home,
                config_path,
                physical_path,
                device_index,
                #[cfg(target_os = "linux")]
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn config_home(&self) -> &Path {
            self.config_home.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Gambatte launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            let routed = routing(captured);
            self.initial.ensure_same_routing(&routed)?;
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(
                &routed.devices,
                &self.physical_path,
                u32::from(self.device_index),
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
                u32::from(self.device_index),
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
                "Gambatte executable differs from the saved trusted runtime"
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
                "Gambatte launch plan changed after preparation"
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
                    "Gambatte exited before controller handoff"
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
                    "Gambatte did not open the selected SDL2 controller before timeout"
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
            anyhow::bail!("Gambatte calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Gambatte")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Gambatte identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Gambatte launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Gambatte calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Gambatte executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // QSettings resolves gambatte_qt.conf under XDG_CONFIG_HOME.
        plan.environment.push((
            std::ffi::OsString::from("XDG_CONFIG_HOME"),
            inputs.config_home().as_os_str().to_owned(),
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
