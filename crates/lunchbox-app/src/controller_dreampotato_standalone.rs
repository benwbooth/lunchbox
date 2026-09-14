//! DreamPotato MonoGame configuration writer.
//!
//! Pinned source: RikkiGibson/DreamPotato
//! `ba03ef47622ee105f127d47bc359f9c47bb431c2`.  The front end serializes
//! `configuration.json` with System.Text.Json.  InputMappings contains
//! `KeyMappings`, `ButtonMappings`, and `GamePadIndex`; enum values are the
//! case-sensitive MonoGame names emitted by JsonStringEnumConverter.
//!
//! This module deliberately only writes the two input objects.  It does not
//! choose a physical device: GamePadIndex is the MonoGame enumeration slot at
//! launch time, not a stable controller identity.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "ba03ef47622ee105f127d47bc359f9c47bb431c2";
pub(crate) const MONOGAME_SOURCE_COMMIT: &str = "f34200720b558125964273fd3e7ab44cde0b0429";

/// The buttons exposed by DreamPotato's VmuButton enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VmuButton {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Mode,
    Sleep,
    InsertEject,
    Pause,
    FastForward,
    LoadState,
    SaveState,
    TakeScreenshot,
}

impl VmuButton {
    fn as_str(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::A => "A",
            Self::B => "B",
            Self::Mode => "Mode",
            Self::Sleep => "Sleep",
            Self::InsertEject => "InsertEject",
            Self::Pause => "Pause",
            Self::FastForward => "FastForward",
            Self::LoadState => "LoadState",
            Self::SaveState => "SaveState",
            Self::TakeScreenshot => "TakeScreenshot",
        }
    }
}

/// A host keyboard or MonoGame gamepad enum value mapped to a VMU button.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Mapping {
    Key {
        source_key: String,
        target: VmuButton,
    },
    Button {
        source_button: String,
        target: VmuButton,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InputProfile {
    pub mappings: Vec<Mapping>,
    /// MonoGame's GamePad.GetState(index). -1 means no gamepad mappings.
    pub gamepad_index: i32,
}

#[derive(Serialize)]
struct OutputKeyMapping<'a> {
    #[serde(rename = "SourceKey")]
    source_key: &'a str,
    #[serde(rename = "TargetButton")]
    target_button: &'static str,
}

#[derive(Serialize)]
struct OutputButtonMapping<'a> {
    #[serde(rename = "SourceButton")]
    source_button: &'a str,
    #[serde(rename = "TargetButton")]
    target_button: &'static str,
}

#[derive(Serialize)]
struct OutputInputProfile<'a> {
    #[serde(rename = "KeyMappings")]
    key_mappings: Vec<OutputKeyMapping<'a>>,
    #[serde(rename = "ButtonMappings")]
    button_mappings: Vec<OutputButtonMapping<'a>>,
    #[serde(rename = "GamePadIndex")]
    gamepad_index: i32,
}

#[derive(Deserialize)]
struct InputProfileShape {
    #[serde(rename = "KeyMappings")]
    key_mappings: Vec<Value>,
    #[serde(rename = "ButtonMappings")]
    button_mappings: Vec<Value>,
    #[serde(rename = "GamePadIndex")]
    gamepad_index: i32,
}

fn valid_monogame_key(name: &str) -> bool {
    let letter = name.len() == 1 && name.as_bytes()[0].is_ascii_uppercase();
    let digit = name.len() == 2 && name.starts_with('D') && name.as_bytes()[1].is_ascii_digit();
    let numpad = name
        .strip_prefix("NumPad")
        .is_some_and(|value| value.len() == 1 && value.as_bytes()[0].is_ascii_digit());
    let function = name.strip_prefix('F').is_some_and(|value| {
        !value.starts_with('0')
            && value
                .parse::<u8>()
                .is_ok_and(|number| (1..=24).contains(&number))
    });
    letter
        || digit
        || numpad
        || function
        || matches!(
            name,
            "None"
                | "Back"
                | "Tab"
                | "Enter"
                | "CapsLock"
                | "Escape"
                | "Space"
                | "PageUp"
                | "PageDown"
                | "End"
                | "Home"
                | "Left"
                | "Up"
                | "Right"
                | "Down"
                | "Select"
                | "Print"
                | "Execute"
                | "PrintScreen"
                | "Insert"
                | "Delete"
                | "Help"
                | "LeftWindows"
                | "RightWindows"
                | "Apps"
                | "Sleep"
                | "Multiply"
                | "Add"
                | "Separator"
                | "Subtract"
                | "Decimal"
                | "Divide"
                | "NumLock"
                | "Scroll"
                | "LeftShift"
                | "RightShift"
                | "LeftControl"
                | "RightControl"
                | "LeftAlt"
                | "RightAlt"
                | "BrowserBack"
                | "BrowserForward"
                | "BrowserRefresh"
                | "BrowserStop"
                | "BrowserSearch"
                | "BrowserFavorites"
                | "BrowserHome"
                | "VolumeMute"
                | "VolumeDown"
                | "VolumeUp"
                | "MediaNextTrack"
                | "MediaPreviousTrack"
                | "MediaStop"
                | "MediaPlayPause"
                | "LaunchMail"
                | "SelectMedia"
                | "LaunchApplication1"
                | "LaunchApplication2"
                | "OemSemicolon"
                | "OemPlus"
                | "OemComma"
                | "OemMinus"
                | "OemPeriod"
                | "OemQuestion"
                | "OemTilde"
                | "OemOpenBrackets"
                | "OemPipe"
                | "OemCloseBrackets"
                | "OemQuotes"
                | "Oem8"
                | "OemBackslash"
                | "ProcessKey"
                | "Attn"
                | "Crsel"
                | "Exsel"
                | "EraseEof"
                | "Play"
                | "Zoom"
                | "Pa1"
                | "OemClear"
                | "ChatPadGreen"
                | "ChatPadOrange"
                | "Pause"
                | "ImeConvert"
                | "ImeNoConvert"
                | "Kana"
                | "Kanji"
                | "OemAuto"
                | "OemCopy"
                | "OemEnlW"
        )
}

fn valid_monogame_button(name: &str) -> bool {
    matches!(
        name,
        "None"
            | "DPadUp"
            | "DPadDown"
            | "DPadLeft"
            | "DPadRight"
            | "Start"
            | "Back"
            | "LeftStick"
            | "RightStick"
            | "LeftShoulder"
            | "RightShoulder"
            | "BigButton"
            | "A"
            | "B"
            | "X"
            | "Y"
            | "LeftThumbstickLeft"
            | "RightTrigger"
            | "LeftTrigger"
            | "RightThumbstickUp"
            | "RightThumbstickDown"
            | "RightThumbstickRight"
            | "RightThumbstickLeft"
            | "LeftThumbstickUp"
            | "LeftThumbstickDown"
            | "LeftThumbstickRight"
    )
}

fn output_profile(profile: &InputProfile) -> Result<Value> {
    ensure!(
        (-1..=15).contains(&profile.gamepad_index),
        "DreamPotato GamePadIndex must be -1 or a DesktopGL slot from 0 through 15"
    );
    ensure!(
        !profile.mappings.is_empty(),
        "DreamPotato input profile needs at least one mapping"
    );

    let mut key_mappings = Vec::new();
    let mut button_mappings = Vec::new();
    let mut seen = BTreeSet::new();
    for mapping in &profile.mappings {
        match mapping {
            Mapping::Key { source_key, target } => {
                ensure!(
                    valid_monogame_key(source_key),
                    "DreamPotato keyboard source is not a MonoGame 3.8.4 Keys value"
                );
                ensure!(
                    seen.insert(("key", source_key.as_str(), target.as_str())),
                    "DreamPotato keyboard mapping is duplicated"
                );
                key_mappings.push(OutputKeyMapping {
                    source_key,
                    target_button: target.as_str(),
                });
            }
            Mapping::Button {
                source_button,
                target,
            } => {
                ensure!(
                    valid_monogame_button(source_button),
                    "DreamPotato gamepad source is not a MonoGame 3.8.4 Buttons value"
                );
                ensure!(
                    seen.insert(("button", source_button.as_str(), target.as_str())),
                    "DreamPotato gamepad mapping is duplicated"
                );
                button_mappings.push(OutputButtonMapping {
                    source_button,
                    target_button: target.as_str(),
                });
            }
        }
    }

    let output = OutputInputProfile {
        key_mappings,
        button_mappings,
        gamepad_index: profile.gamepad_index,
    };
    let value = serde_json::to_value(output).context("Serializing DreamPotato input profile")?;
    serde_json::from_value::<InputProfileShape>(value.clone())
        .context("DreamPotato input profile does not match the pinned JSON shape")?;
    Ok(value)
}

/// Patch only `PrimaryInput` and `SecondaryInput` in an existing
/// `configuration.json`, preserving unrelated configuration fields.
pub(crate) fn patch_config(
    baseline: &[u8],
    primary: &InputProfile,
    secondary: &InputProfile,
) -> Result<Vec<u8>> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "DreamPotato config is too large"
    );
    let mut root: Value =
        serde_json::from_slice(baseline).context("Parsing DreamPotato configuration.json")?;
    let object = root
        .as_object_mut()
        .context("DreamPotato configuration root must be a JSON object")?;
    object.insert("PrimaryInput".to_owned(), output_profile(primary)?);
    object.insert("SecondaryInput".to_owned(), output_profile(secondary)?);
    serde_json::to_vec_pretty(&root).context("Serializing DreamPotato configuration.json")
}

/// Patch only `PrimaryInput` in an existing `configuration.json`, leaving
/// `SecondaryInput` (the second VMU) exactly as the user configured it. A
/// single-pad session must not invent bindings for a pad it never measured.
pub(crate) fn patch_primary_config(baseline: &[u8], primary: &InputProfile) -> Result<Vec<u8>> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "DreamPotato config is too large"
    );
    let mut root: Value =
        serde_json::from_slice(baseline).context("Parsing DreamPotato configuration.json")?;
    let object = root
        .as_object_mut()
        .context("DreamPotato configuration root must be a JSON object")?;
    object.insert("PrimaryInput".to_owned(), output_profile(primary)?);
    serde_json::to_vec_pretty(&root).context("Serializing DreamPotato configuration.json")
}

pub(crate) fn source_boundary() -> &'static str {
    "DreamPotato's MonoGame front end persists InputMappings in configuration.json. GamePadIndex is the runtime MonoGame enumeration slot (-1 for none), not a stable physical-device identity; probe and recheck it immediately before launch. This writer patches only PrimaryInput and SecondaryInput, preserving VMU data, saves, states, and unrelated configuration fields."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(index: i32) -> InputProfile {
        InputProfile {
            gamepad_index: index,
            mappings: vec![
                Mapping::Key {
                    source_key: "W".into(),
                    target: VmuButton::Up,
                },
                Mapping::Button {
                    source_button: "DPadUp".into(),
                    target: VmuButton::Up,
                },
            ],
        }
    }

    #[test]
    fn patches_only_the_two_input_objects_and_preserves_other_fields() {
        let output = patch_config(
            br#"{"Volume":9,"PrimaryInput":{"old":true},"Keep":{"x":1}}"#,
            &profile(0),
            &profile(-1),
        )
        .unwrap();
        let value: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value["Volume"], 9);
        assert_eq!(value["Keep"]["x"], 1);
        assert_eq!(value["PrimaryInput"]["GamePadIndex"], 0);
        assert_eq!(value["SecondaryInput"]["GamePadIndex"], -1);
        assert_eq!(value["PrimaryInput"]["KeyMappings"][0]["SourceKey"], "W");
        assert_eq!(
            value["PrimaryInput"]["KeyMappings"][0]["TargetButton"],
            "Up"
        );
    }

    #[test]
    fn rejects_invalid_slot_empty_profile_and_malformed_enum_identifier() {
        assert!(output_profile(&profile(16)).is_err());
        assert!(output_profile(&profile(15)).is_ok());
        assert!(
            output_profile(&InputProfile {
                gamepad_index: -1,
                mappings: vec![],
            })
            .is_err()
        );
        assert!(
            output_profile(&InputProfile {
                gamepad_index: -1,
                mappings: vec![Mapping::Key {
                    source_key: "not-a-key".into(),
                    target: VmuButton::A,
                }],
            })
            .is_err()
        );
        assert!(
            output_profile(&InputProfile {
                gamepad_index: -1,
                mappings: vec![Mapping::Button {
                    source_button: "South".into(),
                    target: VmuButton::A,
                }],
            })
            .is_err()
        );
    }
}

/// Registered catalog profile id (`{core}:standalone-{layout}` convention).
pub(crate) const PROFILE_ID: &str = "dreampotato:standalone-dreampotato-vmu";

/// Gameplay VMU buttons with the layout target feeding each one. Menu and
/// system buttons (Mode, Sleep, screenshots, states) have no gamepad control
/// in the shared layout and stay on the user's own configuration.
pub(crate) const TARGETS: [(&str, VmuButton); 6] = [
    ("up", VmuButton::Up),
    ("down", VmuButton::Down),
    ("left", VmuButton::Left),
    ("right", VmuButton::Right),
    ("a", VmuButton::A),
    ("b", VmuButton::B),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 6] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "A"),
    ("b", "B"),
];

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use std::{collections::HashMap, path::PathBuf};

    #[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Player {
        pub player: u8,
        pub controller_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        /// The user's real `configuration.json`; only `PrimaryInput` is
        /// patched, so saves, states, BIOS, and the second VMU survive.
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
                "DreamPotato setup needs an emulator identity"
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
                    "DreamPotato setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "DreamPotato setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing DreamPotato native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "DreamPotato supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "DreamPotato player needs a saved controller identity"
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
                .context("Missing DreamPotato native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("DreamPotato controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "DreamPotato mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "DreamPotato needs native calibration for every VMU control"
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
                "detail": "Native Linux launch stages a session configuration.json with a gamepad PrimaryInput, then rechecks the exact SDL2 routes. Only the single VMU pad in MonoGame slot 0 is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many DreamPotato saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate DreamPotato emulator/content setup"
            );
        }
        Ok(())
    }
}

/// SDL2 game-controller button output to MonoGame `Buttons` name.
fn monogame_button(output: &str) -> Result<&'static str> {
    Ok(match output {
        "a" => "A",
        "b" => "B",
        "x" => "X",
        "y" => "Y",
        "back" => "Back",
        "guide" => "BigButton",
        "start" => "Start",
        "leftstick" => "LeftStick",
        "rightstick" => "RightStick",
        "leftshoulder" => "LeftShoulder",
        "rightshoulder" => "RightShoulder",
        "dpup" => "DPadUp",
        "dpdown" => "DPadDown",
        "dpleft" => "DPadLeft",
        "dpright" => "DPadRight",
        _ => anyhow::bail!("DreamPotato gamepad button {output} has no MonoGame name"),
    })
}

/// SDL2 game-controller axis output and polarity to MonoGame `Buttons` name.
fn monogame_axis(output: &str, positive: bool) -> Result<&'static str> {
    Ok(match (output, positive) {
        ("leftx", false) => "LeftThumbstickLeft",
        ("leftx", true) => "LeftThumbstickRight",
        ("lefty", false) => "LeftThumbstickUp",
        ("lefty", true) => "LeftThumbstickDown",
        ("rightx", false) => "RightThumbstickLeft",
        ("rightx", true) => "RightThumbstickRight",
        ("righty", false) => "RightThumbstickUp",
        ("righty", true) => "RightThumbstickDown",
        ("lefttrigger", _) => "LeftTrigger",
        ("righttrigger", _) => "RightTrigger",
        _ => anyhow::bail!("DreamPotato gamepad axis {output} has no MonoGame name"),
    })
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
            serde_json::from_slice(&output).context("Invalid DreamPotato SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "DreamPotato helper inspected a different SDL2 runtime"
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

    /// Raw SDL2 control to MonoGame `Buttons` source name through the pad's
    /// effective game-controller mapping.
    fn source_button(
        fields: &BTreeMap<String, String>,
        translated: DigitalInput,
        positive: bool,
    ) -> Result<String> {
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
        const AXIS_OUTPUTS: [&str; 6] = [
            "leftx",
            "lefty",
            "rightx",
            "righty",
            "lefttrigger",
            "righttrigger",
        ];
        Ok(match translated {
            DigitalInput::Button(index) => {
                let raw = u32::try_from(index).context("DreamPotato button is too large")?;
                let mut found = None;
                for output in BUTTON_OUTPUTS {
                    if let Some(input) = fields.get(output) {
                        let candidate: u32 = input
                            .trim_end_matches('~')
                            .strip_prefix('b')
                            .context("DreamPotato mapping entry is not a button")?
                            .parse()
                            .context("DreamPotato mapping button is invalid")?;
                        if candidate == raw {
                            found = Some(output);
                            break;
                        }
                    }
                }
                monogame_button(found.context("DreamPotato raw button is unmapped")?)?.to_owned()
            }
            DigitalInput::Hat { direction, .. } => {
                let want = match direction {
                    0x01 => "dpup",
                    0x04 => "dpdown",
                    0x08 => "dpleft",
                    0x02 => "dpright",
                    _ => anyhow::bail!("DreamPotato hat direction is not cardinal"),
                };
                let input = fields
                    .get(want)
                    .with_context(|| format!("DreamPotato dpad entry {want} is unmapped"))?;
                ensure!(
                    input.trim_end_matches('~').starts_with('h'),
                    "DreamPotato dpad entry is not a hat"
                );
                monogame_button(want)?.to_owned()
            }
            DigitalInput::Axis { index, .. } => {
                let raw = u32::try_from(index).context("DreamPotato axis is too large")?;
                let mut found = None;
                for output in AXIS_OUTPUTS {
                    if let Some(input) = fields.get(output) {
                        let candidate: u32 = input
                            .trim_start_matches(['+', '-'])
                            .trim_end_matches('~')
                            .strip_prefix('a')
                            .context("DreamPotato stick entry is not an axis")?
                            .parse()
                            .context("DreamPotato stick axis is invalid")?;
                        if candidate == raw {
                            found = Some(output);
                            break;
                        }
                    }
                }
                monogame_axis(found.context("DreamPotato raw axis is unmapped")?, positive)?
                    .to_owned()
            }
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        physical_path: String,
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
                "DreamPotato content must be a direct regular file with canonical ancestry"
            );
            ensure!(
                fs::symlink_metadata(&setup.config_source)?
                    .file_type()
                    .is_file(),
                "DreamPotato config source must be a regular file"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "DreamPotato physical controller is missing or ambiguous"
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
            // MonoGame opens DesktopGL slots in order; the pad must be the
            // first SDL joystick so slot 0 selects it.
            ensure!(
                device.device_index == 0,
                "DreamPotato opens SDL joystick 0; selected pad is index {}",
                device.device_index
            );
            let fields = mapping_fields(
                device
                    .mapping
                    .as_deref()
                    .context("DreamPotato SDL mapping is absent")?,
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("DreamPotato calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing DreamPotato native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("DreamPotato SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("DreamPotato SDL control counts are missing")?,
            )?;
            let mut mappings = Vec::new();
            for row in calibration.plan_profile(profile)?.rows {
                let (_, target) = TARGETS
                    .iter()
                    .find(|(known, _)| *known == row.target_id)
                    .context("DreamPotato target is outside the VMU profile")?;
                let input = row
                    .input
                    .as_ref()
                    .context("DreamPotato VMU control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("DreamPotato requires measured native controls")?;
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
                    "Release the DreamPotato controls before launch preparation"
                );
                mappings.push(Mapping::Button {
                    source_button: source_button(&fields, translated, native.direction > 0)?,
                    target: *target,
                });
            }
            ensure!(
                mappings.len() == TARGETS.len(),
                "DreamPotato VMU mapping is incomplete"
            );
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-dreampotato-")
                .tempdir()?;
            // configuration.json resolves under $XDG_DATA_HOME/DreamPotato;
            // only PrimaryInput is patched, everything else survives.
            let config_dir = directory.path().join("DreamPotato");
            fs::create_dir(&config_dir)?;
            let config_path = config_dir.join("configuration.json");
            fs::write(
                &config_path,
                patch_primary_config(
                    &fs::read(&setup.config_source)?,
                    &InputProfile {
                        mappings,
                        gamepad_index: 0,
                    },
                )?,
            )?;
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
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        /// Private root for `XDG_DATA_HOME`; DreamPotato appends
        /// `/DreamPotato` itself.
        pub(crate) fn config_home(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(
                    file_hash(path)? == *hash,
                    "DreamPotato launch input changed"
                );
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "DreamPotato SDL index 0 moved before launch"
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
                "DreamPotato executable differs from the saved trusted runtime"
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
                "DreamPotato launch plan changed after preparation"
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
                    "DreamPotato exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "DreamPotato did not open the selected SDL controller before timeout"
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
            anyhow::bail!("DreamPotato calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("DreamPotato")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "DreamPotato identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "DreamPotato launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "DreamPotato calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "DreamPotato executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // configuration.json resolves under $XDG_DATA_HOME/DreamPotato; the
        // game keeps its default positional slot.
        plan.environment.push((
            std::ffi::OsString::from("XDG_DATA_HOME"),
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
