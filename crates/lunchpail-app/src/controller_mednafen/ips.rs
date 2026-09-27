//! In-memory native IPS semantics; never modifies user ROM or patch files.
use anyhow::{Result, ensure};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) enum Snapshot {
    Absent(super::paths::MissingOverride),
    Present {
        path: PathBuf,
        canonical: PathBuf,
        bytes: Vec<u8>,
    },
}

impl Snapshot {
    pub(crate) fn capture(rom: &Path) -> Result<Self> {
        let mut name = rom.as_os_str().to_owned();
        name.push(".ips");
        let path = PathBuf::from(name);
        match std::fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Self::Absent(super::paths::MissingOverride::capture(path)?))
            }
            Err(e) => Err(e.into()),
            Ok(_) => {
                let canonical = path.canonicalize()?;
                let bytes = read(&path)?;
                let result = Self::Present {
                    path,
                    canonical,
                    bytes,
                };
                result.verify()?;
                Ok(result)
            }
        }
    }
    pub(crate) fn verify(&self) -> Result<()> {
        match self {
            Self::Absent(guard) => guard.verify(),
            Self::Present {
                path,
                canonical,
                bytes,
            } => {
                ensure!(
                    path.canonicalize()? == *canonical && read(path)? == *bytes,
                    "Mednafen IPS patch changed during preparation"
                );
                Ok(())
            }
        }
    }
    pub(crate) fn apply(&self, content: &[u8]) -> Result<Vec<u8>> {
        self.verify()?;
        match self {
            Self::Absent(_) => Ok(content.to_vec()),
            Self::Present { bytes, .. } => apply(content, bytes),
        }
    }
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "IPS patch must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(32 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 32 * 1024 * 1024,
        "IPS patch exceeds preparation limit"
    );
    Ok(bytes)
}

/// Pinned IPSPatcher::Apply: zero RLE length means 65536; EOF ignores trailing
/// bytes (including purported truncate extensions); MemoryStream gaps are zero.
fn apply(content: &[u8], patch: &[u8]) -> Result<Vec<u8>> {
    ensure!(patch.starts_with(b"PATCH"), "Invalid IPS header");
    let mut input = std::io::Cursor::new(&patch[5..]);
    let mut result = content.to_vec();
    loop {
        let mut offset = [0; 3];
        input.read_exact(&mut offset)?;
        if &offset == b"EOF" {
            return Ok(result);
        }
        let offset =
            (usize::from(offset[0]) << 16) | (usize::from(offset[1]) << 8) | usize::from(offset[2]);
        let mut size = [0; 2];
        input.read_exact(&mut size)?;
        let mut count = usize::from(u16::from_be_bytes(size));
        let rle = count == 0;
        if rle {
            input.read_exact(&mut size)?;
            count = usize::from(u16::from_be_bytes(size));
            if count == 0 {
                count = 65536;
            }
        }
        let end = offset + count;
        ensure!(
            end <= 32 * 1024 * 1024,
            "Patched ROM exceeds preparation limit"
        );
        if end > result.len() {
            result.resize(end, 0);
        }
        if rle {
            let mut byte = [0];
            input.read_exact(&mut byte)?;
            result[offset..end].fill(byte[0]);
        } else {
            input.read_exact(&mut result[offset..end])?;
        }
    }
}
