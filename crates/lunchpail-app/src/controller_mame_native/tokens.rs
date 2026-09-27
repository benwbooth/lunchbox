//! Standard MAME input item tokens, from pinned src/emu/input.cpp.
//! Indices here are MAME-native identities, not physical or SDL indices.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Item {
    Button { number: u8 },
    Hat { number: u8, direction: Direction },
    AxisHalf { axis: Axis, positive: bool },
    Start,
    Select,
}

impl Item {
    pub(crate) fn token(self, native_device_number: u8) -> Result<String> {
        ensure!(
            native_device_number > 0,
            "MAME native device numbers are one-based"
        );
        let item = match self {
            Self::Button { number } => {
                ensure!(
                    (1..=32).contains(&number),
                    "MAME standard button number must be 1–32"
                );
                format!("BUTTON{number}")
            }
            Self::Hat { number, direction } => {
                ensure!(
                    (1..=4).contains(&number),
                    "MAME standard hat number must be 1–4"
                );
                let direction = match direction {
                    Direction::Up => "UP",
                    Direction::Down => "DOWN",
                    Direction::Left => "LEFT",
                    Direction::Right => "RIGHT",
                };
                format!("HAT{number}{direction}")
            }
            Self::AxisHalf { axis, positive } => {
                let axis = match axis {
                    Axis::X => "XAXIS",
                    Axis::Y => "YAXIS",
                    Axis::Z => "ZAXIS",
                    Axis::Rx => "RXAXIS",
                    Axis::Ry => "RYAXIS",
                    Axis::Rz => "RZAXIS",
                    Axis::Slider1 => "SLIDER1",
                    Axis::Slider2 => "SLIDER2",
                };
                // Explicit SWITCH class requests a digital half-axis from the
                // native absolute item; threshold settings remain launch-owned.
                format!("{axis}_{}_SWITCH", if positive { "POS" } else { "NEG" })
            }
            Self::Start => "START".into(),
            Self::Select => "SELECT".into(),
        };
        Ok(format!("JOYCODE_{native_device_number}_{item}"))
    }
}
