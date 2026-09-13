//! Retain a native Snes9x GTK config overlay without changing the user's INI or saves.
use anyhow::{Context, Result, ensure};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
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

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)]
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

fn ensure_mode(path: &Path, mode: u32, directory: bool) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(
        if directory {
            metadata.is_dir()
        } else {
            metadata.is_file()
        },
        "Snes9x private staging type or mode changed for {}",
        path.display()
    );
    #[cfg(unix)]
    ensure!(
        metadata.mode() & 0o7777 == mode,
        "Snes9x private staging type or mode changed for {}",
        path.display()
    );
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
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
        // Use Lunchbox's host-visible cache rather than a sandbox-private /tmp.
        // This lets a separately launched target Flatpak receive one exact
        // per-session directory without exposing or rewriting its real profile.
        let home = directories::BaseDirs::new()
            .context("Finding the user home for Snes9x controller staging")?
            .home_dir()
            .to_path_buf();
        let cache = home.join(".cache/lunchbox/controller-launch");
        std::fs::create_dir_all(&cache)?;
        let directory = tempfile::Builder::new()
            .prefix("snes9x-config-")
            .tempdir_in(cache)?;
        set_mode(directory.path(), 0o700)?;
        for name in ["snes9x", "cache", "data", "state"] {
            let path = directory.path().join(name);
            std::fs::create_dir(&path)?;
            set_mode(&path, 0o700)?;
        }
        let config_path = directory.path().join("snes9x/snes9x.conf");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut config_file = options.open(&config_path)?;
        #[cfg(unix)]
        config_file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        config_file.write_all(config.as_bytes())?;
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
        self.directory.path().join("snes9x/snes9x.conf")
    }

    pub(crate) fn root(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure_mode(self.root(), 0o700, true)?;
        for name in ["snes9x", "cache", "data", "state"] {
            ensure_mode(&self.root().join(name), 0o700, true)?;
        }
        ensure_mode(&self.config_path(), 0o600, false)?;
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
