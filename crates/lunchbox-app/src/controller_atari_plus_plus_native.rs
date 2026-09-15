//! Atari++ native Linux joystick overlay writer.
//!
//! The 1.85 source reads Linux joystick units directly through
//! `/dev/input/js<n>`.  This writer emits only the documented AnalogJoystick
//! path after the caller has probed that exact unit; it never substitutes the
//! SDL numeric-unit or digital/paddle modes.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_VERSION: &str = "1.85";
pub(crate) const PROFILE_ID: &str = "atari-plus-plus:standalone-atari-plus-plus-native-joystick";
pub(crate) const CONTROLS: [(&str, &str); 8] = [
    ("up", "AnalogJoystick vertical negative"),
    ("down", "AnalogJoystick vertical positive"),
    ("left", "AnalogJoystick horizontal negative"),
    ("right", "AnalogJoystick horizontal positive"),
    ("fire", "First_Button"),
    ("fire2", "Second_Button"),
    ("fire3", "Third_Button"),
    ("fire4", "Forth_Button"),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortOverlay {
    /// Zero-based emulated Atari++ port (0..3).
    pub port: u8,
    /// Zero-based Linux joystick unit, corresponding to /dev/input/js<n>.
    pub unit: u8,
    pub sensitivity: u16,
    pub horizontal_axis: u8,
    pub vertical_axis: u8,
    /// Physical button numbers are one-based in Atari++'s configuration.
    pub buttons: [u8; 4],
}

fn axis_name(axis: u8) -> Result<&'static str> {
    Ok(match axis {
        0 => "XAxis.1",
        1 => "YAxis.1",
        2 => "XAxis.2",
        3 => "YAxis.2",
        _ => anyhow::bail!("Atari++ axis must be 0..3"),
    })
}

fn render(overlay: &PortOverlay) -> Result<Vec<String>> {
    ensure!(overlay.port < 4, "Atari++ joystick port must be 0..3");
    ensure!(overlay.unit < 8, "Atari++ joystick unit must be 0..7");
    ensure!(
        overlay
            .buttons
            .iter()
            .all(|button| (1..=16).contains(button)),
        "Atari++ button must be 1..16"
    );
    ensure!(
        BTreeSet::from(overlay.buttons).len() == overlay.buttons.len(),
        "Atari++ abstract buttons must be distinct"
    );
    let horizontal = axis_name(overlay.horizontal_axis)?;
    let vertical = axis_name(overlay.vertical_axis)?;
    Ok(vec![
        format!(
            "Joystick.{}.Port = AnalogJoystick.{}",
            overlay.port, overlay.unit
        ),
        format!(
            "Joystick.{}.Sensitivity = {}",
            overlay.port, overlay.sensitivity
        ),
        format!("HAxis.{} = {horizontal}", overlay.unit),
        format!("VAxis.{} = {vertical}", overlay.unit),
        format!("First_Button.{} = {}", overlay.unit, overlay.buttons[0]),
        format!("Second_Button.{} = {}", overlay.unit, overlay.buttons[1]),
        format!("Third_Button.{} = {}", overlay.unit, overlay.buttons[2]),
        format!("Forth_Button.{} = {}", overlay.unit, overlay.buttons[3]),
    ])
}

/// Replace only the explicitly generated Atari++ joystick options and retain
/// all unrelated machine, disk, ROM, and frontend settings.
pub(crate) fn patch_config(baseline: &[u8], overlays: &[PortOverlay]) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "Atari++ config is too large"
    );
    ensure!(overlays.len() <= 4, "Atari++ has four joystick ports");
    let mut seen_ports = BTreeSet::new();
    let mut seen_units = BTreeSet::new();
    let mut generated = Vec::new();
    let mut keys = BTreeSet::new();
    for overlay in overlays {
        ensure!(
            seen_ports.insert(overlay.port),
            "Atari++ port is duplicated"
        );
        ensure!(
            seen_units.insert(overlay.unit),
            "Atari++ joystick unit is reused"
        );
        keys.insert(format!("Joystick.{}.", overlay.port));
        keys.insert(format!("HAxis.{}", overlay.unit));
        keys.insert(format!("VAxis.{}", overlay.unit));
        for name in [
            "First_Button",
            "Second_Button",
            "Third_Button",
            "Forth_Button",
        ] {
            keys.insert(format!("{name}.{}", overlay.unit));
        }
        generated.extend(render(overlay)?);
    }
    let text = std::str::from_utf8(baseline).context("Atari++ config is not UTF-8")?;
    let mut kept = Vec::new();
    for line in text.lines() {
        let key = line.split_once('=').map(|(key, _)| key.trim());
        if key.is_some_and(|key| {
            keys.iter().any(|owned| {
                if owned.ends_with('.') {
                    key.starts_with(owned)
                } else {
                    key == owned
                }
            })
        }) {
            continue;
        }
        kept.push(line);
    }
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&generated.join("\n"));
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
        pub config_path: PathBuf,
        pub bubblewrap_program: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Atari++ setup needs an emulator identity"
            );
            for path in [&self.content, &self.config_path, &self.bubblewrap_program] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Atari++ setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Atari++ setup needs a trusted executable SHA-256"
            );
            ensure!(
                matches!(self.players.len(), 1..=4),
                "Atari++ setup needs one through four joystick players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Atari++ players must be distinct, contiguous, and start at one"
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
                .context("Missing Atari++ native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Atari++ controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux",
                    "Atari++ AnalogJoystick requires Linux calibration"
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
                    "Atari++ needs native calibration for every joystick control"
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
                "detail": "Native Linux launch overlays only the selected configuration at its original path and remaps exact launch-owned joydev nodes. Mounted media, firmware paths, guest saves, and user-selected snapshots retain their native locations. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Atari++ saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Atari++ emulator/content setup"
            );
        }
        Ok(())
    }
}

// Linux-only by source contract: the session reads the kernel joystick
// device directly (`linux_classic::read_raw`) and the pinned mapper
// interprets raw joydev axes/buttons. Other hosts have no joydev nodes,
// so no port is staged.
#[cfg(target_os = "linux")]
mod session {
    use super::*;
    use crate::{
        controller_bizhawk_guard::InputTopology,
        controller_catalog::{Calibration, InputBinding},
        controller_native_process::cancelled,
        controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{duckstation::DigitalInput, file_hash, linux_classic::RawMap};
    use std::{
        collections::{BTreeMap, HashMap},
        path::PathBuf,
        sync::atomic::AtomicBool,
    };

    fn input(raw: &RawMap, binding: &InputBinding) -> Result<DigitalInput> {
        let native = binding
            .native
            .as_ref()
            .context("Atari++ requires a measured native control")?;
        let code = (native.code & 0xffff) as u16;
        match native.code >> 16 {
            1 => raw
                .buttons
                .iter()
                .position(|candidate| *candidate == code)
                .map(|index| DigitalInput::Button(index as u32))
                .context("Atari++ physical button is absent from joydev"),
            3 => {
                let code = u8::try_from(code).context("Invalid Atari++ physical axis")?;
                let index = raw
                    .axes
                    .iter()
                    .position(|candidate| *candidate == code)
                    .context("Atari++ physical axis is absent from joydev")?;
                let endpoints = binding
                    .axis
                    .as_ref()
                    .context("Atari++ axis measurements are absent")?;
                let correction = raw
                    .corrections
                    .get(&code)
                    .context("Atari++ joydev axis correction is absent")?;
                Ok(DigitalInput::Axis {
                    index: index as u32,
                    released: i16::try_from(correction.apply(endpoints.released)?)?,
                    pressed: i16::try_from(correction.apply(endpoints.pressed)?)?,
                })
            }
            _ => anyhow::bail!("Atari++ calibration uses an unsupported event type"),
        }
    }

    fn axis_pair(negative: DigitalInput, positive: DigitalInput, name: &str) -> Result<u8> {
        match (negative, positive) {
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
                && first <= 3
                && first_rest.abs() < 10_000
                && second_rest.abs() < 10_000
                && first_press < -10_000
                && second_press > 10_000 =>
            {
                Ok(u8::try_from(first)?)
            }
            _ => anyhow::bail!(
                "Atari++ {name} directions require opposite halves of joydev axis 0..3"
            ),
        }
    }

    fn overlay(
        player: &settings::Player,
        raw: &RawMap,
        calibration: &Calibration,
    ) -> Result<PortOverlay> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing Atari++ native profile")?;
        let mapped = calibration
            .plan_profile(profile)?
            .rows
            .into_iter()
            .map(|row| {
                let input = row
                    .input
                    .as_ref()
                    .context("Atari++ joystick control is not calibrated")?;
                Ok((row.target_id, self::input(raw, input)?))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let get = |name: &str| {
            mapped
                .get(name)
                .copied()
                .with_context(|| format!("Atari++ control {name} is absent"))
        };
        let button = |name: &str| match get(name)? {
            DigitalInput::Button(index) if index < 16 => Ok(u8::try_from(index)? + 1),
            _ => anyhow::bail!("Atari++ {name} must be a joydev button 0..15"),
        };
        let buttons = [
            button("fire")?,
            button("fire2")?,
            button("fire3")?,
            button("fire4")?,
        ];
        ensure!(
            BTreeSet::from(buttons).len() == buttons.len(),
            "Atari++ fire controls resolve to duplicate buttons"
        );
        Ok(PortOverlay {
            port: player.player - 1,
            unit: player.player - 1,
            sensitivity: 8192,
            horizontal_axis: axis_pair(get("left")?, get("right")?, "horizontal")?,
            vertical_axis: axis_pair(get("up")?, get("down")?, "vertical")?,
            buttons,
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) private_config: PathBuf,
        pub(crate) selected_paths: Vec<PathBuf>,
        topology: InputTopology,
        raw_maps: Vec<RawMap>,
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
            let mut selected_paths = Vec::new();
            for player in &setup.players {
                let matches = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    matches.len() == 1 && !matches[0].is_virtual,
                    "Atari++ physical controller is missing or ambiguous"
                );
                selected_paths.push(matches[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected_paths)?;
            let mut raw_maps = Vec::new();
            let mut overlays = Vec::new();
            for (player, path) in setup.players.iter().zip(&selected_paths) {
                let raw = lunchbox_controller_probe::linux_classic::read_raw(path)?;
                overlays.push(overlay(
                    player,
                    &raw,
                    calibrations
                        .get(&player.controller_id)
                        .context("Atari++ calibration disappeared")?,
                )?);
                raw_maps.push(raw);
            }
            topology.verify()?;
            let baseline = std::fs::read(&setup.config_path)
                .context("Reading the declared Atari++ configuration")?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-atari-plus-plus-native-")
                .tempdir()?;
            let private_config = directory.path().join("atari++.conf");
            std::fs::write(&private_config, patch_config(&baseline, &overlays)?)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.config_path,
                &setup.bubblewrap_program,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let session = Self {
                directory,
                private_config,
                selected_paths,
                topology,
                raw_maps,
                hashes,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir() && self.private_config.is_file(),
                "Atari++ private configuration disappeared"
            );
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "Atari++ launch input changed"
                );
            }
            for (path, expected) in self.selected_paths.iter().zip(&self.raw_maps) {
                ensure!(
                    lunchbox_controller_probe::linux_classic::read_raw(path)? == *expected,
                    "Atari++ joydev numbering or correction changed"
                );
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
                "Atari++ executable differs from the saved trusted runtime"
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
                "Atari++ launch plan changed after preparation"
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
            anyhow::bail!("Atari++ calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Atari++")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "Atari++ identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Atari++ executable differs from selection"
        );
        ensure!(
            original
                .arguments
                .iter()
                .filter(|arg| *arg == setup.content.as_os_str())
                .count()
                == 1
                && !original
                    .arguments
                    .iter()
                    .any(|arg| arg == "-config" || arg == "--config"),
            "Atari++ calibrated launch needs the saved content exactly once and no competing config option"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Atari++ executable differs from the saved trusted runtime"
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
        ];
        for (index, source) in inputs.selected_paths.iter().enumerate() {
            let unit = u8::try_from(index)?;
            arguments.extend([
                "--dev-bind".into(),
                source.as_os_str().to_owned(),
                format!("/dev/input/js{unit}").into(),
                "--dev-bind".into(),
                source.as_os_str().to_owned(),
                format!("/dev/js{unit}").into(),
            ]);
        }
        arguments.extend([
            "--chdir".into(),
            cwd.into_os_string(),
            "--".into(),
            executable.as_os_str().to_owned(),
            "-config".into(),
            setup.config_path.as_os_str().to_owned(),
        ]);
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
    fn emits_linux_analog_contract_without_sdl_substitution() {
        let overlay = PortOverlay {
            port: 0,
            unit: 1,
            sensitivity: 8192,
            horizontal_axis: 0,
            vertical_axis: 1,
            buttons: [1, 2, 3, 4],
        };
        let text = patch_config(
            b"Machine = XL\nJoystick.0.Port = KeypadStick.0\n",
            &[overlay],
        )
        .unwrap();
        assert!(text.contains("Machine = XL"));
        assert!(text.contains("Joystick.0.Port = AnalogJoystick.1"));
        assert!(text.contains("HAxis.1 = XAxis.1"));
        assert!(!text.contains("KeypadStick.0"));
    }
}
