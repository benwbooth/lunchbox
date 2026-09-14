//! shadPS4 native Linux SDL3 input configuration.
//!
//! Pinned to shadps4-emu/shadPS4
//! `678705df8dead58799a3d9a9db38f8fb0c3dbefe`.  Input is not in TOML: the
//! source reads `<UserDir>/input_config/<GameId>.ini`, where each line is an
//! output name and a controller/axis token.  UserDir also contains savedata,
//! system modules, and other runtime data, so launch must isolate only a
//! copied input file while retaining that existing UserDir.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const SOURCE_COMMIT: &str = "678705df8dead58799a3d9a9db38f8fb0c3dbefe";
pub(crate) const PROFILE_ID: &str = "shadps4:standalone-dualshock";

/// Identity-mapped gameplay outputs from the source default profile
/// (`GetDefaultInputConfig`): face buttons, dpad, shoulders, stick clicks,
/// triggers, both sticks, and share/options. The touchpad has no gamepad
/// control in the shared layout and stays on the source default.
pub(crate) const CONTROLS: [(&str, &str); 20] = [
    ("cross", "cross"),
    ("circle", "circle"),
    ("square", "square"),
    ("triangle", "triangle"),
    ("pad_up", "pad_up"),
    ("pad_down", "pad_down"),
    ("pad_left", "pad_left"),
    ("pad_right", "pad_right"),
    ("l1", "l1"),
    ("r1", "r1"),
    ("l3", "l3"),
    ("r3", "r3"),
    ("back", "back"),
    ("options", "options"),
    ("axis_left_x", "axis_left_x"),
    ("axis_left_y", "axis_left_y"),
    ("axis_right_x", "axis_right_x"),
    ("axis_right_y", "axis_right_y"),
    ("l2", "l2"),
    ("r2", "r2"),
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(String),
    Axis(String),
}

fn safe_token(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 128,
        "shadPS4 binding token is invalid"
    );
    ensure!(
        !value
            .chars()
            .any(|c| c.is_control() || c == '=' || c == '#'),
        "shadPS4 binding token contains syntax characters"
    );
    Ok(())
}

/// Render the exact per-game INI syntax parsed by `Input::ParseConfig`.
/// `gamepad` is the source's one-based controller ID (the default is 1).
pub(crate) fn game_input_ini(gamepad: u8, mappings: &BTreeMap<String, Binding>) -> Result<String> {
    ensure!(
        (1..=4).contains(&gamepad),
        "shadPS4 gamepad ID is out of range"
    );
    ensure!(
        mappings.len() == CONTROLS.len(),
        "shadPS4 needs every declared gameplay mapping"
    );
    let mut out = String::new();
    let mut used = std::collections::BTreeSet::new();
    for (target, _) in CONTROLS {
        let binding = mappings
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("shadPS4 mapping {target} is absent"))?;
        let input = match binding {
            Binding::Button(value) | Binding::Axis(value) => value,
        };
        safe_token(input)?;
        ensure!(used.insert(input.clone()), "shadPS4 mapping is duplicated");
        out.push_str(&format!("{target}:{gamepad}={input}:{gamepad}\n"));
    }
    out.push_str(&format!("analog_deadzone:{gamepad}=leftjoystick,5,127\n"));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_gamepad_scoped_ini() {
        let mut map = BTreeMap::new();
        for (name, _) in CONTROLS {
            map.insert(name.to_owned(), Binding::Button(name.to_owned()));
        }
        let text = game_input_ini(2, &map).unwrap();
        assert!(text.contains("cross:2=cross:2"));
        assert!(text.contains("axis_left_x:2=axis_left_x:2"));
    }
}

/// Layout target ids covered by the native profile (dualshock geometry).
pub(crate) const ROUTES: [(&str, &str); 24] = [
    ("b", "Cross"),
    ("a", "Circle"),
    ("y", "Square"),
    ("x", "Triangle"),
    ("l", "L1"),
    ("r", "R1"),
    ("l2", "L2"),
    ("r2", "R2"),
    ("l3", "L3"),
    ("r3", "R3"),
    ("select", "Share"),
    ("start", "Options"),
    ("up", "Dpad up"),
    ("down", "Dpad down"),
    ("left", "Dpad left"),
    ("right", "Dpad right"),
    ("stick_up", "Left stick up"),
    ("stick_down", "Left stick down"),
    ("stick_left", "Left stick left"),
    ("stick_right", "Left stick right"),
    ("right_stick_up", "Right stick up"),
    ("right_stick_down", "Right stick down"),
    ("right_stick_left", "Right stick left"),
    ("right_stick_right", "Right stick right"),
];

/// Writer output each layout target feeds.
pub(crate) const TARGET_KEYS: [(&str, &str); 24] = [
    ("b", "cross"),
    ("a", "circle"),
    ("y", "square"),
    ("x", "triangle"),
    ("l", "l1"),
    ("r", "r1"),
    ("l2", "l2"),
    ("r2", "r2"),
    ("l3", "l3"),
    ("r3", "r3"),
    ("select", "back"),
    ("start", "options"),
    ("up", "pad_up"),
    ("down", "pad_down"),
    ("left", "pad_left"),
    ("right", "pad_right"),
    ("stick_up", "axis_left_y"),
    ("stick_down", "axis_left_y"),
    ("stick_left", "axis_left_x"),
    ("stick_right", "axis_left_x"),
    ("right_stick_up", "axis_right_y"),
    ("right_stick_down", "axis_right_y"),
    ("right_stick_left", "axis_right_x"),
    ("right_stick_right", "axis_right_x"),
];

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
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
        /// PS4 title ID selecting `input_config/<GameId>.ini` (CUSAxxxxx).
        pub game_id: String,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    pub(crate) fn valid_game_id(game_id: &str) -> bool {
        game_id.len() == 9
            && game_id.starts_with("CUSA")
            && game_id[4..].bytes().all(|b| b.is_ascii_digit())
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "shadPS4 setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "shadPS4 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                valid_game_id(&self.game_id),
                "shadPS4 setup needs a CUSA title ID"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "shadPS4 setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing shadPS4 native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "shadPS4 supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "shadPS4 player needs a saved controller identity"
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
                .context("Missing shadPS4 native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("shadPS4 controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "shadPS4 mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "shadPS4 needs native calibration for every DualShock control"
            );
            Ok(serde_json::json!({
                "profile_id": PROFILE_ID,
                "player": player.player,
                "controller_id": player.controller_id,
                "game_id": self.game_id,
                "source_layout": calibration.layout,
                "target_layout": profile.target_layout,
                "mapping": mapping,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch stages a session input_config pair, then rechecks the exact SDL3 routes. Only the single DualShock pad as gamepad 1 is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many shadPS4 saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate shadPS4 emulator/content setup"
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
            serde_json::from_slice(&output).context("Invalid shadPS4 SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "shadPS4 helper inspected a different SDL runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        let object = value
            .as_object_mut()
            .context("Invalid shadPS4 snapshot shape")?;
        object.remove("warnings");
        for device in object
            .get_mut("devices")
            .and_then(serde_json::Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            let object = device
                .as_object_mut()
                .context("Invalid shadPS4 device shape")?;
            object.remove("resolved");
            object.remove("linux_classic");
        }
        Ok(value)
    }

    /// SDL gamepad button index to shadPS4 input token.
    fn button_token(gamepad: u32) -> Result<&'static str> {
        Ok(match gamepad {
            0 => "cross",
            1 => "circle",
            2 => "square",
            3 => "triangle",
            4 => "back",
            6 => "options",
            7 => "l3",
            8 => "r3",
            9 => "l1",
            10 => "r1",
            11 => "pad_up",
            12 => "pad_down",
            13 => "pad_left",
            14 => "pad_right",
            _ => anyhow::bail!("shadPS4 gamepad button {gamepad} has no input token"),
        })
    }

    /// SDL gamepad axis index and polarity to shadPS4 input token.
    fn axis_token(gamepad: u32, positive: bool) -> Result<&'static str> {
        Ok(match (gamepad, positive) {
            (0, false) => "axis_left_x_minus",
            (0, true) => "axis_left_x_plus",
            (1, false) => "axis_left_y_minus",
            (1, true) => "axis_left_y_plus",
            (2, false) => "axis_right_x_minus",
            (2, true) => "axis_right_x_plus",
            (3, false) => "axis_right_y_minus",
            (3, true) => "axis_right_y_plus",
            (4, _) => "l2",
            (5, _) => "r2",
            _ => anyhow::bail!("shadPS4 gamepad axis {gamepad} has no input token"),
        })
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
                "shadPS4 content must be a direct regular file with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "shadPS4 physical controller is missing or ambiguous"
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
                "shadPS4 SDL inventory moved during preparation"
            );
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            ensure!(device.is_gamepad, "shadPS4 needs an SDL-recognized gamepad");
            // Runtime input IDs carry gamepad slot + 1; the pad must be the
            // first gamepad in SDL order so input/output ID 1 selects it.
            let slot = device
                .gamepad_index
                .context("shadPS4 SDL gamepad index is absent; unplug non-gamepad devices")?;
            ensure!(
                slot == 0,
                "shadPS4 pad must be the first SDL gamepad, found index {slot}"
            );
            let resolved = device
                .resolved
                .as_ref()
                .context("shadPS4 SDL resolved bindings are absent")?;
            let classic = device
                .linux_classic
                .as_ref()
                .context("shadPS4 classic Linux control map is absent")?;
            classic.validate_counts(resolved)?;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("shadPS4 calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing shadPS4 native profile")?;
            // Output tokens per writer output; axis outputs take full-axis
            // tokens once halves pair, or button tokens by user choice.
            let mut tokens: BTreeMap<String, String> = BTreeMap::new();
            let mut halves: BTreeMap<String, (u32, bool)> = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                let (_, output) = TARGET_KEYS
                    .iter()
                    .find(|(target, _)| *target == row.target_id)
                    .context("shadPS4 target is outside the DualShock profile")?;
                let input = row
                    .input
                    .as_ref()
                    .context("shadPS4 DualShock control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("shadPS4 requires measured native controls")?;
                let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                });
                let translated = classic.digital_input(native.code, measured)?;
                // Release posture is proven by the saved calibration
                // endpoints; the SDL3 snapshot carries no live sample.
                // Axis outputs need paired analog halves; only button
                // outputs accept buttons and hats.
                let is_axis_output =
                    output.starts_with("axis_") || *output == "l2" || *output == "r2";
                match translated {
                    DigitalInput::Button(index) => {
                        ensure!(
                            !is_axis_output,
                            "shadPS4 output {output} needs a proportional axis, not a button"
                        );
                        let token = invert_button(resolved, index)?;
                        ensure!(
                            tokens.insert((*output).to_owned(), token).is_none(),
                            "shadPS4 output appears twice"
                        );
                    }
                    DigitalInput::Hat { index, direction } => {
                        ensure!(
                            !is_axis_output,
                            "shadPS4 output {output} needs a proportional axis, not a hat"
                        );
                        let token = invert_hat(resolved, index, direction)?;
                        ensure!(
                            tokens.insert((*output).to_owned(), token).is_none(),
                            "shadPS4 output appears twice"
                        );
                    }
                    DigitalInput::Axis { index, .. } => {
                        ensure!(
                            halves
                                .insert(row.target_id.clone(), (index, native.direction > 0))
                                .is_none(),
                            "shadPS4 stick direction appears twice"
                        );
                    }
                }
            }
            // Paired halves collapse to full-axis tokens; lone halves and
            // trigger axes resolve to their own tokens.
            let mut full_axis = |negative: &str, positive: &str, full: &str| -> Result<()> {
                let (neg_index, neg_dir) = halves.remove(negative).with_context(|| {
                    format!("shadPS4 stick direction {negative} is not calibrated")
                })?;
                let (pos_index, pos_dir) = halves.remove(positive).with_context(|| {
                    format!("shadPS4 stick direction {positive} is not calibrated")
                })?;
                ensure!(
                    neg_index == pos_index && !neg_dir && pos_dir,
                    "shadPS4 stick halves must share one axis with opposite polarity"
                );
                let (_, output) = TARGET_KEYS
                    .iter()
                    .find(|(target, _)| *target == negative)
                    .context("shadPS4 target is outside the DualShock profile")?;
                ensure!(
                    tokens
                        .insert((*output).to_owned(), full.to_owned())
                        .is_none(),
                    "shadPS4 output appears twice"
                );
                Ok(())
            };
            full_axis("stick_left", "stick_right", "axis_left_x")?;
            full_axis("stick_up", "stick_down", "axis_left_y")?;
            full_axis("right_stick_left", "right_stick_right", "axis_right_x")?;
            full_axis("right_stick_up", "right_stick_down", "axis_right_y")?;
            for (target, (index, positive)) in &halves {
                let (_, output) = TARGET_KEYS
                    .iter()
                    .find(|(known, _)| known == target)
                    .context("shadPS4 target is outside the DualShock profile")?;
                // Lone trigger axes keep their own token; anything else is
                // an unpaired stick half.
                ensure!(
                    matches!(*output, "l2" | "r2"),
                    "shadPS4 stick direction {target} needs both halves"
                );
                let token = axis_token(invert_axis(resolved, *index)?, *positive)?;
                ensure!(
                    (*output == "l2" && token == "l2") || (*output == "r2" && token == "r2"),
                    "shadPS4 trigger {target} must use a trigger axis"
                );
                ensure!(
                    tokens
                        .insert((*output).to_owned(), token.to_owned())
                        .is_none(),
                    "shadPS4 output appears twice"
                );
            }
            let mut mappings = BTreeMap::new();
            for (output, token) in tokens {
                let is_axis = output.starts_with("axis_") || output == "l2" || output == "r2";
                mappings.insert(
                    output,
                    if is_axis {
                        Binding::Axis(token)
                    } else {
                        Binding::Button(token)
                    },
                );
            }
            ensure!(
                mappings.len() == CONTROLS.len(),
                "shadPS4 DualShock mapping is incomplete"
            );
            let text = game_input_ini(1, &mappings)?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-shadps4-")
                .tempdir()?;
            // The user directory resolves as $XDG_DATA_HOME/shadPS4; only
            // input_config/ is staged, saves stay in the real user dir.
            // Both default.ini (unified mode, the source default) and the
            // per-game file are staged identically.
            let input_dir = directory.path().join("shadPS4").join("input_config");
            fs::create_dir_all(&input_dir)?;
            let default_path = input_dir.join("default.ini");
            let game_path = input_dir.join(format!("{}.ini", setup.game_id));
            fs::write(&default_path, &text)?;
            fs::write(&game_path, &text)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &default_path,
                &game_path,
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

        /// Private root for `XDG_DATA_HOME`; shadPS4 appends `/shadPS4`.
        pub(crate) fn data_home(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "shadPS4 launch input changed");
            }
            let fresh = comparable(&observe(&self.setup, &[], cancel)?)?;
            ensure!(
                fresh == self.initial,
                "shadPS4 SDL inventory moved before launch"
            );
            let captured = observe(&self.setup, &[self.physical_path.clone()], cancel)?;
            ensure!(
                comparable(&captured)? == self.initial,
                "shadPS4 SDL inventory moved before launch"
            );
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()
        }
    }

    /// Raw SDL button index to input token through the resolved mapping.
    fn invert_button(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw: u32,
    ) -> Result<String> {
        for binding in &resolved.bindings {
            if let (ResolvedInput::Button { index }, ResolvedOutput::Button { index: gamepad }) =
                (&binding.input, &binding.output)
            {
                if *index == raw {
                    return Ok(button_token(*gamepad)?.to_owned());
                }
            }
        }
        anyhow::bail!("shadPS4 raw button {raw} has no gamepad mapping")
    }

    /// Raw SDL hat and pressed mask to dpad input token through the
    /// resolved mapping.
    fn invert_hat(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw_hat: u32,
        direction: u8,
    ) -> Result<String> {
        for binding in &resolved.bindings {
            if let (ResolvedInput::Hat { index, mask }, ResolvedOutput::Button { index: gamepad }) =
                (&binding.input, &binding.output)
            {
                if *index == raw_hat && *mask == direction && matches!(gamepad, 11..=14) {
                    return Ok(button_token(*gamepad)?.to_owned());
                }
            }
        }
        anyhow::bail!("shadPS4 raw hat {raw_hat} has no dpad mapping")
    }

    /// Raw SDL axis index to gamepad axis index through the resolved mapping.
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
                    ensure!(*gamepad <= 5, "shadPS4 gamepad axis is out of range");
                    return Ok(*gamepad);
                }
            }
        }
        anyhow::bail!("shadPS4 raw axis {raw} has no stick mapping")
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
                "shadPS4 executable differs from the saved trusted runtime"
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
                "shadPS4 launch plan changed after preparation"
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
                    "shadPS4 exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "shadPS4 did not open the selected SDL controller before timeout"
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
            anyhow::bail!("shadPS4 calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("shadPS4")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "shadPS4 identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "shadPS4 launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "shadPS4 calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "shadPS4 executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // input_config/ resolves under the XDG data root; saves stay in the
        // real user directory. The game keeps its default positional slot.
        plan.environment.push((
            std::ffi::OsString::from("XDG_DATA_HOME"),
            inputs.data_home().as_os_str().to_owned(),
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
