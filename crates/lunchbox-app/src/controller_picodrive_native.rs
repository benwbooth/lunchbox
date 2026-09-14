//! PicoDrive native input.cfg writer.
//!
//! Pinned source: notaz/picodrive@26ecb2b6358fefba24e3d68b9eb2efba7f10d5ee.
//! `platform/common/config_file.c` writes `binddev = ...` and
//! `bind <key> = playerN <action>` lines. This renderer follows that grammar
//! and refuses ambiguous device/action text.

use anyhow::{Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "26ecb2b6358fefba24e3d68b9eb2efba7f10d5ee";
pub(crate) const PROFILE_ID: &str = "picodrive:standalone-genesis-6";
pub(crate) const PLAYER_ACTIONS: [&str; 15] = [
    "UP", "DOWN", "LEFT", "RIGHT", "A", "B", "C", "START", "MODE", "X", "Y", "Z", "A turbo",
    "B turbo", "C turbo",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Binding {
    pub host_key: String,
    pub player: u8,
    pub action: String,
}

fn valid_atom(value: &str, what: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 64,
        "PicoDrive {what} is empty or too long"
    );
    ensure!(
        !value.chars().any(|c| c == '\n' || c == '\r' || c == '\0'),
        "PicoDrive {what} contains a control character"
    );
    Ok(())
}

fn escaped_key(key: &str) -> Result<String> {
    valid_atom(key, "host key")?;
    Ok(match key {
        "#" => "\\x23".to_owned(),
        "=" => "\\x3d".to_owned(),
        _ => key.to_owned(),
    })
}

pub(crate) fn config_text(device: &str, bindings: &[Binding]) -> Result<String> {
    valid_atom(device, "device name")?;
    ensure!(
        !device.contains(['#', '=']),
        "PicoDrive device name conflicts with the source line grammar"
    );
    ensure!(
        !bindings.is_empty() && bindings.len() <= 128,
        "PicoDrive binding count is invalid"
    );
    // `config_file.c` uses CRLF on the non-MSVC path and LF on MSVC.
    let newline = "\r\n";
    let mut out = format!("binddev = {device}{newline}");
    let mut seen = BTreeSet::new();
    for binding in bindings {
        ensure!(
            (1..=4).contains(&binding.player),
            "PicoDrive player must be 1..4"
        );
        ensure!(
            PLAYER_ACTIONS.contains(&binding.action.as_str()),
            "PicoDrive action is not in the source action table"
        );
        ensure!(
            seen.insert((&binding.host_key, binding.player, &binding.action)),
            "Duplicate PicoDrive binding"
        );
        let key = escaped_key(&binding.host_key)?;
        out.push_str(&format!(
            "bind {key} = player{} {}{newline}",
            binding.player, binding.action
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_source_bind_grammar_and_escapes_reserved_keys() {
        let text = config_text(
            "SDL Controller",
            &[
                Binding {
                    host_key: "#".into(),
                    player: 1,
                    action: "A".into(),
                },
                Binding {
                    host_key: "=".into(),
                    player: 2,
                    action: "START".into(),
                },
            ],
        )
        .unwrap();
        assert_eq!(
            text,
            "binddev = SDL Controller\r\nbind \\x23 = player1 A\r\nbind \\x3d = player2 START\r\n"
        );
    }

    #[test]
    fn joystick_buttons_encode_as_world_keysyms() {
        assert_eq!(host_key_for_button(0).unwrap(), "\\x80");
        assert_eq!(host_key_for_button(5).unwrap(), "\\x85");
        assert_eq!(host_key_for_button(95).unwrap(), "\\xDF");
        assert!(host_key_for_button(96).is_err());
    }

    #[test]
    fn stick_axes_encode_as_arrow_keys() {
        assert_eq!(host_key_for_axis(0, false).unwrap(), "left");
        assert_eq!(host_key_for_axis(0, true).unwrap(), "right");
        assert_eq!(host_key_for_axis(1, false).unwrap(), "up");
        assert_eq!(host_key_for_axis(1, true).unwrap(), "down");
        assert!(host_key_for_axis(2, true).is_err());
    }

    #[test]
    fn bind_device_names_carry_the_sdl_prefix() {
        assert_eq!(bind_device_name("Pad").unwrap(), "sdl:Pad");
        assert!(bind_device_name("").is_err());
        assert!(bind_device_name("a#b").is_err());
    }

    #[test]
    fn routes_cover_every_genesis_layout_control() {
        assert_eq!(ROUTES.len(), 12);
        for (target, action) in ROUTES {
            assert!(PLAYER_ACTIONS.contains(&action));
            assert!(!target.is_empty());
        }
        let mut targets: Vec<&str> = ROUTES.iter().map(|(t, _)| *t).collect();
        targets.sort_unstable();
        targets.dedup();
        assert_eq!(targets.len(), ROUTES.len());
    }

    #[test]
    fn rejects_unknown_actions_and_injection() {
        assert!(
            config_text(
                "pad",
                &[Binding {
                    host_key: "A\n".into(),
                    player: 1,
                    action: "A".into()
                }]
            )
            .is_err()
        );
        assert!(
            config_text(
                "pad",
                &[Binding {
                    host_key: "A".into(),
                    player: 1,
                    action: "reset".into()
                }]
            )
            .is_err()
        );
        assert!(
            config_text(
                "pad#comment",
                &[Binding {
                    host_key: "A".into(),
                    player: 1,
                    action: "A".into()
                }]
            )
            .is_err()
        );
        let duplicate = Binding {
            host_key: "A".into(),
            player: 1,
            action: "A".into(),
        };
        assert!(config_text("pad", &[duplicate.clone(), duplicate]).is_err());
    }
}

/// Layout target id to source action name for the `genesis-6` layout.
pub(crate) const ROUTES: [(&str, &str); 12] = [
    ("up", "UP"),
    ("down", "DOWN"),
    ("left", "LEFT"),
    ("right", "RIGHT"),
    ("a", "A"),
    ("b", "B"),
    ("c", "C"),
    ("start", "START"),
    ("mode", "MODE"),
    ("x", "X"),
    ("y", "Y"),
    ("z", "Z"),
];

/// SDL1.2 gameplay input encodes joystick buttons as `SDLK_WORLD_0 + index`
/// (`handle_joy_event`, libpicofe `in_sdl.c` at the pinned submodule commit
/// `c3031dbfda557f7058756583b329b59ea92a72dc`). Unnamed keysyms serialize as
/// `\x%02X`, which `parse_key` reads back with `strtoul(16)`, so button N
/// binds as `\x80+N`. SDL reserves `WORLD_0..WORLD_95`; higher indices have
/// no defined keysym there.
pub(crate) fn host_key_for_button(button: u16) -> Result<String> {
    ensure!(
        button < 96,
        "PicoDrive SDL buttons end at the WORLD key range"
    );
    Ok(format!("\\x{:02X}", 0x80 + button))
}

/// Axes 0/1 are the only stick axes the source handles, as digital arrow
/// keys with a +/-16384 deadzone. Higher axes, hats, and balls have no case
/// in `handle_joy_event` and must be refused, not guessed.
pub(crate) fn host_key_for_axis(axis: u16, positive: bool) -> Result<String> {
    Ok(match (axis, positive) {
        (0, false) => "left".to_owned(),
        (0, true) => "right".to_owned(),
        (1, false) => "up".to_owned(),
        (1, true) => "down".to_owned(),
        _ => anyhow::bail!("PicoDrive SDL handles only axes 0/1 as digital directions"),
    })
}

/// Full `binddev` device name: the `sdl:` driver prefix plus the SDL
/// joystick name, matched with `strcmp` by `in_config_parse_dev`. Names
/// must be exactly unique; identical pads are indistinguishable to the
/// source itself.
pub(crate) fn bind_device_name(joystick_name: &str) -> Result<String> {
    ensure!(
        !joystick_name.is_empty()
            && joystick_name.len() <= 256
            && !joystick_name.contains(['#', '='])
            && joystick_name
                .chars()
                .all(|ch| ch.is_ascii_graphic() || ch == ' '),
        "PicoDrive joystick name is unsafe for binddev"
    );
    Ok(format!("sdl:{joystick_name}"))
}

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
                "PicoDrive setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "PicoDrive setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "PicoDrive setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing PicoDrive native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "PicoDrive setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "PicoDrive players must be distinct, contiguous, and start at player one"
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
                .context("Missing PicoDrive native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("PicoDrive controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux",
                    "PicoDrive mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "PicoDrive needs native calibration for every Genesis control"
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
                "detail": "Native Linux launch writes a private config with binddev/bind lines, then rechecks the exact SDL routes. Only Genesis six-button pads on ports one/two are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many PicoDrive saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate PicoDrive emulator/content setup"
            );
        }
        Ok(())
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
    use anyhow::Context;
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
            serde_json::from_slice(&output).context("Invalid PicoDrive SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "PicoDrive helper inspected a different SDL2 runtime"
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

    fn player_bindings(
        calibration: &Calibration,
        device: &Device,
        player: u8,
    ) -> Result<Vec<Binding>> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing PicoDrive native profile")?;
        let physical = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("PicoDrive SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("PicoDrive SDL control counts are missing")?,
        )?;
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        for row in calibration.plan_profile(profile)?.rows {
            let action = ROUTES
                .iter()
                .find(|(target, _)| *target == row.target_id)
                .map(|(_, action)| (*action).to_owned())
                .with_context(|| format!("PicoDrive target {} outside contract", row.target_id))?;
            let input = row
                .input
                .as_ref()
                .context("PicoDrive Genesis control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("PicoDrive requires measured native controls")?;
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
                "Release the PicoDrive controls before launch preparation"
            );
            let host_key = match translated {
                DigitalInput::Button(index) => host_key_for_button(
                    u16::try_from(index).context("PicoDrive button index is too large")?,
                )?,
                DigitalInput::Axis { index, .. } => {
                    let axis = u16::try_from(index).context("PicoDrive axis index is too large")?;
                    host_key_for_axis(axis, native.direction > 0)?
                }
                DigitalInput::Hat { .. } => {
                    anyhow::bail!("PicoDrive SDL handles no hat inputs")
                }
            };
            ensure!(
                seen.insert((host_key.clone(), action.clone())),
                "PicoDrive host key maps twice"
            );
            result.push(Binding {
                host_key,
                player,
                action,
            });
        }
        ensure!(
            result.len() == ROUTES.len(),
            "PicoDrive native mapping is incomplete"
        );
        Ok(result)
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        config_path: PathBuf,
        physical_paths: Vec<String>,
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
                "PicoDrive content must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "PicoDrive physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            let mut physical_paths = Vec::new();
            let mut names = Vec::new();
            let mut all_bindings = Vec::new();
            for (player, selected_path) in setup.players.iter().zip(&selected) {
                let path = topology.resolve_runtime_path(
                    selected_path,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                ensure!(
                    !physical_paths.contains(&path),
                    "PicoDrive players share a controller"
                );
                let captured = observe(setup, Some(&path), cancel)?;
                initial.ensure_same_routing(&routing(captured.clone()))?;
                topology.verify()?;
                let device = captured.device_at_path(&path)?;
                let joystick_name = device
                    .name
                    .as_deref()
                    .context("PicoDrive SDL device has no name")?;
                ensure!(
                    initial
                        .devices
                        .iter()
                        .filter_map(|other| other.name.as_deref())
                        .filter(|other| *other == joystick_name)
                        .count()
                        == 1,
                    "PicoDrive SDL joystick name is duplicated"
                );
                names.push(bind_device_name(joystick_name)?);
                all_bindings.extend(player_bindings(
                    calibrations
                        .get(&player.controller_id)
                        .context("PicoDrive calibration disappeared")?,
                    device,
                    player.player,
                )?);
                physical_paths.push(path);
            }
            ensure!(
                names.iter().collect::<BTreeSet<_>>().len() == names.len(),
                "PicoDrive players share a joystick identity"
            );
            let mut text = String::new();
            for name in &names {
                text.push_str(&config_text(name, &all_bindings)?);
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-picodrive-")
                .tempdir()?;
            let config_path = directory.path().join("picodrive-input.cfg");
            fs::write(&config_path, text)?;
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
                directory,
                config_path,
                physical_paths,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn config_path(&self) -> &std::path::Path {
            &self.config_path
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "PicoDrive launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for path in &self.physical_paths {
                let captured = observe(&self.setup, Some(path), cancel)?;
                self.initial.ensure_same_routing(&routing(captured))?;
                self.topology.verify()?;
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
                "PicoDrive executable differs from the saved trusted runtime"
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
                "PicoDrive launch plan changed after preparation"
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
                    "PicoDrive exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "PicoDrive did not open the selected SDL controllers before timeout"
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
            anyhow::bail!("PicoDrive calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("PicoDrive")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "PicoDrive identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "PicoDrive launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "PicoDrive calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "PicoDrive executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        // The parser stops option processing at the first readable file
        // argument, so -config must precede the ROM path.
        let mut plan = original.clone();
        plan.program = executable.clone();
        plan.arguments = vec![
            std::ffi::OsString::from("-config"),
            inputs.config_path().as_os_str().to_owned(),
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
