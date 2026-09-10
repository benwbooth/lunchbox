//! Measured SDL3 gamepad outputs translated to RPCS3 polling keys.
use crate::controller_pcsx2::sdl::{AxisRange, Input};
use anyhow::{Result, bail};
use lunchbox_controller_probe::{
    bindings::ResolvedGamepad,
    duckstation::{AnalogInput, DigitalInput},
};

pub(crate) fn digital(gamepad: &ResolvedGamepad, input: DigitalInput) -> Result<String> {
    convert(crate::controller_pcsx2::physical::digital(
        gamepad, true, input,
    )?)
}

pub(crate) fn analog(gamepad: &ResolvedGamepad, input: AnalogInput) -> Result<String> {
    convert(crate::controller_pcsx2::physical::analog(
        gamepad, true, input,
    )?)
}

fn convert(input: Input) -> Result<String> {
    const BUTTONS: &[&str] = &[
        "South",
        "East",
        "West",
        "North",
        "Back",
        "Guide",
        "Start",
        "LS",
        "RS",
        "LB",
        "RB",
        "Up",
        "Down",
        "Left",
        "Right",
        "Misc 1",
        "R Paddle 1",
        "L Paddle 1",
        "R Paddle 2",
        "L Paddle 2",
        "Touchpad",
        "Misc 2",
        "Misc 3",
        "Misc 4",
        "Misc 5",
        "Misc 6",
    ];
    match input {
        Input::GamepadButton { index } => Ok(BUTTONS
            .get(usize::from(index))
            .ok_or_else(|| anyhow::anyhow!("Unknown RPCS3 SDL button"))?
            .to_string()),
        Input::GamepadAxis { index, range } => {
            let positive = match range {
                AxisRange::Positive => true,
                AxisRange::Negative => false,
                AxisRange::Full => bail!("RPCS3 cannot encode this full-range mapped SDL axis"),
            };
            if index >= 4 {
                if !positive {
                    bail!("RPCS3 trigger output requires positive SDL activation");
                }
                return match index {
                    4 => Ok("LT".into()),
                    5 => Ok("RT".into()),
                    _ => bail!("Unknown RPCS3 SDL axis"),
                };
            }
            // RPCS3 labels Y+ as up, opposite SDL's increasing-down axis.
            let sign = if positive ^ (index % 2 == 1) {
                '+'
            } else {
                '-'
            };
            let axis = ["LS X", "LS Y", "RS X", "RS Y"][usize::from(index)];
            Ok(format!("{axis}{sign}"))
        }
        _ => bail!(
            "RPCS3 SDL mapping requires an independent gamepad output; raw joystick and hat fallback are unavailable"
        ),
    }
}
