//! Native SDL mapping names and load precedence from GamepadDevice.
//! SDL instance IDs are process-local; a helper's ID is not proof of child ID.
use anyhow::{Result, ensure};

/// Decode config/Dreamcast.MappingsPath using the pinned Option<vector<string>>
/// grammar. Reject unterminated quoted fields instead of reproducing the native
/// parser's out-of-bounds access. Empty entries are ignored by mapping discovery.
pub(crate) fn mapping_directories(value: &str) -> Result<Vec<std::path::PathBuf>> {
    ensure!(
        value.len() <= 1024 * 1024 && !value.chars().any(char::is_control),
        "Flycast mapping directory list is oversized or contains control characters"
    );
    let bytes = value.as_bytes();
    let mut offset = 0;
    let mut directories = Vec::new();
    while offset < bytes.len() {
        let mut field = Vec::new();
        if bytes[offset] == b'"' {
            offset += 1;
            loop {
                ensure!(
                    offset < bytes.len(),
                    "Unterminated Flycast mapping directory quote"
                );
                if bytes[offset] == b'"' {
                    if offset + 1 == bytes.len() {
                        offset += 1;
                        break;
                    }
                    match bytes[offset + 1] {
                        b'"' => {
                            field.push(b'"');
                            offset += 2;
                        }
                        b';' => {
                            offset += 2;
                            break;
                        }
                        _ => {
                            field.push(b'"');
                            offset += 1;
                        }
                    }
                } else {
                    field.push(bytes[offset]);
                    offset += 1;
                }
            }
        } else {
            while offset < bytes.len() && bytes[offset] != b';' {
                field.push(bytes[offset]);
                offset += 1;
            }
            if offset < bytes.len() {
                offset += 1;
            }
        }
        if !field.is_empty() {
            directories.push(std::path::PathBuf::from(String::from_utf8(field)?));
            ensure!(
                directories.len() <= 32,
                "Too many Flycast mapping directories"
            );
        }
    }
    Ok(directories)
}

/// Encode one owned directory as one native list entry, including semicolons
/// and embedded quotes. This value belongs in [config] Dreamcast.MappingsPath;
/// it does not disable Flycast's separate read-only mapping fallback.
pub(crate) fn mapping_directory_value(path: &std::path::Path) -> Result<String> {
    ensure!(
        path.is_absolute(),
        "Flycast owned mapping directory must be absolute"
    );
    let value = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Flycast mapping directory is not UTF-8"))?;
    ensure!(
        value.len() <= 1024 * 1024 && !value.chars().any(char::is_control),
        "Invalid Flycast mapping directory"
    );
    if value.contains(';') || value.starts_with('"') {
        Ok(format!("\"{}\"", value.replace('"', "\"\"")))
    } else {
        Ok(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Candidate {
    pub filename: String,
    pub instance_specific: bool,
    pub per_game: bool,
    pub arcade: bool,
}

pub(crate) fn filename(
    name: &str,
    native_instance: Option<i32>,
    game_id: Option<&str>,
    arcade: bool,
) -> Result<String> {
    ensure!(
        !name.is_empty() && !name.chars().any(char::is_control),
        "Flycast SDL name is absent or contains control characters"
    );
    let mut value = format!("SDL_{name}");
    if let Some(instance) = native_instance {
        ensure!(instance >= 0, "Flycast native SDL instance ID is invalid");
        value.push_str(&format!("-sdl_joystick_{instance}"));
    }
    if let Some(game) = game_id.filter(|game| !game.is_empty()) {
        ensure!(
            !game.chars().any(char::is_control),
            "Flycast game ID contains control characters"
        );
        value.push('_');
        value.push_str(game);
    }
    if arcade {
        value.push_str("_arcade");
    }
    let mut sanitized: String = value
        .chars()
        .map(|ch| {
            if matches!(ch, '/' | '\\' | ':' | '?' | '*' | '|' | '"' | '<' | '>') {
                '-'
            } else {
                ch
            }
        })
        .collect();
    sanitized.push_str(".cfg");
    ensure!(
        sanitized.len() <= 255,
        "Flycast mapping filename exceeds local component limit"
    );
    Ok(sanitized)
}

/// Exact native order: per-game instance/model, global instance/model, then
/// repeat using Dreamcast fallback when the selected platform is arcade.
pub(crate) fn candidates(
    name: &str,
    native_instance: i32,
    game_id: Option<&str>,
    arcade: bool,
) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    let game_id = game_id.filter(|game| !game.is_empty());
    let systems: &[bool] = if arcade { &[true, false] } else { &[false] };
    for system in systems {
        let scopes: Vec<Option<&str>> = if game_id.is_some() {
            vec![game_id, None]
        } else {
            vec![None]
        };
        for game in scopes {
            for instance in [Some(native_instance), None] {
                candidates.push(Candidate {
                    filename: filename(name, instance, game, *system)?,
                    instance_specific: instance.is_some(),
                    per_game: game.is_some(),
                    arcade: *system,
                });
            }
        }
    }
    Ok(candidates)
}
