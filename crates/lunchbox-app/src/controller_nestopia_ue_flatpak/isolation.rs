//! Private Nestopia configuration staging with persistent user game data.

use super::{configuration, settings::SavedSetup};
use anyhow::{Context, Result, ensure};
use std::{
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const FILE_LIMIT: u64 = 2 * 1024 * 1024;

fn read(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "Nestopia config must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= FILE_LIMIT,
        "Nestopia config exceeds size limit"
    );
    Ok(bytes)
}

fn mode(path: &Path, expected: u32, directory: bool) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(
        if directory {
            metadata.is_dir()
        } else {
            metadata.is_file()
        } && metadata.mode() & 0o7777 == expected,
        "Nestopia private staging type or mode changed for {}",
        path.display()
    );
    Ok(())
}

fn mkdir(path: &Path) -> Result<()> {
    std::fs::create_dir(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(crate) struct PreparedConfig {
    directory: tempfile::TempDir,
    source_main: PathBuf,
    canonical_main: PathBuf,
    original_main: Vec<u8>,
    source_input: PathBuf,
    canonical_input: PathBuf,
    original_input: Vec<u8>,
}

impl PreparedConfig {
    pub(crate) fn prepare(setup: &SavedSetup, pads: &[configuration::Pad]) -> Result<Self> {
        setup.validate()?;
        let source_main = setup.source_main_config.clone();
        let source_input = setup.source_input_config.clone();
        let canonical_main = source_main.canonicalize()?;
        let canonical_input = source_input.canonicalize()?;
        let original_main = read(&source_main)?;
        let original_input = read(&source_input)?;
        let rendered_main = configuration::render_main(
            std::str::from_utf8(&original_main).context("Nestopia main config is not UTF-8")?,
        )?;
        let rendered_input = configuration::render_input(
            std::str::from_utf8(&original_input).context("Nestopia input config is not UTF-8")?,
            pads,
        )?;

        let home = directories::BaseDirs::new()
            .context("Finding the user home for Nestopia staging")?
            .home_dir()
            .to_path_buf();
        let cache = home.join(".cache/lunchbox/controller-launch");
        std::fs::create_dir_all(&cache)?;
        let directory = tempfile::Builder::new()
            .prefix("nestopia-flatpak-config-")
            .tempdir_in(cache)?;
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
        for relative in ["home", "config", "config/nestopia", "cache", "state"] {
            mkdir(&directory.path().join(relative))?;
        }
        write_private(
            &directory.path().join("config/nestopia/nestopia.conf"),
            rendered_main.as_bytes(),
        )?;
        write_private(
            &directory.path().join("config/nestopia/input.conf"),
            rendered_input.as_bytes(),
        )?;
        let prepared = Self {
            directory,
            source_main,
            canonical_main,
            original_main,
            source_input,
            canonical_input,
            original_input,
        };
        prepared.verify()?;
        Ok(prepared)
    }

    pub(crate) fn root(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn home(&self) -> PathBuf {
        self.root().join("home")
    }

    pub(crate) fn config_home(&self) -> PathBuf {
        self.root().join("config")
    }

    pub(crate) fn cache_home(&self) -> PathBuf {
        self.root().join("cache")
    }

    pub(crate) fn state_home(&self) -> PathBuf {
        self.root().join("state")
    }

    pub(crate) fn main_path(&self) -> PathBuf {
        self.config_home().join("nestopia/nestopia.conf")
    }

    pub(crate) fn input_path(&self) -> PathBuf {
        self.config_home().join("nestopia/input.conf")
    }

    pub(crate) fn verify(&self) -> Result<()> {
        mode(self.root(), 0o700, true)?;
        for relative in ["home", "config", "config/nestopia", "cache", "state"] {
            mode(&self.root().join(relative), 0o700, true)?;
        }
        mode(&self.main_path(), 0o600, false)?;
        mode(&self.input_path(), 0o600, false)?;
        ensure!(
            self.source_main.canonicalize()? == self.canonical_main
                && self.source_input.canonicalize()? == self.canonical_input
                && read(&self.source_main)? == self.original_main
                && read(&self.source_input)? == self.original_input,
            "Nestopia user configuration changed during preparation"
        );
        Ok(())
    }
}
