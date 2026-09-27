//! Native module/game override paths from general.cpp GeneratePath.
use anyhow::{Result, ensure};
use std::path::{Component, Path, PathBuf};

pub(crate) fn module(base: &Path, system: &str) -> Result<PathBuf> {
    ensure!(
        base.is_absolute()
            && matches!(
                system,
                "gb" | "gba"
                    | "lynx"
                    | "ngp"
                    | "wswan"
                    | "vb"
                    | "gg"
                    | "sms"
                    | "snes"
                    | "ss"
                    | "psx"
                    | "md"
                    | "snes_faust"
                    | "pce"
                    | "pce_fast"
                    | "nes"
            ),
        "Mednafen handheld override needs an absolute base and supported module"
    );
    Ok(base.join(format!("{system}.cfg")))
}

/// file_base is Mednafen's resolved FileBase, not a display title or hash.
/// Resolve archives and native content-path splitting before calling this.
/// directory is effective filesys.path_pgconfig after loading module overrides.
pub(crate) fn game(
    base: &Path,
    system: &str,
    file_base: &str,
    directory: &Path,
) -> Result<PathBuf> {
    module(base, system)?;
    ensure!(
        !file_base.is_empty()
            && !file_base.contains('\0')
            && Path::new(file_base).components().count() == 1
            && matches!(
                Path::new(file_base).components().next(),
                Some(Component::Normal(_))
            ),
        "Mednafen requires a resolved native content basename"
    );
    Ok(base
        .join(directory)
        .join(format!("{file_base}.{system}.cfg")))
}

/// Track an optional override that was absent at preparation time so it cannot
/// appear later and silently take precedence over the captured mappings.
pub(crate) struct MissingOverride {
    path: PathBuf,
}

impl MissingOverride {
    pub(crate) fn capture(path: PathBuf) -> Result<Self> {
        ensure!(path.is_absolute(), "Mednafen override must be absolute");
        let result = Self { path };
        result.verify()?;
        Ok(result)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        match std::fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
            Ok(_) => anyhow::bail!(
                "Mednafen previously absent override now exists: {}",
                self.path.display()
            ),
        }
    }
}
