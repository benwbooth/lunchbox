//! First-launch discovery for standalone MAME Flatpak. Synthesizes a
//! launch-scoped saved setup instead of failing for a missing hand-written
//! entry (mgba/gopher64 precedent). Settings review never calls this.

use super::{
    Panel, flatpak,
    settings::{Runtime, SavedPlayer, SavedSetup},
};
use crate::{
    controller_catalog::{Calibration, EmulatorProfile},
    controller_guided_native::arcade_sources,
    controller_native_process::{cancelled, capture},
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    platform_process::host_command,
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn flatpak_output(command: &Path, args: &[&str]) -> Result<String> {
    let mut process = host_command(command);
    process.args(args);
    let (output, _) = capture(&mut process, &AtomicBool::new(false))?;
    String::from_utf8(output)
        .context("MAME Flatpak info was not UTF-8")
        .map(|value| value.trim().to_owned())
}

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
    panel: Panel,
    profile: &EmulatorProfile,
    ids: &[String],
    calibrations: &HashMap<String, Calibration>,
    cancel: &AtomicBool,
) -> Result<SavedSetup> {
    cancelled(cancel)?;
    ensure!(
        !ids.is_empty() && ids.len() <= 8,
        "MAME arcade panels support one to eight players; assign players in Controller setup"
    );
    let EmulatorExecutable::Flatpak { command, app_id } = &option.executable else {
        anyhow::bail!("MAME automatic discovery supports only its audited Flatpak")
    };
    ensure!(
        app_id == flatpak::APP_ID,
        "MAME adapter supports only the exact Flathub application"
    );
    ensure!(
        plan.environment.is_empty() && plan.retroarch_content.is_none(),
        "Custom MAME launcher environments need explicit runtime resolution"
    );
    // Builder shape: run, grants, app, -rompath, dir, romset.
    let app_position = plan
        .arguments
        .iter()
        .position(|argument| argument == flatpak::APP_ID)
        .context("MAME Flatpak launch is missing its app id")?;
    ensure!(
        plan.arguments.len() == app_position + 4 && plan.arguments[app_position + 1] == "-rompath",
        "MAME Flatpak launch requires the builder `-rompath <dir> <romset>` plan"
    );
    let location = flatpak_output(command, &["info", "--show-location", flatpak::APP_ID])?;
    let executable = PathBuf::from(location)
        .join("files")
        .join("bin/mame")
        .canonicalize()?;
    ensure!(
        executable.is_file(),
        "MAME Flatpak executable could not be located"
    );
    let runtime_ref = flatpak_output(command, &["info", "--show-runtime", flatpak::APP_ID])?;
    let runtime_location = flatpak_output(command, &["info", "--show-location", &runtime_ref])?;
    let sdl_library = PathBuf::from(runtime_location)
        .join("files")
        .join("lib")
        .join(format!("{}-linux-gnu", std::env::consts::ARCH))
        .join("libSDL2-2.0.so.0")
        .canonicalize()?;
    ensure!(
        sdl_library.is_file(),
        "MAME Flatpak SDL2 library could not be located"
    );
    let home = directories::BaseDirs::new()
        .context("Finding the MAME Flatpak profile")?
        .home_dir()
        .to_path_buf();
    let players: Vec<SavedPlayer> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let calibration = calibrations
                .get(id)
                .context("Finish recording this controller first")?;
            Ok(SavedPlayer {
                player: u8::try_from(index + 1).unwrap(),
                controller_id: id.clone(),
                panel,
                native_device_id: String::new(),
                controls: BTreeMap::new(),
                source_controls: arcade_sources(calibration, profile)?,
            })
        })
        .collect::<Result<_>>()?;
    let setup = SavedSetup {
        emulator_id: option.emulator_id.clone(),
        resolve_inputs_at_launch: true,
        executable: executable.clone(),
        executable_sha256: lunchbox_controller_probe::file_hash(&executable)?,
        joystick_provider: "sdl".into(),
        threshold_basis_points: 3000,
        cfg_directory: Some(
            home.join(".var/app")
                .join(flatpak::APP_ID)
                .join(".mame/cfg"),
        ),
        runtime: Some(Runtime {
            probe_program: helper()?,
            sdl_library,
        }),
        players,
    };
    setup.validate()?;
    cancelled(cancel)?;
    Ok(setup)
}
