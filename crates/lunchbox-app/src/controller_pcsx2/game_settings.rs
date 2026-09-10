//! Native serial/CRC-first, legacy CRC-second game configuration selection.
use anyhow::{Result, ensure};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

struct Source {
    path: PathBuf,
    canonical: Option<PathBuf>,
    bytes: Option<Vec<u8>>,
}

fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        file.metadata()?.is_file(),
        "PCSX2 game settings is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 16 * 1024 * 1024,
        "PCSX2 game settings exceeds size limit"
    );
    Ok(Some(bytes))
}

pub(crate) struct Selection {
    sources: Vec<Source>,
    pub filename: String,
}

impl Selection {
    /// Serial/CRC must be derived from native content identity, not title/name.
    /// Empty serial supports PCSX2's ELF/legacy selection. CRC zero has no layer.
    pub(crate) fn capture(directory: &Path, serial: &str, crc: u32) -> Result<Self> {
        ensure!(
            directory.is_absolute(),
            "PCSX2 game-settings directory must be absolute"
        );
        ensure!(
            crc != 0,
            "PCSX2 CRC zero does not select a game-settings layer"
        );
        ensure!(
            serial.len() <= 128
                && serial
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')),
            "PCSX2 serial requires native sanitization before this mapping adapter"
        );
        let legacy = format!("{crc:08X}.ini");
        let filename = if serial.is_empty() {
            legacy.clone()
        } else {
            format!("{serial}_{crc:08X}.ini")
        };
        let mut candidates = vec![filename.clone()];
        if !serial.is_empty() {
            candidates.push(legacy);
        }
        let mut sources = Vec::new();
        for candidate in candidates {
            let path = directory.join(candidate);
            let bytes = read(&path)?;
            let canonical = if bytes.is_some() {
                Some(path.canonicalize()?)
            } else {
                None
            };
            let present = bytes.is_some();
            sources.push(Source {
                path,
                bytes,
                canonical,
            });
            if present {
                break;
            }
        }
        let selected = Self { sources, filename };
        selected.verify()?;
        Ok(selected)
    }

    /// Write the override under the preferred name even when the source was
    /// legacy; native selection then deterministically chooses the owned copy.
    pub(crate) fn overlay(&self, profile: &str) -> Result<String> {
        self.verify()?;
        let bytes = self
            .sources
            .last()
            .and_then(|source| source.bytes.as_deref())
            .unwrap_or_default();
        super::configuration::with_game_controller_profile(std::str::from_utf8(bytes)?, profile)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        for source in &self.sources {
            ensure!(
                read(&source.path)? == source.bytes,
                "PCSX2 game settings changed during preparation"
            );
            if let Some(canonical) = &source.canonical {
                ensure!(
                    source.path.canonicalize()? == *canonical,
                    "PCSX2 game-settings path was retargeted"
                );
            }
        }
        Ok(())
    }
}
