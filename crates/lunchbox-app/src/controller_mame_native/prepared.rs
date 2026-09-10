//! Compose a controller profile from caller-owned, same-runtime SDL evidence.
//! This is not a probe or a launch dispatcher; topology and freshness remain
//! obligations of the caller before and after child startup.
use super::settings::SavedSetup;
use crate::controller_catalog::Calibration;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    duckstation::DigitalInput, linux_classic::AxisEndpoints, sdl2::Device,
    sdl2_physical::PhysicalMap,
};
use std::collections::{BTreeSet, HashMap};

/// Fill launch-local declarations from physical device evidence. The normal
/// preparation below still verifies every translated item, released state,
/// unique GUID and route before any native configuration can be emitted.
pub(crate) fn resolve_guided(
    setup: &SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    physical_paths: &HashMap<String, String>,
    devices: &[Device],
) -> Result<SavedSetup> {
    setup.validate()?;
    let mut resolved = setup.clone();
    if !setup.resolve_inputs_at_launch {
        return Ok(resolved);
    }
    let threshold = f32::from(setup.threshold_basis_points) / 10_000.0;
    for player in &mut resolved.players {
        let path = physical_paths
            .get(&player.controller_id)
            .context("MAME controller path is missing")?;
        player.native_device_id = super::sdl::native_id(path, devices)?.into();
        let device = devices
            .iter()
            .find(|device| device.path.as_deref() == Some(path.as_str()))
            .context("MAME physical SDL device is missing")?;
        let calibration = calibrations
            .get(&player.controller_id)
            .context("MAME physical calibration is missing")?;
        calibration.validate()?;
        ensure!(
            calibration.os == "linux" && calibration.backend != crate::controller_sdl3::BACKEND,
            "MAME needs physical Linux inputs, not SDL3 logical codes"
        );
        for (target, source) in &player.source_controls {
            let input = calibration
                .bindings
                .get(source)
                .context("MAME source calibration is missing")?;
            let native = input
                .native
                .as_ref()
                .context("MAME source has no physical input")?;
            let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            player.controls.insert(
                target.clone(),
                super::sdl::switch_item(device, native.code, measured, threshold)?,
            );
        }
    }
    resolved.resolve_inputs_at_launch = false;
    resolved.validate()?;
    Ok(resolved)
}

pub(crate) fn controller_xml(
    setup: &SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    physical_paths: &HashMap<String, String>,
    devices: &[Device],
) -> Result<String> {
    setup.review(calibrations)?;
    ensure!(
        setup.joystick_provider == "sdl",
        "MAME physical translation currently requires the raw SDL provider, without Sixaxis mode"
    );
    let threshold = f32::from(setup.threshold_basis_points) / 10_000.0;
    let mut paths = BTreeSet::new();
    for player in &setup.players {
        ensure!(
            player.source_controls.len() == player.controls.len(),
            "MAME launch preparation needs every physical source link"
        );
        let path = physical_paths
            .get(&player.controller_id)
            .context("MAME physical controller path is missing")?;
        ensure!(
            paths.insert(path),
            "MAME players resolve to the same physical path"
        );
        let native_id = super::sdl::native_id(path, devices)?;
        ensure!(
            native_id == player.native_device_id,
            "MAME native device ID differs from saved setup"
        );
        let device = devices
            .iter()
            .find(|device| device.path.as_deref() == Some(path.as_str()))
            .context("MAME SDL device disappeared")?;
        let map = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("MAME SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("MAME SDL counts are missing")?,
        )?;
        let calibration = calibrations
            .get(&player.controller_id)
            .context("MAME physical calibration is missing")?;
        ensure!(
            calibration.os == "linux",
            "MAME physical backend translation requires Linux calibration"
        );
        for (target, source) in &player.source_controls {
            let input = calibration
                .bindings
                .get(source)
                .context("MAME source calibration disappeared")?;
            let native = input
                .native
                .as_ref()
                .context("MAME source needs native physical calibration")?;
            let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let translated = super::sdl::switch_item(device, native.code, measured, threshold)?;
            ensure!(
                player.controls.get(target) == Some(&translated),
                "MAME declared native item differs from measured physical source for {target}"
            );
            let released = match map.digital_input(native.code, measured)? {
                DigitalInput::Button(index) => state.buttons.get(&index) == Some(&false),
                DigitalInput::Hat { index, direction } => state
                    .hats
                    .get(&index)
                    .is_some_and(|mask| mask & direction == 0),
                DigitalInput::Axis {
                    index, released, ..
                } => state.axes.get(&index) == Some(&released),
            };
            ensure!(
                released,
                "Release the MAME panel controls before launch preparation"
            );
        }
    }
    let ids = devices
        .iter()
        .map(|device| device.guid.clone())
        .collect::<Vec<_>>();
    setup.render(&ids)
}
