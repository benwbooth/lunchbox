//! Digital SCPH-1080 pad, pinned psx/input/gamepad.cpp and frontio.cpp.
//! Multitap slots are sequential across physical ports, not player-two-first.
use super::{Input, Polarity, binding};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTROLS: &[&str] = &[
    "up", "down", "left", "right", "select", "start", "l1", "l2", "r1", "r2", "triangle", "circle",
    "cross", "square",
];

pub(crate) struct Player<'a> {
    pub port: u8,
    pub native_id: &'a str,
    pub controls: &'a BTreeMap<String, Input>,
}

pub(crate) const STICK_PAIRS: &[(&str, &str)] = &[
    ("lstick_left", "lstick_right"),
    ("lstick_up", "lstick_down"),
    ("rstick_left", "rstick_right"),
    ("rstick_up", "rstick_down"),
];

pub(crate) const ANALOG_CONTROLS: &[&str] = &[
    "up",
    "down",
    "left",
    "right",
    "select",
    "start",
    "l1",
    "l2",
    "r1",
    "r2",
    "triangle",
    "circle",
    "cross",
    "square",
    "l3",
    "r3",
    "lstick_left",
    "lstick_right",
    "lstick_up",
    "lstick_down",
    "rstick_left",
    "rstick_right",
    "rstick_up",
    "rstick_down",
];

pub(crate) fn mixed_assignments(
    multitaps: [bool; 2],
    players: &[(bool, Player<'_>)],
) -> Result<BTreeMap<String, String>> {
    ensure!(!players.is_empty(), "PlayStation setup has no players");
    let mut ports = BTreeSet::new();
    let mut devices = BTreeSet::new();
    let mut result = BTreeMap::new();
    for (analog, player) in players {
        ensure!(
            ports.insert(player.port)
                && devices.insert(binding(player.native_id, Input::Button(0), 4096)?),
            "PlayStation players share a port or joystick"
        );
        let one = std::slice::from_ref(player);
        let generated = if *analog {
            dual_analog_assignments(multitaps, one)?
        } else {
            assignments(multitaps, one)?
        };
        let prefix = format!("psx.input.port{}", player.port);
        for (key, value) in generated {
            if key == prefix
                || key.starts_with(&format!("{prefix}."))
                || key.starts_with("psx.input.pport")
            {
                result.insert(key, value);
            }
        }
    }
    for port in 1..=8 {
        if !ports.contains(&port) {
            result.insert(format!("psx.input.port{port}"), "none".into());
        }
    }
    Ok(result)
}

/// Native SCPH-1180, forced analog mode. This does not emulate DualShock rumble
/// or its analog-mode switch. Axis halves retain proportional native magnitudes.
pub(crate) fn dual_analog_assignments(
    multitaps: [bool; 2],
    players: &[Player<'_>],
) -> Result<BTreeMap<String, String>> {
    let digital_maps: Vec<BTreeMap<String, Input>> = players
        .iter()
        .map(|player| {
            player
                .controls
                .iter()
                .filter(|(key, _)| CONTROLS.contains(&key.as_str()))
                .map(|(key, input)| (key.clone(), *input))
                .collect()
        })
        .collect();
    let digital_players: Vec<_> = players
        .iter()
        .zip(&digital_maps)
        .map(|(player, controls)| Player {
            port: player.port,
            native_id: player.native_id,
            controls,
        })
        .collect();
    let mut result: BTreeMap<_, _> = assignments(multitaps, &digital_players)?
        .into_iter()
        .map(|(key, value)| {
            (
                key.replace(".gamepad.", ".dualanalog."),
                if value == "gamepad" {
                    "dualanalog".into()
                } else {
                    value
                },
            )
        })
        .collect();
    for (player, digital) in players.iter().zip(&digital_maps) {
        ensure!(
            player.controls.len() == CONTROLS.len() + 10,
            "Dual Analog requires sixteen buttons and eight stick directions"
        );
        let prefix = format!("psx.input.port{}.dualanalog", player.port);
        let mut owners: BTreeSet<_> = digital.values().copied().collect();
        for key in ["l3", "r3"] {
            let input = *player
                .controls
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("Missing Dual Analog stick click"))?;
            ensure!(
                !matches!(
                    input,
                    Input::Absolute {
                        polarity: Polarity::Full | Polarity::FullReversed,
                        ..
                    }
                ) && owners.insert(input),
                "Invalid or shared Dual Analog stick click"
            );
            result.insert(
                format!("{prefix}.{key}"),
                binding(player.native_id, input, 4096)?,
            );
        }
        let mut axes = BTreeSet::new();
        for (negative, positive) in STICK_PAIRS {
            let a = *player
                .controls
                .get(*negative)
                .ok_or_else(|| anyhow::anyhow!("Missing Dual Analog direction"))?;
            let b = *player
                .controls
                .get(*positive)
                .ok_or_else(|| anyhow::anyhow!("Missing Dual Analog direction"))?;
            let (
                Input::Absolute {
                    index: ai,
                    polarity: ap,
                },
                Input::Absolute {
                    index: bi,
                    polarity: bp,
                },
            ) = (a, b)
            else {
                anyhow::bail!("Dual Analog sticks require proportional physical axes");
            };
            ensure!(
                ai == bi
                    && matches!(
                        (ap, bp),
                        (Polarity::Negative, Polarity::Positive)
                            | (Polarity::Positive, Polarity::Negative)
                    )
                    && axes.insert(ai),
                "Dual Analog directions must be opposite halves of four distinct axes"
            );
            ensure!(
                owners.insert(a) && owners.insert(b),
                "Dual Analog sticks overlap button inputs"
            );
            result.insert(
                format!("{prefix}.{negative}"),
                binding(player.native_id, a, 4096)?,
            );
            result.insert(
                format!("{prefix}.{positive}"),
                binding(player.native_id, b, 4096)?,
            );
        }
        result.insert(format!("{prefix}.axis_scale"), "1.00".into());
    }
    Ok(result)
}

pub(crate) fn capacity(multitaps: [bool; 2]) -> usize {
    multitaps
        .iter()
        .map(|enabled| if *enabled { 4 } else { 1 })
        .sum()
}

pub(crate) fn assignments(
    multitaps: [bool; 2],
    players: &[Player<'_>],
) -> Result<BTreeMap<String, String>> {
    let max = capacity(multitaps);
    ensure!(
        !players.is_empty() && players.len() <= max,
        "Invalid PlayStation player count"
    );
    let mut result = BTreeMap::new();
    let mut ports = BTreeSet::new();
    let mut devices = BTreeSet::new();
    for player in players {
        ensure!(
            (1..=max).contains(&usize::from(player.port)) && ports.insert(player.port),
            "Invalid or duplicate PlayStation virtual port"
        );
        ensure!(
            devices.insert(binding(player.native_id, Input::Button(0), 4096)?),
            "PlayStation players share a joystick"
        );
        ensure!(
            player.controls.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|key| player.controls.contains_key(*key)),
            "PlayStation digital pad requires all fourteen controls"
        );
        let prefix = format!("psx.input.port{}", player.port);
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
                "PlayStation digital controls require signed half-axes"
            );
            ensure!(
                owners.insert(*input),
                "PlayStation controls have competing owners"
            );
            result.insert(
                format!("{prefix}.gamepad.{key}"),
                binding(player.native_id, *input, 4096)?,
            );
        }
        // Only the four face buttons are ButtonCR in native IDII.
        for key in ["triangle", "circle", "cross", "square"] {
            result.insert(format!("{prefix}.gamepad.rapid_{key}"), String::new());
        }
    }
    for port in 1..=8 {
        if !ports.contains(&port) {
            result.insert(format!("psx.input.port{port}"), "none".into());
        }
    }
    for (index, enabled) in multitaps.into_iter().enumerate() {
        result.insert(
            format!("psx.input.pport{}.multitap", index + 1),
            if enabled { "1" } else { "0" }.into(),
        );
    }
    // Memory cards are independent native devices: do not overwrite their settings.
    Ok(result)
}
