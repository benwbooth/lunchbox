//! Retain a native mGBA config overlay without changing the user's INI or saves.
use anyhow::{Context, Result, ensure};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(crate) struct PreparedConfig {
    directory: tempfile::TempDir,
    source: PathBuf,
    canonical_source: PathBuf,
    original: Vec<u8>,
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "mGBA config must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 2 * 1024 * 1024,
        "mGBA config exceeds size limit"
    );
    Ok(bytes)
}

impl PreparedConfig {
    pub(crate) fn prepare(
        setup: &super::settings::SavedSetup,
        calibration: &crate::controller_catalog::Calibration,
        snapshot: &lunchbox_controller_probe::sdl2::Snapshot,
        runtime_path: &str,
    ) -> Result<Self> {
        setup.validate()?;
        let source = setup.source_config.clone();
        let canonical_source = source.canonicalize()?;
        let original = read(&source)?;
        let config = super::render_calibrated(
            std::str::from_utf8(&original).context("mGBA configuration is not UTF-8")?,
            calibration,
            snapshot,
            runtime_path,
            setup.handheld == super::settings::Handheld::Gba,
        )?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-mgba-config-")
            .tempdir()?;
        std::fs::write(directory.path().join("config.ini"), config)?;
        let prepared = Self {
            directory,
            source,
            canonical_source,
            original,
        };
        prepared.verify()?;
        Ok(prepared)
    }

    pub(crate) fn config_path(&self) -> PathBuf {
        self.directory.path().join("config.ini")
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.source.canonicalize()? == self.canonical_source
                && read(&self.source)? == self.original,
            "mGBA native config changed during preparation"
        );
        Ok(())
    }

    /// The pinned SDL CLI's -C sets scalar options, not an input INI path.
    /// Overlay only config.ini so native save, state, BIOS and screenshot paths
    /// remain unchanged. Caller must resolve native/portable config selection
    /// and retain this owner through the child lifetime.
    #[cfg(target_os = "linux")]
    pub(crate) fn overlay_arguments(
        &self,
        executable: &Path,
        arguments: &[std::ffi::OsString],
        cwd: &Path,
    ) -> Result<Vec<std::ffi::OsString>> {
        self.verify()?;
        ensure!(
            executable.is_absolute() && cwd.is_absolute(),
            "mGBA overlay needs absolute launch paths"
        );
        let mut result = vec![
            "--die-with-parent".into(),
            "--bind".into(),
            "/".into(),
            "/".into(),
            "--bind".into(),
            self.config_path().into_os_string(),
            self.canonical_source.as_os_str().to_owned(),
            "--chdir".into(),
            cwd.as_os_str().to_owned(),
            "--".into(),
            executable.as_os_str().to_owned(),
        ];
        result.extend_from_slice(arguments);
        Ok(result)
    }
}
