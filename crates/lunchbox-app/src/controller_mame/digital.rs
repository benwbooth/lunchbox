//! Explicit switch overrides for exact inspected native fields.
//! This planner does not select physical controls or open input devices.
use super::{ActiveField, ActiveFieldSnapshot, AnalogAssignment, DigitalFieldPlan};
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DigitalAssignment {
    pub field: ActiveField,
    /// One-based frontend port, translated through the observed native route.
    pub source_player: usize,
    /// Exact frontend output from explicit_switch_channels, not a raw sequence.
    pub output: String,
    #[serde(default)]
    pub sequence: SwitchSequence,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SwitchSequence {
    #[default]
    Standard,
    Increment,
    Decrement,
}

impl SwitchSequence {
    fn token(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Increment => "increment",
            Self::Decrement => "decrement",
        }
    }
}

pub(crate) struct ResolvedSwitchRoute {
    pub assignment: DigitalAssignment,
    pub explicit: bool,
}

type FieldKey<'a> = (&'a str, u32, &'a str, u32);

fn field_key(field: &ActiveField) -> FieldKey<'_> {
    (&field.tag, field.mask, &field.input_type, field.defvalue)
}

/// An identity lookup narrows candidates, but never substitutes for complete
/// field equality. Preserve input order for multiple sequences on one field.
fn assignment_index(
    assignments: &[DigitalAssignment],
) -> BTreeMap<FieldKey<'_>, Vec<&DigitalAssignment>> {
    let mut index: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for assignment in assignments {
        index
            .entry(field_key(&assignment.field))
            .or_default()
            .push(assignment);
    }
    index
}

/// Default inference owns only fields without an explicit switch assignment.
/// This derived view is never substituted for the retained inspection evidence.
fn default_snapshot(
    snapshot: &ActiveFieldSnapshot,
    assignments: &[DigitalAssignment],
) -> ActiveFieldSnapshot {
    let mut defaults = snapshot.clone();
    let index = assignment_index(assignments);
    let keep = |field: &ActiveField| {
        !index.get(&field_key(field)).is_some_and(|candidates| {
            candidates
                .iter()
                .any(|assignment| assignment.field == *field)
        })
    };
    defaults.fields.retain(keep);
    defaults.field_labels.retain(|entry| keep(&entry.field));
    defaults.analog_states.retain(|state| keep(&state.field));
    if let Some(keyboard) = &mut defaults.keyboard_state {
        keyboard.field_owners.retain(|owner| keep(&owner.field));
    }
    defaults
}

/// Complete switch routing after overrides, including defaults that share the
/// same channel. Disabled and unsupported fields are not presented as mapped.
pub(crate) fn resolved_switch_routes(
    snapshot: &ActiveFieldSnapshot,
    players: &BTreeSet<usize>,
    analog: &[AnalogAssignment],
    assignments: &[DigitalAssignment],
) -> Result<Vec<ResolvedSwitchRoute>> {
    plan_explicit_fields(snapshot, None, None, players, analog, assignments)?;
    let routes = snapshot.joystick_routes()?;
    let defaults = default_snapshot(snapshot, assignments);
    let sequences = super::snapshot_digital_sequences(&defaults, players, &routes)?;
    let mut channels = std::collections::BTreeMap::new();
    for player in players {
        for (item, output) in super::explicit_switch_channels() {
            channels.insert(
                format!("JOYCODE_{}_{item}", routes[player - 1]),
                (*player, output),
            );
        }
    }
    let mut resolved = Vec::new();
    let index = assignment_index(assignments);
    for field in &snapshot.fields {
        let explicit: Vec<_> = index
            .get(&field_key(field))
            .into_iter()
            .flatten()
            .copied()
            .filter(|assignment| assignment.field == *field)
            .collect();
        if !explicit.is_empty() {
            resolved.extend(explicit.into_iter().map(|assignment| ResolvedSwitchRoute {
                assignment: assignment.clone(),
                explicit: true,
            }));
            continue;
        }
        if field.analog
            || !matches!(
                field.class,
                super::inspection::FieldClass::Controller | super::inspection::FieldClass::Misc
            )
        {
            continue;
        }
        if let Some((player, output)) = sequences
            .get(&field.input_type)
            .and_then(|sequence| channels.get(sequence))
        {
            resolved.push(ResolvedSwitchRoute {
                assignment: DigitalAssignment {
                    field: field.clone(),
                    source_player: *player,
                    output: (*output).to_owned(),
                    sequence: SwitchSequence::Standard,
                },
                explicit: false,
            });
        }
    }
    resolved.sort_by(|left, right| {
        let a = &left.assignment.field;
        let b = &right.assignment.field;
        (&a.tag, a.mask, &a.input_type, a.defvalue).cmp(&(
            &b.tag,
            b.mask,
            &b.input_type,
            b.defvalue,
        ))
    });
    Ok(resolved)
}

/// Check composed destinations against the complete resolved output contract.
fn validate_profile_outputs(
    profile: Option<&crate::controller_catalog::EmulatorProfile>,
    switches: &BTreeSet<String>,
    analog: &[AnalogAssignment],
    player: usize,
) -> Result<()> {
    let mut expected = switches.clone();
    for assignment in analog
        .iter()
        .filter(|assignment| assignment.source_player == player)
    {
        expected.extend(
            assignment
                .channel
                .controls()
                .iter()
                .map(|(_, output)| (*output).to_owned()),
        );
    }
    let Some(profile) = profile else {
        ensure!(
            expected.is_empty(),
            "MAME profile is absent despite required frontend outputs"
        );
        return Ok(());
    };
    let layout = crate::controller_catalog::catalog()
        .layout(&profile.target_layout)
        .context("MAME composed profile layout is unavailable")?;
    let mut actual = BTreeSet::new();
    for (control, output) in &profile.bindings {
        ensure!(
            layout
                .controls
                .iter()
                .filter(|item| item.id == *control)
                .count()
                == 1,
            "MAME composed destination control {control} is missing or ambiguous"
        );
        ensure!(
            actual.insert(output.clone()),
            "MAME composed output {output} has multiple destination controls"
        );
    }
    ensure!(
        actual == expected,
        "MAME composed profile output mismatch: missing {:?}; unexpected {:?}",
        expected.difference(&actual).collect::<Vec<_>>(),
        actual.difference(&expected).collect::<Vec<_>>()
    );
    Ok(())
}

/// Derive physical requirements from the same native routes as the override
/// planner, then check completeness of the resulting destination bindings.
pub(crate) fn combined_profile(
    snapshot: &ActiveFieldSnapshot,
    players: &BTreeSet<usize>,
    analog: &[AnalogAssignment],
    assignments: &[DigitalAssignment],
    player: usize,
    layout: super::DigitalLayout,
) -> Result<Option<crate::controller_catalog::EmulatorProfile>> {
    ensure!(players.contains(&player), "Profile port is not selected");
    let resolved = resolved_switch_routes(snapshot, players, analog, assignments)?;
    let needed: BTreeSet<_> = resolved
        .into_iter()
        .filter(|route| route.assignment.source_player == player)
        .map(|route| route.assignment.output)
        .collect();
    if assignments.is_empty() {
        let profile = super::combined_profile(snapshot, analog, player, layout)?;
        validate_profile_outputs(profile.as_ref(), &needed, analog, player)?;
        return Ok(profile);
    }
    let defaults = default_snapshot(snapshot, assignments);
    // The field planner above has validated these assignments. Keep shared
    // pressure in `needed` for route coverage, but let the analog composer own
    // its geometry just as it does when there are no explicit switch overrides.
    let pressure_outputs: BTreeSet<_> = analog
        .iter()
        .filter(|assignment| assignment.source_player == player)
        .filter_map(|assignment| match assignment.channel {
            super::AnalogChannel::LeftPressure => Some("LeftTrigger"),
            super::AnalogChannel::RightPressure => Some("RightTrigger"),
            _ => None,
        })
        .collect();
    let geometry_needed: BTreeSet<_> = needed
        .iter()
        .map(String::as_str)
        .filter(|output| !pressure_outputs.contains(output))
        .collect();
    let layout = if layout == super::DigitalLayout::Automatic
        && geometry_needed.iter().any(|output| {
            super::explicit_switch_channels()
                .skip(16)
                .any(|(_, channel)| *output == channel)
        }) {
        super::DigitalLayout::FixedChannels
    } else if layout == super::DigitalLayout::Automatic
        && !super::has_twin_stick(&defaults, player)
        && !geometry_needed.is_empty()
    {
        // Resolve ordinary geometry before composing analog requirements,
        // even when overrides removed every default digital binding.
        super::arcade::automatic_button_layout(geometry_needed.iter().copied())
    } else {
        layout
    };
    let mut profile = match super::combined_profile(&defaults, analog, player, layout)? {
        Some(profile) => profile,
        None if needed.is_empty() => {
            validate_profile_outputs(None, &needed, analog, player)?;
            return Ok(None);
        }
        None => {
            let target = match layout {
                // With every native switch overridden, the default profile
                // is empty. Automatic must still retain arcade geometry for
                // ordinary outputs. Extended channels and button capacity
                // have already selected FixedChannels/EightButton above.
                super::DigitalLayout::Automatic | super::DigitalLayout::SixButton => {
                    "arcade-six-button"
                }
                super::DigitalLayout::EightButton => "arcade-eight-button",
                super::DigitalLayout::NeoGeo => "neogeo",
                _ => "mame-fixed-digital",
            };
            serde_json::from_value(serde_json::json!({
                "id":format!("mame-{}-player-{player}", snapshot.machine),
                "name":format!("MAME {} player {player}", snapshot.machine),
                "core":"mame", "target_layout":target, "transport":"retroarch",
                "status":"runtime-inspected", "source":"MAME native active-field snapshot",
                "conditions":[], "bindings":{}, "frontend_ports":8,
                "requires_fresh_start":true, "explicit_selection":true,
                "core_options":super::digital_options(),
                "retroarch_launch":{"platforms":[],"device":1,"max_players":8}
            }))?
        }
    };
    // Keep analog requirements intact. Remove obsolete digital requirements
    // only when no remaining native field uses their frontend channel.
    profile.bindings.retain(|_, output| {
        !super::DIGITAL_CHANNELS
            .iter()
            .any(|(_, digital)| *digital == output.as_str())
            || needed.contains(output)
    });
    let directional_switches = needed.iter().any(|output| {
        super::explicit_switch_channels()
            .skip(16)
            .any(|(_, channel)| output == channel)
    });
    if directional_switches {
        ensure!(
            matches!(
                profile.target_layout.as_str(),
                "mame-fixed-digital"
                    | "mame-twin-digital"
                    | "mame-fixed-digital-analog"
                    | "mame-twin-digital-analog"
            ),
            "Directional switches require Automatic or fixed-channel geometry"
        );
        profile.target_layout.push_str("-switches");
        profile.conditions.push("Explicit stick-direction switches share four bipolar frontend axes: opposing buttons cancel to neutral, and proportional assignments on the same axis share travel. These are not eight independent simultaneous action buttons. Native switch thresholds apply.".to_owned());
    }
    let catalog = crate::controller_catalog::catalog();
    let target = catalog
        .layout(&profile.target_layout)
        .context("Missing explicit MAME layout")?;
    let base = profile
        .target_layout
        .strip_suffix("-switches")
        .unwrap_or(&profile.target_layout);
    let base = base.strip_suffix("-analog").unwrap_or(base);
    // A trigger already represented by proportional pressure remains on its
    // measured analog control, even when a switch shares that output.
    let analog_outputs: BTreeSet<_> = profile
        .bindings
        .iter()
        .filter_map(|(id, output)| {
            target
                .controls
                .iter()
                .any(|control| control.id == *id && control.analog)
                .then_some(output.clone())
        })
        .collect();
    let positions = super::arcade::button_positions(
        base,
        needed
            .iter()
            .chain(profile.bindings.values())
            .filter(|output| !analog_outputs.contains(*output))
            .map(String::as_str),
    )?;
    if !positions.is_empty() {
        profile.conditions.push("Arcade button positions preserve occupied conventional channels first, then place sparse higher channels into unused positions. Review native action labels; the frontend wires are unchanged.".to_owned());
        // Recompute from the complete default-plus-explicit set so an earlier
        // sparse default never consumes a later explicit conventional position.
        profile.bindings = profile
            .bindings
            .into_iter()
            .map(|(control, output)| (positions.get(&output).cloned().unwrap_or(control), output))
            .collect();
    }
    let fixed = [
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
        "axis_lx_negative_switch",
        "axis_lx_positive_switch",
        "axis_ly_negative_switch",
        "axis_ly_positive_switch",
        "axis_rx_negative_switch",
        "axis_rx_positive_switch",
        "axis_ry_negative_switch",
        "axis_ry_positive_switch",
    ];
    for output in needed.iter().cloned() {
        if profile.bindings.values().any(|value| value == &output) {
            continue;
        }
        let index = super::explicit_switch_channels()
            .position(|(_, value)| value == output)
            .context("Unknown digital profile output")?;
        let control = if let Some(position) = positions.get(&output) {
            position.clone()
        } else if (4..12).contains(&index) {
            match base {
                "arcade-six-button" | "arcade-eight-button" | "neogeo" => positions
                    .get(&output)
                    .context("Missing explicit arcade button position")?
                    .clone(),
                "mame-twin-digital" if index >= 8 => {
                    // Native action numbers are not fixed frontend wire numbers
                    // in twin-stick mode. A surviving Button 6 may already use
                    // LeftBumper, leaving RightBumper free despite button6 being
                    // occupied in the diagram. Prefer an explicit native action
                    // number where available, then any free action position.
                    let mut candidates = BTreeSet::new();
                    for assignment in assignments.iter().filter(|assignment| {
                        assignment.source_player == player && assignment.output == output
                    }) {
                        if let Some((_, number)) =
                            assignment.field.input_type.rsplit_once("_BUTTON")
                            && let Ok(number) = number.parse::<usize>()
                            && (1..=16).contains(&number)
                        {
                            candidates.insert(format!("button{number}"));
                        }
                    }
                    let control = candidates
                        .into_iter()
                        .chain(std::iter::once(format!("button{}", index - 3)))
                        .chain((1..=16).map(|number| format!("button{number}")))
                        .find(|candidate| {
                            !profile.bindings.contains_key(candidate)
                                && target
                                    .controls
                                    .iter()
                                    .any(|item| item.id == *candidate && !item.analog)
                        })
                        .context(
                            "Twin-stick diagram has no free action position for this output",
                        )?;
                    profile.conditions.push(format!(
                        "Explicit {output} uses diagram position {control}; native action names/numbers are shown by the resolved field routes."
                    ));
                    control
                }
                "mame-twin-digital" => {
                    ["right_down", "right_right", "right_left", "right_up"][index - 4].to_string()
                }
                _ => fixed[index].to_owned(),
            }
        } else {
            fixed[index].to_owned()
        };
        ensure!(
            target
                .controls
                .iter()
                .any(|item| item.id == control && !item.analog),
            "Selected MAME layout cannot represent explicit output {output}; choose a larger or fixed-channel layout"
        );
        ensure!(
            !profile.bindings.contains_key(&control),
            "Explicit digital output conflicts with an existing target control"
        );
        profile.bindings.insert(control, output);
    }
    validate_profile_outputs(Some(&profile), &needed, analog, player)?;
    if profile.bindings.is_empty() {
        return Ok(None);
    }
    profile.id.push_str("-explicit-digital");
    profile.name.push_str(" + explicit switch assignments");
    if assignments
        .iter()
        .any(|assignment| assignment.source_player == player && assignment.field.analog)
    {
        profile.conditions.push("Button-driven analog fields use native keydelta/centering/wrapping/sensitivity. These are incremental controls, not measured analog travel or physical relative-device capture.".to_owned());
    }
    profile.conditions.push("Explicit switches may share a frontend channel with other native fields; review all routes and calibrate the resulting combined profile before launch.".to_owned());
    if assignments.iter().any(|assignment| {
        assignment.source_player == player
            && matches!(assignment.output.as_str(), "LeftTrigger" | "RightTrigger")
    }) {
        profile.conditions.push("L2/R2 switch assignments use the native negative-axis switch threshold, not numbered buttons. A channel also assigned to analog pressure shares the same physical travel; otherwise a digital source uses the wrapper's full-travel fallback.".to_owned());
    }
    Ok(Some(profile))
}

/// Compose explicit overrides after the ordinary digital/analog plan. Multiple
/// fields may intentionally share a channel; duplicate field/sequence pairs may not.
/// Launch integration must also add these outputs to the physical profile and
/// require calibration before using the resulting native configuration.
pub(crate) fn plan_explicit_fields(
    snapshot: &ActiveFieldSnapshot,
    original_controller: Option<&str>,
    original_game: Option<&str>,
    players: &BTreeSet<usize>,
    analog: &[AnalogAssignment],
    assignments: &[DigitalAssignment],
) -> Result<DigitalFieldPlan> {
    snapshot.validate(&snapshot.machine)?;
    ensure!(
        assignments.len() <= 32768,
        "Too many explicit digital assignments"
    );
    let routes = snapshot.joystick_routes()?;
    let mut identities = BTreeSet::new();
    let mut ordered: Vec<_> = assignments.iter().collect();
    ordered.sort_by_key(|assignment| {
        let field = &assignment.field;
        (
            &field.tag,
            field.mask,
            &field.input_type,
            field.defvalue,
            assignment.sequence,
        )
    });
    let mut xml = format!(
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n<system name=\"{}\">\n<input>\n",
        snapshot.machine
    );
    let mut field_sequences = std::collections::BTreeMap::new();
    for assignment in &ordered {
        let field = &assignment.field;
        ensure!(
            matches!(
                field.class,
                super::inspection::FieldClass::Controller
                    | super::inspection::FieldClass::Misc
                    | super::inspection::FieldClass::Keyboard
            ) && (field.analog == (assignment.sequence != SwitchSequence::Standard)),
            "Explicit digital assignments require a user-mappable switch field"
        );
        if field.analog {
            ensure!(
                field.class == super::inspection::FieldClass::Controller,
                "Button-driven analog input requires a controller field"
            );
            let kind = field
                .input_type
                .split_once('_')
                .map(|(_, kind)| kind)
                .unwrap_or("");
            ensure!(
                matches!(
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
                        | "DIAL"
                        | "DIAL_V"
                        | "TRACKBALL_X"
                        | "TRACKBALL_Y"
                        | "MOUSE_X"
                        | "MOUSE_Y"
                        | "LIGHTGUN_X"
                        | "LIGHTGUN_Y"
                ),
                "Unsupported button-driven analog field"
            );
            ensure!(
                !analog.iter().any(|axis| axis.field == *field),
                "A native field cannot use both an absolute channel assignment and button-driven control"
            );
        }
        if field.class == super::inspection::FieldClass::Keyboard {
            let keyboard = snapshot.keyboard_state.as_ref().context(
                "Keyboard switch assignment needs fresh native keyboard-state inspection",
            )?;
            let owner = keyboard
                .field_owners
                .iter()
                .find(|owner| owner.field == *field)
                .context(
                    "Keyboard switch assignment lacks exact native device ownership; reinspect",
                )?;
            let device = keyboard
                .devices
                .iter()
                .find(|device| device.tag == owner.device_tag)
                .context("Inspected keyboard device is missing")?;
            ensure!(device.enabled, "Native keyboard/keypad device is disabled");
            ensure!(
                matches!(field.input_type.as_str(), "KEYBOARD" | "KEYPAD"),
                "Unsupported native keyboard token"
            );
            ensure!(
                field.input_type == "KEYPAD" || !keyboard.natural_in_use,
                "Natural keyboard mode locks out keyboard switch sequences; select emulated keyboard mode and reinspect"
            );
        }
        ensure!(
            snapshot.fields.contains(field),
            "Digital field is not in the current inspection"
        );
        ensure!(
            (1..=8).contains(&assignment.source_player)
                && players.contains(&assignment.source_player),
            "Digital source port must be an explicitly selected frontend port"
        );
        ensure!(
            identities.insert((
                &field.tag,
                field.mask,
                &field.input_type,
                field.defvalue,
                assignment.sequence
            )),
            "Duplicate explicit digital field/sequence assignment"
        );
        let (item, _) = super::explicit_switch_channels()
            .find(|(_, output)| *output == assignment.output)
            .context("Digital output is not a supported native switch channel")?;
        let joystick = routes[assignment.source_player - 1];
        field_sequences
            .entry((&field.tag, field.mask, &field.input_type, field.defvalue))
            .or_insert_with(std::collections::BTreeMap::new)
            .insert(assignment.sequence, format!("JOYCODE_{joystick}_{item}"));
    }
    for ((tag, mask, input_type, defvalue), sequences) in &field_sequences {
        let tag = quick_xml::escape::escape(tag.as_str());
        xml.push_str(&format!(
            "<port tag=\"{tag}\" type=\"{input_type}\" mask=\"{mask}\" defvalue=\"{defvalue}\">"
        ));
        // For button-driven axes, clear standard and any unassigned direction
        // so inherited host-axis/key defaults cannot fight the chosen buttons.
        let kinds: &[SwitchSequence] = if sequences.contains_key(&SwitchSequence::Standard) {
            &[SwitchSequence::Standard]
        } else {
            &[
                SwitchSequence::Standard,
                SwitchSequence::Increment,
                SwitchSequence::Decrement,
            ]
        };
        for kind in kinds {
            let value = sequences.get(kind).map(String::as_str).unwrap_or("NONE");
            xml.push_str(&format!(
                "<newseq type=\"{}\">{value}</newseq>",
                kind.token()
            ));
        }
        xml.push_str("</port>\n");
    }
    xml.push_str("</input>\n</system>\n</mameconfig>\n");
    // Validate explicit fields against the original above, then allocate
    // default channels only for the remainder. Extra twin-stick buttons and
    // third direction clusters can now be fully explicitly routed.
    let defaults = default_snapshot(snapshot, assignments);
    let mut plan = if analog.is_empty() {
        super::plan_digital_layer(&defaults, original_controller, original_game, players)?
    } else {
        super::plan_mixed_fields(
            &defaults,
            original_controller,
            original_game,
            players,
            analog,
        )?
    };
    if assignments.is_empty() {
        return Ok(plan);
    }
    plan.controller_xml =
        super::preserve_controller_configuration(Some(&plan.controller_xml), &xml)?;
    let fields: Vec<_> = ordered.iter().map(|assignment| &assignment.field).collect();
    plan.game_config_xml = super::analog::remove_saved_field_sequences(
        &plan.game_config_xml,
        &snapshot.machine,
        &fields,
    )?;
    // Explicit fields were excluded from all default counters.
    plan.mapped_fields += field_sequences.len();
    Ok(plan)
}
