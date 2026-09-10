//! Retained calibrated inputs and private configuration for a native launch.
use super::{
    configuration, evdev, inventory, isolation::PrivateConfiguration, settings::SavedSetup,
};
use crate::{
    controller_bizhawk_guard::InputTopology, controller_catalog::Calibration,
    controller_native_process::cancelled, controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use std::{collections::HashMap, sync::atomic::AtomicBool};

pub(crate) struct PreparedSession {
    pub(crate) configuration: PrivateConfiguration,
    topology: InputTopology,
    inventory: Vec<evdev::DeviceObservation>,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        devices: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.review(calibrations)?;
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matching = devices
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matching
                .next()
                .context("Dolphin selected controller is disconnected")?;
            ensure!(
                matching.next().is_none() && !device.is_virtual,
                "Dolphin needs an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let observed = inventory::capture::capture()?;
        cancelled(cancel)?;
        let mut pads = Vec::new();
        let mut resolved_setup = setup.clone();
        for (index, (player, selected)) in setup.players.iter().zip(&selected).enumerate() {
            // Match the calibrated kernel device, not a name or event number.
            // Merged motion/pointing nodes do not own indexed gameplay inputs.
            let path = topology.resolve_runtime_path(
                selected,
                observed
                    .iter()
                    .flat_map(|device| &device.nodes)
                    .filter(|node| !node.motion && !node.pointing)
                    .filter_map(|node| node.path.to_str()),
            )?;
            let device = observed
                .iter()
                .find(|device| {
                    device
                        .nodes
                        .iter()
                        .any(|node| node.path == std::path::Path::new(&path))
                })
                .context("Dolphin calibrated node has no native device")?;
            if !setup.resolve_devices_at_launch {
                ensure!(
                    device.qualifier == player.device_qualifier,
                    "Dolphin native device numbering changed; review the saved controller selection"
                );
            }
            // The kernel topology selected this exact physical device. Names
            // and same-name ordinals only serialize that resolved observation.
            resolved_setup.players[index].device_qualifier = device.qualifier.clone();
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Dolphin calibration disappeared")?;
            pads.push(evdev::resolve(calibration, player.port, device)?);
        }
        topology.verify()?;
        resolved_setup.resolve_devices_at_launch = false;
        let configuration = PrivateConfiguration::materialize(
            &setup.user_directory,
            configuration::prepare(&resolved_setup, &pads)?,
        )?;
        let result = Self {
            configuration,
            topology,
            inventory: observed,
        };
        result.verify()?;
        cancelled(cancel)?;
        Ok(result)
    }

    /// Device state may change normally; routing and capabilities may not.
    /// Call immediately before spawn, separately from configuration review.
    pub(crate) fn verify(&self) -> Result<()> {
        self.topology.verify()?;
        self.configuration.verify()?;
        let current = inventory::capture::capture()?;
        ensure!(
            current.len() == self.inventory.len(),
            "Dolphin native input inventory changed"
        );
        for (expected, actual) in self.inventory.iter().zip(&current) {
            ensure!(
                expected.qualifier == actual.qualifier
                    && expected.nodes.len() == actual.nodes.len(),
                "Dolphin native controller routing changed"
            );
            for (expected, actual) in expected.nodes.iter().zip(&actual.nodes) {
                ensure!(
                    expected.path == actual.path
                        && expected.motion == actual.motion
                        && expected.pointing == actual.pointing
                        && expected.keys == actual.keys
                        && expected.axes == actual.axes,
                    "Dolphin native controller capabilities changed"
                );
            }
        }
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    /// Confirm the child opened every node used to derive its evdev routing.
    /// This is startup ownership evidence, not a button-response test.
    pub(crate) fn child_inputs_ready(&self, pid: u32) -> Result<bool> {
        use std::os::unix::fs::MetadataExt;
        self.configuration.verify_child_mounts(pid)?;
        let mut opened = std::collections::BTreeSet::new();
        for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
            ensure!(
                index < 4096,
                "Dolphin child descriptor inventory exceeds limit"
            );
            let entry = entry?;
            match std::fs::metadata(entry.path()) {
                Ok(metadata) => {
                    opened.insert((metadata.dev(), metadata.ino(), metadata.rdev()));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            }
        }
        for node in self.inventory.iter().flat_map(|device| &device.nodes) {
            let metadata = std::fs::metadata(&node.path)?;
            if !opened.contains(&(metadata.dev(), metadata.ino(), metadata.rdev())) {
                return Ok(false);
            }
        }
        self.topology.verify()?;
        Ok(true)
    }
}
