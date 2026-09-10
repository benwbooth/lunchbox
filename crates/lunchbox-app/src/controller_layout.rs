//! Deterministic layout-to-layout assignment, independent of emulator numbering.
//! One capability-constrained assignment problem for every source/target pair.
//! Family rules describe ergonomics, never emulator numbering or device models.
//! The solver maximizes required coverage, then optional coverage, then minimizes
//! preference/geometry cost. Missing hardware is not synthesized.
use crate::controller_catalog::{Control, Layout};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const POLICY_VERSION: u32 = 7;

/// Equivalent pressure roles; digital fallback buttons are not aliases.
pub(crate) fn pressure_role(id: &str) -> Option<&'static str> {
    match id {
        "l2" | "trigger_left" => Some("left"),
        "r2" | "trigger_right" => Some("right"),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    Identity,
    FamilyPreference,
    FacePosition,
    SameHandShoulder,
    StickToDigital,
    DirectionalCluster,
    DigitalOverflow,
    UserChoice,
}

impl Rule {
    pub fn description(self) -> &'static str {
        match self {
            Self::UserChoice => "Your saved choice",
            Self::Identity => "Same semantic control",
            Self::FamilyPreference => "Shared layout-family ergonomic rule",
            Self::FacePosition => "Available face button chosen by global position matching",
            Self::SameHandShoulder => "Available shoulder/trigger on the same hand",
            Self::StickToDigital => "Analog stick direction supplies a digital C button",
            Self::DirectionalCluster => "Same direction in the corresponding directional cluster",
            Self::DigitalOverflow => {
                "Spare gameplay button supplies an auxiliary digital control; review this fallback"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Missing {
    IncompatibleHardware,
    NotCalibrated,
    InsufficientDistinctInputs,
}

impl Missing {
    pub fn description(self) -> &'static str {
        match self {
            Self::IncompatibleHardware => {
                "No source control satisfies this target's semantic/capability rules"
            }
            Self::NotCalibrated => "Compatible source controls have not been calibrated",
            Self::InsufficientDistinctInputs => {
                "Compatible inputs are needed by other target controls"
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Resolution {
    pub policy_version: u32,
    pub assignments: BTreeMap<String, String>,
    pub rules: BTreeMap<String, Rule>,
    pub missing: BTreeMap<String, Missing>,
}

fn preferred<'a>(source: &Layout, target: &Layout, id: &'a str) -> &'a str {
    if target.family == "arcade-rows" {
        let slot = match id {
            "button1" => Some(0),
            "button2" => Some(1),
            "button3" => Some(2),
            "button4" => Some(3),
            "button5" => Some(4),
            "button6" => Some(5),
            "button7" => Some(6),
            "button8" => Some(7),
            _ => None,
        };
        if let Some(slot) = slot {
            return match source.family.as_str() {
                "diamond" | "horizontal-four" => ["y", "x", "r", "b", "a", "l", "l2", "r2"][slot],
                "six-button" | "three-button" => ["x", "y", "z", "a", "b", "c", "l", "r"][slot],
                "four-button-row" => [
                    "a", "b", "c", "d", "button5", "button6", "button7", "button8",
                ][slot],
                _ => id,
            };
        }
    }
    if source.family == "arcade-rows" && target.family == "six-button" {
        return match id {
            "x" => "button1",
            "y" => "button2",
            "z" => "button3",
            "a" => "button4",
            "b" => "button5",
            "c" => "button6",
            "l" => "button7",
            "r" => "button8",
            other => other,
        };
    }
    if target.family == "four-button-row" {
        let slot = match id {
            "a" => Some(0),
            "b" => Some(1),
            "c" => Some(2),
            "d" => Some(3),
            _ => None,
        };
        if let Some(slot) = slot {
            return match source.family.as_str() {
                "arcade-rows" => [
                    "button4",
                    "button5",
                    "button6",
                    if source.controls.iter().any(|c| c.id == "button8") {
                        "button8"
                    } else {
                        "button3"
                    },
                ][slot],
                "diamond" | "horizontal-four" => ["y", "b", "a", "x"][slot],
                "n64" => ["b", "a", "c_down", "c_right"][slot],
                "three-button" | "six-button" => ["a", "b", "c", "y"][slot],
                _ => id,
            };
        }
    }
    if source.family == "four-button-row"
        && matches!(target.family.as_str(), "diamond" | "horizontal-four")
    {
        return match id {
            "y" => "a",
            "b" => "b",
            "a" => "c",
            "x" => "d",
            other => other,
        };
    }
    if source.family == "diamond" && target.family == "n64" {
        return match id {
            "a" => "b",
            "b" => "y",
            "z" => "l2",
            "z_right" => "r2",
            "c_up" => "right_stick_up",
            "c_down" => "right_stick_down",
            "c_left" => "right_stick_left",
            "c_right" => "right_stick_right",
            other => other,
        };
    }
    if source.family == "diamond" && target.family == "two-button" {
        return match id {
            "b" => "y",
            "a" => "b",
            other => other,
        };
    }
    if source.family == "n64" && target.family == "diamond" {
        // Keep the N64 B/A thumb pair as the SNES Y/B run/jump pair.
        // The adjacent C-left/C-down pair supplies X/A, without crossing hands.
        return match id {
            "b" => "a",
            "y" => "b",
            "a" => "c_down",
            "x" => "c_left",
            "l2" => "z",
            "r2" => "z_right",
            other => other,
        };
    }
    if source.family == "n64" && matches!(target.family.as_str(), "six-button" | "three-button") {
        return match id {
            "a" => "a",
            "b" => "c_down",
            "c" => "c_right",
            "x" => "b",
            "y" => "c_left",
            "z" => "c_up",
            "mode" => "select",
            other => other,
        };
    }
    if matches!(source.family.as_str(), "diamond" | "horizontal-four")
        && matches!(target.family.as_str(), "three-button" | "six-button")
    {
        // A/B/C traverse west/south/east. The upper row uses left shoulder,
        // north, right shoulder; this rule is independent of any core's wiring.
        return match id {
            "a" => "y",
            "b" => "b",
            "c" => "a",
            "x" => "l",
            "y" => "x",
            "z" => "r",
            "mode" => "select",
            other => other,
        };
    }
    if matches!(source.family.as_str(), "three-button" | "six-button")
        && matches!(target.family.as_str(), "diamond" | "horizontal-four")
    {
        return match id {
            "y" => "a",
            "b" => "b",
            "a" => "c",
            "x" => "y",
            "select" => "mode",
            other => other,
        };
    }
    if source.family == "six-button" && target.family == "n64" {
        return match id {
            "a" => "a",
            "b" => "x",
            "c_down" => "b",
            "c_left" => "y",
            "c_right" => "c",
            "c_up" => "z",
            other => other,
        };
    }
    // Mode and Select are auxiliary menu roles, not Start or Home. The source
    // layout's own control keeps its role; this only bridges a family that
    // spells the role differently, and must not tie with identity mappings.
    if id == "mode"
        && !source.controls.iter().any(|c| c.id == "mode")
        && source.controls.iter().any(|c| c.id == "select")
    {
        return "select";
    }
    if id == "select"
        && !source.controls.iter().any(|c| c.id == "select")
        && source.controls.iter().any(|c| c.id == "mode")
    {
        return "mode";
    }
    id
}

fn normalized(control: &Control, group: &[&Control]) -> (f64, f64) {
    let min_x = group.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
    let max_x = group.iter().map(|c| c.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = group.iter().map(|c| c.y).fold(f64::INFINITY, f64::min);
    let max_y = group.iter().map(|c| c.y).fold(f64::NEG_INFINITY, f64::max);
    (
        (control.x - min_x) / (max_x - min_x).max(1.0),
        (control.y - min_y) / (max_y - min_y).max(1.0),
    )
}

fn shoulder_hand(id: &str) -> Option<bool> {
    match id {
        "l" | "l2" => Some(false),
        "r" | "r2" => Some(true),
        _ => None,
    }
}

/// A two-button thumb pair is an arrangement, not a manufacturer's A/B spelling.
/// For example, the Neo Geo Pocket's left A/right B must not cross the user's
/// comfortable physical pair merely because NES labels run in the other order.
fn primary_pair_input<'a>(source: &'a Layout, target: &Layout, to: &Control) -> Option<&'a str> {
    if !matches!(target.family.as_str(), "two-button" | "dual-direction") || to.group != "face" {
        return None;
    }
    let face_pair = |layout: &'a Layout| {
        let mut faces: Vec<_> = layout
            .controls
            .iter()
            .filter(|control| {
                control.group == "face" && !control.analog && control.repeat_of.is_none()
            })
            .collect();
        faces.sort_by(|a, b| {
            a.x.total_cmp(&b.x)
                .then(a.y.total_cmp(&b.y))
                .then(a.id.cmp(&b.id))
        });
        faces
    };
    let mut targets: Vec<_> = target
        .controls
        .iter()
        .filter(|control| control.group == "face" && !control.analog && control.repeat_of.is_none())
        .collect();
    targets.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.id.cmp(&b.id))
    });
    if targets.len() != 2 {
        return None;
    }
    let index = targets.iter().position(|control| control.id == to.id)?;
    match source.family.as_str() {
        "diamond" => Some(["y", "b"][index]),
        "n64" | "horizontal-four" => Some(["b", "a"][index]),
        "three-button" | "six-button" | "four-button-row" => Some(["a", "b"][index]),
        "two-button" | "dual-direction" => {
            let faces = face_pair(source);
            (faces.len() == 2).then(|| faces[index].id.as_str())
        }
        _ => None,
    }
}

/// A directional cluster retains direction and side when it moves between
/// digital pads, a stick, or N64 C-buttons. Ordinary face buttons/shoulders are
/// not assumed to form a second D-pad merely because four of them are present.
fn directional_cluster<'a>(layout: &Layout, control: &'a Control) -> Option<(bool, &'a str)> {
    let id = control.id.as_str();
    let (right, direction) = match control.group.as_str() {
        "dpad" => {
            if let Some(direction) = id.strip_prefix("right_") {
                (true, direction)
            } else {
                (false, id.strip_prefix("left_").unwrap_or(id))
            }
        }
        "stick" if control.analog => {
            if let Some(direction) = id.strip_prefix("right_stick_") {
                (true, direction)
            } else {
                (
                    false,
                    id.strip_prefix("left_stick_")
                        .or_else(|| id.strip_prefix("stick_"))?,
                )
            }
        }
        "face" if layout.family == "n64" => (true, id.strip_prefix("c_")?),
        _ => return None,
    };
    matches!(direction, "up" | "down" | "left" | "right").then_some((right, direction))
}

/// An edge is allowed only by a semantic/capability rule; geometry cannot make
/// Start into a face button, reverse an axis, or create an analog capability.
fn candidate(
    source: &Layout,
    target: &Layout,
    from: &Control,
    to: &Control,
) -> Option<(i64, Rule)> {
    if to.analog && !from.analog {
        return None;
    }
    if to.analog && from.is_pressure() != to.is_pressure() {
        return None;
    }
    if from.repeat_of.is_some()
        || to.repeat_of.is_some()
        || from.group == "turbo"
        || to.group == "turbo"
    {
        return (from.id == to.id && from.group == to.group && from.repeat_of == to.repeat_of)
            .then_some((0, Rule::Identity));
    }
    let wanted = preferred(source, target, &to.id);
    if !to.analog
        && let Some(target_direction) = directional_cluster(target, to)
        && directional_cluster(source, from) == Some(target_direction)
    {
        let c_button = target.family == "n64" && to.id.starts_with("c_");
        let rule = if from.analog && c_button {
            Rule::StickToDigital
        } else if from.id == to.id && from.group == to.group && !from.analog {
            Rule::Identity
        } else {
            Rule::DirectionalCluster
        };
        return Some((if from.analog && !c_button { 2 } else { 0 }, rule));
    }
    if target.family == "n64"
        && to.id.starts_with("c_")
        && from.id == format!("right_stick_{}", &to.id[2..])
        && from.group == "stick"
        && from.analog
    {
        return Some((0, Rule::StickToDigital));
    }
    // The seventh/eighth arcade buttons are physical triggers on a modern pad,
    // irrespective of which frontend channels the emulator uses to carry them.
    // A measured trigger half can drive a digital button; never flatten sticks
    // or guess an unrecorded axis endpoint here.
    if target.family == "arcade-rows" && to.group == "face" && !to.analog {
        if from.group == "shoulder"
            && pressure_role(&from.id).is_some()
            && pressure_role(&from.id) == pressure_role(wanted)
        {
            return Some((0, Rule::FamilyPreference));
        }
        if matches!(source.family.as_str(), "diamond" | "horizontal-four")
            && !from.analog
            // Stick clicks only; some layouts spell unrelated menu keys "l3"/"r3".
            && from.group == "stick"
            && matches!(
                (to.id.as_str(), from.id.as_str()),
                ("button7", "l3") | ("button8", "r3")
            )
        {
            return Some((1, Rule::DigitalOverflow));
        }
    }
    if from.analog || to.analog {
        if from.analog
            && to.analog
            && from.group == "shoulder"
            && to.group == "shoulder"
            && pressure_role(&from.id).is_some()
            && pressure_role(&from.id) == pressure_role(&to.id)
        {
            return Some((
                0,
                if from.id == to.id {
                    Rule::Identity
                } else {
                    Rule::SameHandShoulder
                },
            ));
        }
        return (from.id == to.id && from.group == to.group && from.analog == to.analog)
            .then_some((0, Rule::Identity));
    }
    if from.group == "face"
        && let Some(pair_input) = primary_pair_input(source, target, to)
    {
        return Some(if from.id == pair_input {
            (
                0,
                if from.id == to.id {
                    Rule::Identity
                } else {
                    Rule::FamilyPreference
                },
            )
        } else {
            (4, Rule::FacePosition)
        });
    }
    if from.id == wanted && from.group == to.group {
        return Some((
            0,
            if from.id == to.id {
                Rule::Identity
            } else {
                Rule::FamilyPreference
            },
        ));
    }
    if source.family == "diamond"
        && target.family == "n64"
        && from.group == "face"
        && to.group == "face"
        && matches!(
            (to.id.as_str(), from.id.as_str()),
            ("c_down", "a") | ("c_left", "x")
        )
    {
        return Some((1, Rule::FamilyPreference));
    }
    if target.family == "arcade-rows"
        && to.group == "face"
        && matches!(from.group.as_str(), "face" | "shoulder" | "stick")
        && from.id == wanted
    {
        return Some((0, Rule::FamilyPreference));
    }
    // Explicit shoulder-to-upper-row conversion (and its reverse); no arbitrary
    // cross-group nearest-neighbour fallback. Existing shoulders win by cost.
    if matches!(source.family.as_str(), "diamond" | "horizontal-four")
        && target.family == "six-button"
        && to.group == "face"
        && from.group == "shoulder"
        && from.id == wanted
    {
        return Some((0, Rule::FamilyPreference));
    }
    if source.family == "six-button"
        && matches!(
            target.family.as_str(),
            "diamond" | "horizontal-four" | "two-button"
        )
        && matches!(to.group.as_str(), "shoulder" | "rear")
        && from.group == "face"
        && matches!(
            (to.id.as_str(), from.id.as_str()),
            ("l" | "l2", "x") | ("r" | "r2", "z")
        )
    {
        return Some((2, Rule::FamilyPreference));
    }
    if matches!(from.group.as_str(), "shoulder" | "rear")
        && matches!(to.group.as_str(), "shoulder" | "rear")
        && shoulder_hand(&from.id).is_some()
        && shoulder_hand(&from.id) == shoulder_hand(&to.id)
    {
        return Some((2, Rule::SameHandShoulder));
    }
    if from.group == "face" && to.group == "face" {
        // Once a family rule changes a control's role, matching its printed ID
        // is not a semantic advantage. Otherwise two mediocre label matches can
        // displace the preferred primary button when calibration is incomplete.
        return Some((4, Rule::FacePosition));
    }
    // Last-resort digital conversion uses real, calibrated gameplay buttons.
    // A control's location is not a missing hardware capability. Directional
    // inputs and system/menu buttons remain reserved; they are not generic
    // button donors. Start/Select/Mode and stick clicks may RECEIVE a spare
    // gameplay button, but Home/system controls never enter this fallback.
    let gameplay_button = |control: &Control| {
        matches!(control.group.as_str(), "face" | "shoulder" | "rear")
            || (control.group == "stick" && matches!(control.id.as_str(), "l3" | "r3"))
    };
    if gameplay_button(from)
        && (gameplay_button(to)
            || to.group == "auxiliary"
            || (to.group == "menu" && matches!(to.id.as_str(), "start" | "select" | "mode")))
    {
        return Some((8, Rule::DigitalOverflow));
    }
    None
}

// Rectangular Hungarian assignment: rows are targets; columns are physical
// controls plus one unmatched slot per target. O(targets^2 * columns), without
// the old exponential face-count limit. Canonical ID order makes ties stable.
fn minimum_assignment(costs: &[Vec<i64>]) -> Vec<usize> {
    let n = costs.len();
    if n == 0 {
        return Vec::new();
    }
    let m = costs[0].len();
    assert!(n <= m && costs.iter().all(|row| row.len() == m));
    let (mut u, mut v) = (vec![0; n + 1], vec![0; m + 1]);
    let (mut owner, mut previous) = (vec![0; m + 1], vec![0; m + 1]);
    for row in 1..=n {
        owner[0] = row;
        let mut column = 0;
        let mut slack = vec![i64::MAX / 4; m + 1];
        let mut used = vec![false; m + 1];
        loop {
            used[column] = true;
            let current_row = owner[column];
            let (mut delta, mut next) = (i64::MAX / 4, 0);
            for c in 1..=m {
                if used[c] {
                    continue;
                }
                let reduced = costs[current_row - 1][c - 1] - u[current_row] - v[c];
                if reduced < slack[c] {
                    slack[c] = reduced;
                    previous[c] = column;
                }
                if slack[c] < delta {
                    delta = slack[c];
                    next = c;
                }
            }
            for c in 0..=m {
                if used[c] {
                    u[owner[c]] += delta;
                    v[c] -= delta;
                } else {
                    slack[c] -= delta;
                }
            }
            column = next;
            if owner[column] == 0 {
                break;
            }
        }
        loop {
            let next = previous[column];
            owner[column] = owner[next];
            column = next;
            if column == 0 {
                break;
            }
        }
    }
    let mut result = vec![0; n];
    for c in 1..=m {
        if owner[c] != 0 {
            result[owner[c] - 1] = c - 1;
        }
    }
    result
}

pub fn resolve_with_choices(
    source: &Layout,
    target: &Layout,
    available: &BTreeSet<&str>,
    requested: &BTreeSet<&str>,
    choices: &BTreeMap<String, String>,
) -> anyhow::Result<Resolution> {
    let mut used = BTreeSet::new();
    for (to, from) in choices {
        anyhow::ensure!(
            requested.contains(to.as_str()),
            "Unknown target control: {to}"
        );
        anyhow::ensure!(
            available.contains(from.as_str()),
            "Record {from} before assigning it"
        );
        anyhow::ensure!(
            used.insert(from.as_str()),
            "{from} is assigned more than once"
        );
        let physical = source.controls.iter().find(|c| c.id == *from);
        let destination = target.controls.iter().find(|c| c.id == *to);
        anyhow::ensure!(
            physical
                .zip(destination)
                .is_some_and(|(a, b)| candidate(source, target, a, b).is_some()),
            "{from} cannot supply {to}; choose a compatible control"
        );
    }
    let remaining_inputs = available.difference(&used).copied().collect();
    let remaining_targets = requested
        .iter()
        .copied()
        .filter(|id| !choices.contains_key(*id))
        .collect();
    let mut result = resolve(source, target, &remaining_inputs, &remaining_targets);
    for (to, from) in choices {
        result.assignments.insert(to.clone(), from.clone());
        result.rules.insert(to.clone(), Rule::UserChoice);
    }
    Ok(result)
}

pub fn resolve(
    source: &Layout,
    target: &Layout,
    available: &BTreeSet<&str>,
    requested: &BTreeSet<&str>,
) -> Resolution {
    let mut physical: Vec<_> = source
        .controls
        .iter()
        .filter(|c| available.contains(c.id.as_str()))
        .collect();
    let mut targets: Vec<_> = target
        .controls
        .iter()
        .filter(|c| requested.contains(c.id.as_str()))
        .collect();
    physical.sort_by_key(|c| &c.id);
    targets.sort_by_key(|c| &c.id);
    let mut result = Resolution {
        policy_version: POLICY_VERSION,
        assignments: BTreeMap::new(),
        rules: BTreeMap::new(),
        missing: BTreeMap::new(),
    };
    if targets.is_empty() {
        return result;
    }
    let n = targets.len() as i64;
    // Each level dominates the total possible cost of all levels below it.
    let preference_weight = n * 2_000 + 1;
    let optional_missing = n * (8 * preference_weight + 2_000) + 1;
    let required_missing = (n + 1) * optional_missing;
    let forbidden = (n + 1) * required_missing;
    // Geometry is normalized against full declared face groups, not whichever
    // controls happened to survive a partial calibration or requested subset.
    let source_face: Vec<_> = source
        .controls
        .iter()
        .filter(|c| c.group == "face")
        .collect();
    let target_face: Vec<_> = target
        .controls
        .iter()
        .filter(|c| c.group == "face")
        .collect();
    let edges: Vec<Vec<_>> = targets
        .iter()
        .map(|to| {
            physical
                .iter()
                .map(|from| candidate(source, target, from, to))
                .collect()
        })
        .collect();
    let costs: Vec<Vec<_>> = targets
        .iter()
        .enumerate()
        .map(|(row, to)| {
            let mut values: Vec<_> = physical
                .iter()
                .enumerate()
                .map(|(col, from)| {
                    let Some((rank, rule)) = edges[row][col] else {
                        return forbidden;
                    };
                    let geometry = if from.group == "face" && to.group == "face" && rank > 0 {
                        let (x, y) = normalized(from, &source_face);
                        let (tx, ty) = normalized(to, &target_face);
                        (((x - tx).powi(2) + (y - ty).powi(2)) * 1000.0).round() as i64
                    } else if rule == Rule::DigitalOverflow {
                        // Cross-group fallbacks still have ergonomic geometry:
                        // prefer a left spare for L and a right spare for R.
                        // Whole-layout coordinates share the catalog's 0..100
                        // front-view range; unrelated groups are not normalized
                        // separately into misleading identical positions.
                        let dx = (from.x - to.x) / 100.0;
                        let dy = (from.y - to.y) / 100.0;
                        ((dx * dx + dy * dy) * 1000.0).round() as i64
                    } else {
                        0
                    };
                    rank * preference_weight + geometry
                })
                .collect();
            values.extend(std::iter::repeat_n(
                if to.optional {
                    optional_missing
                } else {
                    required_missing
                },
                targets.len(),
            ));
            values
        })
        .collect();
    for (row, col) in minimum_assignment(&costs).into_iter().enumerate() {
        let to = targets[row];
        if let Some((_, rule)) = edges[row].get(col).copied().flatten() {
            result
                .assignments
                .insert(to.id.clone(), physical[col].id.clone());
            result.rules.insert(to.id.clone(), rule);
        } else {
            let reason = if edges[row].iter().any(Option::is_some) {
                Missing::InsufficientDistinctInputs
            } else if source
                .controls
                .iter()
                .any(|from| candidate(source, target, from, to).is_some())
            {
                Missing::NotCalibrated
            } else {
                Missing::IncompatibleHardware
            };
            result.missing.insert(to.id.clone(), reason);
        }
    }
    result
}

#[cfg(test)]
fn assignments(source: &Layout, target: &Layout) -> BTreeMap<String, String> {
    resolve(
        source,
        target,
        &source.controls.iter().map(|c| c.id.as_str()).collect(),
        &target.controls.iter().map(|c| c.id.as_str()).collect(),
    )
    .assignments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_catalog::catalog;
    #[test]
    fn guided_choices_reserve_inputs_before_automatic_assignment() {
        let layout = catalog().layout("snes").unwrap();
        let all = layout
            .controls
            .iter()
            .filter(|c| c.repeat_of.is_none())
            .map(|c| c.id.as_str())
            .collect();
        let choices = BTreeMap::from([("a".into(), "b".into())]);
        let result = resolve_with_choices(layout, layout, &all, &all, &choices).unwrap();
        assert_eq!(result.assignments["a"], "b");
        assert_eq!(result.rules["a"], Rule::UserChoice);
        assert_eq!(
            result.assignments.values().filter(|id| *id == "b").count(),
            1
        );
        assert!(result.missing.is_empty());
    }

    #[test]
    fn guided_choices_reject_collisions_unknown_and_unrecorded_inputs() {
        let layout = catalog().layout("snes").unwrap();
        let all = layout.controls.iter().map(|c| c.id.as_str()).collect();
        for choices in [
            BTreeMap::from([("a".into(), "b".into()), ("x".into(), "b".into())]),
            BTreeMap::from([("not-a-control".into(), "b".into())]),
            BTreeMap::from([("a".into(), "not-recorded".into())]),
        ] {
            assert!(resolve_with_choices(layout, layout, &all, &all, &choices).is_err());
        }
        let choices = BTreeMap::from([("a".into(), "b".into())]);
        assert!(resolve_with_choices(layout, layout, &BTreeSet::new(), &all, &choices).is_err());
    }

    #[test]
    fn guided_choices_cannot_supply_analog_input_with_a_button() {
        let source = catalog().layout("snes").unwrap();
        let target = catalog().layout("dualshock").unwrap();
        let all = source.controls.iter().map(|c| c.id.as_str()).collect();
        let analog = target.controls.iter().find(|c| c.analog).unwrap();
        assert!(
            resolve_with_choices(
                source,
                target,
                &all,
                &BTreeSet::from([analog.id.as_str()]),
                &BTreeMap::from([(analog.id.clone(), "a".into())])
            )
            .is_err()
        );
    }
    #[test]
    fn n64_to_diamond_preserves_the_run_jump_thumb_pair() {
        let pairs = assignments(
            catalog().layout("brawler64").unwrap(),
            catalog().layout("snes").unwrap(),
        );
        for (target, source) in [
            ("b", "a"),
            ("y", "b"),
            ("a", "c_down"),
            ("x", "c_left"),
            ("l", "l"),
            ("r", "r"),
        ] {
            assert_eq!(pairs[target], source);
        }
    }
    #[test]
    fn every_pair_is_deterministic_injective_and_preserves_capabilities() {
        for source in &catalog().layouts {
            for target in &catalog().layouts {
                let pairs = assignments(source, target);
                assert_eq!(pairs, assignments(source, target));
                let mut used = std::collections::HashSet::new();
                for (to, from) in &pairs {
                    assert!(
                        used.insert(from),
                        "{} -> {} reused {from}",
                        source.id,
                        target.id
                    );
                    let to = target.controls.iter().find(|c| c.id == *to).unwrap();
                    let from = source.controls.iter().find(|c| c.id == *from).unwrap();
                    assert!(candidate(source, target, from, to).is_some());
                    assert!(!to.analog || from.analog);
                    if to.analog {
                        // Analog shoulders with the same pressure role are the
                        // one intended cross-family analog equivalence.
                        let same_pressure_role = to.group == "shoulder"
                            && from.group == "shoulder"
                            && pressure_role(&from.id).is_some()
                            && pressure_role(&from.id) == pressure_role(&to.id);
                        assert!(
                            to.id == from.id || same_pressure_role,
                            "stick side/direction must be preserved ({} -> {})",
                            from.id,
                            to.id
                        );
                    }
                    if to.group == "dpad" {
                        // A directional cluster keeps its side and direction;
                        // only a matching dpad or left-stick half may supply it.
                        assert_eq!(
                            directional_cluster(source, from),
                            directional_cluster(target, to),
                            "{} -> {}: directional inputs keep their side and direction ({} -> {})",
                            source.id,
                            target.id,
                            from.id,
                            to.id
                        );
                        if from.group == "dpad" {
                            assert_eq!(to.id, from.id);
                        }
                    }
                    if from.group == "menu" {
                        assert_eq!(
                            to.group, from.group,
                            "{} -> {}: menu inputs are reserved ({} -> {})",
                            source.id, target.id, from.id, to.id
                        );
                    }
                    if from.repeat_of.is_some() {
                        assert_eq!(to.repeat_of, from.repeat_of);
                        assert_eq!(to.id, from.id);
                    }
                }
                if source.id == target.id {
                    assert_eq!(pairs.len(), target.controls.len());
                }
            }
        }
    }
    #[test]
    fn ergonomic_presets_keep_run_jump_and_six_button_rows() {
        let layout = |id| catalog().layout(id).unwrap();
        let pairs = assignments(layout("xbox"), layout("nes"));
        assert_eq!(pairs["a"], "b");
        assert_eq!(pairs["b"], "y");
        let pairs = assignments(layout("brawler64"), layout("genesis-6"));
        for (to, from) in [
            ("a", "a"),
            ("b", "c_down"),
            ("c", "c_right"),
            ("x", "b"),
            ("y", "c_left"),
            ("z", "c_up"),
        ] {
            assert_eq!(pairs[to], from);
        }
        let pairs = assignments(layout("nes"), layout("n64"));
        assert!(!pairs.contains_key("stick_up"));
    }
    #[test]
    fn horizontal_n30_pairs_do_not_inherit_the_diamond_run_jump_swap() {
        let source = crate::controller_catalog::catalog()
            .layout("horizontal-four")
            .unwrap();
        for target in ["nes", "gameboy", "pce-2"] {
            let pairs = assignments(
                source,
                crate::controller_catalog::catalog().layout(target).unwrap(),
            );
            assert_eq!(pairs["b"], "b");
            assert_eq!(pairs["a"], "a");
        }
        let pairs = assignments(
            source,
            crate::controller_catalog::catalog().layout("snes").unwrap(),
        );
        for face in ["a", "b", "x", "y"] {
            assert_eq!(pairs[face], face);
        }
        let turbo = crate::controller_catalog::catalog()
            .layout("n30-turbo")
            .unwrap();
        let pairs = assignments(
            turbo,
            crate::controller_catalog::catalog().layout("snes").unwrap(),
        );
        assert!(!pairs.values().any(|id| id.starts_with("turbo_")));
        assert!(!pairs.contains_key("x"));
        assert!(!pairs.contains_key("y"));
    }
    #[test]
    fn modern_dual_stick_pads_can_drive_n64_without_brawler_specific_wiring() {
        let db = crate::controller_catalog::catalog();
        let pairs = assignments(db.layout("xbox").unwrap(), db.layout("n64").unwrap());
        assert_eq!(pairs.len(), db.layout("n64").unwrap().controls.len());
        assert_eq!(pairs["a"], "b");
        assert_eq!(pairs["b"], "y");
        assert_eq!(pairs["c_up"], "right_stick_up");
        assert_eq!(pairs["z"], "l2");
    }

    #[test]
    fn digital_family_overflow_preserves_rows_and_never_reuses_shoulders() {
        let db = catalog();
        let pairs = assignments(db.layout("snes").unwrap(), db.layout("genesis-6").unwrap());
        for (to, from) in [
            ("a", "y"),
            ("b", "b"),
            ("c", "a"),
            ("x", "l"),
            ("y", "x"),
            ("z", "r"),
            ("mode", "select"),
        ] {
            assert_eq!(pairs[to], from);
        }
        assert_eq!(pairs.len(), db.layout("genesis-6").unwrap().controls.len());
        let reverse = assignments(db.layout("genesis-6").unwrap(), db.layout("snes").unwrap());
        assert_eq!(reverse.len(), db.layout("snes").unwrap().controls.len());
        for (to, from) in [
            ("a", "c"),
            ("b", "b"),
            ("y", "a"),
            ("x", "y"),
            ("l", "x"),
            ("r", "z"),
            ("select", "mode"),
        ] {
            assert_eq!(reverse[to], from);
        }
        for source in ["xbox", "dualshock", "playstation-digital"] {
            let pairs = assignments(db.layout(source).unwrap(), db.layout("saturn").unwrap());
            assert_eq!(pairs.len(), db.layout("saturn").unwrap().controls.len());
            assert_eq!(pairs["l"], "l2");
            assert_eq!(pairs["r"], "r2");
        }
        let reverse = assignments(
            db.layout("saturn").unwrap(),
            db.layout("playstation-digital").unwrap(),
        );
        assert_eq!(reverse["l"], "l");
        assert_eq!(reverse["r"], "r");
        assert_eq!(reverse["l2"], "x");
        assert_eq!(reverse["r2"], "z");
        assert!(
            !reverse.contains_key("select"),
            "Saturn has no auxiliary menu button"
        );
    }

    #[test]
    fn n64_family_bridges_are_directional_and_reverse_the_six_button_projection() {
        let db = catalog();
        let pairs = assignments(db.layout("snes").unwrap(), db.layout("n64").unwrap());
        assert_eq!(pairs["a"], "b");
        assert_eq!(pairs["b"], "y");
        assert_eq!(pairs["c_down"], "a");
        assert_eq!(pairs["c_left"], "x");
        assert!(!pairs.contains_key("c_up"));
        assert!(!pairs.contains_key("c_right"));
        let forward = assignments(
            db.layout("brawler64").unwrap(),
            db.layout("genesis-6").unwrap(),
        );
        let reverse = assignments(
            db.layout("genesis-6").unwrap(),
            db.layout("brawler64").unwrap(),
        );
        for face in ["a", "b", "c", "x", "y", "z"] {
            assert_eq!(reverse[&forward[face]], face);
        }
        let modern = assignments(db.layout("xbox").unwrap(), db.layout("brawler64").unwrap());
        assert_eq!(modern["z_right"], "r2");
        assert_eq!(modern["c_up"], "right_stick_up");
    }

    fn full_resolution(source: &Layout, target: &Layout) -> Resolution {
        resolve(
            source,
            target,
            &source.controls.iter().map(|c| c.id.as_str()).collect(),
            &target.controls.iter().map(|c| c.id.as_str()).collect(),
        )
    }

    #[test]
    fn all_pairs_are_catalog_order_independent_and_explain_every_target() {
        let db = catalog();
        for source in &db.layouts {
            for target in &db.layouts {
                let resolution = full_resolution(source, target);
                assert_eq!(
                    resolution.assignments.len() + resolution.missing.len(),
                    target.controls.len()
                );
                assert_eq!(resolution.rules.len(), resolution.assignments.len());
                let mut reordered_source = source.clone();
                let mut reordered_target = target.clone();
                reordered_source.controls.reverse();
                reordered_target.controls.rotate_left(1);
                assert_eq!(
                    serde_json::to_value(&resolution).unwrap(),
                    serde_json::to_value(full_resolution(&reordered_source, &reordered_target))
                        .unwrap(),
                    "{} -> {}",
                    source.id,
                    target.id
                );
                if source.id == target.id {
                    assert!(
                        resolution.assignments.iter().all(|(to, from)| to == from),
                        "identity mapping changed for {}: {:?}",
                        source.id,
                        resolution.assignments
                    );
                }
            }
        }
    }

    #[test]
    fn all_pairs_maximize_required_then_optional_coverage_against_augmenting_path_oracle() {
        // Independent unweighted matching oracle: insert mandatory vertices
        // first, then optional vertices. Augmentation can move but never drop a
        // previously matched vertex. No production cost/solver code is reused.
        fn augment(
            row: usize,
            edges: &[Vec<usize>],
            owner: &mut [Option<usize>],
            seen: &mut [bool],
        ) -> bool {
            for &col in &edges[row] {
                if seen[col] {
                    continue;
                }
                seen[col] = true;
                if owner[col].is_none() || augment(owner[col].unwrap(), edges, owner, seen) {
                    owner[col] = Some(row);
                    return true;
                }
            }
            false
        }
        for source in &catalog().layouts {
            for target in &catalog().layouts {
                let edges: Vec<Vec<_>> = target
                    .controls
                    .iter()
                    .map(|to| {
                        source
                            .controls
                            .iter()
                            .enumerate()
                            .filter(|(_, from)| candidate(source, target, from, to).is_some())
                            .map(|(i, _)| i)
                            .collect()
                    })
                    .collect();
                let mut owner = vec![None; source.controls.len()];
                for optional in [false, true] {
                    for (row, _) in target
                        .controls
                        .iter()
                        .enumerate()
                        .filter(|(_, to)| to.optional == optional)
                    {
                        augment(
                            row,
                            &edges,
                            &mut owner,
                            &mut vec![false; source.controls.len()],
                        );
                    }
                }
                let expected_required = owner
                    .iter()
                    .flatten()
                    .filter(|row| !target.controls[**row].optional)
                    .count();
                let actual = full_resolution(source, target);
                let actual_required = target
                    .controls
                    .iter()
                    .filter(|to| !to.optional && actual.assignments.contains_key(&to.id))
                    .count();
                assert_eq!(
                    actual_required, expected_required,
                    "{} -> {} required",
                    source.id, target.id
                );
                assert_eq!(
                    actual.assignments.len(),
                    owner.iter().flatten().count(),
                    "{} -> {} total",
                    source.id,
                    target.id
                );
            }
        }
    }

    #[test]
    fn all_pairs_with_each_source_input_removed_remain_valid_and_do_not_gain_coverage() {
        let db = catalog();
        for source in &db.layouts {
            for target in &db.layouts {
                let requested = target.controls.iter().map(|c| c.id.as_str()).collect();
                let complete = full_resolution(source, target);
                let covered = |r: &Resolution| {
                    target
                        .controls
                        .iter()
                        .filter(|c| !c.optional && r.assignments.contains_key(&c.id))
                        .count()
                };
                for removed in &source.controls {
                    let available: BTreeSet<_> = source
                        .controls
                        .iter()
                        .filter(|c| c.id != removed.id)
                        .map(|c| c.id.as_str())
                        .collect();
                    let partial = resolve(source, target, &available, &requested);
                    assert!(
                        partial
                            .assignments
                            .values()
                            .all(|id| available.contains(id.as_str()))
                    );
                    assert!(covered(&partial) <= covered(&complete));
                    assert_eq!(
                        partial.assignments.values().collect::<BTreeSet<_>>().len(),
                        partial.assignments.len()
                    );
                    assert_eq!(
                        partial.assignments.len() + partial.missing.len(),
                        requested.len()
                    );
                }
            }
        }
    }

    #[test]
    fn required_coverage_precedes_optional_identity_and_missing_reasons_are_specific() {
        let db = catalog();
        let source = db.layout("nes").unwrap();
        let mut target = source.clone();
        target
            .controls
            .iter_mut()
            .find(|c| c.id == "a")
            .unwrap()
            .optional = true;
        let result = resolve(
            source,
            &target,
            &BTreeSet::from(["a"]),
            &BTreeSet::from(["a", "b"]),
        );
        assert_eq!(result.assignments["b"], "a");
        assert_eq!(result.missing["a"], Missing::InsufficientDistinctInputs);
        let no_inputs = resolve(source, &target, &BTreeSet::new(), &BTreeSet::from(["b"]));
        assert_eq!(no_inputs.missing["b"], Missing::NotCalibrated);
        let no_stick = full_resolution(source, db.layout("n64").unwrap());
        assert_eq!(no_stick.missing["stick_up"], Missing::IncompatibleHardware);
        let subset = resolve(
            source,
            &target,
            &BTreeSet::from(["a"]),
            &BTreeSet::from(["a"]),
        );
        assert_eq!(
            subset.assignments["a"], "a",
            "unrequested controls cannot steal an input"
        );
    }

    #[test]
    fn polynomial_solver_matches_exhaustive_small_assignment_oracle() {
        fn brute(costs: &[Vec<i64>], row: usize, used: u64) -> i64 {
            if row == costs.len() {
                return 0;
            }
            (0..costs[row].len())
                .filter(|c| used & (1 << c) == 0)
                .map(|c| costs[row][c] + brute(costs, row + 1, used | (1 << c)))
                .min()
                .unwrap()
        }
        // Exhaust all 2x3 matrices with costs 0..3, including ties and collisions.
        for encoded in 0..4096 {
            let mut value = encoded;
            let mut costs = vec![vec![0; 3]; 2];
            for cell in costs.iter_mut().flatten() {
                *cell = value % 4;
                value /= 4;
            }
            let chosen = minimum_assignment(&costs);
            assert_ne!(chosen[0], chosen[1]);
            assert_eq!(
                costs[0][chosen[0]] + costs[1][chosen[1]],
                brute(&costs, 0, 0)
            );
        }
        let costs = vec![vec![0, 1, 100], vec![0, 99, 100], vec![99, 0, 1]];
        let chosen = minimum_assignment(&costs);
        assert_eq!(
            chosen
                .iter()
                .enumerate()
                .map(|(r, c)| costs[r][*c])
                .sum::<i64>(),
            brute(&costs, 0, 0)
        );
    }

    #[test]
    fn spare_gameplay_buttons_cover_auxiliary_controls_without_faking_analog() {
        let db = catalog();
        let result = full_resolution(
            db.layout("n64").unwrap(),
            db.layout("playstation-digital").unwrap(),
        );
        assert!(
            result.missing.is_empty(),
            "a standard N64 has fourteen independent digital inputs"
        );
        for target in ["select", "r2"] {
            assert_eq!(result.rules[target], Rule::DigitalOverflow);
            assert!(matches!(
                result.assignments[target].as_str(),
                "c_up" | "c_right"
            ));
        }
        let analog = full_resolution(
            db.layout("brawler64").unwrap(),
            db.layout("dualshock").unwrap(),
        );
        for target in ["l3", "r3"] {
            assert_eq!(analog.rules[target], Rule::DigitalOverflow);
        }
        assert_eq!(analog.missing.len(), 4);
        assert!(
            analog
                .missing
                .keys()
                .all(|id| id.starts_with("right_stick_"))
        );
        let source = db.layout("brawler64").unwrap();
        let target = db.layout("nes").unwrap();
        let reserved = resolve(
            source,
            target,
            &BTreeSet::from(["home", "start", "select", "up", "down", "left", "right"]),
            &BTreeSet::from(["a", "b"]),
        );
        assert!(
            reserved.assignments.is_empty(),
            "reserved navigation cannot become Fire"
        );
    }

    #[test]
    #[ignore = "writes the complete mapping report to the explicitly supplied LUNCHBOX_LAYOUT_REPORT path"]
    fn export_all_pairs_report() {
        let db = catalog();
        let mut pairs = Vec::new();
        for source in &db.layouts {
            for target in &db.layouts {
                let resolution = full_resolution(source, target);
                let missing_required: Vec<_> = target
                    .controls
                    .iter()
                    .filter(|c| !c.optional && resolution.missing.contains_key(&c.id))
                    .map(|c| &c.id)
                    .collect();
                pairs.push(serde_json::json!({"source":source.id, "target":target.id,
                    "required_complete":missing_required.is_empty(), "missing_required":missing_required, "resolution":resolution}));
            }
        }
        let report = serde_json::json!({"policy_version": POLICY_VERSION, "layouts":db.layouts.len(), "pairs":pairs});
        let path = std::path::PathBuf::from(
            std::env::var_os("LUNCHBOX_LAYOUT_REPORT").expect("Set report destination"),
        );
        assert!(path.is_absolute(), "Report destination must be absolute");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(&mut file, &report).unwrap();
        eprintln!(
            "Audited {} layouts and {} ordered pairs",
            db.layouts.len(),
            db.layouts.len().pow(2)
        );
    }
}
