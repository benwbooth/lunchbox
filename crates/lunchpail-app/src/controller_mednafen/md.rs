//! Native MD controls and explicit adapter routing, pinned md/genio.cpp.
use super::{Input, Polarity, binding};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Pad {
    Three,
    Six,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Tap {
    None,
    PortOne,
    PortTwo,
    Dual,
    FourWay,
}

impl Tap {
    pub(crate) fn max_players(self) -> u8 {
        match self {
            Self::None => 2,
            Self::PortOne | Self::PortTwo => 5,
            Self::Dual => 8,
            Self::FourWay => 4,
        }
    }
}

impl Pad {
    pub(crate) fn controls(self) -> &'static [&'static str] {
        match self {
            Self::Three => &["up", "down", "left", "right", "a", "b", "c", "start"],
            Self::Six => &[
                "up", "down", "left", "right", "a", "b", "c", "x", "y", "z", "start", "mode",
            ],
        }
    }
    fn device(self) -> &'static str {
        if self == Self::Three {
            "gamepad"
        } else {
            "gamepad6"
        }
    }
}

pub(crate) struct Player<'a> {
    pub port: u8,
    pub pad: Pad,
    pub native_id: &'a str,
    pub controls: &'a BTreeMap<String, Input>,
}

/// Port numbers are native sequential virtual ports, not physical slots.
/// Disable the native automatic input DB so it cannot replace the saved tap.
pub(crate) fn assignments(tap: Tap, players: &[Player<'_>]) -> Result<BTreeMap<String, String>> {
    let (name, max) = match tap {
        Tap::None => ("none", 2),
        Tap::PortOne => ("tp1", 5),
        Tap::PortTwo => ("tp2", 5),
        Tap::Dual => ("tpd", 8),
        Tap::FourWay => ("4way", 4),
    };
    ensure!(
        !players.is_empty() && players.len() <= max,
        "Invalid MD player count"
    );
    let mut result = BTreeMap::from([
        ("md.input.auto".into(), "0".into()),
        ("md.input.multitap".into(), name.into()),
    ]);
    let mut ports = BTreeSet::new();
    let mut devices = BTreeSet::new();
    for player in players {
        ensure!(
            (1..=max).contains(&usize::from(player.port)) && ports.insert(player.port),
            "Invalid or duplicate MD port"
        );
        ensure!(
            devices.insert(binding(player.native_id, Input::Button(0), 4096)?),
            "MD players share a joystick"
        );
        ensure!(
            player.controls.len() == player.pad.controls().len()
                && player
                    .pad
                    .controls()
                    .iter()
                    .all(|key| player.controls.contains_key(*key)),
            "MD pad requires every selected control"
        );
        let prefix = format!("md.input.port{}", player.port);
        let device = player.pad.device();
        result.insert(prefix.clone(), device.into());
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
                "MD digital controls require signed half-axes"
            );
            ensure!(owners.insert(*input), "MD controls have competing owners");
            result.insert(
                format!("{prefix}.{device}.{key}"),
                binding(player.native_id, *input, 4096)?,
            );
        }
        let rapid: &[&str] = if player.pad == Pad::Three {
            &["a", "b", "c"]
        } else {
            &["a", "b", "c", "x", "y", "z"]
        };
        for key in rapid {
            result.insert(format!("{prefix}.{device}.rapid_{key}"), String::new());
        }
    }
    for port in 1..=8 {
        if !ports.contains(&port) {
            result.insert(format!("md.input.port{port}"), "none".into());
        }
    }
    Ok(result)
}
