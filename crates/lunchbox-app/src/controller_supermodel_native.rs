//! Supermodel native Linux input overlay.
//!
//! Pinned source: trzy/Supermodel commit
//! 24d2ffcfc7f14229337f05f4920fe26b56633d9d. `Src/Inputs/InputSystem.cpp`
//! parses `JOY1_BUTTON1`, `JOY1_XAXIS_NEG`, `JOY1_UP`, and related tokens;
//! `Src/OSD/SDL/Main.cpp` stores them as `Input*` values in the `[ Global ]`
//! section of `Supermodel.ini`. This module only writes explicitly supplied
//! arcade action names; it never guesses a game's control deck.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button {
        joystick: u8,
        button: u8,
    },
    Axis {
        joystick: u8,
        axis: Axis,
        positive: Option<bool>,
    },
    Pov {
        joystick: u8,
        pov: u8,
        direction: PovDirection,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Axis {
    X,
    Y,
    Z,
    Rx,
    Ry,
    Rz,
    Slider1,
    Slider2,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum PovDirection {
    Up,
    Down,
    Left,
    Right,
}

impl Binding {
    pub(crate) fn token(self) -> Result<String> {
        match self {
            Self::Button { joystick, button } => {
                ensure!(
                    (1..=8).contains(&joystick) && (1..=32).contains(&button),
                    "Supermodel joystick button is out of range"
                );
                Ok(format!("JOY{joystick}_BUTTON{button}"))
            }
            Self::Axis {
                joystick,
                axis,
                positive,
            } => {
                ensure!(
                    (1..=8).contains(&joystick),
                    "Supermodel joystick is out of range"
                );
                let name = match axis {
                    Axis::X => "X",
                    Axis::Y => "Y",
                    Axis::Z => "Z",
                    Axis::Rx => "RX",
                    Axis::Ry => "RY",
                    Axis::Rz => "RZ",
                    Axis::Slider1 => "S1",
                    Axis::Slider2 => "S2",
                };
                let suffix = match positive {
                    None => "AXIS",
                    Some(true) => "AXIS_POS",
                    Some(false) => "AXIS_NEG",
                };
                Ok(format!("JOY{joystick}_{name}{suffix}"))
            }
            Self::Pov {
                joystick,
                pov,
                direction,
            } => {
                ensure!(
                    (1..=8).contains(&joystick) && (1..=4).contains(&pov),
                    "Supermodel POV is out of range"
                );
                let direction = match direction {
                    PovDirection::Up => "UP",
                    PovDirection::Down => "DOWN",
                    PovDirection::Left => "LEFT",
                    PovDirection::Right => "RIGHT",
                };
                Ok(format!("JOY{joystick}_POV{pov}_{direction}"))
            }
        }
    }
}

/// Replace explicitly named `[ Global ]` input keys while preserving all
/// machine-specific sections and unrelated settings.
pub(crate) fn patch_global_ini(
    baseline: &[u8],
    mappings: &[(String, Vec<Binding>)],
) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "Supermodel configuration is too large"
    );
    let text =
        String::from_utf8(baseline.to_vec()).context("Supermodel configuration is not UTF-8")?;
    let names = mappings
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<BTreeSet<_>>();
    for (name, bindings) in mappings {
        ensure!(
            !name.is_empty()
                && name.starts_with("Input")
                && name.bytes().all(|b| b.is_ascii_alphanumeric()),
            "Supermodel action name must be an Input* identifier"
        );
        ensure!(
            !bindings.is_empty() && bindings.len() <= 8,
            "Supermodel action needs one to eight bindings"
        );
        for binding in bindings {
            let _ = binding.token()?;
        }
    }
    let mut output = Vec::new();
    let mut global = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            global = trimmed[1..trimmed.len() - 1].trim() == "Global";
        }
        let key = if global {
            trimmed.split_once('=').map(|(key, _)| key.trim())
        } else {
            None
        };
        if key.is_some_and(|key| names.contains(key)) {
            continue;
        }
        output.push(line);
    }
    let mut result = output.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str("[ Global ]\n");
    for (name, bindings) in mappings {
        let values = bindings
            .iter()
            .map(|binding| binding.token())
            .collect::<Result<Vec<_>>>()?
            .join(",");
        result.push_str(&format!("{name} = \"{values}\"\n"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_pinned_supermodel_tokens() {
        assert_eq!(
            Binding::Button {
                joystick: 1,
                button: 9
            }
            .token()
            .unwrap(),
            "JOY1_BUTTON9"
        );
        assert_eq!(
            Binding::Axis {
                joystick: 1,
                axis: Axis::X,
                positive: Some(false)
            }
            .token()
            .unwrap(),
            "JOY1_XAXIS_NEG"
        );
        assert_eq!(
            Binding::Pov {
                joystick: 2,
                pov: 1,
                direction: PovDirection::Up
            }
            .token()
            .unwrap(),
            "JOY2_POV1_UP"
        );
    }
}
