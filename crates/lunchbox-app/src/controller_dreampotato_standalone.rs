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
