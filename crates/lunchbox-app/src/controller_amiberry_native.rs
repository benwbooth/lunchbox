//! Amiberry native SDL gamepad mappings for standalone Amiga joystick ports.
//!
//! This is a source-backed configuration writer, not a claim that an Amiberry
//! binary or an Amiga game has been exercised.  The source pin is
//! BlitterStudio/amiberry 06ff25093b620deef734a395189a1c564ed8beac.
//!
//! Amiberry uses SDL's `gamecontrollerdb.txt` grammar for the physical-to-
//! logical translation and the `.uae` file's `joyportN=joyN`/`joyportNmode`
//! options for port selection.  Keeping those two layers separate is
//! important: a `.uae` file cannot safely contain guessed raw SDL button
//! numbers when SDL's controller database resolves the logical gamepad.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "06ff25093b620deef734a395189a1c564ed8beac";
pub(crate) const PROFILE_ID: &str = "amiberry:standalone-amiga-joystick";

/// The classic Amiga joystick controls represented by Amiberry's normal
/// `gamepad` port mode.  Fire, second-fire and third-fire are SDL south, east
/// and west respectively; the source maps those to Joy1/Joy2 button events.
pub(crate) const CONTROLS: [(&str, &str); 7] = [
    ("up", "dpup"),
    ("down", "dpdown"),
    ("left", "dpleft"),
    ("right", "dpright"),
    ("fire", "a"),
    ("fire2", "b"),
    ("fire3", "x"),
];

/// One raw SDL joystick control in SDL's gamecontroller database syntax.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
    Hat { index: u32, mask: u8 },
}

impl Binding {
    fn sdl_spec(self) -> Result<String> {
        match self {
            Self::Button(index) => {
                ensure!(index < 256, "Amiberry SDL button index is out of range");
                Ok(format!("b{index}"))
            }
            Self::Axis { index, positive } => {
                ensure!(index < 256, "Amiberry SDL axis index is out of range");
                Ok(format!("a{index}{}", if positive { "+" } else { "-" }))
            }
            Self::Hat { index, mask } => {
                ensure!(index < 64, "Amiberry SDL hat index is out of range");
                ensure!(
                    matches!(mask, 1 | 2 | 4 | 8),
                    "Amiberry SDL hat mask is not cardinal"
                );
                Ok(format!("h{index}.{mask}"))
            }
        }
    }
}

fn valid_text(value: &str, what: &str) -> Result<()> {
    ensure!(!value.is_empty(), "Amiberry {what} is empty");
    ensure!(value.len() <= 256, "Amiberry {what} is too long");
    ensure!(
        !value
            .chars()
            .any(|ch| ch == ',' || ch == '\n' || ch == '\r' || ch.is_control()),
        "Amiberry {what} contains a gamecontroller-db separator or control character"
    );
    Ok(())
}

fn valid_guid(guid: &str) -> Result<()> {
    ensure!(
        guid.len() == 32 && guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Amiberry SDL GUID must be 32 hexadecimal characters"
    );
    Ok(())
}

/// Render one SDL gamecontroller database record.  `bindings` uses the
/// target names in [`CONTROLS`], not SDL button numbers.  Directions are
/// either all four digital fields (buttons/hats) or two opposite axis pairs;
/// mixed or incomplete direction descriptions are rejected.
pub(crate) fn gamecontrollerdb_line(
    guid: &str,
    name: &str,
    platform: &str,
    bindings: &BTreeMap<String, Binding>,
) -> Result<String> {
    valid_guid(guid)?;
    valid_text(name, "controller name")?;
    valid_text(platform, "platform")?;
    ensure!(
        bindings.len() == CONTROLS.len(),
        "Amiberry needs all seven classic joystick controls"
    );
    let mut used = BTreeSet::new();
    for (target, _) in CONTROLS {
        ensure!(
            bindings.contains_key(target),
            "Amiberry control {target} is absent"
        );
        let binding = bindings[target];
        ensure!(used.insert(binding), "Amiberry reuses one physical input");
    }

    let directions = ["up", "down", "left", "right"];
    let direction_values: Vec<_> = directions.iter().map(|name| bindings[*name]).collect();
    let axis_pair = |negative: Binding, positive: Binding| match (negative, positive) {
        (
            Binding::Axis {
                index: first,
                positive: false,
            },
            Binding::Axis {
                index: second,
                positive: true,
            },
        ) if first == second => Some(first),
        _ => None,
    };
    let digital = direction_values
        .iter()
        .all(|value| matches!(value, Binding::Button(_) | Binding::Hat { .. }));
    let analog = axis_pair(direction_values[2], direction_values[3]).is_some()
        && axis_pair(direction_values[0], direction_values[1]).is_some()
        && axis_pair(direction_values[0], direction_values[1])
            != axis_pair(direction_values[2], direction_values[3]);
    ensure!(
        digital || analog,
        "Amiberry directions must be one digital set or two opposite SDL axes"
    );

    let mut fields: BTreeMap<&str, String> = BTreeMap::new();
    if digital {
        for (target, output) in CONTROLS {
            fields.insert(output, bindings[target].sdl_spec()?);
        }
    } else {
        fields.insert("leftx", bindings["left"].sdl_spec()?);
        fields.insert("lefty", bindings["up"].sdl_spec()?);
        for (target, output) in [("fire", "a"), ("fire2", "b"), ("fire3", "x")] {
            fields.insert(output, bindings[target].sdl_spec()?);
        }
    }

    let mut output = format!("{},{},platform:{platform}", guid.to_ascii_lowercase(), name);
    for (field, spec) in fields {
        output.push(',');
        output.push_str(field);
        output.push(':');
        output.push_str(&spec);
    }
    output.push('\n');
    Ok(output)
}

/// Render the `.uae` options selecting one SDL joystick for a classic port.
/// Amiberry's source uses zero-based `joyN` IDs and only the first two ports
/// are normal joystick/gamepad ports; ports 2 and 3 are parallel adapters.
pub(crate) fn uae_port_fragment(port: u8, device_index: u8, friendly_name: &str) -> Result<String> {
    ensure!(
        port < 2,
        "Amiberry classic gamepad mode supports ports 0 and 1"
    );
    valid_text(friendly_name, "friendly device name")?;
    Ok(format!(
        "joyport{port}=joy{device_index}\njoyport{port}mode=gamepad\njoyportfriendlyname{port}={friendly_name}\njoyportname{port}=JOY{device_index}\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digital() -> BTreeMap<String, Binding> {
        [
            ("up", Binding::Hat { index: 0, mask: 1 }),
            ("down", Binding::Hat { index: 0, mask: 4 }),
            ("left", Binding::Hat { index: 0, mask: 8 }),
            ("right", Binding::Hat { index: 0, mask: 2 }),
            ("fire", Binding::Button(0)),
            ("fire2", Binding::Button(1)),
            ("fire3", Binding::Button(2)),
        ]
        .into_iter()
        .map(|(name, binding)| (name.to_owned(), binding))
        .collect()
    }

    #[test]
    fn gamecontroller_db_uses_amiberry_logical_fields() {
        let text = gamecontrollerdb_line(
            "0123456789abcdef0123456789abcdef",
            "Pad One",
            "Linux",
            &digital(),
        )
        .unwrap();
        assert!(text.contains("dpup:h0.1"));
        assert!(text.contains("a:b0"));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn uae_fragment_selects_zero_based_joy_and_gamepad_mode() {
        let text = uae_port_fragment(1, 3, "Pad One").unwrap();
        assert_eq!(
            text,
            "joyport1=joy3\njoyport1mode=gamepad\njoyportfriendlyname1=Pad One\njoyportname1=JOY3\n"
        );
        assert!(uae_port_fragment(2, 0, "Pad").is_err());
    }

    #[test]
    fn incomplete_or_mixed_directions_fail_closed() {
        let mut map = digital();
        map.remove("right");
        assert!(
            gamecontrollerdb_line("0123456789abcdef0123456789abcdef", "Pad", "Linux", &map)
                .is_err()
        );
        let mut mixed = digital();
        mixed.insert(
            "up".into(),
            Binding::Axis {
                index: 1,
                positive: false,
            },
        );
        assert!(
            gamecontrollerdb_line("0123456789abcdef0123456789abcdef", "Pad", "Linux", &mixed)
                .is_err()
        );
    }
}
