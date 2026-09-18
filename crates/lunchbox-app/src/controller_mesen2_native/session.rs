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

/// The selected node's Mesen2 pad slot: its position among the qualifying
/// gamepads in `/dev/input` directory order, which is the order
/// `LinuxKeyManager.cpp CheckForGamepads` registers pads in. Other gamepads
/// simply occupy the slots before it, so a second controller on the host is
/// not an error; only a device Mesen2 would never open is.
fn qualifying_slot<'a>(
    catalog: &'a evdev_catalog::EvdevCatalog,
    event: &Path,
) -> Result<(&'a evdev_catalog::EvdevDevice, u32)> {
    let selected = catalog.device_at_event(event)?;
    ensure!(
        qualifies(selected),
        "Mesen2 only opens gamepads (EV_KEY+BTN_GAMEPAD or EV_ABS+ABS_X); the selected controller is not one"
    );
    let slot = catalog
        .devices
        .iter()
        .filter(|device| qualifies(device))
        .position(|device| device.event == selected.event)
        .context("Mesen2 selected gamepad is missing from its own catalog")?;
    let slot = u32::try_from(slot).context("Mesen2 pad slot overflow")?;
    ensure!(slot < 20, "Mesen2 addresses twenty pad slots");
    Ok((selected, slot))
}

/// Translate the calibrated controls into KeyMapping codes for the selected
/// device's Mesen2 pad slot, using the setup's system contract.
pub(super) fn calibrated_bindings(
    setup: &settings::SavedSetup,
    calibration: &Calibration,
    device: &evdev_catalog::EvdevDevice,
    pad: u32,
) -> Result<Vec<(String, Binding)>> {
    ensure!(
        calibration.os == "linux",
        "Mesen2 native calibration requires Linux"
    );
    let pce = setup.system == super::SYSTEM_PCE;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| {
            profile.id
                == if pce {
                    settings::PCE_PROFILE_ID
                } else {
                    settings::PROFILE_ID
                }
        })
        .context("Missing native Mesen2 profile")?;
    let controls = if pce {
        super::PCE_CONTROLS
    } else {
        super::CONTROLS
    };
    let mut bindings = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let field = controls
            .iter()
            .find(|(control, _)| *control == row.target_id)
            .map(|(_, field)| (*field).to_owned())
            .with_context(|| {
                format!(
                    "Mesen2 target {} is outside the {} contract",
                    row.target_id,
                    if pce { "PCE" } else { "NES" }
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
            1 => Binding::from_key(pad, u32::from(code))
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
                Binding::from_axis(pad, u32::from(code), positive).with_context(|| {
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
    /// Kernel event node and pinned Mesen2 pad slot for the selected pad.
    event: PathBuf,
    slot: u32,
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
        let (selected, slot) = qualifying_slot(&catalog, &event)?;
        let bindings = calibrated_bindings(
            setup,
            calibrations
                .get(&setup.controller_id)
                .context("Mesen2 calibration disappeared")?,
            selected,
            slot,
        )?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-mesen2-")
            .tempdir()?;
        let data_root = directory.path().join("data");
        std::fs::create_dir_all(data_root.join("Mesen2"))?;
        let keyfile = data_root.join("Mesen2").join("settings.json");
        let text = if setup.system == super::SYSTEM_PCE {
            super::settings_json_pce(&bindings)?
        } else {
            super::settings_json(&bindings)?
        };
        std::fs::write(&keyfile, text)?;
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
            event,
            slot,
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
        // The pad slot was pinned at preparation: another gamepad plugging in
        // ahead of ours would renumber it, so the slot must still match.
        let catalog = catalog(&self.setup.probe_program, cancel)?;
        let (_, slot) = qualifying_slot(&catalog, &self.event)?;
        ensure!(
            slot == self.slot,
            "Mesen2 pad slot moved before launch; review the controller setup again"
        );
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
