use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::platform_process::host_command;
use crate::settings::{
    AppSettings, CONTROLLER_GAMEPAD_BUTTONS, ControllerButtonMapping, ControllerCustomProfile,
    ControllerMappingSettings, ControllerPlayerMapping,
};

pub const PROFILE_INHERIT: &str = "__inherit";
pub const PROFILE_NONE: &str = "__none";
pub const PROFILE_CREATE: &str = "__create_profile";
pub const TARGET_INHERIT: &str = "__inherit_target";
pub const ACTION_REMAP: &str = "remap";
pub const ACTION_PASSTHROUGH: &str = "passthrough";
pub const ACTION_HIDE: &str = "hide";
pub const TWO_BUTTON_CLOCKWISE_PROFILE_ID: &str = "two-button-clockwise";
pub const DEVICE_LAYOUTS: &[(&str, &str)] = &[
    ("auto", "Detect automatically"),
    ("diamond", "Standard diamond (PS5 / Steam / Xbox)"),
    ("horizontal", "Horizontal — left reports B, right reports A"),
    (
        "horizontal-swapped",
        "Horizontal — left reports A, right reports B",
    ),
    ("nintendo", "Nintendo diamond — swap A/B and X/Y"),
];
pub const SYSTEM_LAYOUTS: &[(&str, &str)] = &[
    ("two-button", "NES / Game Boy / two-button PC Engine"),
    ("n64", "Nintendo 64"),
    ("six-button", "Mega Drive / Saturn / Arcade"),
    ("modern", "SNES and modern systems"),
];

pub fn system_layout(platform: &str) -> &'static str {
    let platform = platform.to_ascii_lowercase();
    if platform.contains("nintendo 64") || platform == "n64" {
        "n64"
    } else if matches!(platform.trim(), "nes" | "gb" | "gbc" | "pce" | "tg16") {
        "two-button"
    } else if [
        "nintendo entertainment",
        "famicom",
        "game boy",
        "gameboy",
        "pc engine",
        "turbografx",
    ]
    .iter()
    .any(|part| platform.contains(part))
        && !platform.contains("super")
        && !platform.contains("advance")
    {
        "two-button"
    } else if ["genesis", "mega drive", "megadrive", "saturn", "arcade"]
        .iter()
        .any(|part| platform.contains(part))
    {
        "six-button"
    } else {
        "modern"
    }
}

pub fn detected_device_layout(device: &ControllerDevice) -> &'static str {
    let name = device.name.to_ascii_lowercase();
    // Both the user's horizontal pad and N64-style pad report this Xbox ID.
    // XInput describes the protocol, not the physical face-button arrangement.
    if device.vendor_id.as_deref() == Some("045e") && device.product_id.as_deref() == Some("028e") {
        return "auto";
    }
    // Do not infer nonstandard button wiring from a marketing name. Receivers
    // and XInput/DInput modes can expose different layouts under the same name.
    if name.contains("dualsense")
        || name.contains("dualshock")
        || name.contains("x-box")
        || name.contains("xbox")
        || name.contains("steam controller")
    {
        "diamond"
    } else {
        "auto"
    }
}

pub fn device_layout<'a>(
    mapping: &'a ControllerMappingSettings,
    device: &ControllerDevice,
) -> &'a str {
    match mapping
        .device_layouts
        .get(&device.stable_id)
        .map(String::as_str)
    {
        Some(layout) if layout != "auto" => layout,
        _ => detected_device_layout(device),
    }
}

fn automatic_profile(layout: &str, system: &str) -> Option<&'static str> {
    match (layout, system) {
        ("diamond", "two-button") => Some(TWO_BUTTON_CLOCKWISE_PROFILE_ID),
        ("horizontal-swapped", "two-button") => Some("horizontal-ab-swap"),
        ("nintendo", _) => Some("nintendo-label-swap"),
        _ => None,
    }
}

fn automatic_player_mappings(
    mapping: &ControllerMappingSettings,
    controllers: &[ControllerDevice],
    platform: Option<&str>,
    game_id: Option<i64>,
) -> ControllerMappingSettings {
    let mut resolved = mapping.clone();
    if !mapping.automatic
        || mapping
            .player_mappings
            .iter()
            .any(|player| player.controller_id.as_deref() == Some(CONTROLLER_SCOPE_ALL))
    {
        return resolved;
    }
    let system = system_layout(platform.unwrap_or(""));
    let mut devices = controllers
        .iter()
        .filter(|device| !mapping.hidden_controller_ids.contains(&device.stable_id))
        .collect::<Vec<_>>();
    let preferred = mapping.preferred_devices.get(system);
    devices.sort_by_key(|device| {
        let explicit_player = mapping
            .player_mappings
            .iter()
            .position(|player| player.controller_id.as_deref() == Some(&device.stable_id));
        let layout = device_layout(mapping, device);
        (
            explicit_player.is_none(),
            explicit_player.unwrap_or(usize::MAX),
            preferred != Some(&device.stable_id),
            if system == "two-button" {
                !layout.starts_with("horizontal")
            } else {
                layout != "diamond"
            },
        )
    });
    let has_override = mapping.default_profile_id.is_some()
        || platform.is_some_and(|platform| mapping.platform_profile_ids.contains_key(platform))
        || game_id.is_some_and(|id| mapping.game_profile_ids.contains_key(&id.to_string()));
    resolved.player_mappings = devices
        .into_iter()
        .map(|device| {
            let mut player = mapping
                .player_mappings
                .iter()
                .find(|player| player.controller_id.as_deref() == Some(&device.stable_id))
                .cloned()
                .unwrap_or_else(|| ControllerPlayerMapping {
                    controller_id: Some(device.stable_id.clone()),
                    ..Default::default()
                });
            let specific_profile = mapping
                .device_system_profiles
                .get(&device.stable_id)
                .and_then(|profiles| profiles.get(system))
                .cloned();
            if game_id.is_some_and(|id| mapping.game_profile_ids.contains_key(&id.to_string())) {
                // Let resolve_active_player_mappings inherit the game override.
                player.profile_id = None;
            } else if specific_profile.is_some() {
                player.profile_id = specific_profile;
            } else if player.profile_id.is_none() && !has_override {
                player.profile_id =
                    automatic_profile(device_layout(mapping, device), system).map(str::to_owned);
            }
            player
        })
        .collect();
    resolved
}

pub const CONTROLLER_PROFILE_BUTTONS: &[(&str, &str)] = &[
    ("LeftStickUp", "Left stick up"),
    ("LeftStickDown", "Left stick down"),
    ("LeftStickLeft", "Left stick left"),
    ("LeftStickRight", "Left stick right"),
    ("RightStickUp", "Right stick up / C-up (axis mode)"),
    ("RightStickDown", "Right stick down / C-down (axis mode)"),
    ("RightStickLeft", "Right stick left / C-left (axis mode)"),
    ("RightStickRight", "Right stick right / C-right (axis mode)"),
    ("South", "South / A / Cross"),
    ("East", "East / B / Circle"),
    ("West", "West / X / Square"),
    ("North", "North / Y / Triangle"),
    ("Start", "Start / Menu / Options"),
    ("Select", "Select / View / Share"),
    ("Guide", "Guide / PS / Xbox"),
    ("DPadUp", "D-pad up"),
    ("DPadDown", "D-pad down"),
    ("DPadLeft", "D-pad left"),
    ("DPadRight", "D-pad right"),
    ("LeftBumper", "L1 / LB"),
    ("RightBumper", "R1 / RB"),
    ("LeftTrigger", "L2 / LT"),
    ("RightTrigger", "R2 / RT"),
    ("LeftStick", "Left stick press"),
    ("RightStick", "Right stick press"),
];

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const DEVICE_LIST_TIMEOUT: Duration = Duration::from_secs(20);
const DEVICE_LIST_RETRIES: usize = 1;
const LAUNCH_QUERY_TIMEOUT: Duration = Duration::from_secs(2);
const LAUNCH_APPLY_TIMEOUT: Duration = Duration::from_secs(3);
const CUSTOM_PROFILE_PREFIX: &str = "custom:";
const CONTROLLER_SCOPE_ALL: &str = "__all";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ControllerInventory {
    pub provider: ControllerProviderStatus,
    pub controllers: Vec<ControllerDevice>,
    pub managed_device_count: usize,
    pub supported_targets: Vec<ControllerTarget>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ControllerProviderStatus {
    pub provider: String,
    pub available: bool,
    pub version: Option<String>,
    pub service_accessible: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerDevice {
    pub stable_id: String,
    pub name: String,
    pub device_path: PathBuf,
    pub event_paths: Vec<PathBuf>,
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub version: Option<String>,
    pub bus_type: Option<String>,
    pub physical_path: Option<String>,
    pub unique_id: Option<String>,
    pub is_virtual: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerTarget {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerProfile {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default)]
pub struct ControllerLaunchSession {
    restore_entries: Vec<InputPlumberRestoreEntry>,
    restore_order: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ControllerActivation {
    pub session: ControllerLaunchSession,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
struct InputPlumberRestoreEntry {
    device_id: String,
    intercept_mode: String,
    profile_path: Option<PathBuf>,
    target_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct InputPlumberManagedDevice {
    id: String,
    source_paths: HashSet<PathBuf>,
}

#[derive(Debug)]
struct InputPlumberLaunchDevice {
    id: String,
    hidden: bool,
    target_ids: Vec<String>,
    profile_path: Option<PathBuf>,
}

#[derive(Debug)]
struct ResolvedPlayerMapping {
    source_paths: HashSet<PathBuf>,
    target_ids: Vec<String>,
    profile_path: Option<PathBuf>,
}

pub fn built_in_profiles() -> Vec<ControllerProfile> {
    vec![ControllerProfile {
        id: TWO_BUTTON_CLOCKWISE_PROFILE_ID.to_owned(),
        name: "2-button X/A diamond".to_owned(),
        description: "Maps NES-style run/jump to the physical X/A face-button positions."
            .to_owned(),
    }]
}

pub fn controller_button_label(layout: &str, button: &str) -> String {
    let label = match layout {
        "playstation" => match button {
            "South" => "Cross",
            "East" => "Circle",
            "West" => "Square",
            "North" => "Triangle",
            "Start" => "Options",
            "Select" => "Share",
            "Guide" => "PS",
            "LeftBumper" => "L1",
            "RightBumper" => "R1",
            "LeftTrigger" => "L2",
            "RightTrigger" => "R2",
            "LeftStick" => "L3",
            "RightStick" => "R3",
            "DPadUp" => "Up",
            "DPadDown" => "Down",
            "DPadLeft" => "Left",
            "DPadRight" => "Right",
            _ => button,
        },
        "generic" => match button {
            "LeftBumper" => "LB",
            "RightBumper" => "RB",
            "LeftTrigger" => "LT",
            "RightTrigger" => "RT",
            "LeftStick" => "L3",
            "RightStick" => "R3",
            "DPadUp" => "Up",
            "DPadDown" => "Down",
            "DPadLeft" => "Left",
            "DPadRight" => "Right",
            _ => button,
        },
        _ => match button {
            "South" => "A",
            "East" => "B",
            "West" => "X",
            "North" => "Y",
            "Start" => "Menu",
            "Select" => "View",
            "Guide" => "Xbox",
            "LeftBumper" => "LB",
            "RightBumper" => "RB",
            "LeftTrigger" => "LT",
            "RightTrigger" => "RT",
            "LeftStick" => "LS",
            "RightStick" => "RS",
            "DPadUp" => "Up",
            "DPadDown" => "Down",
            "DPadLeft" => "Left",
            "DPadRight" => "Right",
            _ => button,
        },
    };
    label.to_owned()
}

pub fn custom_profile_source_for_target(profile: &ControllerCustomProfile, target: &str) -> String {
    profile
        .mappings
        .iter()
        .find(|mapping| mapping.target_button == target)
        .map(|mapping| mapping.source_button.clone())
        .unwrap_or_else(|| target.to_owned())
}

pub fn set_custom_profile_mapping(
    profile: &mut ControllerCustomProfile,
    target: &str,
    source: &str,
) -> bool {
    if !CONTROLLER_PROFILE_BUTTONS
        .iter()
        .any(|(button, _)| *button == target)
        || !CONTROLLER_PROFILE_BUTTONS
            .iter()
            .any(|(button, _)| *button == source)
    {
        return false;
    }
    let before = profile.mappings.clone();
    profile
        .mappings
        .retain(|mapping| mapping.target_button != target);
    if target != source {
        profile.mappings.push(ControllerButtonMapping {
            source_button: source.to_owned(),
            target_button: target.to_owned(),
        });
    }
    profile.mappings.sort_by(|left, right| {
        profile_button_order(&left.target_button).cmp(&profile_button_order(&right.target_button))
    });
    profile.mappings != before
}

pub fn apply_two_button_profile_preset(profile: &mut ControllerCustomProfile) {
    profile.mappings = vec![
        ControllerButtonMapping {
            source_button: "West".to_owned(),
            target_button: "East".to_owned(),
        },
        ControllerButtonMapping {
            source_button: "East".to_owned(),
            target_button: "West".to_owned(),
        },
    ];
}

pub fn remove_custom_profile(mapping: &mut ControllerMappingSettings, profile_id: &str) -> bool {
    let before = mapping.custom_profiles.len();
    mapping
        .custom_profiles
        .retain(|profile| profile.id != profile_id);
    if mapping.custom_profiles.len() == before {
        return false;
    }
    if mapping.default_profile_id.as_deref() == Some(profile_id) {
        mapping.default_profile_id = None;
    }
    for player in &mut mapping.player_mappings {
        if player.profile_id.as_deref() == Some(profile_id) {
            player.profile_id = None;
        }
    }
    mapping
        .platform_profile_ids
        .retain(|_, saved| saved != profile_id);
    mapping
        .game_profile_ids
        .retain(|_, saved| saved != profile_id);
    for profiles in mapping.device_system_profiles.values_mut() {
        profiles.retain(|_, saved| saved != profile_id);
    }
    trim_default_player_mappings(mapping);
    true
}

fn profile_button_order(button: &str) -> usize {
    CONTROLLER_PROFILE_BUTTONS
        .iter()
        .position(|(candidate, _)| *candidate == button)
        .unwrap_or(usize::MAX)
}

pub fn controller_inventory() -> ControllerInventory {
    let mut warnings = Vec::new();
    let controllers = list_local_controllers(&mut warnings);
    let (provider, managed_device_count, supported_targets) = inputplumber_inventory(&mut warnings);
    ControllerInventory {
        provider,
        controllers,
        managed_device_count,
        supported_targets,
        warnings,
    }
}

pub fn configure_linux_routing(enabled: bool) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("System-wide controller routing is only available on Linux.".into());
    }
    let args: &[&str] = if enabled {
        &["devices", "manage-all", "--enable"]
    } else {
        &["devices", "manage-all"]
    };
    run_command_with_timeout("inputplumber", args, COMMAND_TIMEOUT).map(|_| ())
}

pub fn ordered_controllers(
    inventory: &ControllerInventory,
    mapping: &ControllerMappingSettings,
) -> Vec<ControllerDevice> {
    let ids = ordered_controller_ids(inventory, mapping);
    let by_id = inventory
        .controllers
        .iter()
        .cloned()
        .map(|controller| (controller.stable_id.clone(), controller))
        .collect::<HashMap<_, _>>();
    ids.into_iter()
        .filter_map(|id| by_id.get(&id).cloned())
        .collect()
}

// Device names, USB IDs and enumeration indices cannot distinguish identical
// pads. Match the event's actual source, and refuse ambiguous portable keys.
pub fn controller_receives_input(
    controllers: &[ControllerDevice],
    controller_id: &str,
    key: &str,
) -> bool {
    if key.is_empty() {
        return false;
    }
    let path = Path::new(key);
    let mut matches = controllers.iter().filter(|device| {
        device.device_path == path || device.event_paths.iter().any(|event| event == path)
    });
    let first = matches.next();
    first.is_some_and(|device| device.stable_id == controller_id) && matches.next().is_none()
}

#[cfg(not(target_os = "linux"))]
pub fn portable_input_device_key(uuid: &str, name: &str) -> String {
    format!("gilrs://{uuid}/{}", slugify_id(name))
}

pub fn move_controller(
    inventory: &ControllerInventory,
    mapping: &mut ControllerMappingSettings,
    controller_id: &str,
    direction: isize,
) -> bool {
    let mut ids = ordered_controller_ids(inventory, mapping);
    let Some(index) = ids.iter().position(|id| id == controller_id) else {
        return false;
    };
    let target = if direction < 0 {
        index.checked_sub(1)
    } else {
        let next = index + 1;
        (next < ids.len()).then_some(next)
    };
    let Some(target) = target else {
        return false;
    };
    ids.swap(index, target);
    set_ordered_controller_ids(mapping, ids);
    true
}

pub fn controller_action(mapping: &ControllerMappingSettings, controller_id: &str) -> &'static str {
    if mapping
        .hidden_controller_ids
        .iter()
        .any(|saved| saved == controller_id)
    {
        return ACTION_HIDE;
    }
    if mapping.player_mappings.iter().any(|player| {
        player
            .controller_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|saved| saved == controller_id)
            && player.profile_id.as_deref().map(str::trim) == Some("none")
    }) {
        ACTION_PASSTHROUGH
    } else {
        ACTION_REMAP
    }
}

pub fn set_controller_action(
    mapping: &mut ControllerMappingSettings,
    controller_id: &str,
    action: &str,
) -> bool {
    if !matches!(action, ACTION_REMAP | ACTION_PASSTHROUGH | ACTION_HIDE) {
        return false;
    }
    let row = ensure_controller_row(mapping, controller_id);
    match action {
        ACTION_HIDE => {
            row.profile_id = Some("none".to_owned());
            if !mapping
                .hidden_controller_ids
                .iter()
                .any(|saved| saved == controller_id)
            {
                mapping.hidden_controller_ids.push(controller_id.to_owned());
            }
        }
        ACTION_PASSTHROUGH => {
            row.profile_id = Some("none".to_owned());
            mapping
                .hidden_controller_ids
                .retain(|saved| saved != controller_id);
        }
        ACTION_REMAP => {
            if row.profile_id.as_deref() == Some("none") {
                row.profile_id = None;
            }
            mapping
                .hidden_controller_ids
                .retain(|saved| saved != controller_id);
        }
        _ => unreachable!(),
    }
    mapping.enabled = true;
    mapping.manage_all = true;
    trim_default_player_mappings(mapping);
    true
}

pub fn controller_profile(mapping: &ControllerMappingSettings, controller_id: &str) -> String {
    match find_player_mapping(mapping, controller_id)
        .and_then(|player| player.profile_id.as_deref())
        .map(str::trim)
    {
        Some("") | None => PROFILE_INHERIT.to_owned(),
        Some("none") => PROFILE_NONE.to_owned(),
        Some(profile_id) => profile_id.to_owned(),
    }
}

pub fn set_controller_profile(
    mapping: &mut ControllerMappingSettings,
    controller_id: &str,
    profile_id: &str,
) -> bool {
    let profile_id = profile_id.trim();
    let valid = matches!(profile_id, "" | PROFILE_INHERIT | PROFILE_NONE)
        || profile_id == TWO_BUTTON_CLOCKWISE_PROFILE_ID
        || mapping
            .custom_profiles
            .iter()
            .any(|profile| profile.id == profile_id);
    if !valid {
        return false;
    }
    let row = ensure_controller_row(mapping, controller_id);
    row.profile_id = match profile_id {
        "" | PROFILE_INHERIT => None,
        PROFILE_NONE => Some("none".to_owned()),
        _ => Some(profile_id.to_owned()),
    };
    mapping
        .hidden_controller_ids
        .retain(|saved| saved != controller_id);
    mapping.enabled = true;
    mapping.manage_all = true;
    trim_default_player_mappings(mapping);
    true
}

pub fn controller_target(mapping: &ControllerMappingSettings, controller_id: &str) -> String {
    find_player_mapping(mapping, controller_id)
        .and_then(|player| player.output_target.as_deref())
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| TARGET_INHERIT.to_owned())
}

pub fn set_controller_target(
    inventory: &ControllerInventory,
    mapping: &mut ControllerMappingSettings,
    controller_id: &str,
    target: &str,
) -> bool {
    let target = target.trim();
    if target != TARGET_INHERIT
        && !inventory
            .supported_targets
            .iter()
            .any(|candidate| candidate.id == target)
    {
        return false;
    }
    let row = ensure_controller_row(mapping, controller_id);
    row.output_target = if target == TARGET_INHERIT || target.is_empty() {
        None
    } else {
        Some(target.to_owned())
    };
    mapping.enabled = true;
    mapping.manage_all = true;
    trim_default_player_mappings(mapping);
    true
}

fn find_player_mapping<'a>(
    mapping: &'a ControllerMappingSettings,
    controller_id: &str,
) -> Option<&'a ControllerPlayerMapping> {
    mapping.player_mappings.iter().find(|player| {
        player
            .controller_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|saved| saved == controller_id)
    })
}

fn ensure_controller_row<'a>(
    mapping: &'a mut ControllerMappingSettings,
    controller_id: &str,
) -> &'a mut ControllerPlayerMapping {
    let index = mapping.player_mappings.iter().position(|player| {
        player
            .controller_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|saved| saved == controller_id)
    });
    let index = index.unwrap_or_else(|| {
        mapping.player_mappings.push(ControllerPlayerMapping {
            controller_id: Some(controller_id.to_owned()),
            ..ControllerPlayerMapping::default()
        });
        mapping.player_mappings.len() - 1
    });
    &mut mapping.player_mappings[index]
}

fn ordered_controller_ids(
    inventory: &ControllerInventory,
    mapping: &ControllerMappingSettings,
) -> Vec<String> {
    let attached = inventory
        .controllers
        .iter()
        .map(|controller| controller.stable_id.as_str())
        .collect::<HashSet<_>>();
    let mut ordered = Vec::new();
    for player in &mapping.player_mappings {
        let Some(controller_id) = player.controller_id.as_deref().map(str::trim) else {
            continue;
        };
        if controller_id.is_empty()
            || ordered.iter().any(|id| id == controller_id)
            || !attached.contains(controller_id)
        {
            continue;
        }
        ordered.push(controller_id.to_owned());
    }
    for controller in &inventory.controllers {
        if !ordered.iter().any(|id| id == &controller.stable_id) {
            ordered.push(controller.stable_id.clone());
        }
    }
    ordered
}

fn set_ordered_controller_ids(mapping: &mut ControllerMappingSettings, ordered_ids: Vec<String>) {
    let saved_by_id = mapping
        .player_mappings
        .iter()
        .filter_map(|player| {
            player
                .controller_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(|id| (id.to_owned(), player.clone()))
        })
        .collect::<HashMap<_, _>>();
    let attached_ids = ordered_ids.iter().cloned().collect::<HashSet<_>>();
    let mut next = ordered_ids
        .into_iter()
        .map(|controller_id| {
            let mut row = saved_by_id.get(&controller_id).cloned().unwrap_or_default();
            row.controller_id = Some(controller_id);
            row
        })
        .collect::<Vec<_>>();
    next.extend(
        mapping
            .player_mappings
            .iter()
            .filter(|player| {
                player
                    .controller_id
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|id| !id.is_empty() && !attached_ids.contains(id))
            })
            .cloned(),
    );
    mapping.profile_controller_ids.clear();
    mapping.player_mappings = next;
    trim_default_player_mappings(mapping);
}

fn trim_default_player_mappings(mapping: &mut ControllerMappingSettings) {
    while mapping.player_mappings.last().is_some_and(|player| {
        player
            .controller_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
            && player
                .profile_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            && player
                .output_target
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
    }) {
        mapping.player_mappings.pop();
    }
}

pub(crate) fn list_local_controllers(warnings: &mut Vec<String>) -> Vec<ControllerDevice> {
    #[cfg(target_os = "linux")]
    {
        list_linux_joystick_controllers(warnings)
    }
    #[cfg(not(target_os = "linux"))]
    {
        list_gilrs_controllers(warnings)
    }
}

#[cfg(not(target_os = "linux"))]
fn list_gilrs_controllers(warnings: &mut Vec<String>) -> Vec<ControllerDevice> {
    let gilrs = match gilrs::GilrsBuilder::new()
        .with_force_feedback(false)
        .build()
    {
        Ok(gilrs) => gilrs,
        Err(error) => {
            warnings.push(format!(
                "Could not initialize native gamepad discovery on {}: {error}",
                std::env::consts::OS
            ));
            return Vec::new();
        }
    };

    // GilRs exposes a portable SDL-style UUID, but that UUID identifies a
    // controller model rather than a physical instance. Keep the name in the
    // identity and add an ordinal only when identical devices are connected.
    let mut occurrences = HashMap::<String, usize>::new();
    let mut devices = gilrs
        .gamepads()
        .filter_map(|(_, gamepad)| {
            let name = gamepad.name().trim();
            if name.is_empty() || is_likely_non_game_controller(name) {
                return None;
            }
            let uuid_hex = hex::encode(gamepad.uuid());
            let vendor_id = gamepad.vendor_id().map(|value| format!("{value:04x}"));
            let product_id = gamepad.product_id().map(|value| format!("{value:04x}"));
            let base_identity = portable_controller_identity(&uuid_hex, gamepad.os_name());
            let base_id =
                stable_controller_id(vendor_id.as_deref(), product_id.as_deref(), &base_identity);
            let occurrence = occurrences.entry(base_id.clone()).or_default();
            *occurrence += 1;
            let stable_id = if *occurrence == 1 {
                base_id
            } else {
                format!("{base_id}-instance-{occurrence}")
            };
            Some(ControllerDevice {
                stable_id,
                name: name.to_owned(),
                // The synthetic URI deliberately avoids leaking or assuming a
                // host-specific device path. It is never passed to a backend
                // that expects a native path on non-Linux hosts.
                device_path: PathBuf::from(portable_input_device_key(&uuid_hex, name)),
                event_paths: Vec::new(),
                vendor_id,
                product_id,
                version: None,
                bus_type: None,
                physical_path: None,
                // GilRs documents its UUID as a model identifier, so do not
                // present it as a unique physical-device ID in the UI.
                unique_id: None,
                is_virtual: name.to_ascii_lowercase().contains("virtual"),
            })
        })
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    devices
}

#[cfg(not(target_os = "linux"))]
fn portable_controller_identity(uuid_hex: &str, os_name: &str) -> String {
    format!("gilrs-{uuid_hex}-{}", slugify_id(os_name))
}

#[cfg(target_os = "linux")]
fn list_linux_joystick_controllers(warnings: &mut Vec<String>) -> Vec<ControllerDevice> {
    let sys_input = Path::new("/sys/class/input");
    let entries = match std::fs::read_dir(sys_input) {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(format!("Could not read {}: {error}", sys_input.display()));
            return Vec::new();
        }
    };
    let mut devices = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with("js")
                .then(|| linux_controller_from_js(&name, &entry.path()))
                .flatten()
        })
        .filter(should_show_linux_controller)
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| left.device_path.cmp(&right.device_path));
    devices
}

#[cfg(target_os = "linux")]
fn linux_controller_from_js(js_name: &str, js_sys_path: &Path) -> Option<ControllerDevice> {
    let device_dir = js_sys_path.join("device");
    let name = read_trimmed(device_dir.join("name")).unwrap_or_else(|| js_name.to_owned());
    if is_likely_non_game_controller(&name) {
        return None;
    }
    let vendor_id = read_trimmed(device_dir.join("id/vendor"));
    let product_id = read_trimmed(device_dir.join("id/product"));
    let version = read_trimmed(device_dir.join("id/version"));
    let bus_type = read_trimmed(device_dir.join("id/bustype"));
    let physical_path = read_trimmed(device_dir.join("phys"));
    let unique_id = read_trimmed(device_dir.join("uniq"));
    let canonical_device_dir = std::fs::canonicalize(&device_dir).ok();
    let stable_identity = unique_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(physical_path.as_deref())
        .map(ToOwned::to_owned)
        .or({
            canonical_device_dir
                .as_deref()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| format!("/dev/input/{js_name}"));
    let is_virtual = is_virtual_linux_controller(
        &name,
        physical_path.as_deref(),
        canonical_device_dir.as_deref(),
        bus_type.as_deref(),
    );
    Some(ControllerDevice {
        stable_id: stable_controller_id(
            vendor_id.as_deref(),
            product_id.as_deref(),
            &stable_identity,
        ),
        name,
        device_path: PathBuf::from("/dev/input").join(js_name),
        event_paths: linux_input_child_paths(&device_dir, "event"),
        vendor_id,
        product_id,
        version,
        bus_type: bus_type.clone(),
        physical_path: physical_path.clone(),
        unique_id,
        is_virtual,
    })
}

#[cfg(target_os = "linux")]
fn is_virtual_linux_controller(
    name: &str,
    physical_path: Option<&str>,
    canonical_device_dir: Option<&Path>,
    bus_type: Option<&str>,
) -> bool {
    let name = name.to_ascii_lowercase();
    if name.contains("virtual") || name.contains("inputplumber") {
        return true;
    }
    if physical_path.is_some_and(|path| path.to_ascii_lowercase().contains("virtual")) {
        return true;
    }
    let Some(device_dir) = canonical_device_dir else {
        return false;
    };
    let device_dir = device_dir.to_string_lossy().to_ascii_lowercase();
    if device_dir.contains("/devices/virtual/input/") {
        return true;
    }
    device_dir.contains("/devices/virtual/misc/uhid/")
        && !bus_type.is_some_and(|bus| bus.eq_ignore_ascii_case("0005"))
}

#[cfg(target_os = "linux")]
fn should_show_linux_controller(device: &ControllerDevice) -> bool {
    !device.is_virtual || is_steam_input_virtual_gamepad(device)
}

#[cfg(target_os = "linux")]
fn is_steam_input_virtual_gamepad(device: &ControllerDevice) -> bool {
    let valve = device
        .vendor_id
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("28de"));
    let steam_input = device
        .product_id
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("11ff"));
    if !valve || !steam_input {
        return false;
    }
    let name = device.name.to_ascii_lowercase();
    ["x-box", "xbox", "gamepad", "controller", "pad"]
        .iter()
        .any(|token| name.contains(token))
}

fn is_likely_non_game_controller(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "keyboard",
        "mouse",
        "pointer",
        "trackpad",
        "touchpad",
        "motion sensors",
        "headset jack",
        "led controller",
        "chakram",
    ]
    .iter()
    .any(|token| name.contains(token))
}

#[cfg(target_os = "linux")]
fn linux_input_child_paths(device_dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(device_dir) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with(prefix)
                .then(|| PathBuf::from("/dev/input").join(name))
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn stable_controller_id(
    vendor_id: Option<&str>,
    product_id: Option<&str>,
    identity: &str,
) -> String {
    let vendor = vendor_id.unwrap_or("0000").to_ascii_lowercase();
    let product = product_id.unwrap_or("0000").to_ascii_lowercase();
    format!(
        "{}:{vendor}:{product}:{}",
        std::env::consts::OS,
        slugify_id(identity)
    )
}

fn slugify_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn inputplumber_inventory(
    warnings: &mut Vec<String>,
) -> (ControllerProviderStatus, usize, Vec<ControllerTarget>) {
    if !cfg!(target_os = "linux") {
        return (
            ControllerProviderStatus {
                provider: "native".to_owned(),
                available: true,
                service_accessible: true,
                message: Some(
                    "Native gamepad discovery is available; launch-time remapping requires the Linux InputPlumber adapter."
                        .to_owned(),
                ),
                ..ControllerProviderStatus::default()
            },
            0,
            Vec::new(),
        );
    }
    let version = run_command_with_timeout("inputplumber", &["--version"], COMMAND_TIMEOUT).ok();
    let Some(version) = version else {
        return (
            ControllerProviderStatus {
                provider: "inputplumber".to_owned(),
                message: Some("InputPlumber is not available on PATH.".to_owned()),
                ..ControllerProviderStatus::default()
            },
            0,
            Vec::new(),
        );
    };
    let device_output = run_inputplumber_devices_list();
    let managed_device_count = match &device_output {
        Ok(output) => parse_box_rows(output)
            .into_iter()
            .filter(|row| row.first().is_some_and(|value| value != "Id"))
            .count(),
        Err(error) => {
            warnings.push(error.clone());
            0
        }
    };
    let targets = match run_command_with_timeout(
        "inputplumber",
        &["targets", "supported-devices"],
        COMMAND_TIMEOUT,
    ) {
        Ok(output) => parse_box_rows(&output)
            .into_iter()
            .filter_map(|row| {
                (row.len() >= 2 && row[0] != "Id").then(|| ControllerTarget {
                    id: row[0].clone(),
                    name: row[1].clone(),
                })
            })
            .collect(),
        Err(error) => {
            warnings.push(error);
            Vec::new()
        }
    };
    (
        ControllerProviderStatus {
            provider: "inputplumber".to_owned(),
            available: true,
            version: Some(version),
            service_accessible: device_output.is_ok(),
            message: None,
        },
        managed_device_count,
        targets,
    )
}

fn run_inputplumber_devices_list() -> Result<String, String> {
    let mut last_error = None;
    for attempt in 0..=DEVICE_LIST_RETRIES {
        match run_command_with_timeout("inputplumber", &["devices", "list"], DEVICE_LIST_TIMEOUT) {
            Ok(output) => return Ok(output),
            Err(error) => {
                let timed_out = error.contains("timed out");
                last_error = Some(error);
                if !timed_out || attempt == DEVICE_LIST_RETRIES {
                    break;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| "inputplumber devices list failed".to_owned()))
}

fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String, String> {
    let started = Instant::now();
    let mut child = host_command(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not run {program}: {error}"))?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = read_child_pipe(child.stdout.take());
                let stderr = read_child_pipe(child.stderr.take());
                if status.success() {
                    return Ok(stdout.trim().to_owned());
                }
                let detail = if stderr.trim().is_empty() {
                    stdout.trim()
                } else {
                    stderr.trim()
                };
                return Err(format!("{program} {} failed: {detail}", args.join(" ")));
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{program} {} timed out after {} ms",
                    args.join(" "),
                    timeout.as_millis()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(format!("Could not wait for {program}: {error}")),
        }
    }
}

fn read_child_pipe<T: Read>(pipe: Option<T>) -> String {
    let Some(mut pipe) = pipe else {
        return String::new();
    };
    let mut bytes = Vec::new();
    let _ = pipe.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn parse_box_rows(output: &str) -> Vec<Vec<String>> {
    output
        .lines()
        .filter(|line| line.contains('│'))
        .filter_map(|line| {
            let cells = line
                .split('│')
                .skip(1)
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            (cells.len() >= 2).then_some(cells)
        })
        .collect()
}

pub fn activate_for_launch(
    settings: &AppSettings,
    platform_name: Option<&str>,
    launchbox_db_id: Option<i64>,
) -> Result<ControllerActivation, String> {
    let mapping = &settings.controller_mapping;
    if !mapping.enabled {
        return Ok(ControllerActivation::default());
    }
    if !cfg!(target_os = "linux") {
        return Ok(ControllerActivation {
            warning: Some(format!(
                "Controller launch remapping is not yet available on {}.",
                std::env::consts::OS
            )),
            ..ControllerActivation::default()
        });
    }
    if !matches!(mapping.provider.trim(), "" | "auto" | "inputplumber") {
        return Ok(ControllerActivation {
            warning: Some(format!(
                "Controller provider {:?} is not supported; the game was launched without remapping.",
                mapping.provider
            )),
            ..ControllerActivation::default()
        });
    }
    if run_command_with_timeout("inputplumber", &["--version"], LAUNCH_QUERY_TIMEOUT).is_err() {
        return Ok(ControllerActivation {
            warning: Some(
                "InputPlumber is unavailable; the game was launched without controller remapping."
                    .to_owned(),
            ),
            ..ControllerActivation::default()
        });
    }

    let mut inventory_warnings = Vec::new();
    let controllers = list_local_controllers(&mut inventory_warnings)
        .into_iter()
        .filter(|controller| !controller.is_virtual)
        .collect::<Vec<_>>();
    let managed_devices = match inputplumber_managed_devices_for_launch() {
        Ok(devices) if !devices.is_empty() => devices,
        Ok(_) => {
            return Ok(ControllerActivation {
                warning: Some(
                    "InputPlumber has no managed composite devices; the game was launched without controller remapping."
                        .to_owned(),
                ),
                ..ControllerActivation::default()
            });
        }
        Err(error) => {
            return Ok(ControllerActivation {
                warning: Some(format!(
                    "InputPlumber could not list managed devices ({error}); the game was launched without controller remapping."
                )),
                ..ControllerActivation::default()
            });
        }
    };

    let automatic_mapping =
        automatic_player_mappings(mapping, &controllers, platform_name, launchbox_db_id);
    let mapping = &automatic_mapping;
    let inherited_profile = resolve_profile_id(mapping, platform_name, launchbox_db_id);
    let active_players = resolve_active_player_mappings(
        settings,
        mapping,
        &controllers,
        inherited_profile.as_deref(),
    )?;
    let hidden_paths =
        selected_controller_source_paths(&controllers, &mapping.hidden_controller_ids);
    let restore_order = managed_devices
        .iter()
        .map(|device| device.id.clone())
        .collect::<Vec<_>>();
    let mut used_device_ids = HashSet::new();
    let mut launch_devices = Vec::new();

    if !active_players.is_empty() {
        for player in active_players {
            let Some(device) = managed_devices.iter().find(|device| {
                !used_device_ids.contains(&device.id)
                    && device
                        .source_paths
                        .iter()
                        .any(|path| player.source_paths.contains(path))
            }) else {
                continue;
            };
            used_device_ids.insert(device.id.clone());
            launch_devices.push(InputPlumberLaunchDevice {
                id: device.id.clone(),
                hidden: device
                    .source_paths
                    .iter()
                    .any(|path| hidden_paths.contains(path)),
                target_ids: player.target_ids,
                profile_path: player.profile_path,
            });
        }
    } else {
        let profile_paths = if mapping
            .profile_controller_ids
            .iter()
            .all(|id| id.trim().is_empty())
        {
            controller_source_paths(&controllers)
        } else {
            selected_controller_source_paths(&controllers, &mapping.profile_controller_ids)
        };
        let profile_path = inherited_profile
            .as_deref()
            .map(|profile| resolve_profile_path(settings, profile))
            .transpose()?;
        for device in &managed_devices {
            let hidden = device
                .source_paths
                .iter()
                .any(|path| hidden_paths.contains(path));
            let profiled = device
                .source_paths
                .iter()
                .any(|path| profile_paths.contains(path));
            if !hidden && !profiled {
                continue;
            }
            used_device_ids.insert(device.id.clone());
            launch_devices.push(InputPlumberLaunchDevice {
                id: device.id.clone(),
                hidden,
                target_ids: normalize_target_ids(&mapping.output_target),
                profile_path: profile_path.clone(),
            });
        }
    }

    for device in managed_devices {
        if used_device_ids.contains(&device.id)
            || !device
                .source_paths
                .iter()
                .any(|path| hidden_paths.contains(path))
        {
            continue;
        }
        launch_devices.push(InputPlumberLaunchDevice {
            id: device.id,
            hidden: true,
            target_ids: Vec::new(),
            profile_path: None,
        });
    }

    if launch_devices.is_empty() {
        return Ok(ControllerActivation {
            warning: Some(
                "No InputPlumber managed device matched the saved physical controllers; the game was launched without remapping."
                    .to_owned(),
            ),
            ..ControllerActivation::default()
        });
    }
    let session = apply_inputplumber_launch_devices(launch_devices, restore_order)?;
    Ok(ControllerActivation {
        session,
        warning: inventory_warnings.into_iter().next(),
    })
}

fn resolve_active_player_mappings(
    settings: &AppSettings,
    mapping: &ControllerMappingSettings,
    controllers: &[ControllerDevice],
    inherited_profile: Option<&str>,
) -> Result<Vec<ResolvedPlayerMapping>, String> {
    mapping
        .player_mappings
        .iter()
        .filter_map(|player| {
            player
                .controller_id
                .as_deref()
                .map(str::trim)
                .filter(|controller_id| !controller_id.is_empty())
                .map(|controller_id| (player, controller_id))
        })
        .filter_map(|(player, controller_id)| {
            let source_paths = if controller_id == CONTROLLER_SCOPE_ALL {
                controller_source_paths(controllers)
            } else {
                selected_controller_source_paths(controllers, &[controller_id.to_owned()])
            };
            if source_paths.is_empty() {
                return None;
            }
            let target = player
                .output_target
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&mapping.output_target);
            let profile = match player.profile_id.as_deref().map(str::trim) {
                Some("") | None => inherited_profile,
                Some("none") => None,
                Some(profile) => Some(profile),
            };
            Some(
                profile
                    .map(|profile| resolve_profile_path(settings, profile))
                    .transpose()
                    .map(|profile_path| ResolvedPlayerMapping {
                        source_paths,
                        target_ids: normalize_target_ids(target),
                        profile_path,
                    }),
            )
        })
        .collect()
}

fn resolve_profile_id(
    mapping: &ControllerMappingSettings,
    platform_name: Option<&str>,
    launchbox_db_id: Option<i64>,
) -> Option<String> {
    if let Some(database_id) = launchbox_db_id
        && let Some(profile) = mapping.game_profile_ids.get(&database_id.to_string())
    {
        return normalized_profile_id(profile);
    }
    if let Some(platform) = platform_name
        && let Some(profile) = mapping.platform_profile_ids.get(platform)
    {
        return normalized_profile_id(profile);
    }
    mapping
        .default_profile_id
        .as_deref()
        .and_then(normalized_profile_id)
}

fn normalized_profile_id(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value != "none").then(|| value.to_owned())
}

fn resolve_profile_path(settings: &AppSettings, profile_id: &str) -> Result<PathBuf, String> {
    match profile_id {
        "horizontal-ab-swap" | "nintendo-label-swap" => {
            let mut mappings = vec![
                ControllerButtonMapping {
                    source_button: "South".into(),
                    target_button: "East".into(),
                },
                ControllerButtonMapping {
                    source_button: "East".into(),
                    target_button: "South".into(),
                },
            ];
            if profile_id == "nintendo-label-swap" {
                mappings.extend([
                    ControllerButtonMapping {
                        source_button: "North".into(),
                        target_button: "West".into(),
                    },
                    ControllerButtonMapping {
                        source_button: "West".into(),
                        target_button: "North".into(),
                    },
                ]);
            }
            let profile = ControllerCustomProfile {
                id: profile_id.into(),
                name: profile_id.into(),
                layout: "generic".into(),
                mappings,
            };
            write_controller_profile(
                &format!("{profile_id}.yaml"),
                &custom_profile_yaml(&profile),
            )
        }
        TWO_BUTTON_CLOCKWISE_PROFILE_ID => {
            write_controller_profile("two-button-clockwise.yaml", TWO_BUTTON_CLOCKWISE_PROFILE)
        }
        profile_id if profile_id.starts_with(CUSTOM_PROFILE_PREFIX) => {
            let profile = settings
                .controller_mapping
                .custom_profiles
                .iter()
                .find(|profile| profile.id == profile_id)
                .ok_or_else(|| {
                    format!("Custom controller profile {profile_id:?} was not found.")
                })?;
            validate_custom_profile(profile)?;
            let file_name = format!(
                "{}.yaml",
                sanitize_profile_file_name(profile_id.trim_start_matches(CUSTOM_PROFILE_PREFIX))
            );
            write_controller_profile(&file_name, &custom_profile_yaml(profile))
        }
        path => {
            let path = PathBuf::from(path);
            if path.is_file() {
                Ok(path)
            } else {
                Err(format!(
                    "Controller profile {profile_id:?} is neither built in nor a readable file."
                ))
            }
        }
    }
}

fn write_controller_profile(file_name: &str, contents: &str) -> Result<PathBuf, String> {
    let project = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .ok_or_else(|| "Could not determine the application data directory.".to_owned())?;
    let directory = project.data_local_dir().join("controller-profiles");
    std::fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create controller profile directory {}: {error}",
            directory.display()
        )
    })?;
    let path = directory.join(file_name);
    if std::fs::read_to_string(&path).ok().as_deref() != Some(contents) {
        std::fs::write(&path, contents).map_err(|error| {
            format!(
                "Could not write controller profile {}: {error}",
                path.display()
            )
        })?;
    }
    Ok(path)
}

fn validate_custom_profile(profile: &ControllerCustomProfile) -> Result<(), String> {
    if profile.name.trim().is_empty() {
        return Err("Custom controller profile name is empty.".to_owned());
    }
    for mapping in &profile.mappings {
        if !CONTROLLER_GAMEPAD_BUTTONS.contains(&mapping.source_button.as_str())
            || !CONTROLLER_GAMEPAD_BUTTONS.contains(&mapping.target_button.as_str())
        {
            return Err(format!(
                "Custom controller profile {:?} contains an unsupported button mapping.",
                profile.name
            ));
        }
    }
    Ok(())
}

fn custom_profile_yaml(profile: &ControllerCustomProfile) -> String {
    let mut yaml = format!(
        "version: 1\nkind: DeviceProfile\nname: {}\ndescription: \"Created by Lunchbox.\"\n",
        yaml_quote(&profile.name)
    );
    let mappings = profile
        .mappings
        .iter()
        .filter(|mapping| mapping.source_button != mapping.target_button)
        .collect::<Vec<_>>();
    if mappings.is_empty() {
        yaml.push_str("mapping: []\n");
        return yaml;
    }
    yaml.push_str("mapping:\n");
    for mapping in mappings {
        yaml.push_str(&format!(
            "  - name: {} to {}\n    source_event:\n      gamepad:\n{}    target_events:\n      - gamepad:\n{}",
            mapping.source_button,
            mapping.target_button,
            profile_event_yaml(&mapping.source_button, 8),
            profile_event_yaml(&mapping.target_button, 10)
        ));
    }
    yaml
}

fn profile_event_yaml(control: &str, indent: usize) -> String {
    let padding = " ".repeat(indent);
    let axis = if control.starts_with("LeftStick") {
        "LeftStick"
    } else {
        "RightStick"
    };
    if let Some(direction) = control.strip_prefix(axis)
        && matches!(direction, "Up" | "Down" | "Left" | "Right")
    {
        format!(
            "{padding}axis:\n{padding}  name: {axis}\n{padding}  direction: {}\n{padding}  deadzone: 0.5\n",
            direction.to_ascii_lowercase()
        )
    } else {
        format!("{padding}button: {control}\n")
    }
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn sanitize_profile_file_name(value: &str) -> String {
    let value = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let value = value.trim_matches('-');
    if value.is_empty() {
        "custom-profile".to_owned()
    } else {
        value.to_owned()
    }
}

fn normalize_target_ids(output_target: &str) -> Vec<String> {
    let targets = output_target
        .split(',')
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if targets.is_empty() {
        vec!["xb360".to_owned()]
    } else {
        targets
    }
}

fn controller_source_paths(controllers: &[ControllerDevice]) -> HashSet<PathBuf> {
    controllers
        .iter()
        .flat_map(|controller| {
            std::iter::once(controller.device_path.clone())
                .chain(controller.event_paths.iter().cloned())
        })
        .collect()
}

fn selected_controller_source_paths(
    controllers: &[ControllerDevice],
    selected_ids: &[String],
) -> HashSet<PathBuf> {
    let selected_ids = selected_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .collect::<HashSet<_>>();
    controllers
        .iter()
        .filter(|controller| selected_ids.contains(controller.stable_id.as_str()))
        .flat_map(|controller| {
            std::iter::once(controller.device_path.clone())
                .chain(controller.event_paths.iter().cloned())
        })
        .collect()
}

fn inputplumber_managed_devices_for_launch() -> Result<Vec<InputPlumberManagedDevice>, String> {
    let output =
        run_command_with_timeout("inputplumber", &["devices", "list"], LAUNCH_QUERY_TIMEOUT)?;
    parse_box_rows(&output)
        .into_iter()
        .filter(|row| row.len() >= 2 && row[0] != "Id")
        .map(|row| {
            let id = row[0].clone();
            let info = run_command_with_timeout(
                "inputplumber",
                &["device", &id, "info"],
                LAUNCH_QUERY_TIMEOUT,
            )?;
            Ok(InputPlumberManagedDevice {
                id,
                source_paths: extract_quoted_paths(&info),
            })
        })
        .collect()
}

fn extract_quoted_paths(output: &str) -> HashSet<PathBuf> {
    let mut paths = HashSet::new();
    let mut in_quote = false;
    let mut current = String::new();
    for character in output.chars() {
        if character == '"' {
            if in_quote && current.starts_with("/dev/") {
                paths.insert(PathBuf::from(&current));
            }
            current.clear();
            in_quote = !in_quote;
        } else if in_quote {
            current.push(character);
        }
    }
    paths
}

fn apply_inputplumber_launch_devices(
    devices: Vec<InputPlumberLaunchDevice>,
    restore_order: Vec<String>,
) -> Result<ControllerLaunchSession, String> {
    let mut session = ControllerLaunchSession {
        restore_entries: Vec::new(),
        restore_order,
    };
    let order = devices
        .iter()
        .filter(|device| !device.hidden)
        .map(|device| device.id.clone())
        .collect::<Vec<_>>();
    if order.len() > 1 {
        run_inputplumber_dynamic(["devices", "order"], &order, LAUNCH_APPLY_TIMEOUT)?;
    }
    for device in devices {
        let restore = capture_restore_entry(&device)?;
        session.restore_entries.push(restore);
        run_command_with_timeout(
            "inputplumber",
            &["device", &device.id, "intercept", "set", "gamepad-only"],
            LAUNCH_APPLY_TIMEOUT,
        )?;
        if device.hidden {
            run_command_with_timeout(
                "inputplumber",
                &["device", &device.id, "targets", "set", "null"],
                LAUNCH_APPLY_TIMEOUT,
            )?;
        } else {
            run_inputplumber_dynamic(
                ["device", &device.id, "targets", "set"],
                &device.target_ids,
                LAUNCH_APPLY_TIMEOUT,
            )?;
            if let Some(profile_path) = device.profile_path {
                run_command_with_timeout(
                    "inputplumber",
                    &[
                        "device",
                        &device.id,
                        "profile",
                        "load",
                        profile_path.to_string_lossy().as_ref(),
                    ],
                    LAUNCH_APPLY_TIMEOUT,
                )?;
            }
        }
    }
    Ok(session)
}

fn capture_restore_entry(
    device: &InputPlumberLaunchDevice,
) -> Result<InputPlumberRestoreEntry, String> {
    let intercept = run_command_with_timeout(
        "inputplumber",
        &["device", &device.id, "intercept", "get"],
        LAUNCH_QUERY_TIMEOUT,
    )?;
    let intercept_mode = intercept
        .split(':')
        .nth(1)
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            format!(
                "Could not capture InputPlumber intercept mode for device {}.",
                device.id
            )
        })?;
    let targets = run_command_with_timeout(
        "inputplumber",
        &["device", &device.id, "targets", "list"],
        LAUNCH_QUERY_TIMEOUT,
    )?;
    let target_ids = parse_box_rows(&targets)
        .into_iter()
        .filter(|row| row.len() >= 2 && row[0] != "Id")
        .map(|row| row[0].clone())
        .collect::<Vec<_>>();
    let profile_path = if device.profile_path.is_some() {
        let output = run_command_with_timeout(
            "inputplumber",
            &["device", &device.id, "profile", "path"],
            LAUNCH_QUERY_TIMEOUT,
        )?;
        let path = output
            .split_whitespace()
            .find(|part| part.starts_with('/'))
            .map(|part| PathBuf::from(part.trim_matches('"')))
            .filter(|path| path.is_file());
        if path.is_none() {
            return Err(format!(
                "InputPlumber did not report a restorable profile path for device {}; no controller changes were applied.",
                device.id
            ));
        }
        path
    } else {
        None
    };
    Ok(InputPlumberRestoreEntry {
        device_id: device.id.clone(),
        intercept_mode,
        profile_path,
        target_ids,
    })
}

fn run_inputplumber_dynamic<const N: usize>(
    prefix: [&str; N],
    values: &[String],
    timeout: Duration,
) -> Result<String, String> {
    let args = prefix
        .iter()
        .map(|value| (*value).to_owned())
        .chain(values.iter().cloned())
        .collect::<Vec<_>>();
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_command_with_timeout("inputplumber", &args, timeout)
}

impl ControllerLaunchSession {
    fn restore_now(&mut self) {
        for entry in self.restore_entries.drain(..).rev() {
            let targets = if entry.target_ids.is_empty() {
                vec!["null".to_owned()]
            } else {
                entry.target_ids
            };
            let _ = run_inputplumber_dynamic(
                ["device", &entry.device_id, "targets", "set"],
                &targets,
                COMMAND_TIMEOUT,
            );
            if let Some(profile_path) = entry.profile_path {
                let _ = run_command_with_timeout(
                    "inputplumber",
                    &[
                        "device",
                        &entry.device_id,
                        "profile",
                        "load",
                        profile_path.to_string_lossy().as_ref(),
                    ],
                    COMMAND_TIMEOUT,
                );
            }
            let _ = run_command_with_timeout(
                "inputplumber",
                &[
                    "device",
                    &entry.device_id,
                    "intercept",
                    "set",
                    &entry.intercept_mode,
                ],
                COMMAND_TIMEOUT,
            );
        }
        if self.restore_order.len() > 1 {
            let restore_order = std::mem::take(&mut self.restore_order);
            let _ = run_inputplumber_dynamic(["devices", "order"], &restore_order, COMMAND_TIMEOUT);
        } else {
            self.restore_order.clear();
        }
    }
}

impl Drop for ControllerLaunchSession {
    fn drop(&mut self) {
        self.restore_now();
    }
}

const TWO_BUTTON_CLOCKWISE_PROFILE: &str = r#"version: 1
kind: DeviceProfile
name: 2-button X/A diamond
description: Maps NES-style run/jump to the physical X/A face-button positions.
mapping:
  - name: Physical South to target South
    source_event:
      gamepad:
        button: South
    target_events:
      - gamepad:
          button: South
  - name: Physical East to target West
    source_event:
      gamepad:
        button: East
    target_events:
      - gamepad:
          button: West
  - name: Physical North to target North
    source_event:
      gamepad:
        button: North
    target_events:
      - gamepad:
          button: North
  - name: Physical West to target East
    source_event:
      gamepad:
        button: West
    target_events:
      - gamepad:
          button: East
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn controller(id: &str, name: &str) -> ControllerDevice {
        ControllerDevice {
            stable_id: id.to_owned(),
            name: name.to_owned(),
            device_path: PathBuf::from(format!("/dev/input/{id}")),
            event_paths: Vec::new(),
            vendor_id: None,
            product_id: None,
            version: None,
            bus_type: None,
            physical_path: None,
            unique_id: None,
            is_virtual: false,
        }
    }

    #[test]
    fn automatic_layouts_choose_connected_horizontal_pad_without_double_swapping() {
        let pads = vec![
            controller("ps5", "DualSense"),
            controller("n30", "8BitDo N30"),
        ];
        let mut mapping = ControllerMappingSettings {
            automatic: true,
            ..Default::default()
        };
        mapping
            .device_layouts
            .insert("n30".into(), "horizontal".into());
        let resolved =
            automatic_player_mappings(&mapping, &pads, Some("Nintendo Entertainment System"), None);
        assert_eq!(
            resolved.player_mappings[0].controller_id.as_deref(),
            Some("n30")
        );
        assert_eq!(resolved.player_mappings[0].profile_id, None);
        assert_eq!(
            resolved.player_mappings[1].profile_id.as_deref(),
            Some(TWO_BUTTON_CLOCKWISE_PROFILE_ID)
        );
        mapping
            .device_layouts
            .insert("n30".into(), "horizontal-swapped".into());
        let resolved = automatic_player_mappings(&mapping, &pads, Some("NES"), None);
        assert_eq!(
            resolved.player_mappings[0].profile_id.as_deref(),
            Some("horizontal-ab-swap")
        );
    }

    #[test]
    fn automatic_profiles_respect_explicit_game_overrides_and_disconnected_preferences() {
        let pads = vec![controller("ps5", "DualSense")];
        let mut mapping = ControllerMappingSettings {
            automatic: true,
            ..Default::default()
        };
        mapping
            .preferred_devices
            .insert("two-button".into(), "unplugged".into());
        mapping.game_profile_ids.insert("525".into(), "none".into());
        let resolved = automatic_player_mappings(&mapping, &pads, Some("NES"), Some(525));
        assert_eq!(
            resolved.player_mappings[0].controller_id.as_deref(),
            Some("ps5")
        );
        assert_eq!(resolved.player_mappings[0].profile_id, None);
        assert_eq!(mapping.preferred_devices["two-button"], "unplugged");
    }

    #[test]
    fn n64_and_sega_can_use_different_profiles_on_the_same_controller() {
        let pads = vec![controller("n64", "Brawler64")];
        let mut mapping = ControllerMappingSettings {
            automatic: true,
            ..Default::default()
        };
        mapping.device_system_profiles.insert(
            "n64".into(),
            HashMap::from([
                ("n64".into(), "custom:n64".into()),
                ("six-button".into(), "custom:sega".into()),
            ]),
        );
        for (system, profile) in [
            ("Nintendo 64", "custom:n64"),
            ("Sega Genesis", "custom:sega"),
        ] {
            let resolved = automatic_player_mappings(&mapping, &pads, Some(system), None);
            assert_eq!(
                resolved.player_mappings[0].profile_id.as_deref(),
                Some(profile)
            );
        }
        assert_eq!(detected_device_layout(&pads[0]), "auto");
    }

    #[test]
    fn generic_xbox_identity_does_not_prove_a_diamond_layout() {
        for version in ["0110", "0114"] {
            let mut pad = controller("usb-pad", "Microsoft X-Box 360 pad");
            pad.vendor_id = Some("045e".into());
            pad.product_id = Some("028e".into());
            pad.version = Some(version.into());
            assert_eq!(detected_device_layout(&pad), "auto");
            let mut mapping = ControllerMappingSettings::default();
            mapping
                .device_layouts
                .insert(pad.stable_id.clone(), "horizontal".into());
            assert_eq!(device_layout(&mapping, &pad), "horizontal");
        }
        let mut steam = controller("steam", "Microsoft X-Box 360 pad 0");
        steam.vendor_id = Some("28de".into());
        steam.product_id = Some("11ff".into());
        assert_eq!(detected_device_layout(&steam), "diamond");
    }

    #[test]
    fn input_feedback_matches_exact_device_not_identical_names_or_ids() {
        let mut first = controller("first", "Microsoft X-Box 360 pad");
        first.event_paths = vec![PathBuf::from("/dev/input/event259")];
        let mut second = controller("second", "Microsoft X-Box 360 pad");
        second.event_paths = vec![PathBuf::from("/dev/input/event264")];
        let pads = vec![first, second];
        assert!(controller_receives_input(
            &pads,
            "first",
            "/dev/input/event259"
        ));
        assert!(!controller_receives_input(
            &pads,
            "second",
            "/dev/input/event259"
        ));
        assert!(controller_receives_input(
            &pads,
            "second",
            "/dev/input/event264"
        ));
        assert!(!controller_receives_input(&pads, "first", ""));
        assert!(!controller_receives_input(
            &pads[..1],
            "first",
            "/dev/input/event264"
        ));
        let mut ambiguous = pads.clone();
        ambiguous[1].event_paths = ambiguous[0].event_paths.clone();
        assert!(!controller_receives_input(
            &ambiguous,
            "first",
            "/dev/input/event259"
        ));
    }

    #[test]
    fn controller_aliases_roundtrip_and_validate_without_changing_identity() {
        let mut mapping = ControllerMappingSettings::default();
        mapping.device_names.insert("first".into(), "N30".into());
        mapping
            .device_names
            .insert("second".into(), "Retro Fighters".into());
        mapping.validate().unwrap();
        let json = serde_json::to_string(&mapping).unwrap();
        let restored: ControllerMappingSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.device_names["first"], "N30");
        assert_eq!(restored.device_names["second"], "Retro Fighters");
        mapping
            .device_names
            .insert("first".into(), "bad\nname".into());
        assert!(mapping.validate().is_err());
        mapping.device_names.insert("first".into(), "x".repeat(81));
        assert!(mapping.validate().is_err());
        assert!(profile_event_yaml("LeftStickUp", 2).contains("name: LeftStick"));
    }

    #[test]
    fn system_classification_does_not_treat_snes_or_gba_as_nes() {
        for system in [
            "Super Nintendo Entertainment System",
            "Nintendo Game Boy Advance",
        ] {
            assert_eq!(system_layout(system), "modern");
        }
        assert_eq!(system_layout("Nintendo Game Boy Color"), "two-button");
    }

    #[test]
    fn c_button_axis_mappings_generate_typed_axis_events() {
        let yaml = custom_profile_yaml(&ControllerCustomProfile {
            name: "N64 C buttons".into(),
            mappings: vec![ControllerButtonMapping {
                source_button: "RightStickUp".into(),
                target_button: "North".into(),
            }],
            ..Default::default()
        });
        assert!(yaml.contains("axis:\n          name: RightStick\n          direction: up"));
        assert!(yaml.contains("button: North"));
        assert!(!yaml.contains("button: RightStickUp"));
    }

    fn inventory() -> ControllerInventory {
        ControllerInventory {
            controllers: vec![controller("one", "First"), controller("two", "Second")],
            supported_targets: vec![ControllerTarget {
                id: "xb360".to_owned(),
                name: "Xbox 360".to_owned(),
            }],
            ..ControllerInventory::default()
        }
    }

    #[test]
    fn filters_non_game_input_devices_on_every_platform() {
        assert!(is_likely_non_game_controller("USB Keyboard"));
        assert!(is_likely_non_game_controller("Wireless Mouse"));
        assert!(!is_likely_non_game_controller("Xbox Wireless Controller"));
    }

    #[test]
    fn parses_inputplumber_box_tables_without_summary_rows() {
        let output = "│ Id │ Name │\n│ xb360 │ Microsoft X-Box 360 pad │\nFound 1 device(s)";
        assert_eq!(
            parse_box_rows(output),
            vec![
                vec!["Id".to_owned(), "Name".to_owned()],
                vec!["xb360".to_owned(), "Microsoft X-Box 360 pad".to_owned()]
            ]
        );
    }

    #[test]
    fn player_order_and_actions_preserve_unplugged_rows() {
        let inventory = inventory();
        let mut mapping = ControllerMappingSettings {
            player_mappings: vec![ControllerPlayerMapping {
                controller_id: Some("unplugged".to_owned()),
                profile_id: Some("none".to_owned()),
                output_target: None,
            }],
            ..ControllerMappingSettings::default()
        };

        assert!(set_controller_action(&mut mapping, "two", ACTION_HIDE));
        assert!(move_controller(&inventory, &mut mapping, "two", 1));
        assert_eq!(
            ordered_controllers(&inventory, &mapping)
                .iter()
                .map(|controller| controller.stable_id.as_str())
                .collect::<Vec<_>>(),
            vec!["one", "two"]
        );
        assert_eq!(controller_action(&mapping, "two"), ACTION_HIDE);
        assert!(
            mapping
                .player_mappings
                .iter()
                .any(|player| { player.controller_id.as_deref() == Some("unplugged") })
        );
    }

    #[test]
    fn profile_and_target_changes_are_bounded_to_known_values() {
        let inventory = inventory();
        let mut mapping = ControllerMappingSettings::default();
        assert!(!set_controller_profile(&mut mapping, "one", "invented"));
        assert!(set_controller_profile(
            &mut mapping,
            "one",
            TWO_BUTTON_CLOCKWISE_PROFILE_ID
        ));
        assert!(!set_controller_target(
            &inventory,
            &mut mapping,
            "one",
            "invented"
        ));
        assert!(set_controller_target(
            &inventory,
            &mut mapping,
            "one",
            "xb360"
        ));
        assert_eq!(controller_target(&mapping, "one"), "xb360");
    }

    #[test]
    fn game_and_platform_profiles_override_the_default_exactly() {
        let mapping = ControllerMappingSettings {
            default_profile_id: Some("default-profile".to_owned()),
            platform_profile_ids: HashMap::from([(
                "Nintendo Entertainment System".to_owned(),
                "platform-profile".to_owned(),
            )]),
            game_profile_ids: HashMap::from([("140".to_owned(), "game-profile".to_owned())]),
            ..ControllerMappingSettings::default()
        };
        assert_eq!(
            resolve_profile_id(&mapping, Some("Nintendo Entertainment System"), Some(140))
                .as_deref(),
            Some("game-profile")
        );
        assert_eq!(
            resolve_profile_id(&mapping, Some("Nintendo Entertainment System"), Some(141))
                .as_deref(),
            Some("platform-profile")
        );
        assert_eq!(
            resolve_profile_id(&mapping, Some("Arcade"), Some(141)).as_deref(),
            Some("default-profile")
        );
    }

    #[test]
    fn custom_profile_yaml_escapes_names_and_omits_identity_mappings() {
        let profile = ControllerCustomProfile {
            id: "custom:arcade".to_owned(),
            name: "Ben's \"Arcade\"".to_owned(),
            layout: "xbox".to_owned(),
            mappings: vec![
                crate::settings::ControllerButtonMapping {
                    source_button: "South".to_owned(),
                    target_button: "South".to_owned(),
                },
                crate::settings::ControllerButtonMapping {
                    source_button: "East".to_owned(),
                    target_button: "West".to_owned(),
                },
            ],
        };
        validate_custom_profile(&profile).expect("valid custom profile");
        let yaml = custom_profile_yaml(&profile);
        assert!(yaml.contains("name: \"Ben's \\\"Arcade\\\"\""));
        assert!(yaml.contains("button: East"));
        assert!(yaml.contains("button: West"));
        assert!(!yaml.contains("South to South"));
    }

    #[test]
    fn custom_profile_editor_keeps_one_exact_source_per_target() {
        let mut profile = ControllerCustomProfile {
            id: "custom:arcade".to_owned(),
            name: "Arcade".to_owned(),
            layout: "xbox".to_owned(),
            mappings: Vec::new(),
        };
        assert!(set_custom_profile_mapping(&mut profile, "South", "East"));
        assert_eq!(custom_profile_source_for_target(&profile, "South"), "East");
        assert!(set_custom_profile_mapping(&mut profile, "South", "West"));
        assert_eq!(profile.mappings.len(), 1);
        assert_eq!(custom_profile_source_for_target(&profile, "South"), "West");
        assert!(set_custom_profile_mapping(&mut profile, "South", "South"));
        assert!(profile.mappings.is_empty());
        assert!(!set_custom_profile_mapping(
            &mut profile,
            "unsupported",
            "South"
        ));
    }

    #[test]
    fn deleting_a_custom_profile_clears_every_exact_reference() {
        let profile_id = "custom:arcade";
        let mut mapping = ControllerMappingSettings {
            default_profile_id: Some(profile_id.to_owned()),
            player_mappings: vec![ControllerPlayerMapping {
                controller_id: Some("one".to_owned()),
                profile_id: Some(profile_id.to_owned()),
                output_target: Some("xb360".to_owned()),
            }],
            platform_profile_ids: HashMap::from([("Arcade".to_owned(), profile_id.to_owned())]),
            game_profile_ids: HashMap::from([("42".to_owned(), profile_id.to_owned())]),
            custom_profiles: vec![ControllerCustomProfile {
                id: profile_id.to_owned(),
                name: "Arcade".to_owned(),
                layout: "xbox".to_owned(),
                mappings: Vec::new(),
            }],
            ..ControllerMappingSettings::default()
        };

        assert!(remove_custom_profile(&mut mapping, profile_id));
        assert!(mapping.custom_profiles.is_empty());
        assert!(mapping.default_profile_id.is_none());
        assert!(mapping.platform_profile_ids.is_empty());
        assert!(mapping.game_profile_ids.is_empty());
        assert!(
            mapping
                .player_mappings
                .iter()
                .all(|player| player.profile_id.is_none())
        );
        assert!(!remove_custom_profile(&mut mapping, profile_id));
    }

    #[test]
    fn visual_button_labels_follow_the_selected_layout() {
        assert_eq!(controller_button_label("xbox", "South"), "A");
        assert_eq!(controller_button_label("playstation", "South"), "Cross");
        assert_eq!(controller_button_label("generic", "South"), "South");
        assert_eq!(controller_button_label("xbox", "DPadLeft"), "Left");
    }

    #[test]
    fn inputplumber_source_paths_are_parsed_without_guessing() {
        let output = "│ 0 │ Pad │ Profile │ [\"/dev/input/event12\", \"/dev/input/js3\"] │";
        assert_eq!(
            extract_quoted_paths(output),
            HashSet::from([
                PathBuf::from("/dev/input/event12"),
                PathBuf::from("/dev/input/js3")
            ])
        );
    }

    #[test]
    fn disabled_mapping_never_queries_or_changes_the_host() {
        let activation = activate_for_launch(&AppSettings::default(), Some("Arcade"), Some(1))
            .expect("disabled mapping is a no-op");
        assert!(activation.warning.is_none());
        assert!(activation.session.restore_entries.is_empty());
        assert!(activation.session.restore_order.is_empty());
    }
}
