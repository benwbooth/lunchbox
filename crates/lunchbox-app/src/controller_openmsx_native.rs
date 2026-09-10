//! openMSX native `msxjoystickN_config` settings mappings, not libretro
//! bindings.
//!
//! Functional contract pinned to openMSX/openMSX
//! 25179d6b8d5ec69ad68252f3854721c9a02594eb:
//! - `src/input/MSXJoystick.cc` — each MSX joystick reads the TCL setting
//!   `msxjoystick1_config`/`msxjoystick2_config`, a list of key/value pairs
//!   with keys UP, DOWN, LEFT, RIGHT, A, B whose values are lists of event
//!   specs validated by `parseBooleanInput`.
//! - `src/events/BooleanInput.cc` — event specs are `joyN buttonK`,
//!   `joyN hatK up|right|down|left` and `joyN +axisK`/`joyN -axisK` with
//!   indices 0..255; `joyN` is the 1-based host joystick number
//!   (`JoystickId::parse`).
//! - `src/config/SettingsConfig.cc` — settings persist as
//!   `<!DOCTYPE settings SYSTEM 'settings.dtd'>` with nested `<settings>`
//!   elements holding `<setting id="...">value</setting>`; `-setting <file>`
//!   loads that file instead of the user's own settings.xml and auto-saves
//!   back to it on exit.
//! - `src/CommandLineParser.cc` — `-command <tcl>` executes TCL at startup
//!   (used to plug the second joystick port).
//! - `src/input/JoystickManager.cc` — the per-joystick dead zone defaults to
//!   25 percent (≈8192 in SDL axis units).
use anyhow::{Result, ensure};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Default dead zone (25%) in SDL axis units.
pub(crate) const DIGITAL_THRESHOLD: i32 = 8192;

/// MSX joystick target controls: layout id -> settings dict key.
pub(crate) const CONTROLS: [(&str, &str, bool); 6] = [
    ("up", "UP", true),
    ("down", "DOWN", true),
    ("left", "LEFT", true),
    ("right", "RIGHT", true),
    ("a", "A", true),
    ("b", "B", false),
];

/// One event spec inside a dict value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, negative: bool },
    Hat { index: u32, direction: u8 },
}

impl Binding {
    pub(crate) fn spec(&self, joystick: u32) -> Result<String> {
        ensure!(
            joystick >= 1,
            "openMSX event specs use 1-based joystick numbers"
        );
        Ok(match *self {
            Binding::Button(button) => format!("joy{joystick} button{button}"),
            Binding::Axis { index, negative } => format!(
                "joy{joystick} {}axis{index}",
                if negative { "-" } else { "+" }
            ),
            Binding::Hat { index, direction } => {
                let direction = match direction {
                    1 => "up",
                    2 => "right",
                    4 => "down",
                    8 => "left",
                    _ => anyhow::bail!("openMSX hat direction must be a cardinal SDL mask"),
                };
                format!("joy{joystick} hat{index} {direction}")
            }
        })
    }
}

/// Render one MSX joystick's config dict: `KEY { specs } ...`.
pub(crate) fn dict(bindings: &[(String, Binding)], joystick: u32) -> Result<String> {
    let mut result = String::new();
    for (_, key, required) in CONTROLS {
        result.push_str(key);
        result.push_str(" {");
        let mut any = false;
        for (name, binding) in bindings {
            if name == key {
                result.push_str(&binding.spec(joystick)?);
                any = true;
            }
        }
        ensure!(
            any || !required,
            "openMSX dict is missing the required {key} binding"
        );
        result.push_str(" } ");
    }
    Ok(result.trim_end().to_owned())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render the complete private settings file for the given per-player dicts.
pub(crate) fn settings_xml(dicts: &[(usize, &str)]) -> String {
    let mut result =
        String::from("<!DOCTYPE settings SYSTEM 'settings.dtd'>\n<settings>\n<settings>\n");
    for (player, dict) in dicts {
        result.push_str(&format!(
            "<setting id=\"msxjoystick{player}_config\">{}</setting>\n",
            escape(dict)
        ));
    }
    result.push_str("</settings>\n</settings>\n");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_use_the_pinned_grammar() {
        assert_eq!(Binding::Button(3).spec(1).unwrap(), "joy1 button3");
        assert_eq!(
            Binding::Axis {
                index: 0,
                negative: true
            }
            .spec(2)
            .unwrap(),
            "joy2 -axis0"
        );
        assert_eq!(
            Binding::Axis {
                index: 1,
                negative: false
            }
            .spec(1)
            .unwrap(),
            "joy1 +axis1"
        );
        assert_eq!(
            Binding::Hat {
                index: 0,
                direction: 1
            }
            .spec(1)
            .unwrap(),
            "joy1 hat0 up"
        );
        assert_eq!(
            Binding::Hat {
                index: 2,
                direction: 8
            }
            .spec(3)
            .unwrap(),
            "joy3 hat2 left"
        );
        assert!(
            Binding::Hat {
                index: 0,
                direction: 3
            }
            .spec(1)
            .is_err()
        );
        assert!(Binding::Button(0).spec(0).is_err());
    }

    #[test]
    fn dict_requires_every_mandatory_key() {
        let bindings = vec![
            (
                "UP".to_string(),
                Binding::Hat {
                    index: 0,
                    direction: 1,
                },
            ),
            (
                "DOWN".to_string(),
                Binding::Hat {
                    index: 0,
                    direction: 4,
                },
            ),
            (
                "LEFT".to_string(),
                Binding::Hat {
                    index: 0,
                    direction: 8,
                },
            ),
            (
                "RIGHT".to_string(),
                Binding::Hat {
                    index: 0,
                    direction: 2,
                },
            ),
            ("A".to_string(), Binding::Button(0)),
        ];
        assert_eq!(
            dict(&bindings, 1).unwrap(),
            "UP {joy1 hat0 up } DOWN {joy1 hat0 down } LEFT {joy1 hat0 left } RIGHT {joy1 hat0 right } A {joy1 button0 } B { }"
        );
        let missing = bindings[..4].to_vec();
        assert!(dict(&missing, 1).is_err());
    }

    #[test]
    fn settings_xml_wraps_dicts_in_the_pinned_layout() {
        let text = settings_xml(&[(1, "A {joy1 button0}")]);
        assert!(
            text.starts_with("<!DOCTYPE settings SYSTEM 'settings.dtd'>\n<settings>\n<settings>\n")
        );
        assert!(text.contains("<setting id=\"msxjoystick1_config\">A {joy1 button0}</setting>\n"));
        assert!(text.ends_with("</settings>\n</settings>\n"));
        let escaped = settings_xml(&[(2, "UP {a<b&c}")]);
        assert!(escaped.contains("&lt;") && escaped.contains("&amp;"));
    }
}
