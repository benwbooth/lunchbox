//! melonDS Qt/SDL2 standard controls at 906e9ebb27da8c6a715cd7abab4abfe8a8d29427.
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) mod configuration;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod paths;
pub(crate) mod physical;
pub(crate) mod prepared;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

pub(crate) fn visual_routes() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("a", "A"),
        ("b", "B"),
        ("x", "X"),
        ("y", "Y"),
        ("up", "Up"),
        ("down", "Down"),
        ("left", "Left"),
        ("right", "Right"),
        ("start", "Start"),
        ("select", "Select"),
        ("l", "L"),
        ("r", "R"),
    ])
}

/// Native raw joystick indices, not SDL game-controller output indices.
pub(crate) enum Input {
    Button(u16),
    Hat { index: u8, direction: u8 },
    Axis { index: u8, activation: Activation },
}

pub(crate) enum Activation {
    Positive,
    Negative,
    Trigger,
}

impl Input {
    pub(crate) fn encode(&self) -> Result<i32> {
        Ok(match self {
            Self::Button(index) => {
                // Native bit 8 denotes a hat, and 0xffff denotes no button.
                ensure!(
                    *index != 0xffff && index & 0x100 == 0,
                    "melonDS button index collides with native hat/unbound encoding"
                );
                i32::from(*index)
            }
            Self::Hat { index, direction } => {
                ensure!(
                    *index < 16 && matches!(*direction, 1 | 2 | 4 | 8),
                    "melonDS requires a cardinal direction on hat 0..15"
                );
                0x100 | (i32::from(*index) << 4) | i32::from(*direction)
            }
            Self::Axis { index, activation } => {
                ensure!(*index < 16, "melonDS supports raw axes 0..15");
                let direction = match activation {
                    Activation::Positive => 0,
                    Activation::Negative => 1,
                    Activation::Trigger => 2,
                };
                // Disable the low-word button branch; axis activation is ORed
                // with that branch by native joystickButtonDown.
                0x1ffff | (direction << 20) | (i32::from(*index) << 24)
            }
        })
    }
}

impl Activation {
    pub(crate) fn validate_endpoints(&self, released: i16, pressed: i16) -> Result<()> {
        let down = |value: i16| match self {
            Self::Positive => value > 16384,
            Self::Negative => value < -16384,
            Self::Trigger => value > 0,
        };
        ensure!(
            !down(released) && down(pressed),
            "melonDS calibration does not cross the native activation threshold"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_encodings_keep_button_hat_and_axis_namespaces_distinct() {
        assert_eq!(Input::Button(7).encode().unwrap(), 7);
        assert_eq!(
            Input::Hat {
                index: 2,
                direction: 4
            }
            .encode()
            .unwrap(),
            0x124
        );
        assert_eq!(
            Input::Axis {
                index: 3,
                activation: Activation::Negative
            }
            .encode()
            .unwrap(),
            0x0311ffff
        );
        assert!(Input::Button(0xffff).encode().is_err());
        assert!(
            Input::Hat {
                index: 0,
                direction: 3
            }
            .encode()
            .is_err()
        );
    }

    #[test]
    fn activation_requires_release_to_pressed_threshold_crossing() {
        assert!(Activation::Positive.validate_endpoints(0, 16385).is_ok());
        assert!(Activation::Negative.validate_endpoints(0, -16385).is_ok());
        assert!(Activation::Trigger.validate_endpoints(0, 1).is_ok());
        assert!(
            Activation::Positive
                .validate_endpoints(17000, 18000)
                .is_err()
        );
    }
}
