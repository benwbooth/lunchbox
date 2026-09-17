//! Native Linux launch-time SDL capture. Never called from settings review.
use super::settings::{Runtime, SavedSetup};
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

fn observe(
    runtime: &Runtime,
    path: Option<&str>,
    // `LD_LIBRARY_PATH` for the probe when its SDL needs sibling runtime
    // libraries (sdl2-compat's SDL3). Native host SDL resolves without it.
    probe_library_path: Option<&std::ffi::OsStr>,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&runtime.probe_program);
    if let Some(library_path) = probe_library_path {
        command.env("LD_LIBRARY_PATH", library_path);
    }
    command
        .arg("--sdl2-inventory")
        .arg("--sdl-library")
        .arg(&runtime.sdl_library);
    if let Some(path) = path {
        command.arg("--sdl2-controls-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot = serde_json::from_slice(&output).context("Invalid MAME SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == runtime.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&runtime.sdl_library)?,
        "MAME probe inspected a different SDL runtime"
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
    runtime: Runtime,
    probe_hash: String,
    probe_library_path: Option<std::ffi::OsString>,
}

impl InputSession {
    pub(crate) fn capture(
        setup: &SavedSetup,
        inventory: &[ControllerDevice],
        probe_library_path: Option<&std::ffi::OsStr>,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.validate()?;
        let runtime = setup
            .runtime
            .as_ref()
            .context("MAME setup needs explicit probe_program and sdl_library runtime paths")?;
        let probe_hash = file_hash(&runtime.probe_program)?;
        let mut selected = Vec::new();
        for player in &setup.players {
            let found: Vec<_> = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "MAME selected physical controller is missing or ambiguous"
            );
            selected.push(found[0].device_path.clone());
        }
        #[cfg(target_os = "linux")]
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(runtime, None, probe_library_path, cancel)?);
        let mut devices = initial.devices.clone();
        let mut physical_paths = HashMap::new();
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
                    "MAME physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(runtime, Some(&path), probe_library_path, cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            super::sdl::native_id(&path, &captured.devices)?;
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(
                &captured.devices,
                &path,
                captured.device_at_path(&path)?.device_index,
            )?;
            let device = captured
                .devices
                .into_iter()
                .find(|device| device.path.as_deref() == Some(&path))
                .context("MAME physical SDL device disappeared")?;
            let slot = devices
                .iter_mut()
                .find(|device| device.path.as_deref() == Some(&path))
                .context("MAME SDL routing changed")?;
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
            probe_library_path: probe_library_path.map(|path| path.to_owned()),
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
            "MAME probe changed during preparation"
        );
        self.initial.ensure_same_routing(&routing(observe(
            &self.runtime,
            None,
            self.probe_library_path.as_deref(),
            cancel,
        )?))?;
        // Stored capture devices pin their SDL index alongside the path.
        let fresh = routing(observe(
            &self.runtime,
            None,
            self.probe_library_path.as_deref(),
            cancel,
        )?);
        for stored in &self.devices {
            let Some(path) = stored.path.as_deref() else {
                continue;
            };
            let device = fresh.device_at_path(path)?;
            ensure!(
                device.device_index == stored.device_index,
                "MAME SDL joystick moved before launch"
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
        let fresh = routing(observe(
            &self.runtime,
            None,
            self.probe_library_path.as_deref(),
            &AtomicBool::new(false),
        )?);
        self.initial.ensure_same_routing(&fresh)?;
        for stored in &self.devices {
            let Some(path) = stored.path.as_deref() else {
                continue;
            };
            platform::require_unique_device_path(&fresh.devices, path, stored.device_index)?;
        }
        Ok(())
    }
}
