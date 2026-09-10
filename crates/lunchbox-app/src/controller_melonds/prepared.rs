//! Retained generated configuration, separate from user-owned melonDS settings.
use crate::controller_catalog::Calibration;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::sdl2::Device;
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) struct PreparedConfig {
    directory: tempfile::TempDir,
    source: PathBuf,
    canonical_source: PathBuf,
    original: Vec<u8>,
    generated: String,
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "melonDS configuration must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 4 * 1024 * 1024,
        "melonDS configuration exceeds limit"
    );
    Ok(bytes)
}

impl PreparedConfig {
    /// `controller_id` must have been resolved to this measured device by the
    /// retained physical capture session, never by a model-name match.
    pub(crate) fn create(
        setup: &super::settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        controller_id: &str,
        device: &Device,
    ) -> Result<Self> {
        setup.review(calibrations)?;
        let player = &setup.players[0];
        ensure!(
            player.controller_id == controller_id,
            "melonDS measured controller differs from saved selection"
        );
        let controls = super::physical::controls(
            &calibrations[controller_id],
            &player.source_controls,
            device,
        )?;
        let source = setup.source_config.clone();
        let canonical_source = source.canonicalize()?;
        let original = read(&source)?;
        let generated = super::configuration::render(
            std::str::from_utf8(&original).context("melonDS configuration is not UTF-8")?,
            u16::try_from(device.device_index)?,
            &controls,
        )?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-melonds-")
            .tempdir()?;
        std::fs::write(directory.path().join("melonDS.toml"), &generated)?;
        let prepared = Self {
            directory,
            source,
            canonical_source,
            original,
            generated,
        };
        prepared.verify()?;
        Ok(prepared)
    }

    pub(crate) fn path(&self) -> PathBuf {
        self.directory.path().join("melonDS.toml")
    }

    /// Overlay only the verified native config file in a private mount namespace.
    /// Keep the native data directory and its firmware/save paths unchanged.
    /// `native_config` must come from actual runtime path resolution.
    pub(crate) fn append_mount(
        &self,
        native_config: &Path,
        arguments: &mut Vec<std::ffi::OsString>,
    ) -> Result<()> {
        self.verify()?;
        ensure!(
            native_config.is_absolute() && native_config.canonicalize()? == self.canonical_source,
            "melonDS saved source does not match the native configuration path"
        );
        arguments.extend([
            "--bind".into(),
            self.path().into_os_string(),
            self.canonical_source.as_os_str().to_owned(),
        ]);
        Ok(())
    }

    /// Post-startup native config writes are legitimate in the private copy;
    /// verify the original file separately from generated prelaunch contents.
    pub(crate) fn verify_source(&self) -> Result<()> {
        ensure!(
            self.source.canonicalize()? == self.canonical_source
                && read(&self.source)? == self.original,
            "melonDS source configuration changed during the session"
        );
        Ok(())
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.verify_source()?;
        let path = self.path();
        ensure!(
            std::fs::symlink_metadata(&path)?.file_type().is_file()
                && path.canonicalize()?
                    == self.directory.path().canonicalize()?.join("melonDS.toml")
                && read(&path)? == self.generated.as_bytes(),
            "melonDS generated configuration changed"
        );
        Ok(())
    }
}
