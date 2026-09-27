//! Per-game native arcade preferences. Overrides live in a private MAME
//! session, never in the ROM archive or a hand-edited emulator save file.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

use crate::emulator::{EmulatorExecutable, LaunchPlan};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum BloodMode {
    #[default]
    Game,
    Red,
    Censored,
}

impl BloodMode {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        match value {
            "game" => Ok(Self::Game),
            "red" => Ok(Self::Red),
            "censored" => Ok(Self::Censored),
            _ => anyhow::bail!("Unknown arcade blood preference"),
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Red => "red",
            Self::Censored => "censored",
        }
    }
}

/// Limit the UI to the Neo Geo Metal Slug releases and known MAME clones.
/// Atomiswave (Metal Slug 6), console ports and arbitrary renamed archives
/// are not compatible. Runtime ROM metadata is independently validated.
pub(crate) fn blood_available(content: &Path) -> bool {
    matches!(
        content.extension().and_then(|ext| ext.to_str()),
        Some("zip" | "7z" | "cmd")
    ) && matches!(
        content.file_stem().and_then(|name| name.to_str()),
        Some(
            "mslug"
                | "mslug2"
                | "mslug2t"
                | "mslugx"
                | "mslug3"
                | "mslug3h"
                | "mslug3a"
                | "mslug3b6"
                | "mslug4"
                | "mslug4h"
                | "mslug5"
                | "mslug5h"
        )
    )
}

pub(crate) struct ArcadeSession {
    _directory: tempfile::TempDir,
}

impl ArcadeSession {
    pub(crate) fn attach(
        plan: &mut LaunchPlan,
        executable: &EmulatorExecutable,
        mode: BloodMode,
        calibrated: Option<&mut crate::controller_launch::CalibratedLaunch>,
    ) -> Result<Option<Self>> {
        if mode == BloodMode::Game {
            return Ok(None);
        }
        let content = &plan
            .retroarch_content
            .as_ref()
            .context("Missing arcade content")?
            .content;
        ensure!(
            blood_available(content),
            "Blood preference is not supported for this arcade machine"
        );
        let base = crate::app_paths::project_dirs()
            .context("Finding Lunchpail arcade settings directory")?
            .data_local_dir()
            .join("launch-display");
        // This directory is shared with RetroArch Flatpak; /tmp may not be.
        std::fs::create_dir_all(&base)?;
        let directory = tempfile::Builder::new()
            .prefix("arcade-settings-")
            .tempdir_in(base)?;
        let (staged, files) = stage(plan, executable, mode, directory.path())?;
        if let Some(session) = calibrated {
            session.retain_mame_session_transform(plan, &staged, &files)?;
        }
        *plan = staged;
        Ok(Some(Self {
            _directory: directory,
        }))
    }
}

fn stage(
    plan: &LaunchPlan,
    executable: &EmulatorExecutable,
    mode: BloodMode,
    directory: &Path,
) -> Result<(LaunchPlan, Vec<PathBuf>)> {
    let content = &plan
        .retroarch_content
        .as_ref()
        .context("Missing MAME content")?
        .content;
    let script = directory.join("neogeo-blood.lua");
    std::fs::write(
        &script,
        format!(
            "local adapter = (function()\n{}\nend)()\nadapter.start({})\n",
            include_str!("arcade_settings/neogeo_blood.lua"),
            mode == BloodMode::Red
        ),
    )?;
    let machine = content
        .file_stem()
        .and_then(|v| v.to_str())
        .context("Missing machine name")?;
    let remainder = match content.extension().and_then(|s| s.to_str()) {
        Some("cmd") => {
            use std::io::Read;
            let mut text = String::new();
            std::fs::File::open(content)?
                .take(4096)
                .read_to_string(&mut text)?;
            ensure!(text.len() <= 4095, "MAME command exceeds its size limit");
            let remainder = crate::mame_command::options(&text, machine)?;
            ensure!(
                !remainder.contains(['\n', '\r'])
                    && !remainder.split_whitespace().any(|arg| matches!(
                        arg,
                        "-autoboot_script" | "-script" | "-autoboot_delay"
                    )),
                "A custom MAME startup script is already configured; it has not been replaced"
            );
            remainder.to_owned()
        }
        Some("zip" | "7z") => format!(
            "-rompath {}",
            quote(content.parent().context("Missing ROM directory")?)?,
        ),
        _ => anyhow::bail!("Native arcade preferences require a MAME ROM archive"),
    };
    let command = crate::mame_command::build(
        machine,
        &format!(
            "-autoboot_delay 0 -autoboot_script {} {remainder}",
            quote(&script)?
        ),
    )?;
    let command_path = directory
        .join(content.file_stem().context("Missing machine name")?)
        .with_extension("cmd");
    std::fs::write(&command_path, command)?;
    let mut staged = plan.clone();
    let positions: Vec<_> = staged
        .arguments
        .iter()
        .enumerate()
        .filter_map(|(index, arg)| (arg == content.as_os_str()).then_some(index))
        .collect();
    ensure!(
        positions.len() == 1,
        "Arcade preferences require one exact content argument"
    );
    staged.arguments[positions[0]] = command_path.clone().into_os_string();
    staged.retroarch_content.as_mut().unwrap().content = command_path.clone();
    // The temporary command must not move saves out of their normal route.
    let config = directory.join("arcade.cfg");
    std::fs::write(
        &config,
        "savefiles_in_content_dir = \"false\"\nsavestates_in_content_dir = \"false\"\nsort_savefiles_enable = \"false\"\nsort_savestates_enable = \"false\"\nsort_savefiles_by_content_enable = \"false\"\nsort_savestates_by_content_enable = \"false\"\n",
    )?;
    crate::controller_launch::attach_config(&mut staged, executable, &config)?;
    Ok((staged, vec![script, command_path, config]))
}

fn quote(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .context("Arcade path is not UTF-8")?
        .replace('\\', "/");
    ensure!(
        !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | ';')),
        "Unsupported character in arcade path"
    );
    Ok(format!("\"{value}\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emulator::PreparedRetroarchContent;

    #[test]
    fn blood_scope_excludes_other_hardware_and_ports() {
        for name in ["mslug", "mslug2", "mslugx", "mslug3", "mslug4", "mslug5h"] {
            assert!(blood_available(Path::new(&format!("/roms/{name}.zip"))));
        }
        for name in ["mslug6", "mslug7", "Metal Slug", "mslug-malicious", "kof98"] {
            assert!(!blood_available(Path::new(&format!("/roms/{name}.zip"))));
        }
        assert!(BloodMode::parse("invalid").is_err());
        assert!(!blood_available(Path::new("/roms/mslug.iso")));
    }

    fn plan(content: &Path) -> LaunchPlan {
        LaunchPlan {
            emulator_name: "RetroArch".into(),
            program: "/retroarch".into(),
            arguments: vec![
                "-L".into(),
                "/cores/mame_libretro.so".into(),
                content.into(),
            ],
            current_directory: "/roms".into(),
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: Some(PreparedRetroarchContent {
                core: "/cores/mame_libretro.so".into(),
                content: content.into(),
            }),
        }
    }

    #[test]
    fn blood_session_preserves_controller_and_scheduled_resume_command() {
        let root = tempfile::tempdir().unwrap();
        let content = root.path().join("mslug.cmd");
        let original = "mame -state \"/saves/resume.sta\" -ctrlrpath \"/controllers\" -nvram_directory \"/saves/nvram\" -rompath \"/roms\" mslug\n";
        std::fs::write(&content, original).unwrap();
        let out = tempfile::tempdir().unwrap();
        let (staged, files) = stage(
            &plan(&content),
            &EmulatorExecutable::Native("/retroarch".into()),
            BloodMode::Red,
            out.path(),
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&content).unwrap(), original);
        let command_path = &staged.retroarch_content.unwrap().content;
        assert_eq!(command_path.file_name().unwrap(), "mslug.cmd");
        let command = std::fs::read_to_string(command_path).unwrap();
        assert!(command.starts_with("mslug -autoboot_delay "));
        assert!(
            command
                .trim()
                .ends_with(crate::mame_command::options(original, "mslug").unwrap())
        );
        assert!(command.contains("-autoboot_delay 0 -autoboot_script"));
        assert_eq!(files.len(), 3);
        assert!(
            std::fs::read_to_string(&files[0])
                .unwrap()
                .ends_with("adapter.start(true)\n")
        );
    }

    #[test]
    fn blood_session_rejects_competing_scripts_and_ambiguous_content() {
        let root = tempfile::tempdir().unwrap();
        let content = root.path().join("mslug.cmd");
        std::fs::write(&content, "mame -script custom.lua mslug").unwrap();
        let out = tempfile::tempdir().unwrap();
        let exe = EmulatorExecutable::Native("/retroarch".into());
        assert!(stage(&plan(&content), &exe, BloodMode::Red, out.path()).is_err());
        let content = root.path().join("mslug.zip");
        let mut input = plan(&content);
        input.arguments.push(content.into());
        assert!(stage(&input, &exe, BloodMode::Red, out.path()).is_err());
        assert!(quote(Path::new("/bad\npath")).is_err());
    }

    #[test]
    fn blood_session_flatpak_keeps_rom_path_and_shared_session_files() {
        let root = tempfile::tempdir().unwrap();
        let content = root.path().join("mslug2.zip");
        let original = plan(&content);
        let mut input = original.clone();
        input.program = "/usr/bin/flatpak".into();
        input
            .arguments
            .splice(0..0, ["run".into(), "org.libretro.RetroArch".into()]);
        let out = tempfile::tempdir().unwrap();
        let exe = EmulatorExecutable::Flatpak {
            app_id: "org.libretro.RetroArch".into(),
            command: "/usr/bin/flatpak".into(),
        };
        let (staged, files) = stage(&input, &exe, BloodMode::Censored, out.path()).unwrap();
        let command = std::fs::read_to_string(&files[1]).unwrap();
        assert!(command.starts_with("mslug2 -autoboot_delay "));
        assert!(command.contains(&format!("-rompath {}", quote(root.path()).unwrap())));
        assert!(
            std::fs::read_to_string(&files[0])
                .unwrap()
                .ends_with("adapter.start(false)\n")
        );
        assert!(staged.arguments.contains(&"--appendconfig".into()));
        assert_eq!(
            staged
                .retroarch_content
                .unwrap()
                .content
                .file_stem()
                .unwrap(),
            "mslug2"
        );
        assert_eq!(input.retroarch_content, original.retroarch_content);
    }
}
