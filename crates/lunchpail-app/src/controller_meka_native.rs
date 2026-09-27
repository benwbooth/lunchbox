//! MEKA native controller serialization.
//!
//! MEKA's `meka.inp` grammar is fully described by its source. A caller may
//! render it when it has explicitly resolved the Allegro runtime connection
//! slot; the slot is intentionally not mistaken for a stable device id.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "3bf4a519ab7ee52f53d7a739c638475a8a68ab50";

pub(crate) const CONTROLS: [&str; 8] = [
    "up",
    "down",
    "left",
    "right",
    "button1",
    "button2",
    "start_pause",
    "reset",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Binding {
    Button(u16),
    Axis {
        stick: u16,
        axis: u16,
        positive: bool,
    },
}

impl Binding {
    fn value(self) -> String {
        match self {
            Self::Button(button) => format!("joy_button {button}"),
            Self::Axis {
                stick,
                axis,
                positive,
            } => format!("joy stick {stick} axis {axis} dir {}", positive as u8),
        }
    }
}

/// Render one or two complete MEKA joypad source sections. `connection` is
/// the native file's one-based joypad ordinal (not a physical identity).
pub(crate) fn inputs_ini(ports: &[(u8, u16, BTreeMap<String, Binding>)]) -> Result<String> {
    ensure!(
        !ports.is_empty() && ports.len() <= 2,
        "MEKA has two player ports"
    );
    let mut out = String::new();
    let mut players = BTreeSet::new();
    let mut connections = BTreeSet::new();
    for (player, connection, bindings) in ports {
        ensure!(
            (1..=2).contains(player) && players.insert(*player),
            "MEKA player is invalid or duplicated"
        );
        ensure!(
            *connection >= 1 && connections.insert(*connection),
            "MEKA connection is invalid or duplicated"
        );
        ensure!(
            bindings.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|control| bindings.contains_key(*control)),
            "MEKA joypad source needs every digital control"
        );
        // Section names are not semantically parsed, but this matches the
        // canonical name emitted by MEKA's own writer.
        out.push_str(&format!("[Joypad {player}]\n"));
        out.push_str("type = joypad\n");
        out.push_str(&format!("connection = {connection}\n"));
        out.push_str("enabled = yes\n");
        out.push_str(&format!("player = {player}\n"));
        for control in CONTROLS {
            let field = if control == "start_pause" {
                "player_start_pause".to_owned()
            } else {
                format!("player_{control}")
            };
            out.push_str(&format!("{field} = {}\n", bindings[control].value()));
        }
        out.push('\n');
    }
    Ok(out)
}

/// Refuse only when the caller has not supplied a native connection ordinal.
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "MEKA native controller mapping is unavailable: meka.inp selects Allegro joystick ordinals without a stable persisted device identity"
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    #[test]
    fn renders_native_inp_grammar() {
        let bindings = BTreeMap::from([
            (
                "up".into(),
                super::Binding::Axis {
                    stick: 0,
                    axis: 1,
                    positive: false,
                },
            ),
            (
                "down".into(),
                super::Binding::Axis {
                    stick: 0,
                    axis: 1,
                    positive: true,
                },
            ),
            (
                "left".into(),
                super::Binding::Axis {
                    stick: 0,
                    axis: 0,
                    positive: false,
                },
            ),
            (
                "right".into(),
                super::Binding::Axis {
                    stick: 0,
                    axis: 0,
                    positive: true,
                },
            ),
            ("button1".into(), super::Binding::Button(0)),
            ("button2".into(), super::Binding::Button(1)),
            ("start_pause".into(), super::Binding::Button(6)),
            ("reset".into(), super::Binding::Button(7)),
        ]);
        let ini = super::inputs_ini(&[(1, 2, bindings)]).unwrap();
        assert!(ini.contains("[Joypad 1]\n"));
        assert!(ini.contains("type = joypad"));
        assert!(ini.contains("connection = 2"));
        assert!(ini.contains("player_start_pause = joy_button 6"));
        assert!(ini.contains("player_right = joy stick 0 axis 0 dir 1"));
    }

    #[test]
    fn allegro_ordinal_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
