//! Read-only preference selection for pinned SameBoy SDL.
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

/// Readability/writability probe without opening or creating the file.
/// Unix checks real-user access bits; other hosts check existence plus the
/// readonly flag, which is weaker and documented at the call sites.
fn accessible(path: &Path, write: bool) -> Result<bool> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Ok(false);
    }
    if write && metadata.permissions().readonly() {
        return Ok(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let native = std::ffi::CString::new(path.as_os_str().as_bytes()).context("NUL in path")?;
        let mode = if write {
            libc::R_OK | libc::W_OK
        } else {
            libc::R_OK
        };
        Ok(unsafe { libc::access(native.as_ptr(), mode) } == 0)
    }
    #[cfg(not(unix))]
    {
        Ok(true)
    }
}

/// Resource/data roots must be established from the selected executable.
/// In particular, None means no compiled DATA_DIR, not "unknown DATA_DIR".
pub(crate) fn resolve(
    resource_directory: &Path,
    compiled_data_directory: Option<&Path>,
    cwd: &Path,
) -> Result<PathBuf> {
    ensure!(
        resource_directory.is_absolute() && cwd.is_absolute(),
        "SameBoy preference resolution requires absolute runtime paths"
    );
    let local = resource_directory.join("prefs.bin");
    let resource = if let Some(data) = compiled_data_directory {
        ensure!(
            data.is_absolute(),
            "SameBoy compiled data root must be resolved"
        );
        if accessible(&local, false)? {
            local
        } else {
            data.join("prefs.bin")
        }
    } else {
        local
    };
    if accessible(&resource, true)? {
        return Ok(resource);
    }
    let root = if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        ensure!(
            !xdg.is_empty(),
            "SameBoy SDL empty XDG_DATA_HOME is not supported"
        );
        PathBuf::from(xdg)
    } else {
        let home =
            std::env::var_os("HOME").context("SameBoy SDL has neither XDG_DATA_HOME nor HOME")?;
        ensure!(!home.is_empty(), "SameBoy SDL HOME must not be empty");
        PathBuf::from(home).join(".local/share")
    };
    // Preserve SDL's relative environment-path behavior, resolving against
    // launch cwd without SDL_GetPrefPath's mkdir side effects.
    Ok(cwd.join(root).join("SameBoy/prefs.bin"))
}
