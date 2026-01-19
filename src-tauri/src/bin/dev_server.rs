//! Development server that runs the HTTP API without Tauri's webview.
//!
//! Usage:
//!   cargo run --bin dev_server
//!
//! This allows hot-reloading the backend while keeping the browser open.

use lunchbox_lib::{api, db::{self, USER_DB_NAME, GAMES_DB_NAME, IMAGES_DB_NAME, APP_DATA_DIR}, router, state::AppState};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Decompress a zstd-compressed database file
fn decompress_database(compressed_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    use std::fs::File;
    use std::io::{BufReader, BufWriter};

    tracing::info!(
        "Decompressing {} to {}...",
        compressed_path.display(),
        output_path.display()
    );

    let input_file = File::open(compressed_path)?;
    let reader = BufReader::new(input_file);
    let mut decoder = zstd::Decoder::new(reader)?;

    let output_file = File::create(output_path)?;
    let mut writer = BufWriter::new(output_file);

    std::io::copy(&mut decoder, &mut writer)?;

    tracing::info!("Decompression complete");
    Ok(())
}

/// Find and decompress a database if the uncompressed version doesn't exist
fn find_or_decompress_database(db_name: &str, data_dir: &Path) -> Option<PathBuf> {
    let db_file = format!("{}.db", db_name);
    let zst_file = format!("{}.db.zst", db_name);

    // Target location for decompressed database
    let target_path = data_dir.join(&db_file);

    // If uncompressed database already exists in data dir, use it
    if target_path.exists() {
        return Some(target_path);
    }

    // Possible locations for compressed or uncompressed database
    let possible_paths = [
        PathBuf::from(format!("../db/{}", db_file)),  // Dev mode (from src-tauri)
        PathBuf::from(format!("./db/{}", db_file)),   // Dev mode (from root)
        PathBuf::from(format!("/usr/share/lunchbox/{}", db_file)),
    ];

    let possible_zst_paths = [
        PathBuf::from(format!("../db/{}", zst_file)),  // Dev mode (from src-tauri)
        PathBuf::from(format!("./db/{}", zst_file)),   // Dev mode (from root)
        PathBuf::from(format!("/usr/share/lunchbox/{}", zst_file)),
    ];

    // First check if uncompressed exists anywhere
    for path in &possible_paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    // Try to find and decompress a .zst file
    for zst_path in &possible_zst_paths {
        if zst_path.exists() {
            if let Err(e) = decompress_database(zst_path, &target_path) {
                tracing::warn!(
                    "Failed to decompress {} to {}: {}",
                    zst_path.display(),
                    target_path.display(),
                    e
                );
                continue;
            }
            return Some(target_path);
        }
    }

    None
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,lunchbox=debug".into()),
        )
        .init();

    tracing::info!("Starting Lunchbox dev server...");

    // Get the app data directory
    let data_dir = directories::BaseDirs::new()
        .map(|dirs| dirs.data_dir().join(APP_DATA_DIR))
        .unwrap_or_else(|| PathBuf::from("."));

    std::fs::create_dir_all(&data_dir)?;

    // User database path - only open if it exists
    let user_db_path = data_dir.join(USER_DB_NAME);
    let db_pool = if user_db_path.exists() {
        tracing::info!("Found user database at: {}", user_db_path.display());
        Some(db::init_pool(&user_db_path).await?)
    } else {
        tracing::info!("No user database yet (will be created on first write)");
        None
    };

    // Load settings from database if it exists
    let mut settings: lunchbox_lib::state::AppSettings = if let Some(ref pool) = db_pool {
        sqlx::query_as::<_, (String,)>(
            "SELECT value FROM settings WHERE key = 'app_settings'",
        )
        .fetch_optional(pool)
        .await?
        .map(|(json,)| serde_json::from_str(&json).unwrap_or_default())
        .unwrap_or_default()
    } else {
        Default::default()
    };

    // Load credentials from system keyring
    let creds = lunchbox_lib::keyring_store::load_image_source_credentials();
    settings.steamgriddb.api_key = creds.steamgriddb_api_key;
    settings.igdb.client_id = creds.igdb_client_id;
    settings.igdb.client_secret = creds.igdb_client_secret;
    settings.emumovies.username = creds.emumovies_username;
    settings.emumovies.password = creds.emumovies_password;
    settings.screenscraper.dev_id = creds.screenscraper_dev_id;
    settings.screenscraper.dev_password = creds.screenscraper_dev_password;
    settings.screenscraper.user_id = creds.screenscraper_user_id;
    settings.screenscraper.user_password = creds.screenscraper_user_password;

    // Find or decompress games database, then connect
    let games_db_pool = {
        match find_or_decompress_database(GAMES_DB_NAME, &data_dir) {
            Some(path) => {
                tracing::info!("Found games database at: {}", path.display());
                let db_url = format!("sqlite:{}?mode=ro", path.display());
                match SqlitePoolOptions::new()
                    .max_connections(4)
                    .connect_with(SqliteConnectOptions::from_str(&db_url)?.read_only(true))
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Connected to games database");
                        Some(pool)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to connect to games database: {}", e);
                        None
                    }
                }
            }
            None => {
                tracing::warn!("No games database found. Browse-first mode disabled.");
                None
            }
        }
    };

    // Find or decompress game_images database, then connect
    let images_db_pool = {
        match find_or_decompress_database(IMAGES_DB_NAME, &data_dir) {
            Some(path) => {
                tracing::info!("Found images database at: {}", path.display());
                let db_url = format!("sqlite:{}?mode=ro", path.display());
                match SqlitePoolOptions::new()
                    .max_connections(4)
                    .connect_with(SqliteConnectOptions::from_str(&db_url)?.read_only(true))
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Connected to images database");
                        Some(pool)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to connect to images database: {}", e);
                        None
                    }
                }
            }
            None => {
                tracing::info!("No images database found (LaunchBox CDN will be disabled)");
                None
            }
        }
    };

    // Create app state
    let state = Arc::new(RwLock::new(AppState {
        db_pool,
        user_db_path: Some(user_db_path),
        games_db_pool,
        images_db_pool,
        settings,
    }));

    // Build rspc router
    let rspc_router = router::build_router();

    // Create and start HTTP server with both legacy and rspc routes
    let legacy_router = api::create_router(state.clone());

    // Create rspc Axum endpoint
    let rspc_ctx = router::Ctx { state };
    let rspc_axum_router = rspc_axum::endpoint(rspc_router, move || rspc_ctx.clone());

    // Merge routers - rspc at /rspc, legacy at /api
    // Add CORS to allow browser requests
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let combined_router = axum::Router::new()
        .nest("/rspc", rspc_axum_router)
        .merge(legacy_router)
        .layer(cors);

    let addr = "127.0.0.1:3001";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("═══════════════════════════════════════════════════════");
    tracing::info!("  HTTP API server running on http://{}", addr);
    tracing::info!("  Frontend dev server:       http://127.0.0.1:1420");
    tracing::info!("");
    tracing::info!("  Run in another terminal:   trunk serve --port 1420");
    tracing::info!("  Then open browser to:      http://127.0.0.1:1420");
    tracing::info!("═══════════════════════════════════════════════════════");

    axum::serve(listener, combined_router).await?;

    Ok(())
}
