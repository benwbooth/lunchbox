//! Read-only directory prerequisites for Steem SSE's embedded EmuTOS mode.
use anyhow::{Context, Result, bail, ensure};
use std::path::{Path, PathBuf};

pub(crate) struct InputSnapshot {
    system: PathBuf,
    canonical_system: PathBuf,
    run: PathBuf,
    canonical_run: PathBuf,
}

fn require_embedded_rom(run: &Path) -> Result<()> {
    let override_path = run.join("tosEmu.img");
    match std::fs::symlink_metadata(&override_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("Inspecting Steem SSE's EmuTOS override"),
        Ok(_) => bail!(
            "Steem SSE embedded-ROM mode requires no tosEmu.img override in {}; the existing file was not changed",
            run.display()
        ),
    }
}

impl InputSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.system.is_dir()
                && self.run.is_dir()
                && self.system.canonicalize()? == self.canonical_system
                && self.run.canonicalize()? == self.canonical_run,
            "Steem SSE system/run directory changed during controller preparation"
        );
        require_embedded_rom(&self.run)
    }

    pub(crate) fn append_config(&self) -> Result<String> {
        let path = self
            .system
            .to_str()
            .context("Steem SSE system directory must be UTF-8")?;
        ensure!(
            !path
                .chars()
                .any(|c| c.is_control() || matches!(c, '"' | '\\')),
            "Steem SSE system directory cannot be represented in RetroArch configuration"
        );
        Ok(format!("system_directory = \"{path}\"\n"))
    }
}

pub(crate) fn prepare(system: &Path) -> Result<InputSnapshot> {
    let run = system.join("SteemSSE");
    ensure!(
        system.is_absolute() && system.is_dir() && run.is_dir(),
        "Steem SSE requires existing absolute system and system/SteemSSE directories; initialize native setup first"
    );
    let snapshot = InputSnapshot {
        system: system.to_owned(),
        canonical_system: system.canonicalize()?,
        canonical_run: run.canonicalize()?,
        run,
    };
    snapshot.append_config()?;
    snapshot.verify()?;
    Ok(snapshot)
}
