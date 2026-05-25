use crate::backend_api::{
    ControllerButtonMapping, ControllerCustomProfile, ControllerDevice, ControllerInventory,
    ControllerMappingSettings, ControllerPlayerMapping, ControllerProfileInfo,
};
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};

pub const CONTROLLER_PROFILE_INHERIT: &str = "__inherit";
pub const CONTROLLER_PROFILE_NONE: &str = "__none";
pub const CONTROLLER_CREATE_PROFILE: &str = "__create_profile";
pub const CONTROLLER_TARGET_INHERIT: &str = "__inherit_target";
pub const CONTROLLER_ACTION_REMAP: &str = "remap";
pub const CONTROLLER_ACTION_PASSTHROUGH: &str = "passthrough";
pub const CONTROLLER_ACTION_HIDE: &str = "hide";
const CONTROLLER_SCOPE_ALL: &str = "__all";
const TWO_BUTTON_CLOCKWISE_PROFILE_ID: &str = "two-button-clockwise";
const CUSTOM_PROFILE_PREFIX: &str = "custom:";

const BUTTON_CHOICES: &[(&str, &str)] = &[
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

#[derive(Clone)]
pub enum ControllerRowUpdate {
    Reorder {
        source_id: String,
        target_id: String,
    },
    Move {
        controller_id: String,
        direction: isize,
    },
    Action {
        controller_id: String,
        value: String,
    },
    Profile {
        controller_id: String,
        value: String,
    },
    Target {
        controller_id: String,
        value: String,
    },
    CreateProfile {
        controller_id: String,
    },
}

#[derive(Clone, Copy)]
struct DiagramButton {
    id: &'static str,
    x: i32,
    y: i32,
}

const DIAGRAM_BUTTONS: &[DiagramButton] = &[
    DiagramButton {
        id: "LeftTrigger",
        x: 88,
        y: 30,
    },
    DiagramButton {
        id: "LeftBumper",
        x: 88,
        y: 62,
    },
    DiagramButton {
        id: "RightTrigger",
        x: 292,
        y: 30,
    },
    DiagramButton {
        id: "RightBumper",
        x: 292,
        y: 62,
    },
    DiagramButton {
        id: "DPadUp",
        x: 88,
        y: 128,
    },
    DiagramButton {
        id: "DPadLeft",
        x: 62,
        y: 154,
    },
    DiagramButton {
        id: "DPadRight",
        x: 114,
        y: 154,
    },
    DiagramButton {
        id: "DPadDown",
        x: 88,
        y: 180,
    },
    DiagramButton {
        id: "LeftStick",
        x: 142,
        y: 188,
    },
    DiagramButton {
        id: "Select",
        x: 168,
        y: 126,
    },
    DiagramButton {
        id: "Guide",
        x: 190,
        y: 154,
    },
    DiagramButton {
        id: "Start",
        x: 212,
        y: 126,
    },
    DiagramButton {
        id: "RightStick",
        x: 238,
        y: 188,
    },
    DiagramButton {
        id: "West",
        x: 266,
        y: 154,
    },
    DiagramButton {
        id: "North",
        x: 292,
        y: 128,
    },
    DiagramButton {
        id: "East",
        x: 318,
        y: 154,
    },
    DiagramButton {
        id: "South",
        x: 292,
        y: 180,
    },
];

pub fn fallback_controller_profiles() -> Vec<ControllerProfileInfo> {
    vec![ControllerProfileInfo {
        id: TWO_BUTTON_CLOCKWISE_PROFILE_ID.to_string(),
        name: "2-button clockwise diamond".to_string(),
        description:
            "Maps physical bottom/right face buttons to target left/bottom for NES-style layouts."
                .to_string(),
    }]
}

pub fn controller_profile_options(
    inventory: Option<ControllerInventory>,
    custom_profiles: Vec<ControllerCustomProfile>,
) -> Vec<ControllerProfileInfo> {
    let mut profiles = inventory
        .map(|inventory| inventory.built_in_profiles)
        .filter(|profiles| !profiles.is_empty())
        .unwrap_or_else(fallback_controller_profiles);

    profiles.extend(
        custom_profiles
            .into_iter()
            .map(|profile| ControllerProfileInfo {
                id: profile.id,
                name: profile.name,
                description: "Custom Lunchbox controller profile".to_string(),
            }),
    );
    profiles
}

pub fn controller_target_options(inventory: Option<ControllerInventory>) -> Vec<(String, String)> {
    let targets = inventory
        .map(|inventory| inventory.supported_targets)
        .unwrap_or_default();
    let filtered = targets
        .into_iter()
        .filter(|target| {
            matches!(
                target.id.as_str(),
                "xb360" | "xbox-series" | "xbox-elite" | "ds5" | "gamepad"
            )
        })
        .map(|target| (target.id, target.name))
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        vec![
            ("xb360".to_string(), "Microsoft X-Box 360 pad".to_string()),
            (
                "xbox-series".to_string(),
                "Microsoft Xbox Series S|X Controller".to_string(),
            ),
            ("ds5".to_string(), "Sony DualSense".to_string()),
            ("gamepad".to_string(), "InputPlumber Gamepad".to_string()),
        ]
    } else {
        filtered
    }
}

pub fn controller_scope_hint(inventory: Option<ControllerInventory>, loading: bool) -> String {
    if loading {
        return "Checking controllers".to_string();
    }

    match inventory {
        None => "Controllers not checked".to_string(),
        Some(inventory) => match inventory.controllers.len() {
            0 => "No plugged in controllers".to_string(),
            1 => "1 plugged in controller".to_string(),
            count => format!("{count} plugged in controllers"),
        },
    }
}

pub fn controller_profile_select_value(map: &HashMap<String, String>, key: &str) -> String {
    match map.get(key).map(|value| value.trim()) {
        Some("") | Some("none") => CONTROLLER_PROFILE_NONE.to_string(),
        Some(profile_id) => profile_id.to_string(),
        None => CONTROLLER_PROFILE_INHERIT.to_string(),
    }
}

pub fn set_controller_profile_override(
    map: &mut HashMap<String, String>,
    key: String,
    selected_value: String,
) {
    match selected_value.as_str() {
        CONTROLLER_PROFILE_INHERIT => {
            map.remove(&key);
        }
        CONTROLLER_PROFILE_NONE => {
            map.insert(key, "none".to_string());
        }
        _ => {
            map.insert(key, selected_value);
        }
    }
}

pub fn trim_default_player_mappings(mapping: &mut ControllerMappingSettings) {
    while mapping
        .player_mappings
        .last()
        .map(player_mapping_is_default)
        .unwrap_or(false)
    {
        mapping.player_mappings.pop();
    }
}

pub fn apply_controller_row_update(
    mapping: &mut ControllerMappingSettings,
    inventory: Option<ControllerInventory>,
    update: ControllerRowUpdate,
) {
    match update {
        ControllerRowUpdate::Reorder {
            source_id,
            target_id,
        } => reorder_controller(mapping, inventory, &source_id, &target_id),
        ControllerRowUpdate::Move {
            controller_id,
            direction,
        } => move_controller(mapping, inventory, &controller_id, direction),
        ControllerRowUpdate::Action {
            controller_id,
            value,
        } => set_controller_action(mapping, &controller_id, &value),
        ControllerRowUpdate::Profile {
            controller_id,
            value,
        } => set_controller_profile(mapping, &controller_id, &value),
        ControllerRowUpdate::Target {
            controller_id,
            value,
        } => set_controller_target(mapping, &controller_id, &value),
        ControllerRowUpdate::CreateProfile { .. } => {}
    }
}

pub fn add_custom_profile(
    mapping: &mut ControllerMappingSettings,
    profile: ControllerCustomProfile,
) {
    mapping
        .custom_profiles
        .retain(|saved| saved.id != profile.id);
    mapping.custom_profiles.push(profile);
}

pub fn apply_created_profile_to_controller(
    mapping: &mut ControllerMappingSettings,
    controller_id: &str,
    profile_id: &str,
) {
    set_controller_action(mapping, controller_id, CONTROLLER_ACTION_REMAP);
    set_controller_profile(mapping, controller_id, profile_id);
}

fn player_mapping_is_default(player: &ControllerPlayerMapping) -> bool {
    optional_string_is_empty(&player.controller_id)
        && optional_string_is_empty(&player.profile_id)
        && optional_string_is_empty(&player.output_target)
}

fn optional_string_is_empty(value: &Option<String>) -> bool {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
}

fn ordered_controller_ids(
    inventory: Option<ControllerInventory>,
    mapping: &ControllerMappingSettings,
) -> Vec<String> {
    let controllers = inventory
        .map(|inventory| inventory.controllers)
        .unwrap_or_default();
    let attached = controllers
        .iter()
        .map(|controller| controller.stable_id.clone())
        .collect::<HashSet<_>>();
    let mut ordered = Vec::new();

    for player in &mapping.player_mappings {
        let Some(controller_id) = player.controller_id.as_deref().map(str::trim) else {
            continue;
        };
        if controller_id.is_empty()
            || controller_id == CONTROLLER_SCOPE_ALL
            || ordered.iter().any(|id| id == controller_id)
            || !attached.contains(controller_id)
        {
            continue;
        }
        ordered.push(controller_id.to_string());
    }

    for controller in controllers {
        if !ordered.iter().any(|id| id == &controller.stable_id) {
            ordered.push(controller.stable_id);
        }
    }

    ordered
}

fn controller_rows(
    inventory: Option<ControllerInventory>,
    mapping: &ControllerMappingSettings,
) -> Vec<ControllerDevice> {
    let Some(inventory) = inventory else {
        return Vec::new();
    };
    let ids = ordered_controller_ids(Some(inventory.clone()), mapping);
    let by_id = inventory
        .controllers
        .into_iter()
        .map(|controller| (controller.stable_id.clone(), controller))
        .collect::<HashMap<_, _>>();

    ids.into_iter()
        .filter_map(|id| by_id.get(&id).cloned())
        .collect()
}

fn set_ordered_controller_ids(mapping: &mut ControllerMappingSettings, ordered_ids: Vec<String>) {
    let old_by_controller = mapping
        .player_mappings
        .iter()
        .filter_map(|player| {
            player
                .controller_id
                .as_deref()
                .map(str::trim)
                .filter(|controller_id| !controller_id.is_empty())
                .map(|controller_id| (controller_id.to_string(), player.clone()))
        })
        .collect::<HashMap<_, _>>();
    let ordered_set = ordered_ids.iter().cloned().collect::<HashSet<_>>();
    let mut next = ordered_ids
        .into_iter()
        .map(|controller_id| {
            let mut row = old_by_controller
                .get(&controller_id)
                .cloned()
                .unwrap_or_default();
            row.controller_id = Some(controller_id);
            row
        })
        .collect::<Vec<_>>();

    for player in &mapping.player_mappings {
        let Some(controller_id) = player.controller_id.as_deref().map(str::trim) else {
            continue;
        };
        if controller_id.is_empty()
            || controller_id == CONTROLLER_SCOPE_ALL
            || ordered_set.contains(controller_id)
        {
            continue;
        }
        next.push(player.clone());
    }

    mapping.profile_controller_ids.clear();
    mapping.player_mappings = next;
    trim_default_player_mappings(mapping);
}

fn reorder_controller(
    mapping: &mut ControllerMappingSettings,
    inventory: Option<ControllerInventory>,
    source_id: &str,
    target_id: &str,
) {
    if source_id == target_id {
        return;
    }
    let mut ids = ordered_controller_ids(inventory, mapping);
    let Some(source_index) = ids.iter().position(|id| id == source_id) else {
        return;
    };
    let Some(target_index) = ids.iter().position(|id| id == target_id) else {
        return;
    };
    let moved = ids.remove(source_index);
    let insert_at = if source_index < target_index {
        target_index.saturating_sub(1)
    } else {
        target_index
    };
    ids.insert(insert_at, moved);
    set_ordered_controller_ids(mapping, ids);
}

fn move_controller(
    mapping: &mut ControllerMappingSettings,
    inventory: Option<ControllerInventory>,
    controller_id: &str,
    direction: isize,
) {
    let mut ids = ordered_controller_ids(inventory, mapping);
    let Some(index) = ids.iter().position(|id| id == controller_id) else {
        return;
    };
    let target = if direction < 0 {
        index.checked_sub(1)
    } else {
        let next = index + 1;
        (next < ids.len()).then_some(next)
    };
    let Some(target) = target else {
        return;
    };
    ids.swap(index, target);
    set_ordered_controller_ids(mapping, ids);
}

fn ensure_controller_row<'a>(
    mapping: &'a mut ControllerMappingSettings,
    controller_id: &str,
) -> &'a mut ControllerPlayerMapping {
    let position = mapping.player_mappings.iter().position(|player| {
        player
            .controller_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|saved| saved == controller_id)
    });
    let index = if let Some(position) = position {
        position
    } else {
        mapping.player_mappings.push(ControllerPlayerMapping {
            controller_id: Some(controller_id.to_string()),
            ..ControllerPlayerMapping::default()
        });
        mapping.player_mappings.len() - 1
    };
    &mut mapping.player_mappings[index]
}

fn set_controller_action(
    mapping: &mut ControllerMappingSettings,
    controller_id: &str,
    value: &str,
) {
    let row = ensure_controller_row(mapping, controller_id);
    match value {
        CONTROLLER_ACTION_HIDE => {
            row.profile_id = Some("none".to_string());
            if !mapping
                .hidden_controller_ids
                .iter()
                .any(|saved| saved == controller_id)
            {
                mapping
                    .hidden_controller_ids
                    .push(controller_id.to_string());
            }
        }
        CONTROLLER_ACTION_PASSTHROUGH => {
            row.profile_id = Some("none".to_string());
            mapping
                .hidden_controller_ids
                .retain(|saved| saved != controller_id);
        }
        _ => {
            if row.profile_id.as_deref() == Some("none") {
                row.profile_id = None;
            }
            mapping
                .hidden_controller_ids
                .retain(|saved| saved != controller_id);
        }
    }
    mapping.enabled = true;
    mapping.manage_all = true;
    trim_default_player_mappings(mapping);
}

fn set_controller_profile(
    mapping: &mut ControllerMappingSettings,
    controller_id: &str,
    value: &str,
) {
    let selected = value.trim();
    let row = ensure_controller_row(mapping, controller_id);
    row.profile_id = match selected {
        "" | CONTROLLER_PROFILE_INHERIT | CONTROLLER_CREATE_PROFILE => None,
        CONTROLLER_PROFILE_NONE => Some("none".to_string()),
        _ => Some(selected.to_string()),
    };
    mapping
        .hidden_controller_ids
        .retain(|saved| saved != controller_id);
    mapping.enabled = true;
    mapping.manage_all = true;
    trim_default_player_mappings(mapping);
}

fn set_controller_target(
    mapping: &mut ControllerMappingSettings,
    controller_id: &str,
    value: &str,
) {
    let selected = value.trim();
    let row = ensure_controller_row(mapping, controller_id);
    row.output_target = if selected == CONTROLLER_TARGET_INHERIT || selected.is_empty() {
        None
    } else {
        Some(selected.to_string())
    };
    mapping.enabled = true;
    mapping.manage_all = true;
    trim_default_player_mappings(mapping);
}

fn controller_action_value(mapping: &ControllerMappingSettings, controller_id: &str) -> String {
    if mapping
        .hidden_controller_ids
        .iter()
        .any(|saved| saved == controller_id)
    {
        return CONTROLLER_ACTION_HIDE.to_string();
    }
    if mapping.player_mappings.iter().any(|player| {
        player
            .controller_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|saved| saved == controller_id)
            && player.profile_id.as_deref().map(str::trim) == Some("none")
    }) {
        CONTROLLER_ACTION_PASSTHROUGH.to_string()
    } else {
        CONTROLLER_ACTION_REMAP.to_string()
    }
}

fn controller_profile_value(mapping: &ControllerMappingSettings, controller_id: &str) -> String {
    match mapping
        .player_mappings
        .iter()
        .find(|player| {
            player
                .controller_id
                .as_deref()
                .map(str::trim)
                .is_some_and(|saved| saved == controller_id)
        })
        .and_then(|player| player.profile_id.as_deref())
        .map(str::trim)
    {
        Some("") | None => CONTROLLER_PROFILE_INHERIT.to_string(),
        Some("none") => CONTROLLER_PROFILE_NONE.to_string(),
        Some(profile_id) => profile_id.to_string(),
    }
}

fn controller_target_value(mapping: &ControllerMappingSettings, controller_id: &str) -> String {
    mapping
        .player_mappings
        .iter()
        .find(|player| {
            player
                .controller_id
                .as_deref()
                .map(str::trim)
                .is_some_and(|saved| saved == controller_id)
        })
        .and_then(|player| player.output_target.as_deref())
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| CONTROLLER_TARGET_INHERIT.to_string())
}

fn controller_identity(controller: &ControllerDevice) -> String {
    let vid_pid = match (
        controller.vendor_id.as_deref(),
        controller.product_id.as_deref(),
    ) {
        (Some(vendor), Some(product)) => format!("VID:PID {vendor}:{product}"),
        _ => "VID:PID unknown".to_string(),
    };
    let serial = controller
        .unique_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("no serial reported");
    format!("{vid_pid} | Serial: {serial}")
}

fn button_label_for_layout(layout: &str, button: &str) -> String {
    let label = match layout {
        "playstation" => match button {
            "South" => "X",
            "East" => "O",
            "West" => "Square",
            "North" => "Tri",
            "Start" => "Opt",
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
    label.to_string()
}

fn mapping_source_for_target(profile: &ControllerCustomProfile, target: &str) -> String {
    profile
        .mappings
        .iter()
        .find(|mapping| mapping.target_button == target)
        .map(|mapping| mapping.source_button.clone())
        .unwrap_or_else(|| target.to_string())
}

fn set_profile_mapping(profile: &mut ControllerCustomProfile, target: &str, source: &str) {
    profile
        .mappings
        .retain(|mapping| mapping.target_button != target);
    if target != source {
        profile.mappings.push(ControllerButtonMapping {
            source_button: source.to_string(),
            target_button: target.to_string(),
        });
    }
}

fn two_button_clockwise_mappings() -> Vec<ControllerButtonMapping> {
    vec![
        ControllerButtonMapping {
            source_button: "East".to_string(),
            target_button: "South".to_string(),
        },
        ControllerButtonMapping {
            source_button: "North".to_string(),
            target_button: "East".to_string(),
        },
        ControllerButtonMapping {
            source_button: "South".to_string(),
            target_button: "West".to_string(),
        },
        ControllerButtonMapping {
            source_button: "West".to_string(),
            target_button: "North".to_string(),
        },
    ]
}

fn slugify_id(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "custom-profile".to_string()
    } else {
        slug.to_string()
    }
}

fn custom_profile_id(name: &str) -> String {
    format!(
        "{CUSTOM_PROFILE_PREFIX}{}-{}",
        slugify_id(name),
        js_sys::Date::now() as u64
    )
}

#[component]
pub fn ControllerDeviceMappingList(
    inventory: RwSignal<Option<ControllerInventory>>,
    loading: Signal<bool>,
    mapping: Signal<ControllerMappingSettings>,
    profile_options: Signal<Vec<ControllerProfileInfo>>,
    target_options: Signal<Vec<(String, String)>>,
    controls_disabled: Signal<bool>,
    #[prop(into)] on_update: Callback<ControllerRowUpdate>,
) -> impl IntoView {
    let dragged_controller = RwSignal::new(None::<String>);
    let rows = move || controller_rows(inventory.get(), &mapping.get());
    let is_empty = move || {
        inventory
            .get()
            .is_some_and(|inventory| inventory.controllers.is_empty())
    };

    view! {
        <div class="controller-device-mapping">
            <div class="controller-player-table-header">
                <span>"Connected controllers"</span>
                <small>{move || controller_scope_hint(inventory.get(), loading.get())}</small>
            </div>

            <Show when=move || loading.get()>
                <div class="settings-hint">"Checking controllers..."</div>
            </Show>

            <Show when=is_empty>
                <div class="settings-hint">"No joystick controllers were found."</div>
            </Show>

            <div class="controller-device-map-list">
                <For
                    each=rows
                    key=|controller| controller.stable_id.clone()
                    children=move |controller| {
                        let controller_id = controller.stable_id.clone();
                        let controller_id_for_drag = controller_id.clone();
                        let controller_id_for_drop = controller_id.clone();
                        let controller_id_for_up = controller_id.clone();
                        let controller_id_for_down = controller_id.clone();
                        let controller_id_for_action = controller_id.clone();
                        let controller_id_for_profile = controller_id.clone();
                        let controller_id_for_target = controller_id.clone();
                        let player_number = move || {
                            rows()
                                .iter()
                                .position(|row| row.stable_id == controller_id)
                                .map(|index| format!("P{}", index + 1))
                                .unwrap_or_else(|| "P?".to_string())
                        };
                        let action_value = {
                            let controller_id = controller.stable_id.clone();
                            move || controller_action_value(&mapping.get(), &controller_id)
                        };
                        let profile_value = {
                            let controller_id = controller.stable_id.clone();
                            move || controller_profile_value(&mapping.get(), &controller_id)
                        };
                        let target_value = {
                            let controller_id = controller.stable_id.clone();
                            move || controller_target_value(&mapping.get(), &controller_id)
                        };

                        view! {
                            <div
                                class="controller-device-map-row"
                                draggable="true"
                                on:dragstart=move |_| dragged_controller.set(Some(controller_id_for_drag.clone()))
                                on:dragover=move |ev| ev.prevent_default()
                                on:drop=move |ev| {
                                    ev.prevent_default();
                                    if let Some(source_id) = dragged_controller.get_untracked() {
                                        on_update.run(ControllerRowUpdate::Reorder {
                                            source_id,
                                            target_id: controller_id_for_drop.clone(),
                                        });
                                    }
                                    dragged_controller.set(None);
                                }
                            >
                                <div class="controller-device-map-handle" title="Drag to reorder">
                                    <span class="controller-device-map-grip" aria-hidden="true">"::"</span>
                                    <strong>{player_number}</strong>
                                    <div class="controller-device-map-move">
                                        <button
                                            type="button"
                                            disabled=controls_disabled
                                            on:click=move |_| on_update.run(ControllerRowUpdate::Move {
                                                controller_id: controller_id_for_up.clone(),
                                                direction: -1,
                                            })
                                        >"Up"</button>
                                        <button
                                            type="button"
                                            disabled=controls_disabled
                                            on:click=move |_| on_update.run(ControllerRowUpdate::Move {
                                                controller_id: controller_id_for_down.clone(),
                                                direction: 1,
                                            })
                                        >"Down"</button>
                                    </div>
                                </div>

                                <div class="controller-device-map-main">
                                    <div class="controller-device-map-title">
                                        <span>{controller.name.clone()}</span>
                                        <small>{controller_identity(&controller)}</small>
                                        <small>{controller.stable_id.clone()}</small>
                                    </div>

                                    <div class="controller-device-map-controls">
                                        <label class="game-controller-field">
                                            <span>"Action"</span>
                                            <select
                                                prop:value=action_value
                                                disabled=controls_disabled
                                                on:change=move |ev| {
                                                    on_update.run(ControllerRowUpdate::Action {
                                                        controller_id: controller_id_for_action.clone(),
                                                        value: event_target_value(&ev),
                                                    });
                                                }
                                            >
                                                <option value=CONTROLLER_ACTION_REMAP>"Remap to virtual controller"</option>
                                                <option value=CONTROLLER_ACTION_PASSTHROUGH>"Pass through"</option>
                                                <option value=CONTROLLER_ACTION_HIDE>"Hide from emulator"</option>
                                            </select>
                                        </label>

                                        <label class="game-controller-field">
                                            <span>"Profile"</span>
                                            <select
                                                prop:value=profile_value
                                                disabled=controls_disabled
                                                on:change=move |ev| {
                                                    let selected = event_target_value(&ev);
                                                    if selected == CONTROLLER_CREATE_PROFILE {
                                                        on_update.run(ControllerRowUpdate::CreateProfile {
                                                            controller_id: controller_id_for_profile.clone(),
                                                        });
                                                    } else {
                                                        on_update.run(ControllerRowUpdate::Profile {
                                                            controller_id: controller_id_for_profile.clone(),
                                                            value: selected,
                                                        });
                                                    }
                                                }
                                            >
                                                <option value=CONTROLLER_PROFILE_INHERIT>"Use game/system/default"</option>
                                                <option value=CONTROLLER_PROFILE_NONE>"Off"</option>
                                                <For
                                                    each=move || profile_options.get()
                                                    key=|profile| profile.id.clone()
                                                    children=move |profile| view! {
                                                        <option value=profile.id>{profile.name}</option>
                                                    }
                                                />
                                                <option value=CONTROLLER_CREATE_PROFILE>"Create new profile..."</option>
                                            </select>
                                        </label>

                                        <label class="game-controller-field">
                                            <span>"Virtual target"</span>
                                            <select
                                                prop:value=target_value
                                                disabled=controls_disabled
                                                on:change=move |ev| {
                                                    on_update.run(ControllerRowUpdate::Target {
                                                        controller_id: controller_id_for_target.clone(),
                                                        value: event_target_value(&ev),
                                                    });
                                                }
                                            >
                                                <option value=CONTROLLER_TARGET_INHERIT>"Default target"</option>
                                                <For
                                                    each=move || target_options.get()
                                                    key=|(id, _)| id.clone()
                                                    children=move |(id, name)| view! {
                                                        <option value=id>{name}</option>
                                                    }
                                                />
                                            </select>
                                        </label>
                                    </div>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}

#[component]
pub fn ControllerProfileDesigner(
    initial_controller_id: Option<String>,
    #[prop(into)] on_save: Callback<(ControllerCustomProfile, Option<String>)>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let selected_target = RwSignal::new("South".to_string());
    let draft = RwSignal::new(ControllerCustomProfile {
        id: String::new(),
        name: "Custom profile".to_string(),
        layout: "xbox".to_string(),
        mappings: Vec::new(),
    });
    let controller_id = StoredValue::new(initial_controller_id);
    let layout = move || draft.get().layout;
    let selected_source = move || mapping_source_for_target(&draft.get(), &selected_target.get());

    view! {
        <div class="controller-profile-designer">
            <div class="controller-profile-designer-header">
                <div>
                    <h3>"Create Controller Profile"</h3>
                </div>
                <button
                    type="button"
                    class="controller-details-secondary-btn"
                    on:click=move |_| on_cancel.run(())
                >
                    "Cancel"
                </button>
            </div>

            <div class="controller-profile-designer-fields">
                <label class="game-controller-field">
                    <span>"Profile name"</span>
                    <input
                        type="text"
                        prop:value=move || draft.get().name
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            draft.update(|profile| profile.name = value);
                        }
                    />
                </label>
                <label class="game-controller-field">
                    <span>"Diagram"</span>
                    <select
                        prop:value=layout
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            draft.update(|profile| profile.layout = value);
                        }
                    >
                        <option value="xbox">"Xbox"</option>
                        <option value="playstation">"PlayStation 4/5"</option>
                        <option value="generic">"Generic"</option>
                    </select>
                </label>
            </div>

            <div class="controller-profile-designer-workspace">
                <svg
                    class="controller-map-svg"
                    viewBox="0 0 380 240"
                    role="img"
                    aria-label="Controller button mapping diagram"
                >
                    <path
                        class="controller-map-shell"
                        d="M73 78 C91 58 127 62 151 84 L229 84 C253 62 289 58 307 78 C332 105 343 177 324 201 C309 220 283 214 258 178 L122 178 C97 214 71 220 56 201 C37 177 48 105 73 78 Z"
                    />
                    <For
                        each=move || DIAGRAM_BUTTONS.to_vec()
                        key=|button| button.id
                        children=move |button| {
                            let target_id = button.id.to_string();
                            let target_for_click = target_id.clone();
                            let class_target = target_id.clone();
                            let label = move || {
                                button_label_for_layout(&draft.get().layout, button.id)
                            };
                            view! {
                                <g
                                    class=move || {
                                        if selected_target.get() == class_target {
                                            "controller-map-button is-selected"
                                        } else {
                                            "controller-map-button"
                                        }
                                    }
                                    tabindex="0"
                                    role="button"
                                    on:click=move |_| selected_target.set(target_for_click.clone())
                                >
                                    <circle cx=button.x cy=button.y r="18" />
                                    <text x=button.x y=button.y>{label}</text>
                                </g>
                            }
                        }
                    />
                </svg>

                <div class="controller-profile-assignment">
                    <div class="controller-profile-assignment-current">
                        <span>"Virtual target"</span>
                        <strong>{move || {
                            button_label_for_layout(&draft.get().layout, &selected_target.get()).to_string()
                        }}</strong>
                    </div>

                    <label class="game-controller-field">
                        <span>"Physical source"</span>
                        <select
                            prop:value=selected_source
                            on:change=move |ev| {
                                let source = event_target_value(&ev);
                                let target = selected_target.get_untracked();
                                draft.update(|profile| set_profile_mapping(profile, &target, &source));
                            }
                        >
                            <For
                                each=move || BUTTON_CHOICES.to_vec()
                                key=|(id, _)| *id
                                children=move |(id, label)| view! {
                                    <option value=id>{label}</option>
                                }
                            />
                        </select>
                    </label>

                    <div class="controller-action-row">
                        <button
                            type="button"
                            class="controller-details-secondary-btn"
                            on:click=move |_| draft.update(|profile| {
                                profile.mappings = two_button_clockwise_mappings();
                            })
                        >
                            "Apply 2-button clockwise preset"
                        </button>
                        <button
                            type="button"
                            class="controller-details-secondary-btn"
                            on:click=move |_| {
                                let target = selected_target.get_untracked();
                                draft.update(|profile| {
                                    profile.mappings.retain(|mapping| mapping.target_button != target);
                                });
                            }
                        >
                            "Clear selected"
                        </button>
                    </div>
                </div>
            </div>

            <div class="controller-profile-map-summary">
                <For
                    each=move || draft.get().mappings
                    key=|mapping| format!("{}:{}", mapping.source_button, mapping.target_button)
                    children=move |mapping| {
                        view! {
                            <span>
                                {format!(
                                    "{} -> {}",
                                    mapping.source_button,
                                    mapping.target_button
                                )}
                            </span>
                        }
                    }
                />
            </div>

            <div class="controller-action-row">
                <button
                    type="button"
                    class="settings-test-btn"
                    on:click=move |_| {
                        let mut profile = draft.get_untracked();
                        if profile.id.trim().is_empty() {
                            profile.id = custom_profile_id(&profile.name);
                        }
                        if profile.name.trim().is_empty() {
                            profile.name = "Custom profile".to_string();
                        }
                        on_save.run((profile, controller_id.get_value()));
                    }
                >
                    "Save profile"
                </button>
            </div>
        </div>
    }
}
