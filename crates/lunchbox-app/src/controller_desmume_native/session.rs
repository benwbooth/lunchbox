//! Native SDL2 launch-time inventory, calibration ownership and the private
//! XDG config. The user's own DeSmuME configuration is never opened.
use super::{Binding, settings};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{file_hash, sdl2::Snapshot};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(
    setup: &settings::SavedSetup,
    path: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command
        .arg("--sdl2-inventory")
        .arg("--sdl-library")
        .arg(&setup.sdl_library);
    if let Some(path) = path {
        command.arg("--sdl2-controls-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid DeSmuME SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "DeSmuME helper inspected a different SDL runtime"
    );
    Ok(snapshot)
}

fn routing(mut snapshot: Snapshot) -> Snapshot {
    for device in &mut snapshot.devices {
        device.controls = None;
        device.linux_classic = None;
        device.linux_evdev = None;
        device.sampled_state = None;
    }
    snapshot
}

/// Translate the calibrated DS controls into JOYKEYS bindings.
pub(super) fn calibrated_bindings(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
) -> Result<Vec<(String, Binding)>> {
    use lunchbox_controller_probe::{
        duckstation::DigitalInput, linux_classic::AxisEndpoints, sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "DeSmuME native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let counts = device
        .controls
        .as_ref()
        .context("DeSmuME SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("DeSmuME released state is absent")?;
    state.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native DeSmuME profile")?;
    let mut bindings = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let output = super::KEYS
            .iter()
            .find(|(control, _)| *control == row.target_id)
            .map(|(_, output)| (*output).to_owned())
            .with_context(|| {
                format!(
                    "DeSmuME target {} is outside the button contract",
                    row.target_id
                )
            })?;
        let input = row
            .input
            .context("DeSmuME gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("DeSmuME requires measured native controls")?;
        let binding = match physical.control(native.code)? {
            lunchbox_controller_probe::linux_classic::Control::Button(button) => {
                ensure!(
                    state.buttons.get(&button) == Some(&false),
                    "Release DeSmuME controller buttons before capture"
                );
                Binding::Button(button)
            }
            lunchbox_controller_probe::linux_classic::Control::Axis(axis) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("DeSmuME axis measurements are absent")?;
                let code = (native.code & 0xffff) as u8;
                let released = physical.axis_value(code, endpoints.released)?;
                let pressed = physical.axis_value(code, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&axis) == Some(&released),
                    "DeSmuME axis rest differs from calibration"
                );
                ensure!(
                    (-super::DIGITAL_THRESHOLD..super::DIGITAL_THRESHOLD)
                        .contains(&i32::from(released)),
                    "DeSmuME axis rest would hold a key pressed; recalibrate the rest position"
                );
                if i32::from(pressed) >= super::DIGITAL_THRESHOLD {
                    Binding::Axis {
                        index: axis,
                        positive: true,
                    }
                } else if i32::from(pressed) <= -super::DIGITAL_THRESHOLD {
                    Binding::Axis {
                        index: axis,
                        positive: false,
                    }
                } else {
                    anyhow::bail!(
                        "DeSmuME's 16384 axis threshold cannot represent the measured travel"
                    );
                }
            }
            lunchbox_controller_probe::linux_classic::Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("DeSmuME hat measurements are absent")?;
                match physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )? {
                    DigitalInput::Hat { index, direction } => {
                        ensure!(
                            state.hats.get(&index) == Some(&0),
                            "Release DeSmuME directional hat before capture"
                        );
                        Binding::Hat { index, direction }
                    }
                    _ => anyhow::bail!("DeSmuME hat did not resolve to a native SDL hat"),
                }
            }
        };
        bindings.push((output, binding));
    }
    Ok(bindings)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) config_root: std::path::PathBuf,
    runtime_path: String,
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
        let mut matches = inventory
            .iter()
            .filter(|device| device.stable_id == setup.controller_id);
        let device = matches
            .next()
            .context("DeSmuME selected controller is disconnected")?;
        ensure!(
            matches.next().is_none() && !device.is_virtual,
            "DeSmuME requires an unambiguous physical controller"
        );
        let selected = device.device_path.clone();
        let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
        let initial = routing(observe(setup, None, cancel)?);
        let runtime_path = topology.resolve_runtime_path(
            &selected,
            initial
                .devices
                .iter()
                .filter_map(|device| device.path.as_deref()),
        )?;
        let captured = observe(setup, Some(&runtime_path), cancel)?;
        initial.ensure_same_routing(&routing(captured.clone()))?;
        topology.verify()?;
        let bindings = calibrated_bindings(
            calibrations
                .get(&setup.controller_id)
                .context("DeSmuME calibration disappeared")?,
            &captured,
            &runtime_path,
        )?;
        let device_index = captured.device_at_path(&runtime_path)?.device_index;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-desmume-")
            .tempdir()?;
        let config_root = directory.path().join("config");
        std::fs::create_dir_all(config_root.join("desmume"))?;
        std::fs::write(
            config_root.join("desmume").join("config"),
            super::config(device_index, &bindings)?,
        )?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [&setup.probe_program, &setup.sdl_library, &setup.content] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        let keyfile = config_root.join("desmume").join("config");
        hashes.insert(keyfile.clone(), file_hash(&keyfile)?);
        let session = Self {
            directory,
            config_root,
            runtime_path,
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
            ensure!(
                file_hash(path)? == *expected,
                "DeSmuME launch input changed"
            );
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        fresh
            .device_at_path(&self.runtime_path)
            .context("DeSmuME controller disappeared")?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
