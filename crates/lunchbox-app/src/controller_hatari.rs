//! Read-only prerequisites for Hatari's explicit libretro input contracts.
//! This does not install firmware or rewrite the user's native configuration.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Eq, PartialEq)]
struct FileState {
    canonical: PathBuf,
    sha256: [u8; 32],
}

fn inspect_file(
    path: &Path,
    maximum: u64,
    required: bool,
    validate: impl FnOnce(&[u8]) -> Result<()>,
) -> Result<Option<FileState>> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !required => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("Inspecting Hatari input prerequisite {}", path.display())
            });
        }
        Ok(_) => {}
    }
    let canonical = path
        .canonicalize()
        .with_context(|| format!("Resolving {}", path.display()))?;
    let file = std::fs::File::open(&canonical)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= maximum && (!required || metadata.len() > 0),
        "Hatari prerequisite must be a bounded regular file: {}",
        path.display()
    );
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= maximum && bytes.len() as u64 == metadata.len(),
        "Hatari prerequisite changed while reading: {}",
        path.display()
    );
    ensure!(
        path.canonicalize()? == canonical,
        "Hatari prerequisite was redirected while reading"
    );
    validate(&bytes)?;
    Ok(Some(FileState {
        canonical,
        sha256: Sha256::digest(&bytes).into(),
    }))
}

/// Require user settings that override the build-specific global configuration.
/// Hatari loads the first matching section, with the last assignment winning.
/// Accept a deliberately unambiguous subset instead of guessing at malformed INI.
fn validate_extra_ports(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).context("Hatari configuration must be UTF-8")?;
    ensure!(
        !text.starts_with('\u{feff}') && !text.contains('\0'),
        "Hatari configuration contains an ambiguous BOM or NUL"
    );
    let mut seen = [false; 4];
    let mut disabled = [false; 4];
    let mut section = None;
    for line in text.split_inclusive('\n') {
        // The core uses fgets with a 1024-byte buffer. Do not let a long line
        // become a second assignment or header in the core but not here.
        ensure!(
            line.len() < 1023,
            "Hatari configuration line exceeds the supported parser limit"
        );
        let line = line.trim_matches(|c: char| c.is_ascii_whitespace());
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            ensure!(
                line.ends_with(']') && !line[1..line.len() - 1].contains(['[', ']']),
                "Hatari configuration has an ambiguous section header"
            );
            section = match line {
                "[Joystick2]" => Some(0),
                "[Joystick3]" => Some(1),
                "[Joystick4]" => Some(2),
                "[Joystick5]" => Some(3),
                _ => None,
            };
            if let Some(index) = section {
                ensure!(
                    !seen[index],
                    "Hatari configuration repeats an extra joystick section"
                );
                seen[index] = true;
            }
        } else if let Some(index) = section {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim_matches(|c: char| c.is_ascii_whitespace()) == "nJoystickMode" {
                    ensure!(
                        value.trim_matches(|c: char| c.is_ascii_whitespace()) == "0",
                        "Hatari extra joystick ports 2 through 5 must be explicitly disabled (nJoystickMode = 0); saved settings were not changed"
                    );
                    disabled[index] = true;
                }
            }
        }
    }
    ensure!(
        disabled.into_iter().all(|value| value),
        "Hatari needs explicit nJoystickMode = 0 in each [Joystick2] through [Joystick5] section of hatari.cfg to override global input settings; use native setup to configure these ports"
    );
    Ok(())
}

pub(crate) struct InputSnapshot {
    system: PathBuf,
    save: PathBuf,
    canonical_system: PathBuf,
    canonical_save: PathBuf,
    bios: Option<FileState>,
    user_config: Option<FileState>,
}

impl InputSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.system.is_dir()
                && self.save.is_dir()
                && self.system.canonicalize()? == self.canonical_system
                && self.save.canonicalize()? == self.canonical_save,
            "Hatari system or save directory changed during input preparation"
        );
        ensure!(
            inspect_file(
                &self.system.join("tos.img"),
                16 * 1024 * 1024,
                true,
                |_| Ok(())
            )? == self.bios,
            "Hatari TOS image changed during input preparation"
        );
        ensure!(
            inspect_file(
                &self.save.join("hatari.cfg"),
                1024 * 1024,
                true,
                validate_extra_ports
            )? == self.user_config,
            "Hatari native configuration changed during input preparation"
        );
        Ok(())
    }

    pub(crate) fn append_config(&self) -> Result<String> {
        let encode = |path: &Path| -> Result<String> {
            let value = path.to_str().context("Hatari directories must be UTF-8")?;
            ensure!(
                !value
                    .chars()
                    .any(|c| c.is_control() || matches!(c, '"' | '\\')),
                "Hatari directory cannot be represented in RetroArch configuration"
            );
            Ok(value.to_owned())
        };
        Ok(format!(
            "system_directory = \"{}\"\nsavefile_directory = \"{}\"\nsort_savefiles_enable = \"false\"\nsort_savefiles_by_content_enable = \"false\"\nsavefiles_in_content_dir = \"false\"\n",
            encode(&self.system)?,
            encode(&self.save)?
        ))
    }
}

pub(crate) fn prepare(system: &Path, save: &Path) -> Result<InputSnapshot> {
    ensure!(
        system.is_absolute() && system.is_dir() && save.is_absolute() && save.is_dir(),
        "Hatari input preparation requires existing absolute system and save directories"
    );
    let snapshot = InputSnapshot {
        system: system.to_owned(),
        save: save.to_owned(),
        canonical_system: system.canonicalize()?,
        canonical_save: save.canonicalize()?,
        bios: inspect_file(&system.join("tos.img"), 16 * 1024 * 1024, true, |_| Ok(()))?,
        user_config: inspect_file(
            &save.join("hatari.cfg"),
            1024 * 1024,
            true,
            validate_extra_ports,
        )?,
    };
    snapshot.append_config()?;
    snapshot.verify()?;
    Ok(snapshot)
}
