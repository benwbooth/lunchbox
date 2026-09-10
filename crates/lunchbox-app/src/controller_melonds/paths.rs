//! Native Linux Qt configuration lookup, retaining the selected source path.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) struct ConfigPath {
    executable: PathBuf,
    xdg: Option<OsString>,
    home: Option<OsString>,
    path: PathBuf,
}

impl ConfigPath {
    pub(crate) fn capture(executable: &Path, source: &Path) -> Result<Self> {
        ensure!(
            cfg!(target_os = "linux"),
            "melonDS native path lookup currently requires Linux"
        );
        let executable = executable.canonicalize()?;
        let xdg = std::env::var_os("XDG_CONFIG_HOME");
        let home = std::env::var_os("HOME");
        let path = resolve(&executable, xdg.as_ref(), home.as_ref())?;
        ensure!(
            path.canonicalize()? == source.canonicalize()?,
            "melonDS saved configuration differs from the native runtime location"
        );
        Ok(Self {
            executable,
            xdg,
            home,
            path,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            std::env::var_os("XDG_CONFIG_HOME") == self.xdg
                && std::env::var_os("HOME") == self.home,
            "melonDS inherited configuration environment changed"
        );
        ensure!(
            resolve(&self.executable, self.xdg.as_ref(), self.home.as_ref())? == self.path,
            "melonDS portable/configuration selection changed"
        );
        Ok(())
    }
}

fn resolve(executable: &Path, xdg: Option<&OsString>, home: Option<&OsString>) -> Result<PathBuf> {
    let portable = executable
        .parent()
        .context("melonDS executable has no directory")?
        .join("portable");
    let root = if portable.is_dir() {
        portable
    } else {
        let base = if let Some(xdg) = xdg.filter(|value| !value.is_empty()) {
            let path = PathBuf::from(xdg);
            // Reject ambiguous relative XDG roots rather than assuming Qt's
            // build/platform-specific fallback behavior.
            ensure!(
                path.is_absolute(),
                "melonDS XDG_CONFIG_HOME must be absolute"
            );
            path
        } else {
            let home = home
                .filter(|value| !value.is_empty())
                .context("melonDS requires an explicit HOME for default config lookup")?;
            let home = PathBuf::from(home);
            ensure!(home.is_absolute(), "melonDS HOME must be absolute");
            home.join(".config")
        };
        base.join("melonDS")
    };
    let path = root.join("melonDS.toml");
    ensure!(
        path.is_file(),
        "melonDS native configuration must already exist"
    );
    Ok(path)
}
