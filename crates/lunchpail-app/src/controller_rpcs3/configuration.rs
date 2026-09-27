//! Native cfg_input YAML, generated independently of user-owned profiles.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Player<'a> {
    pub player: u8,
    /// Exact identifier returned by the selected native handler, not model GUID.
    pub device: &'a str,
    pub controls: &'a BTreeMap<String, String>,
}

#[derive(Clone, Copy)]
pub(crate) struct AnalogSettings {
    pub stick_deadzone: u16,
    pub stick_anti_deadzone: u16,
    pub trigger_threshold: u16,
}

impl Default for AnalogSettings {
    fn default() -> Self {
        // Pinned SDL handler defaults: 8000, truncate(0.13 * 32767), zero.
        Self {
            stick_deadzone: 8000,
            stick_anti_deadzone: 4259,
            trigger_threshold: 0,
        }
    }
}

fn quoted(value: &str) -> Result<String> {
    // JSON double-quoted strings are a YAML-compatible scalar representation.
    Ok(serde_json::to_string(value)?)
}

pub(crate) fn render(players: &[Player<'_>], analog: AnalogSettings) -> Result<String> {
    ensure!(
        !players.is_empty() && players.len() <= 7,
        "RPCS3 requires one to seven players"
    );
    ensure!(
        [
            analog.stick_deadzone,
            analog.stick_anti_deadzone,
            analog.trigger_threshold
        ]
        .iter()
        .all(|value| *value < 32767),
        "RPCS3 analog threshold must leave a usable SDL range"
    );
    let mut by_player = BTreeMap::new();
    let mut devices = BTreeSet::new();
    for player in players {
        super::player_key(player.player)?;
        ensure!(
            by_player.insert(player.player, player).is_none() && devices.insert(player.device),
            "RPCS3 player/device assignments must be distinct"
        );
        ensure!(
            !player.device.is_empty()
                && player.device.len() <= 4096
                && !player.device.chars().any(char::is_control),
            "RPCS3 native device identifier is missing or invalid"
        );
        super::validate_controls(player.controls)?;
    }
    let mut yaml = String::new();
    for id in 1..=7 {
        yaml.push_str(&format!("{}:\n", quoted(&super::player_key(id)?)?));
        let Some(player) = by_player.get(&id) else {
            yaml.push_str("  Handler: Null\n");
            continue;
        };
        yaml.push_str(&format!(
            "  Handler: SDL\n  Device: {}\n  Buddy Device: \"\"\n  Config:\n",
            quoted(player.device)?
        ));
        let mut controls = player.controls.clone();
        for key in [
            "PS Button",
            "Pressure Intensity Button",
            "Analog Limiter Button",
            "Orientation Reset Button",
        ] {
            controls.entry(key.to_owned()).or_default();
        }
        for (key, value) in controls {
            yaml.push_str(&format!("    {}: {}\n", quoted(&key)?, quoted(&value)?));
        }
        for side in ["Left", "Right"] {
            yaml.push_str(&format!("    {side} Stick Deadzone: {}\n    {side} Stick Anti-Deadzone: {}\n    {side} Trigger Threshold: {}\n",
                analog.stick_deadzone, analog.stick_anti_deadzone, analog.trigger_threshold));
        }
        yaml.push_str("    Device Class Type: 0\n    Orientation Enabled: false\n");
    }
    Ok(yaml)
}
