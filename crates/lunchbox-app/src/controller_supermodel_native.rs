//! Supermodel native Linux input overlay.
//!
//! Pinned source: trzy/Supermodel commit
//! 24d2ffcfc7f14229337f05f4920fe26b56633d9d. `Src/Inputs/InputSystem.cpp`
//! parses `JOY1_BUTTON1`, `JOY1_XAXIS_NEG`, `JOY1_UP`, and related tokens;
//! `Src/OSD/SDL/Main.cpp` stores them as `Input*` values in the `[ Global ]`
//! section of `Supermodel.ini`. This module only writes explicitly supplied
//! arcade action names; it never guesses a game's control deck.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button {
        joystick: u8,
        button: u8,
    },
    Axis {
        joystick: u8,
        axis: Axis,
        positive: Option<bool>,
    },
    Pov {
        joystick: u8,
        pov: u8,
        direction: PovDirection,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Axis {
    X,
    Y,
    Z,
    Rx,
    Ry,
    Rz,
    Slider1,
    Slider2,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum PovDirection {
    Up,
    Down,
    Left,
    Right,
}

impl Binding {
    pub(crate) fn token(self) -> Result<String> {
        match self {
            Self::Button { joystick, button } => {
                ensure!(
                    (1..=8).contains(&joystick) && (1..=32).contains(&button),
                    "Supermodel joystick button is out of range"
                );
                Ok(format!("JOY{joystick}_BUTTON{button}"))
            }
            Self::Axis {
                joystick,
                axis,
                positive,
            } => {
                ensure!(
                    (1..=8).contains(&joystick),
                    "Supermodel joystick is out of range"
                );
                let name = match axis {
                    Axis::X => "X",
                    Axis::Y => "Y",
                    Axis::Z => "Z",
                    Axis::Rx => "RX",
                    Axis::Ry => "RY",
                    Axis::Rz => "RZ",
                    Axis::Slider1 => "S1",
                    Axis::Slider2 => "S2",
                };
                let suffix = match positive {
                    None => "AXIS",
                    Some(true) => "AXIS_POS",
                    Some(false) => "AXIS_NEG",
                };
                Ok(format!("JOY{joystick}_{name}{suffix}"))
            }
            Self::Pov {
                joystick,
                pov,
                direction,
            } => {
                ensure!(
                    (1..=8).contains(&joystick) && (1..=4).contains(&pov),
                    "Supermodel POV is out of range"
                );
                let direction = match direction {
                    PovDirection::Up => "UP",
                    PovDirection::Down => "DOWN",
                    PovDirection::Left => "LEFT",
                    PovDirection::Right => "RIGHT",
                };
                Ok(format!("JOY{joystick}_POV{pov}_{direction}"))
            }
        }
    }
}

/// Set `InputSystem = "sdlgamepad"` in `[ Global ]` so the staged pad uses
/// the game-controller backend. The key is created when absent; machine
/// sections and unrelated settings are preserved.
pub(crate) fn set_input_system(baseline: &[u8]) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "Supermodel configuration is too large"
    );
    let text =
        String::from_utf8(baseline.to_vec()).context("Supermodel configuration is not UTF-8")?;
    let mut output = Vec::new();
    let mut global = false;
    let mut done = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            global = trimmed[1..trimmed.len() - 1].trim() == "Global";
        }
        if global
            && !done
            && trimmed
                .split_once('=')
                .map(|(key, _)| key.trim() == "InputSystem")
                .unwrap_or(false)
        {
            output.push("InputSystem = \"sdlgamepad\"");
            done = true;
        } else {
            output.push(line);
        }
    }
    let mut result = output.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }
    if !done {
        result.push_str("[ Global ]\nInputSystem = \"sdlgamepad\"\n");
    }
    Ok(result)
}

/// Replace explicitly named `[ Global ]` input keys while preserving all
/// machine-specific sections and unrelated settings.
pub(crate) fn patch_global_ini(
    baseline: &[u8],
    mappings: &[(String, Vec<Binding>)],
) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "Supermodel configuration is too large"
    );
    let text =
        String::from_utf8(baseline.to_vec()).context("Supermodel configuration is not UTF-8")?;
    let names = mappings
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<BTreeSet<_>>();
    for (name, bindings) in mappings {
        ensure!(
            !name.is_empty()
                && name.starts_with("Input")
                && name.bytes().all(|b| b.is_ascii_alphanumeric()),
            "Supermodel action name must be an Input* identifier"
        );
        ensure!(
            !bindings.is_empty() && bindings.len() <= 8,
            "Supermodel action needs one to eight bindings"
        );
        for binding in bindings {
            let _ = binding.token()?;
        }
    }
    let mut output = Vec::new();
    let mut global = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            global = trimmed[1..trimmed.len() - 1].trim() == "Global";
        }
        let key = if global {
            trimmed.split_once('=').map(|(key, _)| key.trim())
        } else {
            None
        };
        if key.is_some_and(|key| names.contains(key)) {
            continue;
        }
        output.push(line);
    }
    let mut result = output.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str("[ Global ]\n");
    for (name, bindings) in mappings {
        let values = bindings
            .iter()
            .map(|binding| binding.token())
            .collect::<Result<Vec<_>>>()?
            .join(",");
        result.push_str(&format!("{name} = \"{values}\"\n"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_sdlgamepad_backend() {
        let text = set_input_system(b"[ Global ]\nInputSystem = \"sdl\"\n").unwrap();
        assert!(text.contains("InputSystem = \"sdlgamepad\""));
        assert!(!text.contains("\"sdl\"\n"));
        let text = set_input_system(b"[ Core ]\nFoo = 1\n").unwrap();
        assert!(text.contains("[ Global ]\nInputSystem = \"sdlgamepad\""));
    }
    #[test]
    fn emits_pinned_supermodel_tokens() {
        assert_eq!(
            Binding::Button {
                joystick: 1,
                button: 9
            }
            .token()
            .unwrap(),
            "JOY1_BUTTON9"
        );
        assert_eq!(
            Binding::Axis {
                joystick: 1,
                axis: Axis::X,
                positive: Some(false)
            }
            .token()
            .unwrap(),
            "JOY1_XAXIS_NEG"
        );
        assert_eq!(
            Binding::Pov {
                joystick: 2,
                pov: 1,
                direction: PovDirection::Up
            }
            .token()
            .unwrap(),
            "JOY2_POV1_UP"
        );
    }
}

/// Registered catalog profile id (`{core}:standalone-{layout}` convention).
pub(crate) const PROFILE_ID: &str = "supermodel:standalone-supermodel-fighting";

/// Fighting-game actions on the shared arcade layout: P1 start/coin, the
/// 4-way stick, and punch/kick/guard on the first three buttons. Escape and
/// the remaining buttons stay on the user's own configuration.
pub(crate) const TARGETS: [(&str, &str); 10] = [
    ("start", "Start1"),
    ("select", "Coin1"),
    ("up", "JoyUp"),
    ("down", "JoyDown"),
    ("left", "JoyLeft"),
    ("right", "JoyRight"),
    ("button1", "Punch"),
    ("button2", "Kick"),
    ("button3", "Guard"),
    ("button4", "Escape"),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 10] = [
    ("start", "P1 Start"),
    ("select", "P1 Coin"),
    ("up", "Joystick Up"),
    ("down", "Joystick Down"),
    ("left", "Joystick Left"),
    ("right", "Joystick Right"),
    ("button1", "Punch"),
    ("button2", "Kick"),
    ("button3", "Guard"),
    ("button4", "Escape"),
];

/// SDL game-controller button index to Supermodel JOY1_BUTTON number.
/// `IsJoyButPressed` reads game-controller buttons 0..16 directly.
fn gamepad_button(gamepad: u32) -> Result<u8> {
    ensure!(
        gamepad <= 16,
        "Supermodel game-controller button {gamepad} is out of range"
    );
    Ok((gamepad + 1) as u8)
}

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
        /// The user's real `Supermodel.ini`; only `[ Global ]` input keys
        /// plus `InputSystem` are patched, so ROM paths, saves, and machine
        /// sections survive.
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
                "Supermodel setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_source,
                &self.probe_program,
                &self.sdl_library,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Supermodel setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Supermodel setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Supermodel native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Supermodel supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Supermodel player needs a saved controller identity"
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
                .context("Missing Supermodel native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Supermodel controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "Supermodel mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Supermodel needs native calibration for every arcade control"
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
                "detail": "Native launch stages a session Config/Supermodel.ini with sdlgamepad fighting inputs, then rechecks the exact SDL routes on Linux, Windows, and macOS. Ownership is strongest on Linux; other hosts pin the executable plus a fresh device re-probe. Only the single P1 fighting deck on joystick 1 is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Supermodel saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Supermodel emulator/content setup"
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
            serde_json::from_slice(&output).context("Invalid Supermodel SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Supermodel helper inspected a different SDL2 runtime"
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

    /// Raw SDL2 control to game-controller button index through the pad's
    /// effective mapping. The sdlgamepad backend reads buttons and hats
    /// only; axis halves have no button slot and are refused with a
    /// redirect to buttons or hats.
    fn gamepad_index(fields: &BTreeMap<String, String>, translated: DigitalInput) -> Result<u32> {
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
        Ok(match translated {
            DigitalInput::Button(index) => {
                let raw = u32::try_from(index).context("Supermodel button is too large")?;
                let mut found = None;
                for output in BUTTON_OUTPUTS {
                    if let Some(input) = fields.get(output) {
                        let candidate: u32 = input
                            .trim_end_matches('~')
                            .strip_prefix('b')
                            .context("Supermodel mapping entry is not a button")?
                            .parse()
                            .context("Supermodel mapping button is invalid")?;
                        if candidate == raw {
                            found = Some(
                                BUTTON_OUTPUTS
                                    .iter()
                                    .position(|name| *name == output)
                                    .context("Supermodel SDL button is unknown")?,
                            );
                            break;
                        }
                    }
                }
                u32::try_from(found.context("Supermodel raw button is unmapped")?)
                    .context("Supermodel gamepad button is too large")?
            }
            DigitalInput::Hat { direction, .. } => {
                let want = match direction {
                    0x01 => "dpup",
                    0x04 => "dpdown",
                    0x08 => "dpleft",
                    0x02 => "dpright",
                    _ => anyhow::bail!("Supermodel hat direction is not cardinal"),
                };
                let input = fields
                    .get(want)
                    .with_context(|| format!("Supermodel dpad entry {want} is unmapped"))?;
                ensure!(
                    input.trim_end_matches('~').starts_with('h'),
                    "Supermodel dpad entry is not a hat"
                );
                u32::try_from(
                    BUTTON_OUTPUTS
                        .iter()
                        .position(|name| *name == want)
                        .context("Supermodel SDL dpad is unknown")?,
                )
                .context("Supermodel gamepad button is too large")?
            }
            DigitalInput::Axis { .. } => {
                anyhow::bail!(
                    "Supermodel fighting inputs need buttons or hats; stick axes have no button slot"
                )
            }
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
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
                "Supermodel content must be a direct regular file with canonical ancestry"
            );
            ensure!(
                fs::symlink_metadata(&setup.config_source)?
                    .file_type()
                    .is_file(),
                "Supermodel config source must be a regular file"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Supermodel physical controller is missing or ambiguous"
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
                    "supermodel physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // SDL_GameControllerOpen(joyNum): JOY1 selects SDL index 0.
            ensure!(
                device.device_index == 0,
                "Supermodel JOY1 needs SDL index 0; selected pad is index {}",
                device.device_index
            );
            let fields = mapping_fields(
                device
                    .mapping
                    .as_deref()
                    .context("Supermodel SDL mapping is absent")?,
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Supermodel calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Supermodel native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("Supermodel SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("Supermodel SDL control counts are missing")?,
            )?;
            let mut mappings = Vec::new();
            for row in calibration.plan_profile(profile)?.rows {
                let (_, action) = TARGETS
                    .iter()
                    .find(|(target, _)| *target == row.target_id)
                    .context("Supermodel target is outside the fighting profile")?;
                let input = row
                    .input
                    .as_ref()
                    .context("Supermodel arcade control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("Supermodel requires measured native controls")?;
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
                    "Release the Supermodel controls before launch preparation"
                );
                mappings.push((
                    (*action).to_owned(),
                    vec![Binding::Button {
                        joystick: 1,
                        button: gamepad_button(gamepad_index(&fields, translated)?)?,
                    }],
                ));
            }
            ensure!(
                mappings.len() == TARGETS.len(),
                "Supermodel fighting mapping is incomplete"
            );
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-supermodel-")
                .tempdir()?;
            // `./Config` wins the config search, so running here fully
            // isolates inputs while saves stay session-local too.
            let config_dir = directory.path().join("Config");
            fs::create_dir(&config_dir)?;
            let config_path = config_dir.join("Supermodel.ini");
            let staged = patch_global_ini(&fs::read(&setup.config_source)?, &mappings)?;
            fs::write(&config_path, set_input_system(staged.as_bytes())?)?;
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

        pub(crate) fn directory(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Supermodel launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "Supermodel SDL index 0 moved before launch"
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
                "Supermodel executable differs from the saved trusted runtime"
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
                "Supermodel launch plan changed after preparation"
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
                    "Supermodel exited before controller handoff"
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
                    "Supermodel did not open the selected SDL controller before timeout"
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
            anyhow::bail!("Supermodel calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Supermodel")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Supermodel identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Supermodel launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Supermodel calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Supermodel executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // ./Config wins the config search; the ROM zip keeps its default
        // positional slot.
        plan.current_directory = inputs.directory().to_path_buf();
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
