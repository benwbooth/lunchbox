//! MAME's direct libretro unserialize bypasses its startup timer barrier.
//! Feed the same state to MAME's scheduled `-state` loader instead. It waits
//! for scheduler.can_save(), preventing anonymous startup timers from undoing
//! a successful restore. No game-specific delay, second load, or save migration.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use crate::emulator::{EmulatorExecutable, LaunchPlan};

const MAX_STATE: usize = 256 * 1024 * 1024;
const MAME_HEADER: usize = 32;

pub(crate) struct MameResume {
    _directory: tempfile::TempDir,
}

impl MameResume {
    /// Keep this owner alive until the frontend exits, just like controller
    /// session files. Errors stop the launch rather than risk overwriting an
    /// unreadable resume point with a newly booted game's exit state.
    pub(crate) fn attach(
        plan: &mut LaunchPlan,
        executable: &EmulatorExecutable,
        core_name: &str,
        observation: Option<&super::AutoSaveObservation>,
        calibrated: Option<&mut crate::controller_launch::CalibratedLaunch>,
    ) -> Result<Option<Self>> {
        if core_name != "mame" {
            return Ok(None);
        }
        let Some(state_path) = observation.and_then(|value| value.resume_state()) else {
            return Ok(None);
        };
        let base = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
            .context("Finding Lunchbox resume directory")?
            .data_local_dir()
            .join("launch-display");
        // The application data directory is also visible inside RetroArch's
        // Flatpak. /tmp is not necessarily shared with that sandbox.
        std::fs::create_dir_all(&base)?;
        let directory = tempfile::Builder::new()
            .prefix("mame-resume-")
            .tempdir_in(base)?;
        let (staged, files) = stage(plan, executable, state_path, directory.path())?;
        if let Some(session) = calibrated {
            session.retain_mame_resume(plan, &staged, &files)?;
        }
        *plan = staged;
        eprintln!(
            "LUNCHBOX_MAME_RESUME: scheduled native restore from {}",
            state_path.display()
        );
        Ok(Some(Self {
            _directory: directory,
        }))
    }
}

fn stage(
    plan: &LaunchPlan,
    executable: &EmulatorExecutable,
    state_path: &Path,
    directory: &Path,
) -> Result<(LaunchPlan, Vec<PathBuf>)> {
    let content = &plan
        .retroarch_content
        .as_ref()
        .context("Missing MAME content")?
        .content;
    let stem = content
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid MAME content name")?;
    let source = read_bounded(std::fs::File::open(state_path)?, MAX_STATE)?;
    let raw = decode_state(&source)
        .context("Reading the existing MAME auto-save; it has not been changed")?;
    let machine = raw[10..28]
        .split(|byte| *byte == 0)
        .next()
        .unwrap_or_default();
    ensure!(
        machine == stem.as_bytes(),
        "MAME save belongs to a different machine than {stem}"
    );
    let native = directory.join("resume.sta");
    let mut file = std::fs::File::create(&native)?;
    file.write_all(&raw[..MAME_HEADER])?;
    let mut encoder = flate2::write::ZlibEncoder::new(file, flate2::Compression::fast());
    encoder.write_all(&raw[MAME_HEADER..])?;
    encoder.finish()?.sync_all()?;
    let command = command_for(content, &native)?;
    // Same basename means RetroArch continues saving <game>.state.auto, not a
    // temporary session name. Only the startup load uses the transient .sta.
    let command_path = directory.join(format!("{stem}.cmd"));
    std::fs::write(&command_path, command)?;
    let config = directory.join("resume.cfg");
    std::fs::write(
        &config,
        "savestate_auto_load = \"false\"\nsavestates_in_content_dir = \"false\"\nsort_savestates_enable = \"false\"\nsort_savestates_by_content_enable = \"false\"\n",
    )?;
    let mut staged = plan.clone();
    let positions: Vec<_> = staged
        .arguments
        .iter()
        .enumerate()
        .filter_map(|(index, arg)| (arg == content.as_os_str()).then_some(index))
        .collect();
    ensure!(
        positions.len() == 1,
        "MAME resume requires one exact content argument"
    );
    staged.arguments[positions[0]] = command_path.clone().into_os_string();
    staged.retroarch_content.as_mut().unwrap().content = command_path.clone();
    crate::controller_launch::attach_config(&mut staged, executable, &config)?;
    Ok((staged, vec![native, command_path, config]))
}

fn quote(path: &Path) -> Result<String> {
    // MAME's libretro command parser treats backslashes literally; forward
    // slashes work for drive-qualified Windows paths too.
    let value = path
        .to_str()
        .context("MAME resume path is not UTF-8")?
        .replace('\\', "/");
    ensure!(
        !value.chars().any(|c| c.is_control() || c == '"'),
        "Invalid MAME resume path"
    );
    Ok(format!("\"{value}\""))
}

fn command_for(content: &Path, state: &Path) -> Result<String> {
    let remainder = match content.extension().and_then(|s| s.to_str()) {
        Some("cmd") => {
            let text = String::from_utf8(read_bounded(std::fs::File::open(content)?, 4095)?)?;
            let text = text.trim();
            let remainder = text
                .strip_prefix("mame ")
                .context("Unsupported MAME command file for automatic resume")?;
            ensure!(
                !remainder.contains(['\n', '\r'])
                    && !remainder
                        .split_whitespace()
                        .any(|word| matches!(word, "-state" | "-autosave")),
                "MAME command already selects a native startup state"
            );
            remainder.to_owned()
        }
        Some("zip" | "7z") => {
            let stem = content
                .file_stem()
                .and_then(|s| s.to_str())
                .context("Invalid MAME machine")?;
            ensure!(
                !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
                "Invalid MAME machine name"
            );
            format!(
                "-rompath {} {stem}",
                quote(content.parent().context("Missing MAME ROM directory")?)?
            )
        }
        _ => bail!("Automatic MAME resume requires a ROM archive or a Lunchbox MAME command"),
    };
    let command = format!("mame -state {} -noautosave {remainder}\n", quote(state)?);
    ensure!(
        command.len() <= 4095,
        "MAME resume command exceeds the core's command limit"
    );
    Ok(command)
}

fn read_bounded(reader: impl Read, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= limit, "Save state exceeds its size limit");
    Ok(bytes)
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<usize> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .context("Truncated save state")?
            .try_into()?,
    ) as usize)
}

fn decode_state(input: &[u8]) -> Result<Vec<u8>> {
    ensure!(input.len() <= MAX_STATE, "Save state is too large");
    let unpacked;
    let bytes = if input.starts_with(b"#RZIPv") {
        ensure!(
            input.len() >= 20 && input[7] == b'#',
            "Invalid RZIP save header"
        );
        let chunk_size = u32_at(input, 8)?;
        ensure!(
            chunk_size > 0 && chunk_size <= 64 * 1024 * 1024,
            "Invalid RZIP chunk size"
        );
        let size = u64::from_le_bytes(input[12..20].try_into()?);
        ensure!(
            size > 0 && size <= MAX_STATE as u64,
            "Invalid RZIP expanded size"
        );
        let mut output = Vec::new();
        let mut offset = 20;
        while offset < input.len() {
            let length = u32_at(input, offset)?;
            ensure!(
                length <= MAX_STATE,
                "RZIP chunk exceeds the input size limit"
            );
            offset += 4;
            let compressed = input
                .get(offset..offset + length)
                .context("Truncated RZIP chunk")?;
            offset += length;
            let limit = chunk_size.min(size as usize - output.len());
            let chunk = match input[6] {
                1 => read_bounded(flate2::read::ZlibDecoder::new(compressed), limit)?,
                2 => read_bounded(zstd::stream::read::Decoder::new(compressed)?, limit)?,
                _ => bail!("Unsupported RZIP compression version"),
            };
            ensure!(!chunk.is_empty(), "Empty RZIP chunk");
            output.extend(chunk);
        }
        ensure!(
            output.len() == size as usize,
            "RZIP save size does not match its header"
        );
        unpacked = output;
        unpacked.as_slice()
    } else {
        input
    };
    let raw = if bytes.starts_with(b"RASTATE") {
        ensure!(
            bytes.get(7) == Some(&1),
            "Unsupported RetroArch state version"
        );
        let mut offset = 8;
        let mut memory = None;
        loop {
            let tag = bytes
                .get(offset..offset + 4)
                .context("Missing RetroArch state terminator")?;
            let length = u32_at(bytes, offset + 4)?;
            ensure!(
                length <= MAX_STATE,
                "RetroArch state block exceeds its size limit"
            );
            offset += 8;
            let block = bytes
                .get(offset..offset + length)
                .context("Truncated RetroArch state block")?;
            if tag == b"END " {
                ensure!(length == 0, "Invalid state terminator");
                break;
            }
            if tag == b"MEM " {
                ensure!(memory.is_none(), "Duplicate core state block");
                memory = Some(block);
            }
            offset += (length + 7) & !7;
        }
        memory.context("RetroArch state has no core memory block")?
    } else {
        bytes
    };
    ensure!(
        raw.len() > MAME_HEADER && raw.starts_with(b"MAMESAVE") && raw[8] == 2,
        "Not a supported MAME save state"
    );
    Ok(raw.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emulator::PreparedRetroarchContent;

    fn raw_state() -> Vec<u8> {
        let mut bytes = vec![0; MAME_HEADER];
        bytes[..8].copy_from_slice(b"MAMESAVE");
        bytes[8] = 2;
        bytes[10..15].copy_from_slice(b"mslug");
        bytes.extend((0..400).map(|v| (v % 251) as u8));
        bytes
    }

    fn block(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
        out.extend(tag);
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out.resize((out.len() + 7) & !7, 0);
    }

    fn envelope(raw: &[u8]) -> Vec<u8> {
        let mut bytes = b"RASTATE\x01".to_vec();
        block(&mut bytes, b"RPLY", b"replay");
        block(&mut bytes, b"MEM ", raw);
        block(&mut bytes, b"ACHV", b"achievements");
        block(&mut bytes, b"END ", b"");
        bytes
    }

    fn compressed(bytes: &[u8], version: u8) -> Vec<u8> {
        let mut out = b"#RZIPv".to_vec();
        out.extend([version, b'#']);
        out.extend(128u32.to_le_bytes());
        out.extend((bytes.len() as u64).to_le_bytes());
        for chunk in bytes.chunks(128) {
            let compressed = if version == 1 {
                let mut writer =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
                writer.write_all(chunk).unwrap();
                writer.finish().unwrap()
            } else {
                zstd::stream::encode_all(chunk, 1).unwrap()
            };
            out.extend((compressed.len() as u32).to_le_bytes());
            out.extend(compressed);
        }
        out
    }

    #[test]
    fn decodes_raw_enveloped_deflate_and_zstd_without_changing_core_state() {
        let raw = raw_state();
        let wrapped = envelope(&raw);
        for bytes in [
            raw.clone(),
            wrapped.clone(),
            compressed(&wrapped, 1),
            compressed(&wrapped, 2),
            compressed(&raw, 1),
        ] {
            assert_eq!(decode_state(&bytes).unwrap(), raw);
        }
    }

    #[test]
    fn rejects_corrupt_truncated_oversized_or_duplicate_state_blocks() {
        let raw = raw_state();
        let wrapped = envelope(&raw);
        let compressed = compressed(&wrapped, 1);
        for length in [0, 7, 12, 19, 20, compressed.len() - 1] {
            assert!(decode_state(&compressed[..length]).is_err());
        }
        let mut oversized = compressed.clone();
        oversized[12..20].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(decode_state(&oversized).is_err());
        let mut duplicate = b"RASTATE\x01".to_vec();
        block(&mut duplicate, b"MEM ", &raw);
        block(&mut duplicate, b"MEM ", &raw);
        block(&mut duplicate, b"END ", b"");
        assert!(decode_state(&duplicate).is_err());
        assert!(decode_state(&wrapped[..wrapped.len() - 8]).is_err());
        assert!(decode_state(b"some other core's state").is_err());
    }

    fn plan(content: &Path, flatpak: bool) -> (LaunchPlan, EmulatorExecutable) {
        let core = PathBuf::from("/cores/mame_libretro.so");
        let mut arguments = vec![
            "-L".into(),
            core.clone().into_os_string(),
            content.as_os_str().to_owned(),
        ];
        let executable = if flatpak {
            arguments.splice(0..0, ["run".into(), "org.libretro.RetroArch".into()]);
            EmulatorExecutable::Flatpak {
                app_id: "org.libretro.RetroArch".into(),
                command: "/usr/bin/flatpak".into(),
            }
        } else {
            EmulatorExecutable::Native("/retroarch".into())
        };
        (
            LaunchPlan {
                emulator_name: "RetroArch".into(),
                program: "/retroarch".into(),
                arguments,
                current_directory: content.parent().unwrap().to_owned(),
                environment: vec![],
                cleanup_paths: vec![],
                retroarch_content: Some(PreparedRetroarchContent {
                    core,
                    content: content.to_owned(),
                }),
            },
            executable,
        )
    }

    #[test]
    fn stages_native_and_flatpak_resume_without_rewriting_the_original_save() {
        for flatpak in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let content = root.path().join("mslug.zip");
            let state = root.path().join("mslug.state.auto");
            let original = compressed(&envelope(&raw_state()), 1);
            std::fs::write(&state, &original).unwrap();
            let (plan, executable) = plan(&content, flatpak);
            let (staged, _) = stage(&plan, &executable, &state, root.path()).unwrap();
            assert_eq!(plan.retroarch_content.unwrap().content, content);
            assert_eq!(
                staged.retroarch_content.unwrap().content,
                root.path().join("mslug.cmd")
            );
            assert_eq!(std::fs::read(&state).unwrap(), original);
            let native = std::fs::read(root.path().join("resume.sta")).unwrap();
            let mut restored = native[..MAME_HEADER].to_vec();
            restored.extend(
                read_bounded(
                    flate2::read::ZlibDecoder::new(&native[MAME_HEADER..]),
                    MAX_STATE,
                )
                .unwrap(),
            );
            assert_eq!(restored, raw_state());
            let config = std::fs::read_to_string(root.path().join("resume.cfg")).unwrap();
            assert!(config.contains("savestate_auto_load = \"false\""));
            assert!(!config.contains("savestate_auto_save"));
            let command = std::fs::read_to_string(root.path().join("mslug.cmd")).unwrap();
            assert!(command.contains("-state "));
            assert!(command.ends_with("mslug\n"));
            if flatpak {
                assert!(
                    staged
                        .arguments
                        .iter()
                        .any(|arg| arg.to_string_lossy().starts_with("--filesystem="))
                );
            }
        }
    }

    #[test]
    fn preserves_native_controller_and_persistence_arguments() {
        let root = tempfile::tempdir().unwrap();
        let command = root.path().join("mslug.cmd");
        let original = "mame -cfg_directory \"/private/cfg\" -ctrlr lunchbox-original -nvram_directory \"/saves/nvram\" -noautosave mslug\n";
        std::fs::write(&command, original).unwrap();
        let resumed = command_for(&command, &root.path().join("resume.sta")).unwrap();
        assert!(resumed.ends_with(original.strip_prefix("mame ").unwrap()));
        assert_eq!(std::fs::read_to_string(&command).unwrap(), original);
        std::fs::write(&command, "mame -state old mslug").unwrap();
        assert!(command_for(&command, &root.path().join("resume.sta")).is_err());
    }

    #[test]
    fn skips_other_cores_and_disabled_or_missing_resume_points() {
        let (mut plan, executable) = plan(Path::new("/roms/mslug.zip"), false);
        let original = plan.clone();
        assert!(
            MameResume::attach(&mut plan, &executable, "mame", None, None)
                .unwrap()
                .is_none()
        );
        let observation = super::super::AutoSaveObservation {
            state: "/missing.state.auto".into(),
            sram: "/missing.srm".into(),
            state_before: Some(std::time::SystemTime::now()),
            sram_before: None,
            auto_state_load_enabled: false,
            auto_state_save_enabled: true,
        };
        assert!(
            MameResume::attach(&mut plan, &executable, "mame", Some(&observation), None)
                .unwrap()
                .is_none()
        );
        let observation = super::super::AutoSaveObservation {
            auto_state_load_enabled: true,
            ..observation
        };
        assert!(
            MameResume::attach(&mut plan, &executable, "fbneo", Some(&observation), None)
                .unwrap()
                .is_none()
        );
        assert_eq!(plan, original);
    }

    #[test]
    fn wrong_machine_state_does_not_alter_launch() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("other.state.auto");
        std::fs::write(&state, raw_state()).unwrap();
        let (plan, executable) = plan(&root.path().join("other.zip"), false);
        let original = plan.clone();
        assert!(stage(&plan, &executable, &state, root.path()).is_err());
        assert_eq!(plan, original);
    }
}
