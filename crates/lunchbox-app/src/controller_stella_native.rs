//! Stella 7.x native `stella.sqlite3` controller mappings for the SDL3
//! joypad driver with `SDL_JOYSTICK_LINUX_CLASSIC=1`, not libretro bindings.
//!
//! Functional contract pinned to stella-emu/stella
//! c65c845c8686c81698ffbd2fc9dfc5ccea5b32a1 (7.x line):
//! - `PJoystickHandler.cxx`: the `joymap` settings key holds a JSON array with
//!   one object per joystick: `name`, `port` ("Left"/"Right"/"Auto") and six
//!   event-mode arrays (kMenuMode, kJoystickMode, kPaddlesMode, kKeyboardMode,
//!   kDrivingMode, kCommonMode). Sticks are matched by name; same-name sticks
//!   are disambiguated with " #N" by enumeration order among active sticks.
//! - `JoyMap.cxx`: each mapping entry has `event` plus `button`, or `axis` +
//!   `axisDirection`, or `hat` + `hatDirection`. A missing `event_ver` setting
//!   (Event::VERSION = 9) drops all saved mappings.
//! - `PJoystickHandler::handleRegularAxisEvent`: SDL axis indices are cast
//!   straight to JoyAxis (X=0, Y=1, Z=2, A3..A7=3..8), and digital engagement
//!   uses the dead zone 3200 + joydeadzone*1000 (default 13 → 16200).
//! - `main.cxx`: `-basedir <dir>` relocates every configuration file, and a
//!   bare path argument is the ROM.
//! - `OSystemStandalone`/`StellaDb`/`KeyValueRepositorySqlite`: settings live
//!   in `<basedir>/stella.sqlite3`, table `settings(setting TEXT PRIMARY KEY,
//!   value TEXT) WITHOUT ROWID`.
//! - The launch and the observation probe run with
//!   `SDL_JOYSTICK_LINUX_CLASSIC=1`, pinning the verified SDL 3.2.20 classic
//!   backend whose joystick numbering equals the kernel joydev order (the
//!   same semantics the DuckStation contract relies on).
use anyhow::{Context, Result};
use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Atari 2600 panel target controls: layout id -> Stella event name. The
/// writer expands JoystickZero* to JoystickOne* for player two.
pub(crate) const CONTROLS: [(&str, &str); 15] = [
    ("up", "JoystickZeroUp"),
    ("down", "JoystickZeroDown"),
    ("left", "JoystickZeroLeft"),
    ("right", "JoystickZeroRight"),
    ("fire", "JoystickZeroFire"),
    ("trigger", "JoystickZeroFire5"),
    ("booster", "JoystickZeroFire9"),
    ("select", "ConsoleSelect"),
    ("reset", "ConsoleReset"),
    ("left_diff_a", "ConsoleLeftDiffA"),
    ("left_diff_b", "ConsoleLeftDiffB"),
    ("right_diff_a", "ConsoleRightDiffA"),
    ("right_diff_b", "ConsoleRightDiffB"),
    ("color", "ConsoleColor"),
    ("black_white", "ConsoleBlackWhite"),
];

/// Controls owned by player two's stick (console switches stay on player one).
pub(crate) const PLAYER_EVENTS: [&str; 7] = [
    "JoystickZeroUp",
    "JoystickZeroDown",
    "JoystickZeroLeft",
    "JoystickZeroRight",
    "JoystickZeroFire",
    "JoystickZeroFire5",
    "JoystickZeroFire9",
];

/// The six event-mode keys every persisted joystick object must carry.
pub(crate) const MODES: [&str; 6] = [
    "kMenuMode",
    "kJoystickMode",
    "kPaddlesMode",
    "kKeyboardMode",
    "kDrivingMode",
    "kCommonMode",
];

/// JoyAxis names indexed by SDL axis index (X=0, Y=1, Z=2, A3..A7=3..7).
pub(crate) const AXES: [&str; 8] = ["x", "y", "z", "a3", "a4", "a5", "a6", "a7"];

/// Default joydeadzone 13: 3200 + 13*1000 in SDL axis units.
pub(crate) const DIGITAL_THRESHOLD: i32 = 16200;

/// One native mapping entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, negative: bool },
    Hat { input: u32, up: bool },
}

impl Binding {
    pub(crate) fn entry(&self, event: &str) -> Result<serde_json::Value> {
        Ok(match *self {
            Binding::Button(button) => serde_json::json!({"event": event, "button": button}),
            Binding::Axis { index, negative } => {
                let axis = AXES
                    .get(usize::try_from(index).ok().unwrap_or(usize::MAX))
                    .with_context(|| format!("Stella has no JoyAxis name for axis {index}"))?;
                serde_json::json!({
                    "event": event,
                    "axis": axis,
                    "axisDirection": if negative { "negative" } else { "positive" },
                })
            }
            Binding::Hat { input, up } => serde_json::json!({
                "event": event,
                "hat": input,
                // SDL hats report horizontal X first, vertical Y second.
                "hatDirection": if input % 2 == 0 {
                    if up { "left" } else { "right" }
                } else if up {
                    "up"
                } else {
                    "down"
                },
            }),
        })
    }
}

/// One stick's persisted mapping object.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stick {
    pub(crate) name: String,
    pub(crate) port: &'static str,
    /// kJoystickMode entries; every other mode key is an empty array.
    pub(crate) joystick_mode: BTreeMap<&'static str, Binding>,
}

impl Stick {
    pub(crate) fn to_json(&self) -> Result<serde_json::Value> {
        let mut mapping = serde_json::Map::new();
        mapping.insert("name".into(), self.name.clone().into());
        mapping.insert("port".into(), self.port.into());
        for mode in MODES {
            let entries: Vec<_> = if mode == "kJoystickMode" {
                self.joystick_mode
                    .iter()
                    .map(|(event, binding)| binding.entry(event))
                    .collect::<Result<_>>()?
            } else {
                Vec::new()
            };
            mapping.insert(mode.into(), entries.into());
        }
        Ok(serde_json::Value::Object(mapping))
    }
}

/// Apply Stella's same-name disambiguation to the connected stick names, in
/// enumeration order: earlier active sticks sharing the name prefix each add
/// one count, and a colliding stick becomes "<name> #<count+1>".
pub(crate) fn unique_names(names: &[String]) -> Vec<String> {
    let mut assigned: Vec<String> = Vec::new();
    for name in names {
        let count = assigned
            .iter()
            .filter(|existing| existing.to_lowercase().starts_with(&name.to_lowercase()))
            .count();
        assigned.push(if count > 0 {
            format!("{name} #{}", count + 1)
        } else {
            name.clone()
        });
    }
    assigned
}

/// Render the complete `joymap` value for the saved players.
pub(crate) fn joymap(sticks: &[Stick]) -> Result<String> {
    let array = sticks
        .iter()
        .map(Stick::to_json)
        .collect::<Result<Vec<_>>>()?;
    Ok(serde_json::to_string(&array)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bindings() -> BTreeMap<&'static str, Binding> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(i, (control, event))| {
                (
                    *event,
                    match i {
                        0 => Binding::Axis {
                            index: 1,
                            negative: true,
                        },
                        1 => Binding::Axis {
                            index: 1,
                            negative: false,
                        },
                        2 => Binding::Axis {
                            index: 0,
                            negative: true,
                        },
                        3 => Binding::Axis {
                            index: 0,
                            negative: false,
                        },
                        _ => Binding::Button(u32::try_from(i).unwrap()),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn entries_use_the_pinned_grammar() {
        assert_eq!(
            Binding::Button(3).entry("JoystickZeroFire").unwrap(),
            serde_json::json!({"event": "JoystickZeroFire", "button": 3})
        );
        assert_eq!(
            Binding::Axis {
                index: 0,
                negative: true
            }
            .entry("JoystickZeroLeft")
            .unwrap(),
            serde_json::json!({"event": "JoystickZeroLeft", "axis": "x", "axisDirection": "negative"})
        );
        assert_eq!(
            Binding::Axis {
                index: 3,
                negative: false
            }
            .entry("JoystickZeroUp")
            .unwrap(),
            serde_json::json!({"event": "JoystickZeroUp", "axis": "a3", "axisDirection": "positive"})
        );
        assert_eq!(
            Binding::Hat { input: 0, up: true }
                .entry("JoystickZeroUp")
                .unwrap(),
            serde_json::json!({"event": "JoystickZeroUp", "hat": 0, "hatDirection": "left"})
        );
        assert_eq!(
            Binding::Hat {
                input: 1,
                up: false
            }
            .entry("JoystickZeroDown")
            .unwrap(),
            serde_json::json!({"event": "JoystickZeroDown", "hat": 1, "hatDirection": "down"})
        );
        assert!(
            Binding::Axis {
                index: 8,
                negative: true
            }
            .entry("X")
            .is_err()
        );
    }

    #[test]
    fn stick_objects_carry_every_mode_key() {
        let stick = Stick {
            name: "Brawler64".into(),
            port: "Left",
            joystick_mode: bindings(),
        };
        let json = stick.to_json().unwrap();
        assert_eq!(json["name"], "Brawler64");
        assert_eq!(json["port"], "Left");
        for mode in MODES {
            let entries = json[mode].as_array().unwrap();
            assert_eq!(
                entries.len(),
                if mode == "kJoystickMode" {
                    CONTROLS.len()
                } else {
                    0
                }
            );
        }
        let up = json["kJoystickMode"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["event"] == "JoystickZeroUp")
            .unwrap();
        assert_eq!(up["axis"], "y");
    }

    #[test]
    fn duplicate_names_get_stella_suffixes() {
        let names = vec!["Xbox Pad".to_string(), "Other".into(), "xbox pad".into()];
        assert_eq!(unique_names(&names), ["Xbox Pad", "Other", "xbox pad #2"]);
    }

    #[test]
    fn joymap_is_a_json_array_round_tripping_the_sticks() {
        let sticks = vec![Stick {
            name: "Pad".into(),
            port: "Right",
            joystick_mode: bindings(),
        }];
        let parsed: serde_json::Value = serde_json::from_str(&joymap(&sticks).unwrap()).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed[0]["port"], "Right");
    }
}
