//! Private copies of MAME's default.cfg and current machine basename cfg.
use super::settings::SavedSetup;
use anyhow::{Result, ensure};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
};

struct Source {
    path: PathBuf,
    canonical: Option<PathBuf>,
    bytes: Option<Vec<u8>>,
}

fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        file.metadata()?.is_file(),
        "MAME configuration is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 16 * 1024 * 1024,
        "MAME configuration exceeds size limit"
    );
    Ok(Some(bytes))
}

pub(crate) struct PreparedConfigs {
    directory: tempfile::TempDir,
    sources: Vec<Source>,
    copies: Vec<(PathBuf, Vec<u8>)>,
}

impl PreparedConfigs {
    pub(crate) fn create(
        setup: &SavedSetup,
        source_directory: &Path,
        machine_basename: &str,
    ) -> Result<Self> {
        setup.validate()?;
        ensure!(
            source_directory.is_absolute(),
            "MAME cfg directory must be resolved to an absolute path"
        );
        ensure!(
            !machine_basename.is_empty()
                && machine_basename != "default"
                && machine_basename
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
            "MAME configuration needs an exact machine basename"
        );
        let mut types = BTreeSet::new();
        for player in &setup.players {
            for direction in ["UP", "DOWN", "LEFT", "RIGHT"] {
                types.insert(format!("P{}_JOYSTICK_{direction}", player.player));
            }
            for button in 1..=player.panel.buttons() {
                types.insert(format!("P{}_BUTTON{button}", player.player));
            }
            types.insert(format!("START{}", player.player));
            types.insert(format!("COIN{}", player.player));
        }
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-mame-cfg-")
            .tempdir()?;
        let mut sources = Vec::new();
        let mut copies = Vec::new();
        for name in ["default.cfg".to_owned(), format!("{machine_basename}.cfg")] {
            let path = source_directory.join(&name);
            let bytes = read(&path)?;
            let canonical = if bytes.is_some() {
                Some(path.canonicalize()?)
            } else {
                None
            };
            if let Some(bytes) = &bytes {
                let filtered = super::overrides::without_panel_overrides(bytes, &types)?;
                let copy = directory.path().join(name);
                std::fs::write(&copy, &filtered)?;
                copies.push((copy, filtered));
            }
            sources.push(Source {
                path,
                canonical,
                bytes,
            });
        }
        let prepared = Self {
            directory,
            sources,
            copies,
        };
        prepared.verify_before_launch()?;
        Ok(prepared)
    }

    /// Call before startup only: MAME legitimately writes its private cfg files
    /// during execution. Do not mistake those writes for source modifications.
    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        for source in &self.sources {
            ensure!(
                read(&source.path)? == source.bytes,
                "MAME source configuration changed during preparation"
            );
            if let Some(canonical) = &source.canonical {
                ensure!(
                    source.path.canonicalize()? == *canonical,
                    "MAME source configuration was redirected"
                );
            }
        }
        for (path, bytes) in &self.copies {
            ensure!(
                std::fs::symlink_metadata(path)?.is_file() && std::fs::read(path)? == *bytes,
                "MAME private configuration changed before startup"
            );
        }
        Ok(())
    }

    pub(crate) fn arguments(&self) -> [OsString; 2] {
        [
            "-cfg_directory".into(),
            self.directory.path().as_os_str().to_owned(),
        ]
    }
}
