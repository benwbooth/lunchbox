//! Native snes module: pinned src/snes/interface.cpp GamepadIDII and PortInfo.
use super::{Input, Polarity, binding};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTROLS: &[&str] = &[
    "up", "down", "left", "right", "a", "b", "x", "y", "l", "r", "select", "start",
];
const RAPID: &[&str] = &["rapid_a", "rapid_b", "rapid_x", "rapid_y"];

pub(crate) struct Player<'a> {
    /// Native logical port: 1/2 are primary; 3..5 hang off port 2,
    /// while 6..8 hang off port 1. Not physical SDL device enumeration.
    pub port: u8,
    pub native_id: &'a str,
    pub controls: &'a BTreeMap<String, Input>,
}

pub(crate) fn assignments(players: &[Player<'_>]) -> Result<BTreeMap<String, String>> {
    ensure!(
        !players.is_empty() && players.len() <= 8,
        "SNES needs one to eight players"
    );
    let mut ports = BTreeSet::new();
    let mut devices = BTreeSet::new();
    let mut result = BTreeMap::new();
    for player in players {
        ensure!(
            (1..=8).contains(&player.port) && ports.insert(player.port),
            "Invalid or duplicate SNES port"
        );
        ensure!(
            devices.insert(binding(player.native_id, Input::Button(0), 4096)?),
            "SNES players share a joystick"
        );
        ensure!(
            player.controls.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|key| player.controls.contains_key(*key)),
            "SNES pad requires all twelve controls"
        );
        let prefix = format!("snes.input.port{}", player.port);
        // Tap subports have only one device: their selector setting does not exist.
        if player.port <= 2 {
            result.insert(prefix.clone(), "gamepad".into());
        }
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
                "SNES digital controls require signed half-axes"
            );
            ensure!(owners.insert(*input), "SNES controls have competing owners");
            result.insert(
                format!("{prefix}.gamepad.{key}"),
                binding(player.native_id, *input, 4096)?,
            );
        }
        for key in RAPID {
            result.insert(format!("{prefix}.gamepad.{key}"), String::new());
        }
    }
    let tap1 = ports.iter().any(|port| *port >= 6);
    let tap2 = ports.iter().any(|port| (3..=5).contains(port));
    for port in 1..=8 {
        if !ports.contains(&port) {
            let prefix = format!("snes.input.port{port}");
            if port <= 2 {
                // A tap requires a primary gamepad even if its A slot is unused.
                let tap = if port == 1 { tap1 } else { tap2 };
                result.insert(prefix.clone(), if tap { "gamepad" } else { "none" }.into());
            }
            for key in CONTROLS.iter().chain(RAPID) {
                result.insert(format!("{prefix}.gamepad.{key}"), String::new());
            }
        }
    }
    result.insert(
        "snes.input.port1.multitap".into(),
        if tap1 { "1" } else { "0" }.into(),
    );
    result.insert(
        "snes.input.port2.multitap".into(),
        if tap2 { "1" } else { "0" }.into(),
    );
    Ok(result)
}

/// Faust consumes virtual ports sequentially across physical ports, unlike
/// the older snes module's fixed 1,6,7,8 / 2,3,4,5 ordering. Keep the saved
/// physical-slot convention stable and translate explicitly at configuration.
/// Source: snes_faust/input.cpp MapDevices and input/gamepad.cpp GamepadIDII.
pub(crate) fn faust_assignments(players: &[Player<'_>]) -> Result<BTreeMap<String, String>> {
    let standard = assignments(players)?;
    let tap1 = standard["snes.input.port1.multitap"] == "1";
    let tap2 = standard["snes.input.port2.multitap"] == "1";
    let mut slots = vec![1u8];
    if tap1 {
        slots.extend([6, 7, 8]);
    }
    slots.push(2);
    if tap2 {
        slots.extend([3, 4, 5]);
    }
    let mut result = BTreeMap::new();
    for native_port in 1..=8 {
        let prefix = format!("snes_faust.input.port{native_port}");
        let logical = slots.get(native_port - 1);
        let active = logical.is_some_and(|port| players.iter().any(|player| player.port == *port));
        // Faust supports none on every virtual slot, even inside a multitap.
        result.insert(
            prefix.clone(),
            if active { "gamepad" } else { "none" }.into(),
        );
        for control in CONTROLS.iter().chain(RAPID) {
            let value = if active {
                let logical = logical.expect("active native slot has a logical slot");
                standard[&format!("snes.input.port{logical}.gamepad.{control}")].clone()
            } else {
                String::new()
            };
            result.insert(format!("{prefix}.gamepad.{control}"), value);
        }
    }
    result.insert(
        "snes_faust.input.sport1.multitap".into(),
        if tap1 { "1" } else { "0" }.into(),
    );
    result.insert(
        "snes_faust.input.sport2.multitap".into(),
        if tap2 { "1" } else { "0" }.into(),
    );
    Ok(result)
}
