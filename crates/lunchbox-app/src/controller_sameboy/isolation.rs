//! Retain a native SameBoy SDL config overlay without changing the user's INI or saves.
use anyhow::{Result, ensure};
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
        "SameBoy SDL config must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 2 * 1024 * 1024,
        "SameBoy SDL config exceeds size limit"
    );
    Ok(bytes)
}

impl PreparedConfig {
    pub(crate) fn prepare(
        source: &Path,
        bindings: &super::Bindings,
        abi: super::configuration::Abi,
    ) -> Result<Self> {
        ensure!(
            source.is_absolute(),
            "SameBoy preferences path must be absolute"
        );
        let source = source.to_owned();
        let canonical_source = source.canonicalize()?;
        let original = read(&source)?;
        let config = super::configuration::render(&original, bindings, abi)?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-sameboy-config-")
            .tempdir()?;
        std::fs::write(directory.path().join("prefs.bin"), config)?;
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
        self.directory.path().join("prefs.bin")
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.source.canonicalize()? == self.canonical_source
                && read(&self.source)? == self.original,
            "SameBoy SDL native config changed during preparation"
        );
        Ok(())
    }

    /// Overlay only prefs.bin so native save, state and screenshot paths
    /// remain unchanged. Caller must resolve native SDL preference selection
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
            "SameBoy SDL overlay needs absolute launch paths"
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
