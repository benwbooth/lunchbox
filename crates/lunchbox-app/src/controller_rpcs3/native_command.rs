//! Owned native Linux RPCS3 session; startup checks are not gameplay validation.
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
    let controllers = inputs.controllers()?;
    let prepared = PreparedLaunch::create(setup, option, original, calibrations, &controllers)?;
    let session = NativeSession {
        inputs,
        prepared,
        setup: setup.clone(),
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
            "RPCS3 launch plan changed after mapping preparation"
        );
        self.verify(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan(plan)?;
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
                "RPCS3 exited before controller handoff"
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
                "RPCS3 did not establish SDL controller ownership before timeout"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn ready(&self, pid: u32) -> Result<bool> {
        let Some(log) = super::startup::child_log(pid)? else {
            return Ok(false);
        };
        if !super::startup::loaded_profile(&log, self.prepared.profile_path())?
            || !super::startup::confirm_routing(&log, &self.inputs.assignments)?
        {
            return Ok(false);
        }
        let expected_sdl = self.setup.sdl_library.canonicalize()?;
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
                "RPCS3 open descriptor inventory exceeds limit"
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
