//! A7800 standalone native MAME-controller XML writer.
//!
//! Pinned source: 7800-devtools/a7800 commit
//! 7a2afdc1ea08fc331b16b750d8c1f02d4ef62fc8. The target identifies itself as
//! `a7800` and explicitly retains MAME configuration syntax. Its controller
//! devices define the generic MAME input items `IPT_JOYSTICK_*` and
//! `IPT_BUTTON1/2`; this module emits only those source-backed input names.
//! The caller must resolve each `JOYCODE_*`/`KEYCODE_*` token from the same
//! MAME input provider inventory.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "7a2afdc1ea08fc331b16b750d8c1f02d4ef62fc8";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ControllerKind {
    /// `vcs_joystick` exposes four directions and IPT_BUTTON1.
    VcsJoystick,
    /// `proline_joystick` exposes four directions and IPT_BUTTON1/2.
    ProlineJoystick,
}

impl ControllerKind {
    fn button_count(self) -> u8 {
        match self {
            Self::VcsJoystick => 1,
            Self::ProlineJoystick => 2,
        }
    }
}

pub(crate) struct Port<'a> {
    /// Emulated controller port number, one-based.
    pub player: u8,
    pub kind: ControllerKind,
    /// Provider-resolved MAME tokens, keyed by `up`, `down`, `left`, `right`,
    /// and `button1`/`button2` as required by `kind`.
    pub tokens: &'a BTreeMap<String, String>,
}

fn token_is_resolved(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 128
        && token
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && (token.starts_with("JOYCODE_") || token.starts_with("KEYCODE_"))
}

/// Produce a MAME v10 `default.cfg` input section for explicitly resolved
/// A7800 joystick ports. Console switches, paddles, lightguns, keypads,
/// trackballs and mice are intentionally outside this writer until each is
/// separately measured and declared by the caller.
pub(crate) fn controller_xml(ports: &[Port<'_>]) -> Result<String> {
    ensure!(
        !ports.is_empty() && ports.len() <= 2,
        "A7800 has two controller ports"
    );
    let mut players = BTreeSet::new();
    let mut xml = String::from(
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n  <system name=\"default\">\n    <input>\n",
    );
    for port in ports {
        ensure!(
            (1..=2).contains(&port.player) && players.insert(port.player),
            "A7800 port is duplicated or invalid"
        );
        let mut controls = vec![
            ("up", format!("P{}_JOYSTICK_UP", port.player)),
            ("down", format!("P{}_JOYSTICK_DOWN", port.player)),
            ("left", format!("P{}_JOYSTICK_LEFT", port.player)),
            ("right", format!("P{}_JOYSTICK_RIGHT", port.player)),
        ];
        for button in 1..=port.kind.button_count() {
            controls.push((
                if button == 1 { "button1" } else { "button2" },
                format!("P{}_BUTTON{}", port.player, button),
            ));
        }
        ensure!(
            port.tokens.len() == controls.len()
                && controls
                    .iter()
                    .all(|(name, _)| port.tokens.contains_key(*name)),
            "A7800 port is missing a direction or fire binding"
        );
        for (control, native_type) in controls {
            let token = &port.tokens[control];
            ensure!(
                token_is_resolved(token),
                "A7800 binding is not a resolved MAME token"
            );
            xml.push_str(&format!("      <port type=\"{native_type}\"><newseq type=\"standard\">{token}</newseq></port>\n"));
        }
    }
    xml.push_str("    </input>\n  </system>\n</mameconfig>\n");
    Ok(xml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_only_declared_proline_controls() {
        let tokens = BTreeMap::from([
            ("up".into(), "JOYCODE_1_UP".into()),
            ("down".into(), "JOYCODE_1_DOWN".into()),
            ("left".into(), "JOYCODE_1_LEFT".into()),
            ("right".into(), "JOYCODE_1_RIGHT".into()),
            ("button1".into(), "JOYCODE_1_BUTTON1".into()),
            ("button2".into(), "JOYCODE_1_BUTTON2".into()),
        ]);
        let xml = controller_xml(&[Port {
            player: 1,
            kind: ControllerKind::ProlineJoystick,
            tokens: &tokens,
        }])
        .unwrap();
        assert!(xml.contains("P1_JOYSTICK_UP"));
        assert!(xml.contains("P1_BUTTON2"));
        assert!(!xml.contains("START1"));
    }

    #[test]
    fn vcs_requires_no_second_button() {
        let tokens = BTreeMap::from([
            ("up".into(), "KEYCODE_UP".into()),
            ("down".into(), "KEYCODE_DOWN".into()),
            ("left".into(), "KEYCODE_LEFT".into()),
            ("right".into(), "KEYCODE_RIGHT".into()),
            ("button1".into(), "KEYCODE_Z".into()),
        ]);
        assert!(
            controller_xml(&[Port {
                player: 1,
                kind: ControllerKind::VcsJoystick,
                tokens: &tokens
            }])
            .is_ok()
        );
    }
}
