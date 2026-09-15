//! Launch-time SDL3 capture; never invoked by setup review.
use super::{prepared::Controller, settings::SavedSetup};
#[cfg(target_os = "linux")]
use crate::controller_bizhawk_guard::InputTopology;
#[cfg(not(target_os = "linux"))]
use crate::controller_native_platform as platform;
use crate::{
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{Snapshot, file_hash, players::PCSX2_CONTRACT};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(setup: &SavedSetup, paths: &[String], cancel: &AtomicBool) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command
        .arg("--sdl-library")
        .arg(&setup.sdl_library)
        .arg("--pcsx2-player-probe")
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
        serde_json::from_slice(&output).context("Invalid PCSX2 SDL3 capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
            && snapshot
                .player_probe
                .as_ref()
                .is_some_and(|report| report.duckstation_revision == PCSX2_CONTRACT),
        "PCSX2 capture used a different runtime or player contract"
    );
    Ok(snapshot)
}

pub(crate) struct InputSession {
    pub snapshot: Snapshot,
    pub physical_paths: HashMap<String, String>,
    #[cfg(target_os = "linux")]
    topology: InputTopology,
    setup: SavedSetup,
    probe_hash: String,
}

fn comparable(snapshot: &Snapshot, bindings: bool) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(snapshot)?;
    // Event counts and warnings are diagnostic, not controller identity.
    value
        .as_object_mut()
        .context("Invalid PCSX2 snapshot shape")?
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
                .context("Invalid PCSX2 device shape")?;
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
                "PCSX2 physical controller is missing or ambiguous"
            );
            selected.push(found[0].device_path.clone());
        }
        #[cfg(target_os = "linux")]
        let topology = InputTopology::capture(&selected)?;
        let initial = observe(setup, &[], cancel)?;
        let mut physical_paths = HashMap::new();
        let mut paths = Vec::new();
        for (player, selected) in setup.players.iter().zip(&selected) {
            // Linux resolves through the sysfs topology; other hosts
            // match the SDL device-interface path and require uniqueness.
            #[cfg(target_os = "linux")]
            let path = topology.resolve_runtime_path(
                selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            #[cfg(not(target_os = "linux"))]
            let path = {
                let selected_string = selected.to_string_lossy().into_owned();
                let candidates = initial
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(selected_string.as_str()))
                    .collect::<Vec<_>>();
                ensure!(
                    candidates.len() == 1,
                    "PCSX2 physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            physical_paths.insert(player.controller_id.clone(), path.clone());
            paths.push(path);
        }
        let snapshot = observe(setup, &paths, cancel)?;
        ensure!(
            comparable(&initial, false)? == comparable(&snapshot, false)?,
            "PCSX2 SDL routing changed between inventory and binding capture"
        );
        let session = Self {
            snapshot,
            physical_paths,
            #[cfg(target_os = "linux")]
            topology,
            setup: setup.clone(),
            probe_hash,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn controllers(&self) -> Result<Vec<Controller<'_>>> {
        let projection = self
            .snapshot
            .player_probe
            .as_ref()
            .context("PCSX2 player capture is missing")?;
        self.setup
            .players
            .iter()
            .map(|player| {
                let path = &self.physical_paths[&player.controller_id];
                let device = self.snapshot.device_at_path(path)?;
                let assigned = projection.at_path(path)?;
                ensure!(
                    assigned.instance_id == device.instance_id
                        && assigned.is_gamepad == device.is_gamepad,
                    "PCSX2 physical and player captures disagree"
                );
                Ok(Controller {
                    controller_id: &player.controller_id,
                    device,
                    sdl_player: u8::try_from(assigned.projected_player_id)?,
                })
            })
            .collect()
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        #[cfg(target_os = "linux")]
        self.topology.verify()?;
        ensure!(
            file_hash(&self.setup.probe_program)? == self.probe_hash,
            "PCSX2 probe changed"
        );
        for runtime in &self.snapshot.runtime_libraries {
            ensure!(
                file_hash(&runtime.path)? == runtime.sha256,
                "PCSX2 runtime dependency changed"
            );
        }
        ensure!(
            file_hash(&self.snapshot.library)? == self.snapshot.library_sha256,
            "PCSX2 SDL library changed"
        );
        self.controllers()?;
        let paths = self
            .setup
            .players
            .iter()
            .map(|player| self.physical_paths[&player.controller_id].clone())
            .collect::<Vec<_>>();
        let current = observe(&self.setup, &paths, cancel)?;
        ensure!(
            comparable(&current, true)? == comparable(&self.snapshot, true)?,
            "PCSX2 SDL player routing or physical bindings changed before handoff"
        );
        #[cfg(not(target_os = "linux"))]
        for path in self.physical_paths.values() {
            platform::require_unique_sdl3_path(&current.devices, path)?;
        }
        #[cfg(target_os = "linux")]
        {
            return self.topology.verify();
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Ok(());
        }
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        return self.topology.verify();
        #[cfg(not(target_os = "linux"))]
        return self.verify_health_probe();
    }

    #[cfg(not(target_os = "linux"))]
    fn verify_health_probe(&self) -> Result<()> {
        let snapshot = observe(&self.setup, &[], &AtomicBool::new(false))?;
        ensure!(
            comparable(&snapshot, false)? == comparable(&self.snapshot, false)?,
            "PCSX2 SDL routing changed"
        );
        for path in self.physical_paths.values() {
            platform::require_unique_sdl3_path(&snapshot.devices, path)?;
        }
        Ok(())
    }
}
