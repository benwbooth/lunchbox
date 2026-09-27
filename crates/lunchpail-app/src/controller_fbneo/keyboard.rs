//! FBNeo keyboard targets through RetroArch's internal gamepad key mapper.
//! This renders remap-file content, not a normal append-config fragment.
use super::{FRONTEND_PORTS, InputAddress, MappingTarget};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// Pinned configuration.c input_remapping_save_file order. Analog directions
/// are thresholded switches here, not proportional keyboard values.
pub(crate) const CHANNELS: [&str; 24] = [
    "b", "y", "select", "start", "up", "down", "left", "right", "a", "x", "l", "r", "l2", "r2",
    "l3", "r3", "l_x+", "l_x-", "l_y+", "l_y-", "r_x+", "r_x-", "r_y+", "r_y-",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeyboardBinding {
    pub target: InputAddress,
    pub channel: u8,
}

pub(super) fn valid_key(id: u32) -> bool {
    // Exact non-UNKNOWN key ranges in the pinned libretro.h enum; holes are
    // rejected, not interpreted as Unicode, evdev or platform key codes.
    matches!(id, 8 | 9 | 12 | 13 | 19 | 27 | 32..=36 | 38..=64 | 91..=127 | 256..=296 | 300..=341)
}

/// Actual non-Android rarch_key_map_linux entries at the pinned frontend
/// revision. Valid RETROK punctuation is not necessarily a physical keysym.
pub(crate) fn udev_passthrough_key(id: u32) -> bool {
    valid_key(id)
        && !matches!(
            id,
            33 | 34 | 35 | 38 | 58 | 60 | 62 | 64 | 94 | 95 | 123..=126 | 309 | 310
        )
}

/// Render one explicitly selected keyboard port. The pinned frontend key state
/// is shared, so independent multi-keyboard ownership is not promised here.
/// Caller must establish isolated remap loading, keyboard-hotkey isolation,
/// native port selection and calibrated physical channel bindings before launch.
pub(crate) fn render(
    targets: &[MappingTarget],
    port: u32,
    bindings: &[KeyboardBinding],
    selected_devices: &BTreeMap<u32, u32>,
) -> Result<String> {
    ensure!(
        port < FRONTEND_PORTS as u32,
        "FBNeo keyboard port exceeds frontend limits"
    );
    ensure!(
        selected_devices.get(&port) == Some(&3)
            && selected_devices
                .keys()
                .all(|port| *port < FRONTEND_PORTS as u32),
        "Keyboard remap requires the explicitly selected native keyboard port"
    );
    ensure!(
        selected_devices
            .iter()
            .all(|(owner, device)| *device != 3 || *owner == port),
        "Independent simultaneous keyboard ports need a separate ownership contract"
    );
    ensure!(
        !bindings.is_empty() && bindings.len() <= CHANNELS.len(),
        "Keyboard mapping requires 1–24 independently assigned channels"
    );
    let required: BTreeSet<_> = targets
        .iter()
        .filter(|target| target.address.port == port && target.address.device == 3)
        .map(|target| target.address.clone())
        .collect();
    ensure!(
        !required.is_empty(),
        "Selected FBNeo port has no inspected keyboard targets"
    );
    ensure!(
        required
            .iter()
            .all(|target| target.index == 0 && valid_key(target.id)),
        "Inspected keyboard target has an unsupported key identity"
    );
    let mut assigned = BTreeSet::new();
    let mut channels = BTreeMap::new();
    for binding in bindings {
        ensure!(
            required.contains(&binding.target) && assigned.insert(binding.target.clone()),
            "Keyboard assignment is absent, wrong-port or duplicated"
        );
        ensure!(
            (binding.channel as usize) < CHANNELS.len(),
            "Keyboard source channel exceeds mapper capacity"
        );
        ensure!(
            channels
                .insert(binding.channel as usize, binding.target.id)
                .is_none(),
            "Independent keyboard targets cannot share one source channel"
        );
    }
    ensure!(
        assigned == required,
        "Keyboard mapping has unresolved inspected keys; use a larger input contract rather than dropping keys"
    );
    let mut result = String::new();
    // Fully clear key maps for every FBNeo port to avoid inherited key owners.
    // Device choices must come from the same freshly validated native topology.
    for player in 0..FRONTEND_PORTS {
        for (channel, suffix) in CHANNELS.iter().enumerate() {
            let key = if player as u32 == port {
                channels.get(&channel).copied().unwrap_or(0)
            } else {
                0
            };
            writeln!(
                result,
                "input_player{}_key_{} = \"{}\"",
                player + 1,
                suffix,
                key
            )?;
            // Key maps must not inherit a gamepad remap or analog-D-pad layer.
            let kind = if channel < 16 { "btn" } else { "stk" };
            writeln!(
                result,
                "input_player{}_{}_{} = \"{}\"",
                player + 1,
                kind,
                suffix,
                channel
            )?;
        }
        let device = selected_devices.get(&(player as u32)).copied().unwrap_or(0);
        writeln!(
            result,
            "input_libretro_device_p{} = \"{}\"",
            player + 1,
            device
        )?;
        writeln!(result, "input_remap_port_p{} = \"{}\"", player + 1, player)?;
        writeln!(
            result,
            "input_player{}_analog_dpad_mode = \"0\"",
            player + 1
        )?;
    }
    result.push_str("input_turbo_enable = \"false\"\n");
    Ok(result)
}

/// Core-level fallback under a private remap directory, with controller-name
/// sorting disabled by the caller. Never derive this from a ROM display title.
pub(crate) fn relative_remap_path(library_name: &str) -> Result<std::path::PathBuf> {
    ensure!(
        !library_name.is_empty()
            && library_name.len() <= 255
            && !matches!(library_name, "." | "..")
            && !library_name
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')),
        "Core library name cannot identify a private remap path"
    );
    Ok(std::path::Path::new(library_name).join(format!("{library_name}.rmp")))
}

/// Translate explicit key assignments to frontend channels for the existing
/// physical calibration planner. These targets are internal channel requirements,
/// never replacements for the retained native descriptor/query evidence.
pub(crate) fn physical_plan(
    targets: &[MappingTarget],
    port: u32,
    keys: &[KeyboardBinding],
    assignments: &[super::SourceBinding],
    selected_devices: &BTreeMap<u32, u32>,
) -> Result<(Vec<MappingTarget>, Vec<super::SourceBinding>, String)> {
    use super::{BindingPart, InputEncoding, SourceBinding};
    let remap = render(targets, port, keys, selected_devices)?;
    ensure!(
        assignments.len() <= 8192,
        "Keyboard physical assignment limit exceeded"
    );
    let channels: BTreeMap<_, _> = keys.iter().map(|key| (&key.target, key.channel)).collect();
    for key in keys {
        ensure!(
            assignments
                .iter()
                .any(|assignment| assignment.target == key.target
                    && assignment.part == super::BindingPart::Digital),
            "Keyboard key has no explicit physical assignment"
        );
    }
    let translate = |channel: u8| -> (InputAddress, BindingPart) {
        if channel < 16 {
            (
                InputAddress {
                    port,
                    device: 1,
                    index: 0,
                    id: channel as u32,
                },
                BindingPart::Digital,
            )
        } else {
            let axis = u32::from(channel - 16);
            (
                InputAddress {
                    port,
                    device: 5,
                    index: axis / 4,
                    id: (axis % 4) / 2,
                },
                if axis % 2 == 0 {
                    BindingPart::Positive
                } else {
                    BindingPart::Negative
                },
            )
        }
    };
    let mut planned_targets: BTreeMap<_, _> = targets
        .iter()
        .filter(|target| target.address.port == port && target.address.device != 3)
        .map(|target| (target.address.clone(), target.clone()))
        .collect();
    for key in keys {
        let (address, _) = translate(key.channel);
        planned_targets
            .entry(address.clone())
            .or_insert_with(|| MappingTarget {
                id: format!(
                    "frontend_keyboard_channel_p{}_d{}_i{}_id{}",
                    port, address.device, address.index, address.id
                ),
                encoding: if address.device == 1 {
                    InputEncoding::JoypadButton
                } else {
                    InputEncoding::SignedAxis
                },
                address,
                descriptions: vec![
                    "Internal frontend channel for an explicitly mapped keyboard key".to_owned(),
                ],
                label_source: Some(key.target.clone()),
                queried_during_idle_frame: false,
            });
    }
    let mut originals = BTreeSet::new();
    let mut planned = BTreeMap::<_, SourceBinding>::new();
    for assignment in assignments {
        ensure!(
            assignment.target.port == port
                && targets
                    .iter()
                    .any(|target| target.address == assignment.target),
            "Physical assignment is absent from the inspected keyboard port"
        );
        ensure!(
            originals.insert((assignment.target.clone(), assignment.part)),
            "Duplicate physical keyboard assignment"
        );
        let (target, part) = if assignment.target.device == 3 {
            ensure!(
                assignment.part == BindingPart::Digital,
                "Keyboard key assignments are switches"
            );
            let channel = channels.get(&assignment.target).ok_or_else(|| {
                anyhow::anyhow!("Keyboard assignment has no explicit frontend channel")
            })?;
            translate(*channel)
        } else {
            (assignment.target.clone(), assignment.part)
        };
        let translated = SourceBinding {
            target: target.clone(),
            part,
            source: assignment.source.clone(),
        };
        if let Some(previous) = planned.insert((target, part), translated.clone()) {
            ensure!(
                previous.source == translated.source,
                "Keyboard and ordinary input share a frontend channel but use different physical gestures"
            );
        }
    }
    Ok((
        planned_targets.into_values().collect(),
        planned.into_values().collect(),
        remap,
    ))
}
