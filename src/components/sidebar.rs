//! Sidebar component with platforms and collections

use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::tauri::{self, Platform, Collection};
use crate::components::QueueStatus;
use web_sys::console;

#[component]
pub fn Sidebar(
    selected_platform: ReadSignal<Option<String>>,
    set_selected_platform: WriteSignal<Option<String>>,
    selected_collection: ReadSignal<Option<String>>,
    set_selected_collection: WriteSignal<Option<String>>,
    collections_refresh: ReadSignal<u32>,
    set_collections_refresh: WriteSignal<u32>,
) -> impl IntoView {
    // Fetch platforms from Tauri backend
    let (platforms, set_platforms) = signal::<Vec<Platform>>(Vec::new());
    let (platforms_loading, set_platforms_loading) = signal(true);
    let (platform_search, set_platform_search) = signal(String::new());

    // Load platforms on component mount
    spawn_local(async move {
        console::log_1(&"Sidebar: Loading platforms...".into());
        match tauri::get_platforms().await {
            Ok(p) => {
                console::log_1(&format!("Sidebar: Loaded {} platforms", p.len()).into());
                set_platforms.set(p);
            }
            Err(e) => console::error_1(&format!("Failed to load platforms: {}", e).into()),
        }
        set_platforms_loading.set(false);
    });

    // Filter platforms based on search query (matches name and aliases)
    let filtered_platforms = move || {
        let query = platform_search.get().to_lowercase();
        if query.is_empty() {
            platforms.get()
        } else {
            platforms.get().into_iter().filter(|p| {
                p.name.to_lowercase().contains(&query) ||
                p.aliases.as_ref().map(|a| a.to_lowercase().contains(&query)).unwrap_or(false)
            }).collect()
        }
    };

    // Collections state
    let (collections, set_collections) = signal::<Vec<Collection>>(Vec::new());
    let (show_create_dialog, set_show_create_dialog) = signal(false);
    let (new_collection_name, set_new_collection_name) = signal(String::new());

    // Load collections and refresh when trigger changes
    Effect::new(move || {
        let _ = collections_refresh.get(); // Subscribe to refresh trigger
        spawn_local(async move {
            match tauri::get_collections().await {
                Ok(cols) => set_collections.set(cols),
                Err(e) => console::error_1(&format!("Failed to load collections: {}", e).into()),
            }
        });
    });

    // Handle creating a new collection
    let do_create_collection = move || {
        let name = new_collection_name.get();
        if name.trim().is_empty() {
            return;
        }
        spawn_local(async move {
            match tauri::create_collection(name, None).await {
                Ok(_) => {
                    set_new_collection_name.set(String::new());
                    set_show_create_dialog.set(false);
                    set_collections_refresh.update(|n| *n += 1);
                }
                Err(e) => console::error_1(&format!("Failed to create collection: {}", e).into()),
            }
        });
    };

    let on_create_click = move |_: web_sys::MouseEvent| {
        do_create_collection();
    };

    let on_create_keypress = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            do_create_collection();
        }
    };

    // Handle clicking a platform (deselect collection)
    let on_platform_click = move |platform: Option<String>| {
        set_selected_platform.set(platform);
        set_selected_collection.set(None);
    };

    // Handle clicking a collection (deselect platform)
    let on_collection_click = move |collection_id: String| {
        set_selected_collection.set(Some(collection_id));
        set_selected_platform.set(None);
    };

    view! {
        <aside class="sidebar">
            <div class="sidebar-header platform-search-header">
                <div class="platform-search-box">
                    <input
                        type="text"
                        placeholder="Search platforms..."
                        prop:value=move || platform_search.get()
                        on:input=move |ev| {
                            set_platform_search.set(event_target_value(&ev));
                        }
                    />
                    <Show when=move || !platform_search.get().is_empty()>
                        <button
                            class="search-clear"
                            on:click=move |_| set_platform_search.set(String::new())
                            title="Clear search"
                        >
                            "×"
                        </button>
                    </Show>
                </div>
            </div>
            <nav class="platform-list">
                <button
                    class="platform-item"
                    class:selected=move || selected_platform.get().is_none() && selected_collection.get().is_none()
                    on:click=move |_| on_platform_click(None)
                >
                    <span class="platform-name">"All Games"</span>
                    <span class="platform-count">
                        {move || {
                            let total: i64 = platforms.get().iter().map(|p| p.game_count).sum();
                            total.to_string()
                        }}
                    </span>
                </button>
                {move || if platforms_loading.get() {
                    view! { <div class="loading">"Loading platforms..."</div> }.into_any()
                } else if filtered_platforms().is_empty() {
                    view! { <div class="empty-platforms">"No platforms found"</div> }.into_any()
                } else {
                    view! {
                        <For
                            each=move || filtered_platforms()
                            key=|p| p.id
                            let:platform
                        >
                            <PlatformItem
                                platform=platform.clone()
                                selected_platform=selected_platform
                                on_click=on_platform_click
                                search_query=platform_search
                            />
                        </For>
                    }.into_any()
                }}
            </nav>

            <div class="sidebar-header collections-header">
                <h2>"Collections"</h2>
                <button
                    class="add-collection-btn"
                    title="Create Collection"
                    on:click=move |_| set_show_create_dialog.set(true)
                >
                    "+"
                </button>
            </div>
            <nav class="collection-list">
                <For
                    each=move || collections.get()
                    key=|c| c.id.clone()
                    let:collection
                >
                    <CollectionItem
                        collection=collection
                        selected_collection=selected_collection
                        on_click=on_collection_click
                        set_collections_refresh=set_collections_refresh
                    />
                </For>
                {move || collections.get().is_empty().then(|| view! {
                    <div class="empty-collections">"No collections yet"</div>
                })}
            </nav>

            // Download queue status
            <QueueStatus />

            // Create collection dialog
            <Show when=move || show_create_dialog.get()>
                <div class="dialog-overlay" on:click=move |_| set_show_create_dialog.set(false)>
                    <div class="dialog" on:click=|ev| ev.stop_propagation()>
                        <h3>"Create Collection"</h3>
                        <input
                            type="text"
                            class="dialog-input"
                            placeholder="Collection name"
                            prop:value=move || new_collection_name.get()
                            on:input=move |ev| set_new_collection_name.set(event_target_value(&ev))
                            on:keypress=on_create_keypress
                        />
                        <div class="dialog-actions">
                            <button
                                class="dialog-cancel"
                                on:click=move |_| set_show_create_dialog.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="dialog-confirm"
                                on:click=on_create_click
                            >
                                "Create"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </aside>
    }
}

/// Highlight matching text in a string with yellow background
fn highlight_matches(text: &str, query: &str) -> AnyView {
    if query.is_empty() {
        return view! { <>{text.to_string()}</> }.into_any();
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    // Find all match positions
    let mut parts: Vec<AnyView> = Vec::new();
    let mut last_end = 0;

    for (start, _) in text_lower.match_indices(&query_lower) {
        // Add non-matching text before this match
        if start > last_end {
            let before = &text[last_end..start];
            parts.push(view! { <>{before.to_string()}</> }.into_any());
        }
        // Add the matching text with highlight
        let matched = &text[start..start + query.len()];
        parts.push(view! { <span class="search-highlight">{matched.to_string()}</span> }.into_any());
        last_end = start + query.len();
    }

    // Add remaining text after last match
    if last_end < text.len() {
        let after = &text[last_end..];
        parts.push(view! { <>{after.to_string()}</> }.into_any());
    }

    view! { <>{parts}</> }.into_any()
}

#[component]
fn PlatformItem(
    platform: Platform,
    selected_platform: ReadSignal<Option<String>>,
    on_click: impl Fn(Option<String>) + Copy + 'static,
    search_query: ReadSignal<String>,
) -> impl IntoView {
    let name = platform.name.clone();
    let name_for_click = name.clone();
    let name_for_display = name.clone();
    let game_count = platform.game_count;

    view! {
        <button
            class="platform-item"
            class:selected=move || selected_platform.get().as_ref() == Some(&name)
            on:click=move |_| on_click(Some(name_for_click.clone()))
        >
            <span class="platform-name">
                {move || highlight_matches(&name_for_display, &search_query.get())}
            </span>
            {(game_count > 0).then(|| view! {
                <span class="platform-count">{game_count}</span>
            })}
        </button>
    }
}

#[component]
fn CollectionItem(
    collection: Collection,
    selected_collection: ReadSignal<Option<String>>,
    on_click: impl Fn(String) + Copy + 'static,
    set_collections_refresh: WriteSignal<u32>,
) -> impl IntoView {
    let id = collection.id.clone();
    let id_for_click = id.clone();
    let id_for_delete = id.clone();
    let name = collection.name.clone();
    let game_count = collection.game_count;

    let on_delete = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        let id = id_for_delete.clone();
        spawn_local(async move {
            match tauri::delete_collection(id).await {
                Ok(_) => set_collections_refresh.update(|n| *n += 1),
                Err(e) => console::error_1(&format!("Failed to delete collection: {}", e).into()),
            }
        });
    };

    view! {
        <button
            class="collection-item"
            class:selected=move || selected_collection.get().as_ref() == Some(&id)
            on:click=move |_| on_click(id_for_click.clone())
        >
            <span class="collection-name">{name}</span>
            <span class="collection-count">{game_count}</span>
            <button
                class="delete-collection-btn"
                title="Delete Collection"
                on:click=on_delete
            >
                "x"
            </button>
        </button>
    }
}
