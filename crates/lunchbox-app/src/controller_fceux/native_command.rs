//! Native Qt command preparation, without spawning an emulator.
use super::{session::PreparedSession, settings::SavedSetup};
use crate::{
    controller_catalog::Calibration,
    controller_native_process::cancelled,
    controllers::ControllerDevice,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::atomic::AtomicBool,
};

pub(crate) struct NativeSession {
    pub(crate) inputs: PreparedSession,
    pub(crate) executable: PathBuf,
    pub(crate) setup: SavedSetup,
    pub(crate) plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
}

mod startup;

impl NativeSession {
    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == &self.plan,
            "FCEUX launch plan changed after preparation"
        );
        self.verify(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan(plan)?;
        if let Err(error) = startup::confirm(self, &mut child, cancel) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "FCEUX launch input/runtime changed"
            );
        }
        self.inputs.verify(cancel)
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.inputs.check_health()
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
        anyhow::bail!("FCEUX Qt calibrated launch requires native Linux");
    };
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "FCEUX identity differs or custom environment needs resolution"
    );
    let executable = executable.canonicalize()?;
    ensure!(
        executable == original.program.canonicalize()?,
        "FCEUX executable differs from selection"
    );
    ensure!(
        original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
        "FCEUX calibrated launch requires exactly the saved ROM argument"
    );
    let cwd = original.current_directory.canonicalize()?;
    let mut files = BTreeMap::new();
    for path in [
        &original.program,
        &executable,
        &setup.probe_program,
        &setup.sdl_library,
        &setup.bubblewrap_program,
        &setup.content,
    ] {
        files.insert(path.clone(), file_hash(path)?);
    }
    ensure!(
        files[&executable].eq_ignore_ascii_case(&setup.executable_sha256),
        "FCEUX executable differs from saved trusted Qt runtime"
    );
    let inputs = PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
    let mut plan = original.clone();
    plan.program = setup.bubblewrap_program.clone();
    // Qt GetBaseDirectory gives FCEUX_CONFIG_DIR precedence over both
    // FCEUX_HOME/.fceux and HOME/.fceux. Set it only inside the child sandbox.
    plan.arguments = vec![
        "--die-with-parent".into(),
        "--bind".into(),
        "/".into(),
        "/".into(),
        "--setenv".into(),
        "FCEUX_CONFIG_DIR".into(),
        setup.base_directory.as_os_str().to_owned(),
    ];
    inputs.append_mounts(&mut plan.arguments)?;
    plan.arguments.extend([
        "--chdir".into(),
        cwd.into_os_string(),
        "--".into(),
        executable.as_os_str().to_owned(),
    ]);
    plan.arguments.extend_from_slice(&original.arguments);
    let session = NativeSession {
        inputs,
        executable,
        setup: setup.clone(),
        plan,
        files,
    };
    session.verify(cancel)?;
    Ok(session)
}
