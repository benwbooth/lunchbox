//! Native pce (not pce_fast) gamepads from the pinned input/gamepad.cpp.
use super::{Input, Polarity, binding};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Two,
    Six,
}

pub(crate) struct Player<'a> {
    pub port: u8,
    pub native_id: &'a str,
    pub mode: Mode,
    pub controls: &'a BTreeMap<String, Input>,
}

impl Mode {
    pub(crate) fn controls(self) -> &'static [&'static str] {
        match self {
            Self::Two => &["up", "down", "left", "right", "i", "ii", "select", "run"],
            Self::Six => &[
                "up", "down", "left", "right", "i", "ii", "iii", "iv", "v", "vi", "select", "run",
            ],
        }
    }
}

/// Assemble all five native ports in one transaction, including disconnecting
/// unused ports. A mode is a saved device choice, not a mapped toggle button.
pub(crate) fn assignments(players: &[Player<'_>]) -> Result<BTreeMap<String, String>> {
    ensure!(
        !players.is_empty() && players.len() <= 5,
        "PCE needs one to five players"
    );
    let mut ports = BTreeSet::new();
    let mut identities = BTreeSet::new();
    let mut result = BTreeMap::new();
    for player in players {
        ensure!(
            (1..=5).contains(&player.port) && ports.insert(player.port),
            "PCE port is invalid or duplicated"
        );
        // Normalize native IDs before comparing ownership; case and optional
        // 0x spelling must not allow one joystick to impersonate two players.
        let identity = binding(player.native_id, Input::Button(0), 4096)?;
        ensure!(
            identities.insert(identity),
            "PCE players share a native joystick"
        );
        ensure!(
            player.controls.len() == player.mode.controls().len()
                && player
                    .mode
                    .controls()
                    .iter()
                    .all(|key| player.controls.contains_key(*key)),
            "PCE mapping must contain every control for the selected pad mode"
        );
        let prefix = format!("pce.input.port{}", player.port);
        result.insert(prefix.clone(), "gamepad".into());
        let mut owners = BTreeSet::new();
        for (key, input) in player.controls {
            ensure!(
                !matches!(
                    input,
                    Input::Absolute {
                        polarity: Polarity::Full | Polarity::FullReversed,
                        ..
                    }
                ),
                "PCE digital controls require signed half-axes"
            );
            ensure!(
                owners.insert(*input),
                "PCE gameplay controls have competing owners"
            );
            result.insert(
                format!("{prefix}.gamepad.{key}"),
                binding(player.native_id, *input, 4096)?,
            );
        }
        for key in ["rapid_i", "rapid_ii", "mode_select"] {
            result.insert(format!("{prefix}.gamepad.{key}"), String::new());
        }
        result.insert(
            format!("{prefix}.gamepad.mode_select.defpos"),
            match player.mode {
                Mode::Two => "2",
                Mode::Six => "6",
            }
            .into(),
        );
        if player.mode == Mode::Two {
            for key in ["iii", "iv", "v", "vi"] {
                result.insert(format!("{prefix}.gamepad.{key}"), String::new());
            }
        }
    }
    for port in 1..=5 {
        if !ports.contains(&port) {
            result.insert(format!("pce.input.port{port}"), "none".into());
        }
    }
    result.insert(
        "pce.input.multitap".into(),
        if ports.iter().any(|port| *port > 1) {
            "1"
        } else {
            "0"
        }
        .into(),
    );
    Ok(result)
}
