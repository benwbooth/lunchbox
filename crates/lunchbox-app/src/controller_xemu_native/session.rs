//! Native SDL3 launch-time inventory, calibration ownership and the private
//! config file. The user's own xemu.toml is never opened.
use super::{BTreeMapRef, Standard, settings};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{Snapshot, file_hash};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(
    setup: &settings::SavedSetup,
    path: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command
        .arg("--sdl-library")
        .arg(&setup.sdl_library)
        .arg("--hint")
        .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
    if let Some(path) = path {
        command.arg("--bindings-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot = serde_json::from_slice(&output).context("Invalid xemu SDL capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
            && snapshot
                .effective_hints
                .get("SDL_JOYSTICK_LINUX_CLASSIC")
                .and_then(Option::as_deref)
                == Some("1"),
        "xemu helper inspected a different SDL runtime or backend"
    );
    Ok(snapshot)
}

fn routing(mut snapshot: Snapshot) -> Snapshot {
    for device in &mut snapshot.devices {
        device.resolved = None;
        device.linux_classic = None;
    }
    snapshot
}

/// Match a raw joystick control against a resolved gamepad binding and
/// return the standard index the binding feeds.
fn standard_for(
    resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
    raw: &Raw,
) -> Option<Standard> {
    resolved.bindings.iter().find_map(|binding| {
        let matches = match (&binding.input, raw) {
            (lunchbox_controller_probe::bindings::Input::Button { index }, Raw::Button(button)) => {
                index == button
            }
            (lunchbox_controller_probe::bindings::Input::Axis { index, .. }, Raw::Axis(axis)) => {
                index == axis
            }
            (
                lunchbox_controller_probe::bindings::Input::Hat { index, mask },
                Raw::Hat {
                    hat,
                    mask: hat_mask,
                },
            ) => index == hat && mask & hat_mask != 0,
            _ => false,
        };
        matches.then(|| match binding.output {
            lunchbox_controller_probe::bindings::Output::Button { index } => {
                Standard::Button(index)
            }
            lunchbox_controller_probe::bindings::Output::Axis { index, .. } => {
                Standard::Axis(index)
            }
        })
    })
}

#[derive(Clone, Copy, Debug)]
enum Raw {
    Button(u32),
    Axis(u32),
    Hat { hat: u32, mask: u8 },
}

/// Translate one player's calibrated controls into the per-controller
/// standard-index mapping fields.
pub(super) fn calibrated_mapping(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
) -> Result<BTreeMapRef> {
    use lunchbox_controller_probe::{linux_classic::Control, sdl2_physical::PhysicalMap};
    ensure!(
        calibration.os == "linux",
        "xemu native calibration requires Linux"
    );
    let device = snapshot
        .devices
        .iter()
        .find(|device| device.path.as_deref() == Some(runtime_path))
        .context("xemu device disappeared from the snapshot")?;
    let resolved = device
        .resolved
        .as_ref()
        .context("xemu SDL gamepad bindings are absent; the device is not a gamepad")?;
    resolved.validate()?;
    let classic = device.linux_classic.as_ref().context(
        "xemu classic joystick numbering is absent; the pinned classic backend did not capture it",
    )?;
    let physical = PhysicalMap::Classic(classic);
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native xemu profile")?;
    let plan = calibration.plan_profile(profile)?;
    let mut fields = BTreeMapRef::default();
    let mut raws: std::collections::BTreeMap<String, Raw> = std::collections::BTreeMap::new();
    for row in &plan.rows {
        let input = row
            .input
            .as_ref()
            .context("xemu gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("xemu requires measured native controls")?;
        let raw = match physical.control(native.code)? {
            Control::Button(button) => Raw::Button(button),
            Control::Axis(axis) => Raw::Axis(axis),
            Control::HatAxis { index, horizontal } => {
                // Digital hat halves enter the binding mask as their bit.
                let direction = input
                    .axis
                    .as_ref()
                    .map(|axis| axis.pressed.saturating_sub(axis.released).signum())
                    .unwrap_or(1);
                let mask: u8 = match (horizontal, direction) {
                    (true, -1) => 8,  // left
                    (true, _) => 2,   // right
                    (false, -1) => 1, // up
                    (false, _) => 4,  // down
                };
                Raw::Hat { hat: index, mask }
            }
        };
        raws.insert(row.target_id.clone(), raw);
    }
    // Digital fields.
    for (control, field) in super::BUTTON_FIELDS
        .iter()
        .chain(super::OPTIONAL_BUTTON_FIELDS.iter())
    {
        let (control, field): (&str, &str) = (control, field);
        if let Some(raw) = raws.get(control) {
            match standard_for(resolved, raw) {
                Some(Standard::Button(index)) => {
                    fields.buttons.insert(field.to_owned(), index);
                }
                Some(Standard::Axis(_)) | None => anyhow::bail!(
                    "xemu control {control} has no standard gamepad button on this device"
                ),
            }
        }
    }
    // Trigger axes.
    for (control, field) in super::TRIGGER_FIELDS {
        let (control, field): (&str, &str) = (control, field);
        if let Some(raw) = raws.get(control) {
            match standard_for(resolved, raw) {
                Some(Standard::Axis(index)) => {
                    fields.axes.insert(field.to_owned(), index);
                }
                Some(Standard::Button(_)) | None => anyhow::bail!(
                    "xemu trigger {control} has no standard gamepad axis on this device"
                ),
            }
        }
    }
    // Stick axes from directional pairs.
    for (negative, positive, field, invert_field) in super::AXIS_PAIRS {
        let (negative, positive, field, invert_field): (&str, &str, &str, &str) =
            (negative, positive, field, invert_field);
        let (Some(neg), Some(pos)) = (raws.get(negative), raws.get(positive)) else {
            continue;
        };
        let (Raw::Axis(neg_axis), Raw::Axis(pos_axis)) = (*neg, *pos) else {
            anyhow::bail!("xemu stick directions for {field} must live on axes");
        };
        ensure!(
            neg_axis == pos_axis,
            "xemu stick directions for {field} span two different axes"
        );
        let Some(Standard::Axis(index)) = standard_for(resolved, neg) else {
            anyhow::bail!("xemu stick {field} has no standard gamepad axis");
        };
        fields.axes.insert(field.to_owned(), index);
        // SDL's standard axis positive direction must agree with the
        // calibrated positive control; otherwise invert.
        let binding = resolved
            .bindings
            .iter()
            .find(|binding| {
                matches!(binding.output,
                    lunchbox_controller_probe::bindings::Output::Axis { index: i, .. } if i == index)
            })
            .context("xemu standard axis binding disappeared")?;
        let lunchbox_controller_probe::bindings::Input::Axis { min, .. } = binding.input else {
            anyhow::bail!("xemu standard axis is not bound to a raw axis");
        };
        let standard_positive = min < 0;
        let calibrated_positive_on_positive_half = matches!(pos, Raw::Axis(a) if *a == pos_axis)
            && calibrated_half_positive(&plan, positive)?;
        if standard_positive != calibrated_positive_on_positive_half {
            fields.inverted.insert(invert_field.to_owned());
        }
    }
    Ok(fields)
}

fn calibrated_half_positive(
    plan: &crate::controller_catalog::MappingPlan,
    control: &str,
) -> Result<bool> {
    let row = plan
        .rows
        .iter()
        .find(|row| row.target_id == control)
        .context("xemu stick control disappeared from the plan")?;
    let native = row
        .input
        .as_ref()
        .and_then(|input| input.native.as_ref())
        .context("xemu stick control lacks a native binding")?;
    Ok(native.direction > 0)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) config_path: std::path::PathBuf,
    runtime_paths: Vec<String>,
    topology: InputTopology,
    initial: Snapshot,
    setup: settings::SavedSetup,
    hashes: std::collections::BTreeMap<std::path::PathBuf, String>,
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
        ensure!(
            setup.mcpx_bootrom.is_file() && setup.flashrom.is_file(),
            "xemu needs the declared MCPX boot ROM and flash image to exist"
        );
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("xemu selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "xemu requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let mut guids: Vec<(usize, String, BTreeMapRef)> = Vec::new();
        let mut runtime_paths = Vec::new();
        for (player, selected) in setup.players.iter().zip(&selected) {
            let path = topology.resolve_runtime_path(
                selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            ensure!(
                !runtime_paths.contains(&path),
                "xemu players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            topology.verify()?;
            let guid = captured
                .devices
                .iter()
                .find(|device| device.path.as_deref() == Some(path.as_str()))
                .and_then(|device| Some(device.guid.clone()))
                .context("xemu device has no SDL GUID")?;
            let mapping = calibrated_mapping(
                calibrations
                    .get(&player.controller_id)
                    .context("xemu calibration disappeared")?,
                &captured,
                &path,
            )?;
            ensure!(
                guids.iter().all(|(_, existing, _)| *existing != guid),
                "xemu cannot distinguish controllers sharing one SDL GUID"
            );
            guids.push((usize::from(player.player), guid, mapping));
            runtime_paths.push(path);
        }
        let references: Vec<_> = guids
            .iter()
            .map(|(port, guid, mapping)| (*port, guid.as_str(), mapping))
            .collect();
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-xemu-")
            .tempdir()?;
        let config_path = directory.path().join("xemu.toml");
        std::fs::write(
            &config_path,
            super::config_toml(
                &references,
                &setup.content,
                &setup.mcpx_bootrom,
                &setup.flashrom,
            )?,
        )?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [
            &setup.probe_program,
            &setup.sdl_library,
            &setup.content,
            &setup.mcpx_bootrom,
            &setup.flashrom,
        ] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(config_path.clone(), file_hash(&config_path)?);
        let session = Self {
            directory,
            config_path,
            runtime_paths,
            topology,
            initial,
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
            ensure!(file_hash(path)? == *expected, "xemu launch input changed");
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        for path in &self.runtime_paths {
            ensure!(
                fresh
                    .devices
                    .iter()
                    .any(|device| device.path.as_deref() == Some(path.as_str())),
                "xemu controller disappeared: {path}"
            );
        }
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
