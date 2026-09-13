//! Native GTK launch ownership; never invoked during settings review.
use super::{
    flatpak::PreparedFlatpak,
    isolation::PreparedConfig,
    session::{PreparedSession, Runtime},
    settings::SavedSetup,
};
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
    native_cwd: Option<PathBuf>,
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
            "Snes9x launch plan changed after preparation"
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
        if let Some(cwd) = &self.native_cwd {
            ensure!(
                PreparedConfig::native_path(cwd)?.canonicalize()?
                    == self.setup.source_config.canonicalize()?,
                "Snes9x native configuration selection changed"
            );
        }
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "Snes9x launch input/runtime changed"
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
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "Snes9x identity differs or custom environment needs resolution"
    );
    let cwd = original.current_directory.canonicalize()?;
    let (runtime, executable, native_cwd) = match &option.executable {
        EmulatorExecutable::Native(executable) => {
            let executable = executable.canonicalize()?;
            ensure!(
                executable == original.program.canonicalize()?,
                "Snes9x launch executable differs from selection"
            );
            ensure!(
                original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
                "Snes9x native calibrated launch currently requires exactly the saved ROM argument"
            );
            ensure!(
                PreparedConfig::native_path(&cwd)?.canonicalize()?
                    == setup.source_config.canonicalize()?,
                "Snes9x saved config differs from GTK's XDG/HOME/cwd selection"
            );
            (Runtime::Native, executable, Some(cwd.clone()))
        }
        EmulatorExecutable::Flatpak { command, app_id } => {
            let runtime =
                PreparedFlatpak::prepare(setup, option, original, command, app_id, cancel)?;
            let executable = runtime.executable().to_path_buf();
            (Runtime::Flatpak(runtime), executable, None)
        }
        EmulatorExecutable::Wine { .. } => {
            anyhow::bail!("Snes9x GTK calibrated launch does not support Wine");
        }
    };
    let mut files = BTreeMap::new();
    let mut paths = vec![
        &executable,
        &setup.probe_program,
        &setup.sdl_library,
        &setup.content,
    ];
    if native_cwd.is_some() {
        paths.extend([&original.program, &setup.bubblewrap_program]);
    }
    for path in paths {
        files.insert(path.clone(), file_hash(path)?);
    }
    ensure!(
        files[&executable].eq_ignore_ascii_case(&setup.executable_sha256),
        "Snes9x executable differs from the saved trusted GTK runtime"
    );
    // GTK initializes input devices and reads/writes configuration before
    // parsing ordinary options. Do not launch a supposedly harmless --version
    // subprocess against the original configuration as a version probe.
    let mut inputs = PreparedSession::prepare(setup, calibrations, inventory, runtime, cancel)?;
    let mut plan = original.clone();
    if let Some(arguments) = inputs.stage_flatpak_launch(original)? {
        plan.arguments = arguments;
    } else {
        plan.program = setup.bubblewrap_program.clone();
        plan.arguments =
            inputs
                .configuration
                .overlay_arguments(&executable, &original.arguments, &cwd)?;
    }
    let session = NativeSession {
        inputs,
        executable,
        setup: setup.clone(),
        plan,
        files,
        native_cwd,
    };
    session.verify(cancel)?;
    Ok(session)
}
