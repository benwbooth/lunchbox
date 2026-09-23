//! Lunchbox-owned per-core save/state directories for RetroArch launches.
//!
//! RetroArch keeps one shared frontend save/state tree for every core, which
//! makes per-core save synchronization unsafe to capture. Each Lunchbox
//! RetroArch launch therefore pins `savestate_directory` and
//! `savefile_directory` to per-core directories under the Lunchbox data root
//! through the launch-time private config, and pre-migrates any existing
//! saves for that core from the frontend tree so nothing is left behind.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};

use crate::emulator::{EmulatorExecutable, LaunchPlan};

/// Per-core save/state roots owned by Lunchbox. The sync layer treats these
/// as the exact physical route for `retroarch-core-<name>` slugs.
pub fn lunchbox_route_roots(core_name: &str) -> Vec<PathBuf> {
    let base = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox").map(|dirs| {
        dirs.data_local_dir()
            .join("retroarch-launch")
            .join(core_name)
    });
    match base {
        Some(base) => vec![base.join("saves"), base.join("states")],
        None => Vec::new(),
    }
}

/// Observe the actual files used by RetroArch without claiming that merely
/// enabling auto-load proves the core consumed a previous state.
pub struct AutoSaveObservation {
    state: PathBuf,
    sram: PathBuf,
    state_before: Option<SystemTime>,
    sram_before: Option<SystemTime>,
    auto_state_load_enabled: bool,
    auto_state_save_enabled: bool,
}

impl AutoSaveObservation {
    pub fn for_content(
        core_name: &str,
        content: &Path,
        auto_state_load_enabled: bool,
        auto_state_save_enabled: bool,
    ) -> Option<Self> {
        let stem = content.file_stem()?.to_str()?;
        let roots = lunchbox_route_roots(core_name);
        let state = roots.get(1)?.join(format!("{stem}.state.auto"));
        let sram = roots.first()?.join(format!("{stem}.srm"));
        Some(Self {
            state_before: modified(&state),
            sram_before: modified(&sram),
            state,
            sram,
            auto_state_load_enabled,
            auto_state_save_enabled,
        })
    }

    pub fn launch_notice(&self) -> Option<&'static str> {
        match (
            self.auto_state_load_enabled && self.state_before.is_some(),
            self.sram_before.is_some(),
            self.auto_state_load_enabled,
            self.auto_state_save_enabled,
        ) {
            (true, true, _, _) => Some("Saved state and SRAM found; preparing to resume…"),
            (true, false, _, _) => Some("Saved state found; preparing to resume…"),
            (false, true, _, _) => Some("Saved SRAM found; preparing to load…"),
            (false, false, _, true) => Some("Auto-save enabled; no previous state or SRAM yet"),
            (false, false, true, false) => Some("Auto-resume enabled; no saved state yet"),
            (false, false, false, false) => None,
        }
    }

    pub fn exit_notice(&self) -> Option<(String, bool)> {
        let state_saved = self.auto_state_save_enabled
            && modified(&self.state)
                .is_some_and(|after| self.state_before.is_none_or(|before| after > before));
        let sram_saved = modified(&self.sram)
            .is_some_and(|after| self.sram_before.is_none_or(|before| after > before));
        if !self.auto_state_save_enabled && self.sram_before.is_none() && !sram_saved {
            return None;
        }
        let state = if state_saved {
            format!("save state saved to {}", self.state.display())
        } else if self.auto_state_save_enabled {
            "save state not updated".to_owned()
        } else {
            "automatic save state disabled".to_owned()
        };
        let sram = if sram_saved {
            format!("saved RAM written to {}", self.sram.display())
        } else if self.sram_before.is_some() {
            "saved RAM not updated".to_owned()
        } else {
            "no saved RAM file created".to_owned()
        };
        Some((format!("{state}; {sram}"), state_saved || sram_saved))
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn retroarch_config_base(executable: &EmulatorExecutable) -> Result<PathBuf> {
    let dirs = directories::BaseDirs::new().context("Finding RetroArch config directory")?;
    Ok(match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => dirs
            .home_dir()
            .join(".var/app")
            .join(app_id)
            .join("config/retroarch"),
        _ => dirs.config_dir().join("retroarch"),
    })
}

fn same_core_directory(candidate: &str, core_name: &str) -> bool {
    let normalize = |value: &str| value.to_lowercase().replace([' ', '-', '_'], "");
    normalize(candidate) == normalize(core_name)
}

/// Copy any existing saves/states for this core from the frontend tree into
/// the Lunchbox-owned directories. Best-effort and idempotent: files already
/// present in the destination are never overwritten, and failures are
/// returned so the caller can log them without blocking the launch.
fn migrate_existing_files(
    frontend_base: &Path,
    core_name: &str,
    saves_dir: &Path,
    states_dir: &Path,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for (purpose, target) in [("saves", saves_dir), ("states", states_dir)] {
        let source_root = frontend_base.join(purpose);
        let mut sources = Vec::new();
        if let Ok(entries) = fs::read_dir(&source_root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let path = entry.path();
                if path.is_dir() {
                    if same_core_directory(&name, core_name) {
                        sources.push(path);
                    }
                } else if entry.metadata().is_ok_and(|meta| meta.is_file()) {
                    // Unsorted layouts keep saves directly in the tree.
                    sources.push(path);
                }
            }
        }
        if let Err(error) = fs::create_dir_all(target) {
            warnings.push(format!("could not create {}: {error}", target.display()));
            continue;
        }
        for source in sources {
            let copy = |from: &Path, to: &Path| -> Result<()> {
                if to.exists() {
                    return Ok(());
                }
                fs::copy(from, to)
                    .with_context(|| format!("migrating {} to {}", from.display(), to.display()))?;
                Ok(())
            };
            if source.is_dir() {
                let Ok(entries) = fs::read_dir(&source) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let destination = target.join(entry.file_name());
                    if let Err(error) = copy(&entry.path(), &destination) {
                        warnings.push(format!("{error:#}"));
                    }
                }
            } else if let Some(name) = source.file_name() {
                let destination = target.join(name);
                if let Err(error) = copy(&source, &destination) {
                    warnings.push(format!("{error:#}"));
                }
            }
        }
    }
    warnings
}

/// Attach the per-core save/state directory override to a RetroArch launch
/// plan. Returns warnings (the launch continues regardless).
pub fn attach_launch_save_override(
    plan: &mut LaunchPlan,
    executable: &EmulatorExecutable,
    core_name: &str,
) -> Vec<String> {
    if core_name.trim().is_empty() {
        return Vec::new();
    }
    let mut warnings = Vec::new();
    let roots = lunchbox_route_roots(core_name);
    let (Some(saves_dir), Some(states_dir)) = (roots.first(), roots.get(1)) else {
        return warnings;
    };
    let frontend_base = match retroarch_config_base(executable) {
        Ok(base) => base,
        Err(error) => {
            warnings.push(format!("{error:#}"));
            return warnings;
        }
    };
    warnings.extend(migrate_existing_files(
        &frontend_base,
        core_name,
        saves_dir,
        states_dir,
    ));
    let config = format!(
        "savefile_directory = \"{}\"\nsavestate_directory = \"{}\"\n\
         sort_savefiles_enable = \"false\"\nsort_savestates_enable = \"false\"\n",
        saves_dir.display(),
        states_dir.display()
    );
    let directory = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("launch-display"))
        .expect("Lunchbox data directory");
    if let Err(error) = fs::create_dir_all(&directory) {
        warnings.push(format!(
            "The per-core save directories could not be prepared: {error}"
        ));
        return warnings;
    }
    let config_path = directory.join(format!("retroarch-saves-{core_name}.cfg"));
    if let Err(error) = fs::write(&config_path, config) {
        warnings.push(format!(
            "The per-core save override could not be written: {error}"
        ));
        return warnings;
    }
    if let Err(error) = crate::controller_launch::attach_config(plan, executable, &config_path) {
        warnings.push(format!(
            "The per-core save override could not be attached: {error:#}"
        ));
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_only_claim_save_files_that_exist_or_changed() {
        let temporary = tempfile::tempdir().unwrap();
        let state = temporary.path().join("game.state.auto");
        let sram = temporary.path().join("game.srm");
        let mut observation = AutoSaveObservation {
            state: state.clone(),
            sram: sram.clone(),
            state_before: None,
            sram_before: None,
            auto_state_load_enabled: false,
            auto_state_save_enabled: false,
        };
        assert_eq!(observation.launch_notice(), None);
        assert_eq!(observation.exit_notice(), None);

        observation.auto_state_load_enabled = true;
        observation.auto_state_save_enabled = true;
        assert_eq!(
            observation.launch_notice(),
            Some("Auto-save enabled; no previous state or SRAM yet")
        );
        assert_eq!(
            observation.exit_notice(),
            Some((
                "save state not updated; no saved RAM file created".into(),
                false
            ))
        );

        fs::write(&state, b"state").unwrap();
        fs::write(&sram, b"sram").unwrap();
        let (notice, success) = observation.exit_notice().unwrap();
        assert!(success);
        assert!(notice.contains(&format!("save state saved to {}", state.display())));
        assert!(notice.contains(&format!("saved RAM written to {}", sram.display())));
        observation.state_before = modified(&state);
        observation.sram_before = modified(&sram);
        assert_eq!(
            observation.launch_notice(),
            Some("Saved state and SRAM found; preparing to resume…")
        );
        assert_eq!(
            observation.exit_notice(),
            Some((
                "save state not updated; saved RAM not updated".into(),
                false
            ))
        );
    }

    #[test]
    fn state_only_save_does_not_claim_sram_was_written() {
        let temporary = tempfile::tempdir().unwrap();
        let state = temporary.path().join("Faxanadu.state.auto");
        let observation = AutoSaveObservation {
            state: state.clone(),
            sram: temporary.path().join("Faxanadu.srm"),
            state_before: None,
            sram_before: None,
            auto_state_load_enabled: true,
            auto_state_save_enabled: true,
        };
        fs::write(&state, b"state").unwrap();
        assert_eq!(
            observation.exit_notice(),
            Some((
                format!(
                    "save state saved to {}; no saved RAM file created",
                    state.display()
                ),
                true
            ))
        );
    }

    #[test]
    fn route_roots_are_per_core_and_ordered_saves_then_states() {
        let roots = lunchbox_route_roots("mesen-s");
        assert_eq!(roots.len(), 2);
        assert!(roots[0].ends_with("mesen-s/saves"));
        assert!(roots[1].ends_with("mesen-s/states"));
        let other = lunchbox_route_roots("snes9x");
        assert_ne!(roots[0], other[0]);
    }

    #[test]
    fn migration_copies_core_directory_and_skips_other_cores() {
        let directory = tempfile::tempdir().unwrap();
        let frontend = directory.path().join("retroarch");
        fs::create_dir_all(frontend.join("saves/Mesen-S")).unwrap();
        fs::create_dir_all(frontend.join("saves/Snes9x")).unwrap();
        fs::write(frontend.join("saves/Mesen-S/game.srm"), b"sram").unwrap();
        fs::write(frontend.join("saves/Snes9x/other.srm"), b"other").unwrap();
        let saves = directory.path().join("owned/mesen-s/saves");
        let states = directory.path().join("owned/mesen-s/states");
        let warnings = migrate_existing_files(&frontend, "mesen-s", &saves, &states);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(fs::read(saves.join("game.srm")).unwrap(), b"sram");
        assert!(!saves.join("other.srm").exists());
    }
}
