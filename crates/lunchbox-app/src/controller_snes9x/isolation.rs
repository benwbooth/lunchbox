//! Retain a native Snes9x GTK config overlay without changing the user's INI or saves.
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
        "Snes9x GTK config must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 2 * 1024 * 1024,
        "Snes9x GTK config exceeds size limit"
    );
    Ok(bytes)
}

impl PreparedConfig {
    /// Mirrors GTK get_config_dir without its directory-creation side effect.
    /// A relative XDG path is native behavior and is resolved against launch cwd.
    pub(crate) fn native_path(cwd: &Path) -> Result<PathBuf> {
        ensure!(
            cwd.is_absolute(),
            "Snes9x launch directory must be absolute"
        );
        let root = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join("snes9x")
        } else if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join(".config/snes9x")
        } else {
            PathBuf::from(".snes9x")
        };
        Ok(cwd.join(root).join("snes9x.conf"))
    }

    pub(crate) fn prepare(
        setup: &super::settings::SavedSetup,
        pads: &[super::configuration::Pad],
    ) -> Result<Self> {
        setup.validate()?;
        let source = setup.source_config.clone();
        let canonical_source = source.canonicalize()?;
        let original = read(&source)?;
        let config = super::configuration::render(
            std::str::from_utf8(&original).context("Snes9x GTK configuration is not UTF-8")?,
            pads,
        )?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-snes9x-config-")
            .tempdir()?;
        std::fs::write(directory.path().join("snes9x.conf"), config)?;
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
        self.directory.path().join("snes9x.conf")
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.source.canonicalize()? == self.canonical_source
                && read(&self.source)? == self.original,
            "Snes9x GTK native config changed during preparation"
        );
        Ok(())
    }

    /// Overlay only snes9x.conf so native save, state and screenshot paths
    /// remain unchanged. Caller must resolve native GTK config selection
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
            "Snes9x GTK overlay needs absolute launch paths"
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
