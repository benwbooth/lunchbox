use leptos::prelude::*;
use crate::components::{Sidebar, GameGrid, GameDetails, Toolbar, Settings};
use crate::tauri::Game;

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

    view! {
        <div class="app-container">
            <Toolbar
                view_mode=view_mode
                set_view_mode=set_view_mode
                search_query=search_query
                set_search_query=set_search_query
                set_show_settings=set_show_settings
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
