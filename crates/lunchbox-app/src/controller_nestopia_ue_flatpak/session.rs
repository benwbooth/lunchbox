//! Target-SDL inventory, physical calibration, and private config ownership.

use super::{
    flatpak::PreparedFlatpak, isolation::PreparedConfig, physical::calibrated_pad,
    settings::SavedSetup,
};
use crate::{
    controller_bizhawk_guard::InputTopology, controller_catalog::Calibration,
    controller_native_process::cancelled, controllers::ControllerDevice, emulator::LaunchPlan,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::sdl2::Snapshot;
use std::{collections::HashMap, sync::atomic::AtomicBool};

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
    runtime_paths: Vec<String>,
    topology: InputTopology,
    initial: Snapshot,
    setup: SavedSetup,
    runtime: PreparedFlatpak,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        runtime: PreparedFlatpak,
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
                .context("Nestopia selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "Nestopia requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(runtime.observe(None, cancel)?);
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
                "Nestopia players resolved to the same controller"
            );
            let captured = runtime.observe(Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            pads.push(calibrated_pad(
                calibrations
                    .get(&player.controller_id)
                    .context("Nestopia calibration disappeared")?,
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
            runtime,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn stage_launch(
        &mut self,
        original: &LaunchPlan,
    ) -> Result<Vec<std::ffi::OsString>> {
        self.runtime.prepare_launch(
            &self.setup,
            &self.configuration,
            &self.initial,
            &self.runtime_paths,
            original,
        )
    }

    pub(crate) fn receipt_ready(&self) -> Result<bool> {
        self.runtime.receipt_ready()
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        self.configuration.verify()?;
        self.runtime.verify(cancel)?;
        self.initial
            .ensure_same_routing(&routing(self.runtime.observe(None, cancel)?))?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
