//! jgenesis v0.14.1 native `jgenesis-config.toml` input mappings, not
//! libretro bindings.
//!
//! Functional contract pinned to jsgroth/jgenesis
//! cbe7f129e3f5c805a2a2e4318981834192116e90 (v0.14.1):
//! - `frontend/jgenesis-native-config/src/paths.rs`: the config is
//!   `~/.config/jgenesis/jgenesis-config.toml`.
//! - `frontend/jgenesis-native-config/src/input/mappings.rs`:
//!   `GenesisInputMapping` holds `p1`/`p2` (plus turbo twins) of
//!   `GenesisControllerMapping` whose fields are up/left/right/down, a, b,
//!   c, x, y, z, start, mode. Every struct is `#[serde(default)]`, so a
//!   partial TOML containing only `[input.genesis.p1]` merges safely.
//! - `frontend/jgenesis-native-config/src/input/serialize.rs`: each
//!   `GenericInput` serializes as an inline table
//!   `{ type = "Gamepad", gamepad_idx = N, action = "..." }` where action
//!   strings come from `GamepadAction::from_str`: `Button N`,
//!   `Axis N positive|negative`, `Hat N up|down|left|right`.
//! - `frontend/jgenesis-native-driver/src/input.rs`: `gamepad_idx` is the
//!   position of the device among open joysticks in SDL enumeration order
//!   (`regenerate_id_maps`), and button/axis/hat indices are raw SDL
//!   joystick indices — the same kernel-order semantics our SDL2 probe
//!   captures when a single qualifying device pins the index to zero.
use anyhow::{Result, ensure};

/// Genesis target controls: layout id -> jgenesis mapping field.
pub(crate) const CONTROLS: [(&str, &str); 12] = [
    ("a", "a"),
    ("b", "b"),
    ("c", "c"),
    ("x", "x"),
    ("y", "y"),
    ("z", "z"),
    ("start", "start"),
    ("mode", "mode"),
    ("up", "up"),
    ("down", "down"),
    ("left", "left"),
    ("right", "right"),
];

/// One raw host input backing, in SDL raw joystick terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
    Hat { hat: u32, direction: u8 },
}

impl Binding {
    /// The GenericInput inline-table TOML for gamepad zero.
    pub(crate) fn toml(&self) -> Result<String> {
        let action = match *self {
            Binding::Button(index) => {
                ensure!(index < 32, "jgenesis button index overflows its byte");
                format!("Button {index}")
            }
            Binding::Axis { index, positive } => {
                ensure!(index < 32, "jgenesis axis index overflows its byte");
                format!(
                    "Axis {index} {}",
                    if positive { "positive" } else { "negative" }
                )
            }
            Binding::Hat { hat, direction } => {
                let direction = match direction {
                    1 => "up",
                    2 => "right",
                    4 => "down",
                    8 => "left",
                    _ => anyhow::bail!("jgenesis hat direction must be a cardinal SDL mask"),
                };
                format!("Hat {hat} {direction}")
            }
        };
        Ok(format!(
            "{{ type = \"Gamepad\", gamepad_idx = 0, action = \"{action}\" }}"
        ))
    }
}

/// Render the partial TOML: only the genesis p1/p2 input sections. Every
/// other key in jgenesis's serde-default config stays at its default.
fn write_player(out: &mut String, controls: &[(u8, &str, Binding)]) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for (field, binding) in controls
        .iter()
        .map(|(port, field, binding)| (*field, *binding))
    {
        ensure!(
            seen.insert(field),
            "jgenesis mapping field {} appears twice",
            field
        );
        ensure!(
            CONTROLS.iter().any(|(id, _)| *id == field),
            "jgenesis target {} is outside the six-button contract",
            field
        );
        out.push_str(&format!("{field} = {}\n", binding.toml()?));
    }
    Ok(())
}

/// Render the partial TOML: only the genesis p1 input section. Every other
/// key in jgenesis's serde-default config stays at its default.
pub(crate) fn config_toml(p1: &[(u8, &str, Binding)]) -> Result<String> {
    let mut result = String::from("[input.genesis.p1]\n");
    write_player(&mut result, p1)?;
    result.push('\n');
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_render_the_pinned_inline_tables() {
        assert_eq!(
            Binding::Button(5).toml().unwrap(),
            "{ type = \"Gamepad\", gamepad_idx = 0, action = \"Button 5\" }"
        );
        assert_eq!(
            Binding::Axis {
                index: 0,
                positive: false
            }
            .toml()
            .unwrap(),
            "{ type = \"Gamepad\", gamepad_idx = 0, action = \"Axis 0 negative\" }"
        );
        assert_eq!(
            Binding::Hat {
                hat: 0,
                direction: 8
            }
            .toml()
            .unwrap(),
            "{ type = \"Gamepad\", gamepad_idx = 0, action = \"Hat 0 left\" }"
        );
        assert!(
            Binding::Hat {
                hat: 0,
                direction: 3
            }
            .toml()
            .is_err()
        );
        assert!(Binding::Button(64).toml().is_err());
    }

    #[test]
    fn partial_toml_targets_only_genesis_p1() {
        let p1: Vec<(u8, &str, Binding)> = vec![
            (1, "a", Binding::Button(5)),
            (
                1,
                "up",
                Binding::Hat {
                    hat: 0,
                    direction: 1,
                },
            ),
        ];
        let text = config_toml(&p1).unwrap();
        assert!(text.starts_with("[input.genesis.p1]\n"));
        assert!(
            text.contains("a = { type = \"Gamepad\", gamepad_idx = 0, action = \"Button 5\" }\n")
        );
        assert!(
            text.contains("up = { type = \"Gamepad\", gamepad_idx = 0, action = \"Hat 0 up\" }\n")
        );
        assert!(!text.contains("[input.genesis.p2]"));
    }
}
