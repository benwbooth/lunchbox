//! Native SDL player allocation and actual startup-record parsing.
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Opened {
    pub instance: u32,
    pub player: u8,
    pub is_gamepad: bool,
    pub name: String,
}

/// Input order and reported indices must be captured after SDL opens devices.
/// Inventory hints alone are not evidence of native opening order or indices.
pub(crate) fn project(open_order: &[(u32, i32)]) -> Result<BTreeMap<u32, u8>> {
    ensure!(
        !open_order.is_empty() && open_order.len() <= 256,
        "Invalid PCSX2 SDL opening sequence"
    );
    let mut assignments = BTreeMap::new();
    let mut occupied = BTreeSet::new();
    for &(instance, reported) in open_order {
        ensure!(
            instance != 0 && !assignments.contains_key(&instance),
            "PCSX2 SDL instance was opened twice or is invalid"
        );
        let player = if reported < 0 || occupied.contains(&reported) {
            (0..=255)
                .find(|id| !occupied.contains(id))
                .context("PCSX2 SDL player space exhausted")?
        } else {
            reported
        };
        let encoded =
            u8::try_from(player).context("PCSX2 SDL player ID exceeds native binding field")?;
        occupied.insert(player);
        assignments.insert(instance, encoded);
    }
    Ok(assignments)
}

pub(crate) fn startup_records(log: &str) -> Result<BTreeMap<u32, Opened>> {
    ensure!(
        log.len() <= 4 * 1024 * 1024,
        "PCSX2 startup log exceeds limit"
    );
    let mut records = BTreeMap::new();
    let mut players = BTreeSet::new();
    for line in log
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
    {
        let Some((_, tail)) = line.split_once("SDLInputSource: Opened ") else {
            continue;
        };
        let (is_gamepad, tail) = if let Some(tail) = tail.strip_prefix("gamepad ") {
            (true, tail)
        } else if let Some(tail) = tail.strip_prefix("joystick ") {
            (false, tail)
        } else {
            anyhow::bail!("Unknown PCSX2 opened-device record");
        };
        let (index, tail) = tail
            .split_once(" (instance id ")
            .context("Malformed PCSX2 device record")?;
        let (instance, tail) = tail
            .split_once(", player id ")
            .context("Malformed PCSX2 instance record")?;
        let (player, name) = tail
            .split_once("): ")
            .context("Malformed PCSX2 player record")?;
        let instance: u32 = instance.parse()?;
        let player: u8 = player.parse()?;
        let name = name.trim_end_matches(['\r', '\n']);
        ensure!(
            index.parse::<u32>()? == instance
                && instance != 0
                && !name.is_empty()
                && !name.chars().any(char::is_control),
            "Invalid PCSX2 opened-device identity"
        );
        ensure!(
            records
                .insert(
                    instance,
                    Opened {
                        instance,
                        player,
                        is_gamepad,
                        name: name.to_owned()
                    }
                )
                .is_none()
                && players.insert(player),
            "PCSX2 opened duplicate device or player during handoff"
        );
        ensure!(
            records.len() <= 256,
            "PCSX2 startup device count exceeds limit"
        );
    }
    Ok(records)
}

/// Native disc-detail log emitted before UpdateGameSettingsLayer. Do not use
/// the separately reported current ELF CRC as the disc-settings identity.
pub(crate) fn disc_identity(log: &str) -> Result<Option<(String, u32)>> {
    ensure!(
        log.len() <= 4 * 1024 * 1024,
        "PCSX2 identity log exceeds limit"
    );
    let mut serial = None;
    let mut identity = None;
    for line in log
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
    {
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some((_, value)) = line.split_once("  Serial: ") {
            serial = Some(value.to_owned());
        } else if let Some((_, value)) = line.split_once("  CRC: ") {
            ensure!(
                value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "Invalid PCSX2 disc CRC record"
            );
            let serial = serial
                .take()
                .context("PCSX2 disc CRC has no matching serial record")?;
            let current = (serial, u32::from_str_radix(value, 16)?);
            ensure!(
                identity
                    .as_ref()
                    .is_none_or(|previous| previous == &current),
                "PCSX2 disc identity changed during startup"
            );
            identity = Some(current);
        }
    }
    Ok(identity)
}
