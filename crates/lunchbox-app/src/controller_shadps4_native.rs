//! shadPS4 native Linux SDL3 input configuration.
//!
//! Pinned to shadps4-emu/shadPS4
//! `678705df8dead58799a3d9a9db38f8fb0c3dbefe`.  Input is not in TOML: the
//! source reads `<UserDir>/input_config/<GameId>.ini`, where each line is an
//! output name and a controller/axis token.  UserDir also contains savedata,
//! system modules, and other runtime data, so launch must isolate only a
//! copied input file while retaining that existing UserDir.

use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) const SOURCE_COMMIT: &str = "678705df8dead58799a3d9a9db38f8fb0c3dbefe";
pub(crate) const PROFILE_ID: &str = "shadps4:standalone-dualshock";

pub(crate) const CONTROLS: [(&str, &str); 14] = [
    ("cross", "cross"),
    ("circle", "circle"),
    ("square", "square"),
    ("triangle", "triangle"),
    ("pad_up", "pad_up"),
    ("pad_down", "pad_down"),
    ("pad_left", "pad_left"),
    ("pad_right", "pad_right"),
    ("l1", "l1"),
    ("r1", "r1"),
    ("back", "back"),
    ("options", "options"),
    ("axis_left_x", "axis_left_x"),
    ("axis_left_y", "axis_left_y"),
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(String),
    Axis(String),
}

fn safe_token(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 128,
        "shadPS4 binding token is invalid"
    );
    ensure!(
        !value
            .chars()
            .any(|c| c.is_control() || c == '=' || c == '#'),
        "shadPS4 binding token contains syntax characters"
    );
    Ok(())
}

/// Render the exact per-game INI syntax parsed by `Input::ParseConfig`.
/// `gamepad` is the source's one-based controller ID (the default is 1).
pub(crate) fn game_input_ini(gamepad: u8, mappings: &BTreeMap<String, Binding>) -> Result<String> {
    ensure!(
        (1..=4).contains(&gamepad),
        "shadPS4 gamepad ID is out of range"
    );
    ensure!(
        mappings.len() == CONTROLS.len(),
        "shadPS4 needs every declared gameplay mapping"
    );
    let mut out = String::new();
    let mut used = std::collections::BTreeSet::new();
    for (target, _) in CONTROLS {
        let binding = mappings
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("shadPS4 mapping {target} is absent"))?;
        let input = match binding {
            Binding::Button(value) | Binding::Axis(value) => value,
        };
        safe_token(input)?;
        ensure!(used.insert(input.clone()), "shadPS4 mapping is duplicated");
        out.push_str(&format!("{target}:{gamepad}={input}:{gamepad}\n"));
    }
    out.push_str(&format!("analog_deadzone:{gamepad}=leftjoystick,5,127\n"));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_gamepad_scoped_ini() {
        let mut map = BTreeMap::new();
        for (name, _) in CONTROLS {
            map.insert(name.to_owned(), Binding::Button(name.to_owned()));
        }
        let text = game_input_ini(2, &map).unwrap();
        assert!(text.contains("cross:2=cross:2"));
        assert!(text.contains("axis_left_x:2=axis_left_x:2"));
    }
}
