//! Exact private configuration for Nestopia UE 1.53.2's FLTK/SDL2 frontend.
//!
//! The target uses mINI, whose effective data model is a case-insensitive
//! section/key map.  Rebuilding the two private files as maps avoids relying on
//! duplicate-key ordering while preserving every unrelated effective value.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

const INPUT_LIMIT: usize = 2 * 1024 * 1024;

pub(crate) const CONTROLS: [(&str, &str); 8] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("select", "Select"),
    ("start", "Start"),
    ("a", "A"),
    ("b", "B"),
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u8),
    Axis {
        index: u8,
        positive: bool,
    },
    /// Nestopia 1.53.2 ignores SDL's hat index. Only hat zero is representable.
    Hat0 {
        direction: u8,
    },
}

impl Binding {
    pub(crate) fn code(self, player_index: u8) -> Result<String> {
        ensure!(
            player_index <= 9,
            "Nestopia joystick player index does not fit its one-digit grammar"
        );
        let code = match self {
            Self::Button(index) => {
                // The source keys maps by player * 100 + input number. A value
                // at 100 or above aliases a different player's route.
                ensure!(
                    index < 100,
                    "Nestopia joystick button aliases another player"
                );
                format!("j{player_index}b{index}")
            }
            Self::Axis { index, positive } => {
                let half = u16::from(index) * 2 + u16::from(positive);
                ensure!(half < 100, "Nestopia joystick axis aliases another player");
                format!("j{player_index}a{half}")
            }
            Self::Hat0 { direction } => {
                ensure!(
                    direction < 4,
                    "Nestopia needs a cardinal hat-zero direction"
                );
                format!("j{player_index}h{direction}")
            }
        };
        Ok(code)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Pad {
    pub(crate) player: u8,
    pub(crate) joystick: u8,
    pub(crate) bindings: BTreeMap<String, Binding>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Ini {
    sections: BTreeMap<String, BTreeMap<String, String>>,
}

impl Ini {
    fn parse(text: &str, what: &str) -> Result<Self> {
        ensure!(
            text.len() <= INPUT_LIMIT && !text.contains('\0'),
            "{what} is oversized or contains NUL"
        );
        let mut result = Self::default();
        let mut section = None::<String>;
        for (number, source_line) in text.lines().enumerate() {
            ensure!(
                source_line.len() <= 64 * 1024,
                "{what} has an oversized line"
            );
            let line = source_line.trim();
            if line.is_empty() || line.starts_with([';', '#']) {
                continue;
            }
            if line.starts_with('[') {
                let name = line
                    .strip_prefix('[')
                    .and_then(|line| line.strip_suffix(']'))
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .with_context(|| format!("Malformed {what} section at line {}", number + 1))?;
                ensure!(
                    !name.contains(['[', ']', '=', '\r', '\n']),
                    "Malformed {what} section at line {}",
                    number + 1
                );
                let normalized = name.to_ascii_lowercase();
                result.sections.entry(normalized.clone()).or_default();
                section = Some(normalized);
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .with_context(|| format!("Malformed {what} entry at line {}", number + 1))?;
            let key = key.trim();
            ensure!(
                !key.is_empty() && !key.contains(['[', ']', '\r', '\n']),
                "Malformed {what} key at line {}",
                number + 1
            );
            let section = section.as_ref().with_context(|| {
                format!("{what} entry precedes its section at line {}", number + 1)
            })?;
            result
                .sections
                .get_mut(section)
                .expect("current section exists")
                .insert(key.to_ascii_lowercase(), value.trim().to_owned());
        }
        Ok(result)
    }

    fn render(&self) -> String {
        let mut result = String::new();
        for (section, values) in &self.sections {
            result.push('[');
            result.push_str(section);
            result.push_str("]\n");
            for (key, value) in values {
                result.push_str(key);
                result.push_str(" = ");
                result.push_str(value);
                result.push('\n');
            }
            result.push('\n');
        }
        result
    }
}

fn joystick_code(value: &str) -> Result<Option<&str>> {
    if value.is_empty() {
        return Ok(None);
    }
    let bytes = value.as_bytes();
    ensure!(
        bytes.len() >= 4
            && bytes[0] == b'j'
            && bytes[1].is_ascii_digit()
            && matches!(bytes[2], b'a' | b'b' | b'h')
            && bytes[3..].iter().all(u8::is_ascii_digit),
        "Nestopia private input contains a malformed joystick code"
    );
    let input: u16 = value[3..].parse()?;
    ensure!(
        input < 100 && (bytes[2] != b'h' || input < 4),
        "Nestopia private input contains an aliased joystick code"
    );
    Ok(Some(value))
}

/// Build the complete private `input.conf`. Each selected pad section owns all
/// eight standard controls and clear turbo. Any unrelated binding which would
/// compete for one of those exact native events is cleared only in this copy.
/// With a single player, port two stays connected but its managed bindings are
/// cleared so no stale route can drive player two.
pub(crate) fn render_input(original: &str, pads: &[Pad]) -> Result<String> {
    ensure!(
        (1..=2).contains(&pads.len()),
        "Nestopia production mapping needs one or two pads"
    );
    let mut players = BTreeSet::new();
    let mut joysticks = BTreeSet::new();
    let mut claimed = BTreeSet::new();
    let mut selected_sections = BTreeSet::new();
    for pad in pads {
        ensure!(
            (1..=2).contains(&pad.player)
                && players.insert(pad.player)
                && pad.joystick <= 9
                && joysticks.insert(pad.joystick),
            "Nestopia pads need distinct players one/two and joystick indices"
        );
        ensure!(
            pad.bindings.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|(control, _)| pad.bindings.contains_key(*control)),
            "Nestopia needs all eight standard NES controls"
        );
        let section = format!("nespad{}j", pad.player);
        selected_sections.insert(section);
        let mut local = BTreeSet::new();
        for binding in pad.bindings.values() {
            let code = binding.code(pad.joystick)?;
            ensure!(
                local.insert(code.clone()),
                "Nestopia pad reuses one physical input"
            );
            ensure!(
                claimed.insert(code),
                "Nestopia pads have competing physical routes"
            );
        }
    }
    ensure!(
        players == BTreeSet::from([1]) || players == BTreeSet::from([1, 2]),
        "Nestopia needs player one, optionally plus player two"
    );

    let mut ini = Ini::parse(original, "Nestopia input.conf")?;
    // A connected-but-unbound player two must not keep stale bindings: clear
    // exactly the managed keys, leaving unknown keys untouched.
    for port in [1u8, 2u8] {
        if players.contains(&port) {
            continue;
        }
        if let Some(values) = ini.sections.get_mut(&format!("nespad{port}j")) {
            for (_, native_name) in CONTROLS {
                values.insert(native_name.to_ascii_lowercase(), String::new());
            }
            values.insert("turboa".into(), String::new());
            values.insert("turbob".into(), String::new());
        }
    }
    // Validate every effective joystick value before indexing it the way the
    // pinned C++ frontend does, and clear exact conflicts outside owned pads.
    for (section, values) in &mut ini.sections {
        if !section.ends_with('j') {
            continue;
        }
        for value in values.values_mut() {
            if let Some(code) = joystick_code(value)?
                && claimed.contains(code)
                && !selected_sections.contains(section)
            {
                value.clear();
            }
        }
    }
    for pad in pads {
        let values = ini
            .sections
            .entry(format!("nespad{}j", pad.player))
            .or_default();
        for (control, native_name) in CONTROLS {
            values.insert(
                native_name.to_ascii_lowercase(),
                pad.bindings[control].code(pad.joystick)?,
            );
        }
        values.insert("turboa".into(), String::new());
        values.insert("turbob".into(), String::new());
    }
    Ok(ini.render())
}

/// Build the complete private `nestopia.conf` and force the exact ordinary
/// two-controller topology. This excludes auto-detected and special devices.
pub(crate) fn render_main(original: &str) -> Result<String> {
    let mut ini = Ini::parse(original, "Nestopia nestopia.conf")?;
    let settings = ini.sections.entry("nestopia".into()).or_default();
    settings.insert("port1".into(), "1".into());
    settings.insert("port2".into(), "1".into());
    settings.insert("port3".into(), "0".into());
    settings.insert("port4".into(), "0".into());
    settings.insert("portexp".into(), "0".into());
    Ok(ini.render())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(player: u8, joystick: u8) -> Pad {
        Pad {
            player,
            joystick,
            bindings: CONTROLS
                .iter()
                .enumerate()
                .map(|(index, (name, _))| ((*name).into(), Binding::Button(index as u8)))
                .collect(),
        }
    }

    #[test]
    fn private_input_owns_selected_routes_and_clears_competition() {
        let source = "[uij]\nquit = j0b0\n[nespad1j]\na = old\n[other]\nx = keep\n";
        assert!(render_input(source, &[pad(1, 0), pad(2, 1)]).is_err());
        let source = "[uij]\nquit = j0b0\n[nespad1j]\na = j0b7\n[other]\nx = keep\n";
        let output = render_input(source, &[pad(1, 0), pad(2, 1)]).unwrap();
        assert!(output.contains("[uij]\nquit = \n"));
        assert!(output.contains("[other]\nx = keep\n"));
        assert!(output.contains("[nespad1j]\na = j0b6\n"));
        assert!(output.contains("turboa = \n"));
    }

    #[test]
    fn single_player_clears_unowned_port_two_bindings() {
        let source = "[nespad1j]\na = j0b7\n[nespad2j]\na = j1b7\nb = j1b6\nturboa = j1b0\n[other]\ncustom = keep\n";
        let output = render_input(source, &[pad(1, 0)]).unwrap();
        assert!(output.contains("[nespad1j]\na = j0b6\n"));
        assert!(output.contains("[nespad2j]\na = \n"));
        assert!(output.contains("b = \n"));
        assert!(output.contains("turboa = \n"));
        assert!(output.contains("[other]\ncustom = keep\n"));
        assert!(render_input(source, &[]).is_err());
        assert!(render_input(source, &[pad(2, 1)]).is_err());
    }

    #[test]
    fn main_config_forces_only_two_standard_ports() {
        let output =
            render_main("[frontend]\na_mute = 1\n[nestopia]\nport1 = 0\nport3 = 1\n").unwrap();
        assert!(output.contains("[frontend]\na_mute = 1\n"));
        for expected in [
            "port1 = 1",
            "port2 = 1",
            "port3 = 0",
            "port4 = 0",
            "portexp = 0",
        ] {
            assert!(output.contains(expected));
        }
    }

    #[test]
    fn native_encoding_rejects_source_aliases_and_nonzero_hats() {
        assert_eq!(
            Binding::Axis {
                index: 1,
                positive: false
            }
            .code(0)
            .unwrap(),
            "j0a2"
        );
        assert_eq!(Binding::Hat0 { direction: 3 }.code(9).unwrap(), "j9h3");
        assert!(Binding::Button(100).code(0).is_err());
        assert!(
            Binding::Axis {
                index: 50,
                positive: false
            }
            .code(0)
            .is_err()
        );
        assert!(Binding::Hat0 { direction: 4 }.code(0).is_err());
        assert!(Binding::Button(0).code(10).is_err());
    }
}
