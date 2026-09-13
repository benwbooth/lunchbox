//! Translate measured physical controls into Nestopia UE's raw SDL2 grammar.

use super::{
    configuration::{Binding, CONTROLS, Pad},
    settings::MAPPING_PROFILE,
};
use anyhow::{Context, Result, ensure};

pub(crate) fn calibrated_pad(
    calibration: &crate::controller_catalog::Calibration,
    snapshot: &lunchbox_controller_probe::sdl2::Snapshot,
    runtime_path: &str,
    player: u8,
) -> Result<Pad> {
    use lunchbox_controller_probe::{
        duckstation::DigitalInput,
        linux_classic::{AxisEndpoints, Control},
        sdl2_physical::PhysicalMap,
    };

    ensure!(
        calibration.os == "linux",
        "Nestopia Flatpak needs Linux calibration"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    ensure!(
        device.device_index <= 9,
        "Nestopia selected SDL index is not encodable"
    );
    let counts = device
        .controls
        .as_ref()
        .context("Nestopia SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("Nestopia released state is absent")?;
    state.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == MAPPING_PROFILE)
        .context("Missing NES two-pad mapping profile")?;
    let mut bindings = std::collections::BTreeMap::new();
    for row in calibration.plan_profile(profile)?.rows {
        ensure!(
            CONTROLS.iter().any(|(name, _)| *name == row.target_id),
            "Nestopia mapping target changed"
        );
        let input = row
            .input
            .context("Nestopia gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("Nestopia needs measured native controls")?;
        let binding = match physical.control(native.code)? {
            Control::Button(index) => {
                ensure!(
                    state.buttons.get(&index) == Some(&false),
                    "Release Nestopia controller buttons before capture"
                );
                Binding::Button(index.try_into()?)
            }
            Control::Axis(index) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("Nestopia axis measurements are absent")?;
                let released =
                    physical.axis_value((native.code & 0xffff) as u8, endpoints.released)?;
                let pressed =
                    physical.axis_value((native.code & 0xffff) as u8, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&index) == Some(&released),
                    "Nestopia axis rest differs from calibration"
                );
                ensure!(
                    (-16384..=16384).contains(&released) && (pressed < -16384 || pressed > 16384),
                    "Nestopia fixed axis threshold does not separate release and press"
                );
                Binding::Axis {
                    index: index.try_into()?,
                    positive: pressed > 16384,
                }
            }
            Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("Nestopia hat measurements are absent")?;
                let DigitalInput::Hat { index, direction } = physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )?
                else {
                    anyhow::bail!("Nestopia hat did not resolve to SDL hat input");
                };
                ensure!(
                    index == 0 && state.hats.get(&index) == Some(&0),
                    "Nestopia can represent only a released SDL hat zero"
                );
                let direction = match direction {
                    1 => 0, // SDL_HAT_UP
                    4 => 1, // SDL_HAT_DOWN
                    8 => 2, // SDL_HAT_LEFT
                    2 => 3, // SDL_HAT_RIGHT
                    _ => anyhow::bail!("Nestopia needs one cardinal SDL hat direction"),
                };
                Binding::Hat0 { direction }
            }
        };
        ensure!(
            bindings.insert(row.target_id, binding).is_none(),
            "Duplicate Nestopia gameplay target"
        );
    }
    let pad = Pad {
        player,
        joystick: device.device_index.try_into()?,
        bindings,
    };
    // Exercise all representability and uniqueness checks before returning.
    for binding in pad.bindings.values() {
        binding.code(pad.joystick)?;
    }
    Ok(pad)
}
