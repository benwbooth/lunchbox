//! Construct a pinned native Dolphin command without running the emulator.
use super::{session::PreparedSession, settings::SavedSetup};
use crate::{
    controller_catalog::Calibration,
    controller_dolphin::{ContentSnapshot, prepare_raw_content},
    controller_native_process::cancelled,
    controllers::ControllerDevice,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsString,
    path::PathBuf,
    sync::atomic::AtomicBool,
};

pub(crate) struct PreparedCommand {
    pub(crate) session: PreparedSession,
    pub(crate) plan: LaunchPlan,
    content: ContentSnapshot,
    runtime_files: BTreeMap<PathBuf, String>,
    executable: PathBuf,
}

impl PreparedCommand {
    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == &self.plan,
            "Dolphin launch plan changed after preparation"
        );
        cancelled(cancel)?;
        self.verify()?;
        let mut child = crate::emulator::spawn_launch_plan(plan)?;
        let startup = (|| -> Result<()> {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            loop {
                cancelled(cancel)?;
                ensure!(
                    child.try_wait()?.is_none(),
                    "Dolphin exited before controller handoff"
                );
                if let Some(pid) =
                    crate::controller_native_process::native_pid(child.id(), &self.executable)?
                    && self.session.child_inputs_ready(pid)?
                {
                    self.verify()?;
                    return Ok(());
                }
                ensure!(
                    std::time::Instant::now() < deadline,
                    "Dolphin controller handoff timed out"
                );
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        })();
        if let Err(error) = startup {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }
    pub(crate) fn verify(&self) -> Result<()> {
        self.content.verify()?;
        self.session.verify()?;
        for (path, expected) in &self.runtime_files {
            ensure!(
                file_hash(path)? == *expected,
                "Dolphin launch runtime changed during preparation"
            );
        }
        Ok(())
    }
}

pub(crate) fn prepare(
    setup: &SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    devices: &[ControllerDevice],
    option: &RomEmulatorOption,
    original: &LaunchPlan,
    cancel: &AtomicBool,
) -> Result<PreparedCommand> {
    cancelled(cancel)?;
    setup.validate()?;
    let EmulatorExecutable::Native(executable) = &option.executable else {
        anyhow::bail!("Dolphin calibrated launch requires a native Linux executable");
    };
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "Dolphin setup identity differs or custom environment needs resolution"
    );
    let executable = executable.canonicalize()?;
    ensure!(
        original.program.canonicalize()? == executable,
        "Dolphin launch executable differs from selected emulator"
    );
    // Normalize only the ordinary content/batch launch form. Movie, save-state,
    // NAND and arbitrary -C overrides have different controller precedence.
    let mut found_content = false;
    let mut expects_content = false;
    for argument in &original.arguments {
        if argument == setup.content.as_os_str() {
            ensure!(
                !found_content,
                "Dolphin launch has multiple content selections"
            );
            found_content = true;
            expects_content = false;
        } else if !expects_content && matches!(argument.to_str(), Some("-e" | "--exec")) {
            ensure!(!found_content, "Dolphin launch has an extra content option");
            expects_content = true;
        } else {
            ensure!(
                !expects_content && matches!(argument.to_str(), Some("-b" | "--batch")),
                "Dolphin calibrated launch currently accepts only content and batch arguments"
            );
        }
    }
    ensure!(
        found_content && !expects_content,
        "Dolphin launch does not select the saved disc"
    );
    let content = prepare_raw_content(&setup.content)?;
    ensure!(
        content.game_id.as_slice() == setup.game_id.as_bytes()
            && content.revision == setup.revision,
        "Dolphin saved game ID/revision differs from the actual disc"
    );
    let mut runtime_files = BTreeMap::new();
    for path in [&original.program, &executable, &setup.bubblewrap_program] {
        runtime_files.insert(path.clone(), file_hash(path)?);
    }
    ensure!(
        runtime_files[&executable].eq_ignore_ascii_case(&setup.executable_sha256),
        "Dolphin executable differs from the saved trusted runtime"
    );
    // CLI query is part of explicit launch preparation, never settings review.
    let (version, _) = crate::controller_native_process::capture(
        std::process::Command::new(&executable).arg("--version"),
        cancel,
    )?;
    ensure!(
        String::from_utf8_lossy(&version)
            .split_whitespace()
            .any(|word| word == "2606"),
        "Dolphin version differs from the pinned 2606 controller contract"
    );
    let session = PreparedSession::prepare(setup, calibrations, devices, cancel)?;
    let arguments = vec![
        OsString::from("--batch"),
        OsString::from("--user"),
        setup.user_directory.as_os_str().to_owned(),
        OsString::from("--exec"),
        setup.content.as_os_str().to_owned(),
    ];
    let mut plan = original.clone();
    plan.program = setup.bubblewrap_program.clone();
    plan.arguments = session.configuration.overlay_arguments(
        &executable,
        &arguments,
        &original.current_directory,
    )?;
    let result = PreparedCommand {
        session,
        plan,
        content,
        runtime_files,
        executable,
    };
    result.verify()?;
    cancelled(cancel)?;
    Ok(result)
}
