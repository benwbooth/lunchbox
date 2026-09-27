//! FCEUX Qt's flat key/value profile selections, not an INI section format.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Selection {
    pub player: u8,
    pub guid: String,
    pub profile: String,
}

/// Interpret explicit assignments using native configSys.cpp's name/value
/// boundaries. Values are not unquoted and trailing whitespace is significant.
pub(crate) fn assignments(text: &str) -> Result<BTreeMap<String, String>> {
    ensure!(!text.contains('\0'), "FCEUX configuration contains NUL");
    let mut values = BTreeMap::new();
    for line in text.split('\n') {
        ensure!(
            line.len() < 4095,
            "FCEUX config line exceeds native reader capacity"
        );
        if line.starts_with('#') {
            continue;
        }
        let Some(equal) = line.find('=') else {
            continue;
        };
        let end = line.find(' ').map_or(equal, |space| space.min(equal));
        if end == 0 {
            continue;
        }
        values.insert(
            line[..end].to_owned(),
            line[equal + 1..].trim_start_matches(' ').to_owned(),
        );
    }
    Ok(values)
}

pub(crate) fn render(original: &str, selections: &[Selection]) -> Result<String> {
    assignments(original)?;
    ensure!(
        !selections.is_empty() && selections.len() <= 4,
        "FCEUX needs one to four selected profiles"
    );
    let mut players = BTreeSet::new();
    let mut result = original.to_owned();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    // Qt loads automatic input presets before ParseGIInput, so presets do
    // not override ROM metadata. Disable inherited presets here rather than
    // allowing them to change Four Score or microphone state behind our back.
    // The launch owner must still account for ROM-selected input types.
    result.push_str(
        "SDL.AutoInputPreset = 0\nSDL.ABStartSelectExit = 0\n\
         SDL.Input.0 = GamePad.0\nSDL.Input.1 = GamePad.1\nSDL.Input.2 = None\n",
    );
    result.push_str(&format!(
        "SDL.FourScore = {}\n",
        u8::from(selections.iter().any(|selection| selection.player >= 3))
    ));
    for selection in selections {
        ensure!(
            (1..=4).contains(&selection.player) && players.insert(selection.player),
            "FCEUX profile players must be distinct and within 1–4"
        );
        ensure!(
            selection.guid.len() == 32
                && selection.guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "FCEUX selected native GUID is invalid"
        );
        ensure!(
            !selection.profile.is_empty()
                && selection.profile.len() <= 48
                && selection
                    .profile
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
            "FCEUX needs a safe private profile basename"
        );
        let prefix = format!("SDL.Input.GamePad.{}", selection.player - 1);
        result.push_str(&format!(
            "{prefix}.DeviceType = Joystick\n{prefix}.DeviceGUID = {}\n{prefix}.Profile = {}\n",
            selection.guid, selection.profile
        ));
    }
    Ok(result)
}

/// Call on the effective merged assignments, including native cfg.d layers.
pub(crate) fn port_guids(values: &BTreeMap<String, String>) -> [String; 4] {
    std::array::from_fn(|port| {
        values
            .get(&format!("SDL.Input.GamePad.{port}.DeviceGUID"))
            .cloned()
            .unwrap_or_default()
    })
}
