//! Replace controller sections in a private PCSX2.ini copy.
use anyhow::{Result, ensure};

pub(crate) fn folder_values(original: &str) -> Result<std::collections::BTreeMap<String, String>> {
    // Validate the same supported INI subset as the overlay before reading it.
    with_controller_profile(original, "")?;
    let mut folders = false;
    let mut values = std::collections::BTreeMap::new();
    for line in original.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if let Some(section) = line.strip_prefix('[') {
            folders = section
                .split(']')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("Folders");
        } else if folders
            && !line.starts_with([';', '#'])
            && let Some((key, value)) = line.split_once('=')
        {
            let key = key.trim().to_ascii_lowercase();
            ensure!(
                !key.is_empty() && values.insert(key, value.trim().to_owned()).is_none(),
                "PCSX2 folder settings contain duplicate or empty keys"
            );
        }
    }
    Ok(values)
}

pub(crate) fn with_folders(
    original: &str,
    values: &std::collections::BTreeMap<String, String>,
) -> Result<String> {
    with_controller_profile(original, "")?;
    let mut folders = false;
    let mut result = String::new();
    for line in original.split_inclusive('\n') {
        let trimmed = line.trim().trim_start_matches('\u{feff}');
        if let Some(section) = trimmed.strip_prefix('[') {
            folders = section
                .split(']')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("Folders");
        }
        if !folders {
            result.push_str(line);
        }
    }
    result.push_str("\n[Folders]\n");
    for (key, value) in values {
        ensure!(
            key.bytes().all(|byte| byte.is_ascii_alphanumeric())
                && !key.is_empty()
                && !value.chars().any(char::is_control)
                && !value.starts_with("<<<"),
            "Invalid PCSX2 folder assignment"
        );
        result.push_str(&format!("{key} = {value}\n"));
    }
    Ok(result)
}

fn owned_section(section: &str) -> bool {
    section.eq_ignore_ascii_case("InputSources")
        || section.eq_ignore_ascii_case("Pad")
        || (1..=8).any(|slot| section.eq_ignore_ascii_case(&format!("Pad{slot}")))
}

/// Preserve non-controller settings. Whole-section replacement avoids duplicate
/// binding lists and inherited macros in SimpleINI's multi-key configuration.
/// This is not sufficient to override separately loaded per-game input profiles.
pub(crate) fn with_controller_profile(original: &str, profile: &str) -> Result<String> {
    ensure!(
        original.len() <= 16 * 1024 * 1024
            && profile.len() <= 1024 * 1024
            && !original.contains('\0')
            && !profile.contains('\0'),
        "Invalid PCSX2 config size or NUL"
    );
    let mut result = String::new();
    let mut owned = false;
    for line in original.split_inclusive('\n') {
        let trimmed = line.trim().trim_start_matches('\u{feff}');
        if let Some(section) = trimmed.strip_prefix('[') {
            let end = section
                .find(']')
                .ok_or_else(|| anyhow::anyhow!("Malformed PCSX2 config section"))?;
            let tail = section[end + 1..].trim();
            ensure!(
                tail.is_empty() || tail.starts_with([';', '#']),
                "Unexpected PCSX2 section suffix"
            );
            owned = owned_section(section[..end].trim());
        } else if !trimmed.starts_with([';', '#'])
            && let Some((_, value)) = trimmed.split_once('=')
        {
            ensure!(
                !value.trim_start().starts_with("<<<"),
                "PCSX2 multiline INI values require normalization before overlay"
            );
        }
        if !owned {
            result.push_str(line);
        }
    }
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result.push_str(profile);
    Ok(result)
}

/// Apply the same owned controls to a private game-settings copy and remove its
/// external InputProfileName selector. Other per-game emulator settings remain.
/// Callers must retain native serial/CRC filename selection and absence guards.
pub(crate) fn with_game_controller_profile(original: &str, profile: &str) -> Result<String> {
    let replaced = with_controller_profile(original, profile)?;
    let mut result = String::with_capacity(replaced.len());
    let mut emucore = false;
    for line in replaced.split_inclusive('\n') {
        let trimmed = line.trim().trim_start_matches('\u{feff}');
        if let Some(section) = trimmed.strip_prefix('[') {
            let end = section
                .find(']')
                .ok_or_else(|| anyhow::anyhow!("Malformed PCSX2 game-settings section"))?;
            emucore = section[..end].trim().eq_ignore_ascii_case("EmuCore");
        } else if emucore
            && !trimmed.starts_with([';', '#'])
            && let Some((key, _)) = trimmed.split_once('=')
            && key.trim().eq_ignore_ascii_case("InputProfileName")
        {
            continue;
        }
        result.push_str(line);
    }
    Ok(result)
}
