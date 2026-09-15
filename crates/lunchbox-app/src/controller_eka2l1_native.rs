//! EKA2L1 native SDL2 keybind-profile writer.
//!
//! Pinned source: EKA2L1/EKA2L1 commit
//! `8dd86cffc59d12c59661acecdfddfab5ffc810db`.  `config.cpp` serializes
//! `config.yml` and `bindings/<current-keybind-profile>.yml`; the SDL2
//! controller backend uses the enumerated joystick index as `controller_id`.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const PROFILE_ID: &str = "eka2l1:standalone-eka2l1-phone";
pub(crate) const SOURCE_COMMIT: &str = "8dd86cffc59d12c59661acecdfddfab5ffc810db";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BindingSource {
    Keyboard { keycode: u32 },
    Mouse { button: u32 },
    Controller { controller_id: i32, button_id: i32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KeyBinding {
    /// EKA2L1 guest scan/key code. The source casts this to std_scan_code.
    pub target: u32,
    pub source: BindingSource,
}

fn validate_name(name: &str) -> Result<()> {
    ensure!(!name.is_empty(), "EKA2L1 keybind profile name is empty");
    ensure!(name.len() <= 128, "EKA2L1 keybind profile name is too long");
    ensure!(
        name.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte)),
        "EKA2L1 profile name must be a safe YAML scalar and file stem"
    );
    ensure!(
        name != "." && name != "..",
        "EKA2L1 profile name is invalid"
    );
    Ok(())
}

fn validate_binding(binding: KeyBinding) -> Result<()> {
    ensure!(
        binding.target <= u16::MAX as u32,
        "EKA2L1 guest key code is out of range"
    );
    match binding.source {
        BindingSource::Keyboard { .. } | BindingSource::Mouse { .. } => {}
        BindingSource::Controller {
            controller_id,
            button_id,
        } => {
            ensure!(
                controller_id >= 0,
                "EKA2L1 controller ID must be non-negative"
            );
            ensure!(
                button_id >= 0 && (button_id <= 20 || (300..=311).contains(&button_id)),
                "EKA2L1 controller button code is unsupported"
            );
        }
    }
    Ok(())
}

/// Patch only the top-level `current-keybind-profile` scalar in `config.yml`.
/// The returned string is written beside the copied EKA2L1 install/data root;
/// the caller writes `bindings_yaml` to `bindings/<profile>.yml`.
pub(crate) fn patch_config(
    config_baseline: &[u8],
    profile_name: &str,
    bindings: &[KeyBinding],
) -> Result<(String, String)> {
    validate_name(profile_name)?;
    ensure!(
        bindings.len() <= 4096,
        "EKA2L1 keybind profile is too large"
    );
    let mut targets = BTreeSet::new();
    for binding in bindings {
        validate_binding(*binding)?;
        ensure!(
            targets.insert(binding.target),
            "EKA2L1 keybind target is duplicated"
        );
    }
    ensure!(
        config_baseline.len() <= 4 * 1024 * 1024,
        "EKA2L1 config.yml is too large"
    );
    let original =
        std::str::from_utf8(config_baseline).context("EKA2L1 config.yml is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "EKA2L1 config.yml contains a NUL byte"
    );
    let config = patch_profile_scalar(original, profile_name)?;
    Ok((config, serialize_bindings(bindings)))
}

fn patch_profile_scalar(original: &str, profile_name: &str) -> Result<String> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut seen = false;
    for line in original.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let key = body.split_once(':').map(|(key, _)| key.trim());
        if key == Some("current-keybind-profile") {
            ensure!(!seen, "EKA2L1 current-keybind-profile is duplicated");
            seen = true;
            output.push_str("current-keybind-profile: ");
            output.push_str(profile_name);
            output.push_str(newline);
        } else {
            output.push_str(line);
        }
    }
    if !seen {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str("current-keybind-profile: ");
        output.push_str(profile_name);
        output.push_str(newline);
    }
    Ok(output)
}

/// Serialize the exact YAML object shape consumed by EKA2L1's `keybind_profile`.
/// The file is owned by the selected profile, so unrelated `config.yml`, data,
/// firmware, media, and save/state files are not touched.
pub(crate) fn serialize_bindings(bindings: &[KeyBinding]) -> String {
    let mut output = String::new();
    for binding in bindings {
        output.push_str("- source:\n    type: ");
        match binding.source {
            BindingSource::Keyboard { keycode } => {
                output.push_str("key\n    data:\n      keycode: ");
                output.push_str(&keycode.to_string());
            }
            BindingSource::Mouse { button } => {
                output.push_str("mouse\n    data:\n      keycode: ");
                output.push_str(&button.to_string());
            }
            BindingSource::Controller {
                controller_id,
                button_id,
            } => {
                output.push_str("controller\n    data:\n      controller_id: ");
                output.push_str(&controller_id.to_string());
                output.push_str("\n      button_id: ");
                output.push_str(&button_id.to_string());
            }
        }
        output.push_str("\n  target: ");
        output.push_str(&binding.target.to_string());
        output.push('\n');
    }
    if bindings.is_empty() {
        "[]\n".to_owned()
    } else {
        output
    }
}

pub(crate) fn source_boundary() -> &'static str {
    "EKA2L1 stores a selected current-keybind-profile in config.yml and a complete YAML sequence in bindings/<profile>.yml. The SDL2 backend uses the process enumeration index as controller_id and maps standard buttons 0..20 plus virtual axis/trigger codes; probe and recheck SDL indices immediately before launch. Preserve the data/drives save tree, installed device/Z-drive firmware, ROM/media, and unrelated config keys."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_profile_name_and_serializes_guest_keyboard_and_controller_actions() {
        let (config, bindings) = patch_config(
            b"cpu: dynarmic\r\ncurrent-keybind-profile: default\r\nkeep: true\r\n",
            "arcade",
            &[
                KeyBinding {
                    target: 632,
                    source: BindingSource::Keyboard { keycode: 16777220 },
                },
                KeyBinding {
                    target: 633,
                    source: BindingSource::Controller {
                        controller_id: 1,
                        button_id: 11,
                    },
                },
            ],
        )
        .unwrap();
        assert!(config.contains("current-keybind-profile: arcade\r\nkeep: true\r\n"));
        assert!(
            bindings.contains("type: key\n    data:\n      keycode: 16777220\n  target: 632\n")
        );
        assert!(bindings.contains("type: controller\n    data:\n      controller_id: 1\n      button_id: 11\n  target: 633\n"));
    }

    #[test]
    fn rejects_duplicate_targets_invalid_controller_codes_and_path_names() {
        let duplicate = [
            KeyBinding {
                target: 1,
                source: BindingSource::Keyboard { keycode: 1 },
            },
            KeyBinding {
                target: 1,
                source: BindingSource::Keyboard { keycode: 2 },
            },
        ];
        assert!(patch_config(b"", "default", &duplicate).is_err());
        assert!(patch_config(b"", "../escape", &[]).is_err());
        assert!(patch_config(b"", "#yaml-comment", &[]).is_err());
        assert!(
            patch_config(
                b"",
                "default",
                &[KeyBinding {
                    target: 1,
                    source: BindingSource::Controller {
                        controller_id: 0,
                        button_id: 99
                    }
                }]
            )
            .is_err()
        );
    }
}

/// Private keybind profile stem written under `bindings/`, selected with `--kbp`.
pub(crate) const SESSION_PROFILE_NAME: &str = "lunchbox";

/// Guest `std_scan_code` targets mirroring the source default keybind roles
/// (`settings_dialog.cpp`): arrows, softkeys, middle select, green/red call
/// keys, star/hash, and clear.
pub(crate) const TARGETS: [(&str, u32); 12] = [
    ("up", 0x10),
    ("down", 0x11),
    ("left", 0x0e),
    ("right", 0x0f),
    ("a", 0xa7),
    ("b", 0xa5),
    ("x", 0xa4),
    ("y", 0xb4),
    ("l", 0x2a),
    ("r", 0x7f),
    ("start", 0xb5),
    ("select", 0x01),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 12] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "Select (middle)"),
    ("b", "Right softkey"),
    ("x", "Left softkey"),
    ("y", "Call (green)"),
    ("l", "Star"),
    ("r", "Hash"),
    ("start", "End (red)"),
    ("select", "Clear"),
];

/// SDL2 `SDL_GameControllerButton` indices in enum order.
const SDL_BUTTONS: [&str; 21] = [
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
    "misc1",
    "paddle1",
    "paddle2",
    "paddle3",
    "paddle4",
    "touchpad",
];

/// EKA2L1 frontend `controller_button_code` for each SDL2 game-controller
/// button index (`SDL_TO_FRONTEND_BUTTON_MAP` in `emu_controller_sdl2.cpp`).
/// Stick clicks have no frontend button code; sticks arrive via axes.
const FRONTEND_BUTTON_MAP: [Option<i32>; 21] = [
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(6),
    Some(8),
    Some(7),
    None,
    None,
    Some(4),
    Some(5),
    Some(11),
    Some(13),
    Some(14),
    Some(12),
    Some(15),
    Some(16),
    Some(17),
    Some(18),
    Some(19),
    Some(20),
];

/// SDL2 game-controller axis order to EKA2L1 stick/trigger base codes.
const STICK_AXES: [(&str, i32); 6] = [
    ("leftx", 300),
    ("lefty", 302),
    ("rightx", 304),
    ("righty", 306),
    ("lefttrigger", 308),
    ("righttrigger", 309),
];

/// Translate one raw SDL2 joystick control to an EKA2L1 frontend
/// `button_id` through the pad's effective game-controller mapping string.
/// Buttons resolve through button entries, hats through dpad hat entries,
/// and axis halves through full-axis stick entries (triggers included).
/// Anything else is refused rather than guessed.
pub(crate) fn frontend_button_id(
    mapping: &str,
    translated: lunchbox_controller_probe::duckstation::DigitalInput,
    positive: bool,
) -> Result<i32> {
    use lunchbox_controller_probe::duckstation::DigitalInput;
    let mut fields = std::collections::BTreeMap::new();
    for entry in mapping.split(',').skip(2) {
        if let Some((key, value)) = entry.split_once(':') {
            fields.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    /// A parsed mapping input: raw button, hat, or axis number.
    enum Input {
        Button(u32),
        Hat(u32),
        Axis(u32),
    }
    let parse = |input: &str| -> Result<Input> {
        let input = input.trim_end_matches('~');
        if let Some(rest) = input.strip_prefix('b') {
            return Ok(Input::Button(
                rest.parse().context("EKA2L1 mapping button is invalid")?,
            ));
        }
        if let Some(rest) = input.strip_prefix('h') {
            let (hat, _) = rest
                .split_once('.')
                .context("EKA2L1 mapping hat is invalid")?;
            return Ok(Input::Hat(
                hat.parse().context("EKA2L1 mapping hat is invalid")?,
            ));
        }
        if let Some(rest) = input.trim_start_matches(['+', '-']).strip_prefix('a') {
            return Ok(Input::Axis(
                rest.parse().context("EKA2L1 mapping axis is invalid")?,
            ));
        }
        anyhow::bail!("EKA2L1 gamepad mapping input is not a button, hat, or axis")
    };
    let sdl_index_of = |output: &str| -> Result<usize> {
        SDL_BUTTONS
            .iter()
            .position(|name| *name == output)
            .context("EKA2L1 SDL button output is unknown")
    };
    Ok(match translated {
        DigitalInput::Button(index) => {
            let raw = u32::try_from(index).context("EKA2L1 button index is too large")?;
            let mut found = None;
            for (output, input) in &fields {
                if SDL_BUTTONS.contains(&output.as_str())
                    && matches!(parse(input)?, Input::Button(candidate) if candidate == raw)
                {
                    found = Some(sdl_index_of(output)?);
                    break;
                }
            }
            let sdl_index = found.context("EKA2L1 raw button has no gamepad mapping")?;
            FRONTEND_BUTTON_MAP[sdl_index]
                .context("EKA2L1 stick clicks have no button code; use the stick axes")?
        }
        DigitalInput::Hat { direction, .. } => {
            // Hats surface as dpad buttons through the mapping.
            let want = match direction {
                0x01 => "dpup",
                0x04 => "dpdown",
                0x08 => "dpleft",
                0x02 => "dpright",
                _ => anyhow::bail!("EKA2L1 hat direction is not cardinal"),
            };
            let input = fields.get(want).context("EKA2L1 dpad entry is unmapped")?;
            ensure!(
                matches!(parse(input)?, Input::Hat(_)),
                "EKA2L1 dpad entry is not a hat"
            );
            let sdl_index = sdl_index_of(want)?;
            FRONTEND_BUTTON_MAP[sdl_index].context("EKA2L1 dpad has no button code")?
        }
        DigitalInput::Axis { index, .. } => {
            let raw = u32::try_from(index).context("EKA2L1 axis index is too large")?;
            let mut found = None;
            for (output, input) in &fields {
                if let Some(base) = STICK_AXES
                    .iter()
                    .find(|(name, _)| name == output)
                    .map(|(_, base)| *base)
                    && matches!(parse(input)?, Input::Axis(candidate) if candidate == raw)
                {
                    found = Some(base);
                    break;
                }
            }
            let base = found.context("EKA2L1 raw axis has no stick mapping")?;
            if base >= 308 || positive {
                base + i32::from(base < 308 && positive)
            } else {
                base
            }
        }
    })
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
        /// The user's real `config.yml`; only `current-keybind-profile` is
        /// patched, so storage, device, and firmware roots survive.
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
                "EKA2L1 setup needs an emulator identity"
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
                    "EKA2L1 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "EKA2L1 setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing EKA2L1 native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "EKA2L1 supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "EKA2L1 player needs a saved controller identity"
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
                .context("Missing EKA2L1 native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("EKA2L1 controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "EKA2L1 mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "EKA2L1 needs native calibration for every phone control"
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
                "detail": "Native launch stages a session config.yml plus bindings profile, then rechecks the exact SDL2 routes. Only the single phone pad is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many EKA2L1 saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate EKA2L1 emulator/content setup"
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
            serde_json::from_slice(&output).context("Invalid EKA2L1 SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "EKA2L1 helper inspected a different SDL2 runtime"
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
                "EKA2L1 content must be a direct regular file with canonical ancestry"
            );
            ensure!(
                fs::symlink_metadata(&setup.config_source)?
                    .file_type()
                    .is_file(),
                "EKA2L1 config source must be a regular file"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "EKA2L1 physical controller is missing or ambiguous"
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
                    "eka2l1 physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            ensure!(
                device.is_game_controller,
                "EKA2L1 needs an SDL-recognized game controller"
            );
            let mapping = device
                .mapping
                .as_deref()
                .context("EKA2L1 SDL mapping is absent")?;
            // controller_id is the SDL joystick index the backend opens.
            let controller_id =
                i32::try_from(device.device_index).context("EKA2L1 SDL index is too large")?;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("EKA2L1 calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing EKA2L1 native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("EKA2L1 SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("EKA2L1 SDL control counts are missing")?,
            )?;
            let mut bindings = Vec::new();
            for row in calibration.plan_profile(profile)?.rows {
                let (_, guest) = TARGETS
                    .iter()
                    .find(|(target, _)| *target == row.target_id)
                    .context("EKA2L1 target is outside the phone profile")?;
                let input = row
                    .input
                    .as_ref()
                    .context("EKA2L1 phone control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("EKA2L1 requires measured native controls")?;
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
                    "Release the EKA2L1 controls before launch preparation"
                );
                bindings.push(KeyBinding {
                    target: *guest,
                    source: BindingSource::Controller {
                        controller_id,
                        button_id: frontend_button_id(mapping, translated, native.direction > 0)?,
                    },
                });
            }
            ensure!(
                bindings.len() == TARGETS.len(),
                "EKA2L1 phone mapping is incomplete"
            );
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-eka2l1-")
                .tempdir()?;
            // config.yml and bindings/ resolve in the working directory; the
            // launch layer runs there. The baseline keeps storage, device,
            // and firmware roots; only the profile scalar is patched.
            let baseline = fs::read(&setup.config_source)?;
            let (config, bindings_yaml) = patch_config(&baseline, SESSION_PROFILE_NAME, &bindings)?;
            fs::write(directory.path().join("config.yml"), config)?;
            fs::create_dir(directory.path().join("bindings"))?;
            fs::write(
                directory
                    .path()
                    .join("bindings")
                    .join(format!("{SESSION_PROFILE_NAME}.yml")),
                bindings_yaml,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.config_source,
                &setup.probe_program,
                &setup.sdl_library,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            hashes.insert(
                directory.path().join("config.yml"),
                file_hash(&directory.path().join("config.yml"))?,
            );
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
                ensure!(file_hash(path)? == *hash, "EKA2L1 launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.is_game_controller,
                "EKA2L1 game controller disappeared before launch"
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
                "EKA2L1 executable differs from the saved trusted runtime"
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
                "EKA2L1 launch plan changed after preparation"
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
                    "EKA2L1 exited before controller handoff"
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
                    "EKA2L1 did not open the selected SDL controller before timeout"
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
            anyhow::bail!("EKA2L1 calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("EKA2L1")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "EKA2L1 identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "EKA2L1 launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "EKA2L1 calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "EKA2L1 executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // config.yml and bindings/ resolve in the working directory; --kbp
        // selects the staged profile and --runng the N-Gage game.
        plan.current_directory = inputs.directory().to_path_buf();
        plan.arguments = vec![
            std::ffi::OsString::from("--keybindprofile"),
            std::ffi::OsString::from(SESSION_PROFILE_NAME),
            std::ffi::OsString::from("--runng"),
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

#[cfg(test)]
mod translation_tests {
    use super::*;
    use lunchbox_controller_probe::duckstation::DigitalInput;

    const MAPPING: &str = "030000005e0400008e02000000000000,Pad,platform:Linux,a:b0,b:b1,x:b2,y:b3,back:b4,guide:b5,start:b6,leftstick:b7,rightstick:b8,leftshoulder:b9,rightshoulder:b10,dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,leftx:a0,lefty:a1,rightx:a2,righty:a3,lefttrigger:a4,righttrigger:a5";

    #[test]
    fn buttons_resolve_through_frontend_map() {
        // SDL back (4) -> frontend BACK (6); SDL start (6) -> frontend START (7).
        assert_eq!(
            frontend_button_id(MAPPING, DigitalInput::Button(4), false).unwrap(),
            6
        );
        assert_eq!(
            frontend_button_id(MAPPING, DigitalInput::Button(6), false).unwrap(),
            7
        );
        assert_eq!(
            frontend_button_id(MAPPING, DigitalInput::Button(0), false).unwrap(),
            0
        );
    }

    #[test]
    fn hats_resolve_through_dpad_entries() {
        assert_eq!(
            frontend_button_id(
                MAPPING,
                DigitalInput::Hat {
                    index: 0,
                    direction: 0x01
                },
                false
            )
            .unwrap(),
            11
        );
    }

    #[test]
    fn stick_halves_resolve_with_polarity() {
        assert_eq!(
            frontend_button_id(
                MAPPING,
                DigitalInput::Axis {
                    index: 0,
                    released: 0,
                    pressed: 0
                },
                false
            )
            .unwrap(),
            300
        );
        assert_eq!(
            frontend_button_id(
                MAPPING,
                DigitalInput::Axis {
                    index: 0,
                    released: 0,
                    pressed: 0
                },
                true
            )
            .unwrap(),
            301
        );
        assert_eq!(
            frontend_button_id(
                MAPPING,
                DigitalInput::Axis {
                    index: 4,
                    released: 0,
                    pressed: 0
                },
                true
            )
            .unwrap(),
            308
        );
    }

    #[test]
    fn unmapped_controls_are_refused() {
        assert!(frontend_button_id(MAPPING, DigitalInput::Button(11), false).is_err());
        assert!(frontend_button_id(MAPPING, DigitalInput::Button(7), false).is_err());
        assert!(
            frontend_button_id(
                MAPPING,
                DigitalInput::Axis {
                    index: 9,
                    released: 0,
                    pressed: 0
                },
                true
            )
            .is_err()
        );
    }
}
