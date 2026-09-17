//! Flatpak launch preparation for the pinned DuckStation revision
//! (`0.1-9482-g0a53bc47c`, the same input contract as native).
//!
//! The target SDL3 library ships inside the application (`files/bin`) and is
//! host-readable, so the controller probe runs on the host exactly like the
//! native flow. Configuration delivery differs: Flatpak remaps
//! XDG_CONFIG_HOME inside the sandbox (verified: `--env` is ignored), so a
//! private staged tree can never reach the app. The session instead swaps the
//! staged documents over the live configuration with backup/restore and a
//! stale-proof lock (Gopher64 precedent), then restores on drop. Saves,
//! memory cards and BIOS stay live: only the staged settings, per-game and
//! input-profile documents move.

use super::{SavedSetup, prepare_resolved};
use crate::{
    controller_native_platform::child_exe_matches,
    controller_native_process::{cancelled, capture},
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    platform_process::host_command,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{duckstation_config::LaunchConfig, file_hash};
use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub(crate) const APP_ID: &str = "org.duckstation.DuckStation";
const APP_EXECUTABLE: &str = "bin/duckstation-qt";
const BUNDLED_SDL: &str = "bin/libSDL3.so.0";
const REVISION: &str = "0.1-9482-g0a53bc47c";
const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;

fn same_program(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

fn flatpak_output(command: &Path, args: &[&str], cancel: &AtomicBool) -> Result<String> {
    let mut process = host_command(command);
    process.args(args);
    let (output, _) = capture(&mut process, cancel)?;
    String::from_utf8(output)
        .context("DuckStation Flatpak info was not UTF-8")
        .map(|value| value.trim().to_owned())
}

/// Application files, executable and bundled SDL3 from the live deployment.
/// The executable hash is the trust anchor (live trust: the saved hash must
/// match the installed application).
pub(crate) struct Deployment {
    pub(crate) executable: PathBuf,
    pub(crate) sdl_library: PathBuf,
    pub(crate) runtime_ref: String,
}

pub(crate) fn deployment(command: &Path, cancel: &AtomicBool) -> Result<Deployment> {
    let location = flatpak_output(command, &["info", "--show-location", APP_ID], cancel)?;
    let app_files = PathBuf::from(location).join("files").canonicalize()?;
    let executable = app_files.join(APP_EXECUTABLE).canonicalize()?;
    ensure!(
        executable.is_file(),
        "DuckStation Flatpak executable could not be located"
    );
    let sdl_library = app_files.join(BUNDLED_SDL).canonicalize()?;
    ensure!(
        sdl_library.is_file(),
        "DuckStation Flatpak SDL3 library could not be located"
    );
    let runtime_ref = flatpak_output(command, &["info", "--show-runtime", APP_ID], cancel)?;
    Ok(Deployment {
        executable,
        sdl_library,
        runtime_ref,
    })
}

/// Companion runtime libraries the host probe loads alongside SDL3, in load
/// order (libudev for device topology, libcap beneath it: the loader must see
/// libcap first to satisfy libudev's own dependency). Resolved from the
/// application's own runtime so the probe observes the target stack.
pub(crate) fn companion_libraries(
    deployment: &Deployment,
    command: &Path,
    cancel: &AtomicBool,
) -> Result<Vec<PathBuf>> {
    let mut parts = deployment.runtime_ref.split('/');
    let (platform, arch) = (parts.next(), parts.next());
    ensure!(
        platform.is_some_and(|p| p == "org.freedesktop.Platform" || p == "org.kde.Platform")
            && arch == Some(std::env::consts::ARCH),
        "DuckStation Flatpak runtime is outside the supported Linux contracts"
    );
    let location = flatpak_output(
        command,
        &["info", "--show-location", &deployment.runtime_ref],
        cancel,
    )?;
    let libdir = PathBuf::from(location)
        .join("files")
        .join("lib")
        .join(format!("{}-linux-gnu", std::env::consts::ARCH));
    let mut libraries = Vec::new();
    for name in ["libcap.so.2", "libudev.so.1"] {
        let path = libdir.join(name).canonicalize().with_context(|| {
            format!("DuckStation Flatpak runtime library {name} could not be located")
        })?;
        libraries.push(path);
    }
    Ok(libraries)
}

pub(crate) fn profile_data_root() -> Result<PathBuf> {
    let root = directories::BaseDirs::new()
        .context("Finding the DuckStation Flatpak profile")?
        .home_dir()
        .join(".var/app")
        .join(APP_ID)
        .join("config/duckstation");
    ensure!(
        root.join("settings.ini").is_file(),
        "Open DuckStation once to create its configuration, then launch through Lunchbox again"
    );
    Ok(root)
}

/// Restore helper shared by the failure path and drop.
fn restore_backups(backups: &[(PathBuf, Option<PathBuf>)]) {
    for (live, backup) in backups {
        match backup {
            Some(backup) if backup.is_file() => {
                let _ = std::fs::copy(backup, live);
                let _ = std::fs::remove_file(backup);
            }
            _ => {
                let _ = std::fs::remove_file(live);
            }
        }
    }
}

/// Backup/restore guard for the swapped live configuration files. Mirrors the
/// Gopher64 live-config guard across the staged document set: backups always
/// refresh from the live files at prepare time, a lock file refuses
/// concurrent Lunchbox sessions (dead PIDs count as stale), partial swaps
/// roll back on failure, and drop restores every backup or removes files the
/// session created.
struct LiveSwapGuard {
    backups: Vec<(PathBuf, Option<PathBuf>)>,
    lock: PathBuf,
}

impl LiveSwapGuard {
    fn acquire(live_root: &Path, staged: &LaunchConfig, cancel: &AtomicBool) -> Result<Self> {
        cancelled(cancel)?;
        let lock = live_root.join(".lunchbox-duckstation-session.lock");
        if let Ok(contents) = std::fs::read_to_string(&lock) {
            let mut parts = contents.split_whitespace();
            let pid = parts.next().and_then(|pid| pid.parse::<u32>().ok());
            let exe = parts.next().map(PathBuf::from).unwrap_or_default();
            let alive = match (pid, exe.as_os_str().is_empty()) {
                (Some(pid), false) => child_exe_matches(pid, &exe).unwrap_or(true),
                _ => false,
            };
            ensure!(
                !alive,
                "Another Lunchbox DuckStation session is active; refusing to swap its live config (lock {})",
                lock.display()
            );
        }
        let me = format!(
            "{} {}",
            std::process::id(),
            std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        );
        std::fs::write(&lock, me).context("Recording the DuckStation session lock")?;
        let mut backups = Vec::new();
        let result = (|| {
            let staged_root = staged.data_root();
            for (staged_path, staged_bytes) in staged.document_bytes() {
                cancelled(cancel)?;
                let relative = staged_path.strip_prefix(&staged_root).with_context(|| {
                    format!(
                        "Staged DuckStation path escapes its root: {}",
                        staged_path.display()
                    )
                })?;
                ensure!(
                    !relative
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Staged DuckStation path escapes its root"
                );
                let live = live_root.join(relative);
                let live_bytes = std::fs::read(&live).ok();
                if live_bytes.as_deref() == Some(staged_bytes.as_slice()) {
                    continue;
                }
                let backup = if live_bytes.is_some() {
                    let backup = live.with_extension("lunchbox-backup");
                    std::fs::copy(&live, &backup)
                        .with_context(|| format!("Backing up {}", live.display()))?;
                    Some(backup)
                } else {
                    if let Some(parent) = live.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    None
                };
                std::fs::write(&live, &staged_bytes)
                    .with_context(|| format!("Swapping {}", live.display()))?;
                backups.push((live, backup));
            }
            Ok::<_, anyhow::Error>(())
        })();
        if let Err(error) = result {
            restore_backups(&backups);
            let _ = std::fs::remove_file(&lock);
            return Err(error);
        }
        Ok(Self { backups, lock })
    }

    fn verify(&self) -> Result<()> {
        for (live, _) in &self.backups {
            ensure!(
                live.is_file(),
                "Swapped DuckStation config disappeared: {}",
                live.display()
            );
        }
        Ok(())
    }
}

impl Drop for LiveSwapGuard {
    fn drop(&mut self) {
        restore_backups(&self.backups);
        let _ = std::fs::remove_file(&self.lock);
    }
}

pub(crate) struct FlatpakSession {
    input: super::PreparedSession,
    plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
    live_settings: PathBuf,
    guard: LiveSwapGuard,
    startup_confirmed: bool,
}

/// Parse the ordinary one-ROM Flatpak argument plan and return the content.
/// The Flatpak prefix is `run` plus `--filesystem` grants plus the audited
/// environment, then the app id, then app arguments. Every grant must resolve
/// to the exact launch directory (no wider sandbox access), and the app
/// arguments must hold the saved content exactly once plus reviewed flags.
fn parse_original<'a>(
    setup: &SavedSetup,
    original: &'a LaunchPlan,
    command: &Path,
) -> Result<&'a Path> {
    ensure!(
        original.environment.is_empty() && original.retroarch_content.is_none(),
        "DuckStation Flatpak launch requires the ordinary one-ROM argument plan"
    );
    ensure!(
        same_program(command, &original.program),
        "DuckStation Flatpak command differs from selection"
    );
    let arguments = &original.arguments;
    ensure!(
        arguments.first().is_some_and(|arg| arg == "run"),
        "DuckStation Flatpak launch requires the ordinary one-ROM argument plan"
    );
    let app_position = arguments
        .iter()
        .position(|argument| argument == APP_ID)
        .context("DuckStation Flatpak launch is missing its app id")?;
    ensure!(
        app_position >= 2,
        "DuckStation Flatpak launch has no sandbox grants"
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
                "DuckStation Flatpak launch grants a directory other than its exact launch directory"
            );
        } else {
            ensure!(
                option.to_str() == Some("--env=SDL_JOYSTICK_LINUX_CLASSIC=1"),
                "Custom DuckStation Flatpak sandbox option needs input/configuration routing resolution"
            );
        }
    }
    let setup_canonical = setup.content.canonicalize()?;
    let mut content: Option<&'a Path> = None;
    for argument in &arguments[app_position + 1..] {
        if Path::new(argument).canonicalize().ok().as_ref() == Some(&setup_canonical) {
            ensure!(
                content.is_none(),
                "DuckStation launch selects the saved content more than once"
            );
            content = Some(Path::new(argument));
        } else {
            ensure!(
                matches!(
                    argument.to_str(),
                    Some("-batch" | "-fullscreen" | "-nofullscreen")
                ),
                "Custom DuckStation argument needs input/configuration routing resolution"
            );
        }
    }
    let content = content.context("DuckStation launch does not select the saved ROM")?;
    ensure!(
        content
            .parent()
            .is_some_and(|parent| parent.canonicalize().ok().as_ref()
                == original.current_directory.canonicalize().ok().as_ref()),
        "DuckStation ROM must sit directly in its launch directory"
    );
    Ok(content)
}

pub(crate) fn prepare(
    setup: &SavedSetup,
    calibrations: &HashMap<String, crate::controller_catalog::Calibration>,
    inventory: &[crate::controllers::ControllerDevice],
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    cancel: &AtomicBool,
) -> Result<FlatpakSession> {
    setup.validate()?;
    cancelled(cancel)?;
    let runtime = setup
        .runtime
        .as_ref()
        .context("Configure trusted DuckStation helper and SDL runtime paths first")?;
    let EmulatorExecutable::Flatpak { command, app_id } = &option.executable else {
        anyhow::bail!("DuckStation Flatpak preparation needs a Flatpak runtime selection")
    };
    ensure!(
        app_id == APP_ID && setup.emulator_id == option.emulator_id,
        "DuckStation adapter supports only the audited Flathub application"
    );
    let content = parse_original(setup, plan, command)?.to_path_buf();
    // The settings layer follows the pressed disc, never a title guess.
    let serial = crate::controller_psx::single_disc_serial(&content)?;
    ensure!(
        serial == setup.serial,
        "DuckStation disc serial differs from the saved setup; review the mapping again"
    );
    let live_root = profile_data_root()?;
    ensure!(
        setup.data_root.canonicalize()? == live_root.canonicalize()?,
        "DuckStation saved data root differs from the target Flatpak profile"
    );
    let deployment = deployment(command, cancel)?;
    ensure!(
        file_hash(&deployment.executable)? == runtime.executable_sha256,
        "DuckStation executable differs from the saved trusted runtime"
    );
    ensure!(
        runtime.sdl_library.canonicalize()? == deployment.sdl_library.canonicalize()?,
        "DuckStation SDL library differs from the installed application"
    );
    let companions = companion_libraries(&deployment, command, cancel)?;
    ensure!(
        runtime.runtime_libraries.len() == companions.len() + 1,
        "DuckStation runtime library set differs from the installed target stack"
    );
    for (declared, resolved) in runtime
        .runtime_libraries
        .iter()
        .zip(std::iter::once(&deployment.sdl_library).chain(companions.iter()))
    {
        ensure!(
            declared.canonicalize()? == resolved.canonicalize()?,
            "DuckStation runtime library differs from the installed target stack"
        );
    }
    // The installed application reports the pinned input-contract revision.
    // `-version` prints the revision but exits nonzero, so check the output
    // text rather than the status; a missing application cannot print it.
    let version_output = Command::new(command)
        .args(["run", APP_ID, "-version"])
        .stdin(std::process::Stdio::null())
        .output()
        .context("Querying the DuckStation Flatpak revision")?;
    ensure!(
        version_output.stdout.len() + version_output.stderr.len() <= 64 * 1024,
        "DuckStation version report is oversized"
    );
    let version = format!(
        "{}\n{}",
        String::from_utf8_lossy(&version_output.stdout),
        String::from_utf8_lossy(&version_output.stderr)
    );
    ensure!(
        version.contains(REVISION),
        "DuckStation Flatpak revision is outside the supported input contract"
    );
    let mut staged = LaunchConfig::stage(
        &setup.data_root,
        Some(
            lunchbox_controller_probe::duckstation_config::GameIdentity {
                serial: &setup.serial,
                first_disc_serial: setup.first_disc_serial.as_deref(),
            },
        ),
    )?;
    let hints = staged.configure_classic_sdl()?;
    let mut probe = Command::new(&runtime.probe_program);
    probe
        .arg("--sdl-library")
        .arg(&runtime.sdl_library)
        .arg("--duckstation-player-probe")
        .arg(lunchbox_controller_probe::players::CONTRACT);
    for library in &runtime.runtime_libraries {
        probe.arg("--runtime-library").arg(library);
    }
    for (key, value) in &hints {
        probe.arg("--hint").arg(format!("{key}={value}"));
    }
    let database = setup.data_root.join("gamecontrollerdb.txt");
    if database.try_exists()? {
        probe.arg("--mapping-db").arg(&database);
    }
    for player in &setup.players {
        let mut devices = inventory
            .iter()
            .filter(|device| device.stable_id == player.controller_id);
        let device = devices
            .next()
            .context("DuckStation controller is disconnected")?;
        ensure!(
            devices.next().is_none(),
            "Ambiguous DuckStation physical controller identity"
        );
        probe.arg("--bindings-for-path").arg(&device.device_path);
    }
    let (output, _) = capture(&mut probe, cancel)?;
    staged.verify_originals_unchanged()?;
    let snapshot =
        serde_json::from_slice(&output).context("Invalid DuckStation SDL helper response")?;
    let mut input = prepare_resolved(setup, calibrations, inventory, &snapshot)?;
    // The sandbox cannot see the private staged tree, so point the staged
    // per-game and input-profile folders at the live directories whose
    // copies are swapped in below; saves and cards already reference live
    // paths.
    input.configuration.redirect_folders_for_swap(&live_root)?;
    let guard = LiveSwapGuard::acquire(&live_root, &input.configuration, cancel)?;
    let live_settings = live_root.join("settings.ini");
    let mut files = BTreeMap::new();
    for path in [
        &content,
        &runtime.probe_program,
        &runtime.sdl_library,
        &deployment.executable,
    ] {
        files.insert(path.clone(), file_hash(path)?);
    }
    for library in &runtime.runtime_libraries {
        files.insert(library.clone(), file_hash(library)?);
    }
    let mut configured = plan.clone();
    let app_position = configured
        .arguments
        .iter()
        .position(|argument| argument == APP_ID)
        .context("DuckStation Flatpak plan lost its app id")?;
    configured
        .arguments
        .insert(app_position + 1, "-earlyconsole".into());
    let session = FlatpakSession {
        input,
        plan: configured.clone(),
        files,
        live_settings,
        guard,
        startup_confirmed: false,
    };
    session.verify()?;
    *plan = configured;
    Ok(session)
}

impl FlatpakSession {
    pub(crate) fn verify(&self) -> Result<()> {
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "DuckStation Flatpak launch input changed: {}",
                path.display()
            );
        }
        self.guard.verify()?;
        Ok(())
    }

    pub(crate) fn confirm_startup(&mut self, log: &str) -> Result<()> {
        self.startup_confirmed = false;
        self.input
            .configuration
            .verify_startup_routing_for(log, &self.live_settings)?;
        self.input.projection.verify_startup_log(log)?;
        self.verify()?;
        self.startup_confirmed = true;
        Ok(())
    }

    pub(crate) fn spawn(&mut self, plan: &LaunchPlan, cancel: &AtomicBool) -> Result<Child> {
        ensure!(
            &self.plan == plan,
            "DuckStation Flatpak launch plan changed after input preparation"
        );
        self.verify()?;
        cancelled(cancel)?;
        // Same piped startup proof as native. The /proc library audit is
        // native-only: the host PID is the Flatpak wrapper, not the sandbox
        // emulator, so log routing plus the executable hash carry the proof
        // (the same standard as non-Linux hosts).
        let mut child = crate::emulator::spawn_launch_plan_with_controller_pipes(plan)?;
        let result = (|| {
            let output = Arc::new(Mutex::new([Vec::<u8>::new(), Vec::<u8>::new()]));
            let collecting = Arc::new(AtomicBool::new(true));
            let overflow = Arc::new(AtomicBool::new(false));
            let readers: Vec<Box<dyn Read + Send>> = vec![
                Box::new(child.stdout.take().context("Missing DuckStation stdout")?),
                Box::new(child.stderr.take().context("Missing DuckStation stderr")?),
            ];
            for (stream, mut reader) in readers.into_iter().enumerate() {
                let output = output.clone();
                let collecting = collecting.clone();
                let overflow = overflow.clone();
                std::thread::Builder::new()
                    .name("duckstation-flatpak-startup-log".into())
                    .spawn(move || {
                        let mut buffer = [0_u8; 4096];
                        loop {
                            let count = match reader.read(&mut buffer) {
                                Ok(0) | Err(_) => break,
                                Ok(count) => count,
                            };
                            if collecting.load(Ordering::Relaxed) {
                                let Ok(mut output) = output.lock() else {
                                    break;
                                };
                                if output[stream].len() + count <= OUTPUT_LIMIT {
                                    output[stream].extend_from_slice(&buffer[..count]);
                                } else {
                                    overflow.store(true, Ordering::Relaxed);
                                }
                            }
                        }
                    })?;
            }
            let deadline = Instant::now() + Duration::from_secs(20);
            let confirmed = (|| {
                loop {
                    cancelled(cancel)?;
                    ensure!(
                        !overflow.load(Ordering::Relaxed),
                        "DuckStation startup log exceeded limit"
                    );
                    ensure!(
                        child.try_wait()?.is_none(),
                        "DuckStation exited before controller startup was confirmed"
                    );
                    let log = {
                        let output = output
                            .lock()
                            .map_err(|_| anyhow::anyhow!("DuckStation log reader failed"))?;
                        format!(
                            "{}\n{}",
                            String::from_utf8_lossy(&output[0]),
                            String::from_utf8_lossy(&output[1])
                        )
                    };
                    if self.confirm_startup(&log).is_ok() {
                        return Ok(());
                    }
                    ensure!(
                        Instant::now() < deadline,
                        "DuckStation did not confirm private configuration and controller routing before timeout"
                    );
                    std::thread::sleep(Duration::from_millis(25));
                }
            })();
            collecting.store(false, Ordering::Relaxed);
            confirmed
        })();
        if let Err(error) = result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.verify()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_root_with_settings(dir: &Path, body: &str) -> PathBuf {
        let root = dir.join("config/duckstation");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("settings.ini"), body).unwrap();
        root
    }

    const SETTINGS: &str = "[Main]\nSettingsVersion = 3\n";

    #[test]
    fn live_swap_restores_backups_and_removes_the_lock() {
        let directory = tempfile::tempdir().unwrap();
        let live_root = data_root_with_settings(directory.path(), SETTINGS);
        let staged = LaunchConfig::stage(&live_root, None).unwrap();
        let guard = LiveSwapGuard::acquire(&live_root, &staged, &AtomicBool::new(false)).unwrap();
        // settings.ini always differs (diagnostics + folder routing).
        let swapped = std::fs::read(live_root.join("settings.ini")).unwrap();
        assert_ne!(swapped, SETTINGS.as_bytes());
        assert!(
            live_root
                .join(".lunchbox-duckstation-session.lock")
                .is_file()
        );
        guard.verify().unwrap();
        drop(guard);
        assert_eq!(
            std::fs::read(live_root.join("settings.ini")).unwrap(),
            SETTINGS.as_bytes()
        );
        assert!(
            !live_root
                .join(".lunchbox-duckstation-session.lock")
                .try_exists()
                .unwrap()
        );
        assert!(
            !live_root
                .join("settings.lunchbox-backup")
                .try_exists()
                .unwrap()
        );
    }

    #[test]
    fn live_swap_refuses_a_live_session_but_takes_over_a_stale_lock() {
        let directory = tempfile::tempdir().unwrap();
        let live_root = data_root_with_settings(directory.path(), SETTINGS);
        let staged = LaunchConfig::stage(&live_root, None).unwrap();
        let lock = live_root.join(".lunchbox-duckstation-session.lock");
        std::fs::write(
            &lock,
            format!(
                "{} {}",
                std::process::id(),
                std::env::current_exe().unwrap().display()
            ),
        )
        .unwrap();
        assert!(LiveSwapGuard::acquire(&live_root, &staged, &AtomicBool::new(false)).is_err());
        // Unparseable locks count as stale without depending on /proc visibility.
        std::fs::write(&lock, "garbage-lock-contents").unwrap();
        let guard = LiveSwapGuard::acquire(&live_root, &staged, &AtomicBool::new(false)).unwrap();
        drop(guard);
        assert_eq!(
            std::fs::read(live_root.join("settings.ini")).unwrap(),
            SETTINGS.as_bytes()
        );
    }
}
