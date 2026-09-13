//! BlastEm native tern-config bindings for the SDL2 frontend, not libretro
//! bindings.
//!
//! Functional contract pinned to the BlastEm sources mirrored at
//! libretro/blastem b4d75247ebad8852fd9bc385b423df704c6c5af5:
//! - `config.c serialize_config` — the configuration is the tern tree
//!   serialized as nested `name { ... }` blocks of `key value` lines.
//! - `bindings.c get_binding_node_for_pad` — pad bindings live under
//!   `bindings pads <idx>` where idx is the SDL device index (GUID and
//!   controller-type fallbacks follow, then `default`).
//! - `bindings.c handle_joy_added` — children are `dpads <n>
//!   up|down|left|right <target>` (host hat/dpad directions), `buttons
//!   <n> <target>` and `axes <n>.positive|.negative <target>`;
//!   `bindings.c process_pad_axis` parses the axis modifiers.
//! - `bindings.c parse_binding_target`/`get_pad_buttons` — targets are
//!   `gamepads.<1-8>.<button>` with buttons up/down/left/right, a, b, c,
//!   x, y, z, start and mode.
//! - `paths.c get_config_dir` — the configuration directory is
//!   `$HOME/.config/blastem`, so a private HOME isolates every read and write;
//!   `blastem.c main` takes the ROM as a bare argument.
use anyhow::{Result, ensure};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Genesis target controls: layout id -> binding target button.
pub(crate) const CONTROLS: [(&str, &str); 12] = [
    ("up", "up"),
    ("down", "down"),
    ("left", "left"),
    ("right", "right"),
    ("a", "a"),
    ("b", "b"),
    ("c", "c"),
    ("x", "x"),
    ("y", "y"),
    ("z", "z"),
    ("start", "start"),
    ("mode", "mode"),
];

/// Conservative digital-axis travel check; BlastEm's own dead zone comes
/// from SDL-derived controller info and is not exposed as a constant.
pub(crate) const DIGITAL_THRESHOLD: i32 = 16384;

/// One host input binding in BlastEm's vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
    HatDirection { hat: u32, direction: u8 },
}

impl Binding {
    /// Render the tern key/value pair feeding `gamepads.<port>.<button>`.
    pub(crate) fn lines(&self, target: &str) -> Vec<String> {
        match *self {
            Binding::Button(button) => {
                vec![format!("buttons {{ {button} {target} }}")]
            }
            Binding::Axis { index, positive } => {
                let half = if positive { "positive" } else { "negative" };
                vec![format!("axes {{ {index}.{half} {target} }}")]
            }
            Binding::HatDirection { hat, direction } => {
                let dir = match direction {
                    1 => "up",
                    2 => "right",
                    4 => "down",
                    8 => "left",
                    _ => "center",
                };
                if dir == "center" {
                    return Vec::new();
                }
                vec![format!("dpads {{ {hat} {{ {dir} {target} }} }}")]
            }
        }
    }
}

fn escape(value: &str) -> Result<String> {
    ensure!(
        !value
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b == b'{' || b == b'}'),
        "BlastEm tern values cannot contain whitespace or braces"
    );
    Ok(value.to_owned())
}

/// Render one pad's nested tern block for the given per-control bindings.
/// `controls` pairs the layout control with its resolved binding.
pub(crate) fn pad_block(device: u32, port: u8, controls: &[(&str, Binding)]) -> Result<String> {
    ensure!(
        (1..=8).contains(&port),
        "BlastEm targets gamepads one through eight"
    );
    let mut buttons = std::collections::BTreeMap::new();
    let mut axes = std::collections::BTreeMap::new();
    let mut dpads = std::collections::BTreeMap::new();
    let mut inputs = std::collections::BTreeSet::new();
    for (control, binding) in controls {
        let button = CONTROLS
            .iter()
            .find(|(id, _)| id == control)
            .map(|(_, target)| *target)
            .ok_or_else(|| anyhow::anyhow!("Unknown BlastEm gameplay control {control}"))?;
        ensure!(
            inputs.insert(*binding),
            "BlastEm physical input has multiple gameplay owners"
        );
        let target = escape(&format!("gamepads.{port}.{button}"))?;
        match *binding {
            Binding::Button(index) => {
                buttons.insert(format!("{index}"), target);
            }
            Binding::Axis { index, positive } => {
                let half = if positive { "positive" } else { "negative" };
                axes.insert(format!("{index}.{half}"), target);
            }
            Binding::HatDirection { hat, direction } => {
                let dir = match direction {
                    1 => "up",
                    2 => "right",
                    4 => "down",
                    8 => "left",
                    _ => anyhow::bail!("BlastEm hat direction must be a cardinal SDL mask"),
                };
                dpads.insert((format!("{hat}"), dir.to_owned()), target);
            }
        }
    }
    let mut result = format!("bindings {{\n\tpads {{\n\t\t{device} {{\n");
    if !dpads.is_empty() {
        result.push_str("\t\t\tdpads {\n");
        for hat in dpads
            .keys()
            .map(|(hat, _)| hat)
            .collect::<std::collections::BTreeSet<_>>()
        {
            result.push_str(&format!("\t\t\t\t{hat} {{\n"));
            for ((hat_key, dir), target) in &dpads {
                if hat_key == hat {
                    result.push_str(&format!("\t\t\t\t\t{dir} {target}\n"));
                }
            }
            result.push_str("\t\t\t\t}\n");
        }
        result.push_str("\t\t\t}\n");
    }
    if !buttons.is_empty() {
        result.push_str("\t\t\tbuttons {\n");
        for (index, target) in &buttons {
            result.push_str(&format!("\t\t\t\t{index} {target}\n"));
        }
        result.push_str("\t\t\t}\n");
    }
    if !axes.is_empty() {
        result.push_str("\t\t\taxes {\n");
        for (index, target) in &axes {
            result.push_str(&format!("\t\t\t\t{index} {target}\n"));
        }
        result.push_str("\t\t\t}\n");
    }
    result.push_str("\t\t}\n\t}\n}\n");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_block_renders_the_pinned_tern_shape() {
        let controls = [
            (
                "up",
                Binding::HatDirection {
                    hat: 0,
                    direction: 1,
                },
            ),
            (
                "down",
                Binding::HatDirection {
                    hat: 0,
                    direction: 4,
                },
            ),
            ("a", Binding::Button(0)),
            ("b", Binding::Button(1)),
            (
                "left",
                Binding::Axis {
                    index: 0,
                    positive: false,
                },
            ),
        ];
        let text = pad_block(3, 2, &controls).unwrap();
        assert!(text.starts_with("bindings {\n\tpads {\n\t\t3 {\n"));
        assert!(text.contains("\t\t\tdpads {\n\t\t\t\t0 {\n"));
        assert!(text.contains("\t\t\t\t\tdown gamepads.2.down\n"));
        assert!(text.contains("\t\t\t\t\tup gamepads.2.up\n"));
        assert!(text.contains("\t\t\t\t}\n\t\t\t}\n"));
        assert!(text.contains("\t\t\tbuttons {\n\t\t\t\t0 gamepads.2.a\n\t\t\t\t1 gamepads.2.b\n"));
        assert!(text.contains("\t\t\taxes {\n\t\t\t\t0.negative gamepads.2.left\n\t\t\t}\n"));
        assert!(text.ends_with("\t\t}\n\t}\n}\n"));
        assert!(pad_block(0, 0, &controls).is_err());
        assert!(pad_block(0, 9, &controls).is_err());
    }

    #[test]
    fn diagonal_and_centered_hats_are_rejected_or_skipped() {
        assert!(
            Binding::HatDirection {
                hat: 0,
                direction: 3
            }
            .lines("gamepads.1.a")
            .is_empty()
        );
        let controls = [(
            "up",
            Binding::HatDirection {
                hat: 0,
                direction: 3,
            },
        )];
        assert!(pad_block(0, 1, &controls).is_err());
    }

    #[test]
    fn binding_lines_use_the_documented_keys() {
        assert_eq!(
            Binding::Button(2).lines("gamepads.1.a"),
            vec!["buttons { 2 gamepads.1.a }".to_string()]
        );
        assert_eq!(
            Binding::Axis {
                index: 1,
                positive: false
            }
            .lines("gamepads.1.left"),
            vec!["axes { 1.negative gamepads.1.left }".to_string()]
        );
    }

    #[test]
    fn pad_block_rejects_unknown_or_shared_controls() {
        assert!(pad_block(0, 1, &[("unknown", Binding::Button(0))]).is_err());
        assert!(
            pad_block(
                0,
                1,
                &[("a", Binding::Button(0)), ("b", Binding::Button(0))]
            )
            .is_err()
        );
    }
}
