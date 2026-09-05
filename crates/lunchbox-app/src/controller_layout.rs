//! Deterministic layout-to-layout assignment, independent of emulator numbering.
//! Semantic groups are hard constraints. Face assignments minimize an explicit
//! ergonomic cost globally, not a greedy nearest-button guess. Missing physical
//! capabilities stay missing (a digital button cannot become an analog stick).
use crate::controller_catalog::{Control, Layout};
use std::collections::{BTreeMap, HashMap};

fn preferred<'a>(source: &Layout, target: &Layout, id: &'a str) -> &'a str {
    if source.family == "diamond" && target.family == "n64" {
        return match id {
            "a" => "b",
            "b" => "y",
            "z" => "l2",
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

pub fn assignments(source: &Layout, target: &Layout) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    // Reviewed capability conversion: right-stick directions may drive digital
    // N64 C inputs. The reverse cannot provide a real analog stick and is not
    // inferred. Missing sticks on e.g. SNES pads stay missing.
    if source.family == "diamond" && target.family == "n64" {
        for control in target.controls.iter().filter(|c| c.id.starts_with("c_")) {
            let id = preferred(source, target, &control.id);
            if source.controls.iter().any(|c| c.id == id && c.analog) {
                result.insert(control.id.clone(), id.to_string());
            }
        }
    }
    for control in target.controls.iter().filter(|c| c.group != "face") {
        let id = preferred(source, target, &control.id);
        if let Some(physical) = source
            .controls
            .iter()
            .find(|c| c.id == id && c.group == control.group && (!control.analog || c.analog))
        {
            result.insert(control.id.clone(), physical.id.clone());
        }
    }
    let physical = source
        .controls
        .iter()
        .filter(|c| c.group == "face" && !result.values().any(|id| *id == c.id))
        .collect::<Vec<_>>();
    let targets = target
        .controls
        .iter()
        .filter(|c| c.group == "face" && !result.contains_key(&c.id))
        .collect::<Vec<_>>();
    // Catalog validation bounds this exponential search to eight face controls.
    // Stable catalog order breaks equal-cost ties, making every pair reproducible.
    let mut states = HashMap::from([(0usize, (0u64, Vec::<Option<usize>>::new()))]);
    for control in &targets {
        let mut next: HashMap<usize, (u64, Vec<Option<usize>>)> = HashMap::new();
        let mut ordered = states.into_iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(mask, _)| *mask);
        for (mask, (cost, chosen)) in ordered {
            let mut add = |mask, cost, input| {
                let mut chosen = chosen.clone();
                chosen.push(input);
                if next.get(&mask).is_none_or(|(previous, _)| cost < *previous) {
                    next.insert(mask, (cost, chosen));
                }
            };
            for (index, candidate) in physical.iter().enumerate() {
                if mask & (1 << index) != 0 || (control.analog && !candidate.analog) {
                    continue;
                }
                let (x, y) = normalized(candidate, &physical);
                let (tx, ty) = normalized(control, &targets);
                let geometry = (((x - tx).powi(2) + (y - ty).powi(2)) * 1000.0).round() as u64;
                let semantic = if candidate.id == preferred(source, target, &control.id) {
                    0
                } else {
                    10_000
                };
                add(mask | (1 << index), cost + semantic + geometry, Some(index));
            }
            add(
                mask,
                cost + if control.optional { 100_000 } else { 1_000_000 },
                None,
            );
        }
        states = next;
    }
    if let Some((_, (_, chosen))) = states
        .into_iter()
        .min_by_key(|(mask, (cost, _))| (*cost, *mask))
    {
        for (control, index) in targets.iter().zip(chosen) {
            if let Some(index) = index {
                result.insert(control.id.clone(), physical[index].id.clone());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_catalog::catalog;
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
                    assert!(to.group == from.group || (to.id.starts_with("c_") && from.analog));
                    assert!(!to.analog || from.analog);
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
}
