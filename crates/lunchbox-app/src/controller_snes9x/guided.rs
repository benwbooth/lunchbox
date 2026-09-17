//! First-launch discovery for the Snes9x GTK frontend, native and Flatpak.
//! Synthesizes a launch-scoped saved setup instead of failing for a missing
//! hand-written entry (mgba/gopher64 precedent). Settings review never calls
//! this.
use super::{
    flatpak,
    isolation::PreparedConfig,
    settings::{Player, SavedSetup},
};
use crate::{
    controller_native_process::{cancelled, capture},
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    platform_process::host_command,
};
use anyhow::{Context, Result, ensure};
use std::{
    io::Read,
    path::{Component, Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn executable_on_path(name: &str) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(name))
        .find(|path| {
            std::fs::metadata(path)
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
        .with_context(|| format!("Install {name} to use private native controller configurations"))
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

fn native_sdl_library(program: &Path, cancel: &AtomicBool) -> Result<PathBuf> {
    let mut header = [0_u8; 4];
    std::fs::File::open(program)?.read_exact(&mut header)?;
    ensure!(
        header == *b"\x7fELF",
        "Choose the actual Snes9x GTK executable rather than a launcher script"
    );
    let mut ldd = host_command("ldd");
    ldd.arg(program);
    let (out, _) = capture(&mut ldd, cancel)?;
    let paths: std::collections::BTreeSet<_> = std::str::from_utf8(&out)?
        .lines()
        .filter_map(|line| {
            let (name, target) = line.trim().split_once(" => ")?;
            if name != "libSDL2-2.0.so.0" && name != "libSDL2.so" {
                return None;
            }
            let path = PathBuf::from(target.split_whitespace().next()?);
            (path.is_absolute() && path.is_file()).then_some(path)
        })
        .collect();
    ensure!(
        paths.len() == 1,
        "Could not resolve one SDL2 library for this Snes9x executable"
    );
    Ok(paths.into_iter().next().unwrap())
}

fn flatpak_output(command: &Path, args: &[&str]) -> Result<String> {
    let mut process = host_command(command);
    process.args(args);
    let (output, _) = capture(&mut process, &AtomicBool::new(false))?;
    String::from_utf8(output)
        .context("Snes9x Flatpak info was not UTF-8")
        .map(|value| value.trim().to_owned())
}

/// SDL2 library inside the active Flatpak runtime, mirroring the prepare
/// contract: under `files/lib*`, named `libSDL2-2.0.so.0`.
fn flatpak_sdl_library(command: &Path) -> Result<PathBuf> {
    let runtime = flatpak_output(command, &["info", "--show-runtime", flatpak::APP_ID])?;
    let mut parts = runtime.split('/');
    ensure!(
        parts.next() == Some("org.freedesktop.Platform")
            && parts.next() == Some(std::env::consts::ARCH),
        "Snes9x Flatpak runtime is outside the supported Freedesktop contract"
    );
    let location = flatpak_output(command, &["info", "--show-location", &runtime])?;
    let root = PathBuf::from(location).join("files");
    let mut dirs = vec![root.join("lib")];
    if let Ok(entries) = std::fs::read_dir(root.join("lib")) {
        dirs.extend(
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_dir()),
        );
    }
    let mut matches: Vec<_> = dirs
        .iter()
        .map(|dir| dir.join("libSDL2-2.0.so.0"))
        .filter(|path| path.is_file())
        .collect();
    matches.sort();
    matches.dedup();
    ensure!(
        matches.len() == 1,
        "Could not resolve one SDL2 library in the Snes9x Flatpak runtime"
    );
    Ok(matches.into_iter().next().unwrap())
}

fn players(ids: &[String]) -> Result<Vec<Player>> {
    ensure!(
        (1..=5).contains(&ids.len()),
        "Snes9x needs one to five players; assign players in Controller setup"
    );
    Ok(ids
        .iter()
        .enumerate()
        .map(|(index, id)| Player {
            player: u8::try_from(index + 1).unwrap(),
            controller_id: id.clone(),
        })
        .collect())
}

fn absolute_content(path: &std::ffi::OsStr) -> Result<PathBuf> {
    let content = PathBuf::from(path);
    ensure!(
        content.is_absolute()
            && !content
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            && content.is_file(),
        "Snes9x needs the selected ROM's absolute file path"
    );
    Ok(content)
}

pub(crate) fn discover(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
    ids: &[String],
    cancel: &AtomicBool,
) -> Result<SavedSetup> {
    cancelled(cancel)?;
    ensure!(
        !crate::platform_process::is_flatpak(),
        "Snes9x automatic runtime discovery from a Lunchbox Flatpak still needs host-namespace integration"
    );
    ensure!(
        plan.environment.is_empty(),
        "Custom Snes9x launcher environments need explicit runtime resolution"
    );
    let players = players(ids)?;
    let setup = match &option.executable {
        EmulatorExecutable::Native(program) => {
            ensure!(
                program == &plan.program,
                "Custom Snes9x launcher environments need explicit runtime resolution"
            );
            ensure!(
                plan.arguments.len() == 1,
                "Snes9x guided launch needs exactly the ROM argument"
            );
            let content = absolute_content(&plan.arguments[0])?;
            let cwd = plan.current_directory.canonicalize()?;
            let source_config = PreparedConfig::native_path(&cwd)?;
            ensure!(
                source_config.is_file(),
                "Open Snes9x GTK once to create snes9x.conf, then launch through Lunchbox again"
            );
            SavedSetup {
                emulator_id: option.emulator_id.clone(),
                content,
                source_config,
                probe_program: helper()?,
                sdl_library: native_sdl_library(&program.canonicalize()?, cancel)?,
                bubblewrap_program: executable_on_path("bwrap")?,
                executable_sha256: lunchbox_controller_probe::file_hash(program)?,
                players,
            }
        }
        EmulatorExecutable::Flatpak { command, app_id } => {
            ensure!(
                app_id == flatpak::APP_ID,
                "Snes9x automatic discovery supports only the exact Flathub GTK application"
            );
            ensure!(
                plan.arguments.len() == 4
                    && plan.arguments[0] == "run"
                    && plan.arguments[2] == flatpak::APP_ID,
                "Snes9x Flatpak launch requires the ordinary one-ROM argument plan"
            );
            let grant = plan.arguments[1]
                .to_str()
                .and_then(|value| value.strip_prefix("--filesystem="))
                .context("Snes9x Flatpak launch is missing its ROM-directory grant")?;
            ensure!(
                !grant.contains(':')
                    && Path::new(grant).canonicalize()? == plan.current_directory.canonicalize()?,
                "Snes9x Flatpak launch grants a directory other than its exact launch directory"
            );
            let content = absolute_content(&plan.arguments[3])?;
            ensure!(
                content
                    .parent()
                    .is_some_and(|parent| parent.canonicalize().ok().as_ref()
                        == plan.current_directory.canonicalize().ok().as_ref()),
                "Snes9x ROM must sit directly in its launch directory"
            );
            let home = directories::BaseDirs::new()
                .context("Finding the Snes9x Flatpak profile")?
                .home_dir()
                .to_path_buf();
            let source_config = home
                .join(".var/app")
                .join(flatpak::APP_ID)
                .join("config/snes9x/snes9x.conf");
            ensure!(
                source_config.is_file(),
                "Open Snes9x once to create snes9x.conf, then launch through Lunchbox again"
            );
            let location = flatpak_output(command, &["info", "--show-location", flatpak::APP_ID])?;
            let executable = PathBuf::from(location)
                .join("files")
                .join(flatpak::APP_EXECUTABLE);
            ensure!(
                executable.is_file(),
                "Snes9x Flatpak executable could not be located"
            );
            SavedSetup {
                emulator_id: option.emulator_id.clone(),
                content,
                source_config,
                probe_program: helper()?,
                sdl_library: flatpak_sdl_library(command)?,
                bubblewrap_program: executable_on_path("bwrap")?,
                executable_sha256: lunchbox_controller_probe::file_hash(&executable)?,
                players,
            }
        }
        EmulatorExecutable::Wine { .. } => {
            anyhow::bail!("Snes9x automatic discovery needs a native or Flatpak build, not Wine")
        }
    };
    setup.validate()?;
    cancelled(cancel)?;
    Ok(setup)
}
