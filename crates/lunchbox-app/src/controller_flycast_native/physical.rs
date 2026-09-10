//! Physical-to-SDL-to-Flycast switch translation, pinned core/sdl/sdl.cpp.
use super::Input;

/// Must come from Flycast's effective trigger list or the same SDL controller
/// mapping parser. An empty saved trigger list can be populated by Flycast.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AxisMode {
    Ordinary,
    Trigger { reversed: bool },
}

pub(crate) fn digital_axis_binding(
    index: u32,
    released: i16,
    pressed: i16,
    current: i16,
    mode: AxisMode,
) -> Result<Input> {
    ensure!(index <= 255, "Flycast SDL axis index exceeds event range");
    ensure!(
        current == released,
        "Release Flycast axis before preparation"
    );
    let positive = match mode {
        AxisMode::Ordinary => {
            let positive = pressed > released;
            let magnitude = |value: i16| {
                if positive {
                    i32::from(value)
                } else {
                    -i32::from(value)
                }
            };
            ensure!(
                magnitude(released) < 16_384 && magnitude(pressed) >= 16_384,
                "Flycast axis gesture does not cross its native digital threshold"
            );
            positive
        }
        AxisMode::Trigger { reversed } => {
            let active = |value: i16| {
                if reversed {
                    i32::from(value) <= 32_667
                } else {
                    i32::from(value) >= -32_668
                }
            };
            ensure!(
                !active(released) && active(pressed),
                "Flycast trigger gesture does not cross its native activation threshold"
            );
            // Native triggers always select the positive mapping slot, even
            // when their numerical press direction is reversed.
            true
        }
    };
    Ok(Input::AxisHalf {
        code: index,
        positive,
    })
}
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    duckstation::DigitalInput, linux_classic::AxisEndpoints, sdl2::Device,
    sdl2_physical::PhysicalMap,
};

pub(crate) fn switch_binding(
    device: &Device,
    physical_code: u32,
    measured: Option<AxisEndpoints>,
    axis_modes: &std::collections::BTreeMap<u32, AxisMode>,
) -> Result<Input> {
    let map = PhysicalMap::from_device(device)?;
    let state = device
        .sampled_state
        .as_ref()
        .context("Flycast SDL released state is missing")?;
    state.validate(
        device
            .controls
            .as_ref()
            .context("Flycast SDL control counts are missing")?,
    )?;
    match map.digital_input(physical_code, measured)? {
        DigitalInput::Button(index) => {
            // SDL_JoyButtonEvent.button is Uint8. Flycast uses it unchanged;
            // code 256 onward is reserved for synthetic hat-button identities.
            ensure!(index <= 255, "Flycast SDL button index exceeds event range");
            ensure!(
                state.buttons.get(&index) == Some(&false),
                "Release Flycast controller buttons before preparation"
            );
            Ok(Input::Button(index))
        }
        DigitalInput::Hat { index, direction } => {
            ensure!(index <= 255, "Flycast SDL hat index exceeds event range");
            let offset = match direction {
                1 => 0,
                4 => 1,
                8 => 2,
                2 => 3,
                _ => anyhow::bail!("Flycast panel directions require a cardinal hat input"),
            };
            ensure!(
                state
                    .hats
                    .get(&index)
                    .is_some_and(|value| value & direction == 0),
                "Release Flycast controller hats before preparation"
            );
            Ok(Input::Button(((index + 1) << 8) + offset))
        }
        DigitalInput::Axis {
            index,
            released,
            pressed,
        } => digital_axis_binding(
            index,
            released,
            pressed,
            *state
                .axes
                .get(&index)
                .context("Flycast sampled axis value is absent")?,
            *axis_modes
                .get(&index)
                .context("Flycast axis has no effective trigger classification")?,
        ),
    }
}

/// Build one ordinary arcade panel using explicit physical layout links and
/// externally resolved native axis classification. No device discovery here.
pub(crate) fn panel_mapping(
    calibration: &crate::controller_catalog::Calibration,
    sources: &std::collections::BTreeMap<String, String>,
    device: &Device,
    panel: super::arcade::Panel,
    axis_modes: &std::collections::BTreeMap<u32, AxisMode>,
) -> Result<String> {
    use std::collections::{BTreeMap, BTreeSet};
    calibration.validate()?;
    ensure!(
        calibration.os == "linux",
        "Flycast physical SDL translation requires Linux calibration"
    );
    let counts = device
        .controls
        .as_ref()
        .context("Flycast SDL control counts are absent")?;
    ensure!(
        axis_modes.len() == counts.axes as usize
            && (0..counts.axes).all(|index| axis_modes.contains_key(&index)),
        "Flycast preparation requires effective classification for every SDL axis"
    );
    let routes = panel.routes();
    ensure!(
        sources.len() == routes.len() && routes.keys().all(|target| sources.contains_key(target)),
        "Flycast panel needs every physical source link"
    );
    let mut owners = BTreeSet::new();
    let mut physical = BTreeMap::new();
    for (target, source) in sources {
        ensure!(
            owners.insert(source),
            "Flycast panel repeats a physical layout control"
        );
        let input = calibration
            .bindings
            .get(source)
            .context("Flycast panel source has no calibration")?;
        let native = input
            .native
            .as_ref()
            .context("Flycast panel requires native physical calibration")?;
        if let Some(axis) = &input.axis {
            axis.validate()?;
        }
        let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
            released: axis.released,
            pressed: axis.pressed,
        });
        physical.insert(
            target.clone(),
            switch_binding(device, native.code, measured, axis_modes)?,
        );
    }
    let native = panel.native_controls(&physical)?;
    let triggers = axis_modes
        .iter()
        .filter_map(|(index, mode)| match mode {
            AxisMode::Trigger { reversed } => Some((*index, *reversed)),
            AxisMode::Ordinary => None,
        })
        .collect();
    // Ordinary arcade targets are digital; radial stick dead zone and
    // saturation do not alter these targets' native digital thresholds.
    super::mapping_with_triggers(
        &[super::Port {
            port: 0,
            controls: &native,
        }],
        10,
        100,
        &triggers,
    )
}

pub(crate) fn panel_mapping_from_sdl(
    calibration: &crate::controller_catalog::Calibration,
    sources: &std::collections::BTreeMap<String, String>,
    device: &Device,
    panel: super::arcade::Panel,
    saved_triggers: &std::collections::BTreeMap<u32, bool>,
) -> Result<String> {
    let modes = super::triggers::effective_modes(device, saved_triggers)?;
    panel_mapping(calibration, sources, device, panel, &modes)
}
