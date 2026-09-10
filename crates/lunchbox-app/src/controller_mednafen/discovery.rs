//! Modern native settings order; legacy migration is deliberately not triggered.
use super::{
    layers::Snapshot,
    paths::{self, MissingOverride},
};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

pub(crate) struct Discovered {
    snapshot: Snapshot,
    missing: Vec<MissingOverride>,
}

fn include(
    path: PathBuf,
    files: &mut Vec<PathBuf>,
    missing: &mut Vec<MissingOverride>,
) -> Result<()> {
    match std::fs::symlink_metadata(&path) {
        Ok(_) => files.push(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            missing.push(MissingOverride::capture(path)?)
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

impl Discovered {
    pub(crate) fn effective(&self) -> Result<std::collections::BTreeMap<String, String>> {
        self.verify()?;
        self.snapshot.effective()
    }

    pub(crate) fn replacements(
        &self,
        assignments: &std::collections::BTreeMap<String, String>,
    ) -> Result<Vec<(PathBuf, String)>> {
        self.verify()?;
        self.snapshot.replacements(assignments)
    }

    /// Plain native launch only: command-line -loadconfig overrides must be
    /// resolved separately. file_base must be the native resolved FileBase.
    pub(crate) fn capture(base: &Path, system: &str, file_base: &str) -> Result<Self> {
        let mut files = vec![base.join("mednafen.cfg")];
        let global = Snapshot::capture(&files)?;
        ensure!(
            !global.first_line().contains(";VERSION 0."),
            "Mednafen legacy settings migration must be completed before calibrated launch"
        );
        let mut missing = Vec::new();
        include(paths::module(base, system)?, &mut files, &mut missing)?;
        let module = Snapshot::capture(&files)?;
        let effective = module.effective()?;
        let directory = effective
            .get("filesys.path_pgconfig")
            .map(String::as_str)
            .unwrap_or("pgconfig");
        include(
            paths::game(base, system, file_base, Path::new(directory))?,
            &mut files,
            &mut missing,
        )?;
        let snapshot = Snapshot::capture(&files)?;
        global.verify()?;
        module.verify()?;
        let result = Self { snapshot, missing };
        result.verify()?;
        Ok(result)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.snapshot.verify()?;
        for missing in &self.missing {
            missing.verify()?;
        }
        Ok(())
    }
}
