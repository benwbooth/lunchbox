//! Retained native command preparation; no emulator is spawned by preparation.
use super::{
    paths::ConfigPath, prepared::PreparedConfig, session::InputSession, settings::SavedSetup,
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

pub(crate) struct NativeSession {
    pub(crate) setup: SavedSetup,
    pub(crate) inputs: InputSession,
    pub(crate) config: PreparedConfig,
    pub(crate) native_path: ConfigPath,
    pub(crate) executable: PathBuf,
    pub(crate) plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
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
        anyhow::bail!("melonDS mapping requires native Linux");
    };
    let executable = executable.canonicalize()?;
    ensure!(
        setup.emulator_id == option.emulator_id
            && original.program.canonicalize()? == executable
            && original.environment.is_empty()
            && original.retroarch_content.is_none()
            && original.arguments.len() == 1,
        "melonDS mapping requires the selected native executable and one plain ROM argument"
    );
    let argument = PathBuf::from(&original.arguments[0]);
    let argument = if argument.is_absolute() {
        argument
    } else {
        original.current_directory.join(argument)
    };
    let content = setup.content.canonicalize()?;
    ensure!(
        argument.canonicalize()? == content && content.is_file(),
        "melonDS ROM differs from saved setup"
    );
    let native_path = ConfigPath::capture(&executable, &setup.source_config)?;
    let mut files = BTreeMap::new();
    for path in [
        &executable,
        &content,
        &setup.probe_program,
        &setup.sdl_library,
        &setup.bubblewrap_program,
    ] {
        files.insert(path.clone(), file_hash(path)?);
    }
    ensure!(
        files[&executable].eq_ignore_ascii_case(&setup.executable_sha256),
        "melonDS executable differs from trusted hash"
    );
    let inputs = InputSession::capture(setup, inventory, cancel)?;
    let controller_id = &setup.players[0].controller_id;
    let config = PreparedConfig::create(
        setup,
        calibrations,
        controller_id,
        inputs.device(controller_id)?,
    )?;
    let mut plan = original.clone();
    plan.program = setup.bubblewrap_program.clone();
    plan.arguments = vec![
        "--die-with-parent".into(),
        "--bind".into(),
        "/".into(),
        "/".into(),
    ];
    config.append_mount(native_path.path(), &mut plan.arguments)?;
    plan.arguments.extend([
        "--chdir".into(),
        original.current_directory.canonicalize()?.into_os_string(),
        "--".into(),
        executable.as_os_str().to_owned(),
        content.into_os_string(),
    ]);
    let session = NativeSession {
        setup: setup.clone(),
        inputs,
        config,
        native_path,
        executable,
        plan,
        files,
    };
    session.verify(cancel)?;
    Ok(session)
}

impl NativeSession {
    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == &self.plan,
            "melonDS launch plan changed after preparation"
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
    pub(crate) fn plan(&self) -> &LaunchPlan {
        &self.plan
    }
    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "melonDS launch input/runtime changed"
            );
        }
        self.native_path.verify()?;
        self.config.verify()?;
        self.inputs.verify(cancel)
    }
    pub(crate) fn check_health(&self) -> Result<()> {
        self.config.verify_source()?;
        self.inputs.check_health()
    }
}

mod startup;
