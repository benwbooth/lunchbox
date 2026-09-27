//! Upgrade compatibility for the Lunchpail rename. Old names below are data
//! contracts, not branding: changing them would strand saves and preferences.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use directories::ProjectDirs;

pub(crate) struct AppDirectories {
    data: PathBuf,
    local_data: PathBuf,
    config: PathBuf,
    cache: PathBuf,
}

impl AppDirectories {
    fn from_project(dirs: ProjectDirs) -> Self {
        Self {
            data: dirs.data_dir().to_owned(),
            local_data: dirs.data_local_dir().to_owned(),
            config: dirs.config_dir().to_owned(),
            cache: dirs.cache_dir().to_owned(),
        }
    }
    pub(crate) fn data_dir(&self) -> &Path {
        &self.data
    }
    pub(crate) fn data_local_dir(&self) -> &Path {
        &self.local_data
    }
    pub(crate) fn config_dir(&self) -> &Path {
        &self.config
    }
    pub(crate) fn cache_dir(&self) -> &Path {
        &self.cache
    }
}

fn current_dirs() -> Option<AppDirectories> {
    ProjectDirs::from("com", "Lunchpail", "Lunchpail").map(AppDirectories::from_project)
}

fn legacy_dirs() -> Option<AppDirectories> {
    // Flatpak changes XDG directories with the app ID. Its old profile is
    // outside the new sandbox's XDG base, so ordinary ProjectDirs misses it.
    #[cfg(target_os = "linux")]
    if std::env::var("FLATPAK_ID").as_deref() == Ok("io.github.benwbooth.Lunchpail") {
        let old = directories::BaseDirs::new()?
            .home_dir()
            .join(".var/app/io.github.benwbooth.Lunchbox");
        if old.join("data/lunchbox").is_dir() {
            return Some(AppDirectories {
                data: old.join("data/lunchbox"),
                local_data: old.join("data/lunchbox"),
                config: old.join("config/lunchbox"),
                cache: old.join("cache/lunchbox"),
            });
        }
    }
    ProjectDirs::from("com", "Lunchbox", "Lunchbox").map(AppDirectories::from_project)
}

pub(crate) fn project_dirs() -> Option<AppDirectories> {
    let current = current_dirs()?;
    let legacy = legacy_dirs()?;
    // Before the GUI owns the instance lock, helpers must use the same store
    // as the old running app. Also supports Windows installations where a
    // compatibility directory link cannot be created without extra rights.
    if !current.data_local_dir().exists() && legacy.data_local_dir().exists() {
        Some(legacy)
    } else {
        Some(current)
    }
}

pub(crate) fn migrate_user_directories() -> Result<()> {
    let current = current_dirs().context("Lunchpail application directories are unavailable")?;
    let legacy = legacy_dirs().context("legacy application directories are unavailable")?;
    for (old, new) in [
        (legacy.data_local_dir(), current.data_local_dir()),
        (legacy.data_dir(), current.data_dir()),
        (legacy.config_dir(), current.config_dir()),
        (legacy.cache_dir(), current.cache_dir()),
    ] {
        migrate_directory(old, new)?;
    }
    Ok(())
}

fn migrate_directory(old: &Path, new: &Path) -> Result<()> {
    if !old.exists() || old == new {
        return Ok(());
    }
    if new.exists() {
        ensure!(
            old.canonicalize()? == new.canonicalize()?,
            "Both {} and {} exist; refusing to overwrite either profile",
            old.display(),
            new.display()
        );
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(old)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "Legacy application directory is not a regular directory: {}",
        old.display()
    );
    if let Some(parent) = new.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(old, new)
        .with_context(|| format!("moving {} to {}", old.display(), new.display()))?;
    // Saved emulator configs and artwork records contain absolute paths.
    // Keep those paths valid without rewriting user-owned files or databases.
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(new, old);
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(new, old);
    if let Err(error) = linked {
        std::fs::rename(new, old).context("restoring the original application directory")?;
        #[cfg(windows)]
        {
            eprintln!("LUNCHPAIL_PROFILE_LEGACY_PATH: {error}");
            return Ok(());
        }
        #[cfg(not(windows))]
        return Err(error).context("preserving existing absolute application paths");
    }
    Ok(())
}

pub(crate) fn compatible_format(actual: &str, current: &str) -> bool {
    actual == current || actual == current.replace("lunchpail", "lunchbox")
}

/// Called once at process entry, before Qt, worker threads or model runtimes.
pub(crate) fn import_legacy_environment() {
    for (key, value) in std::env::vars_os() {
        if let Some(suffix) = key.to_str().and_then(|key| key.strip_prefix("LUNCHBOX_")) {
            let current = format!("LUNCHPAIL_{suffix}");
            if std::env::var_os(&current).is_none() {
                // SAFETY: run() calls this before starting any other threads.
                unsafe { std::env::set_var(current, value) };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_formats_remain_readable_but_unknown_formats_do_not() {
        assert!(compatible_format("lunchbox-profile", "lunchpail-profile"));
        assert!(compatible_format("lunchpail-profile", "lunchpail-profile"));
        assert!(!compatible_format("other-profile", "lunchpail-profile"));
    }

    #[test]
    fn rename_preserves_saved_absolute_paths_and_is_repeatable() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("lunchbox");
        let new = temp.path().join("lunchpail");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("save.srm"), b"save data").unwrap();
        migrate_directory(&old, &new).unwrap();
        assert_eq!(std::fs::read(old.join("save.srm")).unwrap(), b"save data");
        #[cfg(unix)]
        assert_eq!(std::fs::read(new.join("save.srm")).unwrap(), b"save data");
        migrate_directory(&old, &new).unwrap();
    }

    #[test]
    fn rename_refuses_to_merge_or_overwrite_profiles() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("lunchbox");
        let new = temp.path().join("lunchpail");
        std::fs::create_dir(&old).unwrap();
        std::fs::create_dir(&new).unwrap();
        std::fs::write(old.join("state.db"), b"old").unwrap();
        std::fs::write(new.join("state.db"), b"new").unwrap();
        assert!(migrate_directory(&old, &new).is_err());
        assert_eq!(std::fs::read(old.join("state.db")).unwrap(), b"old");
        assert_eq!(std::fs::read(new.join("state.db")).unwrap(), b"new");
    }
}
