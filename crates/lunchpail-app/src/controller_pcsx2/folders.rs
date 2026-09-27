//! Pinned EmuFolders::LoadConfig root-relative path semantics.
use anyhow::{Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

const DEFAULTS: &[(&str, &str)] = &[
    ("Bios", "bios"),
    ("Snapshots", "snaps"),
    ("Savestates", "sstates"),
    ("MemoryCards", "memcards"),
    ("Logs", "logs"),
    ("Cheats", "cheats"),
    ("Patches", "patches"),
    ("Covers", "covers"),
    ("GameSettings", "gamesettings"),
    ("UserResources", "resources"),
    ("Cache", "cache"),
    ("Textures", "textures"),
    ("InputProfiles", "inputprofiles"),
    ("Videos", "videos"),
    ("DebuggerLayouts", "debuggerlayouts"),
    ("DebuggerSettings", "debuggersettings"),
];

/// effective_values must come from the selected native [Folders] settings.
/// Do not infer the user's original data root from the current working folder.
pub(crate) fn resolve(
    data_root: &Path,
    effective_values: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, PathBuf>> {
    ensure!(
        data_root.is_absolute(),
        "PCSX2 original data root must be absolute"
    );
    let mut seen = BTreeSet::new();
    for key in effective_values.keys() {
        ensure!(
            seen.insert(key.to_ascii_lowercase()),
            "Ambiguous case-insensitive PCSX2 folder key"
        );
    }
    let mut folders = BTreeMap::new();
    for &(key, default) in DEFAULTS {
        let value = effective_values
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .map(|(_, value)| value.as_str())
            .unwrap_or(default);
        ensure!(
            value.len() <= 32768 && !value.chars().any(char::is_control),
            "Invalid PCSX2 folder value"
        );
        let path = Path::new(value);
        let root = if key.starts_with("Debugger") {
            data_root.join("inis")
        } else {
            data_root.to_owned()
        };
        folders.insert(
            key.to_owned(),
            if path.is_absolute() {
                path.to_owned()
            } else {
                root.join(path)
            },
        );
    }
    Ok(folders)
}

/// Relocate only the controller-related directories. The caller writes a fresh
/// [Folders] section, replacing existing entries rather than appending aliases.
pub(crate) fn isolated_values(
    original: &BTreeMap<String, PathBuf>,
    game_settings: &Path,
    input_profiles: &Path,
) -> Result<BTreeMap<String, String>> {
    ensure!(
        game_settings.is_absolute() && input_profiles.is_absolute(),
        "PCSX2 private folders must be absolute"
    );
    let mut folders = BTreeMap::new();
    for &(key, _) in DEFAULTS {
        let path = match key {
            "GameSettings" => game_settings,
            "InputProfiles" => input_profiles,
            _ => original
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("PCSX2 original folder was not resolved: {key}"))?
                .as_path(),
        };
        ensure!(path.is_absolute(), "PCSX2 preserved folder is not absolute");
        let value = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("PCSX2 folder is not UTF-8"))?;
        ensure!(
            !value.chars().any(char::is_control),
            "PCSX2 folder contains control characters"
        );
        folders.insert(key.to_owned(), value.to_owned());
    }
    Ok(folders)
}
