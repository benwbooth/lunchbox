//! Typed SDL3 joystick-to-gamepad bindings, queried from the target runtime.
//! These indices are SDL indices, never Linux evdev or joydev indices.
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Input {
    Button { index: u32 },
    Axis { index: u32, min: i32, max: i32 },
    Hat { index: u32, mask: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Output {
    Button { index: u32 },
    Axis { index: u32, min: i32, max: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub input: Input,
    pub output: Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedGamepad {
    pub joystick_axes: u32,
    pub joystick_buttons: u32,
    pub joystick_hats: u32,
    pub bindings: Vec<Binding>,
}

impl ResolvedGamepad {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            [
                self.joystick_axes,
                self.joystick_buttons,
                self.joystick_hats
            ]
            .iter()
            .all(|n| *n <= 1024),
            "Invalid SDL control count"
        );
        ensure!(self.bindings.len() <= 4096, "Invalid SDL binding count");
        for binding in &self.bindings {
            match binding.input {
                Input::Button { index } => {
                    ensure!(index < self.joystick_buttons, "SDL button out of range")
                }
                Input::Axis { index, min, max } => {
                    ensure!(index < self.joystick_axes, "SDL axis out of range");
                    validate_range(min, max)?;
                }
                Input::Hat { index, mask } => {
                    ensure!(index < self.joystick_hats, "SDL hat out of range");
                    ensure!(mask > 0 && mask <= 15, "Invalid SDL hat mask");
                }
            }
            match binding.output {
                Output::Button { index } => ensure!(index < 26, "Unknown SDL3 gamepad button"),
                Output::Axis { index, min, max } => {
                    ensure!(index < 6, "Unknown SDL3 gamepad axis");
                    validate_range(min, max)?;
                }
            }
        }
        Ok(())
    }
}

fn validate_range(min: i32, max: i32) -> Result<()> {
    ensure!(
        (-32768..=32767).contains(&min) && (-32768..=32767).contains(&max) && min != max,
        "Invalid SDL axis range"
    );
    Ok(())
}

// SDL3's public ABI. Only read the union member selected by its discriminant.
// See SDL_gamepad.h, SDL_GamepadBinding (since SDL 3.2.0).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Axis {
    pub index: i32,
    pub min: i32,
    pub max: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Hat {
    pub index: i32,
    pub mask: i32,
}
#[repr(C)]
pub(crate) union InputData {
    pub button: i32,
    pub axis: Axis,
    pub hat: Hat,
}
#[repr(C)]
pub(crate) union OutputData {
    pub button: i32,
    pub axis: Axis,
}
#[repr(C)]
pub(crate) struct SdlBinding {
    pub input_type: i32,
    pub input: InputData,
    pub output_type: i32,
    pub output: OutputData,
}
const _: () = assert!(std::mem::size_of::<SdlBinding>() == 32);
const _: () = assert!(std::mem::align_of::<SdlBinding>() == 4);

impl SdlBinding {
    /// Caller supplies a live SDL_GetGamepadBindings entry.
    pub(crate) unsafe fn copy(&self) -> Result<Binding> {
        let input = match self.input_type {
            1 => Input::Button {
                index: unsafe { self.input.button }.try_into()?,
            },
            2 => {
                let a = unsafe { self.input.axis };
                Input::Axis {
                    index: a.index.try_into()?,
                    min: a.min,
                    max: a.max,
                }
            }
            3 => {
                let h = unsafe { self.input.hat };
                Input::Hat {
                    index: h.index.try_into()?,
                    mask: h.mask.try_into()?,
                }
            }
            value => bail!("Unknown SDL binding input type {value}"),
        };
        let output = match self.output_type {
            1 => Output::Button {
                index: unsafe { self.output.button }.try_into()?,
            },
            2 => {
                let a = unsafe { self.output.axis };
                Output::Axis {
                    index: a.index.try_into()?,
                    min: a.min,
                    max: a.max,
                }
            }
            value => bail!("Unknown SDL binding output type {value}"),
        };
        Ok(Binding { input, output })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copies_only_active_abi_union_members() {
        let b = SdlBinding {
            input_type: 3,
            input: InputData {
                hat: Hat { index: 0, mask: 8 },
            },
            output_type: 2,
            output: OutputData {
                axis: Axis {
                    index: 1,
                    min: 0,
                    max: -32768,
                },
            },
        };
        assert_eq!(
            unsafe { b.copy() }.unwrap(),
            Binding {
                input: Input::Hat { index: 0, mask: 8 },
                output: Output::Axis {
                    index: 1,
                    min: 0,
                    max: -32768
                },
            }
        );
        let invalid = SdlBinding {
            input_type: 77,
            ..b
        };
        assert!(unsafe { invalid.copy() }.is_err());
    }
    #[test]
    fn validates_indices_and_inverted_ranges() {
        let mut gamepad = ResolvedGamepad {
            joystick_axes: 1,
            joystick_buttons: 1,
            joystick_hats: 1,
            bindings: vec![Binding {
                input: Input::Axis {
                    index: 0,
                    min: 32767,
                    max: -32768,
                },
                output: Output::Axis {
                    index: 4,
                    min: 0,
                    max: 32767,
                },
            }],
        };
        gamepad.validate().unwrap();
        gamepad.joystick_axes = 0;
        assert!(gamepad.validate().is_err());
        gamepad.joystick_axes = 1;
        gamepad.bindings[0].output = Output::Axis {
            index: 4,
            min: 0,
            max: 65535,
        };
        assert!(gamepad.validate().is_err());
    }
}
