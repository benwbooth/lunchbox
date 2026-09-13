//! Private puNES configuration staging with persistent native game data.

use super::{configuration, settings::SavedSetup};
use anyhow::{Context, Result, ensure};
use std::{
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const FILE_LIMIT: u64 = 4 * 1024 * 1024;
const CONFIG_DIRECTORIES: [&str; 5] = [
    "config/puNES/cheat",
    "config/puNES/jsc",
    "config/puNES/pgs",
    "config/puNES/shp",
    "config/puNES",
];

fn read(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "puNES config must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= FILE_LIMIT,
        "puNES config exceeds size limit"
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
        "puNES private staging type or mode changed for {}",
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
    pads: Vec<configuration::Pad>,
}

impl PreparedConfig {
    pub(crate) fn prepare(setup: &SavedSetup, pads: Vec<configuration::Pad>) -> Result<Self> {
        setup.validate()?;
        configuration::validate_pads(&pads)?;
        ensure!(
            setup.players.len() == pads.len(),
            "puNES target-pad count changed"
        );
        let source_main = setup.source_main_config.clone();
        let source_input = setup.source_input_config.clone();
        let canonical_main = source_main.canonicalize()?;
        let canonical_input = source_input.canonicalize()?;
        let original_main = read(&source_main)?;
        let original_input = read(&source_input)?;
        let rendered_main = configuration::render_main(&original_main)?;
        let rendered_input = configuration::render_input(&original_input, &pads)?;

        let home = directories::BaseDirs::new()
            .context("Finding the user home for puNES staging")?
            .home_dir()
            .to_path_buf();
        let cache = home.join(".cache/lunchbox/controller-launch");
        std::fs::create_dir_all(&cache)?;
        let directory = tempfile::Builder::new()
            .prefix("punes-flatpak-config-")
            .tempdir_in(cache)?;
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
        for relative in [
            "home",
            "config",
            "config/puNES",
            "config/puNES/cheat",
            "config/puNES/jsc",
            "config/puNES/pgs",
            "config/puNES/shp",
            "cache",
            "state",
        ] {
            mkdir(&directory.path().join(relative))?;
        }
        write_private(
            &directory.path().join("config/puNES/puNES.cfg"),
            rendered_main.as_bytes(),
        )?;
        write_private(
            &directory.path().join("config/puNES/input.cfg"),
            rendered_input.as_bytes(),
        )?;
        for pad in &pads {
            write_private(
                &directory
                    .path()
                    .join("config/puNES/jsc")
                    .join(configuration::jsc_file_name(&pad.guid)?),
                configuration::render_jsc().as_bytes(),
            )?;
        }
        let prepared = Self {
            directory,
            source_main,
            canonical_main,
            original_main,
            source_input,
            canonical_input,
            original_input,
            pads,
        };
        prepared.verify()?;
        Ok(prepared)
    }

    pub(crate) fn root(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn main_path(&self) -> PathBuf {
        self.root().join("config/puNES/puNES.cfg")
    }

    pub(crate) fn input_path(&self) -> PathBuf {
        self.root().join("config/puNES/input.cfg")
    }

    pub(crate) fn jsc_paths(&self) -> Result<Vec<PathBuf>> {
        self.pads
            .iter()
            .map(|pad| {
                Ok(self
                    .root()
                    .join("config/puNES/jsc")
                    .join(configuration::jsc_file_name(&pad.guid)?))
            })
            .collect()
    }

    pub(crate) fn verify(&self) -> Result<()> {
        mode(self.root(), 0o700, true)?;
        for relative in ["home", "config", "cache", "state"] {
            mode(&self.root().join(relative), 0o700, true)?;
        }
        for relative in CONFIG_DIRECTORIES {
            mode(&self.root().join(relative), 0o700, true)?;
        }
        mode(&self.main_path(), 0o600, false)?;
        mode(&self.input_path(), 0o600, false)?;
        for path in self.jsc_paths()? {
            mode(&path, 0o600, false)?;
            ensure!(
                read(&path)? == configuration::render_jsc().as_bytes(),
                "puNES target mapping changed"
            );
        }
        ensure!(
            self.source_main.canonicalize()? == self.canonical_main
                && self.source_input.canonicalize()? == self.canonical_input
                && read(&self.source_main)? == self.original_main
                && read(&self.source_input)? == self.original_input,
            "puNES user configuration changed during preparation"
        );
        Ok(())
    }
}
