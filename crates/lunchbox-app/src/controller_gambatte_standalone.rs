//! Gambatte Qt standalone-native controller settings.
//!
//! The Qt frontend uses QSettings with organization `gambatte` and
//! application `gambatte_qt`. Its `input` group stores each event as a
//! packed SDL event id plus a value; the SDL frontend instead has only a
//! command-line `--input` mapping and no persistent profile.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "04e7ddf85ff23032cb7132f155c42c7d1857f474";

/// Order and labels used by `GambatteSource::createInputDialog`.
pub(crate) const CONTROLS: [(&str, &str); 8] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "A"),
    ("b", "B"),
    ("start", "Start"),
    ("select", "Select"),
];

const JOY_AXIS: u8 = 0x01;
const JOY_HAT: u8 = 0x02;
const JOY_BUTTON: u8 = 0x08;
const AXIS_POSITIVE: i32 = 1;
const AXIS_NEGATIVE: i32 = 2;
const HAT_UP: u8 = 0x01;
const HAT_RIGHT: u8 = 0x02;

/// One value accepted by the native Qt frontend's `InputBox`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Keyboard(u32),
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
        direction: u8,
    },
}

impl Binding {
    fn event(self) -> Result<(u32, i32)> {
        match self {
            Self::Keyboard(key) => {
                ensure!(key != 0, "Gambatte keyboard key cannot be zero");
                Ok((key, 0x7fff_ffff))
            }
            Self::JoystickButton { device, button } => {
                Ok((pack_event(JOY_BUTTON, device, button), 1))
            }
            Self::JoystickAxis {
                device,
                axis,
                positive,
            } => Ok((
                pack_event(JOY_AXIS, device, axis),
                if positive {
                    AXIS_POSITIVE
                } else {
                    AXIS_NEGATIVE
                },
            )),
            Self::JoystickHat {
                device,
                hat,
                direction,
            } => {
                ensure!(
                    direction != 0 && direction & !0x0f == 0,
                    "Gambatte hat direction is invalid"
                );
                Ok((pack_event(JOY_HAT, device, hat), i32::from(direction)))
            }
        }
    }
}

fn pack_event(kind: u8, device: u8, number: u8) -> u32 {
    u32::from(kind) | (u32::from(device) << 8) | (u32::from(number) << 16)
}

/// Render the `[input]` QSettings INI group used by `gambatte_qt`.
///
/// The second slot is explicitly cleared. `device` is SDL's runtime
/// joystick index, not a stable physical-device identifier.
pub(crate) fn input_ini(bindings: &BTreeMap<String, Binding>) -> Result<String> {
    ensure!(
        bindings.len() == CONTROLS.len()
            && CONTROLS
                .iter()
                .all(|(control, _)| bindings.contains_key(*control)),
        "Gambatte requires every Game Boy gameplay control"
    );

    let mut seen = BTreeSet::new();
    let mut out = String::from("[input]\n");
    for (control, label) in CONTROLS {
        let event = bindings[control].event()?;
        ensure!(
            seen.insert(event),
            "Gambatte mapping reuses one input event"
        );
        out.push_str(&format!(
            "Game{label}Key1={}\nGame{label}Value1={}\nGame{label}Key2=0\nGame{label}Value2=0\n",
            event.0, event.1
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controls() -> BTreeMap<String, Binding> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(i, (control, _))| {
                (
                    (*control).to_owned(),
                    Binding::JoystickButton {
                        device: 1,
                        button: i as u8,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn emits_qsettings_input_group_and_clears_alternates() {
        let text = input_ini(&controls()).unwrap();
        assert!(text.starts_with("[input]\n"));
        assert!(text.contains("GameUpKey1=264\nGameUpValue1=1\n"));
        assert!(text.contains("GameSelectKey2=0\nGameSelectValue2=0\n"));
    }

    #[test]
    fn emits_native_axis_and_hat_event_values() {
        let mut map = controls();
        map.insert(
            "up".to_owned(),
            Binding::JoystickAxis {
                device: 2,
                axis: 3,
                positive: false,
            },
        );
        map.insert(
            "down".to_owned(),
            Binding::JoystickHat {
                device: 2,
                hat: 1,
                direction: HAT_RIGHT | HAT_UP,
            },
        );
        let text = input_ini(&map).unwrap();
        assert!(text.contains("GameUpKey1=197121\nGameUpValue1=2\n"));
        assert!(text.contains("GameDownKey1=66050\nGameDownValue1=3\n"));
    }

    #[test]
    fn rejects_duplicate_gameplay_events() {
        let mut map = controls();
        map.insert(
            "b".to_owned(),
            Binding::JoystickButton {
                device: 1,
                button: 0,
            },
        );
        assert!(input_ini(&map).is_err());
    }

    #[test]
    fn keyboard_uses_inputbox_keyboard_sentinel() {
        let mut map = controls();
        map.insert("a".to_owned(), Binding::Keyboard(0x44));
        let text = input_ini(&map).unwrap();
        assert!(text.contains("GameAKey1=68\nGameAValue1=2147483647\n"));
    }
}
