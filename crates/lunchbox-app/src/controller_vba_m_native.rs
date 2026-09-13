//! VBA-M standalone-native controller mappings.
//!
//! Pinned to visualboyadvance-m/visualboyadvance-m commit
//! fd13034143c128c8b68133a7a18bc785178ec4e4.  Both the Qt and wx frontends
//! persist the same `Joypad/<player>/<control>` values in their INI config.
//! `JoyN` is the SDL runtime joystick slot, not a stable physical identity.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "fd13034143c128c8b68133a7a18bc785178ec4e4";
pub(crate) const PROFILE_ID: &str = "vba-m:standalone-native-gba-controller";
pub(crate) const UPSTREAM_URL: &str = "https://github.com/visualboyadvance-m/visualboyadvance-m";

/// The ten gameplay controls emitted by the source's first joypad.
pub(crate) const CONTROLS: [(&str, &str); 10] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "A"),
    ("b", "B"),
    ("l", "L"),
    ("r", "R"),
    ("select", "Select"),
    ("start", "Start"),
];

/// One value accepted by VBA-M's `UserInput::FromConfigString` parser.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Keyboard(String),
    JoystickButton {
        device: u8,
        button: u8,
    },
    JoystickAxis {
        device: u8,
        axis: u8,
        positive: bool,
    },
    JoystickHat {
        device: u8,
        hat: u8,
        direction: HatDirection,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum HatDirection {
    North,
    South,
    West,
    East,
}

impl Binding {
    fn config_string(&self) -> Result<String> {
        match self {
            Self::Keyboard(value) => {
                ensure!(!value.is_empty(), "VBA-M keyboard binding is empty");
                ensure!(
                    !value
                        .chars()
                        .any(|ch| ch == ',' || ch == '\n' || ch == '\r' || ch.is_control()),
                    "VBA-M keyboard binding contains a separator or control character"
                );
                Ok(value.clone())
            }
            Self::JoystickButton { device, button } => {
                Ok(format!("Joy{}-Button{}", u16::from(*device) + 1, button))
            }
            Self::JoystickAxis {
                device,
                axis,
                positive,
            } => Ok(format!(
                "Joy{}-Axis{}{}",
                u16::from(*device) + 1,
                axis,
                if *positive { '+' } else { '-' }
            )),
            Self::JoystickHat {
                device,
                hat,
                direction,
            } => Ok(format!(
                "Joy{}-Hat{}{}",
                u16::from(*device) + 1,
                hat,
                match direction {
                    HatDirection::North => 'N',
                    HatDirection::South => 'S',
                    HatDirection::West => 'W',
                    HatDirection::East => 'E',
                }
            )),
        }
    }
}

fn validated_values(bindings: &BTreeMap<String, Binding>) -> Result<Vec<(&'static str, String)>> {
    ensure!(
        bindings.len() == CONTROLS.len(),
        "VBA-M requires all ten first-player gameplay controls"
    );
    let mut used = BTreeSet::new();
    let mut values = Vec::with_capacity(CONTROLS.len());
    for (name, key) in CONTROLS {
        let binding = bindings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("VBA-M control {name} is absent"))?;
        let value = binding.config_string()?;
        ensure!(
            used.insert(value.clone()),
            "VBA-M reuses one physical input"
        );
        values.push((key, value));
    }
    Ok(values)
}

/// Render a Qt `QSettings` INI fragment for the one-based first player.
/// QSettings escapes nested groups as `1\Control` under `[Joypad]`.
pub(crate) fn input_ini(bindings: &BTreeMap<String, Binding>) -> Result<String> {
    let mut output = String::from("[Joypad]\n");
    for (key, value) in validated_values(bindings)? {
        output.push_str("1\\");
        output.push_str(key);
        output.push('=');
        output.push_str(&value);
        output.push('\n');
    }
    Ok(output)
}

/// Render the equivalent wxFileConfig fragment, whose nested group is a
/// literal `[Joypad/1]` section.  Keep this separate from [`input_ini`]: Qt
/// and wx use different on-disk spellings for the same logical config path.
pub(crate) fn wx_input_ini(bindings: &BTreeMap<String, Binding>) -> Result<String> {
    let mut output = String::from("[Joypad/1]\n");
    for (key, value) in validated_values(bindings)? {
        output.push_str(key);
        output.push('=');
        output.push_str(&value);
        output.push('\n');
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_shared_joypad_ini_grammar() {
        let bindings = CONTROLS
            .iter()
            .enumerate()
            .map(|(index, (name, _))| {
                (
                    (*name).to_owned(),
                    Binding::JoystickButton {
                        device: 0,
                        button: index as u8,
                    },
                )
            })
            .collect();
        let text = input_ini(&bindings).unwrap();
        assert!(text.starts_with("[Joypad]\n1\\Up=Joy1-Button0\n"));
        assert!(text.contains("Start=Joy1-Button9\n"));
        assert!(
            wx_input_ini(&bindings)
                .unwrap()
                .starts_with("[Joypad/1]\nUp=Joy1-Button0\n")
        );
        assert_eq!(SOURCE_COMMIT.len(), 40);
    }

    #[test]
    fn emits_axis_hat_and_keyboard_values() {
        let mut bindings = CONTROLS
            .iter()
            .map(|(name, _)| ((*name).to_owned(), Binding::Keyboard("A".into())))
            .collect::<BTreeMap<_, _>>();
        bindings.insert(
            "up".into(),
            Binding::JoystickAxis {
                device: 1,
                axis: 2,
                positive: false,
            },
        );
        bindings.insert(
            "down".into(),
            Binding::JoystickHat {
                device: 1,
                hat: 0,
                direction: HatDirection::South,
            },
        );
        // Make the remaining keyboard values unique so duplicate rejection
        // tests the physical mappings rather than intentional keyboard reuse.
        for (index, (name, _)) in CONTROLS.iter().enumerate().skip(2) {
            bindings.insert((*name).into(), Binding::Keyboard(format!("F{}", index + 1)));
        }
        let text = input_ini(&bindings).unwrap();
        assert!(text.contains("Up=Joy2-Axis2-\n"));
        assert!(text.contains("Down=Joy2-Hat0S\n"));
    }

    #[test]
    fn duplicate_or_missing_mapping_fails_closed() {
        let mut bindings = BTreeMap::new();
        for (name, _) in CONTROLS {
            bindings.insert(name.to_owned(), Binding::Keyboard(name.to_owned()));
        }
        bindings.insert("a".into(), Binding::Keyboard("up".into()));
        bindings.insert("b".into(), Binding::Keyboard("up".into()));
        assert!(input_ini(&bindings).is_err());
        bindings.remove("start");
        assert!(input_ini(&bindings).is_err());
    }
}
