//! Native Linux launch-time SDL capture. Never called from settings review.
use super::settings::SavedSetup;
#[cfg(target_os = "linux")]
use crate::controller_bizhawk_guard::InputTopology;
#[cfg(not(target_os = "linux"))]
use crate::controller_native_platform as platform;
use crate::{
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
        serde_json::from_slice(&output).context("Invalid Flycast SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == runtime.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&runtime.sdl_library)?,
        "Flycast probe inspected a different SDL runtime"
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
    #[cfg(target_os = "linux")]
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
                "Flycast selected physical controller is missing or ambiguous"
            );
            selected.push(found[0].device_path.clone());
        }
        #[cfg(target_os = "linux")]
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(runtime, None, cancel)?);
        let mut devices = initial.devices.clone();
        let mut physical_paths = HashMap::new();
        for (player, selected) in setup.players.iter().zip(&selected) {
            // Linux resolves through the sysfs topology; other hosts
            // match the SDL device-interface path (uniqueness is enforced
            // again below for every host).
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
                    "Flycast physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(runtime, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            ensure!(
                captured
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(&path))
                    .count()
                    == 1,
                "Flycast SDL physical path is ambiguous"
            );
            let device = captured
                .devices
                .into_iter()
                .find(|device| device.path.as_deref() == Some(&path))
                .context("Flycast physical SDL device disappeared")?;
            let slot = devices
                .iter_mut()
                .find(|device| device.path.as_deref() == Some(&path))
                .context("Flycast SDL routing changed")?;
            *slot = device;
            physical_paths.insert(player.controller_id.clone(), path);
        }
        let session = Self {
            physical_paths,
            devices,
            #[cfg(target_os = "linux")]
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
        #[cfg(target_os = "linux")]
        self.topology.verify()?;
        ensure!(
            file_hash(&self.runtime.probe_program)? == self.probe_hash,
            "Flycast probe changed during preparation"
        );
        self.initial
            .ensure_same_routing(&routing(observe(&self.runtime, None, cancel)?))?;
        // Stored capture devices pin their SDL index alongside the path.
        let fresh = routing(observe(&self.runtime, None, cancel)?);
        for stored in &self.devices {
            let Some(path) = stored.path.as_deref() else {
                continue;
            };
            let device = fresh.device_at_path(path)?;
            ensure!(
                device.device_index == stored.device_index,
                "Flycast SDL joystick moved before launch"
            );
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(&fresh.devices, path, stored.device_index)?;
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
        let fresh = routing(observe(&self.runtime, None, &AtomicBool::new(false))?);
        self.initial.ensure_same_routing(&fresh)?;
        Ok(())
    }
}
