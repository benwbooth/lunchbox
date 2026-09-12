//! Native Kronos launch ownership; never invoked during settings review.
use super::settings::SavedSetup;
use crate::controller_catalog::Calibration;
use crate::controller_native_process::cancelled;
use crate::controllers::ControllerDevice;
use crate::emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};

pub(crate) struct NativeSession {
    pub(crate) executable: std::path::PathBuf,
    pub(crate) setup: SavedSetup,
    pub(crate) plan: LaunchPlan,
}

impl NativeSession {
    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        _cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(plan == &self.plan, "Kronos launch plan changed");
        Ok(crate::emulator::spawn_launch_plan(plan)?)
    }
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
    setup.validate()?;
    let EmulatorExecutable::Native(executable) = &option.executable else {
        anyhow::bail!("Kronos calibrated launch requires native Linux");
    };
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "Kronos identity differs or custom environment needs resolution"
    );
    let executable = executable.canonicalize()?;
    ensure!(
        executable == original.program.canonicalize()?,
        "Kronos launch executable differs from selection"
    );
    ensure!(
        original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
        "Kronos native calibrated launch requires exactly the saved game argument"
    );
    ensure!(
        file_hash(&executable)? == setup.executable_sha256,
        "Kronos executable differs from the saved trusted runtime"
    );
    let plan = original.clone();
    Ok(NativeSession {
        executable,
        setup: setup.clone(),
        plan,
    })
}
