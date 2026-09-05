//! Actual installed-Flatpak startup oracle. No ROM, BIOS or user config is loaded.
//! Run with a freshly captured player-probe snapshot as the single argument.
#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("This oracle requires the installed Linux DuckStation Flatpak")
}

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    use lunchbox_controller_probe::{Snapshot, players::CONTRACT};
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        path::PathBuf,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    const APP: &str = "org.duckstation.DuckStation";
    let snapshot_path = std::env::args_os()
        .nth(1)
        .context("Pass a fresh player-probe JSON snapshot")?;
    let snapshot: Snapshot = serde_json::from_slice(&fs::read(snapshot_path)?)?;
    let projection = snapshot
        .player_probe
        .as_ref()
        .context("Snapshot has no player projection")?;
    ensure!(
        projection.duckstation_revision == CONTRACT && snapshot.sdl_version == 3_002_020,
        "Oracle only covers the verified DuckStation and SDL revisions"
    );
    ensure!(
        !snapshot.runtime_libraries.is_empty(),
        "Snapshot lacks verified runtime dependencies"
    );
    for library in &snapshot.runtime_libraries {
        ensure!(
            library.path.is_absolute(),
            "Invalid runtime dependency path"
        );
        let hash = Command::new("flatpak")
            .args(["run", "--command=/usr/bin/sha256sum", APP])
            .arg(&library.path)
            .output()?;
        ensure!(
            hash.status.success()
                && String::from_utf8_lossy(&hash.stdout)
                    .split_whitespace()
                    .next()
                    == Some(library.sha256.as_str()),
            "Runtime dependency changed since player projection: {}",
            library.path.display()
        );
    }
    if let Some(expected_hash) = &snapshot.mapping_database_sha256 {
        let path = snapshot
            .effective_hints
            .get("SDL_GAMECONTROLLERCONFIG_FILE")
            .and_then(Option::as_ref)
            .context("Missing effective mapping database")?;
        ensure!(
            PathBuf::from(path).is_absolute(),
            "Invalid mapping database path"
        );
        let hash = Command::new("flatpak")
            .args(["run", "--command=/usr/bin/sha256sum", APP, path])
            .output()?;
        ensure!(
            hash.status.success()
                && String::from_utf8_lossy(&hash.stdout)
                    .split_whitespace()
                    .next()
                    == Some(expected_hash.as_str()),
            "Mapping database changed since player projection"
        );
    }
    let location = Command::new("flatpak")
        .args(["info", "--show-location", APP])
        .output()?;
    ensure!(
        location.status.success(),
        "DuckStation Flatpak not installed"
    );
    let install = PathBuf::from(std::str::from_utf8(&location.stdout)?.trim());
    ensure!(
        install.is_absolute(),
        "Invalid Flatpak installation location"
    );
    for name in ["portable.txt", "settings.ini"] {
        ensure!(
            !install.join("files/bin").join(name).exists(),
            "Portable config would override isolation"
        );
    }
    let library = snapshot
        .library
        .strip_prefix("/app/")
        .context("Expected target Flatpak SDL library")?;
    ensure!(
        !library
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir)),
        "Invalid library path"
    );
    ensure!(
        format!(
            "{:x}",
            Sha256::digest(fs::read(install.join("files").join(library))?)
        ) == snapshot.library_sha256,
        "Target SDL library changed since projection"
    );
    let version = Command::new("flatpak")
        .args(["run", APP, "-version"])
        .output()?;
    ensure!(
        String::from_utf8_lossy(&version.stderr).contains("0.1-9482-g0a53bc47c"),
        "Installed DuckStation revision differs"
    );
    let original = PathBuf::from(std::env::var_os("HOME").context("HOME unavailable")?)
        .join(".var/app")
        .join(APP)
        .join("config/duckstation/settings.ini");
    let before = fs::read(&original).context("Reading baseline config fingerprint")?;
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-duckstation-startup-")
        .tempdir_in("/tmp")?
        .keep();
    eprintln!("Startup evidence: {}", directory.display());
    let config_root = directory.join("config");
    let duck_root = config_root.join("duckstation");
    fs::create_dir_all(&duck_root)?;
    fs::write(
        duck_root.join("settings.ini"),
        "[Main]\nSettingsVersion = 3\nSetupWizardIncomplete = false\n\n[Logging]\nLogLevel = Verbose\nLogToConsole = true\nLogTimestamps = false\n\n[InputSources]\nSDL = true\nSDLControllerEnhancedMode = false\nSDLPS5PlayerLED = false\n\n[SDLHints]\nSDL_JOYSTICK_LINUX_CLASSIC = 1\n\n[AutoUpdater]\nCheckAtStartup = false\n",
    )?;
    let output_path = directory.join("startup.log");
    let output = fs::File::create(&output_path)?;
    let mut command = Command::new("flatpak");
    command
        .arg("run")
        .arg("--die-with-parent")
        .arg(format!("--filesystem={}:rw", directory.display()))
        .arg("--command=/usr/bin/env");
    for (name, value) in &snapshot.effective_hints {
        if let Some(value) = value {
            lunchbox_controller_probe::parse_hint(&format!("{name}={value}"))?;
            command.arg(format!("--env={name}={value}"));
        } else {
            lunchbox_controller_probe::parse_hint(&format!("{name}="))?;
            command.arg(format!("--unset-env={name}"));
        }
    }
    command
        .arg(APP)
        // Flatpak reserves XDG paths: set these *after* entering its sandbox.
        // env executes the binary directly; no shell or shell interpolation.
        .arg(format!("XDG_CONFIG_HOME={}", config_root.display()))
        .arg(format!(
            "XDG_CACHE_HOME={}",
            directory.join("cache").display()
        ))
        .arg(format!(
            "XDG_DATA_HOME={}",
            directory.join("data").display()
        ))
        .arg("QT_QPA_PLATFORM=offscreen")
        .args(["/app/bin/duckstation-qt", "-earlyconsole", "-nofullscreen"])
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output);
    let mut child = command.spawn().context("Starting isolated DuckStation")?;
    let started = Instant::now();
    let check = (|| -> anyhow::Result<()> {
        loop {
            let log = fs::read_to_string(&output_path)?;
            if log.contains("Loading config from ") {
                ensure!(
                    log.contains(&format!(
                        "Loading config from {}",
                        duck_root.join("settings.ini").display()
                    )),
                    "Emulator loaded a non-isolated configuration; stopping oracle"
                );
            }
            if projection.verify_startup_log(&log).is_ok() {
                ensure!(
                    log.contains(&format!(
                        "Loading config from {}",
                        duck_root.join("settings.ini").display()
                    )),
                    "Emulator did not confirm the isolated config path"
                );
                return Ok(());
            }
            ensure!(
                child.try_wait()?.is_none(),
                "DuckStation exited before confirming players; see startup.log"
            );
            ensure!(
                started.elapsed() < Duration::from_secs(10),
                "Actual player log did not match projection; see startup.log"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    })();
    // Terminate only the child this oracle owns; no app-wide process killing.
    if child.try_wait()?.is_none() {
        unsafe {
            libc::kill(child.id() as i32, libc::SIGTERM);
        }
        let stop = Instant::now();
        while child.try_wait()?.is_none() && stop.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(20));
        }
        if child.try_wait()?.is_none() {
            child.kill()?;
        }
    }
    child.wait()?;
    ensure!(
        fs::read(&original)? == before,
        "Original DuckStation settings changed during oracle run"
    );
    check?;
    let log = fs::read_to_string(output_path)?;
    projection.verify_startup_log(&log)?;
    println!(
        "Verified {} actual device opens against the projection. Original settings unchanged. Evidence: {}",
        projection.assignments.len(),
        directory.display()
    );
    Ok(())
}
