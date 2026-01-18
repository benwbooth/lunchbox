//! Application state management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::db;

/// Application state shared across commands
pub struct AppState {
    /// User database (collections, favorites, play stats, settings)
    /// Only created when user actually saves something
    pub db_pool: Option<SqlitePool>,
    /// Path to user database (for lazy creation)
    pub user_db_path: Option<std::path::PathBuf>,
    /// Shipped games database (read-only)
    pub games_db_pool: Option<SqlitePool>,
    /// Separate game images database (LaunchBox CDN metadata, read-only)
    pub images_db_pool: Option<SqlitePool>,
    pub settings: AppSettings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            db_pool: None,
            user_db_path: None,
            games_db_pool: None,
            images_db_pool: None,
            settings: AppSettings::default(),
        }
    }
}

/// User-configurable settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Directory for app data (database, settings). Defaults to OS app data dir.
    #[serde(default)]
    pub data_directory: Option<PathBuf>,
    /// Directory for media files (images, videos). Defaults to data_directory/media.
    #[serde(default)]
    pub media_directory: Option<PathBuf>,
    /// Directory for programs (emulators, cores). Defaults to data_directory/programs.
    #[serde(default)]
    pub programs_directory: Option<PathBuf>,
    /// Directory for save game backups. Defaults to data_directory/saves.
    #[serde(default)]
    pub saves_directory: Option<PathBuf>,

    // Image source API credentials (stored in keyring when available)
    #[serde(default)]
    pub screenscraper: ScreenScraperSettings,
    #[serde(default)]
    pub steamgriddb: SteamGridDBSettings,
    #[serde(default)]
    pub igdb: IGDBSettings,
    #[serde(default)]
    pub emumovies: EmuMoviesSettings,
}

/// ScreenScraper API settings
/// Note: Credentials are stored in system keyring, not in JSON config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// SteamGridDB API settings
/// Note: Credentials are stored in system keyring, not in JSON config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SteamGridDBSettings {
    #[serde(default)]
    pub api_key: String,
}

/// IGDB (Twitch) API settings
/// Note: Credentials are stored in system keyring, not in JSON config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IGDBSettings {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
}

/// EmuMovies API settings
/// Note: Credentials are stored in system keyring, not in JSON config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmuMoviesSettings {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            data_directory: None,
            media_directory: None,
            programs_directory: None,
            saves_directory: None,
            screenscraper: ScreenScraperSettings::default(),
            steamgriddb: SteamGridDBSettings::default(),
            igdb: IGDBSettings::default(),
            emumovies: EmuMoviesSettings::default(),
        }
    }
}

impl AppSettings {
    /// Get the base data directory (uses OS-appropriate default if not set)
    /// - Linux: ~/.local/share/lunchbox
    /// - macOS: ~/Library/Application Support/lunchbox
    /// - Windows: %APPDATA%\lunchbox
    pub fn get_data_directory(&self) -> PathBuf {
        self.data_directory.clone().unwrap_or_else(|| {
            directories::ProjectDirs::from("", "", "lunchbox")
                .map(|dirs| dirs.data_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        })
    }

    /// Get the media directory (images, videos, etc.)
    pub fn get_media_directory(&self) -> PathBuf {
        self.media_directory.clone().unwrap_or_else(|| {
            self.get_data_directory().join("media")
        })
    }

    /// Get the programs directory (emulators, cores, etc.)
    pub fn get_programs_directory(&self) -> PathBuf {
        self.programs_directory.clone().unwrap_or_else(|| {
            self.get_data_directory().join("programs")
        })
    }

    /// Get the saves directory (save game backups)
    pub fn get_saves_directory(&self) -> PathBuf {
        self.saves_directory.clone().unwrap_or_else(|| {
            self.get_data_directory().join("saves")
        })
    }
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

    // User database path - only created when needed (first write operation)
    let user_db_path = app_data_dir.join("user.db");

    // Initialize user database only if it already exists
    // This avoids creating empty database files
    let user_pool = if user_db_path.exists() {
        tracing::info!("Found user database at: {}", user_db_path.display());
        Some(db::init_pool(&user_db_path).await?)
    } else {
        tracing::info!("No user database yet (will be created on first write)");
        None
    };

    // Load settings from user database if it exists
    let settings = if let Some(ref pool) = user_pool {
        load_settings(pool).await.unwrap_or_default()
    } else {
        AppSettings::default()
    };

    // Try to load games database (read-only)
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
                        tracing::info!("Connected to games database (read-only)");
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

    // Try to load game_images database (separate file for LaunchBox CDN metadata)
    let images_db_pool = {
        let resource_path = app.path().resource_dir()
            .ok()
            .map(|p| p.join("game_images.db"));

        let possible_paths = [
            resource_path,
            Some(app_data_dir.join("game_images.db")),
            Some(PathBuf::from("./db/game_images.db")),  // Dev mode
            Some(PathBuf::from("/usr/share/lunchbox/game_images.db")),
        ];

        let mut found_pool = None;
        for path_opt in possible_paths.iter().flatten() {
            if path_opt.exists() {
                tracing::info!("Found images database at: {}", path_opt.display());
                let db_url = format!("sqlite:{}?mode=ro", path_opt.display());
                match SqlitePoolOptions::new()
                    .max_connections(4)
                    .connect_with(SqliteConnectOptions::from_str(&db_url)?.read_only(true))
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Connected to images database (read-only)");
                        found_pool = Some(pool);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to connect to images database at {}: {}", path_opt.display(), e);
                    }
                }
            }
        }

        if found_pool.is_none() {
            tracing::info!("No images database found (LaunchBox CDN will be disabled)");
        }

        found_pool
    };

    // Update state
    let mut state_guard = state.write().await;
    state_guard.db_pool = user_pool;
    state_guard.games_db_pool = games_db_pool;
    state_guard.images_db_pool = images_db_pool;
    state_guard.settings = settings;
    state_guard.user_db_path = Some(user_db_path);

    tracing::info!("App state initialized successfully");
    Ok(())
}

/// Ensure user database exists and is initialized
/// Call this before any write operation to the user database
pub async fn ensure_user_db(state: &mut AppState) -> Result<&SqlitePool> {
    if state.db_pool.is_some() {
        return Ok(state.db_pool.as_ref().unwrap());
    }

    let path = state.user_db_path.as_ref()
        .ok_or_else(|| anyhow::anyhow!("User database path not set"))?;

    tracing::info!("Creating user database at: {}", path.display());
    let pool = db::init_pool(path).await?;
    state.db_pool = Some(pool);

    Ok(state.db_pool.as_ref().unwrap())
}

/// Load settings from database and keyring
async fn load_settings(pool: &SqlitePool) -> Result<AppSettings> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = 'app_settings'")
        .fetch_optional(pool)
        .await?;

    let mut settings = if let Some((json,)) = row {
        serde_json::from_str(&json)?
    } else {
        AppSettings::default()
    };

    // Load credentials from system keyring
    let creds = crate::keyring_store::load_image_source_credentials();
    settings.steamgriddb.api_key = creds.steamgriddb_api_key;
    settings.igdb.client_id = creds.igdb_client_id;
    settings.igdb.client_secret = creds.igdb_client_secret;
    settings.emumovies.username = creds.emumovies_username;
    settings.emumovies.password = creds.emumovies_password;
    settings.screenscraper.dev_id = creds.screenscraper_dev_id;
    settings.screenscraper.dev_password = creds.screenscraper_dev_password;
    settings.screenscraper.user_id = creds.screenscraper_user_id;
    settings.screenscraper.user_password = creds.screenscraper_user_password;

    Ok(settings)
}

/// Save settings to database and credentials to keyring (if available)
pub async fn save_settings(pool: &SqlitePool, settings: &AppSettings) -> Result<()> {
    // Try to save credentials to system keyring
    crate::keyring_store::store_image_source_credentials(
        &settings.steamgriddb.api_key,
        &settings.igdb.client_id,
        &settings.igdb.client_secret,
        &settings.emumovies.username,
        &settings.emumovies.password,
        &settings.screenscraper.dev_id,
        &settings.screenscraper.dev_password,
        settings.screenscraper.user_id.as_deref(),
        settings.screenscraper.user_password.as_deref(),
    )?;

    // If keyring is available, clear credentials from DB copy
    // If not, store them in DB as fallback
    let settings_for_db = if crate::keyring_store::is_keyring_available() {
        let mut s = settings.clone();
        s.steamgriddb = SteamGridDBSettings::default();
        s.igdb = IGDBSettings::default();
        s.emumovies = EmuMoviesSettings::default();
        s.screenscraper = ScreenScraperSettings::default();
        s
    } else {
        settings.clone()
    };

    let json = serde_json::to_string(&settings_for_db)?;

    sqlx::query(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('app_settings', ?)"
    )
    .bind(&json)
    .execute(pool)
    .await?;

    Ok(())
}
