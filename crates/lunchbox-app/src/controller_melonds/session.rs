//! Native Linux launch-time SDL capture. Never called from settings review.
use super::settings::SavedSetup;
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    file_hash,
    sdl2::{Device, Snapshot},
};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(runtime: &SavedSetup, path: Option<&str>, cancel: &AtomicBool) -> Result<Snapshot> {
    ensure!(
        runtime.runtime_libraries.is_empty(),
        "melonDS SDL2 capture does not yet support explicit dependency preloading"
    );
    let mut command = Command::new(&runtime.probe_program);
    command
        .arg("--sdl2-inventory")
        .arg("--sdl-library")
        .arg(&runtime.sdl_library);
    if let Some(path) = path {
        command.arg("--sdl2-controls-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid melonDS SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == runtime.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&runtime.sdl_library)?,
        "melonDS probe inspected a different SDL runtime"
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

pub(crate) struct InputSession {
    pub physical_paths: HashMap<String, String>,
    pub devices: Vec<Device>,
    topology: InputTopology,
    initial: Snapshot,
    runtime: SavedSetup,
    probe_hash: String,
}

impl InputSession {
    pub(crate) fn capture(
        setup: &SavedSetup,
        inventory: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.validate()?;
        let runtime = setup;
        let probe_hash = file_hash(&runtime.probe_program)?;
        let mut selected = Vec::new();
        for player in &setup.players {
            let found: Vec<_> = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "melonDS selected physical controller is missing or ambiguous"
            );
            selected.push(found[0].device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(runtime, None, cancel)?);
        let mut devices = initial.devices.clone();
        let mut physical_paths = HashMap::new();
        for (player, selected) in setup.players.iter().zip(&selected) {
            let path = topology.resolve_runtime_path(
                selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            let captured = observe(runtime, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            ensure!(
                captured
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(&path))
                    .count()
                    == 1,
                "melonDS SDL physical path is ambiguous"
            );
            let device = captured
                .devices
                .into_iter()
                .find(|device| device.path.as_deref() == Some(&path))
                .context("melonDS physical SDL device disappeared")?;
            let slot = devices
                .iter_mut()
                .find(|device| device.path.as_deref() == Some(&path))
                .context("melonDS SDL routing changed")?;
            *slot = device;
            physical_paths.insert(player.controller_id.clone(), path);
        }
        let session = Self {
            physical_paths,
            devices,
            topology,
            initial,
            runtime: runtime.clone(),
            probe_hash,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        ensure!(
            file_hash(&self.runtime.probe_program)? == self.probe_hash,
            "melonDS probe changed during preparation"
        );
        self.initial
            .ensure_same_routing(&routing(observe(&self.runtime, None, cancel)?))?;
        self.topology.verify()
    }

    pub(crate) fn device(&self, controller_id: &str) -> Result<&Device> {
        let path = self
            .physical_paths
            .get(controller_id)
            .context("melonDS controller was not captured")?;
        let found = self
            .devices
            .iter()
            .filter(|device| device.path.as_deref() == Some(path.as_str()))
            .collect::<Vec<_>>();
        ensure!(
            found.len() == 1,
            "melonDS captured controller is missing or ambiguous"
        );
        Ok(found[0])
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
