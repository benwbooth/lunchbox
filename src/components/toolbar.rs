use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::app::{ViewMode, ArtworkDisplayType};
use crate::tauri;

/// Format seconds ago as human readable string
fn format_time_ago(seconds: u32) -> String {
    if seconds < 5 {
        "now".to_string()
    } else if seconds < 60 {
        format!("{}s ago", seconds)
    } else if seconds < 3600 {
        format!("{}m ago", seconds / 60)
    } else {
        format!("{}h ago", seconds / 3600)
    }
}

/// Status indicator showing backend connectivity
#[component]
fn StatusIndicator() -> impl IntoView {
    let (is_connected, set_is_connected) = signal(false);
    let (backend_hash, set_backend_hash) = signal::<Option<String>>(None);
    let (last_check_secs, set_last_check_secs) = signal(0u32);
    let (check_count, set_check_count) = signal(0u32);

    // Poll health every 10 seconds
    Effect::new(move || {
        let _ = check_count.get(); // Subscribe to trigger re-checks
        spawn_local(async move {
            match tauri::check_health().await {
                Ok(health) => {
                    set_is_connected.set(true);
                    set_backend_hash.set(Some(health.build_hash));
                    set_last_check_secs.set(0);
                }
                Err(_) => {
                    set_is_connected.set(false);
                }
            }
        });
    });

    // Increment timer every second
    Effect::new(move || {
        use gloo_timers::callback::Interval;
        let interval = Interval::new(1000, move || {
            set_last_check_secs.update(|s| *s += 1);
            // Trigger health check every 10 seconds
            if last_check_secs.get_untracked() >= 10 {
                set_check_count.update(|c| *c += 1);
            }
        });
        interval.forget();
    });

    // Build tooltip text
    let tooltip = move || {
        let status = if is_connected.get() { "Connected" } else { "Disconnected" };
        let hash = backend_hash.get().unwrap_or_else(|| "unknown".to_string());
        let ago = format_time_ago(last_check_secs.get());
        format!("Backend: {}\nBuild: {}\nChecked: {}", status, hash, ago)
    };

    view! {
        <div class="status-indicator" data-tooltip=tooltip>
            <span
                class="status-dot"
                class:connected=move || is_connected.get()
                class:disconnected=move || !is_connected.get()
            />
            <span class="status-hash">
                {move || backend_hash.get().unwrap_or_else(|| "...".to_string())}
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
                <StatusIndicator />
                <h1 class="app-title">"Lunchbox"</h1>
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
