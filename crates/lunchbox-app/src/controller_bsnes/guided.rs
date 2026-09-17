//! First-launch discovery for bsnes v115+, native and Flatpak. Synthesizes a
//! launch-scoped saved setup instead of failing for a missing hand-written
//! entry (mgba/gopher64 precedent). Settings review never calls this.
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
    io::Read,
    path::{Component, Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn flatpak_output(command: &Path, args: &[&str]) -> Result<String> {
    let mut process = host_command(command);
    process.args(args);
    let (output, _) = capture(&mut process, &AtomicBool::new(false))?;
    String::from_utf8(output)
        .context("bsnes Flatpak info was not UTF-8")
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

fn native_sdl_library(program: &Path, cancel: &AtomicBool) -> Result<PathBuf> {
    let mut header = [0_u8; 4];
    std::fs::File::open(program)?.read_exact(&mut header)?;
    ensure!(
        header == *b"\x7fELF",
        "Choose the actual bsnes executable rather than a launcher script"
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
        "Could not resolve one SDL2 library for this bsnes executable"
    );
    Ok(paths.into_iter().next().unwrap())
}

fn absolute_content(path: &std::ffi::OsStr) -> Result<PathBuf> {
    let content = PathBuf::from(path);
    ensure!(
        content.is_absolute()
            && !content
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            && content.is_file(),
        "bsnes needs the selected ROM's absolute file path"
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
        "bsnes automatic runtime discovery from a Lunchbox Flatpak still needs host-namespace integration"
    );
    ensure!(
        plan.environment.is_empty(),
        "Custom bsnes launcher environments need explicit runtime resolution"
    );
    ensure!(
        (1..=2).contains(&ids.len()),
        "bsnes supports one or two controller ports; assign players in Controller setup"
    );
    let players: Vec<Player> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| Player {
            player: u8::try_from(index + 1).unwrap(),
            controller_id: id.clone(),
        })
        .collect();
    let setup = match &option.executable {
        EmulatorExecutable::Native(program) => {
            ensure!(
                program == &plan.program,
                "Custom bsnes launcher environments need explicit runtime resolution"
            );
            ensure!(
                plan.arguments.len() == 1,
                "bsnes guided launch needs exactly the ROM argument"
            );
            SavedSetup {
                emulator_id: option.emulator_id.clone(),
                content: absolute_content(&plan.arguments[0])?,
                probe_program: helper()?,
                sdl_library: native_sdl_library(&program.canonicalize()?, cancel)?,
                executable_sha256: lunchbox_controller_probe::file_hash(program)?,
                players,
            }
        }
        EmulatorExecutable::Flatpak { command, app_id } => {
            ensure!(
                app_id == flatpak::APP_ID,
                "bsnes automatic discovery supports only the exact Flathub application"
            );
            ensure!(
                plan.arguments.len() == 4
                    && plan.arguments[0] == "run"
                    && plan.arguments[2] == flatpak::APP_ID,
                "bsnes Flatpak launch requires the ordinary one-ROM argument plan"
            );
            let grant = plan.arguments[1]
                .to_str()
                .and_then(|value| value.strip_prefix("--filesystem="))
                .context("bsnes Flatpak launch is missing its ROM-directory grant")?;
            ensure!(
                !grant.contains(':')
                    && Path::new(grant).canonicalize()? == plan.current_directory.canonicalize()?,
                "bsnes Flatpak launch grants a directory other than its exact launch directory"
            );
            let content = absolute_content(&plan.arguments[3])?;
            ensure!(
                content
                    .parent()
                    .is_some_and(|parent| parent.canonicalize().ok().as_ref()
                        == plan.current_directory.canonicalize().ok().as_ref()),
                "bsnes ROM must sit directly in its launch directory"
            );
            let location = flatpak_output(command, &["info", "--show-location", flatpak::APP_ID])?;
            let executable = PathBuf::from(location).join("files").join("bin/bsnes");
            ensure!(
                executable.is_file(),
                "bsnes Flatpak executable could not be located"
            );
            let runtime = flatpak_output(command, &["info", "--show-runtime", flatpak::APP_ID])?;
            let runtime_location = flatpak_output(command, &["info", "--show-location", &runtime])?;
            let sdl_library = PathBuf::from(runtime_location)
                .join("files")
                .join("lib")
                .join(format!("{}-linux-gnu", std::env::consts::ARCH))
                .join("libSDL2-2.0.so.0");
            ensure!(
                sdl_library.is_file(),
                "bsnes Flatpak SDL2 library could not be located"
            );
            SavedSetup {
                emulator_id: option.emulator_id.clone(),
                content,
                probe_program: helper()?,
                sdl_library,
                executable_sha256: lunchbox_controller_probe::file_hash(&executable)?,
                players,
            }
        }
        EmulatorExecutable::Wine { .. } => {
            anyhow::bail!("bsnes automatic discovery needs a native or Flatpak build, not Wine")
        }
    };
    setup.validate()?;
    cancelled(cancel)?;
    Ok(setup)
}
