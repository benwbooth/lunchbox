use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::app::{ViewMode, ArtworkDisplayType};
use crate::tauri;

/// Frontend build info (embedded at compile time)
const FRONTEND_BUILD_HASH: &str = env!("BUILD_HASH");
const FRONTEND_BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");

/// Status indicator showing backend connectivity and build info
#[component]
fn StatusIndicator() -> impl IntoView {
    let (is_connected, set_is_connected) = signal(false);
    let (backend_hash, set_backend_hash) = signal::<Option<String>>(None);
    let (backend_timestamp, set_backend_timestamp) = signal::<Option<String>>(None);

    // Initial health check
    spawn_local(async move {
        match tauri::check_health().await {
            Ok(health) => {
                set_is_connected.set(true);
                set_backend_hash.set(Some(health.build_hash));
                set_backend_timestamp.set(Some(health.build_timestamp));
            }
            Err(_) => {
                set_is_connected.set(false);
            }
        }
    });

    // Set up polling interval (runs once on mount)
    use gloo_timers::callback::Interval;
    let interval = Interval::new(10000, move || {
        spawn_local(async move {
            match tauri::check_health().await {
                Ok(health) => {
                    set_is_connected.set(true);
                    set_backend_hash.set(Some(health.build_hash));
                    set_backend_timestamp.set(Some(health.build_timestamp));
                }
                Err(_) => {
                    set_is_connected.set(false);
                }
            }
        });
    });
    interval.forget();

    // Build tooltip text
    let tooltip = move || {
        let status = if is_connected.get() { "Connected" } else { "Disconnected" };
        let be_hash = backend_hash.get().unwrap_or_else(|| "unknown".to_string());
        let be_time = backend_timestamp.get().unwrap_or_else(|| "unknown".to_string());
        format!(
            "Backend: {}\nBackend build: {} ({})\nFrontend build: {} ({})",
            status, be_hash, be_time, FRONTEND_BUILD_HASH, FRONTEND_BUILD_TIMESTAMP
        )
    };

    view! {
        <div class="status-indicator" title=tooltip>
            <span
                class="status-dot"
                class:connected=move || is_connected.get()
                class:disconnected=move || !is_connected.get()
            />
            <span class="status-info">
                <span class="status-label">"BE:"</span>
                <span class="status-hash">{move || backend_hash.get().unwrap_or_else(|| "...".to_string())}</span>
                <span class="status-label">"FE:"</span>
                <span class="status-hash">{FRONTEND_BUILD_HASH}</span>
            </span>
        </div>
    }
}

#[component]
pub fn Toolbar(
    view_mode: ReadSignal<ViewMode>,
    set_view_mode: WriteSignal<ViewMode>,
    search_query: ReadSignal<String>,
    set_search_query: WriteSignal<String>,
    set_show_settings: WriteSignal<bool>,
    artwork_type: ReadSignal<ArtworkDisplayType>,
    set_artwork_type: WriteSignal<ArtworkDisplayType>,
) -> impl IntoView {
    view! {
        <header class="toolbar">
            <div class="toolbar-left">
                <img src="/assets/logo.svg" alt="Lunchbox" class="app-logo" />
                <h1 class="app-title">"Lunchbox"</h1>
                <StatusIndicator />
            </div>
            <div class="toolbar-center">
                <div class="search-box">
                    <input
                        type="text"
                        placeholder="Search games..."
                        prop:value=move || search_query.get()
                        on:input=move |ev| {
                            set_search_query.set(event_target_value(&ev));
                        }
                    />
                    <Show when=move || !search_query.get().is_empty()>
                        <button
                            class="search-clear"
                            on:click=move |_| set_search_query.set(String::new())
                            title="Clear search"
                        >
                            "×"
                        </button>
                    </Show>
                </div>
            </div>
            <div class="toolbar-right">
                // Artwork type dropdown (only show in grid view)
                <Show when=move || view_mode.get() == ViewMode::Grid>
                    <select
                        class="artwork-dropdown"
                        prop:value=move || artwork_type.get().media_type_id()
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            let art_type = match value.as_str() {
                                "box-front" => ArtworkDisplayType::BoxFront,
                                "screenshot" => ArtworkDisplayType::Screenshot,
                                "title-screen" => ArtworkDisplayType::TitleScreen,
                                "fanart" => ArtworkDisplayType::Fanart,
                                "clear-logo" => ArtworkDisplayType::ClearLogo,
                                _ => ArtworkDisplayType::BoxFront,
                            };
                            set_artwork_type.set(art_type);
                        }
                    >
                        <For
                            each=move || ArtworkDisplayType::all().iter().copied()
                            key=|at| at.media_type_id()
                            children=move |at| {
                                view! {
                                    <option value=at.media_type_id()>
                                        {at.label()}
                                    </option>
                                }
                            }
                        />
                    </select>
                </Show>
                <div class="view-toggle">
                    <button
                        class="view-btn"
                        class:active=move || view_mode.get() == ViewMode::Grid
                        on:click=move |_| set_view_mode.set(ViewMode::Grid)
                        title="Grid View"
                    >
                        "Grid"
                    </button>
                    <button
                        class="view-btn"
                        class:active=move || view_mode.get() == ViewMode::List
                        on:click=move |_| set_view_mode.set(ViewMode::List)
                        title="List View"
                    >
                        "List"
                    </button>
                </div>
                <button class="import-btn" title="Import ROMs">
                    "Import"
                </button>
                <button
                    class="settings-btn"
                    title="Settings"
                    on:click=move |_| set_show_settings.set(true)
                >
                    "Settings"
                </button>
            </div>
        </header>
    }
}
