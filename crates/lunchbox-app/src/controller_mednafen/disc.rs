//! Native disc dependencies: CloneCD and CUE track files.
//! Pin: f0ee9d595db68ad5247ba5ac6a8367fdced9c3fc/cdrom/CDAccess_CCD.cpp.
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

struct File {
    path: PathBuf,
    canonical: PathBuf,
    hash: String,
    size: u64,
}

pub(crate) struct Snapshot {
    files: Vec<File>,
}

impl Snapshot {
    pub(crate) fn capture(path: &Path) -> Result<Self> {
        match path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "ccd" => Self::ccd(path),
            "cue" => Self::cue(path),
            _ => anyhow::bail!("Mednafen calibrated disc launch requires CCD or CUE content"),
        }
    }

    fn cue(path: &Path) -> Result<Self> {
        ensure!(path.is_absolute(), "Mednafen disc path must be absolute");
        ensure!(
            std::fs::metadata(path)?.len() <= 16 * 1024 * 1024,
            "CUE descriptor exceeds preparation limit"
        );
        let descriptor_hash = file_hash(path)?;
        let text = std::fs::read_to_string(path)?;
        ensure!(!text.contains('\0'), "CUE descriptor contains NUL");
        let mut paths = BTreeSet::from([path.to_owned()]);
        for line in text.trim_start_matches('\u{feff}').lines() {
            let mut rest = line.trim_matches(|c: char| c.is_ascii_whitespace());
            if !cue_arg(&mut rest, false).eq_ignore_ascii_case("FILE") {
                continue;
            }
            let name = cue_arg(&mut rest, true);
            let format = cue_arg(&mut rest, true);
            ensure!(!name.is_empty(), "CUE FILE directive has no filename");
            ensure!(
                [
                    "BINARY", "OGG", "VORBIS", "WAVE", "WAV", "PCM", "MPC", "MP+"
                ]
                .iter()
                .any(|value| format.eq_ignore_ascii_case(value)),
                "Unsupported CUE track format"
            );
            paths.insert(path.parent().unwrap().join(name));
        }
        ensure!(paths.len() > 1, "CUE descriptor has no track dependencies");
        let mut files = Vec::new();
        for path in paths {
            let metadata = std::fs::metadata(&path)?;
            ensure!(metadata.is_file(), "CUE dependency is not a regular file");
            files.push(File {
                canonical: path.canonicalize()?,
                hash: file_hash(&path)?,
                size: metadata.len(),
                path,
            });
        }
        ensure!(
            file_hash(path)? == descriptor_hash,
            "CUE descriptor changed while reading dependencies"
        );
        let snapshot = Self { files };
        snapshot.verify()?;
        Ok(snapshot)
    }

    /// Capture all required companions, not merely the small CCD descriptor.
    /// Native CCD mirrors the descriptor extension's case character by character.
    pub(crate) fn ccd(path: &Path) -> Result<Self> {
        ensure!(path.is_absolute(), "Mednafen disc path must be absolute");
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        ensure!(
            extension.eq_ignore_ascii_case("ccd"),
            "Expected a CloneCD descriptor"
        );
        let companion = |suffix: &str| {
            let mirrored: String = suffix
                .bytes()
                .zip(extension.bytes())
                .map(|(byte, case)| {
                    char::from(if case.is_ascii_uppercase() {
                        byte.to_ascii_uppercase()
                    } else {
                        byte
                    })
                })
                .collect();
            path.with_extension(mirrored)
        };
        let mut files = Vec::new();
        let mut identities = BTreeSet::new();
        for path in [path.to_owned(), companion("img"), companion("sub")] {
            let canonical = path.canonicalize()?;
            let metadata = std::fs::metadata(&path)?;
            ensure!(
                metadata.is_file() && identities.insert(canonical.clone()),
                "Disc dependencies must be distinct regular files"
            );
            let size = metadata.len();
            ensure!(
                size <= 0x7fffffff,
                "CloneCD dependency exceeds native size limit"
            );
            files.push(File {
                hash: file_hash(&path)?,
                path,
                canonical,
                size,
            });
        }
        ensure!(
            files[0].size <= 16 * 1024 * 1024,
            "CCD descriptor exceeds preparation limit"
        );
        ensure!(
            files[1].size > 0 && files[1].size % 2352 == 0,
            "CCD image has invalid sector size"
        );
        ensure!(
            files[2].size == files[1].size / 2352 * 96,
            "CCD subchannel size differs from image sector count"
        );
        let snapshot = Self { files };
        snapshot.verify()?;
        Ok(snapshot)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        for file in &self.files {
            let metadata = std::fs::metadata(&file.path)?;
            ensure!(
                metadata.is_file()
                    && metadata.len() == file.size
                    && file.path.canonicalize()? == file.canonical
                    && file_hash(&file.path)? == file.hash,
                "Mednafen disc dependency changed during preparation"
            );
        }
        Ok(())
    }
}

/// CDAccess_Image.cpp UnQuotify: quotes terminate the argument, without escapes.
fn cue_arg(rest: &mut &str, quotes: bool) -> String {
    let source = *rest;
    let mut quoted = false;
    let mut normal = false;
    let mut value = String::new();
    let mut end = source.len();
    for (offset, ch) in source.char_indices() {
        if matches!(ch, ' ' | '\t') && !quoted {
            if normal {
                end = offset;
                break;
            }
            continue;
        }
        if ch == '"' && quotes {
            if quoted {
                end = offset + 1;
                break;
            }
            quoted = true;
        } else {
            value.push(ch);
            normal = true;
        }
    }
    *rest = source[end..].trim_start_matches([' ', '\t']);
    value
}
