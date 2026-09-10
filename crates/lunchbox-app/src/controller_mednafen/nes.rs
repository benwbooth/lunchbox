//! Native NES pad contracts from pinned src/nes/input.cpp and nes.cpp.
use super::{Input, Polarity, binding};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTROLS: &[&str] = &["up", "down", "left", "right", "a", "b", "select", "start"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Adapter {
    TwoPlayer,
    FourScore,
    FamicomFourPlayer,
}

pub(crate) struct Player<'a> {
    pub port: u8,
    pub native_id: &'a str,
    pub controls: &'a BTreeMap<String, Input>,
}

/// These settings do not override CurGame::DesiredInput. The launch owner must
/// resolve ROM/UNIF device metadata before claiming the selected device is used.
pub(crate) fn assignments(
    adapter: Adapter,
    players: &[Player<'_>],
) -> Result<BTreeMap<String, String>> {
    let max = if adapter == Adapter::TwoPlayer { 2 } else { 4 };
    ensure!(
        !players.is_empty() && players.len() <= max,
        "Invalid NES player count"
    );
    let mut ports = BTreeSet::new();
    let mut devices = BTreeSet::new();
    let mut result = BTreeMap::new();
    for player in players {
        ensure!(
            (1..=max).contains(&usize::from(player.port)) && ports.insert(player.port),
            "NES player port is invalid or duplicated"
        );
        ensure!(
            devices.insert(binding(player.native_id, Input::Button(0), 4096)?),
            "NES players share a native joystick"
        );
        ensure!(
            player.controls.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|key| player.controls.contains_key(*key)),
            "NES pad requires all eight controls"
        );
        let prefix = format!("nes.input.port{}", player.port);
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
                "NES digital controls require signed half-axes"
            );
            ensure!(owners.insert(*input), "NES controls have competing owners");
            result.insert(
                format!("{prefix}.gamepad.{key}"),
                binding(player.native_id, *input, 4096)?,
            );
        }
        for key in ["rapid_a", "rapid_b"] {
            result.insert(format!("{prefix}.gamepad.{key}"), String::new());
        }
    }
    for port in 1..=4 {
        if !ports.contains(&port) {
            result.insert(format!("nes.input.port{port}"), "none".into());
        }
    }
    result.insert(
        "nes.nofs".into(),
        if adapter == Adapter::FourScore {
            "0"
        } else {
            "1"
        }
        .into(),
    );
    result.insert(
        "nes.input.fcexp".into(),
        if adapter == Adapter::FamicomFourPlayer {
            "4player"
        } else {
            "none"
        }
        .into(),
    );
    Ok(result)
}
