//! Native relative-axis composition; physical transport is a separate contract.
use super::{
    ActiveField, ActiveFieldSnapshot, AnalogAssignment, DigitalAssignment, DigitalFieldPlan,
};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelativeAssignment {
    pub field: ActiveField,
    pub source_player: usize,
    pub output_axis: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelativeSource {
    pub source_player: usize,
    pub device: crate::controller_axis::relative_settings::RelativeDeviceSettings,
}

/// Saved structure only. Runtime must reopen exact identities and restrict
/// forwarded controls to those explicitly assigned by the session.
fn validate_relative_sources(
    assignments: &[RelativeAssignment],
    buttons: &[super::relative_buttons::RelativeButtonAssignment],
    sources: &[RelativeSource],
) -> Result<()> {
    ensure!(sources.len() <= 8, "Too many relative source players");
    ensure!(
        assignments.len() <= 32768 && buttons.len() <= 32768,
        "Too many relative source assignments"
    );
    let required: BTreeSet<_> = assignments
        .iter()
        .map(|entry| entry.source_player)
        .chain(buttons.iter().map(|entry| entry.source_player))
        .collect();
    let mut selected = BTreeSet::new();
    for source in sources {
        ensure!(
            (1..=8).contains(&source.source_player) && selected.insert(source.source_player),
            "Invalid or duplicate relative source player"
        );
        for assignment in assignments
            .iter()
            .filter(|entry| entry.source_player == source.source_player)
        {
            ensure!(
                assignment.output_axis <= 1
                    && source.device.supports_output(2, assignment.output_axis)?,
                "Selected relative device does not supply every assigned X/Y output"
            );
        }
        for assignment in buttons
            .iter()
            .filter(|entry| entry.source_player == source.source_player)
        {
            let output = super::relative_buttons::output_evdev_code(assignment.output_button)?;
            ensure!(
                source.device.supports_output(1, output)?,
                "Selected relative device does not supply every assigned mouse-button output"
            );
        }
    }
    ensure!(
        selected == required,
        "Relative source selections must exactly cover assigned source players"
    );
    let devices: Vec<_> = sources.iter().map(|source| source.device.clone()).collect();
    crate::controller_axis::relative_settings::validate_devices(&devices)
}

/// Prepare only the physical axes that feed assigned post-transform outputs.
/// This creates session settings, not a reader, grab or virtual device.
pub(crate) fn prepare_relative_sources(
    assignments: &[RelativeAssignment],
    sources: &[RelativeSource],
) -> Result<BTreeMap<u8, crate::controller_axis::relative_settings::RelativeDeviceSettings>> {
    // Existing axis-only callers cannot acquire click permissions merely
    // because their saved device contains button mappings.
    prepare_relative_sources_with_buttons(assignments, &[], sources)
}

pub(super) fn prepare_relative_sources_with_buttons(
    assignments: &[RelativeAssignment],
    buttons: &[super::relative_buttons::RelativeButtonAssignment],
    sources: &[RelativeSource],
) -> Result<BTreeMap<u8, crate::controller_axis::relative_settings::RelativeDeviceSettings>> {
    validate_relative_sources(assignments, buttons, sources)?;
    let mut prepared = BTreeMap::new();
    for source in sources {
        let mut device = source.device.clone();
        let required: BTreeSet<_> = assignments
            .iter()
            .filter(|entry| entry.source_player == source.source_player)
            .map(|entry| {
                if device.motion.swap_xy {
                    entry.output_axis ^ 1
                } else {
                    entry.output_axis
                }
            })
            .collect();
        device.axes = required.into_iter().collect();
        let required_buttons: BTreeSet<_> = buttons
            .iter()
            .filter(|entry| entry.source_player == source.source_player)
            .map(|entry| super::relative_buttons::output_evdev_code(entry.output_button))
            .collect::<Result<_>>()?;
        // Retain the physical-to-output remap, but no unassigned click or
        // scroll permissions from the broader saved device.
        device
            .buttons
            .retain(|(_, output)| required_buttons.contains(output));
        device.validate()?;
        prepared.insert(u8::try_from(source.source_player)?, device);
    }
    Ok(prepared)
}

pub(crate) fn plan_relative_fields(
    snapshot: &ActiveFieldSnapshot,
    original_controller: Option<&str>,
    original_game: Option<&str>,
    players: &BTreeSet<usize>,
    analog: &[AnalogAssignment],
    digital: &[DigitalAssignment],
    relative: &[RelativeAssignment],
) -> Result<DigitalFieldPlan> {
    if relative.is_empty() {
        return super::plan_explicit_fields(
            snapshot,
            original_controller,
            original_game,
            players,
            analog,
            digital,
        );
    }
    ensure!(relative.len() <= 32768, "Too many relative assignments");
    let routes = snapshot.mouse_routes()?;
    fn identity(field: &ActiveField) -> (&str, u32, &str, u32) {
        (&field.tag, field.mask, &field.input_type, field.defvalue)
    }
    let inspected: BTreeMap<_, _> = snapshot
        .fields
        .iter()
        .map(|field| (identity(field), field))
        .collect();
    ensure!(
        inspected.len() == snapshot.fields.len(),
        "Duplicate inspected field identity"
    );
    let other_owners: BTreeSet<_> = analog
        .iter()
        .map(|entry| &entry.field)
        .chain(digital.iter().map(|entry| &entry.field))
        .map(identity)
        .collect();
    let mut ordered: Vec<_> = relative.iter().collect();
    ordered.sort_by_key(|entry| {
        (
            &entry.field.tag,
            entry.field.mask,
            &entry.field.input_type,
            entry.field.defvalue,
        )
    });
    let mut seen = BTreeSet::new();
    let mut xml = format!(
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n<system name=\"{}\">\n<input>\n",
        snapshot.machine
    );
    for entry in &ordered {
        let field = &entry.field;
        ensure!(
            (1..=8).contains(&entry.source_player) && players.contains(&entry.source_player),
            "Relative assignment source player is not selected"
        );
        ensure!(
            inspected
                .get(&identity(field))
                .is_some_and(|known| *known == field),
            "Relative field does not uniquely match inspection"
        );
        ensure!(
            seen.insert(identity(field)),
            "Duplicate relative field assignment"
        );
        ensure!(
            !other_owners.contains(&identity(field)),
            "Relative field already has an analog or digital assignment"
        );
        let sequence = super::relative_axis_sequence(
            field,
            entry.output_axis,
            routes[entry.source_player - 1],
        )?;
        let tag = quick_xml::escape::escape(&field.tag);
        xml.push_str(&format!("<port tag=\"{tag}\" type=\"{}\" mask=\"{}\" defvalue=\"{}\"><newseq type=\"standard\">{sequence}</newseq><newseq type=\"increment\">NONE</newseq><newseq type=\"decrement\">NONE</newseq></port>\n", field.input_type, field.mask, field.defvalue));
    }
    xml.push_str("</input>\n</system>\n</mameconfig>\n");
    let keep = |field: &ActiveField| !seen.contains(&identity(field));
    let mut defaults = snapshot.clone();
    defaults.fields.retain(keep);
    defaults.field_labels.retain(|entry| keep(&entry.field));
    defaults.analog_states.retain(|entry| keep(&entry.field));
    let mut plan = super::plan_explicit_fields(
        &defaults,
        original_controller,
        original_game,
        players,
        analog,
        digital,
    )?;
    plan.controller_xml =
        super::preserve_controller_configuration(Some(&plan.controller_xml), &xml)?;
    let fields: Vec<_> = ordered.iter().map(|entry| &entry.field).collect();
    plan.game_config_xml = super::analog::remove_saved_field_sequences(
        &plan.game_config_xml,
        &snapshot.machine,
        &fields,
    )?;
    plan.mapped_fields += relative.len();
    Ok(plan)
}
