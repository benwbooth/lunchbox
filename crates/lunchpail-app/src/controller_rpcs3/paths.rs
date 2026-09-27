//! Linux config lookup from pinned Utilities/File.cpp, without native execution.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) struct ConfigDirectory {
    executable: PathBuf,
    cwd: PathBuf,
    xdg: Option<OsString>,
    home: Option<OsString>,
    root: PathBuf,
}

impl ConfigDirectory {
    pub(crate) fn capture(executable: &Path, cwd: &Path) -> Result<Self> {
        ensure!(
            cfg!(target_os = "linux"),
            "RPCS3 directory discovery currently requires Linux"
        );
        let executable = executable.canonicalize()?;
        let cwd = cwd.canonicalize()?;
        let xdg = std::env::var_os("XDG_CONFIG_HOME");
        let home = std::env::var_os("HOME");
        let root = resolve(&executable, &cwd, xdg.as_ref(), home.as_ref())?;
        Ok(Self {
            executable,
            cwd,
            xdg,
            home,
            root,
        })
    }

    pub(crate) fn input_directory(&self) -> PathBuf {
        self.root.join("input_configs/global")
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            std::env::var_os("XDG_CONFIG_HOME") == self.xdg
                && std::env::var_os("HOME") == self.home,
            "RPCS3 inherited configuration environment changed"
        );
        ensure!(
            resolve(
                &self.executable,
                &self.cwd,
                self.xdg.as_ref(),
                self.home.as_ref()
            )? == self.root,
            "RPCS3 portable/configuration directory changed"
        );
        Ok(())
    }
}

fn resolve(
    executable: &Path,
    cwd: &Path,
    xdg: Option<&OsString>,
    home: Option<&OsString>,
) -> Result<PathBuf> {
    let portable = executable
        .parent()
        .context("RPCS3 executable has no parent")?
        .join("portable");
    let root = if portable.is_dir() {
        portable
    } else {
        // Native getenv distinguishes an absent variable from an empty value.
        // Append strings rather than Path::join to preserve that distinction.
        let mut base = if let Some(xdg) = xdg {
            xdg.clone()
        } else if let Some(home) = home {
            let mut base = home.clone();
            base.push("/.config");
            base
        } else {
            OsString::from("./config")
        };
        base.push("/rpcs3/");
        let path = PathBuf::from(base);
        if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        }
    };
    let root = root
        .canonicalize()
        .context("RPCS3 configuration directory must already exist")?;
    ensure!(root.is_dir(), "RPCS3 configuration root is not a directory");
    Ok(root)
}
