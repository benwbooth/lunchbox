//! Tauri command bindings for the frontend

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
}

/// Invoke a Tauri command with arguments
async fn invoke<T: DeserializeOwned>(cmd: &str, args: impl Serialize) -> Result<T, String> {
    let args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let result = tauri_invoke(cmd, args).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

/// Invoke a Tauri command with no arguments
async fn invoke_no_args<T: DeserializeOwned>(cmd: &str) -> Result<T, String> {
    let result = tauri_invoke(cmd, JsValue::NULL).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

// ============ Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    pub id: i64,
    pub name: String,
    pub game_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub database_id: i64,
    pub title: String,
    pub platform: String,
    pub platform_id: i64,
    pub description: Option<String>,
    pub release_date: Option<String>,
    pub release_year: Option<i32>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genres: Option<String>,
    pub rating: Option<f64>,
    pub box_front_path: Option<String>,
    pub screenshot_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub total_files: usize,
    pub roms: Vec<RomFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RomFile {
    pub path: String,
    pub file_name: String,
    pub clean_name: String,
    pub extension: String,
    pub size: u64,
    pub region: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub platforms_imported: usize,
    pub games_imported: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub rom_directories: Vec<std::path::PathBuf>,
    pub launchbox_path: Option<std::path::PathBuf>,
    pub retroarch_path: Option<std::path::PathBuf>,
    pub emulators: Vec<EmulatorConfig>,
    pub default_platform_emulators: std::collections::HashMap<String, String>,
    pub screenscraper: ScreenScraperSettings,
    #[serde(default)]
    pub steamgriddb: SteamGridDBSettings,
    #[serde(default)]
    pub igdb: IGDBSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScreenScraperSettings {
    pub dev_id: String,
    pub dev_password: String,
    pub user_id: Option<String>,
    pub user_password: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SteamGridDBSettings {
    pub api_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IGDBSettings {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorConfig {
    pub id: String,
    pub name: String,
    pub executable_path: std::path::PathBuf,
    pub emulator_type: EmulatorType,
    pub command_template: String,
    pub supported_platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulatorType {
    RetroArch,
    Standalone,
}

// ============ Helpers ============

/// Convert a file path to an asset URL for Tauri's asset protocol
pub fn file_to_asset_url(path: &str) -> String {
    // Tauri 2 uses asset://localhost/{path}
    // The path needs to be URL-encoded
    let encoded = path
        .replace(' ', "%20")
        .replace('#', "%23")
        .replace('?', "%3F")
        .replace('&', "%26");
    format!("asset://localhost/{}", encoded)
}

// ============ Commands ============

/// Get all platforms
pub async fn get_platforms() -> Result<Vec<Platform>, String> {
    invoke_no_args("get_platforms").await
}

/// Get total game count for a platform/search
pub async fn get_game_count(platform: Option<String>, search: Option<String>) -> Result<i64, String> {
    #[derive(Serialize)]
    struct Args {
        platform: Option<String>,
        search: Option<String>,
    }
    invoke("get_game_count", Args { platform, search }).await
}

/// Get games, optionally filtered by platform or search query
pub async fn get_games(
    platform: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Game>, String> {
    #[derive(Serialize)]
    struct Args {
        platform: Option<String>,
        search: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    }
    invoke("get_games", Args { platform, search, limit, offset }).await
}

/// Get a single game by database ID
pub async fn get_game_by_id(database_id: i64) -> Result<Option<Game>, String> {
    #[derive(Serialize)]
    struct Args {
        database_id: i64,
    }
    invoke("get_game_by_id", Args { database_id }).await
}

/// Scan ROM directories
pub async fn scan_roms(paths: Vec<String>) -> Result<ScanResult, String> {
    #[derive(Serialize)]
    struct Args {
        paths: Vec<String>,
    }
    invoke("scan_roms", Args { paths }).await
}

/// Import from LaunchBox
pub async fn import_launchbox() -> Result<ImportResult, String> {
    invoke_no_args("import_launchbox").await
}

/// Launch a game
pub async fn launch_game(rom_path: String, platform: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        rom_path: String,
        platform: String,
    }
    invoke("launch_game", Args { rom_path, platform }).await
}

/// Get settings
pub async fn get_settings() -> Result<AppSettings, String> {
    invoke_no_args("get_settings").await
}

/// Save settings
pub async fn save_settings(settings: AppSettings) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        settings: AppSettings,
    }
    invoke("save_settings", Args { settings }).await
}

/// Greet (test command)
pub async fn greet(name: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        name: &'a str,
    }
    invoke("greet", Args { name }).await
}

// ============ Collection Types and Commands ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_smart: bool,
    pub filter_rules: Option<String>,
    pub game_count: i64,
}

/// Get all collections
pub async fn get_collections() -> Result<Vec<Collection>, String> {
    invoke_no_args("get_collections").await
}

/// Create a new collection
pub async fn create_collection(name: String, description: Option<String>) -> Result<Collection, String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
        description: Option<String>,
    }
    invoke("create_collection", Args { name, description }).await
}

/// Update a collection
pub async fn update_collection(id: String, name: String, description: Option<String>) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
        name: String,
        description: Option<String>,
    }
    invoke("update_collection", Args { id, name, description }).await
}

/// Delete a collection
pub async fn delete_collection(id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke("delete_collection", Args { id }).await
}

/// Get games in a collection
pub async fn get_collection_games(collection_id: String) -> Result<Vec<Game>, String> {
    #[derive(Serialize)]
    struct Args {
        collection_id: String,
    }
    invoke("get_collection_games", Args { collection_id }).await
}

/// Add a game to a collection
pub async fn add_game_to_collection(collection_id: String, game_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        collection_id: String,
        game_id: String,
    }
    invoke("add_game_to_collection", Args { collection_id, game_id }).await
}

/// Remove a game from a collection
pub async fn remove_game_from_collection(collection_id: String, game_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        collection_id: String,
        game_id: String,
    }
    invoke("remove_game_from_collection", Args { collection_id, game_id }).await
}

// ============ Play Statistics Types and Commands ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayStats {
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    pub play_count: i64,
    pub total_play_time_seconds: i64,
    pub last_played: Option<String>,
    pub first_played: Option<String>,
}

/// Record a play session (call when launching a game)
pub async fn record_play_session(launchbox_db_id: i64, game_title: String, platform: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        launchbox_db_id: i64,
        game_title: String,
        platform: String,
    }
    invoke("record_play_session", Args { launchbox_db_id, game_title, platform }).await
}

/// Get play statistics for a specific game
pub async fn get_play_stats(launchbox_db_id: i64) -> Result<Option<PlayStats>, String> {
    #[derive(Serialize)]
    struct Args {
        launchbox_db_id: i64,
    }
    invoke("get_play_stats", Args { launchbox_db_id }).await
}

/// Get recently played games
pub async fn get_recent_games(limit: Option<i64>) -> Result<Vec<PlayStats>, String> {
    #[derive(Serialize)]
    struct Args {
        limit: Option<i64>,
    }
    invoke("get_recent_games", Args { limit }).await
}

/// Get most played games
pub async fn get_most_played(limit: Option<i64>) -> Result<Vec<PlayStats>, String> {
    #[derive(Serialize)]
    struct Args {
        limit: Option<i64>,
    }
    invoke("get_most_played", Args { limit }).await
}

// ============ Favorites Commands ============

/// Add a game to favorites
pub async fn add_favorite(launchbox_db_id: i64, game_title: String, platform: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        launchbox_db_id: i64,
        game_title: String,
        platform: String,
    }
    invoke("add_favorite", Args { launchbox_db_id, game_title, platform }).await
}

/// Remove a game from favorites
pub async fn remove_favorite(launchbox_db_id: i64) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        launchbox_db_id: i64,
    }
    invoke("remove_favorite", Args { launchbox_db_id }).await
}

/// Check if a game is a favorite
pub async fn is_favorite(launchbox_db_id: i64) -> Result<bool, String> {
    #[derive(Serialize)]
    struct Args {
        launchbox_db_id: i64,
    }
    invoke("is_favorite", Args { launchbox_db_id }).await
}

/// Get all favorite games
pub async fn get_favorites() -> Result<Vec<Game>, String> {
    invoke_no_args("get_favorites").await
}

// ============ Service Connection Tests ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub user_info: Option<String>,
}

/// Test ScreenScraper API connection
pub async fn test_screenscraper_connection(
    dev_id: String,
    dev_password: String,
    user_id: Option<String>,
    user_password: Option<String>,
) -> Result<ConnectionTestResult, String> {
    #[derive(Serialize)]
    struct Args {
        dev_id: String,
        dev_password: String,
        user_id: Option<String>,
        user_password: Option<String>,
    }
    invoke("test_screenscraper_connection", Args { dev_id, dev_password, user_id, user_password }).await
}

/// Test SteamGridDB API connection
pub async fn test_steamgriddb_connection(api_key: String) -> Result<ConnectionTestResult, String> {
    #[derive(Serialize)]
    struct Args {
        api_key: String,
    }
    invoke("test_steamgriddb_connection", Args { api_key }).await
}

/// Test IGDB API connection
pub async fn test_igdb_connection(
    client_id: String,
    client_secret: String,
) -> Result<ConnectionTestResult, String> {
    #[derive(Serialize)]
    struct Args {
        client_id: String,
        client_secret: String,
    }
    invoke("test_igdb_connection", Args { client_id, client_secret }).await
}
