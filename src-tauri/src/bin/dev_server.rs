//! Development server that runs the HTTP API without Tauri's webview.
//!
//! Usage:
//!   cargo run --bin dev_server
//!
//! This allows hot-reloading the backend while keeping the browser open.

use lunchbox_lib::{api, db, router, state::AppState};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
        .map(|dirs| dirs.data_dir().join("lunchbox"))
        .unwrap_or_else(|| PathBuf::from("."));

    std::fs::create_dir_all(&data_dir)?;

    // Initialize user database
    let db_path = data_dir.join("lunchbox.db");
    tracing::info!("User database path: {}", db_path.display());
    let db_pool = db::init_pool(&db_path).await?;

    // Load settings from database
    let mut settings: lunchbox_lib::state::AppSettings = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM settings WHERE key = 'app_settings'",
    )
    .fetch_optional(&db_pool)
    .await?
    .map(|(json,)| serde_json::from_str(&json).unwrap_or_default())
    .unwrap_or_default();

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

    // Try to load shipped games database
    let games_db_pool = {
        let possible_paths = [
            Some(data_dir.join("games.db")),
            Some(PathBuf::from("./db/games.db")),  // Dev mode
            Some(PathBuf::from("/usr/share/lunchbox/games.db")),
        ];

        let mut found_pool = None;
        for path_opt in possible_paths.iter().flatten() {
            if path_opt.exists() {
                tracing::info!("Found games database at: {}", path_opt.display());
                let db_url = format!("sqlite:{}?mode=ro", path_opt.display());
                match SqlitePoolOptions::new()
                    .max_connections(4)
                    .connect_with(SqliteConnectOptions::from_str(&db_url)?.read_only(true))
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Connected to games database");
                        found_pool = Some(pool);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to connect to games database at {}: {}",
                            path_opt.display(),
                            e
                        );
                    }
                }
            }
        }

        if found_pool.is_none() {
            tracing::warn!("No games database found. Browse-first mode disabled.");
        }

        found_pool
    };

    // Create app state
    let state = Arc::new(RwLock::new(AppState {
        db_pool: Some(db_pool),
        games_db_pool,
        launchbox_importer: None,
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
