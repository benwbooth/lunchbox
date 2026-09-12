//! Native SDL2 launch-time inventory, calibration ownership and the private
//! HOME config. The user's own blastem.cfg is never opened.
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
        serde_json::from_slice(&output).context("Invalid BlastEm SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "BlastEm helper inspected a different SDL runtime"
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

/// Translate one player's calibrated controls into BlastEm bindings.
pub(super) fn calibrated_bindings(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
) -> Result<Vec<(&'static str, Binding)>> {
    use lunchbox_controller_probe::{
        duckstation::DigitalInput, linux_classic::AxisEndpoints, sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "BlastEm native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let counts = device
        .controls
        .as_ref()
        .context("BlastEm SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("BlastEm released state is absent")?;
    state.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native BlastEm profile")?;
    let mut bindings = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let control = super::CONTROLS
            .iter()
            .find(|(id, _)| *id == row.target_id)
            .map(|(id, _)| *id)
            .with_context(|| {
                format!(
                    "BlastEm target {} is outside the six-button contract",
                    row.target_id
                )
            })?;
        let input = row
            .input
            .context("BlastEm gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("BlastEm requires measured native controls")?;
        let binding = match physical.control(native.code)? {
            lunchbox_controller_probe::linux_classic::Control::Button(button) => {
                ensure!(
                    state.buttons.get(&button) == Some(&false),
                    "Release BlastEm controller buttons before capture"
                );
                Binding::Button(button)
            }
            lunchbox_controller_probe::linux_classic::Control::Axis(axis) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("BlastEm axis measurements are absent")?;
                let code = (native.code & 0xffff) as u8;
                let released = physical.axis_value(code, endpoints.released)?;
                let pressed = physical.axis_value(code, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&axis) == Some(&released),
                    "BlastEm axis rest differs from calibration"
                );
                ensure!(
                    (-super::DIGITAL_THRESHOLD..super::DIGITAL_THRESHOLD)
                        .contains(&i32::from(released)),
                    "BlastEm axis rest would hold a direction pressed; recalibrate the rest position"
                );
                if i32::from(pressed) > super::DIGITAL_THRESHOLD {
                    Binding::Axis {
                        index: axis,
                        positive: true,
                    }
                } else if i32::from(pressed) < -super::DIGITAL_THRESHOLD {
                    Binding::Axis {
                        index: axis,
                        positive: false,
                    }
                } else {
                    anyhow::bail!(
                        "BlastEm's SDL dead zone cannot be represented by the measured axis travel"
                    );
                }
            }
            lunchbox_controller_probe::linux_classic::Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("BlastEm hat measurements are absent")?;
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
                            "Release BlastEm directional hat before capture"
                        );
                        ensure!(
                            matches!(direction, 1 | 2 | 4 | 8),
                            "BlastEm needs a cardinal hat direction"
                        );
                        Binding::HatDirection {
                            hat: index,
                            direction,
                        }
                    }
                    _ => anyhow::bail!("BlastEm hat did not resolve to a native SDL hat"),
                }
            }
        };
        bindings.push((control, binding));
    }
    Ok(bindings)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) home_path: std::path::PathBuf,
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
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("BlastEm selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "BlastEm requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let mut blocks = String::new();
        let mut devices = Vec::new();
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
                "BlastEm players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            let bindings = calibrated_bindings(
                calibrations
                    .get(&player.controller_id)
                    .context("BlastEm calibration disappeared")?,
                &captured,
                &path,
            )?;
            let device = captured.device_at_path(&path)?.device_index;
            blocks.push_str(&super::pad_block(device, player.player, &bindings)?);
            devices.push(device);
            runtime_paths.push(path);
        }
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-blastem-")
            .tempdir()?;
        let home_path = directory.path().join("home");
        let config_dir = home_path.join(".config/blastem");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("blastem.cfg");
        std::fs::write(&config_path, blocks)?;
        ensure!(
            devices.windows(2).all(|pair| pair[0] != pair[1]),
            "BlastEm players resolved to the same SDL device index"
        );
        let mut hashes = std::collections::BTreeMap::new();
        for path in [&setup.probe_program, &setup.sdl_library, &setup.content] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(config_path.clone(), file_hash(&config_path)?);
        let session = Self {
            directory,
            home_path,
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
            ensure!(
                file_hash(path)? == *expected,
                "BlastEm launch input changed"
            );
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        for path in &self.runtime_paths {
            fresh
                .device_at_path(path)
                .with_context(|| format!("BlastEm controller disappeared: {path}"))?;
        }
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
