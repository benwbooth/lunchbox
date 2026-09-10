//! Retain a generated input profile for a launch, never overwrite user profiles.
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

pub(crate) struct PreparedProfile {
    directory: tempfile::TempDir,
    content: String,
}

impl PreparedProfile {
    pub(crate) const NAME: &'static str = "lunchbox-controller";

    pub(crate) fn create(content: String) -> Result<Self> {
        ensure!(
            !content.is_empty() && content.len() <= 1024 * 1024 && !content.contains('\0'),
            "PCSX2 generated input profile is empty, oversized or contains NUL"
        );
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-pcsx2-input-")
            .tempdir()?;
        let profile = Self { directory, content };
        std::fs::write(profile.path(), &profile.content)?;
        profile.verify()?;
        Ok(profile)
    }

    /// EmuFolders::InputProfiles joins this directory with profile name + .ini.
    pub(crate) fn directory(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn path(&self) -> PathBuf {
        self.directory().join(format!("{}.ini", Self::NAME))
    }

    pub(crate) fn verify(&self) -> Result<()> {
        let path = self.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        ensure!(
            metadata.is_file()
                && metadata.len() == self.content.len() as u64
                && path.canonicalize()?
                    == self
                        .directory()
                        .canonicalize()?
                        .join(format!("{}.ini", Self::NAME))
                && std::fs::read(&path)? == self.content.as_bytes(),
            "PCSX2 private input profile changed before launch"
        );
        Ok(())
    }
}
