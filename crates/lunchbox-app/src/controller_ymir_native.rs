//! Ymir standalone-native Control Pad profile writer.
//!
//! Pinned source: ymir-emu/Ymir commit
//! `8a8e8402dff5abcfaa3794ba0ae756c416650f52`. Port 1 Control Pad binds live
//! in `Ymir.toml` under `[Input.Port1.ControlPad]` as single-element arrays
//! of `{GamepadName}@{id}` strings (`input_events.cpp`), where `id` is the
//! SDL gamepad index. `gamecontrollerdb.txt` in the profile root is layered
//! over the bundled database (`rom_service.cpp`). `-p` selects the profile
//! directory and `-d` the disc image (`main.cpp`).

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "8a8e8402dff5abcfaa3794ba0ae756c416650f52";
pub(crate) const PROFILE_ID: &str = "ymir:standalone-saturn-pad";
pub(crate) const TOML_FILE: &str = "Ymir.toml";
pub(crate) const DB_FILE: &str = "gamecontrollerdb.txt";

/// Saturn Control Pad actions in TOML key order with the layout target that
/// feeds each one. The DPad 2D action is derived from dpad presence.
pub(crate) const ACTIONS: [(&str, &str); 13] = [
    ("A", "a"),
    ("B", "b"),
    ("C", "c"),
    ("X", "x"),
    ("Y", "y"),
    ("Z", "z"),
    ("L", "l"),
    ("R", "r"),
    ("Start", "start"),
    ("Up", "up"),
    ("Down", "down"),
    ("Left", "left"),
    ("Right", "right"),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 13] = [
    ("a", "A"),
    ("b", "B"),
    ("c", "C"),
    ("x", "X"),
    ("y", "Y"),
    ("z", "Z"),
    ("l", "L"),
    ("r", "R"),
    ("start", "Start"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
];

/// SDL3 gamepad button index to Ymir element name.
pub(crate) fn button_element(gamepad: u32) -> Result<&'static str> {
    Ok(match gamepad {
        0 => "GamepadA",
        1 => "GamepadB",
        2 => "GamepadX",
        3 => "GamepadY",
        4 => "GamepadBack",
        5 => "GamepadGuide",
        6 => "GamepadStart",
        7 => "GamepadLeftThumb",
        8 => "GamepadRightThumb",
        9 => "GamepadLeftBumper",
        10 => "GamepadRightBumper",
        11 => "GamepadDpadUp",
        12 => "GamepadDpadDown",
        13 => "GamepadDpadLeft",
        14 => "GamepadDpadRight",
        15 => "GamepadMisc1",
        16 => "GamepadRightPaddle1",
        17 => "GamepadLeftPaddle1",
        18 => "GamepadRightPaddle2",
        19 => "GamepadLeftPaddle2",
        20 => "GamepadTouchPad",
        21 => "GamepadMisc2",
        22 => "GamepadMisc3",
        23 => "GamepadMisc4",
        24 => "GamepadMisc5",
        25 => "GamepadMisc6",
        _ => anyhow::bail!("Ymir gamepad button {gamepad} has no element name"),
    })
}

/// SDL3 gamepad axis index to 1D trigger element name. Sticks have no
/// Control Pad action; the dpad covers directions.
pub(crate) fn trigger_element(gamepad: u32) -> Result<&'static str> {
    Ok(match gamepad {
        4 => "GamepadLeftTrigger",
        5 => "GamepadRightTrigger",
        _ => anyhow::bail!("Ymir gamepad axis {gamepad} is not a trigger"),
    })
}

/// Patch only `[Input.Port1.ControlPad]` binds in a copied `Ymir.toml`,
/// replacing each action array with one `{name}@{id}` element. Port 1 must
/// already select the Control Pad; every other table, key, comment, and
/// formatting detail is preserved by `toml_edit`.
pub(crate) fn patch_toml(baseline: &[u8], elements: &BTreeMap<String, String>) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "Ymir configuration is too large"
    );
    let text = std::str::from_utf8(baseline).context("Ymir configuration is not UTF-8")?;
    ensure!(
        !text.contains('\0'),
        "Ymir configuration contains a NUL byte"
    );
    ensure!(
        elements.len() == ACTIONS.len(),
        "Ymir needs every Control Pad action element"
    );
    for (action, _) in ACTIONS {
        elements
            .get(action)
            .with_context(|| format!("Ymir action {action} is absent"))?;
    }
    let mut document: toml_edit::DocumentMut = text
        .parse()
        .context("Ymir configuration is not valid TOML")?;
    let port = document
        .get("Input")
        .and_then(|input| input.get("Port1"))
        .and_then(|port| port.get("ControlPad"))
        .context("Ymir baseline has no [Input.Port1.ControlPad] table")?;
    ensure!(
        port.is_table(),
        "Ymir [Input.Port1.ControlPad] is not a table"
    );
    if let Some(peripheral) = document
        .get("Input")
        .and_then(|input| input.get("Port1"))
        .and_then(|port| port.get("PeripheralType"))
        .and_then(|value| value.as_str())
    {
        ensure!(
            peripheral == "ControlPad",
            "Ymir port 1 uses {peripheral}, not the Control Pad"
        );
    }
    let table = &mut document["Input"]["Port1"]["ControlPad"];
    for (action, _) in ACTIONS {
        let element = &elements[action];
        ensure!(
            !element.is_empty()
                && element.len() <= 128
                && !element
                    .chars()
                    .any(|c| c.is_control() || c == '"' || c == '\n'),
            "Ymir element string is invalid"
        );
        let mut array = toml_edit::Array::new();
        array.push(element.as_str());
        table[action] = toml_edit::value(array);
    }
    Ok(document.to_string())
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
        /// The user's real Ymir profile directory holding `Ymir.toml`.
        pub profile_dir: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Ymir setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Ymir setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.profile_dir.is_absolute()
                    && !self
                        .profile_dir
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                "Ymir profile directory must be absolute without parent traversal"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Ymir setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Ymir native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Ymir supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Ymir player needs a saved controller identity"
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
                .context("Missing Ymir native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Ymir controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Ymir mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Ymir needs native calibration for every Saturn control"
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
                "detail": "Native Linux launch stages a session profile with a patched Ymir.toml, then rechecks the exact SDL3 routes. Only the single Control Pad is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Ymir saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Ymir emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elements() -> BTreeMap<String, String> {
        ACTIONS
            .iter()
            .map(|(action, _)| ((*action).to_owned(), "GamepadA@0".to_owned()))
            .collect()
    }

    #[test]
    fn patches_only_port1_control_pad_binds() {
        let baseline =
            "[General]\nfoo = 1\n[Input.Port1.ControlPad]\nA = [\"KeyJ\"]\nB = [\"KeyK\"]\n";
        let text = patch_toml(baseline.as_bytes(), &elements()).unwrap();
        assert!(text.contains("[General]"));
        assert!(text.contains("A = [\"GamepadA@0\"]"));
        assert!(text.contains("DPad") || !text.contains("DPad = [\"KeyJ\"]"));
        assert!(!text.contains("\"KeyJ\""));
    }

    #[test]
    fn refuses_foreign_peripheral_and_bad_tables() {
        let baseline = "[Input.Port1]\nPeripheralType = \"AnalogPad\"\n[Input.Port1.ControlPad]\n";
        assert!(patch_toml(baseline.as_bytes(), &elements()).is_err());
        assert!(patch_toml(b"[Input]\n", &elements()).is_err());
        let mut bad = elements();
        bad.remove("A");
        assert!(patch_toml(b"[Input.Port1.ControlPad]\n", &bad).is_err());
    }

    #[test]
    fn button_names_cover_sdl3_gamepad() {
        assert_eq!(button_element(0).unwrap(), "GamepadA");
        assert_eq!(button_element(14).unwrap(), "GamepadDpadRight");
        assert_eq!(button_element(20).unwrap(), "GamepadTouchPad");
        assert!(button_element(26).is_err());
        assert_eq!(trigger_element(4).unwrap(), "GamepadLeftTrigger");
        assert!(trigger_element(0).is_err());
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
        Snapshot,
        bindings::{Input as ResolvedInput, Output as ResolvedOutput},
        duckstation::DigitalInput,
        file_hash,
        linux_classic::AxisEndpoints,
    };
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
        os::unix::fs::symlink,
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
            serde_json::from_slice(&output).context("Invalid Ymir SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "Ymir helper inspected a different SDL runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        let object = value
            .as_object_mut()
            .context("Invalid Ymir snapshot shape")?;
        object.remove("warnings");
        for device in object
            .get_mut("devices")
            .and_then(serde_json::Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            let object = device
                .as_object_mut()
                .context("Invalid Ymir device shape")?;
            object.remove("resolved");
            object.remove("linux_classic");
        }
        Ok(value)
    }

    /// Mirror one profile entry as a symlink, recursing into directories.
    fn mirror_entry(source: &PathBuf, link: &PathBuf) -> Result<()> {
        let kind = fs::symlink_metadata(source)?.file_type();
        if kind.is_symlink() {
            symlink(fs::read_link(source)?, link)?;
        } else if kind.is_dir() {
            fs::create_dir(link)?;
            for entry in fs::read_dir(source)? {
                let entry = entry?;
                mirror_entry(&entry.path(), &link.join(entry.file_name()))?;
            }
        } else if kind.is_file() {
            symlink(source, link)?;
        } else {
            anyhow::bail!("Ymir profile entry is not a file, directory, or symlink");
        }
        Ok(())
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
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
                fs::symlink_metadata(&setup.content)?.file_type().is_file()
                    && setup.content.canonicalize()? == setup.content,
                "Ymir content must be a direct regular file with canonical ancestry"
            );
            ensure!(
                setup.profile_dir.is_dir()
                    && setup.profile_dir.canonicalize()? == setup.profile_dir,
                "Ymir profile directory must be a canonical directory"
            );
            let baseline_path = setup.profile_dir.join(TOML_FILE);
            ensure!(
                fs::symlink_metadata(&baseline_path)?.file_type().is_file(),
                "Ymir profile has no Ymir.toml baseline"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Ymir physical controller is missing or ambiguous"
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
                "Ymir SDL inventory moved during preparation"
            );
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            ensure!(device.is_gamepad, "Ymir needs an SDL-recognized gamepad");
            let pad = device
                .gamepad_index
                .context("Ymir SDL gamepad index is absent; unplug non-gamepad devices")?;
            let resolved = device
                .resolved
                .as_ref()
                .context("Ymir SDL resolved bindings are absent")?;
            let classic = device
                .linux_classic
                .as_ref()
                .context("Ymir classic Linux control map is absent")?;
            classic.validate_counts(resolved)?;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Ymir calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Ymir native profile")?;
            // Element per Saturn action; the DPad 2D action comes from dpad
            // presence. Triggers accept trigger axes; sticks have no Control
            // Pad action and are refused with a redirect to the dpad.
            let mut elements: BTreeMap<String, String> = BTreeMap::new();
            // The DPad 2D action needs a real dpad in the resolved mapping,
            // independent of how directions were calibrated.
            ensure!(
                resolved.bindings.iter().any(|binding| matches!(
                    &binding.output,
                    ResolvedOutput::Button {
                        index: 11 | 12 | 13 | 14
                    }
                )),
                "Ymir pad exposes no dpad for the DPad action"
            );
            for row in calibration.plan_profile(profile)?.rows {
                let (_, action) = ACTIONS
                    .iter()
                    .find(|(_, target)| *target == row.target_id)
                    .context("Ymir target is outside the Saturn profile")?;
                let input = row
                    .input
                    .as_ref()
                    .context("Ymir Saturn control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("Ymir requires measured native controls")?;
                let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                });
                let translated = classic.digital_input(native.code, measured)?;
                // Release posture is proven by the saved calibration
                // endpoints; the SDL3 snapshot carries no live sample.
                let element = match translated {
                    DigitalInput::Button(index) => {
                        format!("{}@{pad}", invert_button(resolved, index)?)
                    }
                    DigitalInput::Hat { index, direction } => {
                        let name = invert_hat(resolved, index, direction)?;
                        format!("{name}@{pad}")
                    }
                    DigitalInput::Axis { index, .. } => {
                        let axis = invert_axis(resolved, index)?;
                        if !matches!(axis, 4 | 5) {
                            anyhow::bail!(
                                "Ymir Saturn buttons need raw buttons or hats; stick axes have no Control Pad action"
                            );
                        }
                        // L/R ride the trigger axes; anything else on a
                        // trigger is a user rebind, still a valid element.
                        format!("{}@{pad}", trigger_element(axis)?)
                    }
                };
                ensure!(
                    elements.insert((*action).to_owned(), element).is_none(),
                    "Ymir Saturn action appears twice"
                );
            }
            elements.insert("DPad".to_owned(), format!("GamepadDPad@{pad}"));
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-ymir-")
                .tempdir()?;
            // Mirror the profile tree through symlinks so saves survive,
            // but keep Ymir.toml and the controller database private.
            for entry in fs::read_dir(&setup.profile_dir)? {
                let entry = entry?;
                if entry.file_name() == TOML_FILE || entry.file_name() == DB_FILE {
                    continue;
                }
                mirror_entry(&entry.path(), &directory.path().join(entry.file_name()))?;
            }
            let baseline = fs::read(&baseline_path)?;
            fs::write(
                directory.path().join(TOML_FILE),
                patch_toml(&baseline, &elements)?,
            )?;
            if let Some(mapping) = device.mapping.as_deref() {
                fs::write(directory.path().join(DB_FILE), format!("{mapping}\n"))?;
            }
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &baseline_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            hashes.insert(
                directory.path().join(TOML_FILE),
                file_hash(&directory.path().join(TOML_FILE))?,
            );
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

        pub(crate) fn directory(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Ymir launch input changed");
            }
            let fresh = comparable(&observe(&self.setup, &[], cancel)?)?;
            ensure!(
                fresh == self.initial,
                "Ymir SDL inventory moved before launch"
            );
            let captured = observe(&self.setup, &[self.physical_path.clone()], cancel)?;
            ensure!(
                comparable(&captured)? == self.initial,
                "Ymir SDL inventory moved before launch"
            );
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()
        }
    }

    /// Raw SDL button index to Ymir element name through the resolved mapping.
    fn invert_button(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw: u32,
    ) -> Result<&'static str> {
        for binding in &resolved.bindings {
            if let (ResolvedInput::Button { index }, ResolvedOutput::Button { index: gamepad }) =
                (&binding.input, &binding.output)
            {
                if *index == raw {
                    return button_element(*gamepad);
                }
            }
        }
        anyhow::bail!("Ymir raw button {raw} has no gamepad mapping")
    }

    /// Raw SDL hat and pressed mask to dpad element name.
    fn invert_hat(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw_hat: u32,
        direction: u8,
    ) -> Result<&'static str> {
        for binding in &resolved.bindings {
            if let (ResolvedInput::Hat { index, mask }, ResolvedOutput::Button { index: gamepad }) =
                (&binding.input, &binding.output)
            {
                if *index == raw_hat && *mask == direction && matches!(gamepad, 11..=14) {
                    return button_element(*gamepad);
                }
            }
        }
        anyhow::bail!("Ymir raw hat {raw_hat} has no dpad mapping")
    }

    /// Raw SDL axis index to gamepad axis index.
    fn invert_axis(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw: u32,
    ) -> Result<u32> {
        for binding in &resolved.bindings {
            if let (
                ResolvedInput::Axis { index, .. },
                ResolvedOutput::Axis { index: gamepad, .. },
            ) = (&binding.input, &binding.output)
            {
                if *index == raw {
                    ensure!(*gamepad <= 5, "Ymir gamepad axis is out of range");
                    return Ok(*gamepad);
                }
            }
        }
        anyhow::bail!("Ymir raw axis {raw} has no stick mapping")
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
                "Ymir executable differs from the saved trusted runtime"
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
                "Ymir launch plan changed after preparation"
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
                    "Ymir exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Ymir did not open the selected SDL controller before timeout"
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
            anyhow::bail!("Ymir calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Ymir")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Ymir identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Ymir launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Ymir calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Ymir executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // -p selects the session profile; the disc keeps its default slot.
        plan.arguments = vec![
            std::ffi::OsString::from("-p"),
            inputs.directory().as_os_str().to_owned(),
            std::ffi::OsString::from("-d"),
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
