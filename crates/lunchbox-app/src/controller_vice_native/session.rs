//! Native SDL2 launch-time inventory, calibration ownership and the private
//! -config/-joymap pair. The user's own vicerc is never opened.
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
    let snapshot: Snapshot = serde_json::from_slice(&output).context("Invalid VICE SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "VICE helper inspected a different SDL runtime"
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

/// Translate one player's calibrated controls into .vjm pin assignments.
pub(super) fn calibrated_pins(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
) -> Result<Vec<(u32, Binding)>> {
    use lunchbox_controller_probe::{
        duckstation::DigitalInput, linux_classic::AxisEndpoints, sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "VICE native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let counts = device
        .controls
        .as_ref()
        .context("VICE SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("VICE released state is absent")?;
    state.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native VICE profile")?;
    let mut pins = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let pin = super::pin(&row.target_id)?;
        let input = row
            .input
            .context("VICE gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("VICE requires measured native controls")?;
        let binding = match physical.control(native.code)? {
            lunchbox_controller_probe::linux_classic::Control::Button(button) => {
                ensure!(
                    state.buttons.get(&button) == Some(&false),
                    "Release VICE controller buttons before capture"
                );
                Binding::Button(button)
            }
            lunchbox_controller_probe::linux_classic::Control::Axis(axis) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("VICE axis measurements are absent")?;
                let code = (native.code & 0xffff) as u8;
                let released = physical.axis_value(code, endpoints.released)?;
                let pressed = physical.axis_value(code, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&axis) == Some(&released),
                    "VICE axis rest differs from calibration"
                );
                ensure!(
                    (-super::DIGITAL_THRESHOLD..super::DIGITAL_THRESHOLD)
                        .contains(&i32::from(released)),
                    "VICE axis rest would hold a digital direction pressed; recalibrate the rest position"
                );
                if i32::from(pressed) > super::DIGITAL_THRESHOLD + super::RELEASE_MARGIN {
                    Binding::AxisPositive(axis)
                } else if i32::from(pressed) < -super::DIGITAL_THRESHOLD - super::RELEASE_MARGIN {
                    Binding::AxisNegative(axis)
                } else {
                    anyhow::bail!(
                        "VICE's default threshold (10000) plus fuzz (1000) cannot represent the measured axis travel"
                    );
                }
            }
            lunchbox_controller_probe::linux_classic::Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("VICE hat measurements are absent")?;
                match physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )? {
                    DigitalInput::Hat {
                        index: hat,
                        direction,
                    } => {
                        ensure!(
                            state.hats.get(&hat) == Some(&0),
                            "Release VICE directional hat before capture"
                        );
                        // SDL hat bitmask: 1=up, 2=right, 4=down, 8=left.
                        match direction {
                            1 => Binding::HatUp(hat),
                            2 => Binding::HatRight(hat),
                            4 => Binding::HatDown(hat),
                            8 => Binding::HatLeft(hat),
                            _ => anyhow::bail!("VICE hat direction is a diagonal"),
                        }
                    }
                    _ => anyhow::bail!("VICE hat did not resolve to a native SDL hat"),
                }
            }
        };
        pins.push((pin, binding));
    }
    ensure!(
        super::required_controls().all(|control| {
            pins.iter()
                .any(|(pin, _)| *pin == super::pin(control).expect("known control"))
        }),
        "VICE needs every required joystick control"
    );
    Ok(pins)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) config_path: std::path::PathBuf,
    pub(crate) joymap_path: std::path::PathBuf,
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
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("VICE selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "VICE requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let mut devices = Vec::new();
        let mut assignments = Vec::new();
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
                "VICE players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            let pins = calibrated_pins(
                calibrations
                    .get(&player.controller_id)
                    .context("VICE calibration disappeared")?,
                &captured,
                &path,
            )?;
            let device = captured.device_at_path(&path)?.device_index;
            assignments.push((device, pins));
            devices.push(device);
            runtime_paths.push(path);
        }
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-vice-")
            .tempdir()?;
        let joymap_path = directory.path().join("lunchbox.vjm");
        let config_path = directory.path().join("vice.cfg");
        let references: Vec<_> = assignments
            .iter()
            .map(|(device, pins)| (*device, pins.as_slice()))
            .collect();
        let staged = super::joymap(&references);
        std::fs::write(&joymap_path, &staged)?;
        ensure!(
            std::fs::read_to_string(&joymap_path)? == staged,
            "VICE staged joymap verification failed"
        );
        std::fs::write(&config_path, super::config(&devices))?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [&setup.probe_program, &setup.sdl_library, &setup.content] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(joymap_path.clone(), file_hash(&joymap_path)?);
        hashes.insert(config_path.clone(), file_hash(&config_path)?);
        let session = Self {
            directory,
            config_path,
            joymap_path,
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
            ensure!(file_hash(path)? == *expected, "VICE launch input changed");
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    /// Launch arguments select the private config and joymap ahead of the game.
    pub(crate) fn overlay_arguments(
        &self,
        arguments: &[std::ffi::OsString],
    ) -> Result<Vec<std::ffi::OsString>> {
        ensure!(
            self.config_path.is_absolute() && self.joymap_path.is_absolute(),
            "VICE private configuration paths must be absolute"
        );
        let mut result = Vec::with_capacity(arguments.len() + 4);
        result.push("-config".into());
        result.push(self.config_path.as_os_str().to_owned());
        result.push("-joymap".into());
        result.push(self.joymap_path.as_os_str().to_owned());
        result.extend_from_slice(arguments);
        Ok(result)
    }
}
