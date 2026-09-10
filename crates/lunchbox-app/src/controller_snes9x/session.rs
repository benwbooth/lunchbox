//! Native GTK launch-time SDL inventory and physical calibration ownership.
use super::{isolation::PreparedConfig, physical::calibrated_pad, settings::SavedSetup};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{file_hash, sdl2::Snapshot};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(setup: &SavedSetup, path: Option<&str>, cancel: &AtomicBool) -> Result<Snapshot> {
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
        serde_json::from_slice(&output).context("Invalid Snes9x SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "Snes9x helper inspected a different SDL runtime"
    );
    // Native GTK assigns the lowest free joynum, with ten slots. At a fresh
    // successful enumeration it equals the SDL device index; hotplug invalidates
    // this projection and is caught by the retained topology/inventory checks.
    ensure!(
        snapshot.devices.len() <= 10
            && snapshot
                .devices
                .iter()
                .enumerate()
                .all(|(index, device)| device.device_index as usize == index),
        "Snes9x needs a complete contiguous native joystick inventory within ten slots"
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

pub(crate) struct PreparedSession {
    pub(crate) configuration: PreparedConfig,
    pub(crate) runtime_paths: Vec<String>,
    topology: InputTopology,
    initial: Snapshot,
    setup: SavedSetup,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &SavedSetup,
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
                .context("Snes9x selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "Snes9x requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let mut pads = Vec::new();
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
                "Snes9x players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            pads.push(calibrated_pad(
                calibrations
                    .get(&player.controller_id)
                    .context("Snes9x calibration disappeared")?,
                &captured,
                &path,
                player.player,
            )?);
            runtime_paths.push(path);
        }
        let configuration = PreparedConfig::prepare(setup, &pads)?;
        let session = Self {
            configuration,
            runtime_paths,
            topology,
            initial,
            setup: setup.clone(),
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        self.configuration.verify()?;
        self.initial
            .ensure_same_routing(&routing(observe(&self.setup, None, cancel)?))?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
