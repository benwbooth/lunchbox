//! MAME libretro input contract at 4fc9a9312baaf34963847f884961ad9793fbbc1d.
//! Native sequence serialization is separate from per-game discovery and launch.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

mod analog;
mod arcade;
mod dependencies;
mod digital;
mod inspection;
mod metadata;
mod relative;
pub(crate) mod relative_buttons;
mod runtime;
mod settings;
mod state;
pub(crate) use analog::combined_profile;
pub(crate) use analog::normalized_calibration;
pub(crate) use analog::{AnalogAssignment, AnalogChannel, plan_mixed_fields};
pub(crate) use arcade::{DigitalLayout, optional_digital_profile};
pub(crate) use digital::{
    DigitalAssignment, combined_profile as explicit_profile, plan_explicit_fields,
    resolved_switch_routes,
};
pub(crate) use inspection::{
    ActiveField, ActiveFieldSnapshot, PersistentPaths, active_field_script,
    session_command_with_mouse,
};
pub(crate) use relative::{RelativeAssignment, RelativeSource, plan_relative_fields};
pub(crate) use runtime::validate_mouse_button_configuration_tree;
pub(crate) use runtime::{InspectionInput, InspectionRequest, inspect_runtime};
pub(crate) use settings::NativeLaunchSettings;
pub(crate) use state::persistent_inputs;

/// In-memory files for private staging. Unhandled fields remain visible to the
/// caller; generating this plan does not establish complete game-mode coverage.
pub(crate) struct DigitalFieldPlan {
    pub controller_xml: String,
    pub game_config_xml: String,
    pub mapped_fields: usize,
    pub disabled_fields: usize,
    pub unhandled_fields: Vec<ActiveField>,
    pub preserved_settings: Vec<ActiveField>,
    pub preserved_internal: Vec<ActiveField>,
    /// Maintenance inputs left on native bindings, not calibrated mappings.
    pub preserved_service: Vec<ActiveField>,
}

/// Use exact runtime-discovered identities to replace driver-specific default
/// sequences as well as saved overrides. Identity/provenance of the inspection
/// runtime and content is the launch owner's responsibility, not inferred here.
pub(crate) fn plan_digital_fields(
    snapshot: &ActiveFieldSnapshot,
    original_controller_config: Option<&str>,
    original_game_config: Option<&str>,
    players: &BTreeSet<usize>,
) -> Result<DigitalFieldPlan> {
    let plan = plan_digital_layer(
        snapshot,
        original_controller_config,
        original_game_config,
        players,
    )?;
    ensure!(
        plan.mapped_fields > 0,
        "The selected MAME ports have no discovered standard digital fields"
    );
    Ok(plan)
}

/// Shared native layer for digital-only and explicit analog plans. A zero
/// digital count is valid here; the public digital-only entry retains its
/// requirement, while analog planning must supply validated active fields.
fn plan_digital_layer(
    snapshot: &ActiveFieldSnapshot,
    original_controller_config: Option<&str>,
    original_game_config: Option<&str>,
    players: &BTreeSet<usize>,
) -> Result<DigitalFieldPlan> {
    snapshot.validate(&snapshot.machine)?;
    ensure!(
        snapshot.joystick_enabled,
        "MAME native joystick input is disabled"
    );
    let routes = snapshot.joystick_routes()?;
    let sequences = snapshot_digital_sequences(snapshot, players, &routes)?;
    let defaults = controller_xml_from_sequences(&sequences);
    let mut controller_xml = defaults
        .strip_suffix("</mameconfig>\n")
        .ok_or_else(|| anyhow::anyhow!("Invalid generated MAME controller root"))?
        .to_owned();
    controller_xml.push_str(&format!(
        "<system name=\"{}\">\n<input>\n",
        snapshot.machine
    ));
    let mut mapped_fields = 0;
    let mut disabled_fields = 0;
    let mut unhandled_fields = Vec::new();
    let mut preserved_settings = Vec::new();
    let mut preserved_internal = Vec::new();
    let mut preserved_service = Vec::new();
    // Stable identity ordering, independent of traversal/serialization order.
    let mut fields: Vec<_> = snapshot.fields.iter().collect();
    fields.sort_by(|left, right| {
        (&left.tag, left.mask, &left.input_type, left.defvalue).cmp(&(
            &right.tag,
            right.mask,
            &right.input_type,
            right.defvalue,
        ))
    });
    for field in fields {
        if field.class == inspection::FieldClass::Misc
            && !field.analog
            && matches!(
                field.input_type.as_str(),
                "SERVICE" | "SERVICE1" | "SERVICE2" | "SERVICE3" | "SERVICE4"
            )
        {
            // Explicit overrides remove their fields from this default layer.
            preserved_service.push(field.clone());
            continue;
        }
        if field.class == inspection::FieldClass::Internal {
            preserved_internal.push(field.clone());
            continue;
        }
        if matches!(
            field.class,
            inspection::FieldClass::Config | inspection::FieldClass::Dipswitch
        ) {
            preserved_settings.push(field.clone());
            continue;
        }
        if matches!(
            field.class,
            inspection::FieldClass::Unknown | inspection::FieldClass::Keyboard
        ) {
            unhandled_fields.push(field.clone());
            continue;
        }
        let Some(sequence) = sequences.get(&field.input_type) else {
            unhandled_fields.push(field.clone());
            continue;
        };
        ensure!(
            !field.analog,
            "MAME digital type reported an analog field; inspect the native mode"
        );
        let tag = quick_xml::escape::escape(&field.tag);
        controller_xml.push_str(&format!(
            "  <port tag=\"{tag}\" type=\"{}\" mask=\"{}\" defvalue=\"{}\"><newseq type=\"standard\">{sequence}</newseq></port>\n",
            field.input_type, field.mask, field.defvalue
        ));
        if sequence == "NONE" {
            disabled_fields += 1;
        } else {
            mapped_fields += 1;
        }
    }
    controller_xml.push_str("</input>\n</system>\n</mameconfig>\n");
    let controller_xml =
        preserve_controller_configuration(original_controller_config, &controller_xml)?;
    let game_config_xml = merge_digital_configuration_with_sequences(
        original_game_config,
        &snapshot.machine,
        &sequences,
    )?;
    Ok(DigitalFieldPlan {
        controller_xml,
        game_config_xml,
        mapped_fields,
        disabled_fields,
        unhandled_fields,
        preserved_settings,
        preserved_internal,
        preserved_service,
    })
}

/// Keep the complete original controller layer ahead of our generated sections.
/// MAME applies matching controller systems in file order; its first applicable
/// device map must remain first. This does not write or replace the source file.
fn preserve_controller_configuration(original: Option<&str>, generated: &str) -> Result<String> {
    use quick_xml::{Reader, events::Event};
    let Some(original) = original else {
        return Ok(generated.to_owned());
    };
    ensure!(
        original.len() <= 8 * 1024 * 1024,
        "MAME controller profile exceeds 8 MiB"
    );
    let additions = generated
        .strip_prefix("<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n")
        .and_then(|body| body.strip_suffix("</mameconfig>\n"))
        .ok_or_else(|| anyhow::anyhow!("Invalid generated MAME controller document"))?;
    let mut reader = Reader::from_str(original);
    let mut stack: Vec<String> = Vec::new();
    let mut roots = 0;
    let mut nodes = 0;
    let mut insertion = None;
    let mut empty_root = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event()?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let empty = matches!(event, Event::Empty(_));
                nodes += 1;
                ensure!(
                    nodes <= 100000 && stack.len() < 64,
                    "MAME controller XML exceeds structural limits"
                );
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                let mut attributes = BTreeMap::new();
                for attribute in element.attributes() {
                    let attribute = attribute?;
                    let key = std::str::from_utf8(attribute.key.as_ref())?.to_owned();
                    let value = attribute.unescape_value()?.into_owned();
                    ensure!(
                        attributes.insert(key, value).is_none(),
                        "Duplicate MAME controller XML attribute"
                    );
                }
                if stack.is_empty() {
                    roots += 1;
                    ensure!(
                        roots == 1
                            && name == "mameconfig"
                            && attributes.get("version").map(String::as_str) == Some("10"),
                        "MAME controller profile requires one version-10 root"
                    );
                    if empty {
                        empty_root = Some((start, end));
                    }
                }
                if !empty {
                    stack.push(name);
                }
            }
            Event::End(ref element) => {
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                ensure!(
                    stack.last() == Some(&name),
                    "Unbalanced MAME controller profile"
                );
                if stack.len() == 1 {
                    insertion = Some(start);
                }
                stack.pop();
            }
            Event::Text(text) => {
                let decoded = text.unescape()?;
                ensure!(
                    !stack.is_empty() || decoded.trim().is_empty(),
                    "Text outside MAME controller root"
                );
            }
            Event::CData(_) if stack.is_empty() => {
                anyhow::bail!("CDATA outside MAME controller root")
            }
            Event::DocType(_) => anyhow::bail!("MAME controller profile DTDs are unsupported"),
            Event::Eof => break,
            _ => {}
        }
    }
    ensure!(
        roots == 1 && stack.is_empty(),
        "Incomplete MAME controller profile"
    );
    let mut output = original.to_owned();
    if let Some((start, end)) = empty_root {
        let opening = original[start..end]
            .strip_suffix("/>")
            .ok_or_else(|| anyhow::anyhow!("Invalid empty MAME controller root"))?;
        output.replace_range(start..end, &format!("{opening}>\n{additions}</mameconfig>"));
    } else {
        let position =
            insertion.ok_or_else(|| anyhow::anyhow!("Missing MAME controller closing tag"))?;
        output.insert_str(position, additions);
    }
    Ok(output)
}

/// Native switch token and its exact frontend source with buttons_profiles off.
/// L2/R2 are trigger axes, not MAME numbered buttons 7/8 in this wrapper.
pub(crate) const DIGITAL_CHANNELS: [(&str, &str); 14] = [
    ("HAT1UP", "DPadUp"),
    ("HAT1DOWN", "DPadDown"),
    ("HAT1LEFT", "DPadLeft"),
    ("HAT1RIGHT", "DPadRight"),
    ("BUTTON1", "South"),
    ("BUTTON2", "East"),
    ("BUTTON3", "West"),
    ("BUTTON4", "North"),
    ("BUTTON5", "LeftBumper"),
    ("BUTTON6", "RightBumper"),
    ("BUTTON7", "LeftStick"),
    ("BUTTON8", "RightStick"),
    ("START", "Start"),
    ("SELECT", "Select"),
];

/// Additional switches: the wrapper registers L2/R2 as negative axes,
/// with digital-button fallback. MAME applies its native switch threshold.
/// Generic defaults remain unchanged; inspected non-twin games may assign
/// these channels to native action buttons 9/10.
pub(crate) fn explicit_switch_channels() -> impl Iterator<Item = (&'static str, &'static str)> {
    DIGITAL_CHANNELS.into_iter().chain([
        ("RZAXIS_NEG_SWITCH", "LeftTrigger"),
        ("ZAXIS_NEG_SWITCH", "RightTrigger"),
        ("XAXIS_NEG_SWITCH", "LeftStickLeft"),
        ("XAXIS_POS_SWITCH", "LeftStickRight"),
        ("YAXIS_NEG_SWITCH", "LeftStickUp"),
        ("YAXIS_POS_SWITCH", "LeftStickDown"),
        ("RXAXIS_NEG_SWITCH", "RightStickLeft"),
        ("RXAXIS_POS_SWITCH", "RightStickRight"),
        ("RYAXIS_NEG_SWITCH", "RightStickUp"),
        ("RYAXIS_POS_SWITCH", "RightStickDown"),
    ])
}

pub(crate) fn digital_options() -> BTreeMap<String, String> {
    [
        ("mame_buttons_profiles", "disabled"),
        ("mame_mame_4way_enable", "disabled"),
        ("mame_auto_save", "disabled"),
        ("mame_write_config", "disabled"),
        ("mame_mame_paths_enable", "disabled"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect()
}

/// Require explicit fixed-wrapper settings at the native process boundary.
/// Missing values are not accepted as defaults: upstream defaults can differ.
pub(crate) fn validate_digital_options(config: &str) -> Result<()> {
    validate_required_options(config, &digital_options())
}

/// Native `-mouse` only enables MAME's input class. The pinned libretro
/// wrapper separately gates relative polling on mame_mouse_enable (libretro.cpp
/// check_variables and input_retro.cpp mouse polling). Require explicit evidence
/// rather than relying on the upstream default for opted-in mouse inspections.
pub(crate) fn validate_inspection_options(config: &str, inspect_mouse: bool) -> Result<()> {
    validate_required_options(config, &inspection_options(inspect_mouse))
}

pub(crate) fn inspection_options(inspect_mouse: bool) -> BTreeMap<String, String> {
    let mut required = digital_options();
    if inspect_mouse {
        required.insert("mame_mouse_enable".to_owned(), "enabled".to_owned());
    }
    required
}

fn validate_required_options(config: &str, required: &BTreeMap<String, String>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for line in config.lines() {
        let line = line.trim();
        ensure!(
            !line.starts_with("#include"),
            "MAME option includes are unresolved"
        );
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let Some(expected) = required.get(key) else {
            continue;
        };
        ensure!(
            seen.insert(key.to_owned()),
            "Duplicate required MAME option: {key}"
        );
        let value = value.split('#').next().unwrap_or("").trim();
        ensure!(
            value == expected || value == format!("\"{expected}\""),
            "MAME option {key} does not match the requested input contract"
        );
    }
    ensure!(
        seen.len() == required.len(),
        "MAME inspection lacks explicit required input options"
    );
    Ok(())
}

/// Source-defined standard digital type defaults for an explicit set of ports.
/// All eight frontend ports are represented; unassigned ports get NONE instead
/// of inherited joystick/keyboard alternatives. Start/select are emitted both
/// as per-player types and arcade start/coin panel types, as native defaults do.
///
/// The result is an owned controller configuration, NOT a complete launch plan:
/// per-game field overrides can supersede these defaults. A caller must resolve
/// that layer before claiming the selected game actually uses these sequences.
pub(crate) fn digital_controller_xml(players: &BTreeSet<usize>) -> Result<String> {
    let sequences = digital_type_sequences(players)?;
    Ok(controller_xml_from_sequences(&sequences))
}

fn controller_xml_from_sequences(sequences: &BTreeMap<String, String>) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n<system name=\"default\">\n<input>\n",
    );
    for (input_type, sequence) in sequences {
        xml.push_str(&format!(
            "  <port type=\"{input_type}\"><newseq type=\"standard\">{sequence}</newseq></port>\n"
        ));
    }
    xml.push_str("</input>\n</system>\n</mameconfig>\n");
    xml
}

fn digital_type_sequences(players: &BTreeSet<usize>) -> Result<BTreeMap<String, String>> {
    routed_digital_type_sequences(players, &[1, 2, 3, 4, 5, 6, 7, 8])
}

/// None means a valid snapshot has no standard digital bindings for this port.
/// It does not certify the other fields: callers still inspect the complete
/// DigitalFieldPlan for unsupported controls before accepting a launch.
fn optional_digital_calibration_profile(
    snapshot: &ActiveFieldSnapshot,
    player: usize,
) -> Result<Option<crate::controller_catalog::EmulatorProfile>> {
    snapshot.validate(&snapshot.machine)?;
    let sequences = snapshot_digital_sequences(
        snapshot,
        &BTreeSet::from([player]),
        &[1, 2, 3, 4, 5, 6, 7, 8],
    )?;
    let twin = has_twin_stick(snapshot, player);
    let controls = [
        "up",
        "down",
        "left",
        "right",
        "a",
        "b",
        "x",
        "y",
        "l",
        "r",
        "l3",
        "r3",
        "start",
        "select",
        "trigger_left_switch",
        "trigger_right_switch",
    ];
    let mut bindings = BTreeMap::new();
    for field in &snapshot.fields {
        if !matches!(
            field.class,
            inspection::FieldClass::Controller | inspection::FieldClass::Misc
        ) {
            continue;
        }
        let Some(sequence) = sequences.get(&field.input_type) else {
            continue;
        };
        if sequence == "NONE" {
            continue;
        }
        ensure!(
            !field.analog,
            "MAME digital calibration encountered an analog field"
        );
        for ((item, output), control) in explicit_switch_channels().zip(controls) {
            if sequence == &format!("JOYCODE_{player}_{item}") {
                let control = match field.input_type.strip_prefix(&format!("P{player}_BUTTON")) {
                    Some(button) if twin => format!("button{button}"),
                    _ => control.to_owned(),
                };
                bindings.insert(control, output.to_owned());
            }
        }
    }
    if bindings.is_empty() {
        return Ok(None);
    }
    if twin {
        for (wire, direction) in [
            ("a", "right_down"),
            ("b", "right_right"),
            ("x", "right_left"),
            ("y", "right_up"),
        ] {
            if let Some(output) = bindings.remove(wire) {
                bindings.insert(direction.to_owned(), output);
            }
        }
    }
    // This per-game profile is not registered as a generally supported core.
    Ok(Some(serde_json::from_value(serde_json::json!({
        "id":format!("mame-{}-player-{player}", snapshot.machine),
        "name":format!("MAME {} player {player}", snapshot.machine),
        "core":"mame", "target_layout":if twin { "mame-twin-digital" } else { "mame-fixed-digital" }, "transport":"retroarch",
        "status":"runtime-inspected", "source":"MAME native active-field snapshot",
        "conditions":["Digital fields only; review the selected game's physical assignments."],
        "bindings":bindings, "frontend_ports":8, "requires_fresh_start":true,
        "explicit_selection":true, "core_options":digital_options(),
        "retroarch_launch":{"platforms":[],"device":1,"max_players":8}
    }))?))
}

/// Guidance for fields already classified as unresolved by the actual planner.
/// This does not diagnose hardware capacity or promote an input to supported.
pub(crate) fn relative_axis_requirement(field: &ActiveField) -> Option<(usize, u16)> {
    if !field.analog || field.class != inspection::FieldClass::Controller {
        return None;
    }
    let (prefix, kind) = field.input_type.split_once('_')?;
    let player = (1..=8).find(|player| prefix == format!("P{player}"))?;
    let axis = match kind {
        "DIAL" | "TRACKBALL_X" | "MOUSE_X" => 0,
        "DIAL_V" | "TRACKBALL_Y" | "MOUSE_Y" => 1,
        _ => return None,
    };
    Some((player, axis))
}

/// Advice only; physical relative requirements are reported separately.
pub(crate) fn relative_axis_sequence(
    field: &ActiveField,
    output_axis: u16,
    native_mouse_index: usize,
) -> Result<String> {
    ensure!(
        relative_axis_requirement(field).is_some(),
        "Field is not a recognized relative axis"
    );
    ensure!(output_axis <= 1, "Only relative X/Y output is supported");
    ensure!(
        (1..=255).contains(&native_mouse_index),
        "Invalid native mouse sequence index"
    );
    // Pinned input.cpp token names. Indices must come from mouse_routes(),
    // never a host event-node suffix or frontend mouse-device enumeration.
    Ok(format!(
        "MOUSECODE_{native_mouse_index}_{}AXIS",
        if output_axis == 0 { "X" } else { "Y" }
    ))
}

/// Advice only; physical relative requirements are reported separately.
pub(crate) fn unresolved_field_guidance(field: &ActiveField) -> &'static str {
    match field.class {
        inspection::FieldClass::Unknown => {
            "The native input contract is unknown. Reinspect or implement its contract; do not treat it as an ordinary button or a disabled input."
        }
        inspection::FieldClass::Keyboard => {
            "Choose an explicit keyboard-key switch assignment with fresh keyboard owner/enable-state evidence. A generic arcade preset does not resolve this key."
        }
        _ if field.analog => {
            "Choose a supported analog assignment or explicit button-driven increment/decrement mapping. These modes are not interchangeable with physical relative motion or gun capture."
        }
        inspection::FieldClass::Controller | inspection::FieldClass::Misc => {
            "No default or explicit switch route resolves this input. Choose a source player and channel. Default independent channels are limited; deliberately sharing a channel can activate multiple actions together."
        }
        _ => {
            "This field remains unresolved. Inspect its native contract before assigning it; it is not established as deliberately disabled."
        }
    }
}

fn has_twin_stick(snapshot: &ActiveFieldSnapshot, player: usize) -> bool {
    snapshot.fields.iter().any(|field| {
        field
            .input_type
            .starts_with(&format!("P{player}_JOYSTICKLEFT_"))
            || field
                .input_type
                .starts_with(&format!("P{player}_JOYSTICKRIGHT_"))
    })
}

fn snapshot_digital_sequences(
    snapshot: &ActiveFieldSnapshot,
    players: &BTreeSet<usize>,
    routes: &[usize; 8],
) -> Result<BTreeMap<String, String>> {
    let mut sequences = routed_digital_type_sequences(players, routes)?;
    for player in 1..=8 {
        if !players.contains(&player) {
            // Unselected ports need no independent action-channel allocation.
            // Directions/start/coin are already NONE in the generic layer;
            // include the full known native action range here as well.
            for button in 1..=16 {
                sequences.insert(format!("P{player}_BUTTON{button}"), "NONE".to_owned());
            }
            continue;
        }
        if !has_twin_stick(snapshot, player) {
            // Allocate additional action inputs only from the exact inspected
            // game, never as blanket defaults for unknown cabinet layouts.
            for (button, item) in [(9, "RZAXIS_NEG_SWITCH"), (10, "ZAXIS_NEG_SWITCH")] {
                let input_type = format!("P{player}_BUTTON{button}");
                if snapshot
                    .fields
                    .iter()
                    .any(|field| field.input_type == input_type)
                {
                    let sequence = if players.contains(&player) {
                        format!("JOYCODE_{}_{item}", routes[player - 1])
                    } else {
                        "NONE".to_owned()
                    };
                    sequences.insert(input_type, sequence);
                }
            }
            // Preserve conventional channels for observed buttons 1-10, then
            // allocate sparse higher-numbered actions only to unused channels.
            let observed: BTreeSet<_> = (1..=16)
                .filter(|button| {
                    snapshot
                        .fields
                        .iter()
                        .any(|field| field.input_type == format!("P{player}_BUTTON{button}"))
                })
                .collect();
            let action_channels = [
                "BUTTON1",
                "BUTTON2",
                "BUTTON3",
                "BUTTON4",
                "BUTTON5",
                "BUTTON6",
                "BUTTON7",
                "BUTTON8",
                "RZAXIS_NEG_SWITCH",
                "ZAXIS_NEG_SWITCH",
            ];
            // Retain a useful partial review when the channel budget is
            // exhausted. Excess high-numbered actions get no sequence and
            // remain unhandled, so staging/launch still require explicit
            // resolution rather than silently disabling or sharing them.
            let free = action_channels
                .iter()
                .enumerate()
                .filter(|(index, _)| !observed.contains(&(index + 1)))
                .map(|(_, item)| *item);
            for (button, item) in observed.iter().filter(|button| **button > 10).zip(free) {
                let sequence = if players.contains(&player) {
                    format!("JOYCODE_{}_{item}", routes[player - 1])
                } else {
                    "NONE".to_owned()
                };
                sequences.insert(format!("P{player}_BUTTON{button}"), sequence);
            }
            continue;
        }
        // Generic defaults alias the ordinary cluster to the left stick.
        // In a twin-stick machine that would silently share a third cluster.
        // Omit those routes instead: observed third-cluster fields remain
        // unresolved and editable, while known twin-stick fields still review.
        for direction in ["UP", "DOWN", "LEFT", "RIGHT"] {
            sequences.remove(&format!("P{player}_JOYSTICK_{direction}"));
        }
        let buttons: Vec<_> = (1..=16)
            .filter(|button| {
                snapshot
                    .fields
                    .iter()
                    .any(|field| field.input_type == format!("P{player}_BUTTON{button}"))
            })
            .collect();
        let action_channels = [
            "BUTTON5",
            "BUTTON6",
            "BUTTON7",
            "BUTTON8",
            "RZAXIS_NEG_SWITCH",
            "ZAXIS_NEG_SWITCH",
        ];
        // Reserve native switches 1-4 for right-stick directions. Clear all
        // numbered action defaults, then allocate only as many observed actions
        // as the independent channels support. Missing excess sequences are
        // unhandled inputs, not NONE (which would mean intentionally disabled).
        for button in 1..=16 {
            sequences.remove(&format!("P{player}_BUTTON{button}"));
        }
        for (button, channel) in buttons.iter().zip(action_channels) {
            let output = format!("JOYCODE_{}_{channel}", routes[player - 1]);
            sequences.insert(format!("P{player}_BUTTON{button}"), output);
        }
    }
    Ok(sequences)
}

fn routed_digital_type_sequences(
    players: &BTreeSet<usize>,
    routes: &[usize; 8],
) -> Result<BTreeMap<String, String>> {
    ensure!(
        !players.is_empty() && players.iter().all(|player| (1..=8).contains(player)),
        "MAME digital routing requires explicit frontend ports in 1..=8"
    );
    ensure!(
        routes.iter().all(|slot| (1..=255).contains(slot))
            && routes.iter().collect::<BTreeSet<_>>().len() == 8,
        "Invalid native MAME joystick slot mapping"
    );
    let mut sequences = BTreeMap::new();
    for player in 1..=8 {
        for (item, _) in DIGITAL_CHANNELS {
            let types = match item {
                "HAT1UP" => vec![
                    format!("P{player}_JOYSTICK_UP"),
                    format!("P{player}_JOYSTICKLEFT_UP"),
                ],
                "HAT1DOWN" => vec![
                    format!("P{player}_JOYSTICK_DOWN"),
                    format!("P{player}_JOYSTICKLEFT_DOWN"),
                ],
                "HAT1LEFT" => vec![
                    format!("P{player}_JOYSTICK_LEFT"),
                    format!("P{player}_JOYSTICKLEFT_LEFT"),
                ],
                "HAT1RIGHT" => vec![
                    format!("P{player}_JOYSTICK_RIGHT"),
                    format!("P{player}_JOYSTICKLEFT_RIGHT"),
                ],
                "BUTTON1" => vec![
                    format!("P{player}_BUTTON1"),
                    format!("P{player}_JOYSTICKRIGHT_DOWN"),
                ],
                "BUTTON2" => vec![
                    format!("P{player}_BUTTON2"),
                    format!("P{player}_JOYSTICKRIGHT_RIGHT"),
                ],
                "BUTTON3" => vec![
                    format!("P{player}_BUTTON3"),
                    format!("P{player}_JOYSTICKRIGHT_LEFT"),
                ],
                "BUTTON4" => vec![
                    format!("P{player}_BUTTON4"),
                    format!("P{player}_JOYSTICKRIGHT_UP"),
                ],
                "START" => vec![format!("P{player}_START"), format!("START{player}")],
                "SELECT" => vec![format!("P{player}_SELECT"), format!("COIN{player}")],
                _ => vec![format!("P{player}_{item}")],
            };
            let sequence = if players.contains(&player) {
                format!("JOYCODE_{}_{item}", routes[player - 1])
            } else {
                "NONE".to_owned()
            };
            for input_type in types {
                sequences.insert(input_type, sequence.clone());
            }
        }
    }
    Ok(sequences)
}

/// Replace standard sequences of the controlled digital types in one exact
/// machine's saved configuration. Preserve field attributes, nonstandard
/// sequences and all unrelated bytes. Stage the result privately, never over
/// the original. Fields absent from saved configuration still require defaults
/// and native driver discovery; this function does not invent their identities.
pub(crate) fn merge_digital_configuration(
    source: Option<&str>,
    machine: &str,
    players: &BTreeSet<usize>,
) -> Result<String> {
    let sequences = digital_type_sequences(players)?;
    merge_digital_configuration_with_sequences(source, machine, &sequences)
}

fn merge_digital_configuration_with_sequences(
    source: Option<&str>,
    machine: &str,
    sequences: &BTreeMap<String, String>,
) -> Result<String> {
    use quick_xml::{Reader, events::Event};
    ensure!(
        !machine.is_empty()
            && machine.len() <= 64
            && machine != "default"
            && machine
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
        "Expected an exact MAME machine short name"
    );
    let Some(source) = source else {
        return Ok(format!(
            "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\"><system name=\"{machine}\" /></mameconfig>\n"
        ));
    };
    ensure!(
        source.len() <= 8 * 1024 * 1024,
        "MAME configuration exceeds 8 MiB"
    );
    let mut reader = Reader::from_str(source);
    let mut stack: Vec<String> = Vec::new();
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let (mut roots, mut systems, mut inputs, mut nodes) = (0, 0, 0, 0);
    let mut active = false;
    let mut port_sequence: Option<String> = None;
    let mut standard_count = 0;
    let mut sequence_start = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event()?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let empty = matches!(event, Event::Empty(_));
                nodes += 1;
                ensure!(
                    nodes <= 100000 && stack.len() < 64,
                    "MAME XML exceeds structural limits"
                );
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                let mut attributes = BTreeMap::new();
                for attribute in element.attributes() {
                    let attribute = attribute?;
                    let key = std::str::from_utf8(attribute.key.as_ref())?.to_owned();
                    let value = attribute.unescape_value()?.into_owned();
                    ensure!(
                        attributes.insert(key, value).is_none(),
                        "Duplicate MAME XML attribute"
                    );
                }
                let attr = |key: &str| attributes.get(key).map(String::as_str);
                if stack.is_empty() {
                    roots += 1;
                    ensure!(
                        roots == 1 && name == "mameconfig" && attr("version") == Some("10"),
                        "MAME requires one version-10 configuration root"
                    );
                }
                if stack.len() == 1 && name == "system" && attr("name") == Some(machine) {
                    systems += 1;
                    ensure!(systems == 1, "Duplicate selected MAME system");
                    active = !empty;
                }
                if active && stack.len() == 2 && name == "input" {
                    inputs += 1;
                    ensure!(inputs == 1, "Duplicate selected MAME input section");
                }
                if active && stack.len() == 3 && stack[2] == "input" && name == "port" {
                    port_sequence = attr("type").and_then(|kind| sequences.get(kind)).cloned();
                    standard_count = 0;
                    if empty && let Some(sequence) = port_sequence.take() {
                        let opening = source[start..end]
                            .strip_suffix("/>")
                            .ok_or_else(|| anyhow::anyhow!("Invalid empty MAME port"))?;
                        edits.push((
                            start,
                            end,
                            format!(
                                "{opening}><newseq type=\"standard\">{sequence}</newseq></port>"
                            ),
                        ));
                    }
                }
                if port_sequence.is_some()
                    && stack.len() == 4
                    && name == "newseq"
                    && attr("type") == Some("standard")
                {
                    standard_count += 1;
                    ensure!(
                        standard_count == 1,
                        "Duplicate MAME standard sequence for one field"
                    );
                    if empty {
                        edits.push((
                            start,
                            end,
                            format!(
                                "<newseq type=\"standard\">{}</newseq>",
                                port_sequence.as_ref().unwrap()
                            ),
                        ));
                    } else {
                        sequence_start = Some(start);
                    }
                }
                ensure!(
                    sequence_start.is_none() || stack.len() <= 4,
                    "Nested elements in a MAME input sequence are unsupported"
                );
                if !empty {
                    stack.push(name);
                }
            }
            Event::End(ref element) => {
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                ensure!(stack.last() == Some(&name), "Unbalanced MAME XML");
                if stack.len() == 5
                    && name == "newseq"
                    && let Some(sequence_start) = sequence_start.take()
                {
                    edits.push((
                        sequence_start,
                        end,
                        format!(
                            "<newseq type=\"standard\">{}</newseq>",
                            port_sequence.as_ref().unwrap()
                        ),
                    ));
                }
                if stack.len() == 4
                    && name == "port"
                    && let Some(sequence) = port_sequence.take()
                {
                    if standard_count == 0 {
                        edits.push((
                            start,
                            start,
                            format!("<newseq type=\"standard\">{sequence}</newseq>"),
                        ));
                    }
                }
                if active && stack.len() == 2 && name == "system" {
                    active = false;
                }
                stack.pop();
            }
            Event::DocType(_) => anyhow::bail!("MAME configuration DTDs are unsupported"),
            Event::Text(text) if stack.is_empty() => {
                ensure!(
                    text.unescape()?.trim().is_empty(),
                    "Text outside MAME XML root"
                );
            }
            Event::CData(_) if stack.is_empty() => anyhow::bail!("CDATA outside MAME XML root"),
            Event::Eof => break,
            _ => {}
        }
    }
    ensure!(
        roots == 1 && systems == 1 && stack.is_empty(),
        "MAME configuration lacks the exact selected system or is incomplete"
    );
    let mut output = source.to_owned();
    for (start, end, replacement) in edits.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    Ok(output)
}
