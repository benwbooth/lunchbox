use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::app::{ViewMode, ArtworkDisplayType};
use crate::tauri;

/// Frontend build info (embedded at compile time)
const FRONTEND_BUILD_HASH: &str = env!("BUILD_HASH");
const FRONTEND_BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");

/// Format relative time in minutes (e.g., "now", "1m ago", "5m ago", "1h ago")
fn format_relative_time(minutes_ago: i64) -> String {
    if minutes_ago < 1 {
        "now".to_string()
    } else if minutes_ago < 60 {
        format!("{}m ago", minutes_ago)
    } else if minutes_ago < 1440 {
        let hours = minutes_ago / 60;
        format!("{}h ago", hours)
    } else {
        let days = minutes_ago / 1440;
        format!("{}d ago", days)
    }
}

/// Get current time in minutes since epoch
fn now_minutes() -> i64 {
    (js_sys::Date::now() / 60000.0) as i64
}

/// Status indicator showing backend connectivity and build info
#[component]
fn StatusIndicator() -> impl IntoView {
    let (is_connected, set_is_connected) = signal(false);
    let (backend_hash, set_backend_hash) = signal::<Option<String>>(None);
    let (backend_timestamp, set_backend_timestamp) = signal::<Option<String>>(None);
    let (backend_last_updated, set_backend_last_updated) = signal::<Option<i64>>(None);
    let (current_minute, set_current_minute) = signal(now_minutes());

    // Frontend load time (constant for this session)
    let frontend_loaded_at = now_minutes();

    // Initial health check
    spawn_local(async move {
        match tauri::check_health().await {
            Ok(health) => {
                set_is_connected.set(true);
                set_backend_hash.set(Some(health.build_hash));
                set_backend_timestamp.set(Some(health.build_timestamp));
                set_backend_last_updated.set(Some(now_minutes()));
            }
            Err(_) => {
                set_is_connected.set(false);
            }
        }
    });

    // Set up polling interval for health checks (every 10 seconds)
    use gloo_timers::callback::Interval;
    let health_interval = Interval::new(10000, move || {
        spawn_local(async move {
            match tauri::check_health().await {
                Ok(health) => {
                    set_is_connected.set(true);
                    set_backend_hash.set(Some(health.build_hash));
                    set_backend_timestamp.set(Some(health.build_timestamp));
                    // Only set last_updated if not already set (first successful check)
                    if backend_last_updated.get_untracked().is_none() {
                        set_backend_last_updated.set(Some(now_minutes()));
                    }
                }
                Err(_) => {
                    set_is_connected.set(false);
                }
            }
        });
    });
    health_interval.forget();

    // Update current minute every 60 seconds for relative time display
    let minute_interval = Interval::new(60000, move || {
        set_current_minute.set(now_minutes());
    });
    minute_interval.forget();

    // Compute relative times
    let backend_relative = move || {
        let _ = current_minute.get(); // Subscribe to updates
        backend_last_updated.get().map(|updated_at| {
            let minutes_ago = now_minutes() - updated_at;
            format!(" ({})", format_relative_time(minutes_ago))
        }).unwrap_or_default()
    };

    let frontend_relative = move || {
        let _ = current_minute.get(); // Subscribe to updates
        let minutes_ago = now_minutes() - frontend_loaded_at;
        format!(" ({})", format_relative_time(minutes_ago))
    };

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
                <span class="status-time">{move || backend_timestamp.get().unwrap_or_else(|| "".to_string())}{backend_relative}</span>
                <span class="status-label">"FE:"</span>
                <span class="status-hash">{FRONTEND_BUILD_HASH}</span>
                <span class="status-time">{FRONTEND_BUILD_TIMESTAMP}{frontend_relative}</span>
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
