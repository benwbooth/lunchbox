//! Reuse measured SDL3 range resolution, then emit PCSX2's current vocabulary.
use super::sdl::{AxisRange, Direction, Input};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::{
    bindings::ResolvedGamepad,
    duckstation::{self, AnalogInput, AxisSuppression, DigitalInput},
};

pub(crate) fn digital(
    gamepad: &ResolvedGamepad,
    is_gamepad: bool,
    input: DigitalInput,
) -> Result<Input> {
    let token = duckstation::digital_binding(gamepad, input, AxisSuppression::VerifiedInputIndex)?;
    convert(&token, is_gamepad)
}

pub(crate) fn analog(
    gamepad: &ResolvedGamepad,
    is_gamepad: bool,
    input: AnalogInput,
) -> Result<Input> {
    let token = duckstation::analog_binding(gamepad, input, AxisSuppression::VerifiedInputIndex)?;
    convert(&token, is_gamepad)
}

fn convert(token: &str, is_gamepad: bool) -> Result<Input> {
    const BUTTONS: &[&str] = &[
        "A",
        "B",
        "X",
        "Y",
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
        "RightPaddle1",
        "LeftPaddle1",
        "RightPaddle2",
        "LeftPaddle2",
        "Touchpad",
        "Misc2",
        "Misc3",
        "Misc4",
        "Misc5",
        "Misc6",
    ];
    if let Some(index) = BUTTONS.iter().position(|name| *name == token) {
        return Ok(Input::GamepadButton { index: index as u8 });
    }
    if let Some(index) = token.strip_prefix("Button") {
        return Ok(Input::JoystickButton {
            index: index.parse()?,
        });
    }
    if let Some(hat) = token.strip_prefix("Hat") {
        // PCSX2 only allocates last_hat_state for non-gamepad joysticks.
        ensure!(
            !is_gamepad,
            "PCSX2 suppresses raw hats on recognized gamepads"
        );
        for (suffix, direction) in [
            ("North", Direction::North),
            ("East", Direction::East),
            ("South", Direction::South),
            ("West", Direction::West),
        ] {
            if let Some(index) = hat.strip_suffix(suffix) {
                return Ok(Input::Hat {
                    index: index.parse()?,
                    direction,
                });
            }
        }
    }
    let (range, axis) = if let Some(axis) = token.strip_prefix("Full") {
        (AxisRange::Full, axis)
    } else if let Some(axis) = token.strip_prefix('+') {
        (AxisRange::Positive, axis)
    } else if let Some(axis) = token.strip_prefix('-') {
        (AxisRange::Negative, axis)
    } else {
        anyhow::bail!("Unrecognized measured SDL binding for PCSX2");
    };
    if let Some(raw) = axis.strip_prefix("Axis") {
        let inverted = raw.ends_with('~');
        let index = raw.strip_suffix('~').unwrap_or(raw).parse()?;
        return Ok(Input::JoystickAxis {
            index,
            range,
            inverted,
        });
    }
    let index = [
        "LeftX",
        "LeftY",
        "RightX",
        "RightY",
        "LeftTrigger",
        "RightTrigger",
    ]
    .iter()
    .position(|name| *name == axis)
    .ok_or_else(|| anyhow::anyhow!("Unknown measured SDL axis for PCSX2"))?;
    Ok(Input::GamepadAxis {
        index: index as u8,
        range,
    })
}
