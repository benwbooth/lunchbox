//! First-launch discovery for the native Mesen2 Linux frontend. Synthesizes a
//! launch-scoped saved setup instead of failing for a missing hand-written
//! entry (mgba/gopher64 precedent). Settings review never calls this.
use super::settings::SavedSetup;
use super::{SYSTEM_NES, SYSTEM_PCE};
use crate::{
    controller_native_process::cancelled,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Context, Result, ensure};
use std::{
    path::{Component, PathBuf},
    sync::atomic::AtomicBool,
};

/// Probe helper: explicit override for integration tests, else the sibling
/// controller-probe binary beside the Lunchbox executable.
fn helper() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("LUNCHBOX_CONTROLLER_PROBE") {
        let path = PathBuf::from(path);
        ensure!(
            path.is_file(),
            "LUNCHBOX_CONTROLLER_PROBE does not name a file"
        );
        return Ok(path);
    }
    let path = std::env::current_exe()?
        .parent()
        .context("Missing Lunchbox application directory")?
        .join("lunchbox-controller-probe");
    ensure!(
        path.is_file(),
        "This Lunchbox installation is missing lunchbox-controller-probe; install the full package"
    );
    Ok(path)
}

/// `target_layout` selects the system contract: the NES pad or the PC
/// Engine/TurboGrafx pad. Anything else has no authored writer yet.
pub(crate) fn system_for_layout(target_layout: &str) -> Result<&'static str> {
    match target_layout {
        "nes" => Ok(SYSTEM_NES),
        "pce-2" => Ok(SYSTEM_PCE),
        other => {
            anyhow::bail!("Mesen2 has no native controller writer for target layout {other} yet")
        }
    }
}

pub(crate) fn discover(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
    target_layout: &str,
    controller_id: &str,
    cancel: &AtomicBool,
) -> Result<SavedSetup> {
    cancelled(cancel)?;
    ensure!(
        !controller_id.trim().is_empty(),
        "Choose a controller for this player in Controller setup first"
    );
    let system = system_for_layout(target_layout)?;
    let EmulatorExecutable::Native(program) = &option.executable else {
        anyhow::bail!("Mesen2 automatic discovery supports the native Linux frontend only")
    };
    ensure!(
        plan.environment.is_empty() && plan.retroarch_content.is_none(),
        "Custom Mesen2 launcher environments need explicit runtime resolution"
    );
    ensure!(
        program == &plan.program,
        "Custom Mesen2 launcher environments need explicit runtime resolution"
    );
    ensure!(
        plan.arguments.len() == 1,
        "Mesen2 guided launch needs exactly the ROM argument"
    );
    let content = PathBuf::from(&plan.arguments[0]);
    ensure!(
        content.is_absolute()
            && !content
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            && content.is_file(),
        "Mesen2 needs the selected ROM's absolute file path"
    );
    let setup = SavedSetup {
        emulator_id: option.emulator_id.clone(),
        content: content.canonicalize()?,
        controller_id: controller_id.to_owned(),
        probe_program: helper()?,
        executable_sha256: lunchbox_controller_probe::file_hash(program)?,
        system: system.to_owned(),
    };
    setup.validate()?;
    cancelled(cancel)?;
    Ok(setup)
}
