//! Translate resolved raw SDL2 controls into Snes9x GTK's native input space.
use super::{Binding, JoystickInput};
use anyhow::{Result, ensure};

pub(crate) fn calibrated_pad(
    calibration: &crate::controller_catalog::Calibration,
    snapshot: &lunchbox_controller_probe::sdl2::Snapshot,
    runtime_path: &str,
    player: u8,
) -> Result<super::configuration::Pad> {
    use anyhow::Context;
    use lunchbox_controller_probe::{
        duckstation::DigitalInput,
        linux_classic::{AxisEndpoints, Control},
        sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "Snes9x native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let counts = device
        .controls
        .as_ref()
        .context("Snes9x SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("Snes9x released state is absent")?;
    state.validate(counts)?;
    let native_counts = Counts {
        buttons: counts.buttons.try_into()?,
        axes: counts.axes.try_into()?,
        hats: counts.hats.try_into()?,
    };
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == "snes9x:standalone-gtk-snes")
        .context("Missing native Snes9x profile")?;
    let mut bindings = std::collections::BTreeMap::new();
    for row in calibration.plan_profile(profile)?.rows {
        let input = row
            .input
            .context("Snes9x gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("Snes9x requires measured native controls")?;
        let control = match physical.control(native.code)? {
            Control::Button(index) => {
                ensure!(
                    state.buttons.get(&index) == Some(&false),
                    "Release Snes9x controller buttons before capture"
                );
                Input::Button {
                    index: index.try_into()?,
                }
            }
            Control::Axis(index) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("Snes9x axis measurements are absent")?;
                let released =
                    physical.axis_value((native.code & 0xffff) as u8, endpoints.released)?;
                let pressed =
                    physical.axis_value((native.code & 0xffff) as u8, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&index) == Some(&released),
                    "Snes9x axis rest differs from calibration"
                );
                Input::Axis {
                    index: index.try_into()?,
                    released,
                    pressed,
                }
            }
            Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("Snes9x hat measurements are absent")?;
                let DigitalInput::Hat { index, direction } = physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )?
                else {
                    anyhow::bail!("Snes9x hat did not resolve to a native SDL hat");
                };
                ensure!(
                    state.hats.get(&index) == Some(&0),
                    "Release Snes9x directional hat before capture"
                );
                Input::Hat {
                    index: index.try_into()?,
                    direction: direction.try_into()?,
                }
            }
        };
        ensure!(
            bindings
                .insert(
                    row.output,
                    resolve(device.device_index.try_into()?, &native_counts, control)?
                )
                .is_none(),
            "Duplicate Snes9x gameplay target"
        );
    }
    let pad = super::configuration::Pad { player, bindings };
    super::configuration::render("", std::slice::from_ref(&pad))?;
    Ok(pad)
}

pub(crate) enum Input {
    Button {
        index: u16,
    },
    Axis {
        index: u16,
        released: i16,
        pressed: i16,
    },
    /// SDL hat cardinal mask: up=1, right=2, down=4, left=8.
    Hat {
        index: u16,
        direction: u8,
    },
}

pub(crate) struct Counts {
    pub buttons: u16,
    pub axes: u16,
    pub hats: u16,
}

pub(crate) fn resolve(joystick: u8, counts: &Counts, input: Input) -> Result<Binding> {
    let input = match input {
        Input::Button { index } => {
            ensure!(
                index < counts.buttons,
                "Snes9x button is absent from native SDL inventory"
            );
            JoystickInput::Button(index)
        }
        Input::Hat { index, direction } => {
            ensure!(
                index < counts.hats,
                "Snes9x hat is absent from native SDL inventory"
            );
            let (horizontal, positive) = match direction {
                1 => (false, true),
                2 => (true, true),
                4 => (false, false),
                8 => (true, false),
                _ => anyhow::bail!("Snes9x needs one cardinal hat direction"),
            };
            // GTK appends virtual hat axes after physical SDL axes: vertical
            // first (up positive), horizontal second (right positive).
            let axis = u32::from(counts.axes) + 2 * u32::from(index) + u32::from(horizontal);
            ensure!(
                axis <= 32511,
                "Snes9x virtual hat axis exceeds native encoding"
            );
            JoystickInput::Axis {
                index: axis as u16,
                positive,
                threshold_percent: 50,
            }
        }
        Input::Axis {
            index,
            released,
            pressed,
        } => {
            ensure!(
                index < counts.axes && released != pressed,
                "Snes9x axis is missing or has no measured travel"
            );
            // Fresh native devices start at center 0 and range ±32767. Pick
            // an integer percentage whose actual inclusive native threshold
            // separates the measured release and press; don't assume 50% fits.
            let positive = pressed > released;
            let sign = if positive { 1_i32 } else { -1 };
            let rest = i32::from(released) * sign;
            let active = i32::from(pressed) * sign;
            let midpoint_twice = rest + active;
            let threshold_percent = (1..=100_u8).filter(|percent| {
                let threshold = 32767 * i32::from(*percent) / 100;
                rest < threshold && active >= threshold
            }).min_by_key(|percent| {
                (2 * (32767 * i32::from(*percent) / 100) - midpoint_twice).abs()
            }).ok_or_else(|| anyhow::anyhow!("Snes9x native centered threshold cannot represent the measured axis endpoint"))?;
            JoystickInput::Axis {
                index,
                positive,
                threshold_percent,
            }
        }
    };
    let binding = Binding { joystick, input };
    binding.packed()?;
    Ok(binding)
}
