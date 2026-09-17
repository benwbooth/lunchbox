//! Flatpak launch preparation for standalone MAME (Flathub
//! `org.mamedev.MAME`).
//!
//! SDL inventory and the sysfs topology resolve on the host exactly like the
//! native flow (with a search path for sdl2-compat's SDL3). The staged
//! controller profile and filtered cfg copies cannot stay under the system
//! tempdir (each sandbox has a private `/tmp`), so preparation copies them
//! into a home-backed staging directory, grants that directory to the
//! sandbox, and rewrites `-ctrlrpath`/`-cfg_directory` to the
//! sandbox-visible copies. The live cfg directory is only read, never
//! written: MAME's own exit-time saves land in the discarded staging copy.

use super::{
    config_copy::PreparedConfigs, configuration::PreparedProfile, prepared::resolve_guided,
    session::InputSession, settings::SavedSetup,
};
use crate::{
    controller_native_process::{cancelled, capture, sandbox_library_path},
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    platform_process::host_command,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsString,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Child,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub(crate) const APP_ID: &str = "org.mamedev.MAME";
const APP_EXECUTABLE: &str = "bin/mame";

fn flatpak_output(command: &Path, args: &[&str], cancel: &AtomicBool) -> Result<String> {
    let mut process = host_command(command);
    process.args(args);
    let (output, _) = capture(&mut process, cancel)?;
    String::from_utf8(output)
        .context("MAME Flatpak info was not UTF-8")
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
/// Trust is live: saved hashes must match what is installed.
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
        "MAME Flatpak executable could not be located"
    );
    let runtime_ref = flatpak_output(command, &["info", "--show-runtime", APP_ID], cancel)?;
    let mut parts = runtime_ref.split('/');
    ensure!(
        parts
            .next()
            .is_some_and(|p| p == "org.freedesktop.Platform" || p == "org.kde.Platform")
            && parts.next() == Some(std::env::consts::ARCH),
        "MAME Flatpak runtime is outside the supported Linux contracts"
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
        "MAME Flatpak SDL2 library could not be located"
    );
    Ok(Deployment {
        executable,
        sdl_library,
        runtime_libdir,
    })
}

/// Parse the builder `-rompath <dir> <romset>` Flatpak plan. Every
/// `--filesystem` grant and every `-rompath` component must resolve inside
/// the exact launch directory set: no wider sandbox access, no unmounted
/// ROM roots. Returns the machine basename.
fn parse_original(setup: &SavedSetup, original: &LaunchPlan, command: &Path) -> Result<String> {
    ensure!(
        original.environment.is_empty() && original.retroarch_content.is_none(),
        "MAME Flatpak launch requires the builder machine plan"
    );
    ensure!(
        same_program(command, &original.program),
        "MAME Flatpak command differs from selection"
    );
    let arguments = &original.arguments;
    ensure!(
        arguments.first().is_some_and(|arg| arg == "run"),
        "MAME Flatpak launch requires the builder machine plan"
    );
    let app_position = arguments
        .iter()
        .position(|argument| argument == APP_ID)
        .context("MAME Flatpak launch is missing its app id")?;
    let mut grants = Vec::new();
    for option in &arguments[1..app_position] {
        if let Some(grant) = option
            .to_str()
            .and_then(|value| value.strip_prefix("--filesystem="))
        {
            ensure!(!grant.contains(':'), "MAME Flatpak grant is malformed");
            grants.push(Path::new(grant).canonicalize()?);
        } else {
            anyhow::bail!(
                "Custom MAME Flatpak sandbox option needs input/configuration routing resolution"
            );
        }
    }
    let current = original.current_directory.canonicalize()?;
    ensure!(
        !grants.is_empty() && grants.iter().all(|grant| grant == &current),
        "MAME Flatpak launch grants a directory other than its exact launch directory"
    );
    ensure!(
        arguments.len() == app_position + 4 && arguments[app_position + 1] == "-rompath",
        "MAME Flatpak launch requires the builder `-rompath <dir> <romset>` plan"
    );
    for component in arguments[app_position + 2]
        .to_str()
        .context("MAME rompath must be UTF-8")?
        .split(';')
    {
        ensure!(
            Path::new(component).canonicalize()? == current,
            "MAME rompath reaches outside its granted launch directory"
        );
    }
    let machine = arguments[app_position + 3]
        .to_str()
        .context("MAME machine argument must be UTF-8")?;
    ensure!(
        !machine.is_empty()
            && machine != "default"
            && machine
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "MAME configuration needs an exact machine basename"
    );
    Ok(machine.to_owned())
}

fn staging_root() -> Result<PathBuf> {
    let home = directories::BaseDirs::new()
        .context("Finding the user home for MAME controller staging")?
        .home_dir()
        .to_path_buf();
    let cache = home.join(".cache/lunchbox/controller-launch");
    std::fs::create_dir_all(&cache)?;
    Ok(cache)
}

pub(crate) struct FlatpakSession {
    inputs: InputSession,
    pub(crate) plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
    profile: PreparedProfile,
    configs: PreparedConfigs,
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
        anyhow::bail!("MAME Flatpak preparation needs a Flatpak runtime selection")
    };
    ensure!(
        app_id == APP_ID && setup.emulator_id == option.emulator_id,
        "MAME Flatpak adapter only supports the exact Flathub application"
    );
    let machine = parse_original(setup, plan, command)?;
    let runtime = setup
        .runtime
        .as_ref()
        .context("MAME setup needs explicit probe_program and sdl_library runtime paths")?;
    let deployment = deployment(command, cancel)?;
    ensure!(
        setup.executable.canonicalize()? == deployment.executable.canonicalize()?
            && file_hash(&deployment.executable)? == setup.executable_sha256,
        "MAME Flatpak executable differs from the saved trusted runtime"
    );
    ensure!(
        setup
            .cfg_directory
            .as_ref()
            .is_some_and(|dir| dir.is_absolute()),
        "MAME Flatpak setup needs an absolute source cfg directory"
    );
    ensure!(
        runtime.sdl_library.canonicalize()? == deployment.sdl_library.canonicalize()?,
        "MAME saved SDL differs from the active target Flatpak runtime"
    );
    let library_path = sandbox_library_path(
        &runtime.probe_program,
        &deployment.runtime_libdir.to_string_lossy(),
    );
    let inputs = InputSession::capture(setup, inventory, Some(&library_path), cancel)?;
    let resolved = resolve_guided(setup, calibrations, &inputs.physical_paths, &inputs.devices)?;
    let source_cfg = resolved
        .cfg_directory
        .as_ref()
        .context("MAME Flatpak setup needs a source cfg directory")?;
    let xml = super::prepared::controller_xml(
        &resolved,
        calibrations,
        &inputs.physical_paths,
        &inputs.devices,
    )?;
    let profile = PreparedProfile::create(&resolved, xml)?;
    let configs = PreparedConfigs::create(&resolved, source_cfg, &machine)?;
    // The system tempdirs are invisible inside the sandbox; serve home-backed
    // copies of the staged controller profile and cfg files instead.
    let staging = tempfile::Builder::new()
        .prefix("mame-flatpak-cfg-")
        .tempdir_in(staging_root()?)?;
    std::fs::set_permissions(staging.path(), std::fs::Permissions::from_mode(0o700))?;
    let mut staged_paths = Vec::new();
    let staged_panel = staging.path().join("lunchbox-panel.cfg");
    std::fs::copy(profile.path(), &staged_panel)?;
    std::fs::set_permissions(&staged_panel, std::fs::Permissions::from_mode(0o600))?;
    staged_paths.push(staged_panel.clone());
    for (source, bytes) in configs.copies() {
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .context("MAME staged cfg copy has no file name")?;
        // Fail closed on unexpected staged files: only the two flat cfg
        // copies belong in the sandbox-visible staging directory.
        ensure!(
            name == "default.cfg" || name == format!("{machine}.cfg"),
            "Unexpected MAME staged cfg file"
        );
        let staged = staging.path().join(name);
        std::fs::write(&staged, bytes)?;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o600))?;
        staged_paths.push(staged);
    }
    let mut files = BTreeMap::new();
    for path in [
        &runtime.probe_program,
        &runtime.sdl_library,
        &deployment.executable,
    ] {
        files.insert(path.clone(), file_hash(path)?);
    }
    for path in &staged_paths {
        files.insert(path.clone(), file_hash(path)?);
    }
    // The ROM set itself is a launch input: hash the resolved archive.
    let rom = plan.current_directory.join(format!("{machine}.zip"));
    if rom.is_file() {
        files.insert(rom.clone(), file_hash(&rom)?);
    }
    let mut configured = plan.clone();
    let app_position = configured
        .arguments
        .iter()
        .position(|argument| argument == APP_ID)
        .context("MAME Flatpak plan lost its app id")?;
    configured.arguments.insert(
        app_position,
        OsString::from(format!("--filesystem={}", staging.path().display())),
    );
    // Re-home the prepared directory arguments from their system tempdirs to
    // the sandbox-visible staging copies.
    let profile_dir = profile.directory().canonicalize()?;
    let configs_dir = configs.directory().canonicalize()?;
    for argument in profile.arguments().into_iter().chain(configs.arguments()) {
        let argument = rewrite_staging_argument(&argument, &profile_dir, &configs_dir, &staging)?;
        configured.arguments.push(argument);
    }
    let session = FlatpakSession {
        inputs,
        plan: configured.clone(),
        files,
        profile,
        configs,
        _staging: staging,
    };
    session.verify(cancel)?;
    *plan = configured;
    Ok(session)
}

/// Rewrite a prepared `-ctrlrpath`/`-cfg_directory` argument from its system
/// tempdir to the sandbox-visible staging copy. Every other prepared
/// argument passes through byte-identical.
fn rewrite_staging_argument(
    argument: &OsString,
    profile_dir: &Path,
    configs_dir: &Path,
    staging: &tempfile::TempDir,
) -> Result<OsString> {
    let path = Path::new(argument);
    for dir in [profile_dir, configs_dir] {
        if path.canonicalize().ok().as_ref() == Some(&dir.to_path_buf()) {
            return Ok(staging.path().as_os_str().to_owned());
        }
    }
    Ok(argument.clone())
}

impl FlatpakSession {
    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "MAME Flatpak launch input changed: {}",
                path.display()
            );
        }
        self.inputs.verify(cancel)?;
        self.profile.verify()?;
        self.configs.verify_before_launch()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.inputs.check_health()
    }

    pub(crate) fn spawn(&mut self, plan: &LaunchPlan, cancel: &AtomicBool) -> Result<Child> {
        ensure!(
            &self.plan == plan,
            "MAME Flatpak launch plan changed after input preparation"
        );
        self.verify(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan(plan)?;
        // The sandbox emulator binary is not visible on the host, so the
        // native /proc tree walk cannot identify it. Physical routing
        // re-verification plus liveness carry the handoff proof instead.
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            cancelled(cancel)?;
            ensure!(
                child.try_wait()?.is_none(),
                "MAME exited before controller handoff"
            );
            if self.inputs.verify(cancel).is_ok() {
                return Ok(child);
            }
            ensure!(
                std::time::Instant::now() < deadline,
                "MAME did not establish SDL controller ownership before timeout"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
