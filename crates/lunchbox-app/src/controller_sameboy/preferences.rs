//! Read-only Linux preference selection for pinned SameBoy SDL.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::CString,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

fn accessible(path: &Path, mode: libc::c_int) -> Result<bool> {
    let path = CString::new(path.as_os_str().as_bytes())?;
    // Match native access(), including real-user permission semantics. This
    // does not open, create, truncate or change the preference file.
    Ok(unsafe { libc::access(path.as_ptr(), mode) } == 0)
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
        if accessible(&local, libc::F_OK)? {
            local
        } else {
            data.join("prefs.bin")
        }
    } else {
        local
    };
    if accessible(&resource, libc::R_OK | libc::W_OK)? {
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
