//! Snes9x GTK configuration overrides; original text remains unchanged.
use super::Binding;
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTROLS: [&str; 12] = [
    "Up", "Down", "Left", "Right", "Start", "Select", "A", "B", "X", "Y", "L", "R",
];

pub(crate) struct Pad {
    /// One-based emulated SNES player, independent of the physical device.
    pub player: u8,
    pub bindings: BTreeMap<String, Binding>,
}

/// ConfigFile::LoadFile replaces earlier duplicate section/key entries.
/// Appending selected overrides therefore preserves unrelated text/settings.
/// Both binding banks for selected players are owned by this session.
pub(crate) fn render(original: &str, pads: &[Pad]) -> Result<String> {
    ensure!(
        !original.contains('\0') && !original.lines().any(|line| line.trim_end().ends_with('\\')),
        "Snes9x controller configuration with NUL or continued lines needs normalization first"
    );
    ensure!(
        !pads.is_empty() && pads.len() <= 5,
        "Snes9x needs one to five selected players"
    );
    let mut players = BTreeSet::new();
    let mut routing = BTreeSet::new();
    for pad in pads {
        ensure!(
            (1..=5).contains(&pad.player) && players.insert(pad.player),
            "Snes9x players must be distinct and within 1–5"
        );
        ensure!(
            pad.bindings.len() == CONTROLS.len()
                && CONTROLS.iter().all(|name| pad.bindings.contains_key(*name)),
            "Snes9x needs all twelve standard SNES controls"
        );
        for binding in pad.bindings.values() {
            ensure!(
                routing.insert(binding.routing_key()?),
                "Snes9x gameplay bindings have competing owners"
            );
        }
    }
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut result = original.to_owned();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push_str(newline);
    }
    // Native port 0 joypad is player 1; port 1 multitap is players 2–5.
    // Only change the physical SNES ports needed by the requested players.
    result.push_str(&format!("[Input]{newline}"));
    if players.contains(&1) {
        result.push_str(&format!("ControllerPort0 = joypad{newline}"));
    }
    if players.iter().any(|player| *player >= 2) {
        let device = if players.iter().any(|player| *player >= 3) {
            "multitap"
        } else {
            "joypad"
        };
        result.push_str(&format!("ControllerPort1 = {device}{newline}"));
    }
    for pad in pads {
        for bank in [pad.player - 1, pad.player + 4] {
            result.push_str(&format!("[Joypad {bank}]{newline}"));
            for name in CONTROLS {
                let value = if bank < 5 {
                    pad.bindings[name].as_setting()?
                } else {
                    "Unset".into()
                };
                result.push_str(&format!("{name} = {value}{newline}"));
            }
            for prefix in ["Turbo", "Sticky"] {
                for button in ["A", "B", "X", "Y", "L", "R"] {
                    result.push_str(&format!("{prefix} {button} = Unset{newline}"));
                }
            }
        }
    }
    // Native GTK maps every bank, then shortcuts. Clear only old routes
    // that compete with a newly assigned input, including another player's
    // banks; keep unrelated controller inputs and keyboard shortcuts.
    let mut section = String::new();
    let mut effective = BTreeMap::new();
    for line in original.lines() {
        let line = line.trim();
        if line.starts_with(['#', ';']) {
            continue;
        }
        if line.starts_with('[') {
            if let Some(name) = line
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
            {
                section = name.trim().to_owned();
            }
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let mut decoded = String::new();
            let mut chars = value.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '#' {
                    if chars.peek() == Some(&'#') {
                        chars.next();
                    } else {
                        break;
                    }
                }
                decoded.push(ch);
            }
            let value = decoded.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .unwrap_or(value);
            effective.insert((section.clone(), key.trim().to_owned()), value.to_owned());
        }
    }
    let mut conflicts: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for ((section, key), value) in effective {
        let bank = section
            .strip_prefix("Joypad ")
            .and_then(|bank| bank.parse::<u8>().ok())
            .filter(|bank| *bank < 10);
        if section != "Shortcuts" && bank.is_none() {
            continue;
        }
        if bank.is_some_and(|bank| players.contains(&(bank % 5 + 1))) {
            continue;
        }
        let words: Vec<_> = value.split_whitespace().collect();
        if words.first() != Some(&"Joystick") {
            continue;
        }
        let Some(device) = words
            .get(1)
            .and_then(|word| word.parse::<u8>().ok())
            .filter(|device| (1..=15).contains(device))
        else {
            continue;
        };
        let input = match words.get(2).copied() {
            Some("Button") => words
                .get(3)
                .and_then(|word| word.parse::<u16>().ok())
                .map(super::JoystickInput::Button),
            Some("Axis") => words
                .get(3)
                .and_then(|word| word.parse::<u16>().ok())
                .and_then(|index| {
                    words.get(4).map(|direction| super::JoystickInput::Axis {
                        index,
                        positive: *direction == "+",
                        threshold_percent: 50,
                    })
                }),
            _ => None,
        };
        if let Some(input) = input {
            let old = Binding {
                joystick: device - 1,
                input,
            };
            if old
                .routing_key()
                .ok()
                .is_some_and(|key| routing.contains(&key))
            {
                ensure!(
                    !key.is_empty() && !key.contains(['[', ']', '\\']),
                    "Invalid Snes9x conflicting input key"
                );
                conflicts.entry(section).or_default().insert(key);
            }
        }
    }
    for (section, keys) in conflicts {
        result.push_str(&format!("[{section}]{newline}"));
        for key in keys {
            result.push_str(&format!("{key} = Unset{newline}"));
        }
    }
    Ok(result)
}
