//! RetroArch udev startup table, pinned to 69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576.
//! This is observed startup numbering, not a persistent device-routing identity.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// Pinned input/input_defines.h. udev_get_mouse rejects indices at or above
// this bound, making the bound itself an explicit no-mouse route.
const MAX_INPUT_DEVICES: u32 = 16;

#[derive(Clone, Debug)]
pub struct MouseEntry {
    pub index: u32,
    pub event_node: PathBuf,
    pub absolute: bool,
}

/// Parse a single frontend startup's log. The caller must establish executable,
/// log provenance and input-driver identity separately. ANSI-decorated, truncated,
/// repeated-startup and malformed tables are rejected rather than guessed.
pub fn parse_startup_table(log: &str) -> Result<Vec<MouseEntry>> {
    ensure!(
        log.len() <= 8 * 1024 * 1024,
        "Frontend startup log is too large"
    );
    let mut entries = BTreeMap::new();
    let mut paths = BTreeSet::new();
    for line in log.lines() {
        let line = line.strip_prefix("[INFO] ").unwrap_or(line);
        let Some(row) = line.strip_prefix("[udev] Mouse/Touch #") else {
            ensure!(
                !line.contains("[udev] Mouse/Touch #"),
                "Unsupported decoration on frontend mouse entry"
            );
            continue;
        };
        ensure!(row.len() <= 4096, "Frontend mouse entry is too long");
        let (index, rest) = row
            .split_once(": \"")
            .ok_or_else(|| anyhow::anyhow!("Malformed frontend mouse entry"))?;
        ensure!(
            !index.is_empty() && index.bytes().all(|c| c.is_ascii_digit()),
            "Invalid frontend mouse index"
        );
        let index: u32 = index.parse()?;
        ensure!(
            index < MAX_INPUT_DEVICES,
            "Frontend mouse index exceeds the pinned driver limit"
        );
        // Use the final delimiter so a device label cannot impersonate an
        // event path or change the reported coordinate mode.
        let (_, endpoint) = rest
            .rsplit_once("\" (")
            .ok_or_else(|| anyhow::anyhow!("Missing frontend mouse mode"))?;
        let (mode, path) = endpoint
            .split_once(") ")
            .ok_or_else(|| anyhow::anyhow!("Missing frontend mouse path"))?;
        ensure!(matches!(mode, "ABS" | "REL"), "Unknown frontend mouse mode");
        let path = path
            .strip_suffix('.')
            .ok_or_else(|| anyhow::anyhow!("Truncated frontend mouse entry"))?;
        let suffix = path
            .strip_prefix("/dev/input/event")
            .ok_or_else(|| anyhow::anyhow!("Unexpected frontend event path"))?;
        ensure!(
            !suffix.is_empty() && suffix.bytes().all(|c| c.is_ascii_digit()),
            "Invalid frontend event path"
        );
        let event_node = PathBuf::from(path);
        ensure!(
            paths.insert(event_node.clone()),
            "Repeated frontend mouse endpoint"
        );
        ensure!(
            entries
                .insert(
                    index,
                    MouseEntry {
                        index,
                        event_node,
                        absolute: mode == "ABS"
                    }
                )
                .is_none(),
            "Repeated frontend mouse index"
        );
        ensure!(
            entries.len() <= MAX_INPUT_DEVICES as usize,
            "Frontend mouse table exceeds bounds"
        );
    }
    ensure!(!entries.is_empty(), "No frontend udev mouse table found");
    ensure!(
        entries.keys().copied().eq(0..entries.len() as u32),
        "Incomplete frontend mouse table"
    );
    Ok(entries.into_values().collect())
}

/// Select the logged REL index for an already-revalidated owned endpoint.
/// The result applies only to this startup snapshot. A caller must detect
/// hotplug/reindexing or use an exact-endpoint frontend before forwarding input.
pub fn relative_index(entries: &[MouseEntry], owned_event: &Path) -> Result<u32> {
    let mut matching = entries
        .iter()
        .filter(|entry| entry.event_node == owned_event);
    let entry = matching
        .next()
        .ok_or_else(|| anyhow::anyhow!("Owned relative endpoint was not opened by the frontend"))?;
    ensure!(
        matching.next().is_none() && !entry.absolute,
        "Owned endpoint is ambiguous or not relative"
    );
    Ok(entry.index)
}

/// Resolve all selected players against one complete startup table. Endpoint
/// paths must come from live, revalidated owned virtual devices. This function
/// does not establish log provenance, endpoint identity or continued topology
/// stability; the launch owner retains those responsibilities.
pub fn relative_player_indices(
    log: &str,
    owned_events: &BTreeMap<u8, PathBuf>,
) -> Result<BTreeMap<u8, u32>> {
    ensure!(
        !owned_events.is_empty() && owned_events.len() <= 8,
        "Select one to eight owned relative endpoints"
    );
    let entries = parse_startup_table(log)?;
    let mut endpoints = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let mut players = BTreeMap::new();
    for (&player, event) in owned_events {
        ensure!((1..=8).contains(&player), "Invalid relative source player");
        ensure!(
            endpoints.insert(event),
            "Multiple players cannot implicitly share one relative endpoint"
        );
        let index = relative_index(&entries, event)?;
        ensure!(
            indices.insert(index),
            "Multiple owned endpoints resolve to the same frontend mouse index"
        );
        players.insert(player, index);
    }
    Ok(players)
}

/// Private startup configuration, not evidence that these routes were applied.
/// Pinned configuration.c reads signed integers for input_playerN_mouse_index;
/// the small out-of-range sentinel is representable and udev returns no mouse.
/// No joypad driver or button settings are changed by this fragment.
pub fn relative_player_config(routes: &BTreeMap<u8, u32>) -> Result<String> {
    let state = FrontendMouseState::desired(routes)?;
    Ok(mouse_config(&state.mouse_indices))
}

/// Start with every port disconnected until the owned-child handshake applies
/// routes from this startup's enumeration. No host mouse is selected by default.
pub fn relative_bootstrap_config() -> String {
    mouse_config(&[MAX_INPUT_DEVICES; 8])
}

pub fn relative_route_command(routes: &BTreeMap<u8, u32>) -> Result<String> {
    let state = FrontendMouseState::desired(routes)?;
    Ok(format!(
        "LUNCHBOX_SET_MOUSE_ROUTES {}\n",
        state
            .mouse_indices
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

fn mouse_config(indices: &[u32; 8]) -> String {
    let mut config = String::from(
        "input_driver = \"udev\"\nstdin_cmd_enable = \"true\"\nnetwork_cmd_enable = \"false\"\nlog_to_file = \"false\"\n",
    );
    for (port, index) in indices.iter().enumerate() {
        config.push_str(&format!(
            "input_player{}_mouse_index = \"{index}\"\n",
            port + 1
        ));
    }
    config
}

/// Complete frontend routing state for the eight supported source ports.
/// Deserialization establishes structure only. The launch owner must obtain
/// effective values from the same process/startup as the mouse enumeration;
/// copying desired settings into this object is not runtime evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendMouseState {
    pub input_driver: String,
    pub mouse_indices: [u32; 8],
}

/// Runtime-only observation; focus cannot be supplied by desired configuration.
#[derive(Clone, Debug)]
pub struct FrontendMouseObservation {
    pub routing: FrontendMouseState,
    pub focused: bool,
}

impl FrontendMouseObservation {
    /// Reply from the opt-in pinned frontend extension in
    /// packaging/retroarch-relative-routing.patch. Parse one reply only;
    /// the owning command channel must establish process identity, freshness
    /// and isolation. An unpatched frontend's `unsupported` is an error.
    pub fn from_command_reply(reply: &str) -> Result<Self> {
        ensure!(
            reply.len() <= 512,
            "Frontend mouse-state reply exceeds bounds"
        );
        let reply = reply.strip_suffix('\n').unwrap_or(reply);
        let body = reply
            .strip_prefix("GET_CONFIG_PARAM lunchbox_mouse_routes_v1 ")
            .ok_or_else(|| anyhow::anyhow!("Unexpected frontend mouse-state reply"))?;
        let mut values = body.split(' ');
        ensure!(
            values.next() == Some("udev"),
            "Frontend does not report supported effective udev mouse routing"
        );
        let mut mouse_indices = [0u32; 8];
        for index in &mut mouse_indices {
            let value = values
                .next()
                .ok_or_else(|| anyhow::anyhow!("Incomplete frontend mouse-state reply"))?;
            ensure!(
                !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()),
                "Invalid effective frontend mouse index"
            );
            *index = value.parse()?;
            ensure!(
                *index <= MAX_INPUT_DEVICES,
                "Effective frontend mouse index exceeds routing contract"
            );
        }
        ensure!(
            values.next() == Some("focus"),
            "Missing frontend focus observation"
        );
        let focused = match values.next() {
            Some("0") => false,
            Some("1") => true,
            _ => anyhow::bail!("Invalid frontend focus observation"),
        };
        ensure!(
            values.next().is_none(),
            "Extra data in frontend mouse-state reply"
        );
        Ok(Self {
            routing: FrontendMouseState {
                input_driver: "udev".to_owned(),
                mouse_indices,
            },
            focused,
        })
    }
}

impl FrontendMouseState {
    pub fn is_disabled(&self) -> bool {
        self.input_driver == "udev" && self.mouse_indices == [MAX_INPUT_DEVICES; 8]
    }
    fn desired(routes: &BTreeMap<u8, u32>) -> Result<Self> {
        ensure!(
            !routes.is_empty() && routes.len() <= 8,
            "Select one to eight relative players"
        );
        let mut indices = BTreeSet::new();
        for (&player, &index) in routes {
            ensure!((1..=8).contains(&player), "Invalid relative source player");
            ensure!(
                index < MAX_INPUT_DEVICES,
                "Frontend mouse index exceeds the pinned driver limit"
            );
            ensure!(
                indices.insert(index),
                "Relative players cannot implicitly share a frontend mouse index"
            );
        }
        let mut mouse_indices = [MAX_INPUT_DEVICES; 8];
        for (&player, &index) in routes {
            mouse_indices[usize::from(player - 1)] = index;
        }
        Ok(Self {
            input_driver: "udev".to_owned(),
            mouse_indices,
        })
    }

    /// Check selected routes, the effective driver and all unused ports.
    /// Provenance and subsequent configuration changes remain owner concerns.
    pub fn validate_routes(&self, routes: &BTreeMap<u8, u32>) -> Result<()> {
        let expected = Self::desired(routes)?;
        ensure!(
            self.input_driver == expected.input_driver,
            "Relative forwarding requires the effective udev input driver"
        );
        for (port, (&actual, &wanted)) in self
            .mouse_indices
            .iter()
            .zip(expected.mouse_indices.iter())
            .enumerate()
        {
            ensure!(
                actual == wanted,
                "Effective mouse route for player {} differs from the owned route or unused-port sentinel",
                port + 1
            );
        }
        Ok(())
    }
}
