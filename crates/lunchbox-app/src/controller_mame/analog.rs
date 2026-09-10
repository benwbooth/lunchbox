//! Explicit native absolute-axis assignments, not inferred gamepad presets.
use super::{ActiveField, ActiveFieldSnapshot, inspection::FieldClass};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

/// Compose the existing digital plan with explicit absolute fields. Original
/// device maps stay first; analog defaults follow digital defaults. Remove only
/// selected fields' saved sequences, retaining sensitivity/reverse attributes.
pub(crate) fn plan_mixed_fields(
    snapshot: &ActiveFieldSnapshot,
    original_controller: Option<&str>,
    original_game: Option<&str>,
    players: &BTreeSet<usize>,
    assignments: &[AnalogAssignment],
) -> Result<super::DigitalFieldPlan> {
    ensure!(
        assignments
            .iter()
            .all(|assignment| players.contains(&assignment.source_player)),
        "Analog source ports must belong to the explicitly selected frontend ports"
    );
    let analog_xml = controller_xml(snapshot, assignments)?;
    let mut plan =
        super::plan_digital_layer(snapshot, original_controller, original_game, players)?;
    plan.controller_xml =
        super::preserve_controller_configuration(Some(&plan.controller_xml), &analog_xml)?;
    let fields: Vec<_> = assignments
        .iter()
        .map(|assignment| &assignment.field)
        .collect();
    plan.game_config_xml =
        remove_saved_field_sequences(&plan.game_config_xml, &snapshot.machine, &fields)?;
    plan.unhandled_fields.retain(|field| {
        !assignments
            .iter()
            .any(|assignment| assignment.field == *field)
    });
    plan.mapped_fields += assignments.len();
    Ok(plan)
}

pub(super) fn remove_saved_field_sequences(
    source: &str,
    machine: &str,
    fields: &[&ActiveField],
) -> Result<String> {
    use quick_xml::{Reader, events::Event};
    // Reuse the bounded root/system/attribute validation, without digital edits.
    let source =
        super::merge_digital_configuration_with_sequences(Some(source), machine, &BTreeMap::new())?;
    let mut reader = Reader::from_str(&source);
    let mut stack = Vec::new();
    let mut selected_system = false;
    let mut selected_port = false;
    let mut removal_start = None;
    let mut removals = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event()?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let empty = matches!(event, Event::Empty(_));
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                let mut attrs = BTreeMap::new();
                for attribute in element.attributes() {
                    let attribute = attribute?;
                    attrs.insert(
                        std::str::from_utf8(attribute.key.as_ref())?.to_owned(),
                        attribute.unescape_value()?.into_owned(),
                    );
                }
                let attr = |key: &str| attrs.get(key).map(String::as_str);
                if stack.len() == 1 && name == "system" {
                    selected_system = attr("name") == Some(machine) && !empty;
                }
                if selected_system && stack.len() == 3 && stack[2] == "input" && name == "port" {
                    selected_port = false;
                    for field in fields {
                        if attr("type") != Some(field.input_type.as_str())
                            || attr("tag") != Some(field.tag.as_str())
                        {
                            continue;
                        }
                        // MAME compares defvalue after masking it. Its own saved
                        // XML emits decimal integers; unsupported encodings fail.
                        let mask: u32 = attr("mask").unwrap_or("0").parse()?;
                        let defvalue: u32 = attr("defvalue").unwrap_or("0").parse()?;
                        if mask == field.mask && defvalue & mask == field.defvalue & mask {
                            selected_port = !empty;
                        }
                    }
                }
                if selected_port
                    && stack.len() == 4
                    && name == "newseq"
                    && matches!(attr("type"), Some("standard" | "increment" | "decrement"))
                {
                    if empty {
                        removals.push((start, end));
                    } else {
                        removal_start = Some(start);
                    }
                }
                ensure!(
                    removal_start.is_none() || stack.len() <= 4,
                    "Nested elements in an analog sequence are unsupported"
                );
                if !empty {
                    stack.push(name);
                }
            }
            Event::End(element) => {
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                if stack.len() == 5
                    && name == "newseq"
                    && let Some(start) = removal_start.take()
                {
                    removals.push((start, end));
                }
                if stack.len() == 4 && name == "port" {
                    selected_port = false;
                }
                if stack.len() == 2 && name == "system" {
                    selected_system = false;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let mut output = source;
    for (start, end) in removals.into_iter().rev() {
        output.replace_range(start..end, "");
    }
    Ok(output)
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AnalogChannel {
    LeftX,
    LeftY,
    RightX,
    RightY,
    LeftPressure,
    RightPressure,
}

impl AnalogChannel {
    pub(crate) fn controls(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::LeftX => &[
                ("stick_left", "LeftStickLeft"),
                ("stick_right", "LeftStickRight"),
            ],
            Self::LeftY => &[("stick_up", "LeftStickUp"), ("stick_down", "LeftStickDown")],
            Self::RightX => &[
                ("right_stick_left", "RightStickLeft"),
                ("right_stick_right", "RightStickRight"),
            ],
            Self::RightY => &[
                ("right_stick_up", "RightStickUp"),
                ("right_stick_down", "RightStickDown"),
            ],
            Self::LeftPressure => &[("l2", "LeftTrigger")],
            Self::RightPressure => &[("r2", "RightTrigger")],
        }
    }
    fn item(self) -> &'static str {
        // input_retro.cpp:1311-1349. Trigger order is intentionally RZ, Z.
        // The core negates 0..32767 pressure (lines 779-796); its pedal
        // assignments use NEG (1482-1490). read_as_absolute expands that
        // negative half to the complete native range, including released=min.
        match self {
            Self::LeftX => "XAXIS",
            Self::LeftY => "YAXIS",
            Self::RightX => "RXAXIS",
            Self::RightY => "RYAXIS",
            Self::LeftPressure => "RZAXIS_NEG",
            Self::RightPressure => "ZAXIS_NEG",
        }
    }
}

/// Combine the chosen digital geometry and explicit analog output requirements.
/// This derives a profile; it does not establish physical measurement validity.
pub(crate) fn combined_profile(
    snapshot: &ActiveFieldSnapshot,
    assignments: &[AnalogAssignment],
    player: usize,
    digital_layout: super::DigitalLayout,
) -> Result<Option<crate::controller_catalog::EmulatorProfile>> {
    let channels: BTreeSet<_> = assignments
        .iter()
        .filter(|assignment| assignment.source_player == player)
        .map(|assignment| assignment.channel)
        .collect();
    if channels.is_empty() {
        return super::optional_digital_profile(snapshot, player, digital_layout);
    }
    // Validate the complete explicit field selection before deriving a profile.
    controller_xml(snapshot, assignments)?;
    let pressure = [
        channels.contains(&AnalogChannel::LeftPressure),
        channels.contains(&AnalogChannel::RightPressure),
    ];
    let digital =
        super::arcade::optional_profile_with_pressure(snapshot, player, digital_layout, pressure)?;
    let mut profile = match digital {
        Some(mut profile) => {
            profile.target_layout.push_str("-analog");
            profile
        }
        None => {
            // An empty default digital subset must not discard the requested
            // geometry: explicit switches can be composed into this profile
            // afterward. Pure analog Automatic still uses fixed channels.
            let target = match digital_layout {
                super::DigitalLayout::SixButton => "arcade-six-button-analog",
                super::DigitalLayout::EightButton => "arcade-eight-button-analog",
                super::DigitalLayout::NeoGeo => "neogeo-analog",
                super::DigitalLayout::Automatic | super::DigitalLayout::FixedChannels => {
                    "mame-fixed-digital-analog"
                }
            };
            serde_json::from_value(serde_json::json!({
            "id":format!("mame-{}-player-{player}", snapshot.machine),
            "name":format!("MAME {} player {player}", snapshot.machine),
            "core":"mame","target_layout":target,
            "transport":"retroarch","status":"runtime-inspected",
            "source":"MAME native active-field snapshot","conditions":[],"bindings":{},
            "frontend_ports":8,"requires_fresh_start":true,"explicit_selection":true,
            "core_options":super::digital_options(),
            "retroarch_launch":{"platforms":[],"device":1,"max_players":8}
            }))?
        }
    };
    if pressure.into_iter().any(|shared| shared) {
        profile.conditions.push("Pressure outputs occupy measured analog controls rather than extra button positions. Native switches sharing those outputs follow the same pressure travel and threshold.".to_owned());
    }
    let target = crate::controller_catalog::catalog()
        .layout(&profile.target_layout)
        .ok_or_else(|| anyhow::anyhow!("Missing combined MAME target layout"))?;
    for channel in channels {
        for (id, output) in channel.controls() {
            // Extra ordinary or twin-stick actions may use the same frontend
            // trigger as a proportional field. Keep one measured binding for
            // both, irrespective of the action's diagram position/number.
            profile.bindings.retain(|control, existing| {
                !(matches!(*output, "LeftTrigger" | "RightTrigger")
                    && existing.as_str() == *output
                    && target
                        .controls
                        .iter()
                        .any(|item| item.id == *control && !item.analog))
            });
            ensure!(
                target
                    .controls
                    .iter()
                    .any(|control| control.id == *id && control.analog),
                "Combined MAME target is missing an analog control"
            );
            ensure!(
                !profile.bindings.values().any(|existing| existing == output),
                "Digital and analog mappings compete for the same frontend output"
            );
            ensure!(
                profile
                    .bindings
                    .insert((*id).to_owned(), (*output).to_owned())
                    .is_none(),
                "Digital and analog target control IDs collide"
            );
        }
    }
    profile.id.push_str("-analog");
    profile.name.push_str(" + explicit analog channels");
    profile.conditions.push("Requires combined physical calibration, fresh native field inspection and an owned normalized transport; channel selection alone is insufficient.".to_owned());
    Ok(Some(profile))
}

/// Validate this player's analog channels in the actual combined mapping plan.
/// Distinct outputs cannot share axes; repeated uses of one output are shared.
pub(crate) fn validate_analog_plan(
    assignments: &[AnalogAssignment],
    player: usize,
    calibration: &crate::controller_catalog::Calibration,
    plan: &crate::controller_catalog::MappingPlan,
) -> Result<()> {
    use anyhow::Context;
    let channels: BTreeSet<_> = assignments
        .iter()
        .filter(|assignment| assignment.source_player == player)
        .map(|assignment| assignment.channel)
        .collect();
    if channels.is_empty() {
        return Ok(());
    }
    calibration.validate()?;
    ensure!(
        calibration.os == "linux",
        "MAME analog measurements currently require the Linux native input contract"
    );
    let mut physical_axes = BTreeSet::new();
    for channel in channels {
        let mut measurements = Vec::new();
        let mut native_code = None;
        for (id, output) in channel.controls() {
            let mut rows = plan.rows.iter().filter(|row| row.target_id == *id);
            let row = rows.next().context("Missing analog mapping row")?;
            ensure!(
                rows.next().is_none() && row.output == *output,
                "Ambiguous or incorrect analog output row"
            );
            let input = row
                .input
                .as_ref()
                .context("MAME analog channel is unmapped")?;
            let physical = row
                .physical_id
                .as_ref()
                .context("Missing analog physical control ID")?;
            ensure!(
                calibration.bindings.get(physical) == Some(input),
                "Analog row does not match the selected saved calibration"
            );
            let native = input
                .native
                .as_ref()
                .context("MAME analog channel lacks native identity")?;
            let measured = input
                .axis
                .as_ref()
                .context("MAME analog channel lacks measured travel")?;
            ensure!(
                native.code >> 16 == 3 && native.direction != 0,
                "MAME analog channel requires a physical absolute axis, not a button"
            );
            if let Some(code) = native_code {
                ensure!(
                    code == native.code,
                    "Opposite analog directions use different physical axes"
                );
            }
            native_code = Some(native.code);
            measurements.push(measured);
        }
        ensure!(
            physical_axes.insert(native_code.context("Missing analog channel identity")?),
            "Distinct MAME analog channels share one physical axis"
        );
        if measurements.len() == 2 {
            crate::controller_axis::BipolarAxis::from_measurements(
                measurements[0],
                measurements[1],
            )?;
        } else {
            crate::controller_axis::PressureAxis::from_measurement(measurements[0])?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StickRange {
    #[default]
    Full,
    Reversed,
    Positive,
    Negative,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AnalogAssignment {
    pub field: ActiveField,
    /// One-based libretro source port, translated through observed native IDs.
    pub source_player: usize,
    pub channel: AnalogChannel,
    /// Applied after measured stick normalization, not to pressure channels.
    #[serde(default)]
    pub stick_range: StickRange,
    /// Explicitly opt into MAME's absolute-stick-as-relative-velocity behavior.
    /// This is not physical mouse/trackball delta capture.
    #[serde(default)]
    pub relative_velocity: bool,
    /// Explicit controller-driven absolute aim, not physical lightgun capture.
    #[serde(default)]
    pub controller_aim: bool,
}

/// Pure transport preparation: select the combined profile's physical inputs,
/// retain opposite travel observations, and translate calibration into the
/// normalized virtual device's units. The launch owner must validate live
/// source bounds and use this calibration only with the corresponding frame.
pub(crate) fn normalized_calibration(
    assignments: &[AnalogAssignment],
    player: usize,
    calibration: &crate::controller_catalog::Calibration,
    profile: &crate::controller_catalog::EmulatorProfile,
) -> Result<(
    crate::controller_axis::GamepadFrame,
    crate::controller_catalog::Calibration,
    BTreeSet<u16>,
)> {
    use anyhow::Context;
    let plan = calibration.plan_profile(profile)?;
    validate_analog_plan(assignments, player, calibration, &plan)?;
    let mut required = BTreeSet::new();
    let mut selected_axes = BTreeSet::new();
    let mut pressure_codes = BTreeSet::new();
    let target = crate::controller_catalog::catalog()
        .layout(&profile.target_layout)
        .context("Missing MAME target layout")?;
    for row in &plan.rows {
        let id = row
            .physical_id
            .as_ref()
            .context("Combined MAME control is unmapped")?;
        let input = row
            .input
            .as_ref()
            .context("Combined MAME input is missing")?;
        ensure!(
            calibration.bindings.get(id) == Some(input),
            "Combined MAME row differs from saved calibration"
        );
        let native = input
            .native
            .as_ref()
            .context("Combined MAME input lacks physical identity")?;
        required.insert(id.clone());
        if native.code >> 16 == 3 {
            selected_axes.insert(native.code);
        }
        let control = target
            .controls
            .iter()
            .find(|control| control.id == row.target_id)
            .context("Unknown combined MAME control")?;
        if control.is_pressure() {
            pressure_codes.insert((native.code & 0xffff) as u16);
        }
    }
    let mut physical = calibration.clone();
    physical.bindings.retain(|id, input| {
        required.contains(id)
            || input
                .native
                .as_ref()
                .is_some_and(|native| selected_axes.contains(&native.code))
    });
    let (frame, mut translated) =
        crate::controller_axis::normalized_gamepad_calibration(&physical)?;
    translated.bindings.retain(|id, _| required.contains(id));
    translated.validate()?;
    // Re-resolve in the virtual units and ensure the same physical control IDs
    // still supply every requested output. Normalization must not remap buttons.
    let normalized = translated.plan_profile(profile)?;
    ensure!(
        plan.rows.len() == normalized.rows.len()
            && plan
                .rows
                .iter()
                .zip(&normalized.rows)
                .all(|(before, after)| before.target_id == after.target_id
                    && before.output == after.output
                    && before.physical_id == after.physical_id),
        "Normalization changed the combined MAME assignment"
    );
    validate_analog_plan(assignments, player, &translated, &normalized)?;
    Ok((frame, translated, pressure_codes))
}

/// Build machine-specific native assignments for explicit absolute fields.
/// The caller must prove calibrated physical capabilities and merge these with
/// digital assignments and saved native overrides before enabling a launch.
/// In particular, trigger tokens alone do not prove proportional pressure:
/// the pinned core falls back to a digital trigger when its analog value is 0.
pub(crate) fn controller_xml(
    snapshot: &ActiveFieldSnapshot,
    assignments: &[AnalogAssignment],
) -> Result<String> {
    snapshot.validate(&snapshot.machine)?;
    ensure!(
        snapshot.joystick_enabled,
        "MAME native joystick input is disabled"
    );
    ensure!(
        !assignments.is_empty() && assignments.len() <= 32768,
        "Invalid analog assignment count"
    );
    let routes = snapshot.joystick_routes()?;
    let mut identities = BTreeSet::new();
    let mut rows = Vec::new();
    for assignment in assignments {
        let field = &assignment.field;
        ensure!(
            field.analog
                && field.class == FieldClass::Controller
                && snapshot.fields.iter().any(|observed| observed == field),
            "Analog assignment does not match an active native controller field"
        );
        let (player, kind) = field
            .input_type
            .split_once('_')
            .ok_or_else(|| anyhow::anyhow!("Missing analog field player"))?;
        let player = player
            .strip_prefix('P')
            .ok_or_else(|| anyhow::anyhow!("Invalid analog field player"))?;
        ensure!(
            matches!(player, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8"),
            "Unsupported analog field player"
        );
        let absolute = matches!(
            kind,
            "AD_STICK_X"
                | "AD_STICK_Y"
                | "AD_STICK_Z"
                | "PADDLE"
                | "PADDLE_V"
                | "POSITIONAL"
                | "POSITIONAL_V"
                | "PEDAL"
                | "PEDAL2"
                | "PEDAL3"
        );
        let relative = matches!(
            kind,
            "DIAL" | "DIAL_V" | "TRACKBALL_X" | "TRACKBALL_Y" | "MOUSE_X" | "MOUSE_Y"
        );
        let lightgun = matches!(kind, "LIGHTGUN_X" | "LIGHTGUN_Y");
        ensure!(
            (absolute && !assignment.relative_velocity && !assignment.controller_aim)
                || (relative && assignment.relative_velocity && !assignment.controller_aim)
                || (lightgun && assignment.controller_aim && !assignment.relative_velocity),
            "Native field requires the matching explicit absolute, stick-velocity or controller-aim mode"
        );
        if assignment.relative_velocity || assignment.controller_aim {
            ensure!(
                !matches!(
                    assignment.channel,
                    AnalogChannel::LeftPressure | AnalogChannel::RightPressure
                ) && matches!(
                    assignment.stick_range,
                    StickRange::Full | StickRange::Reversed
                ),
                "Stick velocity and controller aim require a full/reversed bipolar stick axis"
            );
        }
        ensure!(
            (1..=8).contains(&assignment.source_player),
            "Invalid analog source port"
        );
        let identity = (&field.tag, field.mask, &field.input_type, field.defvalue);
        ensure!(
            identities.insert(identity),
            "Duplicate native analog field assignment"
        );
        let tag = quick_xml::escape::escape(&field.tag);
        let joystick = routes[assignment.source_player - 1];
        let pressure = matches!(
            assignment.channel,
            AnalogChannel::LeftPressure | AnalogChannel::RightPressure
        );
        ensure!(
            !pressure || assignment.stick_range == StickRange::Full,
            "Pressure channels use their fixed native range; stick modifiers do not apply"
        );
        let modifier = match assignment.stick_range {
            StickRange::Full => "",
            StickRange::Reversed => "_REVERSE",
            StickRange::Positive => "_POS",
            StickRange::Negative => "_NEG",
        };
        let item = format!("{}{modifier}", assignment.channel.item());
        rows.push((identity, format!(
            "  <port tag=\"{tag}\" type=\"{}\" mask=\"{}\" defvalue=\"{}\"><newseq type=\"standard\">JOYCODE_{joystick}_{item}</newseq><newseq type=\"increment\">NONE</newseq><newseq type=\"decrement\">NONE</newseq></port>\n",
            field.input_type, field.mask, field.defvalue)));
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    let mut xml = format!(
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n<system name=\"{}\">\n<input>\n",
        snapshot.machine
    );
    for (_, row) in rows {
        xml.push_str(&row);
    }
    xml.push_str("</input>\n</system>\n</mameconfig>\n");
    Ok(xml)
}
