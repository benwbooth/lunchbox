//! Session-owned controller profile and matching native command-line options.
//! Does not alter the user's ctrlr or cfg directories.
use super::settings::SavedSetup;
use anyhow::{Result, ensure};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) struct PreparedProfile {
    directory: tempfile::TempDir,
    profile: PathBuf,
    bytes: Vec<u8>,
    threshold: u16,
}

impl PreparedProfile {
    /// `xml` must be produced by physical preparation, not accepted directly
    /// from saved JSON. The caller retains this owner for the child's lifetime.
    pub(crate) fn create(setup: &SavedSetup, xml: String) -> Result<Self> {
        setup.validate()?;
        ensure!(
            setup.joystick_provider == "sdl",
            "Prepared MAME profile requires raw SDL provider"
        );
        ensure!(
            !xml.is_empty() && xml.len() <= 1024 * 1024,
            "MAME generated profile size is invalid"
        );
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-mame-panel-")
            .tempdir()?;
        let path = directory
            .path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("MAME controller directory must be UTF-8"))?;
        ensure!(
            !path.contains(';'),
            "MAME controller path contains a multipath separator"
        );
        let profile = directory.path().join("lunchbox-panel.cfg");
        let bytes = xml.into_bytes();
        // The directory is privately created; no existing user files are replaced.
        std::fs::write(&profile, &bytes)?;
        let prepared = Self {
            directory,
            profile,
            bytes,
            threshold: setup.threshold_basis_points,
        };
        prepared.verify()?;
        Ok(prepared)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.profile
    }

    pub(crate) fn verify(&self) -> Result<()> {
        let metadata = std::fs::symlink_metadata(&self.profile)?;
        ensure!(
            metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() == self.bytes.len() as u64
                && self.profile.canonicalize()?
                    == self
                        .directory
                        .path()
                        .canonicalize()?
                        .join("lunchbox-panel.cfg")
                && std::fs::read(&self.profile)? == self.bytes,
            "MAME session controller profile changed during preparation"
        );
        Ok(())
    }

    /// The complete launch builder must resolve existing CLI/INI conflicts and
    /// game-specific input overrides before it can claim these mappings apply.
    pub(crate) fn arguments(&self) -> Vec<OsString> {
        let threshold = format!("{}.{:04}", self.threshold / 10_000, self.threshold % 10_000);
        vec![
            "-ctrlrpath".into(),
            self.directory.path().as_os_str().to_owned(),
            "-ctrlr".into(),
            "lunchbox-panel".into(),
            "-joystickprovider".into(),
            "sdl".into(),
            "-joystick".into(),
            "-nosixaxis".into(),
            "-joystick_threshold".into(),
            threshold.into(),
        ]
    }
}
