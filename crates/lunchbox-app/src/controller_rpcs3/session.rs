//! Launch-time SDL3 capture; never invoked by setup review.
use super::{prepared::Controller, settings::SavedSetup};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{Snapshot, file_hash};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(setup: &SavedSetup, paths: &[String], cancel: &AtomicBool) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command
        .arg("--sdl-library")
        .arg(&setup.sdl_library)
        .arg("--hint")
        .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
    for path in &setup.runtime_libraries {
        command.arg("--runtime-library").arg(path);
    }
    for path in paths {
        command.arg("--bindings-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid RPCS3 SDL3 capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "RPCS3 capture used a different SDL runtime"
    );
    Ok(snapshot)
}

pub(crate) struct InputSession {
    pub snapshot: Snapshot,
    pub physical_paths: HashMap<String, String>,
    pub assignments: Vec<super::routing::Assignment>,
    topology: InputTopology,
    setup: SavedSetup,
    probe_hash: String,
}

fn comparable(snapshot: &Snapshot, bindings: bool) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(snapshot)?;
    // Event counts and warnings are diagnostic, not controller identity.
    value
        .as_object_mut()
        .context("Invalid RPCS3 snapshot shape")?
        .remove("warnings");
    if let Some(report) = value
        .get_mut("player_probe")
        .and_then(serde_json::Value::as_object_mut)
    {
        report.remove("events_processed");
    }
    if !bindings
        && let Some(devices) = value
            .get_mut("devices")
            .and_then(serde_json::Value::as_array_mut)
    {
        for device in devices {
            let object = device
                .as_object_mut()
                .context("Invalid RPCS3 device shape")?;
            object.remove("resolved");
            object.remove("linux_classic");
        }
    }
    Ok(value)
}

impl InputSession {
    pub(crate) fn capture(
        setup: &SavedSetup,
        inventory: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.validate()?;
        let probe_hash = file_hash(&setup.probe_program)?;
        let mut selected = Vec::new();
        for player in &setup.players {
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "RPCS3 physical controller is missing or ambiguous"
            );
            selected.push(found[0].device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = observe(setup, &[], cancel)?;
        let mut physical_paths = HashMap::new();
        let mut paths = Vec::new();
        for (player, selected) in setup.players.iter().zip(&selected) {
            let path = topology.resolve_runtime_path(
                selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            physical_paths.insert(player.controller_id.clone(), path.clone());
            paths.push(path);
        }
        let snapshot = observe(setup, &paths, cancel)?;
        ensure!(
            comparable(&initial, false)? == comparable(&snapshot, false)?,
            "RPCS3 SDL routing changed between inventory and binding capture"
        );
        // Provisional naming assumes every enumerated gamepad opens in the child.
        // Native confirmation must reject open failures or any name/order change.
        let candidates = snapshot
            .devices
            .iter()
            .filter(|device| device.is_gamepad)
            .map(|device| {
                Ok(super::routing::Device {
                    instance: device.instance_id,
                    name: device
                        .gamepad_name
                        .as_deref()
                        .context("RPCS3 requires SDL gamepad names from an updated probe")?,
                    path: device
                        .path
                        .as_deref()
                        .context("RPCS3 gamepad path is missing")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let assignments = super::routing::project(&candidates)?;
        let session = Self {
            assignments,
            snapshot,
            physical_paths,
            topology,
            setup: setup.clone(),
            probe_hash,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    /// Provisional assignments belong to this snapshot, not another process's
    /// instance IDs. The session owner must confirm actual native child routing.
    pub(crate) fn controllers(&self) -> Result<Vec<Controller<'_>>> {
        self.setup
            .players
            .iter()
            .map(|player| {
                let path = &self.physical_paths[&player.controller_id];
                let device = self.snapshot.device_at_path(path)?;
                let matches = self
                    .assignments
                    .iter()
                    .filter(|assignment| &assignment.path == path)
                    .collect::<Vec<_>>();
                ensure!(
                    matches.len() == 1,
                    "RPCS3 native device assignment is missing or ambiguous"
                );
                let assigned = matches[0];
                ensure!(
                    assigned.instance == device.instance_id && device.is_gamepad,
                    "RPCS3 physical and player captures disagree"
                );
                Ok(Controller {
                    controller_id: &player.controller_id,
                    device,
                    native_device: &assigned.native_device,
                })
            })
            .collect()
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        ensure!(
            file_hash(&self.setup.probe_program)? == self.probe_hash,
            "RPCS3 probe changed"
        );
        for runtime in &self.snapshot.runtime_libraries {
            ensure!(
                file_hash(&runtime.path)? == runtime.sha256,
                "RPCS3 runtime dependency changed"
            );
        }
        ensure!(
            file_hash(&self.snapshot.library)? == self.snapshot.library_sha256,
            "RPCS3 SDL library changed"
        );

        let paths = self
            .setup
            .players
            .iter()
            .map(|player| self.physical_paths[&player.controller_id].clone())
            .collect::<Vec<_>>();
        let current = observe(&self.setup, &paths, cancel)?;
        ensure!(
            comparable(&current, true)? == comparable(&self.snapshot, true)?,
            "RPCS3 SDL player routing or physical bindings changed before handoff"
        );
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
