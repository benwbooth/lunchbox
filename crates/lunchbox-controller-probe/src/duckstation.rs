//! Digital-control translation for DuckStation's SDL input protocol.
//!
//! Input indices must already be resolved in the *target SDL runtime*. This is
//! not evdev-to-SDL conversion, player assignment, or an emulator launch adapter.
//! Protocol checked against DuckStation 0a53bc47c and SDL 3.2.20; no upstream
//! implementation is embedded. See docs/DUCKSTATION_CONTROLLERS.md.
use crate::bindings::{Binding, Input, Output, ResolvedGamepad};
use anyhow::{Result, ensure};

/// One digital action, including the calibrated rest and pressed values when
/// hardware reports it as an axis (for example an N64 C button or a trigger).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitalInput {
    Button(u32),
    Hat {
        index: u32,
        direction: u8,
    },
    Axis {
        index: u32,
        released: i16,
        pressed: i16,
    },
}

/// Raw-axis suppression differs between DuckStation implementations. Require an
/// explicit verified contract, never guess from the advertised SDL version.
#[derive(Debug, Clone, Copy)]
pub enum AxisSuppression {
    /// Installed 0a53bc47c uses the binding's output index to mark raw axes used.
    DuckStation0a53bc47c,
    /// Use only for an emulator revision verified to mark the input axis index.
    VerifiedInputIndex,
}

/// Compose measured physical input, kernel correction, and the target runtime's
/// SDL mapping. The returned suffix still needs a verified emulator player ID.
pub fn physical_digital_binding(
    gamepad: &ResolvedGamepad,
    physical: &crate::linux_classic::ClassicMap,
    encoded: u32,
    measured: Option<crate::linux_classic::AxisEndpoints>,
    suppression: AxisSuppression,
) -> Result<String> {
    physical.validate_counts(gamepad)?;
    digital_binding(
        gamepad,
        physical.digital_input(encoded, measured)?,
        suppression,
    )
}

/// Return the binding component after `SDL-N/`. The caller must independently
/// establish DuckStation's actual player ID. This only targets digital actions;
/// it must not be used to replace a target analog stick with buttons.
pub fn digital_binding(
    gamepad: &ResolvedGamepad,
    input: DigitalInput,
    suppression: AxisSuppression,
) -> Result<String> {
    gamepad.validate()?;
    validate_input(gamepad, input)?;

    // SDL uses the first matching button/axis binding. Hats may have multiple
    // direction bindings, but a cardinal action must have an independent output.
    let matching: Vec<_> = gamepad
        .bindings
        .iter()
        .filter(|b| matches_press(&b.input, input))
        .collect();
    for binding in matching
        .iter()
        .take(if matches!(input, DigitalInput::Hat { .. }) {
            usize::MAX
        } else {
            1
        })
    {
        if let Some(token) = mapped_token(binding, input) {
            let aliased = gamepad.bindings.iter().any(|other| {
                !std::ptr::eq(*binding, other)
                    && same_output(&binding.output, &other.output)
                    && binding.input != other.input
            });
            if !aliased {
                return Ok(token);
            }
        }
    }
    // DuckStation drops whole mapped buttons/hats, not only the mapped hat
    // direction. Its installed axis-index quirk must also be modeled explicitly.
    let suppressed = gamepad
        .bindings
        .iter()
        .any(|binding| match (input, &binding.input) {
            (DigitalInput::Button(index), Input::Button { index: mapped }) => index == *mapped,
            (DigitalInput::Hat { index, .. }, Input::Hat { index: mapped, .. }) => index == *mapped,
            (DigitalInput::Axis { index, .. }, Input::Axis { index: mapped, .. }) => {
                match suppression {
                    AxisSuppression::VerifiedInputIndex => index == *mapped,
                    AxisSuppression::DuckStation0a53bc47c => {
                        index
                            == match binding.output {
                                Output::Button { index } | Output::Axis { index, .. } => index,
                            }
                    }
                }
            }
            _ => false,
        });
    ensure!(
        !suppressed,
        "SDL input has no independent usable mapped output and its raw event is suppressed"
    );
    match input {
        DigitalInput::Button(index) => Ok(format!("Button{index}")),
        DigitalInput::Hat { index, direction } => {
            let direction = match direction {
                1 => "North",
                2 => "East",
                4 => "South",
                8 => "West",
                _ => unreachable!(),
            };
            Ok(format!("Hat{index}{direction}"))
        }
        DigitalInput::Axis {
            index,
            released,
            pressed,
        } => {
            let sign = axis_sign(f64::from(released), f64::from(pressed)).ok_or_else(|| {
                anyhow::anyhow!("Raw axis needs a verified released/pressed range conversion")
            })?;
            Ok(format!("{sign}Axis{index}"))
        }
    }
}

fn validate_input(gamepad: &ResolvedGamepad, input: DigitalInput) -> Result<()> {
    match input {
        DigitalInput::Button(i) => ensure!(
            i < gamepad.joystick_buttons,
            "Requested SDL button out of range"
        ),
        DigitalInput::Hat { index, direction } => {
            ensure!(
                index < gamepad.joystick_hats,
                "Requested SDL hat out of range"
            );
            ensure!(
                matches!(direction, 1 | 2 | 4 | 8),
                "Expected a cardinal hat direction"
            );
        }
        DigitalInput::Axis {
            index,
            released,
            pressed,
        } => {
            ensure!(
                index < gamepad.joystick_axes,
                "Requested SDL axis out of range"
            );
            ensure!(released != pressed, "Axis did not move");
        }
    }
    Ok(())
}

fn matches_press(binding: &Input, input: DigitalInput) -> bool {
    match (binding, input) {
        (Input::Button { index: a }, DigitalInput::Button(b)) => *a == b,
        (
            Input::Hat { index: a, mask },
            DigitalInput::Hat {
                index: b,
                direction,
            },
        ) => *a == b && mask & direction != 0,
        (
            Input::Axis { index: a, min, max },
            DigitalInput::Axis {
                index: b, pressed, ..
            },
        ) => *a == b && (*min.min(max)..=*min.max(max)).contains(&i32::from(pressed)),
        _ => false,
    }
}

fn same_output(a: &Output, b: &Output) -> bool {
    match (a, b) {
        (Output::Button { index: a }, Output::Button { index: b }) => a == b,
        // Shared opposite half-axes are also rejected for now: SDL event ordering
        // can cause one physical control to reset the other's state.
        (Output::Axis { index: a, .. }, Output::Axis { index: b, .. }) => a == b,
        _ => false,
    }
}

fn mapped_token(binding: &Binding, input: DigitalInput) -> Option<String> {
    if let (Input::Hat { mask, .. }, DigitalInput::Hat { direction, .. }) = (&binding.input, input)
    {
        // A multi-bit mask makes multiple directions activate the same control.
        if *mask != direction {
            return None;
        }
    }
    let (released, pressed) = match input {
        DigitalInput::Axis {
            released, pressed, ..
        } => {
            let Input::Axis { min, max, .. } = binding.input else {
                return None;
            };
            let progress = |value: i16| {
                let value = i32::from(value);
                if (min.min(max)..=min.max(max)).contains(&value) {
                    Some(f64::from(value - min) / f64::from(max - min))
                } else {
                    None
                }
            };
            (progress(released), progress(pressed))
        }
        _ => (Some(0.0), Some(1.0)),
    };
    match binding.output {
        Output::Button { index } => {
            if released.unwrap_or(0.0) >= 0.5 || pressed? < 0.5 {
                return None;
            }
            button_name(index).map(str::to_owned)
        }
        Output::Axis { index, min, max } => {
            let value = |p: Option<f64>| {
                p.map(|p| f64::from(min) + p * f64::from(max - min))
                    .unwrap_or(0.0)
            };
            // Hat release resets an output axis to zero, not the range minimum.
            let rest = if matches!(input, DigitalInput::Hat { .. }) {
                0.0
            } else {
                value(released)
            };
            let sign = axis_sign(rest, value(pressed))?;
            let name = [
                "LeftX",
                "LeftY",
                "RightX",
                "RightY",
                "LeftTrigger",
                "RightTrigger",
            ]
            .get(index as usize)?;
            Some(format!("{sign}{name}"))
        }
    }
}

fn axis_sign(released: f64, pressed: f64) -> Option<char> {
    // Require a centered release; endpoints at rest need full-axis/inversion
    // semantics and must not accidentally leave a digital target held down.
    // Endpoints avoid relying on the user's emulator deadzone/activation
    // threshold. Allow one integer unit for asymmetric SDL range interpolation.
    if released.abs() > 1.0 || pressed.abs() < 32766.0 {
        return None;
    }
    Some(if pressed < 0.0 { '-' } else { '+' })
}

fn button_name(index: u32) -> Option<&'static str> {
    // Functional names in DuckStation's serialized SDL binding protocol.
    Some(match index {
        0 => "A",
        1 => "B",
        2 => "X",
        3 => "Y",
        4 => "Back",
        5 => "Guide",
        6 => "Start",
        7 => "LeftStick",
        8 => "RightStick",
        9 => "LeftShoulder",
        10 => "RightShoulder",
        11 => "DPadUp",
        12 => "DPadDown",
        13 => "DPadLeft",
        14 => "DPadRight",
        15 => "Misc1",
        16 => "RightPaddle1",
        17 => "LeftPaddle1",
        18 => "RightPaddle2",
        19 => "LeftPaddle2",
        20 => "Touchpad",
        21 => "Misc2",
        22 => "Misc3",
        23 => "Misc4",
        24 => "Misc5",
        25 => "Misc6",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pad(bindings: Vec<Binding>) -> ResolvedGamepad {
        ResolvedGamepad {
            joystick_axes: 6,
            joystick_buttons: 12,
            joystick_hats: 1,
            bindings,
        }
    }
    fn translate(pad: &ResolvedGamepad, input: DigitalInput) -> Result<String> {
        digital_binding(pad, input, AxisSuppression::DuckStation0a53bc47c)
    }
    #[test]
    fn physical_button_number_is_not_an_xbox_position() {
        let p = pad(vec![Binding {
            input: Input::Button { index: 0 },
            output: Output::Button { index: 3 },
        }]);
        assert_eq!(translate(&p, DigitalInput::Button(0)).unwrap(), "Y");
        assert_eq!(translate(&p, DigitalInput::Button(11)).unwrap(), "Button11");
        assert!(translate(&p, DigitalInput::Button(12)).is_err());
    }

    #[test]
    fn physical_measurement_composes_kernel_and_sdl_mapping_for_a_trigger() {
        use crate::linux_classic::{AxisCorrection, AxisEndpoints, ClassicMap};
        let mut physical = ClassicMap::from_joydev(&[304], &[16, 17, 2]).unwrap();
        physical.axis_corrections.insert(
            2,
            AxisCorrection {
                coefficients: [127, 127, 4227330, 4227330, 0, 0, 0, 0],
                precision: 0,
                kind: 1,
            },
        );
        let p = ResolvedGamepad {
            joystick_axes: 1,
            joystick_buttons: 1,
            joystick_hats: 1,
            bindings: vec![Binding {
                input: Input::Axis {
                    index: 0,
                    min: -32768,
                    max: 32767,
                },
                output: Output::Axis {
                    index: 4,
                    min: 0,
                    max: 32767,
                },
            }],
        };
        assert_eq!(
            physical_digital_binding(
                &p,
                &physical,
                0x30002,
                Some(AxisEndpoints {
                    released: 0,
                    pressed: 255
                }),
                AxisSuppression::DuckStation0a53bc47c
            )
            .unwrap(),
            "+LeftTrigger"
        );
        let mut old = physical.clone();
        old.axis_corrections.clear();
        assert!(
            physical_digital_binding(
                &p,
                &old,
                0x30002,
                Some(AxisEndpoints {
                    released: 0,
                    pressed: 255
                }),
                AxisSuppression::DuckStation0a53bc47c
            )
            .is_err()
        );
        assert_eq!(
            physical_digital_binding(
                &p,
                &old,
                0x10130,
                None,
                AxisSuppression::DuckStation0a53bc47c
            )
            .unwrap(),
            "Button0"
        );
    }
    #[test]
    fn c_button_on_axis_uses_mapped_axis_and_preserves_direction() {
        let p = pad(vec![Binding {
            input: Input::Axis {
                index: 3,
                min: -32768,
                max: 32767,
            },
            output: Output::Axis {
                index: 2,
                min: 32767,
                max: -32768,
            },
        }]);
        assert_eq!(
            translate(
                &p,
                DigitalInput::Axis {
                    index: 3,
                    released: 0,
                    pressed: 32767
                }
            )
            .unwrap(),
            "-RightX"
        );
        assert_eq!(
            translate(
                &p,
                DigitalInput::Axis {
                    index: 3,
                    released: 0,
                    pressed: -32768
                }
            )
            .unwrap(),
            "+RightX"
        );
    }
    #[test]
    fn trigger_range_and_button_driven_trigger() {
        let p = pad(vec![Binding {
            input: Input::Axis {
                index: 2,
                min: -32768,
                max: 32767,
            },
            output: Output::Axis {
                index: 4,
                min: 0,
                max: 32767,
            },
        }]);
        assert_eq!(
            translate(
                &p,
                DigitalInput::Axis {
                    index: 2,
                    released: -32768,
                    pressed: 32767
                }
            )
            .unwrap(),
            "+LeftTrigger"
        );
        let p = pad(vec![Binding {
            input: Input::Button { index: 5 },
            output: Output::Axis {
                index: 5,
                min: 0,
                max: 32767,
            },
        }]);
        assert_eq!(
            translate(&p, DigitalInput::Button(5)).unwrap(),
            "+RightTrigger"
        );
    }
    #[test]
    fn hats_are_suppressed_as_a_whole_not_per_direction() {
        let p = pad(vec![Binding {
            input: Input::Hat { index: 0, mask: 1 },
            output: Output::Button { index: 11 },
        }]);
        assert_eq!(
            translate(
                &p,
                DigitalInput::Hat {
                    index: 0,
                    direction: 1
                }
            )
            .unwrap(),
            "DPadUp"
        );
        assert!(
            translate(
                &p,
                DigitalInput::Hat {
                    index: 0,
                    direction: 8
                }
            )
            .is_err()
        );
        assert_eq!(
            translate(
                &pad(vec![]),
                DigitalInput::Hat {
                    index: 0,
                    direction: 8
                }
            )
            .unwrap(),
            "Hat0West"
        );
        assert!(
            translate(
                &p,
                DigitalInput::Hat {
                    index: 0,
                    direction: 3
                }
            )
            .is_err()
        );
    }
    #[test]
    fn installed_axis_suppression_quirk_is_not_assumed_for_other_backends() {
        let p = pad(vec![Binding {
            input: Input::Axis {
                index: 3,
                min: 0,
                max: 32767,
            },
            output: Output::Axis {
                index: 2,
                min: 0,
                max: 32767,
            },
        }]);
        let i = DigitalInput::Axis {
            index: 2,
            released: 0,
            pressed: 32767,
        };
        assert!(translate(&p, i).is_err());
        assert_eq!(
            digital_binding(&p, i, AxisSuppression::VerifiedInputIndex).unwrap(),
            "+Axis2"
        );
    }
    #[test]
    fn duplicate_outputs_do_not_become_independent_controls() {
        let p = pad(vec![
            Binding {
                input: Input::Button { index: 0 },
                output: Output::Button { index: 0 },
            },
            Binding {
                input: Input::Button { index: 1 },
                output: Output::Button { index: 0 },
            },
        ]);
        assert!(translate(&p, DigitalInput::Button(0)).is_err());
    }
    #[test]
    fn merged_hat_directions_and_axis_halves_are_not_independent() {
        let p = pad(vec![Binding {
            input: Input::Hat { index: 0, mask: 3 },
            output: Output::Button { index: 0 },
        }]);
        assert!(
            translate(
                &p,
                DigitalInput::Hat {
                    index: 0,
                    direction: 1
                }
            )
            .is_err()
        );
        let p = pad(vec![
            Binding {
                input: Input::Axis {
                    index: 0,
                    min: 0,
                    max: 32767,
                },
                output: Output::Button { index: 0 },
            },
            Binding {
                input: Input::Axis {
                    index: 0,
                    min: 0,
                    max: -32768,
                },
                output: Output::Button { index: 0 },
            },
        ]);
        assert!(
            translate(
                &p,
                DigitalInput::Axis {
                    index: 0,
                    released: 0,
                    pressed: 32767
                }
            )
            .is_err()
        );
    }
    #[test]
    fn split_axis_to_buttons_selects_correct_half_and_rejects_held_rest() {
        let p = pad(vec![
            Binding {
                input: Input::Axis {
                    index: 0,
                    min: 0,
                    max: -32768,
                },
                output: Output::Button { index: 2 },
            },
            Binding {
                input: Input::Axis {
                    index: 0,
                    min: 0,
                    max: 32767,
                },
                output: Output::Button { index: 1 },
            },
        ]);
        assert_eq!(
            translate(
                &p,
                DigitalInput::Axis {
                    index: 0,
                    released: 0,
                    pressed: -32768
                }
            )
            .unwrap(),
            "X"
        );
        assert_eq!(
            translate(
                &p,
                DigitalInput::Axis {
                    index: 0,
                    released: 0,
                    pressed: 32767
                }
            )
            .unwrap(),
            "B"
        );
        assert!(
            digital_binding(
                &p,
                DigitalInput::Axis {
                    index: 0,
                    released: 20000,
                    pressed: 32767
                },
                AxisSuppression::VerifiedInputIndex
            )
            .is_err()
        );
    }
}
