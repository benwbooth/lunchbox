use leptos::prelude::*;
use crate::components::{Sidebar, GameGrid, GameDetails, Toolbar, Settings};
use crate::tauri::Game;

/// Artwork type to display in grid view
#[derive(Clone, Copy, PartialEq, Eq, Default)]
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

#[component]
pub fn App() -> impl IntoView {
    // State for selected platform (now uses platform name)
    let (selected_platform, set_selected_platform) = signal::<Option<String>>(None);
    // State for selected collection
    let (selected_collection, set_selected_collection) = signal::<Option<String>>(None);
    // State for view mode (grid/list)
    let (view_mode, set_view_mode) = signal(ViewMode::Grid);
    // State for search query
    let (search_query, set_search_query) = signal(String::new());
    // State for selected game (for details panel)
    let (selected_game, set_selected_game) = signal::<Option<Game>>(None);
    // State for settings panel
    let (show_settings, set_show_settings) = signal(false);
    // Trigger for refreshing collections
    let (collections_refresh, set_collections_refresh) = signal(0u32);
    // State for artwork display type in grid
    let (artwork_type, set_artwork_type) = signal(ArtworkDisplayType::default());

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
                />
            </div>
            <GameDetails
                game=selected_game
                on_close=set_selected_game
            />
            <Settings
                show=show_settings
                on_close=set_show_settings
            />
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Grid,
    List,
}
