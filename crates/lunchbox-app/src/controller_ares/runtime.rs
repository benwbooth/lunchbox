use super::*;
use crate::{
    controllers::ControllerDevice,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    settings::AppSettings,
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

fn capture(command: &mut Command, cancel: &AtomicBool) -> Result<String> {
    ensure!(!cancel.load(Ordering::Relaxed), "Launch cancelled");
    let stdout = tempfile::tempfile()?;
    let stderr = tempfile::tempfile()?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .spawn()?;
    let result = (|| {
        let start = Instant::now();
        loop {
            ensure!(!cancel.load(Ordering::Relaxed), "Launch cancelled");
            ensure!(
                stdout.metadata()?.len() + stderr.metadata()?.len() < 8 * 1024 * 1024,
                "Controller probe produced excessive output"
            );
            if let Some(status) = child.try_wait()? {
                use std::io::{Read, Seek};
                let mut output = String::new();
                let mut error = String::new();
                let mut stdout = &stdout;
                let mut stderr = &stderr;
                stdout.rewind()?;
                stderr.rewind()?;
                stdout.read_to_string(&mut output)?;
                stderr.read_to_string(&mut error)?;
                ensure!(status.success(), "Controller setup command failed: {error}");
                return Ok(output);
            }
            ensure!(
                start.elapsed() < Duration::from_secs(15),
                "Controller probe timed out"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn info(flatpak: &Path, id: &str, flag: &str, cancel: &AtomicBool) -> Result<String> {
    Ok(capture(
        crate::platform_process::host_command(flatpak).args(["info", flag, id]),
        cancel,
    )?
    .trim()
    .to_owned())
}

fn helper(cancel: &AtomicBool) -> Result<PathBuf> {
    let path = if let Some(path) = std::env::var_os("LUNCHBOX_CONTROLLER_PROBE") {
        // Also permits integration tests to select their freshly built helper.
        PathBuf::from(path)
    } else if crate::platform_process::is_flatpak() {
        let id = std::env::var("FLATPAK_ID").context("Missing Lunchbox Flatpak identity")?;
        PathBuf::from(info(Path::new("flatpak"), &id, "--show-location", cancel)?)
            .join("files/bin/lunchbox-controller-probe")
    } else {
        std::env::current_exe()?
            .parent()
            .context("Missing application directory")?
            .join(if cfg!(windows) {
                "lunchbox-controller-probe.exe"
            } else {
                "lunchbox-controller-probe"
            })
    };
    ensure!(
        path.is_file(),
        "The Lunchbox installation is missing its controller probe. Rebuild/reinstall the full package."
    );
    Ok(path)
}

struct Runtime {
    library: PathBuf,
    library_dir: PathBuf,
    base: PathBuf,
}

fn runtime(option: &RomEmulatorOption, cancel: &AtomicBool) -> Result<Runtime> {
    let dirs = directories::BaseDirs::new().context("Missing user directories")?;
    match &option.executable {
        EmulatorExecutable::Flatpak { command, app_id } => {
            let runtime = info(command, app_id, "--show-runtime", cancel)?;
            let root =
                PathBuf::from(info(command, &runtime, "--show-location", cancel)?).join("files");
            let mut paths = vec![root.join("lib")];
            if let Ok(entries) = std::fs::read_dir(root.join("lib")) {
                paths.extend(
                    entries
                        .filter_map(Result::ok)
                        .map(|e| e.path())
                        .filter(|p| p.is_dir()),
                );
            }
            let app =
                PathBuf::from(info(command, app_id, "--show-location", cancel)?).join("files");
            require_private_settings(&app.join("bin/ares"))?;
            paths.insert(0, app.join("lib"));
            let library = paths
                .iter()
                .map(|dir| dir.join("libSDL3.so.0"))
                .find(|p| p.is_file())
                .context("ares's SDL3 library could not be located")?;
            let base = dirs
                .home_dir()
                .join(".var/app")
                .join(app_id)
                .join("data/ares/settings.bml");
            Ok(Runtime {
                library_dir: library.parent().unwrap().to_path_buf(),
                library,
                base,
            })
        }
        EmulatorExecutable::Native(program) => {
            require_private_settings(program)?;
            let directory = program
                .parent()
                .context("Missing ares executable directory")?;
            let portable = directory.join("settings.bml");
            let base = if portable.is_file() {
                portable
            } else {
                dirs.data_local_dir().join("ares/settings.bml")
            };
            let names = ["SDL3.dll", "libSDL3.dylib", "libSDL3.so.0"];
            let mut candidates: Vec<_> = [
                directory.to_path_buf(),
                directory.join("../lib"),
                directory.join("../Frameworks"),
            ]
            .into_iter()
            .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
            .filter(|p| p.is_file())
            .collect();
            for relative in [
                "../Frameworks/SDL3.framework/SDL3",
                "../Frameworks/SDL3.framework/Versions/A/SDL3",
            ] {
                let framework = directory.join(relative);
                if framework.is_file() {
                    candidates.push(framework);
                }
            }
            #[cfg(target_os = "linux")]
            if candidates.is_empty() {
                let output = capture(
                    crate::platform_process::host_command("ldd").arg(program),
                    cancel,
                )?;
                for line in output.lines().filter(|l| l.contains("libSDL3.so")) {
                    if let Some(path) = line.split_whitespace().find(|s| s.starts_with('/')) {
                        candidates.push(path.into());
                    }
                }
            }
            let library = candidates
                .into_iter()
                .find(|p| p.is_file())
                .context("Could not locate the SDL3 library used by ares")?;
            Ok(Runtime {
                library_dir: library.parent().unwrap().to_path_buf(),
                library,
                base,
            })
        }
        EmulatorExecutable::Wine { .. } => {
            anyhow::bail!("Use native ares for automatic controller mapping")
        }
    }
}

fn require_private_settings(program: &Path) -> Result<()> {
    // Old ares silently ignores unknown CLI flags and writes its normal file
    // even for --help. Check this capability without executing it first.
    let bytes = std::fs::read(program).context("Reading ares version capabilities")?;
    ensure!(
        bytes
            .windows(b"--settings-file".len())
            .any(|s| s == b"--settings-file"),
        "Automatic mapping requires ares 148 or newer with --settings-file support (or its actual executable instead of a wrapper)"
    );
    Ok(())
}

fn emulator_command(option: &RomEmulatorOption, directory: &Path) -> Command {
    match &option.executable {
        EmulatorExecutable::Native(program) => crate::platform_process::host_command(program),
        EmulatorExecutable::Flatpak { command, app_id } => {
            let mut result = crate::platform_process::host_command(command);
            result
                .args(["run", "--command=ares"])
                .arg(format!("--filesystem={}", directory.display()))
                .arg(app_id);
            result
        }
        _ => unreachable!(),
    }
}

/// Generate mappings from the same saved calibration and player order that the
/// guided UI shows. No second emulator-specific setup dialog is required.
pub fn prepare(
    settings: &AppSettings,
    platform: &str,
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    devices: &[&ControllerDevice],
    cancel: &AtomicBool,
) -> Result<tempfile::TempDir> {
    let profile = super::profile(platform)
        .context("ares has no ordinary gamepad contract for this system yet")?;
    ensure!(
        devices.len() <= profile.native_launch.as_ref().unwrap().max_players,
        "Too many players selected for this ares system"
    );
    ensure!(
        !plan.arguments.iter().any(|a| a == "--settings-file"
            || a == "--setting"
            || a.to_string_lossy().starts_with("--settings-file=")),
        "Custom ares settings arguments conflict with Controller setup; remove those arguments first"
    );
    let runtime = runtime(option, cancel)?;
    // Home-backed storage is visible to both Lunchbox and an emulator Flatpak;
    // /tmp is private in each sandbox and cannot be used for this handoff.
    let sessions = directories::BaseDirs::new()
        .context("Missing application data directory")?
        .data_local_dir()
        .join("lunchbox/controller-sessions");
    std::fs::create_dir_all(&sessions)?;
    let directory = tempfile::Builder::new()
        .prefix("ares-")
        .tempdir_in(sessions)?;
    let config = directory.path().join("settings.bml");
    let base = match std::fs::read_to_string(&runtime.base) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let helper = helper(cancel)?;
    let mut probe = if cfg!(target_os = "linux") {
        let mut command = crate::platform_process::host_command("env");
        command
            .arg(format!("LD_LIBRARY_PATH={}", runtime.library_dir.display()))
            .arg(&helper);
        command
    } else {
        Command::new(helper)
    };
    // This is a separate non-Qt helper. Load the emulator's own SDL and its
    // runtime dependencies, not Lunchbox's potentially different SDL version.
    probe.arg("--sdl-library").arg(&runtime.library);
    let udev = runtime.library_dir.join("libudev.so.1");
    if udev.is_file() {
        probe.arg("--runtime-library").arg(udev);
    }
    if cfg!(target_os = "linux") {
        probe.args(["--hint", "SDL_JOYSTICK_LINUX_CLASSIC=1"]);
    }
    probe.args(["--hint", "SDL_JOYSTICK_HIDAPI_STEAM=1"]);
    let mut paths = Vec::new();
    for device in devices {
        let calibration = &settings.controller_mapping.calibrations[&device.stable_id];
        let path = if calibration.backend == crate::controller_sdl3::BACKEND {
            device
                .physical_path
                .clone()
                .context("SDL3 did not report this controller's hardware path")?
        } else {
            device
                .device_path
                .to_str()
                .context("Controller path is not UTF-8")?
                .to_owned()
        };
        probe.arg("--bindings-for-path").arg(&path);
        paths.push(path);
    }
    let snapshot: Snapshot = serde_json::from_str(&capture(&mut probe, cancel)?)
        .context("Invalid SDL runtime response")?;
    if cfg!(target_os = "linux") {
        ensure!(
            snapshot
                .effective_hints
                .get("SDL_JOYSTICK_LINUX_CLASSIC")
                .and_then(Option::as_deref)
                == Some("1"),
            "A global SDL environment override prevents the calibrated input backend"
        );
    }
    let players = devices
        .iter()
        .zip(paths)
        .map(|(device, path)| {
            player_bindings(
                &settings.controller_mapping.calibrations[&device.stable_id],
                profile,
                &snapshot,
                &path,
            )
            .with_context(|| format!("Mapping {} for ares", device.name))
        })
        .collect::<Result<Vec<_>>>()?;
    std::fs::write(&config, configuration(&base, &players)?)?;
    // Real ares parser roundtrip: unknown settings or dropped mappings must
    // fail here, not produce a visually correct preview with unresponsive play.
    let output = capture(
        emulator_command(option, directory.path())
            .arg("--settings-file")
            .arg(&config)
            .args(["--setting", "Input/Driver=SDL", "--dump-all-settings"]),
        cancel,
    )?;
    ensure!(
        output.lines().any(|l| l.trim() == "VirtualPad1/A..South"),
        "ares 148 or newer is required for private controller settings"
    );
    let roundtrip = std::fs::read_to_string(&config)?;
    for (port, player) in players.iter().enumerate() {
        for (key, binding) in player {
            ensure!(
                output
                    .lines()
                    .any(|line| line.trim() == format!("VirtualPad{}/{key}", port + 1))
                    && roundtrip.contains(&format!("{key}: {binding}")),
                "ares did not accept the generated controller mapping"
            );
        }
    }
    if let EmulatorExecutable::Flatpak { app_id, .. } = &option.executable {
        let position = plan
            .arguments
            .iter()
            .position(|arg| arg == app_id.as_str())
            .context("Missing ares Flatpak app argument")?;
        plan.arguments.splice(
            position..position,
            [
                format!("--filesystem={}", directory.path().display()).into(),
                "--env=SDL_JOYSTICK_LINUX_CLASSIC=1".into(),
                "--env=SDL_JOYSTICK_HIDAPI_STEAM=1".into(),
            ],
        );
    } else {
        if cfg!(target_os = "linux") {
            plan.environment
                .retain(|(k, _)| k != "SDL_JOYSTICK_LINUX_CLASSIC");
            plan.environment
                .push(("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into()));
        }
        plan.environment
            .retain(|(k, _)| k != "SDL_JOYSTICK_HIDAPI_STEAM");
        plan.environment
            .push(("SDL_JOYSTICK_HIDAPI_STEAM".into(), "1".into()));
    }
    plan.arguments
        .extend(["--settings-file".into(), config.into_os_string()]);
    plan.arguments
        .extend(["--setting".into(), "Input/Driver=SDL".into()]);
    Ok(directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_executable_is_rejected_without_running_or_writing_settings() {
        let root = tempfile::tempdir().unwrap();
        let program = root.path().join("ares");
        std::fs::write(&program, b"old ares supports --setting only").unwrap();
        assert!(require_private_settings(&program).is_err());
        std::fs::write(&program, b"ares supports --settings-file").unwrap();
        assert!(require_private_settings(&program).is_ok());
    }

    #[test]
    #[ignore = "Requires installed ares Flatpak and an explicitly selected connected calibrated controller"]
    fn installed_ares_accepts_saved_n64_mapping_without_changing_user_config() {
        let id =
            std::env::var("LUNCHBOX_TEST_CONTROLLER").expect("Set the exact saved controller ID");
        let settings = crate::settings::SettingsStore::open_default()
            .unwrap()
            .load()
            .unwrap();
        let inventory = crate::controllers::list_local_controllers(&mut Vec::new());
        let device = inventory
            .iter()
            .find(|d| d.stable_id == id)
            .expect("Selected controller must be connected");
        let option = RomEmulatorOption::standalone(
            "test-ares".into(),
            "ares".into(),
            EmulatorExecutable::Flatpak {
                command: "flatpak".into(),
                app_id: "dev.ares.ares".into(),
            },
        );
        let cancel = AtomicBool::new(false);
        let source = runtime(&option, &cancel).unwrap().base;
        let original = std::fs::read(&source).unwrap();
        let mut plan = LaunchPlan {
            emulator_name: "ares".into(),
            program: "flatpak".into(),
            arguments: vec!["run".into(), "dev.ares.ares".into()],
            current_directory: std::env::temp_dir(),
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };
        let directory = prepare(
            &settings,
            "Nintendo 64",
            &option,
            &mut plan,
            &[device],
            &cancel,
        )
        .unwrap();
        let config = std::fs::read_to_string(directory.path().join("settings.bml")).unwrap();
        assert!(config.contains("A..South: "));
        assert!(config.contains("R-Trigger: "));
        assert_eq!(std::fs::read(source).unwrap(), original);
        assert!(plan.arguments.iter().any(|a| a == "--settings-file"));
        println!(
            "Installed ares accepted all N64 bindings; original configuration unchanged.\n{}",
            plan.command_summary()
        );
    }
}
