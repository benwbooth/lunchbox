//! Arcade geometry is independent of the fixed libretro/native button wires.
use crate::controller_catalog::EmulatorProfile;
use anyhow::{Context, Result, ensure};

const BUTTON_OUTPUTS: [&str; 10] = [
    "South",
    "East",
    "West",
    "North",
    "LeftBumper",
    "RightBumper",
    "LeftStick",
    "RightStick",
    "LeftTrigger",
    "RightTrigger",
];

/// Ordinary button capacity is independent of the highest occupied channel.
/// Callers retain separate twin-stick handling before using this selection.
pub(super) fn automatic_button_layout<'a>(
    outputs: impl IntoIterator<Item = &'a str>,
) -> DigitalLayout {
    let needed: std::collections::BTreeSet<_> = outputs.into_iter().collect();
    let buttons = BUTTON_OUTPUTS
        .iter()
        .filter(|channel| needed.contains(**channel))
        .count();
    // Trigger threshold switches are ordinary action channels here. Analog
    // pressure owners are excluded by the composer before selecting geometry.
    if super::explicit_switch_channels()
        .skip(16)
        .any(|(_, channel)| needed.contains(channel))
        || buttons > 8
    {
        DigitalLayout::FixedChannels
    } else if buttons > 6 {
        DigitalLayout::EightButton
    } else {
        DigitalLayout::SixButton
    }
}

/// Preserve conventional positions first, then pack sparse higher channels into
/// unused positions. These are diagram identities, never native wire changes.
pub(super) fn button_positions<'a>(
    target: &str,
    outputs: impl IntoIterator<Item = &'a str>,
) -> Result<std::collections::BTreeMap<String, String>> {
    let slots: Vec<String> = match target {
        "arcade-six-button" => (1..=6).map(|n| format!("button{n}")).collect(),
        "arcade-eight-button" => (1..=8).map(|n| format!("button{n}")).collect(),
        "neogeo" => ["a", "b", "c", "d"].map(str::to_owned).to_vec(),
        _ => return Ok(Default::default()),
    };
    let needed: std::collections::BTreeSet<_> = outputs.into_iter().collect();
    let active: Vec<_> = BUTTON_OUTPUTS
        .iter()
        .enumerate()
        .filter(|(_, channel)| needed.contains(**channel))
        .collect();
    ensure!(
        active.len() <= slots.len(),
        "MAME {target} has {} button positions but needs {} independent button channels; choose a larger preset",
        slots.len(),
        active.len()
    );
    let mut positions = std::collections::BTreeMap::new();
    for (index, channel) in &active {
        if let Some(slot) = slots.get(*index) {
            positions.insert((**channel).to_owned(), slot.clone());
        }
    }
    let capacity = slots.len();
    let free: Vec<_> = slots
        .into_iter()
        .filter(|slot| !positions.values().any(|used| used == slot))
        .collect();
    let overflow = active.into_iter().filter(|(index, _)| *index >= capacity);
    for ((_, channel), slot) in overflow.zip(free) {
        positions.insert((*channel).to_owned(), slot);
    }
    Ok(positions)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DigitalLayout {
    /// Preserve the assignments of setups saved before arcade presets existed.
    #[default]
    FixedChannels,
    Automatic,
    SixButton,
    EightButton,
    NeoGeo,
}

impl DigitalLayout {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::FixedChannels => "Legacy fixed-channel gamepad",
            Self::Automatic => "Automatic arcade / extended fixed channels",
            Self::SixButton => "Six-button arcade: 1 2 3 / 4 5 6",
            Self::EightButton => "Eight-button arcade: 1 2 3 7 / 4 5 6 8",
            Self::NeoGeo => "Neo Geo: A B C D",
        }
    }
}

/// Only the game's observed active controls are required. A one-button game
/// does not require six calibrated buttons merely because its preset has six.
/// Native buttons 7/8 still emit frontend L3/R3, never the L2/R2 axes.
pub(crate) fn digital_profile(
    snapshot: &super::ActiveFieldSnapshot,
    player: usize,
    layout: DigitalLayout,
) -> Result<EmulatorProfile> {
    optional_digital_profile(snapshot, player, layout)?
        .context("Selected MAME player has no active digital controls")
}

/// Discover a port without turning invalid snapshots or incompatible presets
/// into absent players. Unsupported native fields remain the field planner's
/// responsibility; this only distinguishes an empty digital binding subset.
pub(crate) fn optional_digital_profile(
    snapshot: &super::ActiveFieldSnapshot,
    player: usize,
    layout: DigitalLayout,
) -> Result<Option<EmulatorProfile>> {
    optional_profile_with_pressure(snapshot, player, layout, [false; 2])
}

/// The analog composer validates and owns these pressure requirements. Remove
/// only their duplicate digital representations, not native fields or routes.
pub(super) fn optional_profile_with_pressure(
    snapshot: &super::ActiveFieldSnapshot,
    player: usize,
    layout: DigitalLayout,
    pressure: [bool; 2],
) -> Result<Option<EmulatorProfile>> {
    let Some(mut profile) = super::optional_digital_calibration_profile(snapshot, player)? else {
        return Ok(None);
    };
    profile.bindings.retain(|_, output| match output.as_str() {
        "LeftTrigger" => !pressure[0],
        "RightTrigger" => !pressure[1],
        _ => true,
    });
    if profile.bindings.is_empty() {
        return Ok(None);
    }
    if profile.target_layout == "mame-twin-digital" {
        ensure!(
            matches!(
                layout,
                DigitalLayout::Automatic | DigitalLayout::FixedChannels
            ),
            "MAME twin-stick fields require Automatic layout; a button-row preset cannot describe two direction clusters"
        );
        profile.name = format!(
            "MAME {} player {player} — twin digital sticks",
            snapshot.machine
        );
        return Ok(Some(profile));
    }
    let sparse_high_actions = (11..=16).any(|button| {
        snapshot
            .fields
            .iter()
            .any(|field| field.input_type == format!("P{player}_BUTTON{button}"))
    });
    if sparse_high_actions {
        profile.conditions.push("Sparse actions above Button 10 use unoccupied frontend channels. Arcade positions describe frontend channels, not native action numbers; the mapping review shows the exact game action names and field identities.".to_owned());
    }
    let layout = match layout {
        DigitalLayout::Automatic => {
            automatic_button_layout(profile.bindings.values().map(String::as_str))
        }
        other => other,
    };
    if layout == DigitalLayout::FixedChannels {
        return Ok(Some(profile));
    }
    let (target, buttons): (&str, &[&str]) = match layout {
        DigitalLayout::SixButton => (
            "arcade-six-button",
            &[
                "button1", "button2", "button3", "button4", "button5", "button6",
            ],
        ),
        DigitalLayout::EightButton => (
            "arcade-eight-button",
            &[
                "button1", "button2", "button3", "button4", "button5", "button6", "button7",
                "button8",
            ],
        ),
        DigitalLayout::NeoGeo => ("neogeo", &["a", "b", "c", "d"]),
        DigitalLayout::Automatic | DigitalLayout::FixedChannels => unreachable!(),
    };
    let fixed_ids = [
        "a",
        "b",
        "x",
        "y",
        "l",
        "r",
        "l3",
        "r3",
        "trigger_left_switch",
        "trigger_right_switch",
    ];
    let mut bindings = std::collections::BTreeMap::new();
    let positions = button_positions(target, profile.bindings.values().map(String::as_str))?;
    for (control, output) in profile.bindings {
        let target_control = if let Some(index) = fixed_ids.iter().position(|id| *id == control) {
            if let Some(position) = positions.get(&output) {
                position.clone()
            } else {
                ensure!(
                    index < buttons.len(),
                    "{} cannot cover required frontend action position {}; choose Automatic or a larger preset",
                    layout.label(),
                    index + 1
                );
                buttons[index].to_owned()
            }
        } else {
            control
        };
        bindings.insert(target_control, output);
    }
    profile.id = format!("{}-{target}", profile.id);
    profile.name = format!(
        "MAME {} player {player} — {}",
        snapshot.machine,
        layout.label()
    );
    profile.target_layout = target.to_owned();
    profile.bindings = bindings;
    profile.conditions.push("Preset positions do not infer game-specific action labels or support for analog/peripheral controls.".to_owned());
    profile.conditions.push("Sparse higher button channels may occupy unused preset positions. Native outputs stay unchanged; review the exact game actions beside the diagram.".to_owned());
    if profile
        .bindings
        .values()
        .any(|output| matches!(output.as_str(), "LeftTrigger" | "RightTrigger"))
    {
        profile.conditions.push("A button position using L2/R2 still drives a native negative-axis switch threshold, not a numbered frontend button. A digital source uses full-travel fallback; shared analog pressure uses the same travel.".to_owned());
    }
    Ok(Some(profile))
}
