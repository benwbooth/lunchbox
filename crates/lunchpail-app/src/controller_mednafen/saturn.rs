//! Native Saturn digital pad and six-way taps from pinned ss/smpc.cpp.
use super::{Input, Polarity, binding};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTROLS: &[&str] = &[
    "up", "down", "left", "right", "a", "b", "c", "x", "y", "z", "ls", "rs", "start",
];

pub(crate) struct Player<'a> {
    /// Native virtual index, one-based and sequential across physical ports.
    pub port: u8,
    pub native_id: &'a str,
    pub controls: &'a BTreeMap<String, Input>,
}

pub(crate) fn capacity(multitaps: [bool; 2]) -> usize {
    multitaps
        .iter()
        .map(|enabled| if *enabled { 6 } else { 1 })
        .sum()
}

pub(crate) fn assignments(
    multitaps: [bool; 2],
    players: &[Player<'_>],
) -> Result<BTreeMap<String, String>> {
    let max = capacity(multitaps);
    ensure!(
        !players.is_empty() && players.len() <= max,
        "Invalid Saturn player count"
    );
    let mut result = BTreeMap::new();
    let mut ports = BTreeSet::new();
    let mut devices = BTreeSet::new();
    for player in players {
        ensure!(
            (1..=max).contains(&usize::from(player.port)) && ports.insert(player.port),
            "Invalid or duplicate Saturn port"
        );
        ensure!(
            devices.insert(binding(player.native_id, Input::Button(0), 4096)?),
            "Saturn players share a joystick"
        );
        ensure!(
            player.controls.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|key| player.controls.contains_key(*key)),
            "Saturn digital pad requires all thirteen controls"
        );
        let prefix = format!("ss.input.port{}", player.port);
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
                "Saturn digital controls require signed half-axes"
            );
            ensure!(
                owners.insert(*input),
                "Saturn controls have competing owners"
            );
            result.insert(
                format!("{prefix}.gamepad.{key}"),
                binding(player.native_id, *input, 4096)?,
            );
        }
        // All native Saturn pad entries are Button, not ButtonCR. No rapid
        // setting names are fabricated. Builtin reset remains a separate device.
    }
    for port in 1..=12 {
        if !ports.contains(&port) {
            result.insert(format!("ss.input.port{port}"), "none".into());
        }
    }
    for (index, enabled) in multitaps.into_iter().enumerate() {
        result.insert(
            format!("ss.input.sport{}.multitap", index + 1),
            if enabled { "1" } else { "0" }.into(),
        );
    }
    Ok(result)
}
