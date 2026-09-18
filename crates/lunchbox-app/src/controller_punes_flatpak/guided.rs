//! First-launch discovery for the audited puNES 0.111 Flatpak. Synthesizes a
//! launch-scoped saved setup instead of failing for a missing hand-written
//! entry (nestopia/mgba precedent). Settings review never calls this.

use super::{
    flatpak,
    settings::{Player, SavedSetup},
};
use crate::{
    controller_native_process::{cancelled, capture},
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    platform_process::host_command,
};
use anyhow::{Context, Result, ensure};
use std::{
    path::{Component, PathBuf},
    sync::atomic::AtomicBool,
};

fn flatpak_output(command: &std::path::Path, args: &[&str]) -> Result<String> {
    let mut process = host_command(command);
    process.args(args);
    let (output, _) = capture(&mut process, &AtomicBool::new(false))?;
    String::from_utf8(output)
        .context("puNES Flatpak info was not UTF-8")
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
    ids: &[String],
    cancel: &AtomicBool,
) -> Result<SavedSetup> {
    cancelled(cancel)?;
    ensure!(
        matches!(ids.len(), 1 | 2),
        "puNES supports one or two players; assign players in Controller setup"
    );
    let EmulatorExecutable::Flatpak { command, app_id } = &option.executable else {
        anyhow::bail!("puNES automatic discovery supports only its audited Flatpak")
    };
    ensure!(
        app_id == flatpak::APP_ID,
        "puNES adapter supports only the audited Flathub application"
    );
    ensure!(
        plan.environment.is_empty() && plan.retroarch_content.is_none(),
        "Custom puNES launcher environments need explicit runtime resolution"
    );
    ensure!(
        plan.arguments.len() == 4
            && plan.arguments[0] == "run"
            && plan.arguments[2] == flatpak::APP_ID,
        "puNES Flatpak launch requires the ordinary one-ROM argument plan"
    );
    let grant = plan.arguments[1]
        .to_str()
        .and_then(|value| value.strip_prefix("--filesystem="))
        .context("puNES Flatpak launch is missing its ROM-directory grant")?;
    ensure!(
        !grant.contains(':')
            && PathBuf::from(grant).canonicalize()? == plan.current_directory.canonicalize()?,
        "puNES Flatpak launch grants a directory other than its exact launch directory"
    );
    let content = PathBuf::from(&plan.arguments[3]);
    ensure!(
        content.is_absolute()
            && !content
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            && content.is_file(),
        "puNES needs the selected ROM's absolute file path"
    );
    ensure!(
        content
            .parent()
            .is_some_and(|parent| parent.canonicalize().ok().as_ref()
                == plan.current_directory.canonicalize().ok().as_ref()),
        "puNES ROM must sit directly in its launch directory"
    );
    let config_dir = directories::BaseDirs::new()
        .context("Finding the puNES Flatpak profile")?
        .home_dir()
        .join(".var/app")
        .join(flatpak::APP_ID)
        .join("config/puNES");
    let source_main_config = config_dir.join("puNES.cfg");
    let source_input_config = config_dir.join("input.cfg");
    ensure!(
        source_main_config.is_file() && source_input_config.is_file(),
        "Open puNES once to create puNES.cfg and input.cfg, then launch through Lunchbox again"
    );
    // Fail fast on an unaudited deployment instead of writing a setup whose
    // launch verification would reject it.
    let commit = flatpak_output(command, &["info", "--show-commit", flatpak::APP_ID])?;
    ensure!(
        commit == flatpak::APP_COMMIT,
        "puNES Flatpak application is outside the audited deployment"
    );
    let setup = SavedSetup {
        emulator_id: option.emulator_id.clone(),
        content: content.canonicalize()?,
        source_main_config,
        source_input_config,
        probe_program: helper()?,
        executable_sha256: flatpak::APP_EXECUTABLE_SHA256.into(),
        players: ids
            .iter()
            .enumerate()
            .map(|(index, id)| Player {
                player: u8::try_from(index + 1).unwrap(),
                controller_id: id.clone(),
            })
            .collect(),
    };
    setup.validate()?;
    cancelled(cancel)?;
    Ok(setup)
}
