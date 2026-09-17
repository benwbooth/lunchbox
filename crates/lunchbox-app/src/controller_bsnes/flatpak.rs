//! Flatpak launch preparation for bsnes v115+ (Flathub `dev.bsnes.bsnes`).
//!
//! The target SDL2 library lives in the application's Freedesktop runtime
//! and is host-readable, so the controller probe runs on the host exactly
//! like the native flow (with a search path for sdl2-compat's SDL3). The
//! staged private `settings.bml` cannot stay under the system tempdir (each
//! sandbox has a private `/tmp`), so preparation copies it into a
//! home-backed staging directory, grants that directory to the sandbox, and
//! passes `--settings` pointing at the sandbox-visible copy. Saves keep
//! bsnes's own defaults inside the application sandbox.

use super::{session::PreparedSession, settings::SavedSetup};
use crate::{
    controller_native_process::{cancelled, capture, sandbox_library_path},
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    platform_process::host_command,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{
    collections::{BTreeMap, HashMap},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Child,
    sync::atomic::AtomicBool,
};

pub(crate) const APP_ID: &str = "dev.bsnes.bsnes";
const APP_EXECUTABLE: &str = "bin/bsnes";

fn flatpak_output(command: &Path, args: &[&str], cancel: &AtomicBool) -> Result<String> {
    let mut process = host_command(command);
    process.args(args);
    let (output, _) = capture(&mut process, cancel)?;
    String::from_utf8(output)
        .context("bsnes Flatpak info was not UTF-8")
        .map(|value| value.trim().to_owned())
}

fn same_program(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

/// Application executable and runtime SDL2 from the live deployment.
/// Trust is live (snes9x-style): saved hashes must match what is installed.
struct Deployment {
    executable: PathBuf,
    sdl_library: PathBuf,
    runtime_libdir: PathBuf,
}

fn deployment(command: &Path, cancel: &AtomicBool) -> Result<Deployment> {
    let location = flatpak_output(command, &["info", "--show-location", APP_ID], cancel)?;
    let executable = PathBuf::from(location)
        .join("files")
        .join(APP_EXECUTABLE)
        .canonicalize()?;
    ensure!(
        executable.is_file(),
        "bsnes Flatpak executable could not be located"
    );
    let runtime_ref = flatpak_output(command, &["info", "--show-runtime", APP_ID], cancel)?;
    let mut parts = runtime_ref.split('/');
    ensure!(
        parts.next() == Some("org.freedesktop.Platform")
            && parts.next() == Some(std::env::consts::ARCH),
        "bsnes Flatpak runtime is outside the supported Freedesktop contract"
    );
    let runtime_location =
        flatpak_output(command, &["info", "--show-location", &runtime_ref], cancel)?;
    let runtime_libdir = PathBuf::from(runtime_location)
        .join("files")
        .join("lib")
        .join(format!("{}-linux-gnu", std::env::consts::ARCH));
    let sdl_library = runtime_libdir.join("libSDL2-2.0.so.0").canonicalize()?;
    ensure!(
        sdl_library.is_file(),
        "bsnes Flatpak SDL2 library could not be located"
    );
    Ok(Deployment {
        executable,
        sdl_library,
        runtime_libdir,
    })
}

/// Parse the ordinary one-ROM Flatpak argument plan: `run` plus grants for
/// the exact launch directory, the app id, then the saved game. Preparation
/// later adds the settings staging grant and `--settings` itself.
fn parse_original<'a>(
    setup: &SavedSetup,
    original: &'a LaunchPlan,
    command: &Path,
) -> Result<&'a Path> {
    ensure!(
        original.environment.is_empty() && original.retroarch_content.is_none(),
        "bsnes Flatpak launch requires the ordinary one-ROM argument plan"
    );
    ensure!(
        same_program(command, &original.program),
        "bsnes Flatpak command differs from selection"
    );
    let arguments = &original.arguments;
    ensure!(
        arguments.first().is_some_and(|arg| arg == "run"),
        "bsnes Flatpak launch requires the ordinary one-ROM argument plan"
    );
    let app_position = arguments
        .iter()
        .position(|argument| argument == APP_ID)
        .context("bsnes Flatpak launch is missing its app id")?;
    ensure!(
        app_position >= 2,
        "bsnes Flatpak launch has no sandbox grants"
    );
    for option in &arguments[1..app_position] {
        if let Some(grant) = option
            .to_str()
            .and_then(|value| value.strip_prefix("--filesystem="))
        {
            ensure!(
                !grant.contains(':')
                    && Path::new(grant).canonicalize()?
                        == original.current_directory.canonicalize()?,
                "bsnes Flatpak launch grants a directory other than its exact launch directory"
            );
        } else {
            anyhow::bail!(
                "Custom bsnes Flatpak sandbox option needs input/configuration routing resolution"
            );
        }
    }
    ensure!(
        arguments.len() == app_position + 2,
        "bsnes Flatpak launch requires exactly the saved game argument"
    );
    let content = Path::new(&arguments[app_position + 1]);
    ensure!(
        content.canonicalize()? == setup.content.canonicalize()?,
        "bsnes Flatpak launch does not select the saved ROM exactly once"
    );
    ensure!(
        content
            .parent()
            .is_some_and(|parent| parent.canonicalize().ok().as_ref()
                == original.current_directory.canonicalize().ok().as_ref()),
        "bsnes ROM must sit directly in its launch directory"
    );
    Ok(content)
}

fn staging_root() -> Result<PathBuf> {
    let home = directories::BaseDirs::new()
        .context("Finding the user home for bsnes controller staging")?
        .home_dir()
        .to_path_buf();
    let cache = home.join(".cache/lunchbox/controller-launch");
    std::fs::create_dir_all(&cache)?;
    Ok(cache)
}

pub(crate) struct FlatpakSession {
    inputs: PreparedSession,
    pub(crate) plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
    _staging: tempfile::TempDir,
}

pub(crate) fn prepare(
    setup: &SavedSetup,
    calibrations: &HashMap<String, crate::controller_catalog::Calibration>,
    inventory: &[crate::controllers::ControllerDevice],
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    cancel: &AtomicBool,
) -> Result<FlatpakSession> {
    cancelled(cancel)?;
    setup.validate()?;
    let EmulatorExecutable::Flatpak { command, app_id } = &option.executable else {
        anyhow::bail!("bsnes Flatpak preparation needs a Flatpak runtime selection")
    };
    ensure!(
        app_id == APP_ID && setup.emulator_id == option.emulator_id,
        "bsnes Flatpak adapter only supports the exact Flathub application"
    );
    let content = parse_original(setup, plan, command)?.to_path_buf();
    let deployment = deployment(command, cancel)?;
    ensure!(
        file_hash(&deployment.executable)? == setup.executable_sha256,
        "bsnes Flatpak executable differs from the saved trusted runtime"
    );
    ensure!(
        setup.sdl_library.canonicalize()? == deployment.sdl_library.canonicalize()?,
        "bsnes saved SDL differs from the active target Flatpak runtime"
    );
    // sdl2-compat resolves SDL3 with a bare dlopen; the host probe needs the
    // runtime library directory behind the probe's own loader closure.
    let library_path = sandbox_library_path(
        &setup.probe_program,
        &deployment.runtime_libdir.to_string_lossy(),
    );
    let inputs =
        PreparedSession::prepare(setup, calibrations, inventory, Some(&library_path), cancel)?;
    // The system tempdir is invisible inside the sandbox; copy the staged
    // settings into a home-backed directory and grant exactly that.
    let staging = tempfile::Builder::new()
        .prefix("bsnes-flatpak-settings-")
        .tempdir_in(staging_root()?)?;
    std::fs::set_permissions(staging.path(), std::fs::Permissions::from_mode(0o700))?;
    let staged_copy = staging.path().join("settings.bml");
    std::fs::copy(&inputs.settings_path, &staged_copy)?;
    std::fs::set_permissions(&staged_copy, std::fs::Permissions::from_mode(0o600))?;
    ensure!(
        file_hash(&staged_copy)? == file_hash(&inputs.settings_path)?,
        "Staged bsnes settings copy differs from its source"
    );
    let mut files = BTreeMap::new();
    for path in [
        &content,
        &setup.probe_program,
        &setup.sdl_library,
        &deployment.executable,
        &staged_copy,
    ] {
        files.insert(path.clone(), file_hash(path)?);
    }
    let mut configured = plan.clone();
    let app_position = configured
        .arguments
        .iter()
        .position(|argument| argument == APP_ID)
        .context("bsnes Flatpak plan lost its app id")?;
    configured.arguments.insert(
        app_position,
        format!("--filesystem={}", staging.path().display()).into(),
    );
    let game_position = configured
        .arguments
        .iter()
        .position(|argument| argument == content.as_os_str())
        .context("bsnes Flatpak plan lost its game argument")?;
    configured.arguments.insert(
        game_position,
        format!("--settings={}", staged_copy.display()).into(),
    );
    let session = FlatpakSession {
        inputs,
        plan: configured.clone(),
        files,
        _staging: staging,
    };
    session.verify(cancel)?;
    *plan = configured;
    Ok(session)
}

impl FlatpakSession {
    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "bsnes Flatpak launch input changed: {}",
                path.display()
            );
        }
        self.inputs.verify(cancel)
    }

    pub(crate) fn spawn(&mut self, plan: &LaunchPlan, cancel: &AtomicBool) -> Result<Child> {
        ensure!(
            &self.plan == plan,
            "bsnes Flatpak launch plan changed after input preparation"
        );
        self.verify(cancel)?;
        crate::emulator::spawn_launch_plan(plan)
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.inputs.check_health()
    }
}
