use crate::components::{GameDetails, GameGrid, Settings, Sidebar, Toolbar};
use crate::tauri::Game;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

pub const PLATFORM_SELECTION_MINIGAMES: &str = "__minigames__";
pub const PLATFORM_SELECTION_ALL_GAMES: &str = "__all_games__";
const APP_UI_STATE_KEY: &str = "lunchbox.ui.app.v1";

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

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GameFilters {
    pub installed_only: bool,
    pub hide_homebrew: bool,
    pub hide_adult: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct AppUiState {
    selected_platform: Option<String>,
    selected_collection: Option<String>,
    view_mode: ViewMode,
    search_query: String,
    artwork_type: ArtworkDisplayType,
    zoom_level: f64,
    game_filters: GameFilters,
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
    // Trigger for refreshing collections
    let (collections_refresh, set_collections_refresh) = signal(0u32);
    // State for artwork display type in grid
    let (artwork_type, set_artwork_type) = signal(persisted.artwork_type);
    // State for zoom level (0.5 to 2.0, default 1.0)
    let (zoom_level, set_zoom_level) = signal(persisted.zoom_level.clamp(0.5, 2.0));
    // State for game list filters
    let (game_filters, set_game_filters) = signal(persisted.game_filters);

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
            },
        );
    });

    view! {
        <div class="app-container">
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
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Grid,
    List,
}
