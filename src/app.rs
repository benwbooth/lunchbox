use crate::backend_api::Game;
use crate::components::{
    EmulatorUpdates, GameDetails, GameGrid, MinervaDownloadQueue, Settings, Sidebar, Toolbar,
};
use crate::navigation::{self, NavigationAction};
use gloo_timers::callback::Interval;
use js_sys::{Array, Function, Reflect};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Gamepad, GamepadButton};

pub const PLATFORM_SELECTION_MINIGAMES: &str = "__minigames__";
pub const PLATFORM_SELECTION_ALL_GAMES: &str = "__all_games__";
const APP_UI_STATE_KEY: &str = "lunchbox.ui.app.v1";
const FILTER_DEFAULTS_VERSION_NON_RETAIL: u8 = 1;

#[derive(Default)]
struct RepeatState {
    active: bool,
    next_allowed_ms: f64,
}

#[derive(Default)]
struct GamepadRepeatState {
    up: RepeatState,
    down: RepeatState,
    left: RepeatState,
    right: RepeatState,
    primary: RepeatState,
    secondary: RepeatState,
}

fn consume_repeat(now_ms: f64, is_pressed: bool, state: &mut RepeatState) -> bool {
    const INITIAL_REPEAT_DELAY_MS: f64 = 260.0;
    const HELD_REPEAT_DELAY_MS: f64 = 120.0;

    if !is_pressed {
        state.active = false;
        state.next_allowed_ms = 0.0;
        return false;
    }

    if !state.active {
        state.active = true;
        state.next_allowed_ms = now_ms + INITIAL_REPEAT_DELAY_MS;
        return true;
    }

    if now_ms >= state.next_allowed_ms {
        state.next_allowed_ms = now_ms + HELD_REPEAT_DELAY_MS;
        return true;
    }

    false
}

fn array_item_as<T: JsCast>(array: &Array, index: u32) -> Option<T> {
    array.get(index).dyn_into::<T>().ok()
}

fn navigator_gamepads_array() -> Option<Array> {
    let window = web_sys::window()?;
    let navigator = window.navigator();

    for method_name in ["getGamepads", "webkitGetGamepads"] {
        let method = Reflect::get(navigator.as_ref(), &JsValue::from_str(method_name)).ok()?;
        let Some(function) = method.dyn_ref::<Function>() else {
            continue;
        };
        let result = function.call0(navigator.as_ref()).ok()?;
        if let Ok(gamepads) = result.dyn_into::<Array>() {
            return Some(gamepads);
        }
    }

    None
}

fn first_connected_gamepad() -> Option<Gamepad> {
    let pads = navigator_gamepads_array()?;

    for index in 0..pads.length() {
        if let Some(gamepad) = array_item_as::<Gamepad>(&pads, index) {
            if gamepad.connected() || gamepad.buttons().length() > 0 || gamepad.axes().length() > 0
            {
                return Some(gamepad);
            }
        }
    }

    None
}

fn button_pressed(buttons: &Array, index: u32) -> bool {
    array_item_as::<GamepadButton>(buttons, index)
        .map(|button| button.pressed() || button.value() > 0.5)
        .unwrap_or(false)
}

fn axis_value(axes: &Array, index: u32) -> f64 {
    axes.get(index).as_f64().unwrap_or(0.0)
}

fn next_gamepad_action(
    now_ms: f64,
    repeat_state: &Rc<RefCell<GamepadRepeatState>>,
) -> Option<NavigationAction> {
    const AXIS_THRESHOLD: f64 = 0.55;

    let gamepad = first_connected_gamepad()?;
    let buttons = gamepad.buttons();
    let axes = gamepad.axes();
    let axis_x = axis_value(&axes, 0);
    let axis_y = axis_value(&axes, 1);

    let up_pressed = button_pressed(&buttons, 12) || axis_y <= -AXIS_THRESHOLD;
    let down_pressed = button_pressed(&buttons, 13) || axis_y >= AXIS_THRESHOLD;
    let left_pressed = button_pressed(&buttons, 14) || axis_x <= -AXIS_THRESHOLD;
    let right_pressed = button_pressed(&buttons, 15) || axis_x >= AXIS_THRESHOLD;
    let primary_pressed = button_pressed(&buttons, 0);
    let secondary_pressed = button_pressed(&buttons, 1);

    let mut repeat_state = repeat_state.borrow_mut();

    if consume_repeat(now_ms, up_pressed, &mut repeat_state.up) {
        return Some(NavigationAction::Up);
    }
    if consume_repeat(now_ms, down_pressed, &mut repeat_state.down) {
        return Some(NavigationAction::Down);
    }
    if consume_repeat(now_ms, left_pressed, &mut repeat_state.left) {
        return Some(NavigationAction::Left);
    }
    if consume_repeat(now_ms, right_pressed, &mut repeat_state.right) {
        return Some(NavigationAction::Right);
    }
    if consume_repeat(now_ms, primary_pressed, &mut repeat_state.primary) {
        return Some(NavigationAction::Activate);
    }
    if consume_repeat(now_ms, secondary_pressed, &mut repeat_state.secondary) {
        return Some(NavigationAction::Back);
    }

    None
}

/// Artwork type to display in grid view
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ArtworkDisplayType {
    #[default]
    BoxFront,
    Screenshot,
    TitleScreen,
    Fanart,
    ClearLogo,
}

impl ArtworkDisplayType {
    /// Get the media type identifier for API calls
    pub fn media_type_id(&self) -> &'static str {
        match self {
            ArtworkDisplayType::BoxFront => "box-front",
            ArtworkDisplayType::Screenshot => "screenshot",
            ArtworkDisplayType::TitleScreen => "title-screen",
            ArtworkDisplayType::Fanart => "fanart",
            ArtworkDisplayType::ClearLogo => "clear-logo",
        }
    }

    /// Get LaunchBox image type string (used by LazyImage component)
    pub fn to_image_type(&self) -> &'static str {
        match self {
            ArtworkDisplayType::BoxFront => "Box - Front",
            ArtworkDisplayType::Screenshot => "Screenshot - Gameplay",
            ArtworkDisplayType::TitleScreen => "Screenshot - Game Title",
            ArtworkDisplayType::Fanart => "Fanart - Background",
            ArtworkDisplayType::ClearLogo => "Clear Logo",
        }
    }

    /// Get display label
    pub fn label(&self) -> &'static str {
        match self {
            ArtworkDisplayType::BoxFront => "Box Art",
            ArtworkDisplayType::Screenshot => "Screenshot",
            ArtworkDisplayType::TitleScreen => "Title Screen",
            ArtworkDisplayType::Fanart => "Fanart",
            ArtworkDisplayType::ClearLogo => "Clear Logo",
        }
    }

    /// All artwork types
    pub fn all() -> &'static [ArtworkDisplayType] {
        &[
            ArtworkDisplayType::BoxFront,
            ArtworkDisplayType::Screenshot,
            ArtworkDisplayType::TitleScreen,
            ArtworkDisplayType::Fanart,
            ArtworkDisplayType::ClearLogo,
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameFilters {
    pub installed_only: bool,
    pub hide_homebrew: bool,
    pub hide_adult: bool,
}

impl Default for GameFilters {
    fn default() -> Self {
        Self {
            installed_only: false,
            hide_homebrew: true,
            hide_adult: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct AppUiState {
    selected_platform: Option<String>,
    selected_collection: Option<String>,
    view_mode: ViewMode,
    search_query: String,
    artwork_type: ArtworkDisplayType,
    zoom_level: f64,
    #[serde(default)]
    game_filters: GameFilters,
    #[serde(default)]
    filter_defaults_version: u8,
}

impl Default for AppUiState {
    fn default() -> Self {
        Self {
            selected_platform: Some(PLATFORM_SELECTION_MINIGAMES.to_string()),
            selected_collection: None,
            view_mode: ViewMode::Grid,
            search_query: String::new(),
            artwork_type: ArtworkDisplayType::BoxFront,
            zoom_level: 1.0,
            game_filters: GameFilters::default(),
            filter_defaults_version: FILTER_DEFAULTS_VERSION_NON_RETAIL,
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    let mut persisted =
        crate::ui_state::load_json::<AppUiState>(APP_UI_STATE_KEY).unwrap_or_default();
    if persisted.selected_collection.is_some() {
        persisted.selected_platform = None;
    }
    if persisted.selected_platform.is_none() && persisted.selected_collection.is_none() {
        persisted.selected_platform = Some(PLATFORM_SELECTION_MINIGAMES.to_string());
    }
    if persisted.filter_defaults_version < FILTER_DEFAULTS_VERSION_NON_RETAIL {
        persisted.game_filters.hide_homebrew = true;
        persisted.filter_defaults_version = FILTER_DEFAULTS_VERSION_NON_RETAIL;
    }

    // State for selected platform (now uses platform name)
    let (selected_platform, set_selected_platform) =
        signal::<Option<String>>(persisted.selected_platform.clone());
    // State for selected collection
    let (selected_collection, set_selected_collection) =
        signal::<Option<String>>(persisted.selected_collection.clone());
    // State for view mode (grid/list)
    let (view_mode, set_view_mode) = signal(persisted.view_mode);
    // State for search query
    let (search_query, set_search_query) = signal(persisted.search_query.clone());
    // State for selected game (for details panel)
    let (selected_game, set_selected_game) = signal::<Option<Game>>(None);
    // State for settings panel
    let (show_settings, set_show_settings) = signal(false);
    // State for emulator updates pane and available update count
    let (show_emulator_updates, set_show_emulator_updates) = signal(false);
    let (emulator_update_count, set_emulator_update_count) = signal::<Option<usize>>(None);
    // Trigger for refreshing collections
    let (collections_refresh, set_collections_refresh) = signal(0u32);
    // State for artwork display type in grid
    let (artwork_type, set_artwork_type) = signal(persisted.artwork_type);
    // State for zoom level (0.5 to 2.0, default 1.0)
    let (zoom_level, set_zoom_level) = signal(persisted.zoom_level.clamp(0.5, 2.0));
    // State for game list filters
    let (game_filters, set_game_filters) = signal(persisted.game_filters);

    let repeat_state = Rc::new(RefCell::new(GamepadRepeatState::default()));

    Effect::new(move || {
        crate::ui_state::save_json(
            APP_UI_STATE_KEY,
            &AppUiState {
                selected_platform: selected_platform.get(),
                selected_collection: selected_collection.get(),
                view_mode: view_mode.get(),
                search_query: search_query.get(),
                artwork_type: artwork_type.get(),
                zoom_level: zoom_level.get().clamp(0.5, 2.0),
                game_filters: game_filters.get(),
                filter_defaults_version: FILTER_DEFAULTS_VERSION_NON_RETAIL,
            },
        );
    });

    Effect::new(move || {
        spawn_local(async move {
            match crate::backend_api::get_emulator_updates().await {
                Ok(updates) => set_emulator_update_count.set(Some(updates.len())),
                Err(err) => {
                    crate::backend_api::log_to_backend(
                        "warn",
                        &format!("Failed to check emulator updates: {}", err),
                    );
                    set_emulator_update_count.set(Some(0));
                }
            }
        });
    });

    {
        let repeat_state = repeat_state.clone();
        let _gamepad_interval = Interval::new(50, move || {
            let Some(action) = next_gamepad_action(js_sys::Date::now(), &repeat_state) else {
                return;
            };

            let _ = navigation::handle_navigation_action(action);
        });

        Effect::new(move || {
            let _keep_interval_alive = &_gamepad_interval;
        });
    }

    Effect::new(move || {
        let keydown =
            wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev| {
                let Some(action) = navigation::keyboard_action(&ev) else {
                    return;
                };
                if navigation::should_ignore_keyboard_action(&ev, action) {
                    return;
                }
                if navigation::handle_navigation_action(action) {
                    ev.prevent_default();
                    ev.stop_propagation();
                }
            });

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref());
            keydown.forget();
        }
    });

    view! {
        <div class="app-container" class:details-modal-open=move || selected_game.get().is_some()>
            <Toolbar
                view_mode=view_mode
                set_view_mode=set_view_mode
                search_query=search_query
                set_search_query=set_search_query
                set_show_settings=set_show_settings
                artwork_type=artwork_type
                set_artwork_type=set_artwork_type
                zoom_level=zoom_level
                set_zoom_level=set_zoom_level
                game_filters=game_filters
                set_game_filters=set_game_filters
                emulator_update_count=emulator_update_count
                set_show_emulator_updates=set_show_emulator_updates
            />
            <div class="main-content">
                <Sidebar
                    selected_platform=selected_platform
                    set_selected_platform=set_selected_platform
                    selected_collection=selected_collection
                    set_selected_collection=set_selected_collection
                    collections_refresh=collections_refresh
                    set_collections_refresh=set_collections_refresh
                />
                <GameGrid
                    platform=selected_platform
                    collection=selected_collection
                    search_query=search_query
                    view_mode=view_mode
                    selected_game=set_selected_game
                    artwork_type=artwork_type
                    zoom_level=zoom_level
                    set_zoom_level=set_zoom_level
                    game_filters=game_filters
                />
            </div>
            <GameDetails
                game=selected_game
                on_close=set_selected_game
                set_show_settings=set_show_settings
            />
            <Settings
                show=show_settings
                on_close=set_show_settings
            />
            <EmulatorUpdates
                show=show_emulator_updates
                on_close=set_show_emulator_updates
                set_update_count=set_emulator_update_count
            />
            <MinervaDownloadQueue />
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Grid,
    List,
}
