//! Native launch-time evdev verification and the private XDG settings file.
//! The user's own Mesen2 configuration is never opened.
use super::{Binding, settings};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{evdev_catalog, file_hash};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

const BTN_GAMEPAD: u16 = 0x130;
const ABS_X: u16 = 0x00;

fn event_nodes() -> Result<Vec<PathBuf>> {
    let mut nodes: Vec<PathBuf> = std::fs::read_dir("/dev/input")?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.strip_prefix("event").is_some_and(|suffix| {
                        !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                    })
                })
        })
        .collect();
    nodes.sort();
    Ok(nodes)
}

fn catalog(probe: &Path, cancel: &AtomicBool) -> Result<evdev_catalog::EvdevCatalog> {
    let nodes = event_nodes()?;
    let mut command = Command::new(probe);
    for node in &nodes {
        command.arg("--evdev-catalog").arg(node);
    }
    let (output, _) = capture(&mut command, cancel)?;
    serde_json::from_slice(&output).context("Invalid Mesen2 evdev capture")
}

/// Mesen2's acceptance test from LinuxGameController::GetController:
/// `EV_KEY+BTN_GAMEPAD` or `EV_ABS+ABS_X`.
fn qualifies(device: &evdev_catalog::EvdevDevice) -> bool {
    device.buttons.contains(&BTN_GAMEPAD) || device.axes.iter().any(|axis| axis.code == ABS_X)
}

/// Translate the calibrated controls into KeyMapping codes for the sole
/// qualifying device's pad slot.
pub(super) fn calibrated_bindings(
    calibration: &Calibration,
    device: &evdev_catalog::EvdevDevice,
) -> Result<Vec<(String, Binding)>> {
    ensure!(
        calibration.os == "linux",
        "Mesen2 native calibration requires Linux"
    );
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native Mesen2 profile")?;
    let mut bindings = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let field = super::CONTROLS
            .iter()
            .find(|(control, _)| *control == row.target_id)
            .map(|(_, field)| (*field).to_owned())
            .with_context(|| {
                format!(
                    "Mesen2 target {} is outside the NES contract",
                    row.target_id
                )
            })?;
        let input = row
            .input
            .context("Mesen2 gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("Mesen2 requires measured native controls")?;
        let code = native.code & 0xffff;
        let binding = match native.code >> 16 {
            1 => Binding::from_key(u32::from(code))
                .with_context(|| format!("Mesen2 has no buttonIndex for kernel key {code:#x}"))?,
            3 => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("Mesen2 axis measurements are absent")?;
                let axis = device
                    .axes
                    .iter()
                    .chain(&device.hats)
                    .find(|axis| u32::from(axis.code) == code)
                    .with_context(|| format!("Mesen2 device lacks axis {code:#x}"))?;
                let span = f64::from(axis.info.maximum - axis.info.minimum).max(1.0);
                let released = f64::from(endpoints.released - axis.info.minimum) / span;
                let pressed = f64::from(endpoints.pressed - axis.info.minimum) / span;
                ensure!(
                    (0.45..0.55).contains(&released),
                    "Mesen2 axis rest must sit near the middle of the kernel range"
                );
                let positive = pressed > released;
                ensure!(
                    (pressed - released).abs() > super::RANGE_FRACTION,
                    "Mesen2's default 40% axis dead zone cannot represent the measured travel"
                );
                Binding::from_axis(u32::from(code), positive).with_context(|| {
                    format!("Mesen2 has no buttonIndex for kernel axis {code:#x}")
                })?
            }
            other => anyhow::bail!("Mesen2 cannot consume input class {other}"),
        };
        bindings.push((field, binding));
    }
    Ok(bindings)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) data_root: PathBuf,
    topology: InputTopology,
    setup: settings::SavedSetup,
    hashes: std::collections::BTreeMap<PathBuf, String>,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.review(calibrations)?;
        let mut matches = inventory
            .iter()
            .filter(|device| device.stable_id == setup.controller_id);
        let device = matches
            .next()
            .context("Mesen2 selected controller is disconnected")?;
        ensure!(
            matches.next().is_none() && !device.is_virtual,
            "Mesen2 requires an unambiguous physical controller"
        );
        let event = device
            .event_paths
            .first()
            .cloned()
            .context("Mesen2 needs the controller's event node")?;
        let topology = InputTopology::capture(std::slice::from_ref(&device.device_path))?;
        let catalog = catalog(&setup.probe_program, cancel)?;
        // Mesen2 registers pads in directory-iteration order; with exactly
        // one qualifying device the slot is deterministically zero.
        let qualifying: Vec<_> = catalog
            .devices
            .iter()
            .filter(|device| qualifies(device))
            .collect();
        let selected = catalog.device_at_event(&event)?;
        ensure!(
            qualifying.len() == 1 && qualifying[0].event == selected.event,
            "Mesen2 pad slots follow directory order; exactly one qualifying gamepad (the selected controller) is required"
        );
        let bindings = calibrated_bindings(
            calibrations
                .get(&setup.controller_id)
                .context("Mesen2 calibration disappeared")?,
            selected,
        )?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-mesen2-")
            .tempdir()?;
        let data_root = directory.path().join("data");
        std::fs::create_dir_all(data_root.join("Mesen2"))?;
        let keyfile = data_root.join("Mesen2").join("settings.json");
        std::fs::write(&keyfile, super::settings_json(&bindings)?)?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [&setup.content, &setup.probe_program] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(keyfile.clone(), file_hash(&keyfile)?);
        let session = Self {
            directory,
            data_root,
            topology,
            setup: setup.clone(),
            hashes,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        for (path, expected) in &self.hashes {
            ensure!(file_hash(path)? == *expected, "Mesen2 launch input changed");
        }
        let catalog = catalog(&self.setup.probe_program, cancel)?;
        ensure!(
            catalog
                .devices
                .iter()
                .filter(|device| qualifies(device))
                .count()
                == 1,
            "Mesen2 requires the sole qualifying gamepad at launch"
        );
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
