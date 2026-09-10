//! Owned native Linux Flycast session; startup checks are not gameplay validation.
use super::{launch::PreparedLaunch, session::InputSession, settings::SavedSetup};
use crate::{
    controller_catalog::Calibration,
    controller_native_process::{cancelled, native_pid},
    controllers::ControllerDevice,
    emulator::{LaunchPlan, RomEmulatorOption},
};
use anyhow::{Result, ensure};
use std::{
    collections::{BTreeSet, HashMap},
    os::unix::fs::MetadataExt,
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub(crate) struct NativeSession {
    inputs: InputSession,
    prepared: PreparedLaunch,
    setup: SavedSetup,
    log: Option<super::startup_log::StartupLog>,
}

pub(crate) fn prepare(
    setup: &SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    inventory: &[ControllerDevice],
    option: &RomEmulatorOption,
    original: &LaunchPlan,
    cancel: &AtomicBool,
) -> Result<NativeSession> {
    cancelled(cancel)?;
    let inputs = InputSession::capture(setup, inventory, cancel)?;
    let controllers = setup
        .players
        .iter()
        .map(|player| {
            let path = inputs
                .physical_paths
                .get(&player.controller_id)
                .ok_or_else(|| anyhow::anyhow!("Flycast physical controller disappeared"))?;
            let device = inputs
                .devices
                .iter()
                .find(|device| device.path.as_deref() == Some(path.as_str()))
                .ok_or_else(|| anyhow::anyhow!("Flycast SDL controller disappeared"))?;
            Ok(super::prepared::Controller {
                controller_id: &player.controller_id,
                device,
                native_instance: device.instance_id,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    // Provisional IDs from the matching fresh SDL runtime. They are not treated
    // as confirmed until the actual child's opened-joystick reports agree.
    let instances = inputs
        .devices
        .iter()
        .map(|device| device.instance_id)
        .collect::<Vec<_>>();
    let prepared = PreparedLaunch::create(
        setup,
        option,
        original,
        calibrations,
        &controllers,
        &instances,
    )?;
    let session = NativeSession {
        inputs,
        prepared,
        setup: setup.clone(),
        log: None,
    };
    session.verify(cancel)?;
    Ok(session)
}

impl NativeSession {
    pub(crate) fn plan(&self) -> &LaunchPlan {
        self.prepared.plan()
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        self.inputs.verify(cancel)?;
        self.prepared.verify_before_launch()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.inputs.check_health()
    }

    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == self.plan(),
            "Flycast launch plan changed after mapping preparation"
        );
        self.verify(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan_with_controller_pipes(plan)?;
        match super::startup_log::StartupLog::attach(&mut child) {
            Ok(log) => self.log = Some(log),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
        if let Err(error) = self.confirm(&mut child, cancel) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }

    fn confirm(&self, child: &mut std::process::Child, cancel: &AtomicBool) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            cancelled(cancel)?;
            ensure!(
                child.try_wait()?.is_none(),
                "Flycast exited before controller handoff"
            );
            if let Some(pid) = native_pid(child.id(), &self.plan().program)?
                && self.ready(pid)?
            {
                // Native cfg copies may now be legitimately changing. Only
                // recheck physical routing here, not prelaunch cfg contents.
                self.inputs.verify(cancel)?;
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "Flycast did not establish SDL controller ownership before timeout"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn ready(&self, pid: u32) -> Result<bool> {
        let expected_sdl = self.setup.sdl_library.canonicalize()?;
        let opened = self
            .log
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Flycast startup capture is missing"))?
            .opened()?;
        for device in &self.inputs.devices {
            let Some(record) = opened.get(&device.instance_id) else {
                return Ok(false);
            };
            ensure!(
                device.name.as_deref() == Some(record.name.as_str()),
                "Flycast child joystick identity differs from discovery"
            );
        }
        ensure!(
            opened.len() == self.inputs.devices.len(),
            "Flycast opened unexpected joysticks during handoff"
        );
        let maps = std::fs::read_to_string(format!("/proc/{pid}/maps"))?;
        if !maps.lines().any(|line| {
            let path = line
                .split_whitespace()
                .skip(5)
                .collect::<Vec<_>>()
                .join(" ")
                .replace("\\040", " ");
            Path::new(&path) == expected_sdl
        }) {
            return Ok(false);
        }
        let mut opened = BTreeSet::new();
        for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
            ensure!(
                index < 4096,
                "Flycast open descriptor inventory exceeds limit"
            );
            match std::fs::metadata(entry?.path()) {
                Ok(metadata) => {
                    opened.insert((metadata.dev(), metadata.ino(), metadata.rdev()));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            }
        }
        for path in self.inputs.physical_paths.values() {
            let metadata = std::fs::metadata(path)?;
            if !opened.contains(&(metadata.dev(), metadata.ino(), metadata.rdev())) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
