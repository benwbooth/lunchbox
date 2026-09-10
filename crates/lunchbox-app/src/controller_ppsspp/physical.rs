//! Saved physical calibration -> target SDL2 logical inputs -> PSP settings.
//! The caller supplies a fresh snapshot and captured released state from the
//! same PPSSPP runtime. This module performs no captures or process execution.
use super::{SdlInput, calibrated_controls};
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    duckstation::DigitalInput,
    linux_classic::{AxisEndpoints, Control},
    sdl2::Snapshot,
    sdl2_mapping::{Input, InputState, MappingContext, changed_outputs, parse},
    sdl2_physical::PhysicalMap,
};
use std::collections::BTreeMap;

pub(crate) fn resolve(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
    released: &InputState,
) -> Result<BTreeMap<String, SdlInput>> {
    ensure!(
        calibration.os == "linux",
        "PPSSPP physical translation requires a Linux calibration"
    );
    let context = MappingContext::capture(snapshot, runtime_path)?;
    let device = snapshot.device_at_path(runtime_path)?;
    let physical = PhysicalMap::from_device(device)?;
    let mapping = parse(&context.mapping)?;
    released.outputs(&mapping, &context.counts)?;
    let profile = catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == "ppsspp:standalone-psp")
        .context("Missing standalone PPSSPP profile")?;
    let plan = calibration.plan_profile(profile)?;
    let mut inputs = BTreeMap::new();
    for row in plan.rows {
        let id = row
            .physical_id
            .context("Missing required PPSSPP physical control")?;
        let input = row
            .input
            .context("PPSSPP physical control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("PPSSPP requires native physical calibration")?;
        let mut pressed = released.clone();
        let mut analog_interval = None;
        match physical.control(native.code)? {
            Control::Button(index) => {
                ensure!(
                    released.buttons.get(&index) == Some(&false),
                    "PPSSPP released capture contains a held button"
                );
                pressed.buttons.insert(index, true);
            }
            Control::Axis(index) => {
                let measurement = input
                    .axis
                    .as_ref()
                    .context("PPSSPP physical axis has no measured endpoints")?;
                let rest =
                    physical.axis_value((native.code & 0xffff) as u8, measurement.released)?;
                let extent =
                    physical.axis_value((native.code & 0xffff) as u8, measurement.pressed)?;
                ensure!(
                    released.axes.get(&index) == Some(&rest),
                    "PPSSPP released axis state differs from calibration"
                );
                pressed.axes.insert(index, extent);
                analog_interval = Some((
                    index,
                    i32::from(rest.min(extent)),
                    i32::from(rest.max(extent)),
                ));
            }
            Control::HatAxis { .. } => {
                let measurement = input
                    .axis
                    .as_ref()
                    .context("PPSSPP hat has no measured endpoints")?;
                let DigitalInput::Hat { index, direction } = physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: measurement.released,
                        pressed: measurement.pressed,
                    }),
                )?
                else {
                    anyhow::bail!("PPSSPP physical hat did not translate as a hat");
                };
                ensure!(
                    released.hats.get(&index) == Some(&0),
                    "Release PPSSPP directional hat before capture"
                );
                pressed.hats.insert(index, direction);
            }
        }
        let changes = changed_outputs(&mapping, &context.counts, released, &pressed)?;
        ensure!(
            changes.len() == 1,
            "PPSSPP control {id} resolves to {} logical outputs, not one",
            changes.len()
        );
        let change = &changes[0];
        let analog_target = row.output.starts_with("An.");
        if analog_target {
            let (axis, low, high) = analog_interval
                .context("PPSSPP analog target requires a continuous physical axis")?;
            let routes: Vec<_> = mapping
                .iter()
                .filter(|binding| binding.output == change.output)
                .collect();
            ensure!(
                routes.len() == 1,
                "PPSSPP analog output needs an unambiguous continuous SDL mapping"
            );
            let route = routes[0];
            ensure!(mapping.iter().filter(|binding| matches!(binding.input, Input::Axis { index, .. } if index == axis)).count() == 1,
                "PPSSPP analog source has competing SDL routes across its travel");
            ensure!(
                route.output_range.is_some()
                    && matches!(route.input,
                Input::Axis { index, minimum, maximum } if index == axis && minimum.min(maximum) <= low && minimum.max(maximum) >= high),
                "PPSSPP analog mapping does not cover the full measured interval"
            );
        }
        let resolved = if change.analog {
            ensure!(
                change.released.abs() <= 1 && change.pressed != 0,
                "PPSSPP axis mapping must start at logical rest and move in one direction"
            );
            let index = match change.output.as_str() {
                "leftx" => 0,
                "lefty" => 1,
                "rightx" => 2,
                "righty" => 3,
                "lefttrigger" => 4,
                "righttrigger" => 5,
                _ => anyhow::bail!("Unknown PPSSPP SDL axis"),
            };
            SdlInput::Axis {
                index,
                direction: if change.pressed < 0 { -1 } else { 1 },
            }
        } else {
            ensure!(
                !analog_target && change.released == 0 && change.pressed == 1,
                "PPSSPP button mapping is not a released-to-pressed digital transition"
            );
            let names = [
                "a",
                "b",
                "x",
                "y",
                "back",
                "guide",
                "start",
                "leftstick",
                "rightstick",
                "leftshoulder",
                "rightshoulder",
                "dpup",
                "dpdown",
                "dpleft",
                "dpright",
            ];
            let index = names
                .iter()
                .position(|name| *name == change.output)
                .context("Unsupported PPSSPP SDL logical button")?;
            SdlInput::Button(index as u8)
        };
        if let Some(previous) = inputs.insert(id, resolved) {
            ensure!(
                previous == resolved,
                "PPSSPP physical control resolves inconsistently"
            );
        }
    }
    calibrated_controls(calibration, &inputs)
}
