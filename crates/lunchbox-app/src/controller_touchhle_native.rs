//! touchHLE standalone-native simulated-touch option writer.
//!
//! Pinned source: touchHLE/touchHLE commit
//! `9052ea399c63e733be41262309ace2ff6dcc7f94`. Game-controller input is
//! fixed SDL2 game-controller buttons and sticks (`window.rs`); the staged
//! surface is which touch points they drive: `--button-to-touch=` (10
//! buttons: DPad directions, Start, A/B/X/Y, LeftShoulder),
//! `--dpad-to-touch=` and `--stick-to-touch=` regions. CLI options override
//! the options files, so launch passes them directly: no options-file patch
//! is needed. Content is the app bundle path (positional); the sandbox
//! stays in the real user data because launch never changes the working
//! directory or HOME.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "9052ea399c63e733be41262309ace2ff6dcc7f94";
pub(crate) const PROFILE_ID: &str = "touchhle:standalone-touch-buttons";

/// touchHLE `Button` names accepted by `--button-to-touch=` with the layout
/// target feeding each one.
pub(crate) const BUTTONS: [(&str, &str); 10] = [
    ("DPadLeft", "left"),
    ("DPadUp", "up"),
    ("DPadRight", "right"),
    ("DPadDown", "down"),
    ("Start", "start"),
    ("A", "a"),
    ("B", "b"),
    ("X", "x"),
    ("Y", "y"),
    ("LeftShoulder", "l"),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 10] = [
    ("left", "D-Pad Left"),
    ("up", "D-Pad Up"),
    ("right", "D-Pad Right"),
    ("down", "D-Pad Down"),
    ("start", "Start"),
    ("a", "A"),
    ("b", "B"),
    ("x", "X"),
    ("y", "Y"),
    ("l", "Left Shoulder"),
];

/// Render one `--button-to-touch=` CLI argument. Coordinates are whole
/// touchHLE device points (sub-pixel fractions are meaningless at 320x480);
/// portrait games use the 320x480 space, landscape games 480x320.
pub(crate) fn button_arg(button: &str, x: u32, y: u32) -> Result<String> {
    ensure!(
        BUTTONS.iter().any(|(name, _)| *name == button),
        "touchHLE button {button} is outside --button-to-touch="
    );
    ensure!(
        x <= 480 && y <= 480,
        "touchHLE touch point is outside the device space"
    );
    Ok(format!("--button-to-touch={button},{x},{y}"))
}

/// Render one `--dpad-to-touch=` region argument.
pub(crate) fn dpad_arg(x: u32, y: u32, w: u32, h: u32) -> Result<String> {
    ensure!(
        w > 0 && h > 0 && x + w <= 480 && y + h <= 480,
        "touchHLE dpad region is outside the device space"
    );
    Ok(format!("--dpad-to-touch={x},{y},{w},{h}"))
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

    /// One staged touch point: touchHLE button plus whole device points.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct TouchPoint {
        pub button: String,
        pub x: u32,
        pub y: u32,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        /// Per-button touch points, one per profile button.
        pub touch_points: Vec<TouchPoint>,
        /// Optional dpad region `[x, y, w, h]`, overriding dpad buttons.
        pub dpad_region: Option<[u32; 4]>,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "touchHLE setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "touchHLE setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "touchHLE setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing touchHLE native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "touchHLE supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "touchHLE player needs a saved controller identity"
            );
            ensure!(
                self.touch_points.len() == BUTTONS.len(),
                "touchHLE setup needs every button touch point"
            );
            let mut seen = BTreeSet::new();
            for point in &self.touch_points {
                ensure!(
                    BUTTONS.iter().any(|(name, _)| *name == point.button),
                    "touchHLE touch point button is unknown"
                );
                ensure!(
                    seen.insert(point.button.clone()),
                    "touchHLE touch point button appears twice"
                );
                button_arg(&point.button, point.x, point.y)?;
            }
            if let Some(region) = &self.dpad_region {
                dpad_arg(region[0], region[1], region[2], region[3])?;
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
                .context("Missing touchHLE native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("touchHLE controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "touchHLE mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "touchHLE needs native calibration for every touch control"
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
                "detail": "Native Linux launch passes --button-to-touch options for the fixed SDL2 buttons, then rechecks the exact SDL2 routes. Only the single gamepad with staged touch points is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many touchHLE saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate touchHLE emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_touch_option_grammar() {
        assert_eq!(
            button_arg("A", 470, 310).unwrap(),
            "--button-to-touch=A,470,310"
        );
        assert_eq!(
            dpad_arg(10, 10, 50, 50).unwrap(),
            "--dpad-to-touch=10,10,50,50"
        );
        assert!(button_arg("Guide", 1, 1).is_err());
        assert!(button_arg("A", 481, 1).is_err());
        assert!(dpad_arg(0, 0, 0, 10).is_err());
    }
}

/// SDL2 game-controller button each touchHLE `Button` reads (`window.rs`).
pub(crate) const BUTTON_SDL: [(&str, &str); 10] = [
    ("DPadLeft", "dpleft"),
    ("DPadUp", "dpup"),
    ("DPadRight", "dpright"),
    ("DPadDown", "dpdown"),
    ("Start", "start"),
    ("A", "a"),
    ("B", "b"),
    ("X", "x"),
    ("Y", "y"),
    ("LeftShoulder", "leftshoulder"),
];

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
            serde_json::from_slice(&output).context("Invalid touchHLE SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "touchHLE helper inspected a different SDL2 runtime"
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

    fn mapping_fields(mapping: &str) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::new();
        for entry in mapping.split(',').skip(2) {
            if let Some((key, value)) = entry.split_once(':') {
                fields.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }
        fields
    }

    /// Raw SDL2 button index must resolve to the exact game-controller
    /// button touchHLE reads for its `Button`; anything else would tap the
    /// wrong touch point or nothing at all.
    fn gamepad_output(
        fields: &BTreeMap<String, String>,
        translated: DigitalInput,
        expected: &str,
    ) -> Result<()> {
        const BUTTON_OUTPUTS: [&str; 15] = [
            "a",
            "b",
            "x",
            "y",
            "back",
            "guide",
            "start",
            "leftstick",
            "rightstick",
            "leftshoulder",
            "rightshoulder",
            "dpup",
            "dpdown",
            "dpleft",
            "dpright",
        ];
        let DigitalInput::Button(index) = translated else {
            anyhow::bail!("touchHLE buttons need raw buttons; axes and hats have no touch mapping")
        };
        let raw = u32::try_from(index).context("touchHLE button is too large")?;
        let mut found = None;
        for output in BUTTON_OUTPUTS {
            if let Some(input) = fields.get(output) {
                let candidate: u32 = input
                    .trim_end_matches('~')
                    .strip_prefix('b')
                    .context("touchHLE mapping entry is not a button")?
                    .parse()
                    .context("touchHLE mapping button is invalid")?;
                if candidate == raw {
                    found = Some(output);
                    break;
                }
            }
        }
        let output = found.context("touchHLE raw button is unmapped")?;
        ensure!(
            output == expected,
            "touchHLE control drives gamepad {output}, not the fixed {expected}"
        );
        Ok(())
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        physical_path: String,
        topology: InputTopology,
        initial: Snapshot,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
        arguments: Vec<std::ffi::OsString>,
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
                "touchHLE content must be a direct app bundle directory with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "touchHLE physical controller is missing or ambiguous"
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
            let fields = mapping_fields(
                device
                    .mapping
                    .as_deref()
                    .context("touchHLE SDL mapping is absent")?,
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("touchHLE calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing touchHLE native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("touchHLE SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("touchHLE SDL control counts are missing")?,
            )?;
            for row in calibration.plan_profile(profile)?.rows {
                let (button, _) = BUTTONS
                    .iter()
                    .find(|(_, target)| *target == row.target_id)
                    .context("touchHLE target is outside the touch profile")?;
                let (_, expected) = BUTTON_SDL
                    .iter()
                    .find(|(name, _)| name == button)
                    .context("touchHLE button has no SDL element")?;
                let input = row
                    .input
                    .as_ref()
                    .context("touchHLE touch control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("touchHLE requires measured native controls")?;
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
                    "Release the touchHLE controls before launch preparation"
                );
                gamepad_output(&fields, translated, expected)?;
            }
            let mut arguments = Vec::new();
            for point in &setup.touch_points {
                arguments.push(std::ffi::OsString::from(button_arg(
                    &point.button,
                    point.x,
                    point.y,
                )?));
            }
            if let Some(region) = &setup.dpad_region {
                arguments.push(std::ffi::OsString::from(dpad_arg(
                    region[0], region[1], region[2], region[3],
                )?));
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-touchhle-")
                .tempdir()?;
            // No files are staged: CLI options override the options files,
            // and the sandbox stays in the real user data. The directory
            // only anchors the session lifetime.
            let mut hashes = BTreeMap::new();
            for path in [&setup.content, &setup.probe_program, &setup.sdl_library] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                physical_path,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
                arguments,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn arguments(&self) -> &[std::ffi::OsString] {
            &self.arguments
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "touchHLE launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
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
                "touchHLE executable differs from the saved trusted runtime"
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
                "touchHLE launch plan changed after preparation"
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
                    "touchHLE exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "touchHLE did not open the selected SDL controller before timeout"
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
            anyhow::bail!("touchHLE calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("touchHLE")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "touchHLE identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "touchHLE launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "touchHLE calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "touchHLE executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // CLI options override the options files; the app bundle keeps its
        // default positional slot.
        plan.arguments = inputs
            .arguments()
            .to_vec()
            .into_iter()
            .chain(std::iter::once(setup.content.as_os_str().to_owned()))
            .collect();
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
