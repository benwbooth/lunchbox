//! First-launch discovery for the DuckStation Flatpak. Synthesizes a
//! launch-scoped saved setup instead of failing for a missing hand-written
//! entry (mgba/gopher64 precedent). Settings review never calls this.

use super::{
    SavedPlayer, SavedSetup,
    flatpak::{companion_libraries, deployment, profile_data_root},
};
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

pub(crate) fn discover(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
    analog: bool,
    ids: &[String],
    cancel: &AtomicBool,
) -> Result<SavedSetup> {
    cancelled(cancel)?;
    ensure!(
        (1..=2).contains(&ids.len()),
        "DuckStation guided play supports one or two players; assign players in Controller setup"
    );
    let EmulatorExecutable::Flatpak { command, app_id } = &option.executable else {
        anyhow::bail!("DuckStation automatic discovery supports only its audited Flatpak")
    };
    ensure!(
        app_id == super::flatpak::APP_ID,
        "DuckStation adapter supports only the audited Flathub application"
    );
    ensure!(
        plan.environment.is_empty() && plan.retroarch_content.is_none(),
        "Custom DuckStation launcher environments need explicit runtime resolution"
    );
    // Same ordinary one-ROM shape the launch preparation parses: skip the
    // `run` prefix and sandbox grants up to the app id, then take the single
    // absolute game argument.
    let mut content: Option<PathBuf> = None;
    let mut after_app = false;
    for argument in &plan.arguments {
        if !after_app {
            if argument == super::flatpak::APP_ID {
                after_app = true;
            }
            continue;
        }
        let path = PathBuf::from(argument);
        if !path.is_absolute() {
            continue;
        }
        ensure!(
            content.is_none(),
            "DuckStation Flatpak launch needs exactly one game argument"
        );
        content = Some(path);
    }
    let content = content.context("DuckStation Flatpak launch needs exactly one game argument")?;
    ensure!(
        content.is_file()
            && !content
                .components()
                .any(|part| matches!(part, Component::ParentDir)),
        "DuckStation needs the selected ROM's absolute file path"
    );
    ensure!(
        content
            .parent()
            .is_some_and(|parent| parent.canonicalize().ok().as_ref()
                == plan.current_directory.canonicalize().ok().as_ref()),
        "DuckStation ROM must sit directly in its launch directory"
    );
    // The settings layer follows the pressed disc, never a title guess. A
    // missing or unreadable serial fails here instead of at launch.
    let serial = crate::controller_psx::single_disc_serial(&content)?;
    let live_root = profile_data_root()?;
    let deployment = deployment(command, cancel)?;
    let companions = companion_libraries(&deployment, command, cancel)?;
    let kind = if analog {
        "AnalogController"
    } else {
        "DigitalController"
    };
    let setup = SavedSetup {
        apply_selected_ports: true,
        runtime: Some(super::NativeRuntime {
            probe_program: helper()?,
            sdl_library: deployment.sdl_library.clone(),
            runtime_libraries: std::iter::once(deployment.sdl_library.clone())
                .chain(companions)
                .collect(),
            executable_sha256: lunchbox_controller_probe::file_hash(&deployment.executable)?,
        }),
        emulator_id: option.emulator_id.clone(),
        content: content.canonicalize()?,
        data_root: live_root,
        serial: serial.clone(),
        // The set head defaults to the launched disc; stage() treats this
        // identically to a confirmed single disc while per-set settings are
        // absent, and DuckStation resolves true set membership from its own
        // game database at runtime.
        first_disc_serial: Some(serial),
        players: ids
            .iter()
            .enumerate()
            .map(|(index, id)| SavedPlayer {
                pad: u8::try_from(index + 1).unwrap(),
                controller_id: id.clone(),
                controller_type: kind.into(),
            })
            .collect(),
    };
    setup.validate()?;
    cancelled(cancel)?;
    Ok(setup)
}
