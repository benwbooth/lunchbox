//! Native SDL launch ownership; never invoked during settings review.
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

mod startup;

pub(crate) struct NativeSession {
    pub(crate) inputs: PreparedSession,
    pub(crate) executable: PathBuf,
    pub(crate) setup: SavedSetup,
    pub(crate) plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
    cwd: PathBuf,
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
            "SameBoy launch plan changed after preparation"
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
        ensure!(
            native_path(&self.setup, &self.executable, &self.cwd)?.canonicalize()?
                == self.setup.source_config.canonicalize()?,
            "SameBoy native configuration selection changed"
        );
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "SameBoy launch input/runtime changed"
            );
        }
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
        anyhow::bail!("SameBoy SDL calibrated launch requires native Linux, not Wine/Flatpak");
    };
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "SameBoy identity differs or custom environment needs resolution"
    );
    let executable = executable.canonicalize()?;
    ensure!(
        executable == original.program.canonicalize()?,
        "SameBoy launch executable differs from selection"
    );
    ensure!(
        original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
        "SameBoy native calibrated launch currently requires exactly the saved ROM argument"
    );
    let cwd = original.current_directory.canonicalize()?;
    ensure!(
        native_path(setup, &executable, &cwd)?.canonicalize()?
            == setup.source_config.canonicalize()?,
        "SameBoy saved config differs from SDL's XDG/HOME/cwd selection"
    );
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
        "SameBoy executable differs from the saved trusted SDL runtime"
    );
    // SDL initializes input devices and reads/writes configuration before
    // parsing ordinary options. Do not launch a supposedly harmless --version
    // subprocess against the original configuration as a version probe.
    let inputs =
        PreparedSession::prepare(setup, calibrations, inventory, cancel, setup.runtime.abi)?;
    let mut plan = original.clone();
    plan.program = setup.bubblewrap_program.clone();
    plan.arguments =
        inputs
            .configuration
            .overlay_arguments(&executable, &original.arguments, &cwd)?;
    let session = NativeSession {
        inputs,
        executable,
        setup: setup.clone(),
        plan,
        files,
        cwd,
    };
    session.verify(cancel)?;
    Ok(session)
}

fn native_path(
    setup: &SavedSetup,
    executable: &std::path::Path,
    cwd: &std::path::Path,
) -> Result<PathBuf> {
    let directory = executable
        .parent()
        .ok_or_else(|| anyhow::anyhow!("SameBoy executable has no directory"))?;
    super::preferences::resolve(directory, setup.runtime.data_directory.path(), cwd)
}
