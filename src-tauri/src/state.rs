//! Application state management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::db;
use crate::import::LaunchBoxImporter;

/// Application state shared across commands
pub struct AppState {
    /// User database (collections, favorites, play stats, settings)
    pub db_pool: Option<SqlitePool>,
    /// Shipped games database (LibRetro-based, read-only)
    pub games_db_pool: Option<SqlitePool>,
    /// LaunchBox importer (optional, for users with existing libraries)
    pub launchbox_importer: Option<LaunchBoxImporter>,
    pub settings: AppSettings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            db_pool: None,
            games_db_pool: None,
            launchbox_importer: None,
            settings: AppSettings::default(),
        }
    }
}

/// User-configurable settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub rom_directories: Vec<PathBuf>,
    pub launchbox_path: Option<PathBuf>,
    pub retroarch_path: Option<PathBuf>,
    #[serde(default)]
    pub cache_directory: Option<PathBuf>,
    pub emulators: Vec<EmulatorConfig>,
    pub default_platform_emulators: std::collections::HashMap<String, String>,
    pub screenscraper: ScreenScraperSettings,
    #[serde(default)]
    pub steamgriddb: SteamGridDBSettings,
    #[serde(default)]
    pub igdb: IGDBSettings,
}

/// ScreenScraper API settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScreenScraperSettings {
    pub dev_id: String,
    pub dev_password: String,
    pub user_id: Option<String>,
    pub user_password: Option<String>,
}

/// SteamGridDB API settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SteamGridDBSettings {
    pub api_key: String,
}

/// IGDB (Twitch) API settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IGDBSettings {
    pub client_id: String,
    pub client_secret: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            rom_directories: vec![
                PathBuf::from("/mnt/roms"),
                PathBuf::from("/mnt/ext/roms"),
            ],
            launchbox_path: Some(PathBuf::from("/mnt/Windows/Users/benwb/LaunchBox")),
            retroarch_path: None,
            cache_directory: None, // Will use app data dir if None
            emulators: Vec::new(),
            default_platform_emulators: std::collections::HashMap::new(),
            screenscraper: ScreenScraperSettings::default(),
            steamgriddb: SteamGridDBSettings::default(),
            igdb: IGDBSettings::default(),
        }
    }
}

/// Emulator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorConfig {
    pub id: String,
    pub name: String,
    pub executable_path: PathBuf,
    pub emulator_type: EmulatorType,
    pub command_template: String,
    pub supported_platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulatorType {
    RetroArch,
    Standalone,
}

/// Initialize app state on startup
pub async fn initialize_app_state(app: &AppHandle) -> Result<()> {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    let state = app.state::<std::sync::Arc<tokio::sync::RwLock<AppState>>>();

    // Get the app data directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");

    std::fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("lunchbox.db");
    tracing::info!("User database path: {}", db_path.display());

    // Initialize user database (collections, favorites, settings)
    let pool = db::init_pool(&db_path).await?;

    // Load settings from database
    let settings = load_settings(&pool).await.unwrap_or_default();

    // Try to load shipped games database
    // First check app resource directory, then common paths
    let games_db_pool = {
        let resource_path = app.path().resource_dir()
            .ok()
            .map(|p| p.join("games.db"));

        let possible_paths = [
            resource_path,
            Some(app_data_dir.join("games.db")),
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
                        tracing::warn!("Failed to connect to games database at {}: {}", path_opt.display(), e);
                    }
                }
            }
        }

        if found_pool.is_none() {
            tracing::warn!("No games database found. Browse-first mode disabled.");
            tracing::info!("To enable, place games.db in the app data directory or run:");
            tracing::info!("  lunchbox-cli build-db --output {}", app_data_dir.join("games.db").display());
        }

        found_pool
    };

    // Try to connect to LaunchBox if configured (optional supplement)
    let launchbox_importer = if let Some(ref lb_path) = settings.launchbox_path {
        let metadata_path = lb_path.join("Metadata").join("LaunchBox.Metadata.db");
        if metadata_path.exists() {
            match LaunchBoxImporter::connect(&metadata_path).await {
                Ok(importer) => {
                    tracing::info!("Connected to LaunchBox metadata database");
                    Some(importer)
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to LaunchBox: {}", e);
                    None
                }
            }
        } else {
            tracing::info!("LaunchBox metadata database not found at {}", metadata_path.display());
            None
        }
    } else {
        None
    };

    // Update state
    let mut state_guard = state.write().await;
    state_guard.db_pool = Some(pool);
    state_guard.games_db_pool = games_db_pool;
    state_guard.launchbox_importer = launchbox_importer;
    state_guard.settings = settings;

    tracing::info!("App state initialized successfully");
    Ok(())
}

/// Load settings from database
async fn load_settings(pool: &SqlitePool) -> Result<AppSettings> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = 'app_settings'")
        .fetch_optional(pool)
        .await?;

    if let Some((json,)) = row {
        Ok(serde_json::from_str(&json)?)
    } else {
        Ok(AppSettings::default())
    }
}

/// Save settings to database
pub async fn save_settings(pool: &SqlitePool, settings: &AppSettings) -> Result<()> {
    let json = serde_json::to_string(settings)?;

    sqlx::query(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('app_settings', ?)"
    )
    .bind(&json)
    .execute(pool)
    .await?;

    Ok(())
}
