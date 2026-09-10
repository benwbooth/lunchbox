//! Translate resolved raw SDL2 controls into SameBoy SDL's native input space.
use super::Input;
use anyhow::{Result, ensure};

pub(crate) fn calibrated_bindings(
    calibration: &crate::controller_catalog::Calibration,
    snapshot: &lunchbox_controller_probe::sdl2::Snapshot,
    runtime_path: &str,
) -> Result<super::Bindings> {
    use anyhow::Context;
    use lunchbox_controller_probe::{
        duckstation::DigitalInput,
        linux_classic::{AxisEndpoints, Control},
        sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "SameBoy native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let counts = device
        .controls
        .as_ref()
        .context("SameBoy SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("SameBoy released state is absent")?;
    state.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == "sameboy:standalone-sdl-gameboy")
        .context("Missing native SameBoy profile")?;
    let mut bindings = std::collections::BTreeMap::new();
    for row in calibration.plan_profile(profile)?.rows {
        let input = row
            .input
            .context("SameBoy gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("SameBoy requires measured native controls")?;
        let control = match physical.control(native.code)? {
            Control::Button(index) => {
                ensure!(
                    state.buttons.get(&index) == Some(&false),
                    "Release SameBoy controller buttons before capture"
                );
                Input::Button(index)
            }
            Control::Axis(index) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("SameBoy axis measurements are absent")?;
                let released =
                    physical.axis_value((native.code & 0xffff) as u8, endpoints.released)?;
                let pressed =
                    physical.axis_value((native.code & 0xffff) as u8, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&index) == Some(&released),
                    "SameBoy axis rest differs from calibration"
                );
                Input::Axis {
                    index,
                    released,
                    pressed,
                }
            }
            Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("SameBoy hat measurements are absent")?;
                let DigitalInput::Hat { index, direction } = physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )?
                else {
                    anyhow::bail!("SameBoy hat did not resolve to a native SDL hat");
                };
                ensure!(
                    state.hats.get(&index) == Some(&0),
                    "Release SameBoy directional hat before capture"
                );
                Input::Hat {
                    index,
                    direction: direction.try_into()?,
                }
            }
        };
        ensure!(
            bindings.insert(row.output, control).is_none(),
            "Duplicate SameBoy gameplay target"
        );
    }
    super::Bindings::calibrated(&bindings, counts.buttons, counts.axes, counts.hats)
}
