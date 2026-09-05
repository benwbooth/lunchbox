//! Digital and proportional-axis translation for DuckStation's SDL protocol.
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

/// A calibrated direction of a physical analog control, in target SDL units.
/// The caller establishes analog capability from the chosen physical layout;
/// a C-button reporting axis events is not thereby an analog stick.
#[derive(Debug, Clone, Copy)]
pub struct AnalogInput {
    pub index: u32,
    pub released: i16,
    pub extent: i16,
}

pub fn physical_analog_binding(
    gamepad: &ResolvedGamepad,
    physical: &crate::linux_classic::ClassicMap,
    encoded: u32,
    measured: crate::linux_classic::AxisEndpoints,
    suppression: AxisSuppression,
) -> Result<String> {
    physical.validate_counts(gamepad)?;
    let DigitalInput::Axis {
        index,
        released,
        pressed,
    } = physical.digital_input(encoded, Some(measured))?
    else {
        anyhow::bail!("Buttons and hats cannot provide proportional analog movement");
    };
    analog_binding(
        gamepad,
        AnalogInput {
            index,
            released,
            extent: pressed,
        },
        suppression,
    )
}

/// Preserves a continuous axis interval. Never returns a button token even when
/// SDL mapped the underlying axis to a digital button. No threshold sampling or
/// synthesized analog movement is used.
pub fn analog_binding(
    gamepad: &ResolvedGamepad,
    axis: AnalogInput,
    suppression: AxisSuppression,
) -> Result<String> {
    gamepad.validate()?;
    let input = DigitalInput::Axis {
        index: axis.index,
        released: axis.released,
        pressed: axis.extent,
    };
    validate_input(gamepad, input)?;
    let low = i32::from(axis.released.min(axis.extent));
    let high = i32::from(axis.released.max(axis.extent));
    if let Some((position, binding)) = gamepad
        .bindings
        .iter()
        .enumerate()
        .find(|(_, b)| matches_press(&b.input, input))
        && let (Input::Axis { min, max, .. }, Output::Axis { .. }) =
            (&binding.input, &binding.output)
    {
        let covers_motion = *min.min(max) <= low && *min.max(max) >= high;
        // SDL chooses the first matching input range on every event. A
        // preceding mapping may steal the middle of the motion even though
        // the endpoint matched the later analog mapping.
        let interrupted = gamepad.bindings[..position]
            .iter()
            .any(|prior| match prior.input {
                Input::Axis { index, min, max } if index == axis.index => {
                    min.min(max).max(low) < min.max(max).min(high)
                }
                _ => false,
            });
        let aliased = gamepad.bindings.iter().any(|other| {
            !std::ptr::eq(binding, other)
                && same_output(&binding.output, &other.output)
                && binding.input != other.input
                && !matches!(
                    (&other.input, &binding.output, &other.output),
                    (Input::Axis { index, .. },
                     Output::Axis { min: a, max: b, .. },
                     Output::Axis { min: c, max: d, .. })
                        if *index == axis.index
                            && a.min(b).max(c.min(d)) >= a.max(b).min(c.max(d))
                )
        });
        // Opposite halves of one physical axis may legitimately share an
        // output. At the inclusive release boundary SDL can select the
        // earlier half, so check its actual rest value, not just this half.
        let rest = DigitalInput::Axis {
            index: axis.index,
            released: axis.released,
            pressed: axis.released,
        };
        let centered = gamepad
            .bindings
            .iter()
            .find(|other| matches_press(&other.input, rest))
            .is_none_or(|other| {
                !same_output(&binding.output, &other.output)
                    || mapped_axis_value(other, axis.released).is_some_and(|v| v.abs() <= 1)
            });
        if covers_motion
            && !interrupted
            && !aliased
            && centered
            && let Some(token) = mapped_token(binding, input)
        {
            return Ok(token);
        }
    }
    raw_binding(gamepad, input, suppression)
}

/// SDL's axis interpolation uses float32 and truncates the scaled offset.
/// Called only after the gamepad's ranges and the matching input are validated.
fn mapped_axis_value(binding: &Binding, value: i16) -> Option<i32> {
    let Input::Axis {
        min: input_min,
        max: input_max,
        ..
    } = binding.input
    else {
        return None;
    };
    let Output::Axis { min, max, .. } = binding.output else {
        return None;
    };
    let value = i32::from(value);
    if input_min == min && input_max == max {
        return Some(value);
    }
    let progress = (value - input_min) as f32 / (input_max - input_min) as f32;
    Some(min + (progress * (max - min) as f32) as i32)
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
    raw_binding(gamepad, input, suppression)
}

fn raw_binding(
    gamepad: &ResolvedGamepad,
    input: DigitalInput,
    suppression: AxisSuppression,
) -> Result<String> {
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
    fn measured_analog_motion_composes_physical_correction_and_sdl_axis() {
        use crate::linux_classic::{AxisCorrection, AxisEndpoints, ClassicMap};
        let mut physical = ClassicMap::from_joydev(&[], &[0]).unwrap();
        physical.axis_corrections.insert(
            0,
            AxisCorrection {
                coefficients: [127, 127, 4227330, 4227330, 0, 0, 0, 0],
                precision: 0,
                kind: 1,
            },
        );
        let p = ResolvedGamepad {
            joystick_axes: 1,
            joystick_buttons: 0,
            joystick_hats: 0,
            bindings: vec![Binding {
                input: Input::Axis {
                    index: 0,
                    min: -32768,
                    max: 32767,
                },
                output: Output::Axis {
                    index: 1,
                    min: 32767,
                    max: -32768,
                },
            }],
        };
        for (pressed, expected) in [(0, "+LeftY"), (255, "-LeftY")] {
            assert_eq!(
                physical_analog_binding(
                    &p,
                    &physical,
                    0x30000,
                    AxisEndpoints {
                        released: 127,
                        pressed
                    },
                    AxisSuppression::VerifiedInputIndex
                )
                .unwrap(),
                expected
            );
        }
        physical.axis_corrections.clear();
        assert!(
            physical_analog_binding(
                &p,
                &physical,
                0x30000,
                AxisEndpoints {
                    released: 127,
                    pressed: 255
                },
                AxisSuppression::VerifiedInputIndex
            )
            .is_err()
        );
    }

    #[test]
    fn analog_directions_preserve_axis_tokens_and_mapping_inversion() {
        let p = pad(vec![Binding {
            input: Input::Axis {
                index: 0,
                min: -32768,
                max: 32767,
            },
            output: Output::Axis {
                index: 0,
                min: -32768,
                max: 32767,
            },
        }]);
        for (extent, expected) in [(-32768, "-LeftX"), (32767, "+LeftX")] {
            assert_eq!(
                analog_binding(
                    &p,
                    AnalogInput {
                        index: 0,
                        released: 0,
                        extent
                    },
                    AxisSuppression::VerifiedInputIndex
                )
                .unwrap(),
                expected
            );
        }
        let inverse = pad(vec![Binding {
            input: Input::Axis {
                index: 0,
                min: 32767,
                max: -32768,
            },
            output: Output::Axis {
                index: 1,
                min: -32768,
                max: 32767,
            },
        }]);
        assert_eq!(
            analog_binding(
                &inverse,
                AnalogInput {
                    index: 0,
                    released: 0,
                    extent: 32767
                },
                AxisSuppression::VerifiedInputIndex
            )
            .unwrap(),
            "-LeftY"
        );
        assert_eq!(
            analog_binding(
                &pad(vec![]),
                AnalogInput {
                    index: 3,
                    released: 0,
                    extent: -32768
                },
                AxisSuppression::VerifiedInputIndex
            )
            .unwrap(),
            "-Axis3"
        );
    }

    #[test]
    fn analog_split_axis_halves_preserve_motion_and_require_a_neutral_boundary() {
        let half = |input_min, input_max, output_min, output_max| Binding {
            input: Input::Axis {
                index: 0,
                min: input_min,
                max: input_max,
            },
            output: Output::Axis {
                index: 0,
                min: output_min,
                max: output_max,
            },
        };
        let positive = half(0, 32767, 0, 32767);
        let p = pad(vec![half(-32768, 0, -32768, 0), positive.clone()]);
        for (extent, expected) in [(-32768, "-LeftX"), (32767, "+LeftX")] {
            assert_eq!(
                analog_binding(
                    &p,
                    AnalogInput {
                        index: 0,
                        released: 0,
                        extent
                    },
                    AxisSuppression::VerifiedInputIndex
                )
                .unwrap(),
                expected
            );
        }
        for negative in [
            // Opposite motion cannot also drive the positive output half.
            half(-32768, 0, 32767, 0),
            // Disjoint output halves, but the earlier one is nonzero at rest.
            half(-32768, 0, 0, -32768),
        ] {
            assert!(
                analog_binding(
                    &pad(vec![negative, positive.clone()]),
                    AnalogInput {
                        index: 0,
                        released: 0,
                        extent: 32767
                    },
                    AxisSuppression::VerifiedInputIndex
                )
                .is_err()
            );
        }
    }

    #[test]
    fn analog_rejects_button_outputs_shared_outputs_and_interrupted_motion() {
        let axis = AnalogInput {
            index: 0,
            released: 0,
            extent: 32767,
        };
        let digital = Binding {
            input: Input::Axis {
                index: 0,
                min: -32768,
                max: 32767,
            },
            output: Output::Button { index: 0 },
        };
        assert!(
            analog_binding(
                &pad(vec![digital]),
                axis,
                AxisSuppression::VerifiedInputIndex
            )
            .is_err()
        );
        let analog = Binding {
            input: Input::Axis {
                index: 0,
                min: -32768,
                max: 32767,
            },
            output: Output::Axis {
                index: 0,
                min: -32768,
                max: 32767,
            },
        };
        let middle = Binding {
            input: Input::Axis {
                index: 0,
                min: 10000,
                max: 20000,
            },
            output: Output::Button { index: 1 },
        };
        assert!(
            analog_binding(
                &pad(vec![middle, analog.clone()]),
                axis,
                AxisSuppression::VerifiedInputIndex
            )
            .is_err()
        );
        let alias = Binding {
            input: Input::Button { index: 3 },
            output: analog.output.clone(),
        };
        assert!(
            analog_binding(
                &pad(vec![analog.clone(), alias]),
                axis,
                AxisSuppression::VerifiedInputIndex
            )
            .is_err()
        );
        let half = Binding {
            input: Input::Axis {
                index: 0,
                min: 100,
                max: 32767,
            },
            output: analog.output,
        };
        assert!(
            analog_binding(&pad(vec![half]), axis, AxisSuppression::VerifiedInputIndex).is_err()
        );
    }

    #[test]
    fn analog_physical_conversion_rejects_hats_and_buttons() {
        use crate::linux_classic::{AxisCorrection, AxisEndpoints, ClassicMap};
        let mut physical = ClassicMap::from_joydev(&[304], &[16, 17]).unwrap();
        physical
            .axis_corrections
            .insert(16, AxisCorrection::default());
        let p = ResolvedGamepad {
            joystick_buttons: 1,
            joystick_axes: 0,
            joystick_hats: 1,
            bindings: vec![],
        };
        for encoded in [0x10130, 0x30010] {
            assert!(
                physical_analog_binding(
                    &p,
                    &physical,
                    encoded,
                    AxisEndpoints {
                        released: 0,
                        pressed: 1
                    },
                    AxisSuppression::VerifiedInputIndex
                )
                .is_err()
            );
        }
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
