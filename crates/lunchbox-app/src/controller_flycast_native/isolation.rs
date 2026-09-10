//! Session-local Flycast mapping files. No existing user mapping is replaced.
use super::discovery::Selection;
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, path::Path};

pub(crate) struct PreparedMappings {
    directory: tempfile::TempDir,
    files: BTreeMap<String, String>,
    sources: Vec<Selection>,
}

impl PreparedMappings {
    /// Filenames must come from native path resolution. This owns files only:
    /// the launch caller must isolate higher-priority instance/game overrides.
    pub(crate) fn create(entries: &[(String, String)], sources: Vec<Selection>) -> Result<Self> {
        ensure!(
            !entries.is_empty() && entries.len() <= 32,
            "Invalid Flycast private mapping count"
        );
        let mut files = BTreeMap::new();
        let mut total = 0usize;
        for (filename, mapping) in entries {
            ensure!(
                filename.starts_with("SDL_")
                    && filename.ends_with(".cfg")
                    && filename.len() <= 255
                    && !filename.contains(['/', '\\'])
                    && !filename.chars().any(char::is_control),
                "Flycast mapping filename is not a safe native SDL component"
            );
            ensure!(
                !mapping.is_empty() && mapping.len() <= 4 * 1024 * 1024 && !mapping.contains('\0'),
                "Flycast generated mapping is empty, oversized or contains NUL"
            );
            if let Some(previous) = files.get(filename) {
                ensure!(
                    previous == mapping,
                    "Flycast controllers resolve to the same filename with different mappings"
                );
            } else {
                total += mapping.len();
                ensure!(
                    total <= 16 * 1024 * 1024,
                    "Flycast private mappings exceed total size limit"
                );
                files.insert(filename.clone(), mapping.clone());
            }
        }
        for source in &sources {
            source.verify()?;
        }
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-flycast-mappings-")
            .tempdir()?;
        for (filename, mapping) in &files {
            std::fs::write(directory.path().join(filename), mapping)?;
        }
        let prepared = Self {
            directory,
            files,
            sources,
        };
        prepared.verify_before_launch()?;
        Ok(prepared)
    }

    pub(crate) fn directory(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn mapping_directory_value(&self) -> Result<String> {
        super::paths::mapping_directory_value(self.directory())
    }

    pub(crate) fn verify_sources(&self) -> Result<()> {
        for source in &self.sources {
            source.verify()?;
        }
        Ok(())
    }

    /// Flycast may save trigger metadata after loading a mapping. Only compare
    /// private content before startup; retain source guards separately afterward.
    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        self.verify_sources()?;
        let base = self.directory.path().canonicalize()?;
        for (filename, expected) in &self.files {
            let path = self.directory.path().join(filename);
            let metadata = std::fs::symlink_metadata(&path)?;
            ensure!(
                metadata.is_file()
                    && metadata.len() == expected.len() as u64
                    && path.canonicalize()? == base.join(filename)
                    && std::fs::read(&path)? == expected.as_bytes(),
                "Flycast private mapping changed before startup"
            );
        }
        ensure!(
            std::fs::read_dir(self.directory.path())?.count() == self.files.len(),
            "Unexpected file appeared in Flycast private mapping directory"
        );
        Ok(())
    }
}
