//! Flycast SDLControllerMappingParser and SDLGamepad::find_mapping semantics.
//! Pin fb286f777ce690ef8acf3359a75ab84b61566ad9/core/sdl/.
use super::physical::AxisMode;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::sdl2::Device;
use std::collections::BTreeMap;

pub(crate) fn effective_modes(
    device: &Device,
    saved_triggers: &BTreeMap<u32, bool>,
) -> Result<BTreeMap<u32, AxisMode>> {
    let count = device
        .controls
        .as_ref()
        .context("Flycast SDL counts are absent")?
        .axes;
    ensure!(count <= 256, "Flycast SDL axis count exceeds event range");
    let mut modes = (0..count)
        .map(|index| (index, AxisMode::Ordinary))
        .collect::<BTreeMap<_, _>>();
    if !saved_triggers.is_empty() {
        for (index, reversed) in saved_triggers {
            *modes
                .get_mut(index)
                .context("Flycast saved trigger references an absent axis")? = AxisMode::Trigger {
                reversed: *reversed,
            };
        }
        return Ok(modes);
    }
    if !device.is_game_controller {
        return Ok(modes);
    }
    let mapping = device
        .mapping
        .as_ref()
        .context("Flycast recognized SDL gamepad needs its effective controller mapping")?;
    ensure!(
        mapping.len() <= 1024 * 1024 && !mapping.contains('\0'),
        "Flycast SDL mapping is malformed or oversized"
    );
    // Native find_mapping queries left then right; repeated axis entries follow
    // this same ordering. Native parser searches substrings, not split key names.
    for name in ["lefttrigger:", "righttrigger:"] {
        let Some(offset) = mapping.find(name) else {
            continue;
        };
        let value = mapping[offset + name.len()..]
            .split(',')
            .next()
            .unwrap_or("");
        if value.starts_with('b') || value.starts_with('h') {
            continue;
        }
        let (half, value) =
            if let Some(value) = value.strip_prefix('+').or_else(|| value.strip_prefix('-')) {
                (true, value)
            } else {
                (false, value)
            };
        let Some(axis) = value.strip_prefix('a') else {
            continue;
        };
        let reversed = axis.ends_with('~');
        let number = axis.strip_suffix('~').unwrap_or(axis);
        ensure!(
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()),
            "Unsupported Flycast SDL trigger axis encoding"
        );
        let index: u32 = number.parse()?;
        ensure!(index < count, "Flycast SDL trigger axis is absent");
        // A trailing inversion marker wins over a half-axis prefix in the
        // native parser. Otherwise half axes do not become classified triggers.
        if reversed || !half {
            modes.insert(index, AxisMode::Trigger { reversed });
        }
    }
    Ok(modes)
}
