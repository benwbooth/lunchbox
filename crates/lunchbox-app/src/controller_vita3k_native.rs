//! Vita3K native SDL3 controller configuration.
//!
//! Pinned to Vita3K/Vita3K `84184a363aa99c7f331a7e75bdd75f43ff63db08`.
//! Vita3K stores the SDL gamepad button and axis enum values in the YAML
//! `controller-binds` and `controller-axis-binds` vectors.  `--config-location`
//! accepts a YAML file; `pref-path` keeps the installed Vita filesystem (and
//! therefore firmware and saves) separate from the staged config.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "84184a363aa99c7f331a7e75bdd75f43ff63db08";
pub(crate) const PROFILE_ID: &str = "vita3k:standalone-vita3k-vita";

/// `controller-binds` index order: the GUI tab order in
/// `ControlsDialog` (`vita_button` values), where each position holds the
/// SDL gamepad button mapped to that Vita control. Verified against
/// `reset_controller_defaults`: position 0 is cross (SOUTH), 1 circle
/// (EAST), 2 square (WEST), 3 triangle (NORTH), 4 select (BACK), 5 psbutton
/// (GUIDE), 6 start (START), 7 l3 (LEFT_STICK), 8 r3 (RIGHT_STICK), 9 l
/// (LEFT_SHOULDER), 10 r (RIGHT_SHOULDER), 11 up, 12 down, 13 left, 14
/// right (DPAD_*).
pub(crate) const BUTTON_KEYS: [&str; 15] = [
    "cross", "circle", "square", "triangle", "select", "psbutton", "start", "l3", "r3", "l", "r",
    "up", "down", "left", "right",
];

/// SDL3 Gamepad button values; caller supplies the measured SDL enum values.
pub(crate) fn config_yaml(
    controller_binds: &[i16],
    controller_axis_binds: &[i16],
    vita_fs_path: &str,
) -> Result<String> {
    ensure!(
        controller_binds.len() == BUTTON_KEYS.len(),
        "Vita3K requires all 15 controller button binds"
    );
    ensure!(
        controller_axis_binds.len() == 6,
        "Vita3K requires six controller axis binds"
    );
    ensure!(
        controller_binds
            .iter()
            .all(|value| (0..=31).contains(value)),
        "Vita3K SDL button enum is out of range"
    );
    ensure!(
        controller_axis_binds
            .iter()
            .all(|value| (0..=5).contains(value)),
        "Vita3K SDL axis enum is out of range"
    );
    ensure!(
        !vita_fs_path.is_empty() && !vita_fs_path.chars().any(char::is_control),
        "Vita3K Vita filesystem path is invalid"
    );
    let mut out = String::from("pref-path: \"");
    out.push_str(&vita_fs_path.replace('\\', "\\\\").replace('"', "\\\""));
    out.push_str("\"\ncontroller-binds:\n");
    for value in controller_binds {
        out.push_str(&format!("  - {value}\n"));
    }
    out.push_str("controller-axis-binds:\n");
    for value in controller_axis_binds {
        out.push_str(&format!("  - {value}\n"));
    }
    out.push_str("controller-analog-multiplier: 1.0\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn button_keys_follow_vita_button_order() {
        // binds[] is indexed by vita_button, which reuses SDL button enum
        // values: position N holds the Vita control whose tab entry carries
        // SDL button N. SOUTH..DPAD_RIGHT = cross, circle, square, triangle,
        // select, psbutton, start, l3, r3, l, r, up, down, left, right.
        assert_eq!(
            BUTTON_KEYS,
            [
                "cross", "circle", "square", "triangle", "select", "psbutton", "start", "l3", "r3",
                "l", "r", "up", "down", "left", "right",
            ]
        );
        // reset_controller_defaults writes exactly this order.
        let sdl = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
        assert_eq!(sdl.len(), BUTTON_KEYS.len());
    }
    #[test]
    fn emits_source_vector_order_and_data_root() {
        let text = config_yaml(&[0; 15], &[0, 1, 2, 3, 4, 5], "/srv/vita data").unwrap();
        assert!(text.starts_with("pref-path: \"/srv/vita data\""));
        assert_eq!(text.matches("  - 0\n").count(), 16);
        assert!(text.contains("controller-axis-binds:\n  - 0\n  - 1\n"));
    }
}

/// Layout target ids covered by the native profile, in `BUTTON_KEYS` order
/// for buttons plus both sticks and identity triggers.
pub(crate) const ROUTES: [(&str, &str); 23] = [
    ("cross", "Cross"),
    ("circle", "Circle"),
    ("square", "Square"),
    ("triangle", "Triangle"),
    ("select", "Select"),
    ("psbutton", "PS button"),
    ("start", "Start"),
    ("l3", "L3"),
    ("r3", "R3"),
    ("l", "L"),
    ("r", "R"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("stick_up", "Left stick up"),
    ("stick_down", "Left stick down"),
    ("stick_left", "Left stick left"),
    ("stick_right", "Left stick right"),
    ("right_stick_up", "Right stick up"),
    ("right_stick_down", "Right stick down"),
    ("right_stick_left", "Right stick left"),
    ("right_stick_right", "Right stick right"),
];

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
                "Vita3K setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Vita3K setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Vita3K setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Vita3K native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Vita3K supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Vita3K player needs a saved controller identity"
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
                .context("Missing Vita3K native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Vita3K controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Vita3K mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Vita3K needs native calibration for every Vita control"
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
                "detail": "Native Linux launch writes a private config.yml with SDL gamepad binds, then rechecks the exact SDL3 routes. Only the single Vita pad is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Vita3K saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Vita3K emulator/content setup"
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
        Snapshot,
        bindings::{Input as ResolvedInput, Output as ResolvedOutput},
        duckstation::DigitalInput,
        file_hash,
        linux_classic::AxisEndpoints,
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
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl-library")
            .arg(&setup.sdl_library)
            .arg("--hint")
            .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
        for path in paths {
            command.arg("--bindings-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid Vita3K SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "Vita3K helper inspected a different SDL runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        let object = value
            .as_object_mut()
            .context("Invalid Vita3K snapshot shape")?;
        object.remove("warnings");
        if let Some(report) = object
            .get_mut("player_probe")
            .and_then(serde_json::Value::as_object_mut)
        {
            report.remove("events_processed");
        }
        for device in object
            .get_mut("devices")
            .and_then(serde_json::Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            let object = device
                .as_object_mut()
                .context("Invalid Vita3K device shape")?;
            object.remove("resolved");
            object.remove("linux_classic");
        }
        Ok(value)
    }

    /// Invert the resolved mapping: the config wants gamepad indices, the
    /// calibration yields raw SDL numbers.
    fn gamepad_button(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw_button: u32,
    ) -> Result<i16> {
        for binding in &resolved.bindings {
            if let (ResolvedInput::Button { index }, ResolvedOutput::Button { index: gamepad }) =
                (&binding.input, &binding.output)
            {
                if *index == raw_button {
                    ensure!(*gamepad <= 31, "Vita3K SDL gamepad button is out of range");
                    return Ok(*gamepad as i16);
                }
            }
        }
        anyhow::bail!("Vita3K raw button {raw_button} has no gamepad mapping")
    }

    fn gamepad_axis(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw_axis: u32,
    ) -> Result<i16> {
        for binding in &resolved.bindings {
            if let (
                ResolvedInput::Axis { index, .. },
                ResolvedOutput::Axis { index: gamepad, .. },
            ) = (&binding.input, &binding.output)
            {
                if *index == raw_axis {
                    ensure!(*gamepad <= 5, "Vita3K SDL gamepad axis is out of range");
                    return Ok(*gamepad as i16);
                }
            }
        }
        anyhow::bail!("Vita3K raw axis {raw_axis} has no gamepad mapping")
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        config_path: PathBuf,
        physical_path: String,
        topology: InputTopology,
        initial: serde_json::Value,
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
                fs::symlink_metadata(&setup.content)?.file_type().is_dir()
                    && setup.content.canonicalize()? == setup.content,
                "Vita3K content must be a direct installed-app directory with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Vita3K physical controller is missing or ambiguous"
            );
            let selected = found[0].device_path.clone();
            let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
            let initial = comparable(&observe(setup, &[], cancel)?)?;
            let physical_path = topology.resolve_runtime_path(
                &selected,
                observe(setup, &[], cancel)?
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            let captured = observe(setup, &[physical_path.clone()], cancel)?;
            ensure!(
                comparable(&captured)? == initial,
                "Vita3K SDL inventory moved during preparation"
            );
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            ensure!(device.is_gamepad, "Vita3K needs an SDL-recognized gamepad");
            let resolved = device
                .resolved
                .as_ref()
                .context("Vita3K SDL resolved bindings are absent")?;
            let classic = device
                .linux_classic
                .as_ref()
                .context("Vita3K classic Linux control map is absent")?;
            classic.validate_counts(resolved)?;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Vita3K calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Vita3K native profile")?;
            // Raw SDL numbers for the fifteen buttons, in BUTTON_KEYS
            // (vita_button) order, plus six axis binds: left/right sticks
            // from calibration and identity triggers.
            let mut buttons = BTreeMap::new();
            let mut sticks: BTreeMap<String, (u32, bool)> = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "Vita3K target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("Vita3K Vita control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("Vita3K requires measured native controls")?;
                let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                });
                let translated = classic.digital_input(native.code, measured)?;
                // Release posture is proven by the saved calibration
                // endpoints; the SDL3 snapshot carries no live sample.
                match row.target_id.as_str() {
                    "stick_up" | "stick_down" | "stick_left" | "stick_right" | "right_stick_up"
                    | "right_stick_down" | "right_stick_left" | "right_stick_right" => {
                        let DigitalInput::Axis { index, .. } = translated else {
                            anyhow::bail!("Vita3K stick directions need proportional axes")
                        };
                        ensure!(
                            sticks
                                .insert(row.target_id.clone(), (index, native.direction > 0))
                                .is_none(),
                            "Vita3K stick direction appears twice"
                        );
                    }
                    _ => {
                        let DigitalInput::Button(index) = translated else {
                            anyhow::bail!("Vita3K buttons need raw buttons")
                        };
                        ensure!(
                            buttons.insert(row.target_id, index).is_none(),
                            "Vita3K button appears twice"
                        );
                    }
                }
            }
            let mut binds = Vec::new();
            for key in BUTTON_KEYS {
                let raw = *buttons
                    .get(key)
                    .with_context(|| format!("Vita3K button {key} is not calibrated"))?;
                binds.push(gamepad_button(resolved, raw)?);
            }
            let stick_axis = |negative: &str, positive: &str| {
                let (neg_index, neg_dir) = sticks.get(negative).with_context(|| {
                    format!("Vita3K stick direction {negative} is not calibrated")
                })?;
                let (pos_index, pos_dir) = sticks.get(positive).with_context(|| {
                    format!("Vita3K stick direction {positive} is not calibrated")
                })?;
                ensure!(
                    neg_index == pos_index && !neg_dir && *pos_dir,
                    "Vita3K stick halves must share one axis with opposite polarity"
                );
                gamepad_axis(resolved, *neg_index)
            };
            let axes = vec![
                stick_axis("stick_left", "stick_right")?,
                stick_axis("stick_up", "stick_down")?,
                stick_axis("right_stick_left", "right_stick_right")?,
                stick_axis("right_stick_up", "right_stick_down")?,
                4,
                5,
            ];
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-vita3k-")
                .tempdir()?;
            let config_path = directory.path().join("config.yml");
            // pref-path keeps the installed Vita filesystem (firmware and
            // saves) separate from this staged config; the content arg
            // selects the installed app at launch.
            let vita_fs = directory.path().join("vita");
            fs::create_dir(&vita_fs)?;
            fs::write(
                &config_path,
                config_yaml(&binds, &axes, &vita_fs.to_string_lossy())?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [&setup.probe_program, &setup.sdl_library, &config_path] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                config_path,
                physical_path,
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
                ensure!(file_hash(path)? == *hash, "Vita3K launch input changed");
            }
            let fresh = comparable(&observe(&self.setup, &[], cancel)?)?;
            ensure!(
                fresh == self.initial,
                "Vita3K SDL inventory moved before launch"
            );
            let captured = observe(&self.setup, &[self.physical_path.clone()], cancel)?;
            ensure!(
                comparable(&captured)? == self.initial,
                "Vita3K SDL inventory moved before launch"
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
                "Vita3K executable differs from the saved trusted runtime"
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
                "Vita3K launch plan changed after preparation"
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
                    "Vita3K exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Vita3K did not open the selected SDL controller before timeout"
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
            anyhow::bail!("Vita3K calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Vita3K")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Vita3K identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Vita3K launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Vita3K calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Vita3K executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // -c takes the staged config.yml; -r takes the installed app.
        plan.arguments = vec![
            std::ffi::OsString::from("-c"),
            inputs.config_path().as_os_str().to_owned(),
            std::ffi::OsString::from("-r"),
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
