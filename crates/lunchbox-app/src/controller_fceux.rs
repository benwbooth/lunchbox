//! FCEUX Qt 2.6.6 native joystick profile controls, not libretro.
//! Source: 34eb7601c415b81901fd02afbd5cfdc84b5047ac,
//! src/drivers/Qt/sdl-joystick.cpp.
use anyhow::{Result, ensure};

pub(crate) mod configuration;
pub(crate) mod isolation;
pub(crate) mod layers;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod physical;
pub(crate) mod prepared;
pub(crate) mod profile;
pub(crate) mod profile_mount;
pub(crate) mod routing;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Input {
    Button(u16),
    Axis { index: u8, positive: bool },
    Hat { index: u8, direction: u8 },
}

impl Input {
    pub(crate) fn packed(self) -> Result<u16> {
        Ok(match self {
            Self::Button(index) => {
                ensure!(
                    index < 0x2000,
                    "FCEUX button overlaps native hat/axis flag encoding"
                );
                index
            }
            Self::Axis { index, positive } => {
                0x8000 | if positive { 0 } else { 0x4000 } | u16::from(index)
            }
            Self::Hat { index, direction } => {
                // Although packed hats have five index bits, the text parser
                // accepts exactly one digit before the dot. Do not emit a
                // multi-digit index that silently fails to bind.
                ensure!(
                    index <= 9 && matches!(direction, 1 | 2 | 4 | 8),
                    "FCEUX profile needs a single-digit hat index and cardinal direction"
                );
                0x2000 | (u16::from(index) << 8) | u16::from(direction)
            }
        })
    }

    pub(crate) fn as_setting(self) -> Result<String> {
        self.packed()?;
        Ok(match self {
            Self::Button(index) => format!("b{index}"),
            Self::Axis { index, positive } => {
                format!("{}a{index}", if positive { '+' } else { '-' })
            }
            Self::Hat { index, direction } => format!("h{index}.{direction}"),
        })
    }

    pub(crate) fn measured_axis(index: u8, released: i16, pressed: i16) -> Result<Self> {
        ensure!(released != pressed, "FCEUX axis has no measured travel");
        let positive = pressed > released;
        // Preserve the pinned frontend's asymmetric constants exactly.
        ensure!(
            if positive {
                released < 16363 && pressed >= 16363
            } else {
                released > -16383 && pressed <= -16383
            },
            "FCEUX fixed native threshold cannot represent the measured axis gesture"
        );
        Ok(Self::Axis { index, positive })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_profile_spellings_preserve_axis_and_hat_flags() {
        assert_eq!(Input::Button(7).packed().unwrap(), 7);
        assert_eq!(Input::Button(7).as_setting().unwrap(), "b7");
        assert_eq!(
            Input::Axis {
                index: 2,
                positive: true
            }
            .packed()
            .unwrap(),
            0x8002
        );
        assert_eq!(
            Input::Axis {
                index: 2,
                positive: false
            }
            .as_setting()
            .unwrap(),
            "-a2"
        );
        assert_eq!(
            Input::Hat {
                index: 1,
                direction: 8
            }
            .as_setting()
            .unwrap(),
            "h1.8"
        );
        assert!(
            Input::Hat {
                index: 10,
                direction: 1
            }
            .packed()
            .is_err()
        );
    }

    #[test]
    fn measured_axis_requires_crossing_the_pinned_fixed_threshold() {
        assert_eq!(
            Input::measured_axis(0, 0, 16363).unwrap(),
            Input::Axis {
                index: 0,
                positive: true
            }
        );
        assert_eq!(
            Input::measured_axis(1, 0, -16383).unwrap(),
            Input::Axis {
                index: 1,
                positive: false
            }
        );
        assert!(Input::measured_axis(0, 16363, 17000).is_err());
    }
}
