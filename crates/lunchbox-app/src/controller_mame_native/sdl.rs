//! MAME 0.280 raw `sdl` provider; not `sdlgame` or Sixaxis special handling.
//! Pinned input_sdl.cpp passes the GUID alone as the native device ID, even
//! though it constructs a GUID-plus-serial string locally. Do not use that string.
use super::tokens::{Axis, Direction, Item};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::{sdl2::Device, sdl2_physical::PhysicalMap};

/// Inventory must come from the same SDL runtime/backend intended for MAME.
/// The GUID is acceptable only after proving uniqueness within that inventory.
pub(crate) fn native_id<'a>(selected_path: &str, devices: &'a [Device]) -> Result<&'a str> {
    let matches: Vec<_> = devices
        .iter()
        .filter(|device| device.path.as_deref() == Some(selected_path))
        .collect();
    ensure!(
        matches.len() == 1,
        "MAME SDL physical path is absent or ambiguous"
    );
    let device = matches[0];
    ensure!(
        device.guid.len() == 32
            && device
                .guid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            && device.guid.bytes().any(|byte| byte != b'0'),
        "MAME SDL GUID is missing or malformed"
    );
    ensure!(
        devices
            .iter()
            .filter(|other| other.guid.eq_ignore_ascii_case(&device.guid))
            .count()
            == 1,
        "MAME SDL GUID is shared by multiple controllers; stable mapdevice assignment is ambiguous"
    );
    Ok(&device.guid)
}

/// Translate measured controls through the physical-to-SDL map and raw MAME
/// item numbering. The caller must apply this same threshold at native launch.
pub(crate) fn switch_item(
    device: &Device,
    physical_code: u32,
    measured: Option<lunchbox_controller_probe::linux_classic::AxisEndpoints>,
    threshold: f32,
) -> Result<Item> {
    ensure!(
        threshold.is_finite() && (0.0..=1.0).contains(&threshold),
        "MAME joystick threshold must be between zero and one"
    );
    use lunchbox_controller_probe::duckstation::DigitalInput;
    let mapped = PhysicalMap::from_device(device)?.digital_input(physical_code, measured)?;
    match mapped {
        DigitalInput::Button(index) => {
            ensure!(index < 32, "MAME standard button range exceeded");
            Ok(Item::Button {
                number: (index + 1).try_into()?,
            })
        }
        DigitalInput::Hat { index, direction } => {
            ensure!(index < 4, "MAME standard hat range exceeded");
            let direction = match direction {
                1 => Direction::Up,
                2 => Direction::Right,
                4 => Direction::Down,
                8 => Direction::Left,
                _ => anyhow::bail!("MAME panel requires a single cardinal hat direction"),
            };
            Ok(Item::Hat {
                number: (index + 1).try_into()?,
                direction,
            })
        }
        DigitalInput::Axis {
            index,
            released,
            pressed,
        } => {
            let axes = [
                Axis::X,
                Axis::Y,
                Axis::Z,
                Axis::Rx,
                Axis::Ry,
                Axis::Rz,
                Axis::Slider1,
                Axis::Slider2,
            ];
            let axis = *axes
                .get(index as usize)
                .ok_or_else(|| anyhow::anyhow!("MAME standard absolute axis range exceeded"))?;
            // Raw SDL provider multiplies signed SDL values by two. POS/NEG
            // modifiers bypass the X/Y joystick map and compare raw values.
            let positive = pressed > released;
            let magnitude = |value: i16| {
                let raw = i32::from(value) * 2;
                if positive { raw } else { -raw }
            };
            let native_threshold = ((threshold * 65_536.0_f32) as i32).max(1);
            ensure!(
                magnitude(released) < native_threshold && magnitude(pressed) >= native_threshold,
                "MAME threshold cannot distinguish this axis's measured rest and press"
            );
            Ok(Item::AxisHalf { axis, positive })
        }
    }
}
