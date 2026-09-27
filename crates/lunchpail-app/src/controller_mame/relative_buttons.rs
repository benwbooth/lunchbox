//! Explicit native mouse-button planning only. This does not authorize capture
//! or establish isolation from MAME UI defaults and pointer events.
use super::{
    ActiveField, ActiveFieldSnapshot, AnalogAssignment, DigitalAssignment, DigitalFieldPlan,
    RelativeAssignment,
};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelativeButtonAssignment {
    pub field: ActiveField,
    pub source_player: usize,
    /// Native ordinary mouse button 1–5, not an evdev code or wheel event.
    pub output_button: u16,
}

/// Pinned RetroArch udev input: BTN_LEFT/RIGHT/MIDDLE/SIDE/EXTRA feed
/// libretro mouse left/right/middle/4/5, registered as MAME BUTTON1–BUTTON5.
pub(crate) fn output_evdev_code(button: u16) -> Result<u16> {
    ensure!(
        (1..=5).contains(&button),
        "Only ordinary native mouse buttons 1–5 are supported"
    );
    Ok(0x110 + button - 1)
}

/// Settings preparation only; callers still need explicit native UI isolation
/// and a launch contract before opening devices with these button permissions.
pub(crate) fn prepare_sources(
    axes: &[RelativeAssignment],
    buttons: &[RelativeButtonAssignment],
    sources: &[super::RelativeSource],
) -> Result<BTreeMap<u8, crate::controller_axis::relative_settings::RelativeDeviceSettings>> {
    super::relative::prepare_relative_sources_with_buttons(axes, buttons, sources)
}

fn identity(field: &ActiveField) -> (&str, u32, &str, u32) {
    (&field.tag, field.mask, &field.input_type, field.defvalue)
}

/// Derived profile input only. Preserve the original snapshot as native
/// evidence, and use the full combined planner to validate route ownership.
pub(crate) fn gamepad_snapshot(
    snapshot: &ActiveFieldSnapshot,
    buttons: &[RelativeButtonAssignment],
) -> Result<ActiveFieldSnapshot> {
    ensure!(
        buttons.len() <= 32768,
        "Too many mouse-button profile fields"
    );
    let inspected: BTreeMap<_, _> = snapshot
        .fields
        .iter()
        .map(|field| (identity(field), field))
        .collect();
    ensure!(
        inspected.len() == snapshot.fields.len(),
        "Duplicate inspected field identity"
    );
    let mut owned = BTreeSet::new();
    for button in buttons {
        ensure!(
            inspected
                .get(&identity(&button.field))
                .is_some_and(|known| *known == &button.field),
            "Mouse-owned profile field does not exactly match inspection"
        );
        ensure!(
            owned.insert(identity(&button.field)),
            "Duplicate mouse-owned profile field"
        );
    }
    let keep = |field: &ActiveField| !owned.contains(&identity(field));
    let mut derived = snapshot.clone();
    derived.fields.retain(keep);
    derived.field_labels.retain(|entry| keep(&entry.field));
    derived.analog_states.retain(|entry| keep(&entry.field));
    if let Some(keyboard) = &mut derived.keyboard_state {
        keyboard.field_owners.retain(|entry| keep(&entry.field));
    }
    Ok(derived)
}

/// Reject mouse sequences on saved UI ports without changing user bindings.
/// Check every system block conservatively: controller profiles can apply via
/// native inheritance, not only the selected short machine name. Callers must
/// provide every effective configuration file; absence is not runtime evidence.
pub(crate) fn validate_saved_ui_mouse_bindings(xml: &str) -> Result<()> {
    use quick_xml::{Reader, events::Event};
    // Reuse the bounded document validator (size, depth, nodes, attributes,
    // balanced version-10 root, no DTDs). Discard its in-memory merge result.
    super::preserve_controller_configuration(
        Some(xml),
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n</mameconfig>\n",
    )?;
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<String> = Vec::new();
    let mut ui_port: Option<(usize, String)> = None;
    let mut sequence: Option<String> = None;
    loop {
        match reader.read_event()? {
            Event::Start(element) => {
                ensure!(sequence.is_none(), "Nested elements in saved UI sequence");
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                if name == "port" {
                    ensure!(ui_port.is_none(), "Nested saved UI port");
                    for attribute in element.attributes() {
                        let attribute = attribute?;
                        if attribute.key.as_ref() == b"type" {
                            let kind = attribute.unescape_value()?.into_owned();
                            if kind.starts_with("UI_") {
                                ui_port = Some((stack.len(), kind));
                            }
                        }
                    }
                } else if name == "newseq" && ui_port.is_some() {
                    ensure!(
                        stack.last().is_some_and(|parent| parent == "port"),
                        "Unexpected saved UI sequence location"
                    );
                    sequence = Some(String::new());
                }
                stack.push(name);
            }
            Event::Empty(_) => ensure!(sequence.is_none(), "Nested elements in saved UI sequence"),
            Event::Text(text) => {
                if let Some(sequence) = &mut sequence {
                    sequence.push_str(&text.unescape()?);
                }
            }
            Event::CData(text) => {
                if let Some(sequence) = &mut sequence {
                    sequence.push_str(std::str::from_utf8(text.as_ref())?);
                }
            }
            Event::End(element) => {
                if element.name().as_ref() == b"newseq" {
                    if let Some(sequence) = sequence.take() {
                        ensure!(
                            !sequence.to_ascii_uppercase().contains("MOUSECODE_"),
                            "Saved MAME UI action {} uses mouse input; resolve that binding before enabling game mouse buttons",
                            ui_port
                                .as_ref()
                                .map(|(_, kind)| kind.as_str())
                                .unwrap_or("unknown")
                        );
                    }
                }
                stack.pop();
                if ui_port
                    .as_ref()
                    .is_some_and(|(depth, _)| *depth == stack.len())
                {
                    ui_port = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

/// Compose exact switch ownership with relative axes and gamepad assignments.
/// A successful plan is not permission to forward physical mouse buttons.
pub(crate) fn plan_relative_button_fields(
    snapshot: &ActiveFieldSnapshot,
    original_controller: Option<&str>,
    original_game: Option<&str>,
    players: &BTreeSet<usize>,
    analog: &[AnalogAssignment],
    digital: &[DigitalAssignment],
    relative: &[RelativeAssignment],
    buttons: &[RelativeButtonAssignment],
) -> Result<DigitalFieldPlan> {
    if buttons.is_empty() {
        return super::plan_relative_fields(
            snapshot,
            original_controller,
            original_game,
            players,
            analog,
            digital,
            relative,
        );
    }
    ensure!(
        buttons.len() <= 32768,
        "Too many relative button assignments"
    );
    for original in [original_controller, original_game].into_iter().flatten() {
        validate_saved_ui_mouse_bindings(original)?;
    }
    snapshot.validate(&snapshot.machine)?;
    let routes = snapshot.mouse_routes()?;
    let inspected: BTreeMap<_, _> = snapshot
        .fields
        .iter()
        .map(|field| (identity(field), field))
        .collect();
    ensure!(
        inspected.len() == snapshot.fields.len(),
        "Duplicate inspected field identity"
    );
    let occupied: BTreeSet<_> = analog
        .iter()
        .map(|entry| &entry.field)
        .chain(digital.iter().map(|entry| &entry.field))
        .chain(relative.iter().map(|entry| &entry.field))
        .map(identity)
        .collect();
    let mut ordered: Vec<_> = buttons.iter().collect();
    ordered.sort_by_key(|entry| identity(&entry.field));
    let mut seen = BTreeSet::new();
    let mut xml = format!(
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n<system name=\"{}\">\n<input>\n",
        snapshot.machine
    );
    for entry in &ordered {
        let field = &entry.field;
        ensure!(
            (1..=8).contains(&entry.source_player) && players.contains(&entry.source_player),
            "Mouse-button source player is not selected"
        );
        ensure!(
            (1..=5).contains(&entry.output_button),
            "Only ordinary native mouse buttons 1–5 are supported"
        );
        ensure!(
            inspected
                .get(&identity(field))
                .is_some_and(|known| *known == field),
            "Mouse-button field does not exactly match inspection"
        );
        ensure!(
            !field.analog
                && matches!(
                    field.class,
                    super::inspection::FieldClass::Controller | super::inspection::FieldClass::Misc
                ),
            "Mouse buttons require an inspected controller or auxiliary switch field"
        );
        ensure!(
            seen.insert(identity(field)),
            "Duplicate mouse-button field assignment"
        );
        ensure!(
            !occupied.contains(&identity(field)),
            "Mouse-button field already has another assignment"
        );
        let mouse = routes[entry.source_player - 1];
        ensure!(
            (1..=255).contains(&mouse),
            "Invalid inspected native mouse index"
        );
        let tag = quick_xml::escape::escape(&field.tag);
        xml.push_str(&format!("<port tag=\"{tag}\" type=\"{}\" mask=\"{}\" defvalue=\"{}\"><newseq type=\"standard\">MOUSECODE_{mouse}_BUTTON{}</newseq></port>\n",
            field.input_type, field.mask, field.defvalue, entry.output_button));
    }
    xml.push_str("</input>\n</system>\n</mameconfig>\n");
    // Explicit mouse ownership must remove the field from default gamepad
    // inference, rather than leave a second implicit source for the same action.
    let defaults = gamepad_snapshot(snapshot, buttons)?;
    let mut plan = super::plan_relative_fields(
        &defaults,
        original_controller,
        original_game,
        players,
        analog,
        digital,
        relative,
    )?;
    plan.controller_xml =
        super::preserve_controller_configuration(Some(&plan.controller_xml), &xml)?;
    let fields: Vec<_> = ordered.iter().map(|entry| &entry.field).collect();
    plan.game_config_xml = super::analog::remove_saved_field_sequences(
        &plan.game_config_xml,
        &snapshot.machine,
        &fields,
    )?;
    plan.mapped_fields += buttons.len();
    Ok(plan)
}
