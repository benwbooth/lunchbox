//! Atari800 SDL2 input overlay writer.
//!
//! Atari800's host identity is SDL display-name plus duplicate-name slot. The
//! native Linux session therefore probes the exact SDL2 runtime, translates
//! measured controls, and rechecks both ordered names and physical topology
//! before launching through an overlay of the selected configuration file.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "fe1d2890d9f05fcecb2fd033d09a5c43f534bebf";
pub(crate) const PROFILE_ID: &str = "atari800:standalone-digital-joystick";
pub(crate) const CONTROLS: [(&str, &str); 5] = [
    ("up", "SDL joystick vertical negative"),
    ("down", "SDL joystick vertical positive"),
    ("left", "SDL joystick horizontal negative"),
    ("right", "SDL joystick horizontal positive"),
    ("fire", "Atari trigger"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectionSource {
    Auto,
    Hat,
    Axes { first_axis: u8 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortOverlay {
    /// Zero-based Atari800 port, matching SDL2_JOY_PORT_<n>.
    pub port: u8,
    pub device_name: String,
    /// Duplicate-name slot, zero-based, as written by Atari800.
    pub device_slot: u8,
    pub source: DirectionSource,
    /// (SDL button index, Atari800 action enum, Atari key/action value).
    pub fire: (u8, i32, i32),
    pub diagonal_zone: u8,
}

fn safe_name(value: &str) -> Result<()> {
    ensure!(!value.is_empty(), "Atari800 device name cannot be empty");
    ensure!(value.len() <= 256, "Atari800 device name is too long");
    ensure!(
        !value
            .chars()
            .any(|ch| ch == '\n' || ch == '\r' || ch == '='),
        "Atari800 device name contains a config delimiter"
    );
    Ok(())
}

fn render(overlay: &PortOverlay) -> Result<Vec<String>> {
    ensure!(overlay.port < 4, "Atari800 port must be 0..3");
    safe_name(&overlay.device_name)?;
    ensure!(
        overlay.diagonal_zone <= 2,
        "Atari800 diagonal zone must be 0..2"
    );
    ensure!(overlay.fire.0 < 15, "Atari800 fire button must be 0..14");
    ensure!(
        (0..=3).contains(&overlay.fire.1),
        "Atari800 action enum is invalid"
    );
    let mut lines = vec![
        format!("SDL2_JOY_PORT_{}_MODE=4", overlay.port),
        format!(
            "SDL2_JOY_PORT_{}_NAME={}",
            overlay.port, overlay.device_name
        ),
        format!(
            "SDL2_JOY_PORT_{}_SLOT={}",
            overlay.port, overlay.device_slot
        ),
    ];
    match overlay.source {
        DirectionSource::Auto => lines.push(format!("SDL2_JOY_PORT_{}_USE_HAT=-1", overlay.port)),
        DirectionSource::Hat => lines.push(format!("SDL2_JOY_PORT_{}_USE_HAT=1", overlay.port)),
        DirectionSource::Axes { first_axis } => {
            ensure!(
                matches!(first_axis, 0 | 2),
                "Atari800 axis base must be the source-supported pair 0 or 2"
            );
            lines.push(format!("SDL2_JOY_PORT_{}_USE_HAT=0", overlay.port));
            lines.push(format!("SDL2_JOY_PORT_{}_AXES={first_axis}", overlay.port));
        }
    }
    lines.push(format!(
        "SDL2_JOY_PORT_{}_DIAGONALS={}",
        overlay.port, overlay.diagonal_zone
    ));
    let mut actions = vec![0; 15];
    let mut keys = vec![0; 15];
    actions[usize::from(overlay.fire.0)] = overlay.fire.1;
    keys[usize::from(overlay.fire.0)] = overlay.fire.2;
    lines.push(format!(
        "SDL2_JOY_PORT_{}_BUTTON_ACTIONS={},",
        overlay.port,
        actions
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ));
    lines.push(format!(
        "SDL2_JOY_PORT_{}_BUTTON_KEYS={},",
        overlay.port,
        keys.iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ));
    Ok(lines)
}

/// Replace only the explicitly named port keys, preserving every unrelated
/// Atari800 setting and all other ports.
pub(crate) fn patch_config(baseline: &[u8], overlays: &[PortOverlay]) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "Atari800 config is too large"
    );
    ensure!(overlays.len() <= 4, "Atari800 has four joystick ports");
    let mut seen = BTreeSet::new();
    let mut replacement = Vec::new();
    let mut prefixes = BTreeSet::new();
    for overlay in overlays {
        ensure!(seen.insert(overlay.port), "Atari800 port is duplicated");
        prefixes.insert(format!("SDL2_JOY_PORT_{}", overlay.port));
        replacement.extend(render(overlay)?);
    }
    let text = std::str::from_utf8(baseline).context("Atari800 config is not UTF-8")?;
    let mut kept = Vec::new();
    for line in text.lines() {
        let key = line.split_once('=').map(|(key, _)| key.trim());
        if key.is_some_and(|key| prefixes.iter().any(|prefix| key.starts_with(prefix))) {
            continue;
        }
        kept.push(line);
    }
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&replacement.join("\n"));
    out.push('\n');
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
        /// Existing selected configuration. It is copied and overlaid at this
        /// exact path so Atari800's quick-save data directory remains native.
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        /// Exact SDL2 library linked by the selected Atari800 build.
        pub sdl_library: PathBuf,
        pub bubblewrap_program: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Atari800 setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
                &self.bubblewrap_program,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Atari800 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().is_some(),
                "Atari800 config_path must name a configuration file"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Atari800 setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Atari800 native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "Atari800 setup has an invalid player count"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Atari800 players must be distinct, contiguous, and start at one"
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
                .context("Missing Atari800 native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Atari800 controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux",
                    "Atari800 mapping requires Linux calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| {
                        row.physical_id.is_some()
                            && row
                                .input
                                .as_ref()
                                .is_some_and(|input| input.native.is_some())
                    }),
                    "Atari800 needs native calibration for every joystick control"
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
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch overlays only the selected config at its original path, rechecks exact SDL2 names, duplicate-name slots and raw controls, and leaves mounted media, firmware paths and explicit state destinations native. The quick-save remains beside the real selected config. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Atari800 saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Atari800 emulator/content setup"
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
    use lunchbox_controller_probe::{
        duckstation::DigitalInput, file_hash, linux_classic::AxisEndpoints, sdl2::Snapshot,
        sdl2_physical::PhysicalMap,
    };
    use std::{
        collections::{BTreeMap, HashMap},
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
            serde_json::from_slice(&output).context("Invalid Atari800 SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Atari800 helper inspected a different SDL2 runtime"
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

    fn ensure_same_names(initial: &Snapshot, fresh: &Snapshot) -> Result<()> {
        ensure!(
            initial.devices.len() == fresh.devices.len()
                && initial
                    .devices
                    .iter()
                    .zip(&fresh.devices)
                    .all(|(first, second)| first.name == second.name),
            "Atari800 SDL2 device names changed during launch preparation"
        );
        Ok(())
    }

    fn inputs(
        calibration: &Calibration,
        snapshot: &Snapshot,
        runtime_path: &str,
    ) -> Result<BTreeMap<String, DigitalInput>> {
        let device = snapshot.device_at_path(runtime_path)?;
        let physical = PhysicalMap::from_device(device)?;
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing Atari800 native profile")?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("Atari800 joystick control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Atari800 requires measured native controls")?;
            let endpoints = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let binding = physical.digital_input(native.code, endpoints)?;
            ensure!(
                result.insert(row.target_id, binding).is_none(),
                "Atari800 target control appears twice"
            );
        }
        Ok(result)
    }

    fn direction_source(mapped: &BTreeMap<String, DigitalInput>) -> Result<DirectionSource> {
        let get = |name: &str| {
            mapped
                .get(name)
                .cloned()
                .with_context(|| format!("Atari800 control {name} is absent"))
        };
        let [up, down, left, right] = [get("up")?, get("down")?, get("left")?, get("right")?];
        let axis_pair =
            |negative: &DigitalInput, positive: &DigitalInput| match (negative, positive) {
                (
                    DigitalInput::Axis {
                        index: first,
                        released: first_rest,
                        pressed: first_press,
                    },
                    DigitalInput::Axis {
                        index: second,
                        released: second_rest,
                        pressed: second_press,
                    },
                ) if first == second
                    && first_rest.abs() < 10_000
                    && second_rest.abs() < 10_000
                    && *first_press < -10_000
                    && *second_press > 10_000 =>
                {
                    Some(*first)
                }
                _ => None,
            };
        if let (Some(horizontal), Some(vertical)) =
            (axis_pair(&left, &right), axis_pair(&up, &down))
        {
            ensure!(
                matches!((horizontal, vertical), (0, 1) | (2, 3)),
                "Atari800 directions must use source-supported SDL axis pair 0/1 or 2/3"
            );
            return Ok(DirectionSource::Axes {
                first_axis: u8::try_from(horizontal)?,
            });
        }
        let hat = |first: &DigitalInput, second: &DigitalInput, a: u8, b: u8| {
            matches!((first, second),
                (DigitalInput::Hat { index: first_hat, direction: first_direction },
                 DigitalInput::Hat { index: second_hat, direction: second_direction })
                if *first_hat == 0 && *second_hat == 0
                    && *first_direction == a && *second_direction == b)
        };
        ensure!(
            hat(&left, &right, 8, 2) && hat(&up, &down, 1, 4),
            "Atari800 directions must use SDL axis pair 0/1, 2/3, or cardinal hat 0"
        );
        Ok(DirectionSource::Hat)
    }

    fn overlay(
        player: &settings::Player,
        name: String,
        slot: u8,
        mapped: &BTreeMap<String, DigitalInput>,
    ) -> Result<PortOverlay> {
        let fire = match mapped
            .get("fire")
            .context("Atari800 Fire control is absent")?
        {
            DigitalInput::Button(index) if *index < 15 => u8::try_from(*index)?,
            _ => anyhow::bail!("Atari800 Fire must be SDL joystick button 0..14"),
        };
        Ok(PortOverlay {
            port: player.player - 1,
            device_name: name,
            device_slot: slot,
            source: direction_source(mapped)?,
            fire: (fire, 1, -100),
            diagonal_zone: 1,
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) private_config: PathBuf,
        runtime_paths: Vec<String>,
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
            let mut selected = Vec::new();
            for player in &setup.players {
                let matches = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    matches.len() == 1 && !matches[0].is_virtual,
                    "Atari800 physical controller is missing or ambiguous"
                );
                selected.push(matches[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            let mut runtime_paths = Vec::new();
            let mut overlays = Vec::new();
            for (player, selected) in setup.players.iter().zip(&selected) {
                let runtime_path = topology.resolve_runtime_path(
                    selected,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                ensure!(
                    !runtime_paths.contains(&runtime_path),
                    "Atari800 players resolved to the same controller"
                );
                let captured = observe(setup, Some(&runtime_path), cancel)?;
                let captured_routing = routing(captured.clone());
                initial.ensure_same_routing(&captured_routing)?;
                ensure_same_names(&initial, &captured_routing)?;
                topology.verify()?;
                let device = captured.device_at_path(&runtime_path)?;
                let name = device
                    .name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .context("Atari800 selected SDL joystick has no name")?;
                let slot = captured
                    .devices
                    .iter()
                    .take(device.device_index as usize)
                    .filter(|candidate| candidate.name.as_ref() == Some(&name))
                    .count();
                overlays.push(overlay(
                    player,
                    name,
                    u8::try_from(slot)?,
                    &inputs(
                        calibrations
                            .get(&player.controller_id)
                            .context("Atari800 calibration disappeared")?,
                        &captured,
                        &runtime_path,
                    )?,
                )?);
                runtime_paths.push(runtime_path);
            }
            topology.verify()?;
            let baseline = std::fs::read(&setup.config_path)
                .context("Reading the declared Atari800 configuration")?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-atari800-native-")
                .tempdir()?;
            let private_config = directory.path().join("atari800.cfg");
            std::fs::write(&private_config, patch_config(&baseline, &overlays)?)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.config_path,
                &setup.probe_program,
                &setup.sdl_library,
                &setup.bubblewrap_program,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let session = Self {
                directory,
                private_config,
                runtime_paths,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir() && self.private_config.is_file(),
                "Atari800 private configuration disappeared"
            );
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "Atari800 launch input changed"
                );
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            ensure_same_names(&self.initial, &fresh)?;
            for path in &self.runtime_paths {
                fresh.device_at_path(path)?;
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
        controller_native_process::cancelled,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};

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
                "Atari800 executable differs from the saved trusted runtime"
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
                "Atari800 launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
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
            anyhow::bail!("Atari800 calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Atari800")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "Atari800 identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Atari800 executable differs from selection"
        );
        ensure!(
            original
                .arguments
                .iter()
                .filter(|argument| *argument == setup.content.as_os_str())
                .count()
                == 1
                && !original
                    .arguments
                    .iter()
                    .any(|argument| argument == "-config"),
            "Atari800 calibrated launch needs the saved content exactly once and no competing -config option"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Atari800 executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let cwd = original.current_directory.canonicalize()?;
        let mut arguments = vec![
            "--die-with-parent".into(),
            "--bind".into(),
            "/".into(),
            "/".into(),
            "--bind".into(),
            inputs.private_config.as_os_str().to_owned(),
            setup.config_path.as_os_str().to_owned(),
            "--chdir".into(),
            cwd.into_os_string(),
            "--".into(),
            executable.as_os_str().to_owned(),
            "-config".into(),
            setup.config_path.as_os_str().to_owned(),
        ];
        arguments.extend(original.arguments.iter().cloned());
        let mut plan = original.clone();
        plan.program = setup.bubblewrap_program.clone();
        plan.arguments = arguments;
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
    fn patches_one_measured_port_and_preserves_other_settings() {
        let overlay = PortOverlay {
            port: 0,
            device_name: "Pad One".into(),
            device_slot: 0,
            source: DirectionSource::Axes { first_axis: 0 },
            fire: (0, 1, -100),
            diagonal_zone: 2,
        };
        let text = patch_config(
            b"MACHINE_TYPE=Atari XL\nSDL2_JOY_PORT_0_MODE=1\n",
            &[overlay],
        )
        .unwrap();
        assert!(text.contains("MACHINE_TYPE=Atari XL"));
        assert!(text.contains("SDL2_JOY_PORT_0_MODE=4"));
        assert!(text.contains("SDL2_JOY_PORT_0_AXES=0"));
        assert!(text.contains("SDL2_JOY_PORT_0_BUTTON_ACTIONS=1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,"));
        assert!(text.contains("SDL2_JOY_PORT_0_BUTTON_KEYS=-100,0,0,0,0,0,0,0,0,0,0,0,0,0,0,"));
        assert!(!text.contains("SDL2_JOY_PORT_0_MODE=1"));
    }

    #[test]
    fn rejects_axis_pairs_that_the_source_parser_silently_clamps() {
        let overlay = PortOverlay {
            port: 0,
            device_name: "Pad One".into(),
            device_slot: 0,
            source: DirectionSource::Axes { first_axis: 4 },
            fire: (0, 1, -100),
            diagonal_zone: 1,
        };
        assert!(patch_config(b"", &[overlay]).is_err());
    }
}
