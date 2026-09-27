//! Native [input] player assignments. IDs refer to the emulator process,
//! never implicitly to the separate inventory helper's SDL instance IDs.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

/// Startup constructor report from core/sdl/sdl_gamepad.cpp. The port printed
/// here precedes GamepadDevice::Register's saved-port override; it must not be
/// mistaken for proof of the final configured player assignment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpenedJoystick {
    pub instance: i32,
    pub initial_port: i32,
    pub name: String,
}

pub(crate) fn opened_joysticks(log: &str) -> Result<BTreeMap<i32, OpenedJoystick>> {
    ensure!(
        log.len() <= 4 * 1024 * 1024,
        "Flycast startup log exceeds limit"
    );
    let mut opened = BTreeMap::new();
    // Only complete lines count: callers may read while the child is writing.
    for line in log
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
    {
        let Some((_, record)) = line.split_once("SDL: Opened joystick ") else {
            continue;
        };
        let (id, rest) = record
            .split_once(" on port ")
            .ok_or_else(|| anyhow::anyhow!("Malformed Flycast joystick startup record"))?;
        let (port, rest) = rest
            .split_once(": '")
            .ok_or_else(|| anyhow::anyhow!("Malformed Flycast joystick startup port"))?;
        let (name, unique) = rest
            .rsplit_once("' unique_id=sdl_joystick_")
            .ok_or_else(|| anyhow::anyhow!("Malformed Flycast joystick startup identity"))?;
        let instance: i32 = id.parse()?;
        let initial_port: i32 = port.parse()?;
        let unique: i32 = unique.trim_end_matches(['\r', '\n']).parse()?;
        ensure!(
            instance >= 0
                && instance == unique
                && (-1..=3).contains(&initial_port)
                && !name.is_empty()
                && !name.chars().any(char::is_control),
            "Invalid Flycast joystick startup identity"
        );
        ensure!(
            opened
                .insert(
                    instance,
                    OpenedJoystick {
                        instance,
                        initial_port,
                        name: name.to_owned()
                    }
                )
                .is_none(),
            "Flycast reopened a joystick during startup handoff"
        );
        ensure!(
            opened.len() <= 256,
            "Flycast startup joystick count exceeds limit"
        );
    }
    Ok(opened)
}

pub(crate) struct PlayerPort {
    pub native_instance: i32,
    pub player: u8,
}

/// Generate assignments for every observed child joystick; disable unselected
/// ones so native enumeration defaults cannot also control a selected player.
pub(crate) fn assignments(
    observed_native_instances: &[i32],
    players: &[PlayerPort],
) -> Result<BTreeMap<String, i32>> {
    ensure!(
        !observed_native_instances.is_empty() && observed_native_instances.len() <= 256,
        "Invalid Flycast native joystick inventory"
    );
    let mut observed = BTreeSet::new();
    for &instance in observed_native_instances {
        ensure!(
            instance >= 0 && observed.insert(instance),
            "Invalid or duplicate Flycast native instance ID"
        );
    }
    ensure!(
        !players.is_empty() && players.len() <= 4,
        "Flycast requires one to four mapped players"
    );
    let mut selected = BTreeMap::new();
    let mut ports = BTreeSet::new();
    for player in players {
        ensure!(
            (1..=4).contains(&player.player) && ports.insert(player.player),
            "Invalid or duplicate Flycast player port"
        );
        ensure!(
            observed.contains(&player.native_instance)
                && selected
                    .insert(player.native_instance, i32::from(player.player) - 1)
                    .is_none(),
            "Flycast player refers to an absent or duplicate joystick"
        );
    }
    Ok(observed
        .into_iter()
        .map(|id| {
            (
                format!("maple_sdl_joystick_{id}"),
                selected.get(&id).copied().unwrap_or(-1),
            )
        })
        .collect())
}
