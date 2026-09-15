//! Native Kronos launch ownership; never invoked during settings review.
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
            "Kronos launch plan changed after preparation"
        );
        self.verify(cancel)?;
        crate::emulator::spawn_launch_plan(plan)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        ensure!(
            file_hash(&self.executable)? == self.setup.executable_sha256,
            "Kronos executable differs from the saved trusted runtime"
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
        anyhow::bail!("Kronos calibrated launch requires a native build, not Wine/Flatpak");
    };
    ensure!(
        option.emulator_name.eq_ignore_ascii_case("Kronos")
            && setup.emulator_id == option.emulator_id
            && original.environment.is_empty(),
        "Kronos identity differs or custom environment needs resolution"
    );
    let executable = executable.canonicalize()?;
    ensure!(
        executable == original.program.canonicalize()?,
        "Kronos launch executable differs from selection"
    );
    ensure!(
        original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
        "Kronos calibrated launch currently requires exactly the saved game argument"
    );
    ensure!(
        file_hash(&executable)? == setup.executable_sha256,
        "Kronos executable differs from the saved trusted runtime"
    );
    let inputs = PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
    // Sandbox or direct is a packaging decision, not an OS one. The staged
    // kronos.ini shadows the source config through a bind mount; no
    // environment variable redirects that resolution, so direct launches
    // refuse instead of running unmapped.
    if !crate::controller_native_platform::use_bubblewrap_sandbox(&executable) {
        anyhow::bail!(
            "Kronos direct launch needs mount shadowing, which is unsupported; use a sandboxable native Linux packaging"
        );
    }
    let mut plan = original.clone();
    let cwd = original.current_directory.canonicalize()?;
    plan.program = setup.bubblewrap_program.clone();
    plan.arguments = vec![
        "--die-with-parent".into(),
        "--bind".into(),
        "/".into(),
        "/".into(),
        "--bind".into(),
        inputs.config_path.as_os_str().to_owned(),
        setup.config_path.as_os_str().to_owned(),
        "--chdir".into(),
        cwd.into_os_string(),
        "--".into(),
        executable.as_os_str().to_owned(),
        setup.content.as_os_str().to_owned(),
    ];
    let session = NativeSession {
        inputs,
        executable,
        setup: setup.clone(),
        plan,
    };
    session.verify(cancel)?;
    Ok(session)
}
