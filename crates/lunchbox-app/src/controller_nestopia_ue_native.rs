//! Nestopia UE standalone FLTK/SDL2 controller mappings.
//!
//! Functional contract pinned to 0ldsk00l/nestopia tag 1.53.2 (commit
//! 4470a2e99199d8010322eef4bf680fb3760f6eda):
//! - `source/fltkui/inputmanager.cpp` stores joystick bindings in a sibling
//!   `<device>j` INI section as `j<player>b<N>`, `j<player>h<N>`, or
//!   `j<player>a<N>`; axes encode each half as `axis * 2 + polarity`.
//! - `source/fltkui/jg/jg_nes.h` names the standard controller sections
//!   `nespad1` through `nespad4` and the controls Up, Down, Left, Right,
//!   Select, Start, A, B, TurboA and TurboB.
//! - `source/fltkui/jg.cpp` selects standard controllers with `port1` through
//!   `port4 = 1` in the `[nestopia]` section.
//!
//! The writer emits only these exact fragments.  SDL player indices are
//! assigned dynamically by hotplug/order (`SDL_JoystickSetPlayerIndex`), so
//! callers must verify the selected physical device immediately before
//! launch; this module does not claim persistent device identity.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "4470a2e99199d8010322eef4bf680fb3760f6eda";
pub(crate) const PROFILE_ID: &str = "nestopia-ue:standalone-nes-joystick";

/// Standard NES controller fields in the `jg_nes.h` order.
pub(crate) const CONTROLS: [(&str, &str); 10] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("select", "Select"),
    ("start", "Start"),
    ("a", "A"),
    ("b", "B"),
    ("turbo_a", "TurboA"),
    ("turbo_b", "TurboB"),
];

/// Native SDL joystick input code used by `InputManager::remap_js`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u8),
    Axis { index: u8, positive: bool },
    Hat { index: u8, direction: u8 },
}

impl Binding {
    fn code(self, player: u8) -> Result<String> {
        ensure!(
            player <= 9,
            "Nestopia SDL player index must fit j[0-9] grammar"
        );
        match self {
            Self::Button(index) => {
                ensure!(index < 64, "Nestopia SDL button index is out of range");
                Ok(format!("j{player}b{index}"))
            }
            Self::Axis { index, positive } => {
                ensure!(index < 16, "Nestopia SDL axis index is out of range");
                let half = u16::from(index) * 2 + u16::from(positive);
                Ok(format!("j{player}a{half}"))
            }
            Self::Hat { index, direction } => {
                ensure!(index < 4, "Nestopia SDL hat index is out of range");
                ensure!(direction < 4, "Nestopia hat direction must be cardinal");
                // Nestopia fltkui maps SDL_HAT_UP/DOWN/LEFT/RIGHT to 0/1/2/3.
                Ok(format!(
                    "j{player}h{}",
                    u16::from(index) * 4 + direction as u16
                ))
            }
        }
    }
}

/// Render the `[nespadN j]` section fragment for one emulated port.  The
/// eight standard controls are required; TurboA/TurboB are optional and are
/// left to the existing baseline when omitted.
pub(crate) fn input_fragment(
    emulated_port: u8,
    host_player: u8,
    mappings: &BTreeMap<String, Binding>,
) -> Result<String> {
    ensure!(
        (1..=4).contains(&emulated_port),
        "Nestopia supports four NES ports"
    );
    let required = ["up", "down", "left", "right", "select", "start", "a", "b"];
    ensure!(
        required
            .iter()
            .all(|control| mappings.contains_key(*control)),
        "Nestopia needs every standard NES control"
    );
    let mut used = BTreeSet::new();
    let mut result = format!("[nespad{emulated_port}j]\n");
    for (control, key) in CONTROLS {
        let Some(binding) = mappings.get(control) else {
            continue;
        };
        ensure!(used.insert(*binding), "Nestopia reuses one physical input");
        result.push_str(key);
        result.push_str(" = ");
        result.push_str(&binding.code(host_player)?);
        result.push('\n');
    }
    Ok(result)
}

/// Render a `[nestopia]` fragment selecting standard controller hardware for
/// the listed emulated ports.  Other ports and settings remain in the
/// caller's copied baseline.
pub(crate) fn controller_fragment(ports: &[u8]) -> Result<String> {
    ensure!(
        !ports.is_empty() && ports.len() <= 4,
        "Nestopia has four NES ports"
    );
    let mut used = BTreeSet::new();
    let mut result = String::from("[nestopia]\n");
    for &port in ports {
        ensure!(
            (1..=4).contains(&port),
            "Nestopia port must be one through four"
        );
        ensure!(used.insert(port), "Nestopia port is duplicated");
        result.push_str(&format!("port{port} = 1\n"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mappings() -> BTreeMap<String, Binding> {
        [
            (
                "up",
                Binding::Axis {
                    index: 1,
                    positive: false,
                },
            ),
            (
                "down",
                Binding::Axis {
                    index: 1,
                    positive: true,
                },
            ),
            (
                "left",
                Binding::Axis {
                    index: 0,
                    positive: false,
                },
            ),
            (
                "right",
                Binding::Axis {
                    index: 0,
                    positive: true,
                },
            ),
            ("select", Binding::Button(2)),
            ("start", Binding::Button(3)),
            ("a", Binding::Button(0)),
            ("b", Binding::Button(1)),
        ]
        .into_iter()
        .map(|(name, binding)| (name.to_owned(), binding))
        .collect()
    }

    #[test]
    fn input_fragment_uses_pinned_j_codes() {
        let text = input_fragment(1, 0, &mappings()).unwrap();
        assert!(text.starts_with("[nespad1j]\nUp = j0a2\nDown = j0a3\n"));
        assert!(text.contains("A = j0b0\n"));
        assert!(text.ends_with("B = j0b1\n"));
    }

    #[test]
    fn controller_fragment_selects_standard_ports() {
        assert_eq!(
            controller_fragment(&[1, 3]).unwrap(),
            "[nestopia]\nport1 = 1\nport3 = 1\n"
        );
        assert!(controller_fragment(&[1, 1]).is_err());
        assert!(controller_fragment(&[5]).is_err());
    }

    #[test]
    fn malformed_or_ambiguous_mappings_fail_closed() {
        assert!(input_fragment(1, 10, &mappings()).is_err());
        let mut duplicate = mappings();
        duplicate.insert("turbo_a".into(), Binding::Button(0));
        assert!(input_fragment(1, 0, &duplicate).is_err());
        assert!(
            Binding::Hat {
                index: 0,
                direction: 4
            }
            .code(0)
            .is_err()
        );
    }
}
