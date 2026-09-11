//! Native ScummVM launch ownership; never invoked during settings review.
use super::{session::PreparedSession, settings::SavedSetup};
use crate::{
    controller_catalog::Calibration,
    controller_native_process::cancelled,
    controllers::ControllerDevice,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};

pub(crate) struct NativeSession {
    pub(crate) inputs: PreparedSession,
    pub(crate) executable: PathBuf,
    pub(crate) setup: SavedSetup,
    pub(crate) plan: LaunchPlan,
}

impl NativeSession {
    pub(crate) fn check_health(&self) -> Result<()> {
        self.inputs.check_health()
    }

    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == &self.plan,
            "ScummVM launch plan changed after preparation"
        );
        self.verify(cancel)?;
        crate::emulator::spawn_launch_plan(plan)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        ensure!(
            file_hash(&self.executable)? == self.setup.executable_sha256,
            "ScummVM executable differs from the saved trusted runtime"
        );
        self.inputs.verify(cancel)
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
        anyhow::bail!("ScummVM calibrated launch requires native Linux, not Wine/Flatpak");
    };
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "ScummVM identity differs or custom environment needs resolution"
    );
    let executable = executable.canonicalize()?;
    ensure!(
        executable == original.program.canonicalize()?,
        "ScummVM launch executable differs from selection"
    );
    ensure!(
        original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
        "ScummVM native calibrated launch currently requires exactly the saved game argument"
    );
    ensure!(
        file_hash(&executable)? == setup.executable_sha256,
        "ScummVM executable differs from the saved trusted runtime"
    );
    let inputs = PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
    let mut plan = original.clone();
    // The game is mounted through the private target, so the bare argument
    // is replaced with the ini selection and target name.
    plan.arguments = inputs.overlay_arguments(&original.arguments)?;
    // A private XDG_CONFIG_HOME keeps ScummVM's configuration, saves and
    // screenshots isolated from the user's own files.
    plan.environment.push((
        "XDG_CONFIG_HOME".into(),
        inputs.config_root.as_os_str().to_owned(),
    ));
    let session = NativeSession {
        inputs,
        executable,
        setup: setup.clone(),
        plan,
    };
    session.verify(cancel)?;
    Ok(session)
}
