//! Guided standard-pad targets for the existing native BizHawk decks.
//! BizHawk 8c6b8958bbbe623eaaa36bc82af858b812893628. Runtime discovery and
//! SDL translation remain the responsibilities of the native session owners.
use super::{NativeLaunchSettings, NativePlayerSettings};
use crate::controller_catalog::{Calibration, Catalog, EmulatorProfile, Layout};
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn add_profiles(db: &mut Catalog) -> Result<()> {
    for (layout, limit, platforms) in [
        ("playstation-digital", 8, vec!["Sony Playstation"]),
        ("snes", 8, vec!["Super Nintendo Entertainment System"]),
        ("nes", 4, vec!["Nintendo Entertainment System"]),
        ("master-system", 2, vec!["Sega Master System"]),
        ("gamegear", 1, vec!["Sega Game Gear"]),
        ("sg1000", 2, vec!["Sega SG-1000"]),
        (
            "pce-2",
            5,
            vec![
                "NEC TurboGrafx-16",
                "NEC TurboGrafx-CD",
                "NEC PC Engine SuperGrafx",
                "PC Engine",
            ],
        ),
        (
            "pce-turbonyma",
            5,
            vec![
                "NEC TurboGrafx-16",
                "NEC TurboGrafx-CD",
                "NEC PC Engine SuperGrafx",
                "PC Engine",
            ],
        ),
        (
            "genesis-3",
            8,
            vec!["Sega Genesis", "Sega Mega Drive", "Sega CD"],
        ),
        (
            "genesis-6",
            8,
            vec!["Sega Genesis", "Sega Mega Drive", "Sega CD"],
        ),
    ] {
        crate::controller_native_targets::add(
            db,
            "bizhawk",
            layout,
            limit,
            &platforms,
            "https://github.com/TASEmulators/BizHawk/tree/8c6b8958bbbe623eaaa36bc82af858b812893628/src/BizHawk.Emulation.Cores",
        )?;
    }
    Ok(())
}

pub(crate) fn routes(layout: &str) -> Option<BTreeMap<String, String>> {
    let names: &[&str] = match layout {
        "snes" => &super::snes9x::BUTTONS,
        "nes" => &super::neshawk::BUTTONS,
        "genesis-3" => super::gpgx::Pad::ThreeButton.buttons(),
        "genesis-6" => super::gpgx::Pad::SixButton.buttons(),
        "master-system" | "sg1000" => &["Up", "Down", "Left", "Right", "B", "A"],
        "gamegear" => &["Up", "Down", "Left", "Right", "B", "A", "Start"],
        "pce-2" => &["Up", "Down", "Left", "Right", "Select", "Start", "B", "A"],
        "pce-turbonyma" => &[
            "Up", "Down", "Left", "Right", "Select", "Start", "A", "B", "C", "X", "Y", "Z",
            "Mode2", "Mode6",
        ],
        "playstation-digital" => {
            return Some(
                [
                    ("b", "X"),
                    ("a", "○"),
                    ("y", "□"),
                    ("x", "△"),
                    ("select", "Select"),
                    ("start", "Start"),
                    ("l", "L1"),
                    ("r", "R1"),
                    ("l2", "L2"),
                    ("r2", "R2"),
                    ("up", "Up"),
                    ("down", "Down"),
                    ("left", "Left"),
                    ("right", "Right"),
                ]
                .into_iter()
                .map(|(id, name)| (id.into(), name.into()))
                .collect(),
            );
        }
        _ => return None,
    };
    Some(
        names
            .iter()
            .map(|name| {
                let native = match (layout, *name) {
                    ("master-system" | "sg1000" | "gamegear", "B") | ("pce-2", "A") => "B1",
                    ("master-system" | "sg1000" | "gamegear", "A") | ("pce-2", "B") => "B2",
                    ("pce-2" | "pce-turbonyma", "Start") => "Run",
                    ("pce-turbonyma", "A") => "III",
                    ("pce-turbonyma", "B") => "II",
                    ("pce-turbonyma", "C") => "I",
                    ("pce-turbonyma", "X") => "IV",
                    ("pce-turbonyma", "Y") => "V",
                    ("pce-turbonyma", "Z") => "VI",
                    ("pce-turbonyma", "Mode2") => "Mode: Set 2-button",
                    ("pce-turbonyma", "Mode6") => "Mode: Set 6-button",
                    _ => *name,
                };
                (name.to_ascii_lowercase(), native.to_string())
            })
            .collect(),
    )
}

pub(crate) fn profile_id(layout: &str) -> String {
    format!("bizhawk:standalone-{layout}")
}

/// All native/normalized paths read the same per-profile manual assignments.
/// A saved choice unavailable in the current SDL context is an error, not an
/// invitation to silently remap that button for the user.
pub(crate) fn resolve(
    calibration: &Calibration,
    source: &Layout,
    target: &Layout,
    available: &BTreeSet<&str>,
    requested: &BTreeSet<&str>,
) -> Result<crate::controller_layout::Resolution> {
    let empty = BTreeMap::new();
    let choices = calibration
        .target_mappings
        .get(&profile_id(&target.id))
        .unwrap_or(&empty);
    crate::controller_layout::resolve_with_choices(source, target, available, requested, choices)
}

pub(crate) fn scope(layout: &str) -> Option<&'static str> {
    Some(match layout {
        "playstation-digital" => "nymashock",
        "snes" => "snes9x",
        "nes" => "neshawk",
        "master-system" => "master-system",
        "gamegear" => "gamegear",
        "sg1000" => "sg1000",
        "pce-2" => "pcehawk",
        "pce-turbonyma" => "turbonyma",
        "genesis-3" | "genesis-6" => "gpgx",
        _ => return None,
    })
}

pub(crate) fn apply(
    setup: &mut NativeLaunchSettings,
    profile: &EmulatorProfile,
    ids: &[String],
) -> Result<()> {
    ensure!(
        scope(&profile.target_layout) == Some(setup.scope_id()),
        "The saved BizHawk runtime uses a different core"
    );
    let count = ids.len();
    ensure!(
        count > 0
            && count
                <= profile
                    .native_launch
                    .as_ref()
                    .context("Missing native capacity")?
                    .max_players,
        "Too many players for the selected BizHawk target"
    );
    let old = setup.players.clone();
    // Only change this selected core's topology, retaining runtime paths and
    // unrelated advanced setups. Partially filled taps have unbound slots.
    match profile.target_layout.as_str() {
        "snes" => {
            use super::snes9x::PadPort::{Joypad, Multitap, None as Unplugged};
            setup.snes9x_ports = Some(match count {
                1 => [Joypad, Unplugged],
                2 => [Joypad, Joypad],
                3..=4 => [Multitap, Unplugged],
                5 => [Multitap, Joypad],
                _ => [Multitap, Multitap],
            });
        }
        "nes" => {
            use super::neshawk::PadPort::{FourScore, Joypad, None as Unplugged};
            setup.neshawk_ports = Some(match count {
                1 => [Joypad, Unplugged],
                2 => [Joypad, Joypad],
                _ => [FourScore, FourScore],
            });
        }
        "pce-2" => setup.pcehawk_ports = Some(std::array::from_fn(|index| index < count)),
        "pce-turbonyma" => {
            setup.turbonyma_topology = Some(super::turbonyma::Topology {
                ports: std::array::from_fn(|index| index < count),
                multitap: count > 1,
            })
        }
        "genesis-3" | "genesis-6" => {
            setup.gpgx_topology = Some(super::gpgx::Topology {
                ports: [true, count == 2 || count > 4],
                pad: if profile.target_layout == "genesis-3" {
                    super::gpgx::Pad::ThreeButton
                } else {
                    super::gpgx::Pad::SixButton
                },
                team_players: [count > 2, count > 5],
                wayplay: false,
                activators: [false; 2],
            })
        }
        "playstation-digital" => setup.multitaps = [count > 2, count > 5],
        "master-system" | "sg1000" | "gamegear" => {}
        _ => anyhow::bail!("Unknown BizHawk guided target"),
    }
    if setup.digital_deck().is_some() {
        setup.multitaps = [false; 2];
    }
    setup.players = ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let previous = old.iter().find(|player| &player.controller_id == id);
            NativePlayerSettings {
                controller_id: id.clone(),
                virtual_port: index as u8,
                normalized_input: previous.is_some_and(|player| player.normalized_input),
                dualshock: false,
                dualanalog: false,
                analog_joystick: false,
                rhythm: None,
                negcon: false,
                pointer: None,
                desktop_cursor: false,
                mouse_speed_basis_points: 10000,
                analog_toggle_id: None,
                deadzone_basis_points: previous.map_or(1500, |player| player.deadzone_basis_points),
                rumble: false,
            }
        })
        .collect();
    setup.validate()
}
