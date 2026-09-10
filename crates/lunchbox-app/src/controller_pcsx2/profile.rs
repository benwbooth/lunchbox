//! Standalone, complete controller-profile INI; do not append to user settings.
use super::{BindingKind, sdl::Input};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Player<'a> {
    /// Sequential player order: port-one slots, then port-two slots.
    pub player: u8,
    /// PCSX2's resolved SDL player identifier, independent of target pad slot.
    pub sdl_player: u8,
    pub controls: &'a BTreeMap<String, Input>,
}

pub(crate) fn native_slots(multitaps: [bool; 2]) -> Vec<u8> {
    let mut slots = vec![0];
    if multitaps[0] {
        slots.extend([2, 3, 4]);
    }
    slots.push(1);
    if multitaps[1] {
        slots.extend([5, 6, 7]);
    }
    slots
}

pub(crate) fn controller_profile(multitaps: [bool; 2], players: &[Player<'_>]) -> Result<String> {
    let slots = native_slots(multitaps);
    ensure!(
        !players.is_empty() && players.len() <= slots.len(),
        "PCSX2 player count exceeds selected ports"
    );
    let known = super::bindings();
    let required: BTreeSet<_> = super::visual_routes().into_values().collect();
    let mut sections = BTreeMap::new();
    let mut devices = BTreeSet::new();
    for player in players {
        ensure!(
            player.player > 0 && usize::from(player.player) <= slots.len(),
            "PCSX2 player is outside multitap capacity"
        );
        let slot = slots[usize::from(player.player - 1)];
        ensure!(
            !sections.contains_key(&slot) && devices.insert(player.sdl_player),
            "PCSX2 player ports and SDL devices must be distinct"
        );
        ensure!(
            required
                .iter()
                .all(|name| player.controls.contains_key(*name)),
            "PCSX2 DualShock2 needs every standard control mapping"
        );
        let mut inputs = BTreeSet::new();
        let mut lines = String::from(
            "Type = DualShock2\nInvertL = 0\nInvertR = 0\nDeadzone = 0.00\nAxisScale = 1.00\nButtonDeadzone = 0.00\nPressureModifier = 0.50\n",
        );
        for (name, &input) in player.controls {
            let kind = known
                .get(name.as_str())
                .ok_or_else(|| anyhow::anyhow!("Unknown PCSX2 DualShock2 binding: {name}"))?;
            ensure!(
                (*kind == BindingKind::Motor) == input.is_motor(),
                "PCSX2 motor and control bindings cannot be interchanged"
            );
            if !input.is_motor() {
                ensure!(
                    inputs.insert(input),
                    "PCSX2 physical input is assigned to multiple controls"
                );
            }
            lines.push_str(&format!("{name} = {}\n", input.token(player.sdl_player)?));
        }
        sections.insert(slot, lines);
    }
    let mut ini = format!(
        "[InputSources]\nSDL = true\n\n[Pad]\nMultitapPort1 = {}\nMultitapPort2 = {}\n",
        multitaps[0], multitaps[1]
    );
    for slot in 0..8 {
        ini.push_str(&format!("\n[{}]\n", super::section(slot)?));
        ini.push_str(
            sections
                .get(&slot)
                .map(String::as_str)
                .unwrap_or("Type = None\n"),
        );
    }
    Ok(ini)
}
