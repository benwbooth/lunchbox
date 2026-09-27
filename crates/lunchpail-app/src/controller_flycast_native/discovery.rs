//! Read-only native mapping selection and source freshness guards.
//! Caller resolves custom MappingsPath entries and readonly config fallback
//! directories from the actual launch environment; no paths are guessed here.
use super::paths::Candidate;
use anyhow::{Result, ensure};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

struct Observed {
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
        "Flycast mapping is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 4 * 1024 * 1024,
        "Flycast mapping exceeds preparation limit"
    );
    Ok(Some(bytes))
}

pub(crate) struct Selection {
    observed: Vec<Observed>,
    pub candidate: Option<Candidate>,
}

impl Selection {
    /// Native order is filename-first, then directory order for that filename.
    /// A fresh child must be used: native in-process mapping caches bypass I/O.
    pub(crate) fn capture(candidates: &[Candidate], directories: &[PathBuf]) -> Result<Self> {
        ensure!(
            !candidates.is_empty() && candidates.len() <= 8,
            "Invalid Flycast candidate list"
        );
        ensure!(
            !directories.is_empty()
                && directories.len() <= 32
                && directories.iter().all(|path| path.is_absolute()),
            "Flycast mapping directories must be explicitly resolved absolute paths"
        );
        let mut observed = Vec::new();
        let mut chosen = None;
        'candidate: for candidate in candidates {
            ensure!(
                Path::new(&candidate.filename).components().count() == 1
                    && !candidate.filename.contains(['/', '\\'])
                    && candidate.filename.ends_with(".cfg"),
                "Flycast candidate is not a plain mapping filename"
            );
            for directory in directories {
                let path = directory.join(&candidate.filename);
                let bytes = read(&path)?;
                let canonical = if bytes.is_some() {
                    Some(path.canonicalize()?)
                } else {
                    None
                };
                let found = bytes.is_some();
                observed.push(Observed {
                    path,
                    canonical,
                    bytes,
                });
                if found {
                    chosen = Some(candidate.clone());
                    break 'candidate;
                }
            }
        }
        let selection = Self {
            observed,
            candidate: chosen,
        };
        selection.verify()?;
        Ok(selection)
    }

    pub(crate) fn selected(&self) -> Option<(&Path, &[u8])> {
        self.observed.last().and_then(|entry| {
            entry
                .bytes
                .as_deref()
                .map(|bytes| (entry.path.as_path(), bytes))
        })
    }

    pub(crate) fn verify(&self) -> Result<()> {
        for entry in &self.observed {
            ensure!(
                read(&entry.path)? == entry.bytes,
                "Flycast mapping source or higher-priority absence changed"
            );
            if let Some(canonical) = &entry.canonical {
                ensure!(
                    entry.path.canonicalize()? == *canonical,
                    "Flycast mapping source was redirected"
                );
            }
        }
        Ok(())
    }
}
