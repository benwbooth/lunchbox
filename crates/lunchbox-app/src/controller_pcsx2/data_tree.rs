//! Own the controller-specific private data tree selected with native -datapath.
use super::{configuration, folders, game_settings::Selection};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) struct PreparedData {
    directory: tempfile::TempDir,
    source: PathBuf,
    canonical_source: PathBuf,
    original: Vec<u8>,
    generated: BTreeMap<PathBuf, String>,
    game: Selection,
    secrets_path: PathBuf,
    secrets_original: Option<Vec<u8>>,
    secrets_canonical: Option<PathBuf>,
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "PCSX2 main settings is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 16 * 1024 * 1024,
        "PCSX2 main settings exceeds limit"
    );
    Ok(bytes)
}

impl PreparedData {
    pub(crate) fn create(
        source: &Path,
        original_root: &Path,
        serial: &str,
        crc: u32,
        profile: &str,
    ) -> Result<Self> {
        ensure!(
            source.is_absolute() && original_root.is_absolute(),
            "PCSX2 source paths must be absolute"
        );
        let canonical_source = source.canonicalize()?;
        let original = read(source)?;
        let text = std::str::from_utf8(&original)?;
        let secrets_path = source
            .parent()
            .ok_or_else(|| anyhow::anyhow!("PCSX2 settings directory is missing"))?
            .join("secrets.ini");
        let secrets_original = match read(&secrets_path) {
            Ok(bytes) => Some(bytes),
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        let secrets_canonical = if secrets_original.is_some() {
            Some(secrets_path.canonicalize()?)
        } else {
            None
        };
        let secrets = std::str::from_utf8(secrets_original.as_deref().unwrap_or_default())?;
        let mut folder_values = configuration::folder_values(text)?;
        folder_values.extend(configuration::folder_values(secrets)?);
        let resolved = folders::resolve(original_root, &folder_values)?;
        let game = Selection::capture(&resolved["GameSettings"], serial, crc)?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-pcsx2-data-")
            .tempdir()?;
        let game_dir = directory.path().join("gamesettings");
        let input_dir = directory.path().join("inputprofiles");
        for path in [
            directory.path().join("inis"),
            game_dir.clone(),
            input_dir.clone(),
        ] {
            std::fs::create_dir(path)?;
        }
        let values = folders::isolated_values(&resolved, &game_dir, &input_dir)?;
        let main = configuration::with_folders(
            &configuration::with_controller_profile(text, profile)?,
            &values,
        )?;
        let game_overlay = game.overlay(profile)?;
        let mut game_values = folder_values;
        game_values.extend(configuration::folder_values(&game_overlay)?);
        let game_folders = folders::isolated_values(
            &folders::resolve(original_root, &game_values)?,
            &game_dir,
            &input_dir,
        )?;
        let generated = BTreeMap::from([
            (PathBuf::from("inis/PCSX2.ini"), main),
            (
                PathBuf::from("inis/secrets.ini"),
                configuration::with_folders(
                    &configuration::with_controller_profile(secrets, profile)?,
                    &values,
                )?,
            ),
            (
                PathBuf::from("gamesettings").join(&game.filename),
                configuration::with_folders(&game_overlay, &game_folders)?,
            ),
        ]);
        for (path, text) in &generated {
            std::fs::write(directory.path().join(path), text)?;
        }
        let data = Self {
            directory,
            source: source.to_owned(),
            canonical_source,
            original,
            generated,
            game,
            secrets_path,
            secrets_original,
            secrets_canonical,
        };
        data.verify_before_launch()?;
        Ok(data)
    }

    pub(crate) fn directory(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        ensure!(
            self.source.canonicalize()? == self.canonical_source
                && read(&self.source)? == self.original,
            "PCSX2 source config changed during preparation"
        );
        self.game.verify()?;
        let secrets = match read(&self.secrets_path) {
            Ok(bytes) => Some(bytes),
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        ensure!(
            secrets == self.secrets_original,
            "PCSX2 secrets config changed during preparation"
        );
        if let Some(canonical) = &self.secrets_canonical {
            ensure!(
                self.secrets_path.canonicalize()? == *canonical,
                "PCSX2 secrets path was retargeted"
            );
        }
        let root = self.directory().canonicalize()?;
        for (relative, expected) in &self.generated {
            let path = self.directory().join(relative);
            let metadata = std::fs::symlink_metadata(&path)?;
            ensure!(
                metadata.is_file()
                    && metadata.len() == expected.len() as u64
                    && path.canonicalize()? == root.join(relative)
                    && std::fs::read(path)? == expected.as_bytes(),
                "PCSX2 private config changed before launch"
            );
        }
        Ok(())
    }
}
