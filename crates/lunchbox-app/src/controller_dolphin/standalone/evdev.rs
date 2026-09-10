//! Dolphin 2606 evdev numbering/normalization from captured kernel metadata.
//! Resolution consumes supplied observations; capture_node explicitly reads a
//! device only when called. Native device qualifier/node ownership must be
//! established by the launch layer, not inferred from the device name.
use super::{Pad, ResolvedControl, analog_target, calibrated_pad};
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::sdl2_evdev::AxisInfo;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeObservation {
    pub path: PathBuf,
    pub motion: bool,
    pub pointing: bool,
    /// Every supported EV_KEY code, including keys below BTN_MISC.
    pub keys: BTreeSet<u16>,
    pub axes: BTreeMap<u8, AxisInfo>,
    pub pressed_keys: BTreeSet<u16>,
    pub axis_values: BTreeMap<u8, i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeviceObservation {
    /// Native Dolphin backend/id/name qualifier resolved by its inventory.
    pub qualifier: String,
    /// Complete node membership of this native device, including merged nodes.
    pub nodes: Vec<NodeObservation>,
}

/// Read kernel capabilities/current state without consuming input events or
/// writing device settings. This is called only during explicit launch/capture,
/// never by catalog validation or saved-settings review.
#[cfg(target_os = "linux")]
pub(crate) fn capture_node(path: &std::path::Path) -> Result<NodeObservation> {
    use std::os::{
        fd::AsRawFd,
        unix::fs::{FileTypeExt, MetadataExt},
    };
    ensure!(
        path.parent() == Some(std::path::Path::new("/dev/input"))
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix("event"))
                .is_some_and(
                    |index| !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
                ),
        "Dolphin capture requires an exact /dev/input/eventN node"
    );
    let file = std::fs::File::open(path).context("Opening Dolphin evdev input read-only")?;
    let identity = file.metadata()?;
    ensure!(
        identity.file_type().is_char_device(),
        "Dolphin input path is not a character device"
    );
    let mut properties = [0_u8; 8];
    let mut keys = [0_u8; 96];
    let mut axes = [0_u8; 8];
    let mut pressed = [0_u8; 96];
    // Linux input.h ABI: EVIOCGPROP, EVIOCGBIT(KEY), EVIOCGBIT(ABS), EVIOCGKEY.
    for (request, buffer) in [
        (0x80084509_u64, properties.as_mut_ptr()),
        (0x80604521, keys.as_mut_ptr()),
        (0x80084523, axes.as_mut_ptr()),
        (0x80604518, pressed.as_mut_ptr()),
    ] {
        ensure!(
            unsafe { libc::ioctl(file.as_raw_fd(), request as libc::c_ulong, buffer) } >= 0,
            "Reading Dolphin evdev capabilities/state: {}",
            std::io::Error::last_os_error()
        );
    }
    let bit = |bytes: &[u8], index: usize| bytes[index / 8] & (1 << (index % 8)) != 0;
    let mut metadata = BTreeMap::new();
    let mut values = BTreeMap::new();
    for code in 0..64_u8 {
        if !bit(&axes, usize::from(code)) {
            continue;
        }
        // input_absinfo: value, minimum, maximum, fuzz, flat, resolution.
        let mut info = [0_i32; 6];
        ensure!(
            unsafe {
                libc::ioctl(
                    file.as_raw_fd(),
                    (0x80184540_u64 + u64::from(code)) as libc::c_ulong,
                    info.as_mut_ptr(),
                )
            } >= 0,
            "Reading Dolphin axis state: {}",
            std::io::Error::last_os_error()
        );
        metadata.insert(
            code,
            AxisInfo {
                minimum: info[1],
                maximum: info[2],
                fuzz: info[3],
                flat: info[4],
                resolution: info[5],
            },
        );
        values.insert(code, info[0]);
    }
    let current = std::fs::metadata(path)?;
    ensure!(
        current.dev() == identity.dev()
            && current.ino() == identity.ino()
            && current.rdev() == identity.rdev(),
        "Dolphin evdev node changed while capturing"
    );
    Ok(NodeObservation {
        path: path.to_owned(),
        motion: bit(&properties, 6),
        pointing: bit(&properties, 2),
        keys: (0..768_u16)
            .filter(|code| bit(&keys, usize::from(*code)))
            .collect(),
        pressed_keys: (0..768_u16)
            .filter(|code| bit(&pressed, usize::from(*code)))
            .collect(),
        axes: metadata,
        axis_values: values,
    })
}

pub(crate) fn resolve(
    calibration: &Calibration,
    port: u8,
    observed: &DeviceObservation,
) -> Result<Pad> {
    ensure!(
        calibration.os == "linux" && observed.qualifier.starts_with("evdev/"),
        "Dolphin evdev translation needs Linux calibration and a native evdev device"
    );
    ensure!(
        !observed.nodes.is_empty() && observed.nodes.len() <= 16,
        "Dolphin native node inventory exceeds bounds"
    );
    let mut paths = BTreeSet::new();
    for node in &observed.nodes {
        ensure!(
            node.path.is_absolute() && paths.insert(&node.path),
            "Dolphin node paths must be unique and absolute"
        );
    }
    // Motion/pointing nodes have distinct names (Accel/Gyro/Cursor and named
    // buttons without Button N aliases). Two ordinary nodes would collide.
    let regular: Vec<_> = observed
        .nodes
        .iter()
        .filter(|node| !node.motion && !node.pointing)
        .collect();
    ensure!(
        regular.len() == 1,
        "Dolphin needs exactly one unambiguous ordinary gamepad node"
    );
    let node = regular[0];
    ensure!(
        node.keys.len() <= 256
            && node.keys.iter().all(|code| *code < 768)
            && node.pressed_keys.is_subset(&node.keys),
        "Invalid Dolphin evdev key inventory"
    );
    let axes: Vec<_> = node
        .axes
        .keys()
        .copied()
        .filter(|code| *code < 0x28)
        .collect(); // ABS_MISC excluded
    let profile = catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == "dolphin:standalone-gamecube")
        .context("Missing native Dolphin profile")?;
    let mut physical = BTreeMap::new();
    let mut stick_axes = BTreeMap::new();
    for row in calibration.plan_profile(profile)?.rows {
        let id = row
            .physical_id
            .context("Dolphin target has no physical control")?;
        let input = row.input.context("Dolphin control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("Dolphin physical input identity is absent")?;
        let code = (native.code & 0xffff) as u16;
        let resolved = match native.code >> 16 {
            1 => {
                let index = node
                    .keys
                    .iter()
                    .position(|key| *key == code)
                    .context("Dolphin button is absent from the ordinary evdev node")?;
                ensure!(
                    !node.pressed_keys.contains(&code),
                    "Release Dolphin buttons before capture"
                );
                ensure!(
                    !analog_target(&row.output),
                    "Dolphin analog target needs a measured physical axis"
                );
                ResolvedControl {
                    name: format!("Button {index}"),
                    analog: false,
                    range_percent: 100.0,
                }
            }
            3 => {
                let code = u8::try_from(code).context("Invalid evdev axis code")?;
                let index = axes
                    .iter()
                    .position(|axis| *axis == code)
                    .context("Dolphin ordinary backend does not expose this evdev axis")?;
                let info = &node.axes[&code];
                let measured = input
                    .axis
                    .as_ref()
                    .context("Dolphin measured axis endpoints are absent")?;
                ensure!(
                    measured.minimum == info.minimum
                        && measured.maximum == info.maximum
                        && measured.flat == info.flat
                        && measured.fuzz == info.fuzz
                        && measured.resolution == info.resolution,
                    "Dolphin evdev axis metadata differs from saved calibration"
                );
                ensure!(
                    node.axis_values.get(&code) == Some(&measured.released),
                    "Dolphin captured axis rest differs from calibration"
                );
                let (name, scale) = axis_binding(index, info, measured.released, measured.pressed)?;
                let hat = (0x10..=0x17).contains(&code);
                ensure!(
                    !analog_target(&row.output) || !hat,
                    "Dolphin analog target cannot use a directional hat axis"
                );
                if row.output.starts_with("Main Stick/") || row.output.starts_with("C-Stick/") {
                    ensure!(
                        !name.starts_with("Full "),
                        "Dolphin stick directions require centered physical axes"
                    );
                    stick_axes.insert(
                        row.output.clone(),
                        (code, measured.pressed > measured.released),
                    );
                }
                ResolvedControl {
                    name,
                    analog: !hat,
                    range_percent: scale,
                }
            }
            _ => anyhow::bail!("Dolphin ordinary gamepad mapping requires EV_KEY or EV_ABS"),
        };
        if let Some(previous) = physical.get(&id) {
            ensure!(
                previous == &resolved,
                "Dolphin physical control resolved inconsistently across targets"
            );
        }
        physical.insert(id, resolved);
    }
    let mut used = BTreeSet::new();
    for (a, b) in [
        ("Main Stick/Up", "Main Stick/Down"),
        ("Main Stick/Left", "Main Stick/Right"),
        ("C-Stick/Up", "C-Stick/Down"),
        ("C-Stick/Left", "C-Stick/Right"),
    ] {
        let first = stick_axes
            .get(a)
            .context("Missing Dolphin analog stick direction")?;
        let second = stick_axes
            .get(b)
            .context("Missing Dolphin opposite stick direction")?;
        ensure!(
            first.0 == second.0 && first.1 != second.1 && used.insert(first.0),
            "Dolphin sticks need four independent axes with opposite paired directions"
        );
    }
    calibrated_pad(calibration, port, observed.qualifier.clone(), &physical)
}

fn axis_binding(index: usize, info: &AxisInfo, rest: i32, pressed: i32) -> Result<(String, f64)> {
    let min = i64::from(info.minimum);
    let max = i64::from(info.maximum);
    let sum = min + max;
    ensure!(
        min < max && i32::try_from(sum).is_ok(),
        "Dolphin evdev axis has invalid native bounds"
    );
    let base = sum / 2; // Match native signed integer truncation.
    ensure!(
        base > min
            && base < max
            && i64::from(rest) >= min
            && i64::from(rest) <= max
            && i64::from(pressed) >= min
            && i64::from(pressed) <= max
            && pressed != rest,
        "Dolphin evdev axis gesture is outside supported bounds"
    );
    let positive = pressed > rest;
    let sign = if positive { '+' } else { '-' };
    let high = (i64::from(pressed) - base) as f64 / (max - base) as f64;
    let low = (i64::from(pressed) - base) as f64 / (min - base) as f64;
    let (prefix, extent) = if i64::from(rest) == base {
        ("", if positive { high } else { low })
    } else {
        ensure!(
            (positive && i64::from(rest) == min) || (!positive && i64::from(rest) == max),
            "Dolphin off-center axis rest needs an explicit offset expression"
        );
        let travel = (1.0 + high.max(0.0) - low.max(0.0)) / 2.0;
        ("Full ", if positive { travel } else { 1.0 - travel })
    };
    ensure!(
        extent.is_finite() && extent > 0.0,
        "Dolphin axis gesture has no usable native travel"
    );
    Ok((format!("{prefix}Axis {index}{sign}"), 100.0 / extent))
}
