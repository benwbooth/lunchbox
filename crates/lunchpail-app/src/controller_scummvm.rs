//! Fixed ScummVM RetroPad-to-cursor/key contract, pinned to d79f8bb292c8.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq)]
struct FileStamp {
    canonical: PathBuf,
    digest: [u8; 32],
}

fn read_snapshot(
    path: &Path,
    maximum: u64,
    required: bool,
) -> Result<(Option<FileStamp>, Vec<u8>)> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !required => {
            return Ok((None, Vec::new()));
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Inspecting ScummVM input {}", path.display()));
        }
        Ok(_) => {}
    }
    let canonical = path.canonicalize()?;
    let file = std::fs::File::open(&canonical)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= maximum,
        "ScummVM input must be a bounded regular file: {}",
        path.display()
    );
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= maximum
            && bytes.len() as u64 == metadata.len()
            && path.canonicalize()? == canonical,
        "ScummVM input changed while reading"
    );
    let stamp = FileStamp {
        canonical,
        digest: Sha256::digest(&bytes).into(),
    };
    Ok((Some(stamp), bytes))
}

pub(crate) struct InputSnapshot {
    system: PathBuf,
    canonical_system: PathBuf,
    hook_path: PathBuf,
    hook: Option<FileStamp>,
    config: Option<FileStamp>,
}

impl InputSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.system.is_dir() && self.system.canonicalize()? == self.canonical_system,
            "ScummVM system directory changed during input preparation"
        );
        ensure!(
            read_snapshot(&self.hook_path, 64 * 1024, true)?.0 == self.hook,
            "ScummVM target hook changed during input preparation"
        );
        ensure!(
            read_snapshot(&self.system.join("scummvm.ini"), 8 * 1024 * 1024, false)?.0
                == self.config,
            "ScummVM native configuration changed during input preparation"
        );
        Ok(())
    }

    pub(crate) fn append_config(&self) -> Result<String> {
        let path = self
            .system
            .to_str()
            .context("ScummVM system directory must be UTF-8")?;
        ensure!(
            !path
                .chars()
                .any(|c| c.is_control() || matches!(c, '"' | '\\')),
            "ScummVM system directory cannot be represented in RetroArch configuration"
        );
        Ok(format!("system_directory = \"{path}\"\n"))
    }
}

pub(crate) fn prepare(system: &Path, hook_path: &Path) -> Result<InputSnapshot> {
    ensure!(
        system.is_absolute() && system.is_dir(),
        "ScummVM requires an existing absolute system directory"
    );
    let (hook, bytes) = read_snapshot(hook_path, 64 * 1024, true)?;
    validate_hook(hook_path, &bytes)?;
    let snapshot = InputSnapshot {
        system: system.to_owned(),
        canonical_system: system.canonicalize()?,
        hook_path: hook_path.to_owned(),
        hook,
        config: read_snapshot(&system.join("scummvm.ini"), 8 * 1024 * 1024, false)?.0,
    };
    snapshot.append_config()?;
    snapshot.verify()?;
    Ok(snapshot)
}

/// Validate the .scummvm target hook without executing the engine detector.
/// The core reads one line and inserts it into its own 400-byte command buffer.
pub(crate) fn validate_hook(path: &Path, bytes: &[u8]) -> Result<String> {
    let path_text = path.to_str().context("ScummVM hook path must be UTF-8")?;
    ensure!(
        path.is_absolute() && path_text.ends_with(".scummvm"),
        "This ScummVM contract requires an absolute lowercase .scummvm hook path"
    );
    let first_line = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    ensure!(
        first_line.len() < 399,
        "ScummVM hook target exceeds the core's first-line buffer"
    );
    let target = std::str::from_utf8(first_line)
        .context("ScummVM hook target must be UTF-8")?
        .trim_matches(|c: char| c.is_ascii_whitespace());
    ensure!(
        !target.is_empty()
            && !target.starts_with('-')
            && target
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric()
                    || matches!(byte, b'_' | b'-' | b':' | b'.')),
        "ScummVM hook must contain one plain target or engine:game identifier, not command arguments"
    );
    let parent = path
        .parent()
        .and_then(Path::to_str)
        .context("ScummVM hook requires a UTF-8 parent directory")?;
    ensure!(
        !parent
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | '\\')),
        "ScummVM game directory cannot be represented in the core command parser"
    );
    ensure!(
        parent.len() + target.len() + 6 < 400,
        "ScummVM game directory and target exceed the core's launch buffer"
    );
    Ok(target.to_owned())
}

pub(crate) fn validate_options(options: &BTreeMap<String, String>) -> Result<()> {
    const EXPECTED: &[(&str, &str)] = &[
        ("scummvm_mapper_up", "RETROKE_UP"),
        ("scummvm_mapper_down", "RETROKE_DOWN"),
        ("scummvm_mapper_left", "RETROKE_LEFT"),
        ("scummvm_mapper_right", "RETROKE_RIGHT"),
        ("scummvm_mapper_a", "RETROK_SPACE"),
        ("scummvm_mapper_b", "RETROK_RETURN"),
        ("scummvm_mapper_x", "RETROK_F5"),
        ("scummvm_mapper_y", "RETROK_ESCAPE"),
        ("scummvm_mapper_select", "RETROKE_VKBD"),
        ("scummvm_mapper_start", "RETROKE_SCUMMVM_GUI"),
        ("scummvm_mapper_l", "RETROKE_LEFT_BUTTON"),
        ("scummvm_mapper_r", "RETROKE_RIGHT_BUTTON"),
        ("scummvm_mapper_l2", "---"),
        ("scummvm_mapper_r2", "RETROKE_FINE_CONTROL"),
        ("scummvm_mapper_l3", "---"),
        ("scummvm_mapper_r3", "---"),
        ("scummvm_mapper_lu", "RETROKE_UP"),
        ("scummvm_mapper_ld", "RETROKE_DOWN"),
        ("scummvm_mapper_ll", "RETROKE_LEFT"),
        ("scummvm_mapper_lr", "RETROKE_RIGHT"),
        ("scummvm_mapper_ru", "RETROK_UP"),
        ("scummvm_mapper_rd", "RETROK_DOWN"),
        ("scummvm_mapper_rl", "RETROK_LEFT"),
        ("scummvm_mapper_rr", "RETROK_RIGHT"),
        ("scummvm_pointer_device", "retropad"),
        ("scummvm_gamepad_cursor_speed", "1.0"),
        ("scummvm_gamepad_cursor_acceleration_time", "0.2"),
        ("scummvm_analog_response", "linear"),
        ("scummvm_analog_deadzone", "15"),
        ("scummvm_mouse_fine_control_speed_reduction", "4"),
    ];
    for &(key, value) in EXPECTED {
        ensure!(
            options.get(key).map(String::as_str) == Some(value),
            "ScummVM fixed cursor contract requires {key} = {value}"
        );
    }
    ensure!(
        options
            .keys()
            .filter(|key| key.starts_with("scummvm_mapper_"))
            .count()
            == 24,
        "ScummVM fixed cursor contract must declare exactly 24 mapper slots"
    );
    Ok(())
}
