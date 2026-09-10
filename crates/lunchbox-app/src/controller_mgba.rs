//! Native mGBA SDL 0.10.5 joystick configuration, not libretro controls.
//! Source pin: 26b7884bc25a5933960f3cdcd98bac1ae14d42e2.
//! Inputs are measured SDL joystick coordinates, never SDL GameController
//! logical button ordinals or raw evdev numbers without backend translation.
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::sdl2::ControlCounts;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod configuration;
#[cfg(target_os = "linux")]
pub(crate) mod guided;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod settings;

// GBAInputInfo order is also the integer stored in native hat bindings.
pub(crate) const KEYS: [&str; 10] = [
    "A", "B", "Select", "Start", "Right", "Left", "Up", "Down", "R", "L",
];

pub(crate) fn render_calibrated(
    original: &str,
    calibration: &crate::controller_catalog::Calibration,
    snapshot: &lunchbox_controller_probe::sdl2::Snapshot,
    runtime_path: &str,
    gba: bool,
) -> Result<String> {
    use lunchbox_controller_probe::{
        duckstation::DigitalInput,
        linux_classic::{AxisEndpoints, Control},
        sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "mGBA physical translation requires Linux calibration"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    ensure!(
        snapshot
            .devices
            .iter()
            .filter(|other| other.guid == device.guid)
            .count()
            == 1,
        "mGBA GUID preference cannot distinguish these identical controllers"
    );
    let counts = device
        .controls
        .as_ref()
        .context("mGBA SDL counts were not captured")?;
    let released = device
        .sampled_state
        .as_ref()
        .context("mGBA released state was not captured")?;
    released.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let id = if gba {
        "mgba:standalone-gba"
    } else {
        "mgba:standalone-gameboy"
    };
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == id)
        .context("Missing native mGBA controller profile")?;
    let mut controls = BTreeMap::new();
    for row in calibration.plan_profile(profile)?.rows {
        let input = row.input.context("mGBA gameplay input is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("mGBA requires measured native controls")?;
        let binding = match physical.control(native.code)? {
            Control::Button(index) => {
                ensure!(
                    released.buttons.get(&index) == Some(&false),
                    "Release mGBA controller buttons before capture"
                );
                JoystickInput::Button(index)
            }
            Control::Axis(index) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("mGBA axis endpoints are absent")?;
                let rest = physical.axis_value((native.code & 0xffff) as u8, endpoints.released)?;
                let pressed =
                    physical.axis_value((native.code & 0xffff) as u8, endpoints.pressed)?;
                ensure!(
                    released.axes.get(&index) == Some(&rest) && rest != pressed,
                    "mGBA captured axis rest differs from its measured calibration"
                );
                let threshold = ((i32::from(rest) + i32::from(pressed)) / 2) as i16;
                let positive = pressed > rest;
                ensure!(
                    if positive {
                        rest <= threshold && pressed > threshold
                    } else {
                        rest >= threshold && pressed < threshold
                    },
                    "mGBA axis gesture cannot cross its threshold"
                );
                JoystickInput::Axis {
                    index,
                    positive,
                    threshold,
                }
            }
            Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("mGBA hat endpoints are absent")?;
                let DigitalInput::Hat { index, direction } = physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )?
                else {
                    anyhow::bail!("mGBA directional hat did not resolve to an SDL hat");
                };
                ensure!(
                    released.hats.get(&index) == Some(&0),
                    "Release mGBA directional hat before capture"
                );
                let direction = match direction {
                    1 => HatDirection::Up,
                    2 => HatDirection::Right,
                    4 => HatDirection::Down,
                    8 => HatDirection::Left,
                    _ => anyhow::bail!("mGBA requires a cardinal hat direction"),
                };
                JoystickInput::Hat { index, direction }
            }
        };
        ensure!(
            controls.insert(row.output, binding).is_none(),
            "Duplicate mGBA gameplay target"
        );
    }
    render_config(original, &device.guid, counts, gba, &controls)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JoystickInput {
    Button(u32),
    Axis {
        index: u32,
        positive: bool,
        threshold: i16,
    },
    Hat {
        index: u32,
        direction: HatDirection,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HatDirection {
    Up,
    Right,
    Down,
    Left,
}

impl HatDirection {
    fn name(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Right => "Right",
            Self::Down => "Down",
            Self::Left => "Left",
        }
    }
}

/// Eight Game Boy controls or ten GBA controls, according to target layout.
/// GUID is only the native profile selector. The launcher must first resolve
/// the selected physical device and reject ambiguous identical-GUID pads.
pub(crate) fn render_config(
    original: &str,
    guid: &str,
    counts: &ControlCounts,
    gba: bool,
    controls: &BTreeMap<String, JoystickInput>,
) -> Result<String> {
    ensure!(
        guid.len() == 32 && guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "mGBA requires a resolved SDL2 joystick GUID"
    );
    ensure!(
        counts.axes <= 512 && counts.buttons <= 512 && counts.hats <= 64,
        "mGBA controller counts exceed the supported bounds"
    );
    let expected = if gba { &KEYS[..] } else { &KEYS[..8] };
    ensure!(
        controls.len() == expected.len() && expected.iter().all(|key| controls.contains_key(*key)),
        "mGBA requires every gameplay control for the selected handheld layout"
    );
    let mut fields = BTreeMap::new();
    // mInputMapLoad does not clear existing axes, including SDL's defaults.
    // Move each otherwise unused target off any inherited axis onto a disabled
    // high direction. SDL values cannot exceed 32767; the chosen axis is also
    // beyond the captured device's event range. This is native unbinding, not
    // an assumed empty config. Real measured axes replace it below.
    for key in KEYS {
        fields.insert(format!("key{key}"), "-1".to_owned());
        fields.insert(format!("axis{key}Axis"), format!("+{}", counts.axes));
        fields.insert(format!("axis{key}Value"), "32767".to_owned());
    }
    // _loadAll stops at the first absent hat, so emit a contiguous range and
    // explicitly clear the default hat zero even for a hatless controller.
    for hat in 0..counts.hats.max(1) {
        for direction in ["Up", "Right", "Down", "Left"] {
            fields.insert(format!("hat{hat}{direction}"), "-1".to_owned());
        }
    }
    let mut occupied = BTreeSet::new();
    let mut axes = BTreeMap::<u32, (Option<i16>, Option<i16>)>::new();
    for (target, input) in controls {
        let identity = match *input {
            JoystickInput::Button(index) => {
                ensure!(
                    index < counts.buttons,
                    "mGBA button is outside captured SDL counts"
                );
                fields.insert(format!("key{target}"), index.to_string());
                format!("b{index}")
            }
            JoystickInput::Axis {
                index,
                positive,
                threshold,
            } => {
                ensure!(
                    index < counts.axes,
                    "mGBA axis is outside captured SDL counts"
                );
                ensure!(
                    if positive {
                        threshold < i16::MAX
                    } else {
                        threshold > i16::MIN
                    },
                    "mGBA axis threshold cannot be activated"
                );
                let pair = axes.entry(index).or_default();
                if positive {
                    pair.1 = Some(threshold);
                } else {
                    pair.0 = Some(threshold);
                }
                let sign = if positive { '+' } else { '-' };
                fields.insert(format!("axis{target}Axis"), format!("{sign}{index}"));
                fields.insert(format!("axis{target}Value"), threshold.to_string());
                format!("a{index}{sign}")
            }
            JoystickInput::Hat { index, direction } => {
                ensure!(
                    index < counts.hats,
                    "mGBA hat is outside captured SDL counts"
                );
                let key = format!("hat{index}{}", direction.name());
                let ordinal = KEYS
                    .iter()
                    .position(|key| key == target)
                    .expect("validated target");
                fields.insert(key.clone(), ordinal.to_string());
                key
            }
        };
        ensure!(
            occupied.insert(identity),
            "mGBA physical input has multiple gameplay owners"
        );
    }
    for (low, high) in axes.values() {
        if let (Some(low), Some(high)) = (low, high) {
            ensure!(low <= high, "mGBA opposite axis thresholds overlap");
        }
    }
    let profile = format!("gba.input-profile.{}", guid.to_ascii_lowercase());
    let mut result = replace_fields(original, "gba.input.SDLB", &fields, counts.hats.max(1));
    result = replace_fields(&result, &profile, &fields, counts.hats.max(1));
    // The SDL frontend selects its preferred joystick by GUID, not an index.
    // This preference is not unique identity and must be guarded at launch.
    result = replace_fields(
        &result,
        "gba.input.SDLB",
        &BTreeMap::from([("device0".to_owned(), guid.to_ascii_lowercase())]),
        0,
    );
    Ok(result)
}

fn replace_fields(
    original: &str,
    section: &str,
    fields: &BTreeMap<String, String>,
    hats: u32,
) -> String {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut result = String::new();
    let mut active = false;
    let mut inserted = false;
    for line in original.split_inclusive('\n') {
        let text = line.trim();
        if text.starts_with('[') && text.ends_with(']') {
            active = &text[1..text.len() - 1] == section;
            result.push_str(line);
            if active && !inserted {
                if !result.ends_with('\n') {
                    result.push_str(newline);
                }
                for (key, value) in fields {
                    result.push_str(&format!("{key}={value}{newline}"));
                }
                inserted = true;
            }
        } else {
            let owned = text.split_once('=').is_some_and(|(key, _)| {
                let key = key.trim();
                fields.contains_key(key)
                    || (hats > 0
                        && key.strip_prefix("hat").is_some_and(|tail| {
                            ["Up", "Right", "Down", "Left"].iter().any(|direction| {
                                tail.strip_suffix(direction).is_some_and(|index| {
                                    !index.is_empty()
                                        && index.bytes().all(|byte| byte.is_ascii_digit())
                                })
                            })
                        }))
            });
            if !active || !owned {
                result.push_str(line);
            }
        }
    }
    if !inserted {
        if !result.is_empty() && !result.ends_with('\n') {
            result.push_str(newline);
        }
        result.push_str(&format!("[{section}]{newline}"));
        for (key, value) in fields {
            result.push_str(&format!("{key}={value}{newline}"));
        }
    }
    result
}
