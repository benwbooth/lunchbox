//! A uniquely owned profile in RPCS3's native input_configs/global directory.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) struct PreparedProfile {
    file: tempfile::NamedTempFile,
    directory: PathBuf,
    canonical_file: PathBuf,
    content: String,
    name: String,
}

impl PreparedProfile {
    /// `directory` must be the selected runtime's actual global input directory.
    /// No existing profile is replaced, and dropping the guard removes only the
    /// temporary file. Keep this guard alive for the entire native session.
    pub(crate) fn create(directory: &Path, content: String) -> Result<Self> {
        ensure!(
            directory.is_absolute(),
            "RPCS3 input directory must be absolute"
        );
        ensure!(
            !content.is_empty() && content.len() <= 1024 * 1024 && !content.contains('\0'),
            "RPCS3 generated input profile is empty or invalid"
        );
        let directory = directory
            .canonicalize()
            .context("RPCS3 global input directory is missing")?;
        ensure!(
            directory.is_dir(),
            "RPCS3 global input path is not a directory"
        );
        let mut file = tempfile::Builder::new()
            .prefix("lunchbox-controller-")
            .suffix(".yml")
            .tempfile_in(&directory)?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        let name = file
            .path()
            .file_stem()
            .and_then(|name| name.to_str())
            .context("RPCS3 temporary profile name is not UTF-8")?
            .to_owned();
        let canonical_file = file.path().canonicalize()?;
        let prepared = Self {
            file,
            directory,
            canonical_file,
            content,
            name,
        };
        prepared.verify()?;
        Ok(prepared)
    }

    /// These arguments must precede `--` and the boot path. They deliberately
    /// select no-GUI mode because native RPCS3 rejects this override otherwise.
    pub(crate) fn arguments(&self) -> [OsString; 3] {
        [
            "--no-gui".into(),
            "--input-config".into(),
            self.name.clone().into(),
        ]
    }

    pub(crate) fn path(&self) -> &Path {
        self.file.path()
    }

    pub(crate) fn verify(&self) -> Result<()> {
        let metadata = std::fs::symlink_metadata(self.file.path())?;
        ensure!(
            metadata.file_type().is_file(),
            "RPCS3 temporary profile was replaced"
        );
        ensure!(
            self.file.path().canonicalize()? == self.canonical_file
                && self.canonical_file.parent() == Some(self.directory.as_path())
                && self.directory.canonicalize()? == self.directory,
            "RPCS3 temporary profile location changed"
        );
        ensure!(
            metadata.len() == self.content.len() as u64,
            "RPCS3 temporary profile size changed"
        );
        ensure!(
            std::fs::read(self.file.path())? == self.content.as_bytes(),
            "RPCS3 temporary profile content changed"
        );
        Ok(())
    }
}
