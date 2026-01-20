//! Game grid and list views with virtual scrolling

use leptos::prelude::*;
use leptos::html;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::console;
use chrono::{Datelike, NaiveDate};
use crate::app::{ViewMode, ArtworkDisplayType};
use crate::tauri::{self, Game};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Format a number with comma separators (e.g., 1234567 -> "1,234,567")
fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

// Virtual scroll configuration
const ITEM_HEIGHT: i32 = 280; // Height of each game card in grid

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
const ITEM_WIDTH: i32 = 180;  // Width of each game card
const LIST_ITEM_HEIGHT: i32 = 40; // Height in list view
const BUFFER_ITEMS: i32 = 10; // Extra items to render above/below viewport
const FETCH_CHUNK_SIZE: i64 = 500; // How many games to fetch at once

/// Parse a date string into a NaiveDate, handling various formats
fn parse_date(date_str: &str) -> Option<NaiveDate> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("canceled") {
        return None;
    }

    // Try ISO format first: "2024-01-15T00:00:00+00:00" or "2024-01-15"
    let date_part = trimmed.split('T').next().unwrap_or(trimmed);
    if let Ok(d) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
        return Some(d);
    }

    // Try just year: "1983"
    if let Ok(year) = trimmed.parse::<i32>() {
        if (1900..=2100).contains(&year) {
            return NaiveDate::from_ymd_opt(year, 1, 1);
        }
    }

    // Try "Month Year" format: "July 1990"
    let months = [
        ("january", 1), ("february", 2), ("march", 3), ("april", 4),
        ("may", 5), ("june", 6), ("july", 7), ("august", 8),
        ("september", 9), ("october", 10), ("november", 11), ("december", 12),
    ];
    let lower = trimmed.to_lowercase();
    for (month_name, month_num) in months {
        if lower.starts_with(month_name) {
            if let Some(year_str) = lower.strip_prefix(month_name).map(|s| s.trim()) {
                if let Ok(year) = year_str.parse::<i32>() {
                    return NaiveDate::from_ymd_opt(year, month_num, 1);
                }
            }
        }
    }

    None
}

/// Format a date string to a human-readable format
fn format_date(date_str: &str) -> String {
    if let Some(date) = parse_date(date_str) {
        // If it's Jan 1, likely just a year - show year only
        if date.day() == 1 && date.month() == 1 {
            return date.format("%Y").to_string();
        }
        // If it's the 1st of any month, might be "Month Year" - show month and year
        if date.day() == 1 {
            return date.format("%b %Y").to_string();
        }
        date.format("%b %-d, %Y").to_string()
    } else {
        date_str.to_string()
    }
}

/// Get a sortable date string (YYYY-MM-DD) for comparison
fn sortable_date(date_str: &Option<String>) -> String {
    date_str
        .as_ref()
        .and_then(|s| parse_date(s))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

// ============================================================================
// Column Configuration
// ============================================================================

/// Available columns for list view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    Title,
    Platform,
    Developer,
    Publisher,
    Year,
    ReleaseDate,
    Genre,
    Players,
    Rating,
    Esrb,
    Coop,
    Variants,
    ReleaseType,
    Series,
    Region,
    Notes,
}

impl Column {
    /// Get the display name for the column header
    pub fn label(&self) -> &'static str {
        match self {
            Column::Title => "Title",
            Column::Platform => "Platform",
            Column::Developer => "Developer",
            Column::Publisher => "Publisher",
            Column::Year => "Year",
            Column::ReleaseDate => "Release Date",
            Column::Genre => "Genre",
            Column::Players => "Players",
            Column::Rating => "Rating",
            Column::Esrb => "ESRB",
            Column::Coop => "Co-op",
            Column::Variants => "Variants",
            Column::ReleaseType => "Type",
            Column::Series => "Series",
            Column::Region => "Region",
            Column::Notes => "Notes",
        }
    }

    /// Get the value from a game for this column
    pub fn value(&self, game: &Game) -> String {
        match self {
            Column::Title => game.display_title.clone(),
            Column::Platform => game.platform.clone(),
            Column::Developer => game.developer.clone().unwrap_or_else(|| "-".to_string()),
            Column::Publisher => game.publisher.clone().unwrap_or_else(|| "-".to_string()),
            Column::Year => game.release_year.map(|y| y.to_string()).unwrap_or_else(|| "-".to_string()),
            Column::ReleaseDate => game.release_date.as_ref().map(|d| format_date(d)).unwrap_or_else(|| "-".to_string()),
            Column::Genre => game.genres.clone().unwrap_or_else(|| "-".to_string()),
            Column::Players => game.players.clone().unwrap_or_else(|| "-".to_string()),
            Column::Rating => game.rating.map(|r| format!("{:.1}", r)).unwrap_or_else(|| "-".to_string()),
            Column::Esrb => game.esrb.clone().unwrap_or_else(|| "-".to_string()),
            Column::Coop => game.cooperative.map(|c| if c { "Yes" } else { "No" }.to_string()).unwrap_or_else(|| "-".to_string()),
            Column::Variants => if game.variant_count > 1 { game.variant_count.to_string() } else { "-".to_string() },
            Column::ReleaseType => game.release_type.clone().unwrap_or_else(|| "-".to_string()),
            Column::Series => game.series.clone().unwrap_or_else(|| "-".to_string()),
            Column::Region => game.region.clone().unwrap_or_else(|| "-".to_string()),
            Column::Notes => game.notes.clone().map(|n| if n.len() > 50 { format!("{}...", &n[..47]) } else { n }).unwrap_or_else(|| "-".to_string()),
        }
    }

    /// Compare two games by this column
    pub fn compare(&self, a: &Game, b: &Game) -> Ordering {
        match self {
            Column::Title => a.display_title.to_lowercase().cmp(&b.display_title.to_lowercase()),
            Column::Platform => a.platform.to_lowercase().cmp(&b.platform.to_lowercase()),
            Column::Developer => cmp_opt_str(&a.developer, &b.developer),
            Column::Publisher => cmp_opt_str(&a.publisher, &b.publisher),
            Column::Year => cmp_opt(&a.release_year, &b.release_year),
            Column::ReleaseDate => sortable_date(&a.release_date).cmp(&sortable_date(&b.release_date)),
            Column::Genre => cmp_opt_str(&a.genres, &b.genres),
            Column::Players => cmp_opt_str(&a.players, &b.players),
            Column::Rating => cmp_opt_f64(&a.rating, &b.rating),
            Column::Esrb => cmp_opt_str(&a.esrb, &b.esrb),
            Column::Coop => cmp_opt(&a.cooperative, &b.cooperative),
            Column::Variants => a.variant_count.cmp(&b.variant_count),
            Column::ReleaseType => cmp_opt_str(&a.release_type, &b.release_type),
            Column::Series => cmp_opt_str(&a.series, &b.series),
            Column::Region => cmp_opt_str(&a.region, &b.region),
            Column::Notes => cmp_opt_str(&a.notes, &b.notes),
        }
    }

    /// Get all available columns
    pub fn all() -> Vec<Column> {
        vec![
            Column::Title,
            Column::Platform,
            Column::Developer,
            Column::Publisher,
            Column::Year,
            Column::ReleaseDate,
            Column::Genre,
            Column::Players,
            Column::Rating,
            Column::Esrb,
            Column::Coop,
            Column::Variants,
            Column::ReleaseType,
            Column::Series,
            Column::Region,
            Column::Notes,
        ]
    }

    /// Get default visible columns
    pub fn default_visible() -> Vec<Column> {
        vec![
            Column::Title,
            Column::Platform,
            Column::Developer,
            Column::Publisher,
            Column::Year,
        ]
    }
}

/// Helper functions for comparing optional values
fn cmp_opt<T: Ord>(a: &Option<T>, b: &Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn cmp_opt_str(a: &Option<String>, b: &Option<String>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn cmp_opt_f64(a: &Option<f64>, b: &Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Current sort state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortState {
    pub column: Column,
    pub direction: SortDirection,
}

/// Column filters - maps column to set of allowed values (empty = no filter)
pub type ColumnFilters = HashMap<Column, HashSet<String>>;

impl Column {
    /// Get unique values for this column from a list of games (for filter dropdown)
    pub fn unique_values(&self, games: &[Game]) -> Vec<String> {
        let mut values: HashSet<String> = HashSet::new();
        for game in games {
            let val = self.value(game);
            if val != "-" {
                values.insert(val);
            }
        }
        let mut sorted: Vec<String> = values.into_iter().collect();
        sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        sorted
    }

    /// Check if a game passes the filter for this column
    pub fn passes_filter(&self, game: &Game, allowed: &HashSet<String>) -> bool {
        if allowed.is_empty() {
            return true; // No filter = all pass
        }
        let val = self.value(game);
        allowed.contains(&val) || (val == "-" && allowed.contains("-"))
    }
}

#[component]
pub fn GameGrid(
    platform: ReadSignal<Option<String>>,
    collection: ReadSignal<Option<String>>,
    search_query: ReadSignal<String>,
    view_mode: ReadSignal<ViewMode>,
    selected_game: WriteSignal<Option<Game>>,
    artwork_type: ReadSignal<ArtworkDisplayType>,
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

    // Column configuration for list view
    let (visible_columns, set_visible_columns) = signal(Column::default_visible());
    let (sort_state, set_sort_state) = signal::<Option<SortState>>(None);
    let (context_menu, set_context_menu) = signal::<Option<(i32, i32)>>(None); // (x, y) position
    let (dragging_column, set_dragging_column) = signal::<Option<usize>>(None);
    let (drag_over_column, set_drag_over_column) = signal::<Option<usize>>(None);

    // Column filters
    let (column_filters, set_column_filters) = signal::<ColumnFilters>(HashMap::new());
    let (filter_menu, set_filter_menu) = signal::<Option<(Column, i32, i32)>>(None); // (column, x, y) position

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

    // Find the index of the first game starting with a given character
    let find_first_game_index = move |ch: char| -> Option<usize> {
        let games_list = games.get();
        let ch_lower = ch.to_ascii_lowercase();

        games_list.iter().position(|game| {
            game.display_title
                .chars()
                .next()
                .map(|c| c.to_ascii_lowercase() == ch_lower)
                .unwrap_or(false)
        })
    };

    // Scroll to a specific game index
    let scroll_to_index = move |index: usize| {
        let mode = view_mode.get();
        let width = container_width.get();

        let scroll_pos = match mode {
            ViewMode::Grid => {
                let cols = (width / ITEM_WIDTH).max(1);
                let row = index as i32 / cols;
                row * ITEM_HEIGHT
            }
            ViewMode::List => {
                (index as i32 + 1) * LIST_ITEM_HEIGHT // +1 for header
            }
        };

        if let Some(container) = container_ref.get() {
            container.set_scroll_top(scroll_pos);
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

                    // Calculate actual viewport range (without buffer) for priority
                    let scroll = scroll_top.get();
                    let ch = container_height.get();
                    let (viewport_start, viewport_end) = match mode {
                        ViewMode::Grid => {
                            let rows_before = scroll / ITEM_HEIGHT;
                            let visible_rows = (ch / ITEM_HEIGHT) + 2;
                            let vs = (rows_before * cols as i32) as usize;
                            let ve = ((rows_before + visible_rows) * cols as i32) as usize;
                            (vs, ve)
                        }
                        ViewMode::List => {
                            let vs = (scroll / LIST_ITEM_HEIGHT) as usize;
                            let visible = ch / LIST_ITEM_HEIGHT;
                            let ve = ((scroll / LIST_ITEM_HEIGHT) + visible) as usize;
                            (vs, ve)
                        }
                    };

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

                    // Get current artwork type (accessing signal here ensures re-render when it changes)
                    let current_artwork_type = artwork_type.get();

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
                                            let in_viewport = index >= viewport_start && index < viewport_end;

                                            view! {
                                                <div
                                                    class="virtual-item"
                                                    style:position="absolute"
                                                    style:top=format!("{}px", top)
                                                    style:left=format!("{}px", left)
                                                    style:width=format!("{}px", ITEM_WIDTH)
                                                    style:height=format!("{}px", ITEM_HEIGHT)
                                                >
                                                    <GameCard game=game on_select=selected_game search_query=current_search.get() artwork_type=current_artwork_type render_index=index in_viewport=in_viewport />
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any(),
                                ViewMode::List => {
                                    let columns = visible_columns.get();
                                    let current_sort = sort_state.get();
                                    let filters = column_filters.get();
                                    let col_count = columns.len();

                                    // Filter and sort games
                                    let mut filtered_games: Vec<Game> = games_list.clone();

                                    // Apply column filters
                                    if !filters.is_empty() {
                                        filtered_games.retain(|game| {
                                            filters.iter().all(|(col, allowed)| {
                                                col.passes_filter(game, allowed)
                                            })
                                        });
                                    }

                                    // Sort if needed
                                    if let Some(sort) = current_sort {
                                        filtered_games.sort_by(|a, b| {
                                            let cmp = sort.column.compare(a, b);
                                            match sort.direction {
                                                SortDirection::Ascending => cmp,
                                                SortDirection::Descending => cmp.reverse(),
                                            }
                                        });
                                    }

                                    let sorted_games: Vec<(usize, Game)> = filtered_games
                                        .into_iter()
                                        .enumerate()
                                        .skip(start)
                                        .take(end - start)
                                        .collect();

                                    view! {
                                        <div
                                            class="game-list virtual-list"
                                            on:contextmenu=move |ev| {
                                                ev.prevent_default();
                                                set_context_menu.set(Some((ev.client_x(), ev.client_y())));
                                            }
                                            on:click=move |_| {
                                                set_context_menu.set(None);
                                                set_filter_menu.set(None);
                                            }
                                        >
                                            // Dynamic column header
                                            <div
                                                class="game-list-header"
                                                style:position="sticky"
                                                style:top="0"
                                                style:z-index="10"
                                                style:grid-template-columns=format!("repeat({}, 1fr)", col_count)
                                            >
                                                {columns.iter().enumerate().map(|(idx, col)| {
                                                    let col = *col;
                                                    let is_sorted = current_sort.map(|s| s.column == col).unwrap_or(false);
                                                    let sort_dir = current_sort.filter(|s| s.column == col).map(|s| s.direction);
                                                    let is_dragging = dragging_column.get() == Some(idx);
                                                    let is_drag_over = drag_over_column.get() == Some(idx);
                                                    let has_filter = filters.contains_key(&col);

                                                    view! {
                                                        <span
                                                            class="column-header"
                                                            class:sorted=is_sorted
                                                            class:dragging=is_dragging
                                                            class:drag-over=is_drag_over
                                                            class:filtered=has_filter
                                                            draggable="true"
                                                            on:click=move |_| {
                                                                let new_sort = match current_sort {
                                                                    Some(s) if s.column == col => {
                                                                        match s.direction {
                                                                            SortDirection::Ascending => Some(SortState {
                                                                                column: col,
                                                                                direction: SortDirection::Descending,
                                                                            }),
                                                                            SortDirection::Descending => None,
                                                                        }
                                                                    }
                                                                    _ => Some(SortState {
                                                                        column: col,
                                                                        direction: SortDirection::Ascending,
                                                                    }),
                                                                };
                                                                set_sort_state.set(new_sort);
                                                            }
                                                            on:dragstart=move |_| {
                                                                set_dragging_column.set(Some(idx));
                                                            }
                                                            on:dragend=move |_| {
                                                                set_dragging_column.set(None);
                                                                set_drag_over_column.set(None);
                                                            }
                                                            on:dragover=move |ev| {
                                                                ev.prevent_default();
                                                                set_drag_over_column.set(Some(idx));
                                                            }
                                                            on:dragleave=move |_| {
                                                                if drag_over_column.get() == Some(idx) {
                                                                    set_drag_over_column.set(None);
                                                                }
                                                            }
                                                            on:drop=move |ev| {
                                                                ev.prevent_default();
                                                                if let Some(from_idx) = dragging_column.get() {
                                                                    if from_idx != idx {
                                                                        set_visible_columns.update(|cols| {
                                                                            let col = cols.remove(from_idx);
                                                                            let target = if from_idx < idx { idx } else { idx };
                                                                            cols.insert(target, col);
                                                                        });
                                                                    }
                                                                }
                                                                set_dragging_column.set(None);
                                                                set_drag_over_column.set(None);
                                                            }
                                                        >
                                                            <span class="column-label">
                                                                {col.label()}
                                                                {match sort_dir {
                                                                    Some(SortDirection::Ascending) => " ▲",
                                                                    Some(SortDirection::Descending) => " ▼",
                                                                    None => "",
                                                                }}
                                                            </span>
                                                            <button
                                                                class="filter-btn"
                                                                class:active=has_filter
                                                                title="Filter"
                                                                on:click=move |ev: web_sys::MouseEvent| {
                                                                    ev.stop_propagation();
                                                                    let current = filter_menu.get();
                                                                    if current.map(|(c, _, _)| c) == Some(col) {
                                                                        set_filter_menu.set(None);
                                                                    } else {
                                                                        // Get button position using mouse event coordinates
                                                                        let x = ev.client_x() - 100; // Offset to center dropdown
                                                                        let y = ev.client_y() + 10;
                                                                        set_filter_menu.set(Some((col, x, y)));
                                                                    }
                                                                }
                                                                inner_html="<svg width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2'><path d='M22 3H2l8 9.46V19l4 2v-8.54L22 3z'/></svg>"
                                                            >
                                                            </button>
                                                        </span>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                            // Game rows
                                            {sorted_games.into_iter().map(|(index, game)| {
                                                let top = index as i32 * LIST_ITEM_HEIGHT;
                                                let columns = visible_columns.get();

                                                view! {
                                                    <div
                                                        class="virtual-item"
                                                        style:position="absolute"
                                                        style:top=format!("{}px", top + LIST_ITEM_HEIGHT)
                                                        style:left="0"
                                                        style:right="0"
                                                        style:height=format!("{}px", LIST_ITEM_HEIGHT)
                                                    >
                                                        <GameListItem
                                                            game=game
                                                            on_select=selected_game
                                                            columns=columns
                                                            search_query=current_search.get()
                                                        />
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                            // Context menu
                                            {move || context_menu.get().map(|(x, y)| {
                                                let all_cols = Column::all();
                                                let visible = visible_columns.get();

                                                view! {
                                                    <div
                                                        class="column-context-menu"
                                                        style:position="fixed"
                                                        style:left=format!("{}px", x)
                                                        style:top=format!("{}px", y)
                                                        style:z-index="1000"
                                                        on:click=move |ev| ev.stop_propagation()
                                                    >
                                                        <div class="context-menu-title">"Columns"</div>
                                                        {all_cols.iter().map(|col| {
                                                            let col = *col;
                                                            let is_visible = visible.contains(&col);

                                                            view! {
                                                                <label class="context-menu-item">
                                                                    <input
                                                                        type="checkbox"
                                                                        prop:checked=is_visible
                                                                        on:change=move |_| {
                                                                            set_visible_columns.update(|cols| {
                                                                                if cols.contains(&col) {
                                                                                    if cols.len() > 1 {
                                                                                        cols.retain(|c| *c != col);
                                                                                    }
                                                                                } else {
                                                                                    cols.push(col);
                                                                                }
                                                                            });
                                                                        }
                                                                    />
                                                                    {col.label()}
                                                                </label>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                }
                                            })}
                                            // Filter dropdown menu
                                            {move || filter_menu.get().map(|(col, menu_x, menu_y)| {
                                                let all_games = games.get();
                                                let unique_vals = col.unique_values(&all_games);
                                                let current_filters = column_filters.get();
                                                let current_selection = current_filters.get(&col).cloned().unwrap_or_default();

                                                view! {
                                                    <div
                                                        class="filter-dropdown"
                                                        style:left=format!("{}px", menu_x)
                                                        style:top=format!("{}px", menu_y)
                                                        on:click=move |ev| ev.stop_propagation()
                                                    >
                                                        <div class="filter-header">
                                                            <span class="filter-title">"Filter: " {col.label()}</span>
                                                            <button
                                                                class="filter-clear"
                                                                on:click=move |_| {
                                                                    set_column_filters.update(|f| {
                                                                        f.remove(&col);
                                                                    });
                                                                    set_filter_menu.set(None);
                                                                }
                                                            >
                                                                "Clear"
                                                            </button>
                                                        </div>
                                                        <div class="filter-actions">
                                                            <button
                                                                class="filter-action-btn"
                                                                on:click=move |_| {
                                                                    let all_games = games.get();
                                                                    let all_vals: HashSet<String> = col.unique_values(&all_games).into_iter().collect();
                                                                    set_column_filters.update(|f| {
                                                                        f.insert(col, all_vals);
                                                                    });
                                                                }
                                                            >
                                                                "Select All"
                                                            </button>
                                                            <button
                                                                class="filter-action-btn"
                                                                on:click=move |_| {
                                                                    set_column_filters.update(|f| {
                                                                        f.insert(col, HashSet::new());
                                                                    });
                                                                }
                                                            >
                                                                "Select None"
                                                            </button>
                                                        </div>
                                                        <div class="filter-list">
                                                            {unique_vals.into_iter().map(|val| {
                                                                let val_clone = val.clone();
                                                                let val_for_check = val.clone();
                                                                let is_selected = current_selection.is_empty() || current_selection.contains(&val_for_check);

                                                                view! {
                                                                    <label class="filter-item">
                                                                        <input
                                                                            type="checkbox"
                                                                            prop:checked=is_selected
                                                                            on:change=move |_| {
                                                                                let val_to_toggle = val_clone.clone();
                                                                                set_column_filters.update(|f| {
                                                                                    let entry = f.entry(col).or_insert_with(|| {
                                                                                        // Initialize with all values if first time
                                                                                        let all_games = games.get();
                                                                                        col.unique_values(&all_games).into_iter().collect()
                                                                                    });
                                                                                    if entry.contains(&val_to_toggle) {
                                                                                        entry.remove(&val_to_toggle);
                                                                                    } else {
                                                                                        entry.insert(val_to_toggle);
                                                                                    }
                                                                                });
                                                                            }
                                                                        />
                                                                        {val}
                                                                    </label>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                        <div class="filter-footer">
                                                            <button
                                                                class="filter-apply-btn"
                                                                on:click=move |_| {
                                                                    set_filter_menu.set(None);
                                                                }
                                                            >
                                                                "Apply"
                                                            </button>
                                                        </div>
                                                    </div>
                                                }
                                            })}
                                        </div>
                                    }
                                }.into_any(),
                            }}
                            // Loading indicator at bottom
                            {move || loading.get().then(|| view! {
                                <div class="loading-more">"Loading more..."</div>
                            })}
                            // Game count (with filter info if filtered)
                            <div class="game-count">
                                {move || {
                                    let total = total_count.get();
                                    let filters = column_filters.get();
                                    if filters.is_empty() {
                                        format!("{} games", format_number(total))
                                    } else {
                                        // Count how many pass the filters
                                        let all_games = games.get();
                                        let filtered = all_games.iter().filter(|game| {
                                            filters.iter().all(|(col, allowed)| {
                                                col.passes_filter(game, allowed)
                                            })
                                        }).count();
                                        format!("{} of {} games", format_number(filtered as i64), format_number(total))
                                    }
                                }}
                            </div>
                        </div>
                        // Alphabet navigation bar
                        <div class="alphabet-nav">
                            {['#', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
                              'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z']
                                .into_iter()
                                .map(|ch| {
                                    let display = if ch == '#' { "#".to_string() } else { ch.to_string() };
                                    let title = format!("Jump to {}", display);
                                    view! {
                                        <button
                                            class="alphabet-btn"
                                            title=title
                                            on:click=move |_| {
                                                if ch == '#' {
                                                    // Find first game starting with a digit
                                                    let games_list = games.get();
                                                    if let Some(idx) = games_list.iter().position(|g| {
                                                        g.display_title.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                                                    }) {
                                                        scroll_to_index(idx);
                                                    }
                                                } else if let Some(idx) = find_first_game_index(ch) {
                                                    scroll_to_index(idx);
                                                }
                                            }
                                        >
                                            {display}
                                        </button>
                                    }
                                })
                                .collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}
        </main>
    }
}

#[component]
fn GameCard(
    game: Game,
    on_select: WriteSignal<Option<Game>>,
    /// Search query for highlighting matches
    #[prop(default = String::new())]
    search_query: String,
    /// Artwork type to display
    #[prop(default = ArtworkDisplayType::BoxFront)]
    artwork_type: ArtworkDisplayType,
    /// Render index for image queue priority ordering
    #[prop(default = 0)]
    render_index: usize,
    /// Whether this item is in the actual viewport (not just buffer)
    #[prop(default = false)]
    in_viewport: bool,
) -> impl IntoView {
    use crate::components::LazyImage;

    let display_title = game.display_title.clone();
    let first_char = game.display_title.chars().next().unwrap_or('?').to_string();
    let developer = game.developer.clone();
    let variant_count = game.variant_count;
    let launchbox_db_id = game.database_id;
    let platform = game.platform.clone();
    let title_for_img = game.title.clone();

    let game_for_click = game.clone();

    view! {
        <div
            class="game-card"
            on:click=move |_| on_select.set(Some(game_for_click.clone()))
        >
            <div class="game-cover">
                <LazyImage
                    launchbox_db_id=launchbox_db_id
                    game_title=title_for_img
                    platform=platform
                    image_type=artwork_type.to_image_type().to_string()
                    alt=display_title.clone()
                    class="cover-image".to_string()
                    placeholder=first_char.clone()
                    render_index=render_index
                    in_viewport=in_viewport
                />
                {(variant_count > 1).then(|| view! {
                    <span class="variant-badge">{variant_count}</span>
                })}
            </div>
            <div class="game-info">
                <h3 class="game-title">{highlight_matches(&display_title, &search_query)}</h3>
                {developer.map(|d| view! { <p class="game-developer">{d}</p> })}
            </div>
        </div>
    }
}

#[component]
fn GameListItem(
    game: Game,
    on_select: WriteSignal<Option<Game>>,
    columns: Vec<Column>,
    /// Search query for highlighting matches
    #[prop(default = String::new())]
    search_query: String,
) -> impl IntoView {
    let game_for_click = game.clone();
    let col_count = columns.len();

    view! {
        <div
            class="game-list-item"
            style:grid-template-columns=format!("repeat({}, 1fr)", col_count)
            on:click=move |_| on_select.set(Some(game_for_click.clone()))
        >
            {columns.iter().map(|col| {
                let value = col.value(&game);
                let is_title = *col == Column::Title;
                let variant_count = game.variant_count;
                let search = search_query.clone();

                view! {
                    <span class=format!("game-col game-col-{}", col.label().to_lowercase().replace(" ", "-"))>
                        {if is_title {
                            highlight_matches(&value, &search)
                        } else {
                            view! { <>{value}</> }.into_any()
                        }}
                        {(is_title && variant_count > 1).then(|| view! {
                            <span class="variant-count">{format!(" ({})", variant_count)}</span>
                        })}
                    </span>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
