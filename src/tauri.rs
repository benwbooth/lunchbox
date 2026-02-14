//! API bindings for the frontend
//!
//! Automatically detects whether running in Tauri or browser and uses
//! the appropriate backend (IPC for Tauri, HTTP for browser).

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ============ Backend Detection ============

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
}

/// Check if running in Tauri context
fn is_tauri() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    let result = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .map(|v| !v.is_undefined())
        .unwrap_or(false);
    web_sys::console::log_1(&format!("is_tauri() = {}", result).into());
    result
}

/// The HTTP API base URL for browser mode
const HTTP_API_BASE: &str = "http://127.0.0.1:3001";

// ============ Backend Logging ============

/// Send a log message to the backend (fire and forget)
pub fn log_to_backend(level: &str, message: &str) {
    use wasm_bindgen_futures::spawn_local;
    let level = level.to_string();
    let message = message.to_string();
    spawn_local(async move {
        let _ = log_to_backend_async(&level, &message).await;
    });
}

async fn log_to_backend_async(level: &str, message: &str) -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode};

    #[derive(Serialize)]
    struct LogMessage<'a> {
        level: &'a str,
        message: &'a str,
    }

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body = serde_json::to_string(&LogMessage { level, message }).map_err(|e| e.to_string())?;
    opts.set_body(&JsValue::from_str(&body));

    let url = format!("{}/api/log", HTTP_API_BASE);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;
    request.headers().set("Content-Type", "application/json").map_err(|e| format!("{:?}", e))?;

    let window = web_sys::window().ok_or("No window")?;
    let _ = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await;
    Ok(())
}

// ============ HTTP Fetch Helpers ============

async fn http_get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response, console};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}{}", HTTP_API_BASE, path);
    console::log_1(&format!("http_get: Fetching {}", url).into());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| {
        console::error_1(&format!("http_get: Request creation failed: {:?}", e).into());
        format!("{:?}", e)
    })?;

    let window = web_sys::window().ok_or("No window")?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| {
            console::error_1(&format!("http_get: Fetch failed: {:?}", e).into());
            format!("{:?}", e)
        })?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;
    console::log_1(&format!("http_get: Response status {}", resp.status()).into());

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| {
            console::error_1(&format!("http_get: JSON parse failed: {:?}", e).into());
            format!("{:?}", e)
        })?;

    serde_wasm_bindgen::from_value(json).map_err(|e| {
        console::error_1(&format!("http_get: Deserialization failed: {}", e).into());
        e.to_string()
    })
}

async fn http_post<T: DeserializeOwned, B: Serialize>(path: &str, body: &B) -> Result<T, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    opts.set_body(&JsValue::from_str(&body_str));

    let url = format!("{}{}", HTTP_API_BASE, path);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let window = web_sys::window().ok_or("No window")?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())
}

async fn http_post_empty<B: Serialize>(path: &str, body: &B) -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    opts.set_body(&JsValue::from_str(&body_str));

    let url = format!("{}{}", HTTP_API_BASE, path);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let window = web_sys::window().ok_or("No window")?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    Ok(())
}

async fn http_delete(path: &str) -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}{}", HTTP_API_BASE, path);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    let window = web_sys::window().ok_or("No window")?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    Ok(())
}

// ============ Health Check ============

/// Health check response from backend
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub build_hash: String,
    pub build_timestamp: String,
}

/// Check backend health and get build info
pub async fn check_health() -> Result<HealthResponse, String> {
    http_get("/api/health").await
}

// ============ rspc HTTP Client ============

/// JSON-RPC response wrapper from rspc
#[derive(Debug, Deserialize)]
struct RspcJsonRpcResponse {
    result: RspcResult,
}

/// Inner result from rspc (can be response or error)
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
enum RspcResult {
    #[serde(rename = "response")]
    Response(serde_json::Value),
    #[serde(rename = "error")]
    Error(RspcError),
}

#[derive(Debug, Deserialize)]
struct RspcError {
    code: i32,
    message: String,
}

/// Call an rspc query via HTTP
async fn rspc_query<T: DeserializeOwned, A: Serialize>(procedure: &str, args: &A) -> Result<T, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    // Build URL - only add input param if args is not ()
    let url = if std::any::type_name::<A>() == "()" {
        format!("{}/rspc/{}", HTTP_API_BASE, procedure)
    } else {
        let args_json = serde_json::to_string(args).map_err(|e| e.to_string())?;
        let encoded_args = urlencoding::encode(&args_json);
        format!("{}/rspc/{}?input={}", HTTP_API_BASE, procedure, encoded_args)
    };

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    let window = web_sys::window().ok_or("No window")?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = wasm_bindgen_futures::JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let response: RspcJsonRpcResponse = serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

    match response.result {
        RspcResult::Response(data) => {
            serde_json::from_value(data).map_err(|e| e.to_string())
        }
        RspcResult::Error(error) => {
            Err(error.message)
        }
    }
}

// ============ Tauri IPC Helpers ============

/// Invoke a Tauri command with arguments.
/// Automatically falls back to rspc HTTP when not running in Tauri.
async fn invoke<T: DeserializeOwned>(cmd: &str, args: impl Serialize) -> Result<T, String> {
    if is_tauri() {
        let args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
        let result = tauri_invoke(cmd, args).await;
        serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
    } else {
        // Use rspc HTTP endpoint
        rspc_query(cmd, &args).await
    }
}

/// Invoke a Tauri command with no arguments.
/// Automatically falls back to rspc HTTP when not running in Tauri.
async fn invoke_no_args<T: DeserializeOwned>(cmd: &str) -> Result<T, String> {
    if is_tauri() {
        let result = tauri_invoke(cmd, JsValue::NULL).await;
        serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
    } else {
        // Use rspc HTTP endpoint with empty args
        rspc_query(cmd, &()).await
    }
}

// ============ Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub id: i64,
    pub name: String,
    pub game_count: i64,
    pub aliases: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub database_id: i64,
    pub title: String,
    pub display_title: String,
    pub platform: String,
    pub platform_id: i64,
    pub description: Option<String>,
    pub release_date: Option<String>,
    pub release_year: Option<i32>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genres: Option<String>,
    pub players: Option<String>,
    pub rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub esrb: Option<String>,
    pub cooperative: Option<bool>,
    pub video_url: Option<String>,
    pub wikipedia_url: Option<String>,
    pub release_type: Option<String>,
    pub notes: Option<String>,
    pub sort_title: Option<String>,
    pub series: Option<String>,
    pub region: Option<String>,
    pub play_mode: Option<String>,
    pub version: Option<String>,
    pub status: Option<String>,
    pub steam_app_id: Option<i64>,
    pub box_front_path: Option<String>,
    pub screenshot_path: Option<String>,
    pub variant_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVariant {
    pub id: String,
    pub title: String,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub data_directory: Option<String>,
    #[serde(default)]
    pub media_directory: Option<String>,
    #[serde(default)]
    pub programs_directory: Option<String>,
    #[serde(default)]
    pub saves_directory: Option<String>,
    /// User-defined region priority order (first = highest priority)
    #[serde(default)]
    pub region_priority: Vec<String>,
    #[serde(default)]
    pub screenscraper: ScreenScraperSettings,
    #[serde(default)]
    pub steamgriddb: SteamGridDBSettings,
    #[serde(default)]
    pub igdb: IGDBSettings,
    #[serde(default)]
    pub emumovies: EmuMoviesSettings,
    #[serde(default)]
    pub graboid: GraboidSettings,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScreenScraperSettings {
    #[serde(default)]
    pub dev_id: String,
    #[serde(default)]
    pub dev_password: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub user_password: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SteamGridDBSettings {
    #[serde(default)]
    pub api_key: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IGDBSettings {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EmuMoviesSettings {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraboidSettings {
    #[serde(default)]
    pub server_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub import_directory: Option<String>,
    #[serde(default)]
    pub default_prompt: String,
}

// ============ Helpers ============

/// Convert a file path to an asset URL for Tauri's asset protocol
pub fn file_to_asset_url(path: &str) -> String {
    if is_tauri() {
        // Tauri 2 uses asset://localhost/{path}
        // The path needs to be URL-encoded
        let encoded = path
            .replace(' ', "%20")
            .replace('#', "%23")
            .replace('?', "%3F")
            .replace('&', "%26");
        format!("asset://localhost/{}", encoded)
    } else {
        // Browser mode: use HTTP API to serve assets
        let encoded = urlencoding::encode(path);
        format!("{}/assets/{}", HTTP_API_BASE, encoded)
    }
}

// ============ Commands ============

/// Get all platforms
pub async fn get_platforms() -> Result<Vec<Platform>, String> {
    if is_tauri() {
        invoke_no_args("get_platforms").await
    } else {
        http_get("/api/platforms").await
    }
}

/// Get all unique regions from the games database
pub async fn get_all_regions() -> Result<Vec<String>, String> {
    if is_tauri() {
        invoke_no_args("get_all_regions").await
    } else {
        http_get("/api/regions").await
    }
}

/// Get total game count for a platform/search
pub async fn get_game_count(platform: Option<String>, search: Option<String>) -> Result<i64, String> {
    if is_tauri() {
        #[derive(Serialize)]
        struct Args {
            platform: Option<String>,
            search: Option<String>,
        }
        invoke("get_game_count", Args { platform, search }).await
    } else {
        let mut query = vec![];
        if let Some(p) = &platform {
            query.push(format!("platform={}", urlencoding::encode(p)));
        }
        if let Some(s) = &search {
            query.push(format!("search={}", urlencoding::encode(s)));
        }
        let path = if query.is_empty() {
            "/api/games/count".to_string()
        } else {
            format!("/api/games/count?{}", query.join("&"))
        };
        http_get(&path).await
    }
}

/// Get games, optionally filtered by platform or search query
pub async fn get_games(
    platform: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Game>, String> {
    if is_tauri() {
        #[derive(Serialize)]
        struct Args {
            platform: Option<String>,
            search: Option<String>,
            limit: Option<i64>,
            offset: Option<i64>,
        }
        invoke("get_games", Args { platform, search, limit, offset }).await
    } else {
        let mut query = vec![];
        if let Some(p) = &platform {
            query.push(format!("platform={}", urlencoding::encode(p)));
        }
        if let Some(s) = &search {
            query.push(format!("search={}", urlencoding::encode(s)));
        }
        if let Some(l) = limit {
            query.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            query.push(format!("offset={}", o));
        }
        let path = if query.is_empty() {
            "/api/games".to_string()
        } else {
            format!("/api/games?{}", query.join("&"))
        };
        http_get(&path).await
    }
}

/// Get a single game by database ID
pub async fn get_game_by_id(database_id: i64) -> Result<Option<Game>, String> {
    #[derive(Serialize)]
    struct Args {
        database_id: i64,
    }
    invoke("get_game_by_id", Args { database_id }).await
}

/// Get a game by its UUID
pub async fn get_game_by_uuid(game_id: String) -> Result<Option<Game>, String> {
    if is_tauri() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            game_id: String,
        }
        invoke("get_game_by_uuid", Args { game_id }).await
    } else {
        http_get(&format!("/api/games/{}", urlencoding::encode(&game_id))).await
    }
}

/// Get all variants (regions/versions) for a game
pub async fn get_game_variants(game_id: String, display_title: String, platform_id: i64) -> Result<Vec<GameVariant>, String> {
    if is_tauri() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            display_title: String,
            platform_id: i64,
        }
        invoke("get_game_variants", Args { display_title, platform_id }).await
    } else {
        let path = format!("/api/games/{}/variants", urlencoding::encode(&game_id));
        http_get(&path).await
    }
}

/// Get settings
pub async fn get_settings() -> Result<AppSettings, String> {
    if is_tauri() {
        invoke_no_args("get_settings").await
    } else {
        http_get("/api/settings").await
    }
}

/// Get the name of where credentials are stored (keyring or database)
pub async fn get_credential_storage_name() -> Result<String, String> {
    if is_tauri() {
        invoke_no_args("get_credential_storage_name").await
    } else {
        http_get("/api/credential-storage").await
    }
}

/// Save settings
pub async fn save_settings(settings: AppSettings) -> Result<(), String> {
    if is_tauri() {
        #[derive(Serialize)]
        struct Args {
            settings: AppSettings,
        }
        invoke("save_settings", Args { settings }).await
    } else {
        http_post_empty("/api/settings", &settings).await
    }
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
    if is_tauri() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            launchbox_db_id: i64,
        }
        invoke("get_play_stats", Args { launchbox_db_id }).await
    } else {
        http_get(&format!("/api/stats/{}", launchbox_db_id)).await
    }
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
    if is_tauri() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            launchbox_db_id: i64,
        }
        invoke("is_favorite", Args { launchbox_db_id }).await
    } else {
        http_get(&format!("/api/favorites/check/{}", launchbox_db_id)).await
    }
}

/// Get all favorite games
pub async fn get_favorites() -> Result<Vec<Game>, String> {
    if is_tauri() {
        invoke_no_args("get_favorites").await
    } else {
        http_get("/api/favorites").await
    }
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

/// Test EmuMovies API connection
pub async fn test_emumovies_connection(
    username: String,
    password: String,
) -> Result<ConnectionTestResult, String> {
    #[derive(Serialize)]
    struct Args {
        username: String,
        password: String,
    }
    invoke("test_emumovies_connection", Args { username, password }).await
}

// ============ Image Types and Commands ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    pub id: i64,
    pub launchbox_db_id: i64,
    pub image_type: String,
    pub region: Option<String>,
    pub cdn_url: String,
    pub local_path: Option<String>,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub total_images: i64,
    pub downloaded_images: i64,
    pub disk_usage_bytes: u64,
}

/// Get all images for a game
pub async fn get_game_images(launchbox_db_id: i64) -> Result<Vec<ImageInfo>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
    }
    invoke("get_game_images", Args { launchbox_db_id }).await
}

/// Get a specific image type for a game
pub async fn get_game_image(launchbox_db_id: i64, image_type: String) -> Result<Option<ImageInfo>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
        image_type: String,
    }
    invoke("get_game_image", Args { launchbox_db_id, image_type }).await
}

/// Get available image types for a game
pub async fn get_available_image_types(launchbox_db_id: i64) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
    }
    invoke("get_available_image_types", Args { launchbox_db_id }).await
}

/// Download a specific image and return its local path
pub async fn download_image(image_id: i64) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        image_id: i64,
    }
    invoke("download_image", Args { image_id }).await
}

/// Download images for a game (box front and screenshot by default)
pub async fn download_game_images(
    launchbox_db_id: i64,
    image_types: Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
        image_types: Option<Vec<String>>,
    }
    invoke("download_game_images", Args { launchbox_db_id, image_types }).await
}

/// Get image cache statistics
pub async fn get_image_cache_stats() -> Result<CacheStats, String> {
    invoke_no_args("get_image_cache_stats").await
}

/// Result from cache check
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedMediaResult {
    pub path: String,
    pub source: String,
}

/// Check if media is cached locally (fast path - no network requests)
pub async fn check_cached_media(
    game_title: String,
    platform: String,
    image_type: String,
    launchbox_db_id: Option<i64>,
) -> Result<Option<CachedMediaResult>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        game_title: String,
        platform: String,
        image_type: String,
        launchbox_db_id: Option<i64>,
    }
    invoke("check_cached_media", Args {
        game_title,
        platform,
        image_type,
        launchbox_db_id,
    }).await
}

/// Download an image with fallback to multiple sources
///
/// Tries sources in order: LaunchBox CDN, libretro-thumbnails, SteamGridDB
pub async fn download_image_with_fallback(
    game_title: String,
    platform: String,
    image_type: String,
    launchbox_db_id: Option<i64>,
) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        game_title: String,
        platform: String,
        image_type: String,
        launchbox_db_id: Option<i64>,
    }
    invoke("download_image_with_fallback", Args {
        game_title,
        platform,
        image_type,
        launchbox_db_id,
    }).await
}

/// Redownload an image from the next source in rotation
/// Deletes the current cached image and tries the next available source
pub async fn redownload_image_from_next_source(
    game_title: String,
    platform: String,
    image_type: String,
    launchbox_db_id: Option<i64>,
    current_source: String,
) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        game_title: String,
        platform: String,
        image_type: String,
        launchbox_db_id: Option<i64>,
        current_source: String,
    }
    invoke("redownload_image_from_next_source", Args {
        game_title,
        platform,
        image_type,
        launchbox_db_id,
        current_source,
    }).await
}

/// Download a thumbnail from libretro-thumbnails
pub async fn download_libretro_thumbnail(
    game_title: String,
    platform: String,
    image_type: String,
) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        game_title: String,
        platform: String,
        image_type: String,
    }
    invoke("download_libretro_thumbnail", Args {
        game_title,
        platform,
        image_type,
    }).await
}

// ============ Unified Media Download Commands ============

/// Available media types for download
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    pub id: String,
    pub name: String,
}

/// Media download event from backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MediaEvent {
    /// Download has started
    #[serde(rename_all = "camelCase")]
    Started {
        game_id: i64,
        media_type: String,
        source: String,
    },
    /// Download progress update
    #[serde(rename_all = "camelCase")]
    Progress {
        game_id: i64,
        media_type: String,
        progress: f32,
        source: String,
    },
    /// Download completed successfully
    #[serde(rename_all = "camelCase")]
    Completed {
        game_id: i64,
        media_type: String,
        local_path: String,
        source: String,
    },
    /// Download failed
    #[serde(rename_all = "camelCase")]
    Failed {
        game_id: i64,
        media_type: String,
        error: String,
        source: String,
    },
    /// Download was cancelled
    #[serde(rename_all = "camelCase")]
    Cancelled {
        game_id: i64,
        media_type: String,
    },
}

impl MediaEvent {
    pub fn game_id(&self) -> i64 {
        match self {
            MediaEvent::Started { game_id, .. } => *game_id,
            MediaEvent::Progress { game_id, .. } => *game_id,
            MediaEvent::Completed { game_id, .. } => *game_id,
            MediaEvent::Failed { game_id, .. } => *game_id,
            MediaEvent::Cancelled { game_id, .. } => *game_id,
        }
    }
}

/// Get all available normalized media types
pub async fn get_media_types() -> Result<Vec<MediaType>, String> {
    invoke_no_args("get_media_types").await
}

/// Download media for a game using the unified system with round-robin source selection
pub async fn download_unified_media(
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
    media_type: String,
) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
        game_title: String,
        platform: String,
        media_type: String,
    }
    invoke("download_unified_media", Args {
        launchbox_db_id,
        game_title,
        platform,
        media_type,
    }).await
}

/// Get the cached path for a media file (if it exists)
pub async fn get_cached_media_path(
    launchbox_db_id: i64,
    media_type: String,
) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
        media_type: String,
    }
    invoke("get_cached_media_path", Args {
        launchbox_db_id,
        media_type,
    }).await
}

// ============ Video Download Commands ============

/// Video download event from backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VideoEvent {
    /// Video download has started
    #[serde(rename_all = "camelCase")]
    Started {
        game_id: i64,
        total_bytes: Option<u64>,
    },
    /// Video download progress update
    #[serde(rename_all = "camelCase")]
    Progress {
        game_id: i64,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        progress: f32,
    },
    /// Video download completed successfully
    #[serde(rename_all = "camelCase")]
    Completed {
        game_id: i64,
        local_path: String,
    },
    /// Video download failed
    #[serde(rename_all = "camelCase")]
    Failed {
        game_id: i64,
        error: String,
    },
}

impl VideoEvent {
    pub fn game_id(&self) -> i64 {
        match self {
            VideoEvent::Started { game_id, .. } => *game_id,
            VideoEvent::Progress { game_id, .. } => *game_id,
            VideoEvent::Completed { game_id, .. } => *game_id,
            VideoEvent::Failed { game_id, .. } => *game_id,
        }
    }
}

/// Check if a video is cached for a game
pub async fn check_cached_video(
    game_title: String,
    platform: String,
    launchbox_db_id: Option<i64>,
) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        game_title: String,
        platform: String,
        launchbox_db_id: Option<i64>,
    }
    invoke("check_cached_video", Args {
        game_title,
        platform,
        launchbox_db_id,
    }).await
}

/// Download a video for a game from EmuMovies
pub async fn download_game_video(
    game_title: String,
    platform: String,
    launchbox_db_id: Option<i64>,
) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        game_title: String,
        platform: String,
        launchbox_db_id: Option<i64>,
    }
    invoke("download_game_video", Args {
        game_title,
        platform,
        launchbox_db_id,
    }).await
}

// ============ Emulator Commands ============

/// Emulator information from the emulators database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorInfo {
    pub id: i64,
    pub name: String,
    pub homepage: Option<String>,
    pub supported_os: Option<String>,
    pub winget_id: Option<String>,
    pub homebrew_formula: Option<String>,
    pub flatpak_id: Option<String>,
    pub retroarch_core: Option<String>,
    pub save_directory: Option<String>,
    pub save_extensions: Option<String>,
    pub notes: Option<String>,
}

/// Get all emulators for a platform, filtered by current OS
pub async fn get_emulators_for_platform(platform_name: String) -> Result<Vec<EmulatorInfo>, String> {
    invoke("get_emulators_for_platform", platform_name).await
}

/// Get a specific emulator by name
pub async fn get_emulator(name: String) -> Result<Option<EmulatorInfo>, String> {
    invoke("get_emulator", name).await
}

/// Get all emulators (optionally filtered by current OS)
pub async fn get_all_emulators(filter_os: Option<bool>) -> Result<Vec<EmulatorInfo>, String> {
    #[derive(Serialize)]
    struct Args {
        filter_os: Option<bool>,
    }
    invoke("get_all_emulators", Args { filter_os }).await
}

// ============ Emulator Preference Commands ============

/// Per-game emulator preference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEmulatorPref {
    pub launchbox_db_id: i64,
    pub emulator_name: String,
    pub game_title: Option<String>,
}

/// Per-platform emulator preference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformEmulatorPref {
    pub platform_name: String,
    pub emulator_name: String,
}

/// All emulator preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorPreferences {
    pub game_preferences: Vec<GameEmulatorPref>,
    pub platform_preferences: Vec<PlatformEmulatorPref>,
}

/// Get emulator preference for a game (checks game-specific, then platform)
pub async fn get_emulator_preference(launchbox_db_id: i64, platform_name: String) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
        platform_name: String,
    }
    invoke("get_emulator_preference", Args { launchbox_db_id, platform_name }).await
}

/// Set emulator preference for a specific game
pub async fn set_game_emulator_preference(launchbox_db_id: i64, emulator_name: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
        emulator_name: String,
    }
    invoke("set_game_emulator_preference", Args { launchbox_db_id, emulator_name }).await
}

/// Set emulator preference for a platform (all games on that platform)
pub async fn set_platform_emulator_preference(platform_name: String, emulator_name: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        platform_name: String,
        emulator_name: String,
    }
    invoke("set_platform_emulator_preference", Args { platform_name, emulator_name }).await
}

/// Clear a game-specific preference
pub async fn clear_game_emulator_preference(launchbox_db_id: i64) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
    }
    invoke("clear_game_emulator_preference", Args { launchbox_db_id }).await
}

/// Clear a platform preference
pub async fn clear_platform_emulator_preference(platform_name: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        platform_name: String,
    }
    invoke("clear_platform_emulator_preference", Args { platform_name }).await
}

/// Get all emulator preferences (for settings UI)
pub async fn get_all_emulator_preferences() -> Result<EmulatorPreferences, String> {
    invoke_no_args("get_all_emulator_preferences").await
}

/// Clear all emulator preferences
pub async fn clear_all_emulator_preferences() -> Result<(), String> {
    invoke_no_args("clear_all_emulator_preferences").await
}

// ============ Emulator Installation & Launch ============

/// Emulator with installation status for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorWithStatus {
    // Flatten the base EmulatorInfo fields
    pub id: i64,
    pub name: String,
    pub homepage: Option<String>,
    pub supported_os: Option<String>,
    pub winget_id: Option<String>,
    pub homebrew_formula: Option<String>,
    pub flatpak_id: Option<String>,
    pub retroarch_core: Option<String>,
    pub save_directory: Option<String>,
    pub save_extensions: Option<String>,
    pub notes: Option<String>,
    // Additional status fields
    pub is_installed: bool,
    pub install_method: Option<String>,
    pub is_retroarch_core: bool,
    pub display_name: String,
    pub executable_path: Option<String>,
}

/// Result of launching a game
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub error: Option<String>,
}

/// Get all emulators for a platform with installation status
pub async fn get_emulators_with_status(platform_name: String) -> Result<Vec<EmulatorWithStatus>, String> {
    invoke("get_emulators_with_status", platform_name).await
}

/// Install an emulator using the appropriate package manager
pub async fn install_emulator(emulator_name: String, is_retroarch_core: bool) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        emulator_name: String,
        is_retroarch_core: bool,
    }
    invoke("install_emulator", Args { emulator_name, is_retroarch_core }).await
}

/// Launch an emulator (without a ROM)
pub async fn launch_emulator(emulator_name: String, is_retroarch_core: bool) -> Result<LaunchResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        emulator_name: String,
        is_retroarch_core: bool,
    }
    invoke("launch_emulator", Args { emulator_name, is_retroarch_core }).await
}

/// Launch a game with the specified emulator
pub async fn launch_game(emulator_name: String, rom_path: String) -> Result<LaunchResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        emulator_name: String,
        rom_path: String,
    }
    invoke("launch_game", Args { emulator_name, rom_path }).await
}

/// Get the current operating system
pub async fn get_current_os() -> Result<String, String> {
    invoke_no_args("get_current_os").await
}

// ============ Graboid Import Types & Commands ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFile {
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub imported_at: String,
    pub import_source: String,
    pub graboid_job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportJob {
    pub id: String,
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    pub status: String,
    pub progress_percent: f64,
    pub status_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraboidPrompt {
    pub id: i64,
    pub scope: String,
    pub platform: Option<String>,
    pub launchbox_db_id: Option<i64>,
    pub prompt: String,
}

/// Check if a game has an imported file
pub async fn get_game_file(launchbox_db_id: i64) -> Result<Option<GameFile>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args { launchbox_db_id: i64 }
    invoke("get_game_file", Args { launchbox_db_id }).await
}

/// Get active import job for a game
pub async fn get_active_import(launchbox_db_id: i64) -> Result<Option<ImportJob>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args { launchbox_db_id: i64 }
    invoke("get_active_import", Args { launchbox_db_id }).await
}

/// Start a Graboid import job
pub async fn start_graboid_import(
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
) -> Result<ImportJob, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        launchbox_db_id: i64,
        game_title: String,
        platform: String,
    }
    invoke("start_graboid_import", Args { launchbox_db_id, game_title, platform }).await
}

/// Cancel an import job
pub async fn cancel_import(job_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args { job_id: String }
    invoke("cancel_import", Args { job_id }).await
}

/// Test connection to Graboid server
pub async fn test_graboid_connection(
    server_url: String,
    api_key: String,
) -> Result<ConnectionTestResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        server_url: String,
        api_key: String,
    }
    invoke("test_graboid_connection", Args { server_url, api_key }).await
}

/// Get all graboid prompts
pub async fn get_graboid_prompts() -> Result<Vec<GraboidPrompt>, String> {
    invoke_no_args("get_graboid_prompts").await
}

/// Save a graboid prompt
pub async fn save_graboid_prompt(
    scope: String,
    platform: Option<String>,
    launchbox_db_id: Option<i64>,
    prompt: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        scope: String,
        platform: Option<String>,
        launchbox_db_id: Option<i64>,
        prompt: String,
    }
    invoke("save_graboid_prompt", Args { scope, platform, launchbox_db_id, prompt }).await
}

/// Delete a graboid prompt
pub async fn delete_graboid_prompt(
    scope: String,
    platform: Option<String>,
    launchbox_db_id: Option<i64>,
) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        scope: String,
        platform: Option<String>,
        launchbox_db_id: Option<i64>,
    }
    invoke("delete_graboid_prompt", Args { scope, platform, launchbox_db_id }).await
}

/// Get the effective graboid prompt for a game (global + platform + game combined)
pub async fn get_effective_graboid_prompt(
    platform: String,
    launchbox_db_id: i64,
) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        platform: String,
        launchbox_db_id: i64,
    }
    invoke("get_effective_graboid_prompt", Args { platform, launchbox_db_id }).await
}

/// Get the SSE endpoint URL for a Graboid job
pub fn graboid_sse_url(job_id: &str) -> String {
    format!("{}/api/graboid/jobs/{}/events", HTTP_API_BASE, job_id)
}
