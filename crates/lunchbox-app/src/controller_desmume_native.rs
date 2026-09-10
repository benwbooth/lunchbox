//! DeSmuME native `config` JOYKEYS mappings for the GTK frontend, not
//! libretro bindings.
//!
//! Functional contract pinned to TASEmulators/desmume
//! b3915949700be824253a35affa7f7b8248e84e46 (posix frontend):
//! - `frontend/posix/shared/ctrlssdl.cpp` — joypad key codes are 4-hex-digit
//!   values: `(device & 15) << 12 | type << 8 | index`, with type 0=Axis,
//!   1=Hat, 2=Button. Axis halves are `2*axis` (negative) and `2*axis + 1`
//!   (positive); hat directions are `4*hat + (0=right,1=left,2=up,3=down)`;
//!   buttons use the plain SDL button index. The device digit is the SDL
//!   enumeration index. Axis keys engage at |value| >> 14 (16384).
//! - `frontend/posix/shared/desmume_config.cpp` — the keyfile holds a
//!   `[JOYKEYS]` section with one integer per `key_names[]` entry (A, B,
//!   Select, Start, Right, Left, Up, Down, R, L, X, Y, Debug, Boost, Lid) at
//!   `$XDG_CONFIG_HOME/desmume/config`, so a private XDG_CONFIG_HOME isolates
//!   every read and write from the user's own configuration.
use anyhow::{Result, ensure};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// key_names[] order from ctrlssdl.cpp; the adapter maps the first twelve.
pub(crate) const KEYS: [(&str, &str); 12] = [
    ("a", "A"),
    ("b", "B"),
    ("select", "Select"),
    ("start", "Start"),
    ("right", "Right"),
    ("left", "Left"),
    ("up", "Up"),
    ("down", "Down"),
    ("r", "R"),
    ("l", "L"),
    ("x", "X"),
    ("y", "Y"),
];

/// Axis engagement threshold: |value| >> 14 must be nonzero.
pub(crate) const DIGITAL_THRESHOLD: i32 = 16384;

const TYPE_AXIS: u32 = 0;
const TYPE_HAT: u32 = 1;
const TYPE_BUTTON: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Axis { index: u32, positive: bool },
    Hat { index: u32, direction: u8 },
    Button(u32),
}

impl Binding {
    pub(crate) fn code(&self, device: u32) -> Result<u32> {
        ensure_device(device)?;
        let (kind, index) = match *self {
            Binding::Button(button) => (TYPE_BUTTON, button),
            Binding::Axis { index, positive } => (TYPE_AXIS, index * 2 + u32::from(positive)),
            Binding::Hat { index, direction } => {
                anyhow::ensure!(
                    matches!(direction, 1 | 2 | 4 | 8),
                    "DeSmuME hat direction must be a cardinal SDL mask"
                );
                // SDL masks: right=2, left=8, up=1, down=4 → low bits in
                // ctrlssdl.cpp's order right, left, up, down.
                let low = match direction {
                    2 => 0,
                    8 => 1,
                    1 => 2,
                    _ => 3,
                };
                (TYPE_HAT, index * 4 + low)
            }
        };
        ensure!(
            index < 0x100,
            "DeSmuME joypad code overflows the index field"
        );
        Ok(((device & 15) << 12) | (kind << 8) | index)
    }
}

fn ensure_device(device: u32) -> Result<()> {
    anyhow::ensure!(
        device < 16,
        "DeSmuME joypad codes carry the device index in four bits"
    );
    Ok(())
}

/// Render the complete private keyfile: the twelve mapped controls plus
/// explicit disabled (0xFFFF) entries for Debug, Boost and Lid.
pub(crate) fn config(device: u32, bindings: &[(String, Binding)]) -> Result<String> {
    let mut result = String::from("[JOYKEYS]\n");
    for (_, output) in KEYS {
        let binding = bindings
            .iter()
            .find(|(name, _)| name == output)
            .map(|(_, binding)| *binding);
        let value = match binding {
            Some(binding) => binding.code(device)?,
            None => 0xFFFF,
        };
        result.push_str(&format!("{output}={value}\n"));
    }
    for extra in ["Debug", "Boost", "Lid"] {
        result.push_str(&format!("{extra}=65535\n"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_use_the_pinned_encoding() {
        assert_eq!(Binding::Button(5).code(0).unwrap(), 0x0205);
        assert_eq!(Binding::Button(0).code(1).unwrap(), 0x1200); // device digit 1, button type
        assert_eq!(
            Binding::Axis {
                index: 0,
                positive: true
            }
            .code(0)
            .unwrap(),
            0x0001
        );
        assert_eq!(
            Binding::Axis {
                index: 1,
                positive: false
            }
            .code(2)
            .unwrap(),
            0x2002
        );
        assert_eq!(
            Binding::Hat {
                index: 0,
                direction: 2
            }
            .code(0)
            .unwrap(),
            0x0100
        );
        assert_eq!(
            Binding::Hat {
                index: 0,
                direction: 8
            }
            .code(0)
            .unwrap(),
            0x0101
        );
        assert_eq!(
            Binding::Hat {
                index: 0,
                direction: 1
            }
            .code(0)
            .unwrap(),
            0x0102
        );
        assert_eq!(
            Binding::Hat {
                index: 0,
                direction: 4
            }
            .code(0)
            .unwrap(),
            0x0103
        );
        assert!(
            Binding::Hat {
                index: 0,
                direction: 3
            }
            .code(0)
            .is_err()
        );
        assert!(Binding::Button(0).code(16).is_err());
        assert!(Binding::Button(0x100).code(0).is_err());
    }

    #[test]
    fn config_maps_the_twelve_controls_and_disables_extras() {
        let bindings = vec![
            ("A".to_string(), Binding::Button(0)),
            (
                "Up".to_string(),
                Binding::Axis {
                    index: 1,
                    positive: false,
                },
            ),
        ];
        let text = config(0, &bindings).unwrap();
        assert!(text.starts_with("[JOYKEYS]\n"));
        assert!(text.contains("A=512\n")); // type button (2) << 8
        assert!(text.contains("Up=2\n"));
        assert!(text.contains("B=65535\n"));
        assert!(text.contains("Debug=65535\n"));
        assert!(text.contains("Lid=65535\n"));
        assert_eq!(text.lines().count(), 1 + KEYS.len() + 3);
    }
}
