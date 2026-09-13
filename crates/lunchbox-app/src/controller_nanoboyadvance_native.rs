//! NanoBoyAdvance standalone Qt/SDL3 controller mapping.
//!
//! Functional contract pinned to nba-emu/NanoBoyAdvance
//! 55b5cf0ae3d929582ac5bfd486558173502b8354:
//! - `src/platform/qt/src/config.hh` defines `Input::Map` as the five-element
//!   array `[keyboard, controller.button, controller.axis,
//!   controller.hat, controller.hat_direction]` and stores the selected
//!   SDL GUID in `input.controller_guid`.
//! - `src/platform/qt/src/config.cc` serializes the ten `input.gba` keys as
//!   TOML arrays under `[input.gba]`; `controller_manager.cc` uses SDL3
//!   button/axis/hat values and opens the joystick selected by its GUID.
//! - `config.hh::GetConfigPath` selects `config.toml` below the Qt config
//!   location and the macOS app-container path when built as an app bundle.
//!
//! This module renders an input TOML fragment only.  A caller must merge it
//! into a copied baseline, preserving general.save_folder and every other
//! emulator option and root.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "55b5cf0ae3d929582ac5bfd486558173502b8354";
pub(crate) const PROFILE_ID: &str = "nanoboyadvance:standalone-gba-controller";

/// GBA controls and their `QtConfig::Input::gba` TOML keys.
pub(crate) const CONTROLS: [(&str, &str); 10] = [
    ("a", "a"),
    ("b", "b"),
    ("select", "select"),
    ("start", "start"),
    ("right", "right"),
    ("left", "left"),
    ("up", "up"),
    ("down", "down"),
    ("r", "r"),
    ("l", "l"),
];

/// SDL3 controller half/button/hat fields in NanoBoyAdvance's five-int map.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(i32),
    Axis { index: i32, negative: bool },
    Hat { index: i32, direction: i32 },
}

impl Binding {
    fn fields(self) -> Result<(i32, i32, i32, i32)> {
        match self {
            Self::Button(index) => {
                ensure!(
                    (0..=u8::MAX as i32).contains(&index),
                    "NanoBoyAdvance SDL button index is out of range"
                );
                Ok((index, -1, -1, 0))
            }
            Self::Axis { index, negative } => {
                ensure!(
                    (0..=u8::MAX as i32).contains(&index),
                    "NanoBoyAdvance SDL axis index is out of range"
                );
                let encoded = index | if negative { 0x100 } else { 0 };
                Ok((-1, encoded, -1, 0))
            }
            Self::Hat { index, direction } => {
                ensure!(
                    (0..=u8::MAX as i32).contains(&index),
                    "NanoBoyAdvance SDL hat index is out of range"
                );
                ensure!(
                    matches!(direction, 1 | 2 | 4 | 8),
                    "NanoBoyAdvance hat direction must be a cardinal SDL mask"
                );
                Ok((-1, -1, index, direction))
            }
        }
    }
}

/// Keyboard value plus optional controller mapping for one GBA key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Mapping {
    pub(crate) keyboard: i32,
    pub(crate) controller: Option<Binding>,
}

fn guid_valid(guid: &str) -> Result<()> {
    ensure!(
        guid.len() == 32
            && guid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "NanoBoyAdvance SDL GUID must be 32 lowercase hexadecimal characters"
    );
    Ok(())
}

fn toml_string(value: &str) -> Result<String> {
    ensure!(
        !value.chars().any(char::is_control),
        "NanoBoyAdvance GUID contains a control character"
    );
    Ok(format!("\"{value}\""))
}

/// Render the `[input]` and `[input.gba]` fragment using NanoBoyAdvance's
/// exact five-element arrays.  All ten GBA keys are required; an absent
/// controller binding is emitted as the source's `-1` sentinel.
pub(crate) fn input_toml_fragment(
    guid: &str,
    mappings: &BTreeMap<String, Mapping>,
) -> Result<String> {
    guid_valid(guid)?;
    ensure!(
        mappings.len() == CONTROLS.len(),
        "NanoBoyAdvance needs every GBA key"
    );
    let mut used = BTreeSet::new();
    let mut result = format!(
        "[input]\ncontroller_guid = {}\n\n[input.gba]\n",
        toml_string(guid)?
    );
    for (name, key) in CONTROLS {
        let mapping = *mappings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("NanoBoyAdvance GBA key {name} is absent"))?;
        let (button, axis, hat, direction) = match mapping.controller {
            Some(binding) => {
                ensure!(
                    used.insert(binding),
                    "NanoBoyAdvance reuses one physical input"
                );
                binding.fields()?
            }
            None => (-1, -1, -1, 0),
        };
        result.push_str(&format!(
            "{key} = [{}, {button}, {axis}, {hat}, {direction}]\n",
            mapping.keyboard
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mappings() -> BTreeMap<String, Mapping> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(index, (name, _))| {
                (
                    (*name).to_owned(),
                    Mapping {
                        keyboard: 65 + index as i32,
                        controller: Some(Binding::Button(index as i32)),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn input_fragment_uses_five_element_toml_maps() {
        let text = input_toml_fragment("0123456789abcdef0123456789abcdef", &mappings()).unwrap();
        assert!(text.starts_with(
            "[input]\ncontroller_guid = \"0123456789abcdef0123456789abcdef\"\n\n[input.gba]\n"
        ));
        assert!(text.contains("a = [65, 0, -1, -1, 0]\n"));
        assert!(text.contains("l = [74, 9, -1, -1, 0]\n"));
    }

    #[test]
    fn axis_hat_and_unset_sentinels_match_source() {
        let mut map = mappings();
        map.insert(
            "a".into(),
            Mapping {
                keyboard: 1,
                controller: Some(Binding::Axis {
                    index: 2,
                    negative: true,
                }),
            },
        );
        map.insert(
            "b".into(),
            Mapping {
                keyboard: 2,
                controller: Some(Binding::Hat {
                    index: 0,
                    direction: 1,
                }),
            },
        );
        map.insert(
            "l".into(),
            Mapping {
                keyboard: 3,
                controller: None,
            },
        );
        let text = input_toml_fragment("0123456789abcdef0123456789abcdef", &map).unwrap();
        assert!(text.contains("a = [1, -1, 258, -1, 0]\n"));
        assert!(text.contains("b = [2, -1, -1, 0, 1]\n"));
        assert!(text.contains("l = [3, -1, -1, -1, 0]\n"));
    }

    #[test]
    fn malformed_guid_or_duplicate_mapping_fails_closed() {
        assert!(input_toml_fragment("not-a-guid", &mappings()).is_err());
        assert!(input_toml_fragment("0123456789ABCDEF0123456789abcdef", &mappings()).is_err());
        let mut duplicate = mappings();
        duplicate.insert(
            "b".into(),
            Mapping {
                keyboard: 0,
                controller: Some(Binding::Button(0)),
            },
        );
        assert!(input_toml_fragment("0123456789abcdef0123456789abcdef", &duplicate).is_err());
    }

    #[test]
    fn axis_button_and_hat_indices_cannot_collide_with_axis_flag() {
        let mut map = mappings();
        map.insert(
            "a".into(),
            Mapping {
                keyboard: 1,
                controller: Some(Binding::Axis {
                    index: 256,
                    negative: false,
                }),
            },
        );
        assert!(input_toml_fragment("0123456789abcdef0123456789abcdef", &map).is_err());
    }
}
