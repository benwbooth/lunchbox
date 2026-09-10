//! Launch-time discovery for the native Linux SDL frontend. No second JSON
//! setup is needed for its runtime paths. Settings review never calls this.
use super::settings::{Handheld, SavedSetup};
use crate::{
    controller_native_process::{cancelled, capture},
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Context, Result, ensure};
use std::{
    path::{Path, PathBuf},
    process::Command,
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

fn sdl_library(program: &Path, cancel: &AtomicBool) -> Result<PathBuf> {
    // Do not execute an opaque launcher to guess its library paths. The native
    // adapter also verifies this executable and the child's loaded SDL later.
    use std::io::Read;
    let mut header = [0_u8; 4];
    std::fs::File::open(program)?.read_exact(&mut header)?;
    ensure!(
        header == *b"\x7fELF",
        "Choose the actual mGBA SDL executable rather than a launcher script"
    );
    let (out, _) = capture(Command::new("ldd").arg(program), cancel)?;
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
        "Could not resolve one SDL2 library for this mGBA executable"
    );
    Ok(paths.into_iter().next().unwrap())
}

pub(crate) fn discover(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
    controller_id: &str,
    gba: bool,
    cancel: &AtomicBool,
) -> Result<SavedSetup> {
    cancelled(cancel)?;
    ensure!(
        !crate::platform_process::is_flatpak(),
        "mGBA automatic runtime discovery from a Lunchbox Flatpak still needs host-namespace integration"
    );
    let EmulatorExecutable::Native(program) = &option.executable else {
        anyhow::bail!(
            "mGBA automatic discovery currently supports the native Linux SDL frontend; Flatpak and Qt need their own runtime integration"
        );
    };
    ensure!(
        plan.environment.is_empty() && program == &plan.program,
        "Custom mGBA launcher environments need explicit runtime resolution"
    );
    let content: Vec<_> = plan
        .arguments
        .iter()
        .filter(|arg| !matches!(arg.to_str(), Some("-f" | "--fullscreen")))
        .collect();
    ensure!(
        content.len() == 1,
        "mGBA guided launch needs one ROM plus optional fullscreen"
    );
    let content = PathBuf::from(content[0]);
    ensure!(
        content.is_absolute() && content.is_file(),
        "mGBA needs the selected ROM's absolute file path"
    );
    let cwd = plan.current_directory.canonicalize()?;
    let source_config = super::native_command::native_config(&cwd)?;
    ensure!(
        source_config.is_file(),
        "Open the mGBA SDL frontend once to create config.ini, then launch through Lunchbox again"
    );
    let probe_program = std::env::current_exe()?
        .parent()
        .context("Missing Lunchbox application directory")?
        .join("lunchbox-controller-probe");
    ensure!(
        probe_program.is_file(),
        "This Lunchbox installation is missing lunchbox-controller-probe; install the full package"
    );
    let sdl_library = sdl_library(&program.canonicalize()?, cancel)?;
    let setup = SavedSetup {
        emulator_id: option.emulator_id.clone(),
        content,
        handheld: if gba {
            Handheld::Gba
        } else {
            Handheld::Gameboy
        },
        controller_id: controller_id.to_owned(),
        source_config,
        probe_program,
        sdl_library,
        bubblewrap_program: executable_on_path("bwrap")?,
        // This records the discovered file identity, not a compatibility result.
        // native_command checks the version/frontend and live input handoff.
        executable_sha256: lunchbox_controller_probe::file_hash(program)?,
    };
    setup.validate()?;
    cancelled(cancel)?;
    Ok(setup)
}
