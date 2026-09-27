//! Replay Dolphin 2606's ordered evdev population and node merging.
//! Caller must supply the complete native enumeration order, not UI order.
use super::evdev::{DeviceObservation, NodeObservation};
use anyhow::{Result, ensure};
use std::collections::BTreeSet;

#[cfg(target_os = "linux")]
pub(crate) mod capture;

pub(crate) struct Node {
    pub name: String,
    /// None represents a null libevdev identity, distinct from an empty string.
    pub unique_id: Option<String>,
    pub physical_location: Option<String>,
    pub observation: NodeObservation,
}

struct Group {
    name: String,
    id: u32,
    unique_id: Option<String>,
    physical_location: Option<String>,
    nodes: Vec<NodeObservation>,
}

/// This resolves same-name devices by native registration order, including
/// removal/re-registration when a merged node changes a device's name.
pub(crate) fn qualify(ordered_nodes: Vec<Node>) -> Result<Vec<DeviceObservation>> {
    ensure!(
        ordered_nodes.len() <= 1024,
        "Dolphin evdev inventory exceeds limit"
    );
    let mut paths = BTreeSet::new();
    let mut groups: Vec<Group> = Vec::new();
    for node in ordered_nodes {
        ensure!(
            paths.insert(node.observation.path.clone()),
            "Duplicate Dolphin evdev inventory node"
        );
        let name = node.name.trim_matches([' ', '\t', '\r', '\n']).to_owned();
        ensure!(
            !name.is_empty() && !name.chars().any(char::is_control),
            "Invalid Dolphin evdev name"
        );
        // Dolphin's GetUniqueID converts an empty first-node unique ID to
        // null, so empty identities must never combine otherwise similar pads.
        let existing = groups.iter().position(|group| {
            group.unique_id.as_ref().is_some_and(|id| !id.is_empty())
                && group.physical_location.is_some()
                && group.unique_id == node.unique_id
                && group.physical_location == node.physical_location
        });
        let mut group = if let Some(index) = existing {
            let mut group = groups.remove(index);
            if name < group.name {
                group.name = name;
            }
            group.nodes.push(node.observation);
            group
        } else {
            let observation = node.observation;
            let interesting = observation.motion
                || observation.pointing
                || observation.keys.len() >= 8
                || observation.axes.keys().filter(|code| **code < 0x28).count() >= 2;
            if !interesting {
                continue;
            }
            Group {
                name,
                id: 0,
                unique_id: node.unique_id,
                physical_location: node.physical_location,
                nodes: vec![observation],
            }
        };
        group.id = (0..=1024)
            .find(|id| {
                !groups
                    .iter()
                    .any(|other| other.name == group.name && other.id == *id)
            })
            .expect("bounded inventory has a free ID");
        groups.push(group);
    }
    Ok(groups
        .into_iter()
        .map(|group| DeviceObservation {
            qualifier: format!("evdev/{}/{}", group.id, group.name),
            nodes: group.nodes,
        })
        .collect())
}
