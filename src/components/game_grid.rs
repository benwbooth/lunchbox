//! Game grid and list views with virtual scrolling

use crate::app::{
    ArtworkDisplayType, GameFilters, PLATFORM_SELECTION_ALL_GAMES, PLATFORM_SELECTION_MINIGAMES,
    ViewMode,
};
use crate::backend_api::{self, Game};
use chrono::{Datelike, NaiveDate};
use gloo_timers::callback::Interval;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::console;

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

fn format_hover_video_progress_label(progress: &backend_api::VideoDownloadProgress) -> String {
    let status = progress
        .status
        .clone()
        .unwrap_or_else(|| "Loading preview...".to_string());

    progress
        .progress
        .map(|value| {
            format!(
                "{} {}%",
                status,
                (value.clamp(0.0, 1.0) * 100.0).round() as i32
            )
        })
        .unwrap_or(status)
}

fn format_game_card_download_label(item: &backend_api::MinervaDownloadQueueItem) -> String {
    let status = match item.status.as_str() {
        "fetching_torrent" => "Preparing download",
        "extracting" => "Finishing download",
        "paused" => "Download paused",
        "pending" => "Download queued",
        _ => "Downloading",
    };

    if item.progress_percent > 0.0 {
        format!("{status} {:.0}%", item.progress_percent.clamp(0.0, 100.0))
    } else {
        status.to_string()
    }
}

fn title_matches_alphabet_target(title: &str, target: char) -> bool {
    match target {
        '#' => title
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false),
        _ => {
            let target_lower = target.to_ascii_lowercase();
            title
                .chars()
                .next()
                .map(|c| c.to_ascii_lowercase() == target_lower)
                .unwrap_or(false)
        }
    }
}

fn find_first_game_index_for_target(games: &[Game], target: char) -> Option<usize> {
    games
        .iter()
        .position(|game| title_matches_alphabet_target(&game.display_title, target))
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
        parts
            .push(view! { <span class="search-highlight">{matched.to_string()}</span> }.into_any());
        last_end = start + query.len();
    }

    // Add remaining text after last match
    if last_end < text.len() {
        let after = &text[last_end..];
        parts.push(view! { <>{after.to_string()}</> }.into_any());
    }

    view! { <>{parts}</> }.into_any()
}
const ITEM_WIDTH: i32 = 180; // Width of each game card
const LIST_ITEM_HEIGHT: i32 = 40; // Height in list view
const BUFFER_ITEMS: i32 = 10; // Extra items to render above/below viewport
const FETCH_CHUNK_SIZE: i64 = 500; // How many games to fetch at once
const GAME_GRID_UI_STATE_KEY: &str = "lunchbox.ui.game-grid.v1";
const GAME_GRID_DPAD_EVENT: &str = "lunchbox-grid-dpad";
const GAME_GRID_DPAD_ACTION_ATTR: &str = "data-nav-grid-dpad-action";
const GAME_GRID_DPAD_HANDLED_ATTR: &str = "data-nav-grid-dpad-handled";
const GAME_GRID_DPAD_TARGET_ATTR: &str = "data-nav-grid-dpad-target-index";

fn scaled_grid_item_height(zoom: f64) -> i32 {
    ((ITEM_HEIGHT as f64 * zoom) as i32).max(1)
}

fn grid_nav_columns(width: i32, zoom: f64) -> usize {
    let scaled_item_width = ((ITEM_WIDTH as f64 * zoom) as i32).max(1);
    (width / scaled_item_width).max(1) as usize
}

fn grid_nav_page_step(
    container: &web_sys::HtmlElement,
    mode: ViewMode,
    cols: usize,
    zoom: f64,
) -> usize {
    let client_height = container.client_height().max(1);
    match mode {
        ViewMode::Grid => {
            let row_height = scaled_grid_item_height(zoom);
            let rows = (client_height / row_height).max(1) as usize;
            rows * cols.max(1)
        }
        ViewMode::List => (client_height / LIST_ITEM_HEIGHT).max(1) as usize,
    }
}

fn default_grid_nav_index(
    container: &web_sys::HtmlElement,
    mode: ViewMode,
    count: usize,
    cols: usize,
    zoom: f64,
) -> usize {
    if count == 0 {
        return 0;
    }

    let scroll = container.scroll_top().max(0);
    let index = match mode {
        ViewMode::Grid => {
            let row = scroll / scaled_grid_item_height(zoom);
            row.max(0) as usize * cols.max(1)
        }
        ViewMode::List => (scroll / LIST_ITEM_HEIGHT).max(0) as usize,
    };

    index.min(count.saturating_sub(1))
}

fn next_grid_nav_index(
    current_index: usize,
    count: usize,
    mode: ViewMode,
    cols: usize,
    page_step: usize,
    action: &str,
) -> Option<usize> {
    if count == 0 {
        return None;
    }
    match action {
        "enter" => return Some(current_index.min(count.saturating_sub(1))),
        "home" => return Some(0),
        "end" => return Some(count.saturating_sub(1)),
        "page-up" => return Some(current_index.saturating_sub(page_step.max(1))),
        "page-down" => {
            return Some(
                current_index
                    .saturating_add(page_step.max(1))
                    .min(count.saturating_sub(1)),
            );
        }
        _ => {}
    }

    match mode {
        ViewMode::List => match action {
            "up" => current_index.checked_sub(1),
            "down" if current_index + 1 < count => Some(current_index + 1),
            _ => None,
        },
        ViewMode::Grid => {
            let cols = cols.max(1);
            match action {
                "up" if current_index >= cols => Some(current_index - cols),
                "down" => next_grid_nav_down_index(current_index, cols, count),
                "left" if current_index % cols != 0 => Some(current_index - 1),
                "right" if current_index + 1 < count => Some(current_index + 1),
                _ => None,
            }
        }
    }
}

fn next_grid_nav_down_index(current_index: usize, cols: usize, count: usize) -> Option<usize> {
    let direct = current_index + cols;
    if direct < count {
        return Some(direct);
    }

    let last_row_start = ((count - 1) / cols) * cols;
    if current_index >= last_row_start {
        return None;
    }

    let candidate = last_row_start + (current_index % cols);
    Some(candidate.min(count - 1))
}

fn reveal_grid_nav_index(
    container: &web_sys::HtmlElement,
    mode: ViewMode,
    index: usize,
    cols: usize,
    zoom: f64,
) -> i32 {
    let current_scroll = container.scroll_top().max(0);
    let client_height = container.client_height().max(0);
    if client_height <= 0 {
        return current_scroll;
    }

    let next_scroll = match mode {
        ViewMode::Grid => {
            let row_height = scaled_grid_item_height(zoom);
            let row = index / cols.max(1);
            let row_top = row as i32 * row_height;
            let row_bottom = row_top + row_height;
            let viewport_bottom = current_scroll + client_height;

            if row_top < current_scroll {
                row_top.max(0)
            } else if row_bottom > viewport_bottom {
                (row_bottom - client_height).max(0)
            } else {
                current_scroll
            }
        }
        ViewMode::List => {
            let row_top = LIST_ITEM_HEIGHT + index as i32 * LIST_ITEM_HEIGHT;
            let row_bottom = row_top + LIST_ITEM_HEIGHT;
            let viewport_top = current_scroll + LIST_ITEM_HEIGHT;
            let viewport_bottom = current_scroll + client_height;

            if row_top < viewport_top {
                (row_top - LIST_ITEM_HEIGHT).max(0)
            } else if row_bottom > viewport_bottom {
                (row_bottom - client_height).max(0)
            } else {
                current_scroll
            }
        }
    };

    if next_scroll != current_scroll {
        container.set_scroll_top(next_scroll);
    }
    next_scroll
}

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
        ("january", 1),
        ("february", 2),
        ("march", 3),
        ("april", 4),
        ("may", 5),
        ("june", 6),
        ("july", 7),
        ("august", 8),
        ("september", 9),
        ("october", 10),
        ("november", 11),
        ("december", 12),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

fn sanitize_visible_columns(columns: Vec<Column>) -> Vec<Column> {
    if columns.is_empty() {
        return Column::default_visible();
    }

    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for column in columns {
        if seen.insert(column) {
            deduped.push(column);
        }
    }

    if deduped.is_empty() {
        Column::default_visible()
    } else {
        deduped
    }
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
            Column::Year => game
                .release_year
                .map(|y| y.to_string())
                .unwrap_or_else(|| "-".to_string()),
            Column::ReleaseDate => game
                .release_date
                .as_ref()
                .map(|d| format_date(d))
                .unwrap_or_else(|| "-".to_string()),
            Column::Genre => game.genres.clone().unwrap_or_else(|| "-".to_string()),
            Column::Players => game.players.clone().unwrap_or_else(|| "-".to_string()),
            Column::Rating => game
                .rating
                .map(|r| format!("{:.1}", r))
                .unwrap_or_else(|| "-".to_string()),
            Column::Esrb => game.esrb.clone().unwrap_or_else(|| "-".to_string()),
            Column::Coop => game
                .cooperative
                .map(|c| if c { "Yes" } else { "No" }.to_string())
                .unwrap_or_else(|| "-".to_string()),
            Column::Variants => {
                if game.variant_count > 1 {
                    game.variant_count.to_string()
                } else {
                    "-".to_string()
                }
            }
            Column::ReleaseType => game.release_type.clone().unwrap_or_else(|| "-".to_string()),
            Column::Series => game.series.clone().unwrap_or_else(|| "-".to_string()),
            Column::Region => game.region.clone().unwrap_or_else(|| "-".to_string()),
            Column::Notes => game
                .notes
                .clone()
                .map(|n| {
                    if n.len() > 50 {
                        format!("{}...", &n[..47])
                    } else {
                        n
                    }
                })
                .unwrap_or_else(|| "-".to_string()),
        }
    }

    /// Compare two games by this column
    pub fn compare(&self, a: &Game, b: &Game) -> Ordering {
        match self {
            Column::Title => a
                .display_title
                .to_lowercase()
                .cmp(&b.display_title.to_lowercase()),
            Column::Platform => a.platform.to_lowercase().cmp(&b.platform.to_lowercase()),
            Column::Developer => cmp_opt_str(&a.developer, &b.developer),
            Column::Publisher => cmp_opt_str(&a.publisher, &b.publisher),
            Column::Year => cmp_opt(&a.release_year, &b.release_year),
            Column::ReleaseDate => {
                sortable_date(&a.release_date).cmp(&sortable_date(&b.release_date))
            }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Current sort state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortState {
    pub column: Column,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameGridUiState {
    scroll_top: i32,
    visible_columns: Vec<Column>,
    sort_state: Option<SortState>,
}

impl Default for GameGridUiState {
    fn default() -> Self {
        Self {
            scroll_top: 0,
            visible_columns: Column::default_visible(),
            sort_state: None,
        }
    }
}

/// Column filters - maps column to set of allowed values (empty = no filter)
pub type ColumnFilters = HashMap<Column, HashSet<String>>;

fn apply_list_view_state(
    games: &[Game],
    filters: &ColumnFilters,
    sort: Option<SortState>,
) -> Vec<Game> {
    let mut displayed_games: Vec<Game> = games.to_vec();

    if !filters.is_empty() {
        displayed_games.retain(|game| {
            filters
                .iter()
                .all(|(col, allowed)| col.passes_filter(game, allowed))
        });
    }

    if let Some(sort) = sort {
        displayed_games.sort_by(|a, b| {
            let cmp = sort.column.compare(a, b);
            match sort.direction {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
    }

    displayed_games
}

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
    zoom_level: ReadSignal<f64>,
    set_zoom_level: WriteSignal<f64>,
    game_filters: ReadSignal<GameFilters>,
) -> impl IntoView {
    let is_minigames_selection =
        |plat: &Option<String>| plat.as_deref() == Some(PLATFORM_SELECTION_MINIGAMES);

    let query_platform = |plat: Option<String>| match plat.as_deref() {
        Some(PLATFORM_SELECTION_ALL_GAMES) => None,
        _ => plat,
    };

    let persisted =
        crate::ui_state::load_json::<GameGridUiState>(GAME_GRID_UI_STATE_KEY).unwrap_or_default();
    let initial_scroll_top = persisted.scroll_top.max(0);
    let initial_visible_columns = sanitize_visible_columns(persisted.visible_columns);
    let initial_sort_state = persisted.sort_state;

    // Games cache - we load chunks as needed
    let (games, set_games) = signal::<Vec<Game>>(Vec::new());
    let (total_count, set_total_count) = signal(0i64);
    let (loading, set_loading) = signal(false);
    let (load_error, set_load_error) = signal::<Option<String>>(None);
    let (loaded_up_to, set_loaded_up_to) = signal(0i64); // How many games we've loaded

    // Scroll state
    let (scroll_top, set_scroll_top) = signal(initial_scroll_top);
    let (container_height, set_container_height) = signal(600);
    let (container_width, set_container_width) = signal(800);
    let (nav_selected_index, set_nav_selected_index) = signal::<Option<usize>>(None);

    // Track current filters
    let (current_platform, set_current_platform) = signal::<Option<String>>(None);
    let (current_collection, set_current_collection) = signal::<Option<String>>(None);
    let (current_search, set_current_search) = signal(String::new());
    let (current_filters, set_current_filters) = signal(GameFilters::default());
    let (did_initial_load, set_did_initial_load) = signal(false);
    let resize_listener_attached = Rc::new(Cell::new(false));
    let nav_listener_attached = Rc::new(Cell::new(false));
    let dpad_listener_attached = Rc::new(Cell::new(false));

    // Column configuration for list view
    let (visible_columns, set_visible_columns) = signal(initial_visible_columns);
    let (sort_state, set_sort_state) = signal::<Option<SortState>>(initial_sort_state);
    let (context_menu, set_context_menu) = signal::<Option<(i32, i32)>>(None); // (x, y) position
    let (dragging_column, set_dragging_column) = signal::<Option<usize>>(None);
    let (drag_over_column, set_drag_over_column) = signal::<Option<usize>>(None);

    // Column filters
    let (column_filters, set_column_filters) = signal::<ColumnFilters>(HashMap::new());
    let (filter_menu, set_filter_menu) = signal::<Option<(Column, i32, i32)>>(None); // (column, x, y) position

    // Container ref for scroll handling
    let container_ref = NodeRef::<html::Main>::new();
    let last_scroll_persist_ms = Rc::new(Cell::new(0.0));

    Effect::new(move || {
        crate::ui_state::save_json(
            GAME_GRID_UI_STATE_KEY,
            &GameGridUiState {
                scroll_top: scroll_top.get_untracked().max(0),
                visible_columns: visible_columns.get(),
                sort_state: sort_state.get(),
            },
        );
    });

    // Load more games if needed
    let ensure_loaded = move |needed_index: i64| {
        let current_loaded = loaded_up_to.get();
        let total = total_count.get();

        if needed_index >= current_loaded && current_loaded < total && !loading.get() {
            let plat = current_platform.get();
            if is_minigames_selection(&plat) {
                return;
            }
            let search = current_search.get();
            let filters = current_filters.get();
            let offset = current_loaded;
            let query_plat = query_platform(plat);

            set_load_error.set(None);
            set_loading.set(true);

            spawn_local(async move {
                let search_param = if search.is_empty() {
                    None
                } else {
                    Some(search)
                };
                match backend_api::get_games(
                    query_plat,
                    search_param,
                    Some(backend_api::GameQueryFilters {
                        installed_only: filters.installed_only,
                        hide_homebrew: filters.hide_homebrew,
                        hide_adult: filters.hide_adult,
                    }),
                    Some(FETCH_CHUNK_SIZE),
                    Some(offset),
                )
                .await
                {
                    Ok(new_games) => {
                        let count = new_games.len() as i64;
                        set_games.update(|g| g.extend(new_games));
                        set_loaded_up_to.update(|l| *l += count);
                    }
                    Err(e) => {
                        let message = format!("Failed to load more games: {}", e);
                        console::error_1(&message.clone().into());
                        set_load_error.set(Some(message));
                    }
                }
                set_loading.set(false);
            });
        }
    };

    // Handle scroll events
    let on_scroll = {
        let last_scroll_persist_ms = last_scroll_persist_ms.clone();
        move |ev: web_sys::Event| {
            if let Some(target) = ev.target() {
                let element: web_sys::HtmlElement = target.unchecked_into();
                let current_scroll = element.scroll_top().max(0);
                set_scroll_top.set(current_scroll);
                set_container_height.set(element.client_height());
                set_container_width.set(element.client_width());

                // Persist frequently enough for restoration, but avoid hot-path writes each frame.
                let now_ms = js_sys::Date::now();
                if now_ms - last_scroll_persist_ms.get() > 250.0 {
                    last_scroll_persist_ms.set(now_ms);
                    crate::ui_state::save_json(
                        GAME_GRID_UI_STATE_KEY,
                        &GameGridUiState {
                            scroll_top: current_scroll,
                            visible_columns: visible_columns.get_untracked(),
                            sort_state: sort_state.get_untracked(),
                        },
                    );
                }
            }
        }
    };

    Effect::new({
        let container_ref = container_ref.clone();
        let resize_listener_attached = resize_listener_attached.clone();
        move || {
            let Some(container) = container_ref.get() else {
                return;
            };

            let w = container.client_width();
            let h = container.client_height();
            if w > 0 {
                set_container_width.set(w);
            }
            if h > 0 {
                set_container_height.set(h);
            }

            if resize_listener_attached.get() {
                return;
            }
            resize_listener_attached.set(true);

            let callback =
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    if let Some(container) = container_ref.get() {
                        let width = container.client_width();
                        let height = container.client_height();
                        if width > 0 {
                            set_container_width.set(width);
                        }
                        if height > 0 {
                            set_container_height.set(height);
                        }
                    }
                })
                    as Box<dyn FnMut(web_sys::Event)>);

            if let Some(window) = web_sys::window() {
                let _ = window
                    .add_event_listener_with_callback("resize", callback.as_ref().unchecked_ref());
                callback.forget();
            } else {
                resize_listener_attached.set(false);
            }
        }
    });

    Effect::new({
        let container_ref = container_ref.clone();
        let nav_listener_attached = nav_listener_attached.clone();
        move || {
            let Some(container) = container_ref.get() else {
                return;
            };

            if nav_listener_attached.get() {
                return;
            }
            nav_listener_attached.set(true);

            let listener =
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    if let Some(container) = container_ref.get() {
                        let next_index = container
                            .get_attribute("data-nav-selected-index")
                            .and_then(|value| value.parse::<usize>().ok());
                        set_nav_selected_index.set(next_index);
                    }
                })
                    as Box<dyn FnMut(web_sys::Event)>);

            let _ = container.add_event_listener_with_callback(
                "lunchbox-grid-select",
                listener.as_ref().unchecked_ref(),
            );
            listener.forget();
        }
    });

    // Watch for filter changes and reset
    Effect::new(move || {
        let plat = platform.get();
        let coll = collection.get();
        let search = search_query.get();
        let filters = game_filters.get();
        let is_initial_load = !did_initial_load.get();

        if plat != current_platform.get()
            || coll != current_collection.get()
            || search != current_search.get()
            || filters != current_filters.get()
        {
            set_current_platform.set(plat.clone());
            set_current_collection.set(coll.clone());
            set_current_search.set(search.clone());
            set_current_filters.set(filters);
            set_games.set(Vec::new());
            set_nav_selected_index.set(None);
            set_loaded_up_to.set(0);
            set_total_count.set(0);
            if !is_initial_load {
                set_scroll_top.set(0);
                if let Some(container) = container_ref.get() {
                    container.set_scroll_top(0);
                }
            }
            set_loading.set(true);
            let container_ref_for_restore = container_ref.clone();
            let restore_scroll_top = initial_scroll_top;

            spawn_local(async move {
                // Collections - load all (they're small)
                if let Some(coll_id) = coll {
                    let result = backend_api::get_collection_games(coll_id)
                        .await
                        .unwrap_or_default();
                    let count = result.len() as i64;
                    set_games.set(result);
                    set_total_count.set(count);
                    set_loaded_up_to.set(count);
                    if is_initial_load && restore_scroll_top > 0 {
                        if let Some(container) = container_ref_for_restore.get() {
                            container.set_scroll_top(restore_scroll_top);
                            set_scroll_top.set(restore_scroll_top);
                        }
                    }
                    set_loading.set(false);
                    set_did_initial_load.set(true);
                    return;
                }

                if is_minigames_selection(&plat) {
                    set_loading.set(false);
                    set_did_initial_load.set(true);
                    return;
                }

                let search_param = if search.is_empty() {
                    None
                } else {
                    Some(search.clone())
                };
                let query_plat = query_platform(plat.clone());
                let query_filters = backend_api::GameQueryFilters {
                    installed_only: filters.installed_only,
                    hide_homebrew: filters.hide_homebrew,
                    hide_adult: filters.hide_adult,
                };

                match backend_api::get_game_count(
                    query_plat.clone(),
                    search_param.clone(),
                    Some(query_filters),
                )
                .await
                {
                    Ok(count) => {
                        set_total_count.set(count);

                        if count > 0 {
                            match backend_api::get_games(
                                query_plat,
                                search_param,
                                Some(query_filters),
                                Some(FETCH_CHUNK_SIZE),
                                Some(0),
                            )
                            .await
                            {
                                Ok(initial_games) => {
                                    let loaded = initial_games.len() as i64;
                                    set_games.set(initial_games);
                                    set_loaded_up_to.set(loaded);
                                }
                                Err(e) => {
                                    let message = format!("Failed to load games: {}", e);
                                    console::error_1(&message.clone().into());
                                    set_games.set(Vec::new());
                                    set_total_count.set(0);
                                    set_loaded_up_to.set(0);
                                    set_load_error.set(Some(message));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let message = format!("Failed to load game count: {}", e);
                        console::error_1(&message.clone().into());
                        set_games.set(Vec::new());
                        set_total_count.set(0);
                        set_loaded_up_to.set(0);
                        set_load_error.set(Some(message));
                    }
                }
                if is_initial_load && restore_scroll_top > 0 {
                    if let Some(container) = container_ref_for_restore.get() {
                        container.set_scroll_top(restore_scroll_top);
                        set_scroll_top.set(restore_scroll_top);
                    }
                }
                set_loading.set(false);
                set_did_initial_load.set(true);
            });
        }
    });

    // Calculate visible items based on scroll (with zoom applied)
    let visible_range = move || {
        let mode = view_mode.get();
        let scroll = scroll_top.get();
        let height = container_height.get();
        let width = container_width.get();
        let total = total_count.get() as i32;
        let zoom = zoom_level.get();

        match mode {
            ViewMode::Grid => {
                let scaled_item_width = (ITEM_WIDTH as f64 * zoom) as i32;
                let scaled_item_height = (ITEM_HEIGHT as f64 * zoom) as i32;
                let cols = (width / scaled_item_width).max(1);
                let rows_before = scroll / scaled_item_height;
                let visible_rows = (height / scaled_item_height) + 2;

                let start = ((rows_before - BUFFER_ITEMS).max(0) * cols) as usize;
                let end = (((rows_before + visible_rows + BUFFER_ITEMS) * cols) as usize)
                    .min(total as usize);

                (start, end, cols as usize)
            }
            ViewMode::List => {
                let start = ((scroll / LIST_ITEM_HEIGHT) - BUFFER_ITEMS).max(0) as usize;
                let visible = height / LIST_ITEM_HEIGHT;
                let end =
                    ((scroll / LIST_ITEM_HEIGHT) + visible + BUFFER_ITEMS * 2).min(total) as usize;

                (start, end, 1)
            }
        }
    };

    // Calculate total content height (with zoom applied)
    let total_height = move || {
        let mode = view_mode.get();
        let total = total_count.get() as i32;
        let width = container_width.get();
        let zoom = zoom_level.get();

        match mode {
            ViewMode::Grid => {
                let scaled_item_width = (ITEM_WIDTH as f64 * zoom) as i32;
                let scaled_item_height = (ITEM_HEIGHT as f64 * zoom) as i32;
                let cols = (width / scaled_item_width).max(1);
                let rows = (total + cols - 1) / cols;
                rows * scaled_item_height
            }
            ViewMode::List => total * LIST_ITEM_HEIGHT,
        }
    };

    // Scroll to a specific game index (with zoom applied)
    let scroll_to_index = move |index: usize| {
        let mode = view_mode.get();
        let width = container_width.get();
        let zoom = zoom_level.get();

        let scroll_pos = match mode {
            ViewMode::Grid => {
                let scaled_item_width = (ITEM_WIDTH as f64 * zoom) as i32;
                let scaled_item_height = (ITEM_HEIGHT as f64 * zoom) as i32;
                let cols = (width / scaled_item_width).max(1);
                let row = index as i32 / cols;
                row * scaled_item_height
            }
            ViewMode::List => {
                (index as i32 + 1) * LIST_ITEM_HEIGHT // +1 for header
            }
        };

        if let Some(container) = container_ref.get() {
            container.set_scroll_top(scroll_pos);
        }
    };

    let jump_to_alphabet_target = move |target: char| {
        let loaded_games = games.get();
        if let Some(idx) = find_first_game_index_for_target(&loaded_games, target) {
            scroll_to_index(idx);
            return;
        }

        if loading.get() {
            return;
        }

        let total = total_count.get();
        let already_loaded = loaded_up_to.get();
        if already_loaded >= total {
            return;
        }

        let query_plat = query_platform(current_platform.get());
        let search = current_search.get();
        let search_param = if search.is_empty() {
            None
        } else {
            Some(search)
        };
        let filters = current_filters.get();
        let query_filters = backend_api::GameQueryFilters {
            installed_only: filters.installed_only,
            hide_homebrew: filters.hide_homebrew,
            hide_adult: filters.hide_adult,
        };

        set_load_error.set(None);
        set_loading.set(true);

        spawn_local(async move {
            let mut offset = already_loaded;
            let mut pending_games = Vec::new();
            let mut found_index = None;

            while offset < total {
                match backend_api::get_games(
                    query_plat.clone(),
                    search_param.clone(),
                    Some(query_filters),
                    Some(FETCH_CHUNK_SIZE),
                    Some(offset),
                )
                .await
                {
                    Ok(chunk) => {
                        if chunk.is_empty() {
                            break;
                        }

                        let base_index = already_loaded as usize + pending_games.len();
                        if let Some(pos) = find_first_game_index_for_target(&chunk, target) {
                            found_index = Some(base_index + pos);
                        }

                        offset += chunk.len() as i64;
                        pending_games.extend(chunk);

                        if found_index.is_some() {
                            break;
                        }
                    }
                    Err(e) => {
                        let message = format!("Failed to jump to letter {}: {}", target, e);
                        console::error_1(&message.clone().into());
                        set_load_error.set(Some(message));
                        set_loading.set(false);
                        return;
                    }
                }
            }

            if !pending_games.is_empty() {
                let pending_count = pending_games.len() as i64;
                set_games.update(|games| games.extend(pending_games));
                set_loaded_up_to.update(|loaded| *loaded += pending_count);
            }

            set_loading.set(false);

            if let Some(idx) = found_index {
                scroll_to_index(idx);
            }
        });
    };

    let navigation_games = move || match view_mode.get() {
        ViewMode::Grid => games.get(),
        ViewMode::List => {
            apply_list_view_state(&games.get(), &column_filters.get(), sort_state.get())
        }
    };

    Effect::new({
        let container_ref = container_ref.clone();
        let dpad_listener_attached = dpad_listener_attached.clone();
        move || {
            let Some(container) = container_ref.get() else {
                return;
            };

            if dpad_listener_attached.get() {
                return;
            }
            dpad_listener_attached.set(true);

            let listener =
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    let Some(container) = container_ref.get() else {
                        return;
                    };
                    let action = container
                        .get_attribute(GAME_GRID_DPAD_ACTION_ATTR)
                        .unwrap_or_default();
                    let mode = view_mode.get_untracked();
                    let count = match mode {
                        ViewMode::Grid => games.get_untracked().len(),
                        ViewMode::List => apply_list_view_state(
                            &games.get_untracked(),
                            &column_filters.get_untracked(),
                            sort_state.get_untracked(),
                        )
                        .len(),
                    };
                    if count == 0 {
                        let _ = container.set_attribute(GAME_GRID_DPAD_HANDLED_ATTR, "false");
                        return;
                    }

                    let zoom = zoom_level.get_untracked();
                    let cols = grid_nav_columns(container.client_width(), zoom);
                    let page_step = grid_nav_page_step(&container, mode, cols, zoom);
                    let target_index = container
                        .get_attribute(GAME_GRID_DPAD_TARGET_ATTR)
                        .and_then(|value| value.parse::<usize>().ok())
                        .filter(|index| *index < count);
                    let selected_index = nav_selected_index
                        .get_untracked()
                        .filter(|index| *index < count);
                    let current_index = if action == "enter" {
                        target_index.or(selected_index)
                    } else {
                        selected_index
                    }
                    .unwrap_or_else(|| default_grid_nav_index(&container, mode, count, cols, zoom));
                    let Some(next_index) =
                        next_grid_nav_index(current_index, count, mode, cols, page_step, &action)
                    else {
                        let _ = container.set_attribute(GAME_GRID_DPAD_HANDLED_ATTR, "false");
                        return;
                    };

                    set_nav_selected_index.set(Some(next_index));
                    let _ =
                        container.set_attribute("data-nav-selected-index", &next_index.to_string());
                    let _ = container.set_attribute("data-nav-active-grid", "true");

                    let next_scroll =
                        reveal_grid_nav_index(&container, mode, next_index, cols, zoom);
                    set_scroll_top.set(next_scroll);
                    let _ = container.focus();
                    if container.scroll_top() != next_scroll {
                        container.set_scroll_top(next_scroll);
                    }
                    let _ = container.set_attribute(GAME_GRID_DPAD_HANDLED_ATTR, "true");
                })
                    as Box<dyn FnMut(web_sys::Event)>);

            let _ = container.add_event_listener_with_callback(
                GAME_GRID_DPAD_EVENT,
                listener.as_ref().unchecked_ref(),
            );
            listener.forget();
        }
    });

    Effect::new({
        let container_ref = container_ref.clone();
        move || {
            let available_games = navigation_games();
            if available_games.is_empty() {
                set_nav_selected_index.set(None);
                if let Some(container) = container_ref.get() {
                    let _ = container.remove_attribute("data-nav-selected-index");
                }
                return;
            }

            let clamped_index = nav_selected_index
                .get()
                .map(|index| index.min(available_games.len().saturating_sub(1)));

            if clamped_index != nav_selected_index.get() {
                set_nav_selected_index.set(clamped_index);
            }

            if let Some(index) = clamped_index {
                if let Some(container) = container_ref.get() {
                    let _ = container.set_attribute("data-nav-selected-index", &index.to_string());
                }
            }
        }
    });

    // Pinch-to-zoom state
    let (initial_pinch_distance, set_initial_pinch_distance) = signal::<Option<f64>>(None);
    let (initial_zoom, set_initial_zoom) = signal(1.0f64);

    let on_touchstart = move |ev: web_sys::TouchEvent| {
        let touches = ev.touches();
        if touches.length() == 2 {
            if let (Some(t1), Some(t2)) = (touches.get(0), touches.get(1)) {
                let dx = (t2.client_x() - t1.client_x()) as f64;
                let dy = (t2.client_y() - t1.client_y()) as f64;
                let dist = (dx * dx + dy * dy).sqrt();
                set_initial_pinch_distance.set(Some(dist));
                set_initial_zoom.set(zoom_level.get());
            }
        }
    };

    let on_touchmove = move |ev: web_sys::TouchEvent| {
        let touches = ev.touches();
        if touches.length() == 2 {
            if let Some(initial_dist) = initial_pinch_distance.get() {
                if let (Some(t1), Some(t2)) = (touches.get(0), touches.get(1)) {
                    let dx = (t2.client_x() - t1.client_x()) as f64;
                    let dy = (t2.client_y() - t1.client_y()) as f64;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let scale = dist / initial_dist;
                    let new_zoom = (initial_zoom.get() * scale).clamp(0.5, 2.0);
                    set_zoom_level.set(new_zoom);
                    ev.prevent_default();
                }
            }
        }
    };

    let on_touchend = move |_: web_sys::TouchEvent| {
        set_initial_pinch_distance.set(None);
    };

    // Mouse wheel zoom (Ctrl+scroll)
    let on_wheel = move |ev: web_sys::WheelEvent| {
        // Only zoom when Ctrl is held (like browser zoom behavior)
        if ev.ctrl_key() {
            let delta = ev.delta_y();
            let zoom_change = if delta > 0.0 { -0.1 } else { 0.1 };
            let new_zoom = (zoom_level.get() + zoom_change).clamp(0.5, 2.0);
            set_zoom_level.set(new_zoom);
            ev.prevent_default();
        }
    };

    view! {
        <main
            class="game-content virtual-scroll"
            node_ref=container_ref
            tabindex="0"
            data-nav="true"
            data-nav-kind="game-grid"
            data-nav-grid="true"
            data-nav-view-mode=move || match view_mode.get() {
                ViewMode::Grid => "grid".to_string(),
                ViewMode::List => "list".to_string(),
            }
            data-nav-grid-cols=move || {
                let zoom = zoom_level.get();
                let scaled_item_width = (ITEM_WIDTH as f64 * zoom) as i32;
                ((container_width.get() / scaled_item_width).max(1)).to_string()
            }
            data-nav-game-count=move || navigation_games().len().to_string()
            data-nav-selected-index=move || nav_selected_index.get().map(|index| index.to_string()).unwrap_or_default()
            data-nav-grid-row-height=move || ((ITEM_HEIGHT as f64 * zoom_level.get()) as i32).to_string()
            data-nav-list-row-height=LIST_ITEM_HEIGHT.to_string()
            on:focus=move |_| {
                let available_games = navigation_games();
                if available_games.is_empty() {
                    return;
                }
                let default_index = nav_selected_index
                    .get_untracked()
                    .filter(|index| *index < available_games.len())
                    .unwrap_or_else(|| {
                        let (start, _, _) = visible_range();
                        start.min(available_games.len().saturating_sub(1))
                    });
                set_nav_selected_index.set(Some(default_index));
                if let Some(container) = container_ref.get() {
                    let _ = container.set_attribute("data-nav-selected-index", &default_index.to_string());
                    let _ = container.set_attribute("data-nav-active-grid", "true");
                }
            }
            on:scroll=on_scroll
            on:touchstart=on_touchstart
            on:touchmove=on_touchmove
            on:touchend=on_touchend
            on:wheel=on_wheel
        >
            {move || {
                let games_list = games.get();
                let is_loading = loading.get() && games_list.is_empty();
                let total = total_count.get();
                let platform_selection = platform.get();
                let is_minigames = platform_selection.as_deref() == Some(PLATFORM_SELECTION_MINIGAMES);
                let error = load_error.get();

                if is_loading {
                    view! { <div class="loading">"Loading games..."</div> }.into_any()
                } else if let Some(error_message) = error {
                    view! {
                        <div class="empty-state">
                            <p>{error_message}</p>
                        </div>
                    }.into_any()
                } else if total == 0 && !loading.get() {
                    if collection.get().is_some() {
                        view! {
                            <div class="empty-state">
                                <p>"This collection is empty. Add games to see them here."</p>
                            </div>
                        }.into_any()
                    } else if is_minigames {
                        view! { <crate::components::MarioMinigame zoom_level=zoom_level set_zoom_level=set_zoom_level /> }.into_any()
                    } else if platform_selection.is_some() {
                        view! {
                            <div class="empty-state">
                                <p>"No games found for this platform."</p>
                            </div>
                        }.into_any()
                    } else {
                        // Fallback
                        view! { <crate::components::MarioMinigame zoom_level=zoom_level set_zoom_level=set_zoom_level /> }.into_any()
                    }
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
                    let zoom = zoom_level.get();
                    let scaled_item_width = (ITEM_WIDTH as f64 * zoom) as i32;
                    let scaled_item_height = (ITEM_HEIGHT as f64 * zoom) as i32;

                    // Calculate actual viewport range (without buffer) for priority
                    let scroll = scroll_top.get();
                    let ch = container_height.get();
                    let (viewport_start, viewport_end) = match mode {
                        ViewMode::Grid => {
                            let rows_before = scroll / scaled_item_height;
                            let visible_rows = (ch / scaled_item_height) + 2;
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
                                            let top = row as i32 * scaled_item_height;
                                            let left = col as i32 * scaled_item_width;
                                            let in_viewport = index >= viewport_start && index < viewport_end;

                                            view! {
                                                <div
                                                    class="virtual-item"
                                                    style:position="absolute"
                                                    style:top=format!("{}px", top)
                                                    style:left=format!("{}px", left)
                                                    style:width=format!("{}px", scaled_item_width)
                                                    style:height=format!("{}px", scaled_item_height)
                                                >
                                                    <GameCard
                                                        game=game
                                                        on_select=selected_game
                                                        search_query=current_search.get()
                                                        artwork_type=current_artwork_type
                                                        render_index=index
                                                        in_viewport=in_viewport
                                                        nav_selected=Signal::derive(move || {
                                                            nav_selected_index.get() == Some(index)
                                                        })
                                                        on_nav_select=Callback::new(move |_| {
                                                            set_nav_selected_index.set(Some(index));
                                                            if let Some(container) = container_ref.get() {
                                                                let _ = container.set_attribute("data-nav-selected-index", &index.to_string());
                                                                let _ = container.set_attribute("data-nav-active-grid", "true");
                                                                let _ = container.focus();
                                                            }
                                                        })
                                                    />
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

                                    let sorted_games: Vec<(usize, Game)> = apply_list_view_state(
                                        &games_list,
                                        &filters,
                                        current_sort,
                                    )
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
                                                            render_index=index
                                                            nav_selected=Signal::derive(move || {
                                                                nav_selected_index.get() == Some(index)
                                                            })
                                                            on_nav_select=Callback::new(move |_| {
                                                                set_nav_selected_index.set(Some(index));
                                                                if let Some(container) = container_ref.get() {
                                                                    let _ = container.set_attribute("data-nav-selected-index", &index.to_string());
                                                                    let _ = container.set_attribute("data-nav-active-grid", "true");
                                                                    let _ = container.focus();
                                                                }
                                                            })
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
                                        let label = if total == 1 { "game" } else { "games" };
                                        format!("{} {}", format_number(total), label)
                                    } else {
                                        // Count how many pass the filters
                                        let all_games = games.get();
                                        let filtered = all_games.iter().filter(|game| {
                                            filters.iter().all(|(col, allowed)| {
                                                col.passes_filter(game, allowed)
                                            })
                                        }).count();
                                        let label = if total == 1 { "game" } else { "games" };
                                        format!("{} of {} {}", format_number(filtered as i64), format_number(total), label)
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
                                                jump_to_alphabet_target(ch);
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
    #[prop(default = false.into())] nav_selected: Signal<bool>,
    #[prop(into)] on_nav_select: Callback<usize>,
) -> impl IntoView {
    use crate::components::{LazyImage, minerva_downloads_signal};

    let display_title = game.display_title.clone();
    let first_char = game.display_title.chars().next().unwrap_or('?').to_string();
    let developer = game.developer.clone();
    let variant_count = game.variant_count;
    let has_game_file = game.has_game_file;
    let launchbox_db_id = game.database_id;
    let platform = game.platform.clone();
    let title_for_img = game.title.clone();
    let tooltip_title = game.display_title.clone();
    let tooltip_developer = game.developer.clone();
    let tooltip_publisher = game.publisher.clone();
    let tooltip_genres = game.genres.clone();
    let tooltip_meta = game
        .release_year
        .map(|year| format!("{} • {}", game.platform, year))
        .unwrap_or_else(|| game.platform.clone());
    let hover_video_title = StoredValue::new(game.title.clone());
    let hover_video_platform = StoredValue::new(game.platform.clone());
    let (minerva_downloads, _) = minerva_downloads_signal();

    let (is_hovered, set_is_hovered) = signal(false);
    let (hover_preview_armed, set_hover_preview_armed) = signal(false);
    let (hover_video_url, set_hover_video_url) = signal::<Option<String>>(None);
    let (hover_video_loading, set_hover_video_loading) = signal(false);
    let (hover_video_progress, set_hover_video_progress) = signal::<Option<f32>>(None);
    let (hover_video_status, set_hover_video_status) = signal("Loading preview...".to_string());
    let (hover_video_unavailable, set_hover_video_unavailable) = signal(false);
    let (hover_video_loaded, set_hover_video_loaded) = signal(false);
    let (hover_video_playing, set_hover_video_playing) = signal(false);
    let (tooltip_style, set_tooltip_style) = signal(String::new());
    let hover_token = Rc::new(Cell::new(0u64));
    let hover_progress_poll: Rc<RefCell<Option<Interval>>> = Rc::new(RefCell::new(None));

    let game_for_click = game.clone();

    // Track overflow for marquee
    let card_ref = NodeRef::<html::Div>::new();
    let title_ref = NodeRef::<html::Span>::new();
    let (overflow_style, set_overflow_style) = signal(String::new());
    let (is_truncated, set_is_truncated) = signal(false);

    // Measure overflow after mount
    Effect::new(move || {
        if let Some(el) = title_ref.get() {
            let scroll_width = el.scroll_width();
            let client_width = el.client_width();
            let overflow = scroll_width - client_width;
            if overflow > 0 {
                set_is_truncated.set(true);
                // Calculate duration based on overflow (50px/s)
                let duration = (overflow as f64 / 50.0).max(1.0);
                set_overflow_style.set(format!(
                    "--marquee-offset: -{}px; --marquee-duration: {}s;",
                    overflow, duration
                ));
            }
        }
    });

    // On hover, nudge oversized cards away from viewport edges so full artwork stays visible.
    let on_mouse_enter = {
        let card_ref = card_ref.clone();
        let set_is_hovered = set_is_hovered;
        let set_hover_video_url = set_hover_video_url;
        let set_hover_preview_armed = set_hover_preview_armed;
        let set_hover_video_loading = set_hover_video_loading;
        let set_hover_video_progress = set_hover_video_progress;
        let set_hover_video_unavailable = set_hover_video_unavailable;
        let set_hover_video_playing = set_hover_video_playing;
        let set_tooltip_style = set_tooltip_style;
        let hover_video_url = hover_video_url;
        let hover_video_unavailable = hover_video_unavailable;
        let hover_token = hover_token.clone();
        let hover_progress_poll = hover_progress_poll.clone();
        move |_: web_sys::MouseEvent| {
            const HOVER_SCALE: f64 = 1.95;
            const HOVER_LIFT_PX: f64 = 10.0;
            const EDGE_MARGIN_PX: f64 = 8.0;
            const ALPHABET_NAV_CLEARANCE_PX: f64 = 44.0;
            const TOOLTIP_SCALE: f64 = HOVER_SCALE;

            set_is_hovered.set(true);
            set_hover_preview_armed.set(true);
            set_hover_video_loading.set(false);
            set_hover_video_unavailable.set(false);
            set_hover_video_playing.set(false);

            let Some(card) = card_ref.get() else {
                return;
            };
            let Some(window) = web_sys::window() else {
                return;
            };
            let Ok(viewport_width) = window.inner_width() else {
                return;
            };
            let Ok(viewport_height) = window.inner_height() else {
                return;
            };
            let (Some(vw), Some(vh)) = (viewport_width.as_f64(), viewport_height.as_f64()) else {
                return;
            };

            // Keep hover-expanded cards and preview tooltips out of the right-edge alphabet jump
            // lane so those buttons stay clickable.
            let (min_x, max_x, min_y, max_y) = match card.closest(".game-content") {
                Ok(Some(container)) => {
                    let c = container.get_bounding_client_rect();
                    (
                        c.left() + EDGE_MARGIN_PX,
                        c.right() - EDGE_MARGIN_PX - ALPHABET_NAV_CLEARANCE_PX,
                        c.top() + EDGE_MARGIN_PX,
                        c.bottom() - EDGE_MARGIN_PX,
                    )
                }
                _ => (
                    EDGE_MARGIN_PX,
                    vw - EDGE_MARGIN_PX - ALPHABET_NAV_CLEARANCE_PX,
                    EDGE_MARGIN_PX,
                    vh - EDGE_MARGIN_PX,
                ),
            };

            let rect = card.get_bounding_client_rect();
            let extra_x = (rect.width() * HOVER_SCALE - rect.width()) / 2.0;
            let extra_y = (rect.height() * HOVER_SCALE - rect.height()) / 2.0;

            let projected_left = rect.left() - extra_x;
            let projected_right = rect.right() + extra_x;
            let projected_top = rect.top() - extra_y - HOVER_LIFT_PX;
            let projected_bottom = rect.bottom() + extra_y - HOVER_LIFT_PX;

            let mut dodge_x = 0.0;
            let mut dodge_y = 0.0;

            if projected_left < min_x {
                dodge_x += min_x - projected_left;
            }
            if projected_right > max_x {
                dodge_x -= projected_right - max_x;
            }
            if projected_top < min_y {
                dodge_y += min_y - projected_top;
            }
            if projected_bottom > max_y {
                dodge_y -= projected_bottom - max_y;
            }

            let _ = card.set_attribute(
                "style",
                &format!(
                    "--hover-dodge-x: {:.2}px; --hover-dodge-y: {:.2}px;",
                    dodge_x, dodge_y
                ),
            );
            let tooltip_width = 198.0;
            let tooltip_visual_width = tooltip_width * TOOLTIP_SCALE;
            let tooltip_margin = 24.0;
            let tooltip_max_left =
                (vw - tooltip_visual_width - tooltip_margin - ALPHABET_NAV_CLEARANCE_PX)
                    .max(tooltip_margin);
            let tooltip_left = (rect.left() + (rect.width() / 2.0) + dodge_x
                - (tooltip_visual_width / 2.0))
                .clamp(tooltip_margin, tooltip_max_left);
            let tooltip_top = projected_bottom + dodge_y - 2.0;
            set_tooltip_style.set(format!(
                "top: {:.2}px; left: {:.2}px; --tooltip-scale: {:.3};",
                tooltip_top, tooltip_left, TOOLTIP_SCALE
            ));
            if let Ok(Some(wrapper)) = card.closest(".virtual-item") {
                let _ = wrapper.set_attribute("data-hovered-card", "true");
            }

            let token = hover_token.get().wrapping_add(1);
            hover_token.set(token);

            let hover_token_prefetch = hover_token.clone();
            let hover_video_url_prefetch = hover_video_url;
            let hover_video_unavailable_prefetch = hover_video_unavailable;
            let title = hover_video_title.get_value();
            let platform = hover_video_platform.get_value();
            let db_id_opt = (launchbox_db_id > 0).then_some(launchbox_db_id);

            // Start prefetch immediately so playback starts as soon as media is ready.
            if hover_video_url_prefetch.get_untracked().is_none()
                && !hover_video_unavailable_prefetch.get_untracked()
                && !hover_video_loading.get_untracked()
            {
                let title_async = title.clone();
                let platform_async = platform.clone();
                let hover_token_async = hover_token_prefetch.clone();
                let hover_progress_poll_for_task = hover_progress_poll.clone();
                spawn_local(async move {
                    if hover_token_async.get() != token {
                        return;
                    }
                    set_hover_video_loading.set(true);
                    set_hover_video_progress.set(None);
                    set_hover_video_status.set("Loading preview...".to_string());

                    match backend_api::check_cached_video(
                        title_async.clone(),
                        platform_async.clone(),
                        db_id_opt,
                    )
                    .await
                    {
                        Ok(Some(cached_path)) => {
                            if hover_token_async.get() == token {
                                let url = backend_api::file_to_asset_url(&cached_path);
                                set_hover_video_loaded.set(false);
                                set_hover_video_url.set(Some(url));
                                set_hover_video_loading.set(false);
                            }
                            return;
                        }
                        Ok(None) => {}
                        Err(_) => {}
                    }

                    if hover_token_async.get() != token {
                        return;
                    }

                    let progress_poll = hover_progress_poll_for_task.clone();
                    {
                        let title_for_poll = title_async.clone();
                        let platform_for_poll = platform_async.clone();
                        let hover_token_poll = hover_token_async.clone();
                        let interval = Interval::new(180, move || {
                            let title = title_for_poll.clone();
                            let platform = platform_for_poll.clone();
                            let hover_token_poll_inner = hover_token_poll.clone();
                            spawn_local(async move {
                                if hover_token_poll_inner.get() != token {
                                    return;
                                }
                                if let Ok(Some(progress)) =
                                    backend_api::get_video_download_progress(
                                        title, platform, db_id_opt,
                                    )
                                    .await
                                {
                                    set_hover_video_status
                                        .set(format_hover_video_progress_label(&progress));
                                    set_hover_video_progress
                                        .set(progress.progress.map(|value| value.clamp(0.0, 1.0)));
                                }
                            });
                        });
                        *progress_poll.borrow_mut() = Some(interval);
                    }

                    match backend_api::download_game_video(
                        title_async.clone(),
                        platform_async.clone(),
                        db_id_opt,
                    )
                    .await
                    {
                        Ok(local_path) => {
                            if hover_token_async.get() == token {
                                let url = backend_api::file_to_asset_url(&local_path);
                                set_hover_video_loaded.set(false);
                                set_hover_video_url.set(Some(url));
                                set_hover_video_progress.set(Some(1.0));
                                set_hover_video_status.set("Loading preview... 100%".to_string());
                            }
                        }
                        Err(e) => {
                            if hover_token_async.get() == token {
                                let msg = e.to_lowercase();
                                if msg.contains("not found")
                                    || msg.contains("no video")
                                    || msg.contains("unknown platform")
                                    || msg.contains("not configured")
                                {
                                    set_hover_video_unavailable.set(true);
                                    set_hover_video_status.set("Preview unavailable".to_string());
                                } else {
                                    set_hover_video_status.set(format!("Preview failed: {}", e));
                                }
                            }
                        }
                    }

                    if hover_token_async.get() == token {
                        hover_progress_poll_for_task.borrow_mut().take();
                        set_hover_video_loading.set(false);
                    }
                });
            }
        }
    };

    let on_mouse_leave = {
        let card_ref = card_ref.clone();
        let set_is_hovered = set_is_hovered;
        let set_hover_preview_armed = set_hover_preview_armed;
        let set_hover_video_loading = set_hover_video_loading;
        let set_hover_video_progress = set_hover_video_progress;
        let set_hover_video_status = set_hover_video_status;
        let set_hover_video_unavailable = set_hover_video_unavailable;
        let set_hover_video_playing = set_hover_video_playing;
        let set_tooltip_style = set_tooltip_style;
        let hover_token = hover_token.clone();
        let hover_progress_poll = hover_progress_poll.clone();
        move |_: web_sys::MouseEvent| {
            set_is_hovered.set(false);
            set_hover_preview_armed.set(false);
            set_hover_video_loading.set(false);
            set_hover_video_progress.set(None);
            set_hover_video_status.set("Loading preview...".to_string());
            set_hover_video_unavailable.set(false);
            set_hover_video_playing.set(false);
            set_tooltip_style.set(String::new());
            hover_token.set(hover_token.get().wrapping_add(1));
            hover_progress_poll.borrow_mut().take();
            if let Some(card) = card_ref.get() {
                if let Ok(Some(video_el)) = card.query_selector(".cover-preview-video") {
                    let pause_value = js_sys::Reflect::get(
                        video_el.as_ref(),
                        &wasm_bindgen::JsValue::from_str("pause"),
                    );
                    if let Ok(pause_fn) = pause_value {
                        if let Some(pause_fn) = pause_fn.dyn_ref::<js_sys::Function>() {
                            let _ = pause_fn.call0(video_el.as_ref());
                        }
                    }
                    let _ = js_sys::Reflect::set(
                        video_el.as_ref(),
                        &wasm_bindgen::JsValue::from_str("currentTime"),
                        &wasm_bindgen::JsValue::from_f64(0.0),
                    );
                    let _ = js_sys::Reflect::set(
                        video_el.as_ref(),
                        &wasm_bindgen::JsValue::from_str("muted"),
                        &wasm_bindgen::JsValue::TRUE,
                    );
                }
                let _ = card.set_attribute("style", "--hover-dodge-x: 0px; --hover-dodge-y: 0px;");
                if let Ok(Some(wrapper)) = card.closest(".virtual-item") {
                    let _ = wrapper.remove_attribute("data-hovered-card");
                }
            }
        }
    };

    let on_card_click = {
        let card_ref = card_ref.clone();
        let set_is_hovered = set_is_hovered;
        let set_hover_preview_armed = set_hover_preview_armed;
        let set_hover_video_loading = set_hover_video_loading;
        let set_hover_video_status = set_hover_video_status;
        let set_hover_video_unavailable = set_hover_video_unavailable;
        let set_hover_video_playing = set_hover_video_playing;
        let hover_token = hover_token.clone();
        let game_for_click = game_for_click.clone();
        move |_| {
            on_nav_select.run(render_index);
            set_is_hovered.set(false);
            set_hover_preview_armed.set(false);
            set_hover_video_loading.set(false);
            set_hover_video_status.set("Loading preview...".to_string());
            set_hover_video_unavailable.set(false);
            set_hover_video_playing.set(false);
            hover_token.set(hover_token.get().wrapping_add(1));

            if let Some(card) = card_ref.get() {
                if let Ok(Some(video_el)) = card.query_selector(".cover-preview-video") {
                    let pause_value = js_sys::Reflect::get(
                        video_el.as_ref(),
                        &wasm_bindgen::JsValue::from_str("pause"),
                    );
                    if let Ok(pause_fn) = pause_value {
                        if let Some(pause_fn) = pause_fn.dyn_ref::<js_sys::Function>() {
                            let _ = pause_fn.call0(video_el.as_ref());
                        }
                    }
                    let _ = js_sys::Reflect::set(
                        video_el.as_ref(),
                        &wasm_bindgen::JsValue::from_str("currentTime"),
                        &wasm_bindgen::JsValue::from_f64(0.0),
                    );
                    let _ = js_sys::Reflect::set(
                        video_el.as_ref(),
                        &wasm_bindgen::JsValue::from_str("muted"),
                        &wasm_bindgen::JsValue::TRUE,
                    );
                }
                let _ = card.set_attribute("style", "--hover-dodge-x: 0px; --hover-dodge-y: 0px;");
                if let Ok(Some(wrapper)) = card.closest(".virtual-item") {
                    let _ = wrapper.remove_attribute("data-hovered-card");
                }
            }

            on_select.set(Some(game_for_click.clone()));
        }
    };

    // Start playback only when we are actually showing the preview layer.
    Effect::new(move || {
        let should_play = is_hovered.get()
            && hover_preview_armed.get()
            && hover_video_loaded.get()
            && hover_video_url.with(|url| url.is_some());
        if let Some(card) = card_ref.get() {
            if let Ok(Some(video_el)) = card.query_selector(".cover-preview-video") {
                if !should_play {
                    let pause_value = js_sys::Reflect::get(
                        video_el.as_ref(),
                        &wasm_bindgen::JsValue::from_str("pause"),
                    );
                    if let Ok(pause_fn) = pause_value {
                        if let Some(pause_fn) = pause_fn.dyn_ref::<js_sys::Function>() {
                            let _ = pause_fn.call0(video_el.as_ref());
                        }
                    }
                    let _ = js_sys::Reflect::set(
                        video_el.as_ref(),
                        &wasm_bindgen::JsValue::from_str("currentTime"),
                        &wasm_bindgen::JsValue::from_f64(0.0),
                    );
                    set_hover_video_playing.set(false);
                    return;
                }

                let _ = js_sys::Reflect::set(
                    video_el.as_ref(),
                    &wasm_bindgen::JsValue::from_str("muted"),
                    &wasm_bindgen::JsValue::TRUE,
                );
                let _ = js_sys::Reflect::set(
                    video_el.as_ref(),
                    &wasm_bindgen::JsValue::from_str("volume"),
                    &wasm_bindgen::JsValue::from_f64(0.0),
                );
                let _ = js_sys::Reflect::set(
                    video_el.as_ref(),
                    &wasm_bindgen::JsValue::from_str("currentTime"),
                    &wasm_bindgen::JsValue::from_f64(0.0),
                );
                let play_value = js_sys::Reflect::get(
                    video_el.as_ref(),
                    &wasm_bindgen::JsValue::from_str("play"),
                );
                if let Ok(play_fn) = play_value {
                    if let Some(play_fn) = play_fn.dyn_ref::<js_sys::Function>() {
                        let _ = play_fn.call0(video_el.as_ref());
                    }
                }
            }
        }
    });

    let show_hover_video = move || {
        is_hovered.get()
            && hover_preview_armed.get()
            && hover_video_playing.get()
            && hover_video_url.with(|url| url.is_some())
    };

    let show_hover_loading = move || {
        is_hovered.get()
            && hover_preview_armed.get()
            && !hover_video_playing.get()
            && !hover_video_unavailable.get()
            && (hover_video_loading.get()
                || hover_video_status.get() != "Loading preview..."
                || (hover_video_url.with(|url| url.is_some()) && !hover_video_loaded.get()))
    };

    let show_hover_progress_bar = move || {
        !hover_video_playing.get()
            && !hover_video_unavailable.get()
            && !hover_video_status.get().starts_with("Preview failed")
    };

    let show_hover_layer = move || {
        is_hovered.get()
            && hover_preview_armed.get()
            && !hover_video_unavailable.get()
            && (hover_video_playing.get()
                || hover_video_loading.get()
                || (hover_video_url.with(|url| url.is_some()) && !hover_video_loaded.get())
                || hover_video_status.get().starts_with("Preview failed"))
    };

    let active_download = move || {
        minerva_downloads
            .get()
            .into_iter()
            .find(|item| item.launchbox_db_id == launchbox_db_id)
    };

    view! {
        <>
        <div
            class="game-card-anchor"
            role="button"
            tabindex="-1"
            data-nav="true"
            data-nav-kind="game-item"
            data-game-index=render_index.to_string()
            class:nav-selected=move || nav_selected.get()
            on:mouseenter=on_mouse_enter
            on:mouseleave=on_mouse_leave
            on:click=on_card_click
        >
            <div
                class="game-card"
                node_ref=card_ref
            >
            <div class="game-cover">
                <div class="cover-art-layer" class:faded=show_hover_video>
                    <LazyImage
                        launchbox_db_id=launchbox_db_id
                        game_title=title_for_img.clone()
                        platform=platform.clone()
                        image_type=artwork_type.to_image_type().to_string()
                        alt=display_title.clone()
                        class="cover-image".to_string()
                        placeholder=first_char.clone()
                        render_index=render_index
                        in_viewport=in_viewport
                    />
                </div>
                <div class="cover-video-layer" class:active=show_hover_layer>
                    {move || {
                        hover_video_url
                            .get()
                            .map(|url| {
                                view! {
                                    <video
                                        class="cover-preview-video"
                                        src=url
                                        muted=true
                                        loop=true
                                        playsinline=true
                                        preload="auto"
                                        on:loadedmetadata=move |ev| {
                                            if let Some(target) = ev.target() {
                                                if let Ok(video) = target.dyn_into::<web_sys::HtmlVideoElement>() {
                                                    if video.video_width() == 0 || video.video_height() == 0 {
                                                        set_hover_video_loaded.set(false);
                                                        set_hover_video_playing.set(false);
                                                        set_hover_video_loading.set(false);
                                                        set_hover_video_unavailable.set(true);
                                                    }
                                                }
                                            }
                                        }
                                        on:loadeddata=move |_| {
                                            set_hover_video_loaded.set(true);
                                            set_hover_video_loading.set(false);
                                        }
                                        on:playing=move |ev| {
                                            set_hover_video_playing.set(true);
                                            set_hover_video_loading.set(false);
                                            if let Some(target) = ev.target() {
                                                if let Ok(video) = target.dyn_into::<web_sys::HtmlVideoElement>() {
                                                    video.set_muted(false);
                                                    video.set_volume(0.35);
                                                }
                                            }
                                        }
                                        on:pause=move |ev| {
                                            set_hover_video_playing.set(false);
                                            if let Some(target) = ev.target() {
                                                if let Ok(video) = target.dyn_into::<web_sys::HtmlVideoElement>() {
                                                    video.set_muted(true);
                                                }
                                            }
                                        }
                                        on:ended=move |_| set_hover_video_playing.set(false)
                                        on:error=move |_| {
                                            set_hover_video_loaded.set(false);
                                            set_hover_video_playing.set(false);
                                            set_hover_video_loading.set(false);
                                            set_hover_video_unavailable.set(true);
                                        }
                                    />
                                }.into_any()
                            })
                            .unwrap_or_else(|| view! { <></> }.into_any())
                    }}
                    <div class="cover-video-loading" class:active=show_hover_loading>
                        <span>
                            {move || hover_video_status.get()}
                        </span>
                        <Show when=show_hover_progress_bar>
                            <div class="download-progress">
                                <div
                                    class="progress-bar"
                                    class:indeterminate=move || hover_video_progress.get().is_none()
                                    style:width=move || {
                                        hover_video_progress
                                            .get()
                                            .map(|value| format!("{:.1}%", value.clamp(0.0, 1.0) * 100.0))
                                            .unwrap_or_else(|| "100%".to_string())
                                    }
                                ></div>
                            </div>
                        </Show>
                    </div>
                </div>
                {move || {
                    if let Some(download) = active_download() {
                        let is_paused = download.status == "paused";
                        let label = format_game_card_download_label(&download);
                        view! {
                            <span
                                class="download-status-badge"
                                class:paused=is_paused
                                title=label.clone()
                                aria-label=label
                            >
                                <svg
                                    class="download-status-icon"
                                    viewBox="0 0 24 24"
                                    aria-hidden="true"
                                    focusable="false"
                                >
                                    <circle cx="12" cy="12" r="10" class="download-status-icon-base" />
                                    {if is_paused {
                                        view! {
                                            <>
                                                <rect x="9" y="8" width="2.4" height="8" rx="0.8" class="download-status-icon-pause" />
                                                <rect x="12.6" y="8" width="2.4" height="8" rx="0.8" class="download-status-icon-pause" />
                                            </>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <>
                                                <path d="M12 7.4 V13.4" class="download-status-icon-arrow" />
                                                <path d="M9.6 11.2 L12 13.8 L14.4 11.2" class="download-status-icon-arrow" />
                                                <path d="M8.6 16 H15.4" class="download-status-icon-arrow" />
                                            </>
                                        }.into_any()
                                    }}
                                </svg>
                            </span>
                        }
                            .into_any()
                    } else if has_game_file {
                        view! {
                            <span class="play-ready-badge" title="Ready to play" aria-label="Ready to play">
                                <svg
                                    class="play-ready-icon"
                                    viewBox="0 0 24 24"
                                    aria-hidden="true"
                                    focusable="false"
                                >
                                    <circle cx="12" cy="12" r="10" class="play-ready-icon-base" />
                                    <path d="M10 8.6 L16.3 12 L10 15.4 Z" class="play-ready-icon-triangle" />
                                </svg>
                            </span>
                        }
                            .into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}
                {(variant_count > 1).then(|| view! {
                    <span class="variant-badge">{variant_count}</span>
                })}
            </div>
            <div class="game-info">
                <h3 class="game-title">
                    <span
                        class="game-title-text"
                        class:truncated=move || is_truncated.get()
                        style=move || overflow_style.get()
                        node_ref=title_ref
                    >
                        {highlight_matches(&display_title, &search_query)}
                    </span>
                </h3>
                {developer.map(|d| view! { <p class="game-developer">{d}</p> })}
            </div>
            </div>
        </div>
        <Show when=show_hover_layer>
            <div
                class="game-hover-tooltip active"
                style=move || tooltip_style.get()
                on:click=move |ev| ev.stop_propagation()
            >
                <div class="game-hover-tooltip-art">
                    <LazyImage
                        launchbox_db_id=launchbox_db_id
                        game_title=title_for_img.clone()
                        platform=platform.clone()
                        image_type="Box - Front".to_string()
                        alt=format!("{} box art", tooltip_title.clone())
                        class="tooltip-box-art".to_string()
                        placeholder=first_char.clone()
                        render_index=render_index.saturating_add(10_000_000)
                        in_viewport=true
                    />
                </div>
                <div class="game-hover-tooltip-body">
                    <div class="game-hover-tooltip-title">{tooltip_title.clone()}</div>
                    <div class="game-hover-tooltip-meta">{tooltip_meta.clone()}</div>
                    {tooltip_developer.clone().map(|dev| view! {
                        <div class="game-hover-tooltip-dev">{dev}</div>
                    })}
                    {tooltip_publisher.clone().map(|publisher| view! {
                        <div class="game-hover-tooltip-publisher">{publisher}</div>
                    })}
                    {tooltip_genres.clone().map(|genres| view! {
                        <div class="game-hover-tooltip-genre">{genres}</div>
                    })}
                </div>
            </div>
        </Show>
        </>
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
    #[prop(default = 0)] render_index: usize,
    #[prop(default = false.into())] nav_selected: Signal<bool>,
    #[prop(into)] on_nav_select: Callback<usize>,
) -> impl IntoView {
    let game_for_click = game.clone();
    let col_count = columns.len();

    view! {
        <div
            class="game-list-item"
            role="button"
            tabindex="-1"
            data-nav="true"
            data-nav-kind="game-item"
            data-game-index=render_index.to_string()
            class:nav-selected=move || nav_selected.get()
            style:grid-template-columns=format!("repeat({}, 1fr)", col_count)
            on:click=move |_| {
                on_nav_select.run(render_index);
                on_select.set(Some(game_for_click.clone()))
            }
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
