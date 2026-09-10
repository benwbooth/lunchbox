//! Measured physical sources to native raw SDL2 input encodings.
use super::{Activation, Input};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    duckstation::DigitalInput, linux_classic::AxisEndpoints, sdl2::Device,
    sdl2_physical::PhysicalMap,
};

pub(crate) fn switch_binding(
    device: &Device,
    code: u32,
    endpoints: Option<AxisEndpoints>,
) -> Result<Input> {
    let physical = PhysicalMap::from_device(device)?;
    let state = device
        .sampled_state
        .as_ref()
        .context("melonDS released SDL state is missing")?;
    state.validate(
        device
            .controls
            .as_ref()
            .context("melonDS SDL counts are missing")?,
    )?;
    let input = match physical.digital_input(code, endpoints)? {
        DigitalInput::Button(index) => {
            ensure!(
                state.buttons.get(&index) == Some(&false),
                "Release melonDS source button"
            );
            Input::Button(u16::try_from(index)?)
        }
        DigitalInput::Hat { index, direction } => {
            ensure!(
                state
                    .hats
                    .get(&index)
                    .is_some_and(|value| value & direction == 0),
                "Release melonDS source hat direction"
            );
            Input::Hat {
                index: u8::try_from(index)?,
                direction,
            }
        }
        DigitalInput::Axis {
            index,
            released,
            pressed,
        } => {
            ensure!(
                state.axes.get(&index) == Some(&released),
                "Release melonDS source axis"
            );
            let activation = if released < -16384 && pressed > 0 {
                Activation::Trigger
            } else if pressed > released {
                Activation::Positive
            } else {
                Activation::Negative
            };
            activation.validate_endpoints(released, pressed)?;
            Input::Axis {
                index: u8::try_from(index)?,
                activation,
            }
        }
    };
    input.encode()?;
    Ok(input)
}

pub(crate) fn controls(
    calibration: &crate::controller_catalog::Calibration,
    sources: &std::collections::BTreeMap<String, String>,
    device: &Device,
) -> Result<std::collections::BTreeMap<String, Input>> {
    calibration.validate()?;
    ensure!(
        calibration.os == "linux",
        "melonDS physical mapping currently requires Linux calibration"
    );
    let routes = super::visual_routes();
    ensure!(
        sources.len() == routes.len() && routes.keys().all(|key| sources.contains_key(*key)),
        "melonDS needs all twelve destination controls"
    );
    let mut used = std::collections::BTreeSet::new();
    let mut result = std::collections::BTreeMap::new();
    for (target, source) in sources {
        ensure!(
            used.insert(source),
            "melonDS destination sources must be distinct"
        );
        let binding = calibration
            .bindings
            .get(source)
            .context("melonDS source is not calibrated")?;
        let native = binding
            .native
            .as_ref()
            .context("melonDS source needs native calibration")?;
        let endpoints = binding.axis.as_ref().map(|axis| AxisEndpoints {
            released: axis.released,
            pressed: axis.pressed,
        });
        result.insert(
            routes[target.as_str()].to_owned(),
            switch_binding(device, native.code, endpoints)?,
        );
    }
    Ok(result)
}
