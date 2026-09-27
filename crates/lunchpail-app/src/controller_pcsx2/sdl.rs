//! Current SDL3 tokens, from pinned SDLInputSource::ParseKeyString.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const BUTTONS: &[&str] = &[
    "FaceSouth",
    "FaceEast",
    "FaceWest",
    "FaceNorth",
    "Back",
    "Guide",
    "Start",
    "LeftStick",
    "RightStick",
    "LeftShoulder",
    "RightShoulder",
    "DPadUp",
    "DPadDown",
    "DPadLeft",
    "DPadRight",
    "Misc1",
    "Paddle1",
    "Paddle2",
    "Paddle3",
    "Paddle4",
    "Touchpad",
    "Misc2",
    "Misc3",
    "Misc4",
    "Misc5",
    "Misc6",
];
const AXES: &[&str] = &[
    "LeftX",
    "LeftY",
    "RightX",
    "RightY",
    "LeftTrigger",
    "RightTrigger",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AxisRange {
    Positive,
    Negative,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Direction {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Input {
    GamepadButton {
        index: u8,
    },
    GamepadAxis {
        index: u8,
        range: AxisRange,
    },
    JoystickButton {
        index: u16,
    },
    JoystickAxis {
        index: u16,
        range: AxisRange,
        inverted: bool,
    },
    Hat {
        index: u8,
        direction: Direction,
    },
    LargeMotor,
    SmallMotor,
    Haptic,
}

impl Input {
    /// PCSX2 SDL player ID, not an SDL instance ID or target PS2 pad slot.
    pub(crate) fn token(self, player_id: u8) -> Result<String> {
        let prefix = |range| match range {
            AxisRange::Positive => "+",
            AxisRange::Negative => "-",
            AxisRange::Full => "Full",
        };
        let binding = match self {
            Self::GamepadButton { index } => BUTTONS
                .get(usize::from(index))
                .context("PCSX2 SDL gamepad button index is invalid")?
                .to_string(),
            Self::GamepadAxis { index, range } => format!(
                "{}{}",
                prefix(range),
                AXES.get(usize::from(index))
                    .context("PCSX2 SDL gamepad axis index is invalid")?
            ),
            Self::JoystickButton { index } => format!("JoyButton{index}"),
            Self::JoystickAxis {
                index,
                range,
                inverted,
            } => format!(
                "{}JoyAxis{index}{}",
                prefix(range),
                if inverted { "~" } else { "" }
            ),
            Self::Hat { index, direction } => format!(
                "Hat{index}{}",
                match direction {
                    Direction::North => "North",
                    Direction::East => "East",
                    Direction::South => "South",
                    Direction::West => "West",
                }
            ),
            Self::LargeMotor => "LargeMotor".into(),
            Self::SmallMotor => "SmallMotor".into(),
            Self::Haptic => "Haptic".into(),
        };
        Ok(format!("SDL-{player_id}/{binding}"))
    }

    pub(crate) fn is_motor(self) -> bool {
        matches!(self, Self::LargeMotor | Self::SmallMotor | Self::Haptic)
    }
}
