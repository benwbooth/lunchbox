//! Game grid and list views with virtual scrolling

use leptos::prelude::*;
use leptos::html;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::console;
use crate::app::ViewMode;
use crate::tauri::{self, file_to_asset_url, Game};

// Virtual scroll configuration
const ITEM_HEIGHT: i32 = 280; // Height of each game card in grid
const ITEM_WIDTH: i32 = 180;  // Width of each game card
const LIST_ITEM_HEIGHT: i32 = 40; // Height in list view
const BUFFER_ITEMS: i32 = 10; // Extra items to render above/below viewport
const FETCH_CHUNK_SIZE: i64 = 500; // How many games to fetch at once

#[component]
pub fn GameGrid(
    platform: ReadSignal<Option<String>>,
    collection: ReadSignal<Option<String>>,
    search_query: ReadSignal<String>,
    view_mode: ReadSignal<ViewMode>,
    selected_game: WriteSignal<Option<Game>>,
) -> impl IntoView {
    // Games cache - we load chunks as needed
    let (games, set_games) = signal::<Vec<Game>>(Vec::new());
    let (total_count, set_total_count) = signal(0i64);
    let (loading, set_loading) = signal(false);
    let (loaded_up_to, set_loaded_up_to) = signal(0i64); // How many games we've loaded

    // Scroll state
    let (scroll_top, set_scroll_top) = signal(0);
    let (container_height, set_container_height) = signal(600);
    let (container_width, set_container_width) = signal(800);

    // Track current filters
    let (current_platform, set_current_platform) = signal::<Option<String>>(None);
    let (current_collection, set_current_collection) = signal::<Option<String>>(None);
    let (current_search, set_current_search) = signal(String::new());

    // Container ref for scroll handling
    let container_ref = NodeRef::<html::Main>::new();


    // Load more games if needed
    let ensure_loaded = move |needed_index: i64| {
        let current_loaded = loaded_up_to.get();
        let total = total_count.get();

        if needed_index >= current_loaded && current_loaded < total && !loading.get() {
            let plat = current_platform.get();
            let search = current_search.get();
            let offset = current_loaded;

            set_loading.set(true);

            spawn_local(async move {
                let search_param = if search.is_empty() { None } else { Some(search) };
                match tauri::get_games(plat, search_param, Some(FETCH_CHUNK_SIZE), Some(offset)).await {
                    Ok(new_games) => {
                        let count = new_games.len() as i64;
                        set_games.update(|g| g.extend(new_games));
                        set_loaded_up_to.update(|l| *l += count);
                    }
                    Err(e) => {
                        console::error_1(&format!("Failed to load games: {}", e).into());
                    }
                }
                set_loading.set(false);
            });
        }
    };

    // Handle scroll events
    let on_scroll = move |ev: web_sys::Event| {
        if let Some(target) = ev.target() {
            let element: web_sys::HtmlElement = target.unchecked_into();
            set_scroll_top.set(element.scroll_top());
            set_container_height.set(element.client_height());
            set_container_width.set(element.client_width());
        }
    };

    // Watch for filter changes and reset
    Effect::new(move || {
        let plat = platform.get();
        let coll = collection.get();
        let search = search_query.get();

        if plat != current_platform.get() || coll != current_collection.get() || search != current_search.get() {
            set_current_platform.set(plat.clone());
            set_current_collection.set(coll.clone());
            set_current_search.set(search.clone());
            set_games.set(Vec::new());
            set_loaded_up_to.set(0);
            set_total_count.set(0);
            set_scroll_top.set(0);
            set_loading.set(true);

            spawn_local(async move {
                // Collections - load all (they're small)
                if let Some(coll_id) = coll {
                    let result = tauri::get_collection_games(coll_id)
                        .await
                        .unwrap_or_default();
                    let count = result.len() as i64;
                    set_games.set(result);
                    set_total_count.set(count);
                    set_loaded_up_to.set(count);
                    set_loading.set(false);
                    return;
                }

                // Load all games (backend returns deduplicated results)
                let search_param = if search.is_empty() { None } else { Some(search.clone()) };

                match tauri::get_games(plat.clone(), search_param.clone(), None, None).await {
                    Ok(all_games) => {
                        let count = all_games.len() as i64;
                        set_games.set(all_games);
                        set_total_count.set(count);
                        set_loaded_up_to.set(count);
                    }
                    Err(e) => {
                        console::error_1(&format!("Failed to load games: {}", e).into());
                    }
                }
                set_loading.set(false);
            });
        }
    });

    // Calculate visible items based on scroll
    let visible_range = move || {
        let mode = view_mode.get();
        let scroll = scroll_top.get();
        let height = container_height.get();
        let width = container_width.get();
        let total = total_count.get() as i32;

        match mode {
            ViewMode::Grid => {
                let cols = (width / ITEM_WIDTH).max(1);
                let rows_before = scroll / ITEM_HEIGHT;
                let visible_rows = (height / ITEM_HEIGHT) + 2;

                let start = ((rows_before - BUFFER_ITEMS).max(0) * cols) as usize;
                let end = (((rows_before + visible_rows + BUFFER_ITEMS) * cols) as usize).min(total as usize);

                (start, end, cols as usize)
            }
            ViewMode::List => {
                let start = ((scroll / LIST_ITEM_HEIGHT) - BUFFER_ITEMS).max(0) as usize;
                let visible = height / LIST_ITEM_HEIGHT;
                let end = ((scroll / LIST_ITEM_HEIGHT) + visible + BUFFER_ITEMS * 2).min(total) as usize;

                (start, end, 1)
            }
        }
    };

    // Calculate total content height
    let total_height = move || {
        let mode = view_mode.get();
        let total = total_count.get() as i32;
        let width = container_width.get();

        match mode {
            ViewMode::Grid => {
                let cols = (width / ITEM_WIDTH).max(1);
                let rows = (total + cols - 1) / cols;
                rows * ITEM_HEIGHT
            }
            ViewMode::List => {
                total * LIST_ITEM_HEIGHT
            }
        }
    };

    view! {
        <main
            class="game-content virtual-scroll"
            node_ref=container_ref
            on:scroll=on_scroll
        >
            {move || {
                let games_list = games.get();
                let is_loading = loading.get() && games_list.is_empty();
                let total = total_count.get();

                if is_loading {
                    view! { <div class="loading">"Loading games..."</div> }.into_any()
                } else if total == 0 && !loading.get() {
                    let message = if collection.get().is_some() {
                        "This collection is empty. Add games to see them here."
                    } else if platform.get().is_some() {
                        "No games found for this platform."
                    } else {
                        "Select a platform to view games."
                    };
                    view! {
                        <div class="empty-state">
                            <p>{message}</p>
                        </div>
                    }.into_any()
                } else {
                    // Measure container dimensions if not yet initialized
                    if let Some(container) = container_ref.get() {
                        let w = container.client_width();
                        let h = container.client_height();
                        if w > 0 && container_width.get() != w {
                            set_container_width.set(w);
                        }
                        if h > 0 && container_height.get() != h {
                            set_container_height.set(h);
                        }
                    }

                    let mode = view_mode.get();
                    let height = total_height();
                    let (start, end, cols) = visible_range();

                    // Ensure we have enough data loaded
                    ensure_loaded(end as i64);

                    // Get visible slice of games
                    let visible_games: Vec<(usize, Game)> = games_list
                        .iter()
                        .enumerate()
                        .skip(start)
                        .take(end - start)
                        .map(|(i, g)| (i, g.clone()))
                        .collect();

                    view! {
                        <div
                            class="virtual-container"
                            style:height=format!("{}px", height)
                        >
                            {match mode {
                                ViewMode::Grid => view! {
                                    <div class="game-grid virtual-grid">
                                        {visible_games.into_iter().map(|(index, game)| {
                                            let row = index / cols;
                                            let col = index % cols;
                                            let top = row as i32 * ITEM_HEIGHT;
                                            let left = col as i32 * ITEM_WIDTH;

                                            view! {
                                                <div
                                                    class="virtual-item"
                                                    style:position="absolute"
                                                    style:top=format!("{}px", top)
                                                    style:left=format!("{}px", left)
                                                    style:width=format!("{}px", ITEM_WIDTH)
                                                    style:height=format!("{}px", ITEM_HEIGHT)
                                                >
                                                    <GameCard game=game on_select=selected_game />
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any(),
                                ViewMode::List => view! {
                                    <div class="game-list virtual-list">
                                        <div
                                            class="game-list-header"
                                            style:position="sticky"
                                            style:top="0"
                                            style:z-index="10"
                                        >
                                            <span>"Title"</span>
                                            <span>"Platform"</span>
                                            <span>"Developer"</span>
                                            <span>"Year"</span>
                                        </div>
                                        {visible_games.into_iter().map(|(index, game)| {
                                            let top = index as i32 * LIST_ITEM_HEIGHT;

                                            view! {
                                                <div
                                                    class="virtual-item"
                                                    style:position="absolute"
                                                    style:top=format!("{}px", top + LIST_ITEM_HEIGHT) // Account for header
                                                    style:left="0"
                                                    style:right="0"
                                                    style:height=format!("{}px", LIST_ITEM_HEIGHT)
                                                >
                                                    <GameListItem game=game on_select=selected_game />
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any(),
                            }}
                            // Loading indicator at bottom
                            {move || loading.get().then(|| view! {
                                <div class="loading-more">"Loading more..."</div>
                            })}
                            // Game count
                            <div class="game-count">
                                {move || format!("{} games", total_count.get())}
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </main>
    }
}

#[component]
fn GameCard(game: Game, on_select: WriteSignal<Option<Game>>) -> impl IntoView {
    let display_title = game.display_title.clone();
    let first_char = game.display_title.chars().next().unwrap_or('?').to_string();
    let developer = game.developer.clone();
    let variant_count = game.variant_count;
    let box_front = game.box_front_path.clone();

    // Debug: log variant_count for first few games
    if game.display_title.starts_with("A") || game.display_title.starts_with("B") {
        console::log_1(&format!("GameCard '{}': variant_count={}", display_title, variant_count).into());
    }
    let game_for_click = game.clone();

    view! {
        <div
            class="game-card"
            on:click=move |_| on_select.set(Some(game_for_click.clone()))
        >
            <div class="game-cover">
                {match box_front {
                    Some(path) => {
                        let url = file_to_asset_url(&path);
                        view! {
                            <img
                                src=url
                                alt=display_title.clone()
                                class="cover-image"
                                loading="lazy"
                            />
                        }.into_any()
                    }
                    None => view! {
                        <div class="cover-placeholder">{first_char.clone()}</div>
                    }.into_any()
                }}
                {(variant_count > 1).then(|| view! {
                    <span class="variant-badge">{variant_count}</span>
                })}
            </div>
            <div class="game-info">
                <h3 class="game-title">{display_title}</h3>
                {developer.map(|d| view! { <p class="game-developer">{d}</p> })}
            </div>
        </div>
    }
}

#[component]
fn GameListItem(game: Game, on_select: WriteSignal<Option<Game>>) -> impl IntoView {
    let display_title = game.display_title.clone();
    let platform = game.platform.clone();
    let developer = game.developer.clone().unwrap_or_else(|| "-".to_string());
    let year = game.release_year.map(|y| y.to_string()).unwrap_or_else(|| "-".to_string());
    let variant_count = game.variant_count;
    let game_for_click = game.clone();

    view! {
        <div
            class="game-list-item"
            on:click=move |_| on_select.set(Some(game_for_click.clone()))
        >
            <span class="game-title">
                {display_title}
                {(variant_count > 1).then(|| view! {
                    <span class="variant-count">{format!(" ({})", variant_count)}</span>
                })}
            </span>
            <span class="game-platform">{platform}</span>
            <span class="game-developer">{developer}</span>
            <span class="game-year">{year}</span>
        </div>
    }
}
