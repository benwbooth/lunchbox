//! Application state management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::db;

/// Application state shared across commands
pub struct AppState {
    /// User database (collections, favorites, play stats)
    /// Only created when user actually saves something
    pub db_pool: Option<SqlitePool>,
    /// Path to user database (for lazy creation)
    pub user_db_path: Option<std::path::PathBuf>,
    /// Path to the user settings TOML file.
    pub settings_path: Option<std::path::PathBuf>,
    /// Shipped games database (read-only)
    pub games_db_pool: Option<SqlitePool>,
    /// Separate game images database (LaunchBox CDN metadata, read-only)
    pub images_db_pool: Option<SqlitePool>,
    /// Emulators database (emulator metadata and platform mappings, read-only)
    pub emulators_db_pool: Option<SqlitePool>,
    /// Minerva Archive index database (read-only, maps ROMs to torrent files)
    pub minerva_db_pool: Option<SqlitePool>,
    pub settings: AppSettings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            db_pool: None,
            user_db_path: None,
            settings_path: None,
            games_db_pool: None,
            images_db_pool: None,
            emulators_db_pool: None,
            minerva_db_pool: None,
            settings: AppSettings::default(),
        }
    }
}

/// User-configurable settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Directory for app data (databases, media defaults, downloads). Defaults to OS app data dir.
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

    /// User-defined region priority order (first = highest priority)
    /// Empty means use default order
    #[serde(default)]
    pub region_priority: Vec<String>,

    // Image source API credentials (stored in keyring when available)
    #[serde(default)]
    pub screenscraper: ScreenScraperSettings,
    #[serde(default)]
    pub steamgriddb: SteamGridDBSettings,
    #[serde(default)]
    pub igdb: IGDBSettings,
    #[serde(default)]
    pub emumovies: EmuMoviesSettings,
    #[serde(default)]
    pub torrent: TorrentSettings,
    #[serde(default)]
    pub controller_mapping: ControllerMappingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerMappingSettings {
    /// Enables launch-time controller remapping. Disabled by default.
    #[serde(default)]
    pub enabled: bool,
    /// Provider backend. "auto" currently resolves to InputPlumber on Linux.
    #[serde(default = "default_controller_mapping_provider")]
    pub provider: String,
    /// InputPlumber target device to expose when a profile is active.
    #[serde(default = "default_controller_mapping_target")]
    pub output_target: String,
    /// Allow Lunchbox to ask InputPlumber to manage all supported devices.
    #[serde(default = "default_controller_mapping_manage_all")]
    pub manage_all: bool,
    /// Built-in profile id or custom profile path used when no scope override matches.
    #[serde(default)]
    pub default_profile_id: Option<String>,
    /// Stable controller ids that should receive the launch profile. Empty means all controllers.
    #[serde(default)]
    pub profile_controller_ids: Vec<String>,
    /// Per-player controller/profile/target rows. Empty preserves the legacy global scope.
    #[serde(default)]
    pub player_mappings: Vec<ControllerPlayerMapping>,
    /// Platform name -> built-in profile id or custom profile path.
    #[serde(default)]
    pub platform_profile_ids: HashMap<String, String>,
    /// LaunchBox DB id string -> built-in profile id or custom profile path.
    #[serde(default)]
    pub game_profile_ids: HashMap<String, String>,
    /// Stable controller ids to suppress or remap during launch.
    #[serde(default)]
    pub hidden_controller_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControllerPlayerMapping {
    /// Stable controller id for this player, or "__all" for every plugged in controller.
    #[serde(default)]
    pub controller_id: Option<String>,
    /// Built-in profile id/custom profile path, "none", or empty to inherit the resolved profile.
    #[serde(default)]
    pub profile_id: Option<String>,
    /// InputPlumber target id, or empty to inherit the global target.
    #[serde(default)]
    pub output_target: Option<String>,
}

impl Default for ControllerMappingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_controller_mapping_provider(),
            output_target: default_controller_mapping_target(),
            manage_all: true,
            default_profile_id: None,
            profile_controller_ids: Vec::new(),
            player_mappings: Vec::new(),
            platform_profile_ids: HashMap::new(),
            game_profile_ids: HashMap::new(),
            hidden_controller_ids: Vec::new(),
        }
    }
}

fn default_controller_mapping_provider() -> String {
    "auto".to_string()
}

fn default_controller_mapping_target() -> String {
    "xb360".to_string()
}

fn default_controller_mapping_manage_all() -> bool {
    true
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

/// Torrent client and download settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentSettings {
    /// Directory for downloaded ROM files, organized by platform subdirs.
    /// Defaults to data_directory/roms.
    #[serde(default)]
    pub rom_directory: Option<PathBuf>,
    /// Path to the ROM directory as seen by qBittorrent when it runs in a container.
    /// Optional; when unset, Lunchbox assumes qBittorrent can access the host path directly.
    #[serde(default)]
    pub qbittorrent_container_rom_directory: Option<PathBuf>,
    /// Separate directory for full-torrent downloads.
    /// Defaults to data_directory/torrent-library.
    #[serde(default)]
    pub torrent_library_directory: Option<PathBuf>,
    /// Path to the full-torrent library directory as seen by qBittorrent when it runs
    /// in a container. Optional; when unset, Lunchbox assumes qBittorrent can access the
    /// host path directly.
    #[serde(default)]
    pub qbittorrent_container_torrent_library_directory: Option<PathBuf>,
    /// Download entire torrent (not just the selected file)
    #[serde(default)]
    pub download_entire_torrent: bool,
    /// How to place ROM files from torrent library into rom directory.
    /// "symlink", "hardlink", "reflink", "copy", "leave_in_place"
    #[serde(default = "default_file_link_mode")]
    pub file_link_mode: String,

    // -- qBittorrent --
    #[serde(default = "default_localhost")]
    pub qbittorrent_host: String,
    #[serde(default = "default_qbittorrent_port")]
    pub qbittorrent_port: u16,
    #[serde(default)]
    pub qbittorrent_username: String,
    #[serde(default)]
    pub qbittorrent_password: String,
}

fn default_file_link_mode() -> String {
    "symlink".to_string()
}
fn default_localhost() -> String {
    "localhost".to_string()
}
fn default_qbittorrent_port() -> u16 {
    8080
}

pub const SETTINGS_FILE_NAME: &str = "settings.toml";

/// Get the user config directory for settings.
/// - Linux: ~/.config/lunchbox
/// - macOS/Windows: OS-specific user config directory
pub fn default_config_directory() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.config_dir().join(db::APP_DATA_DIR))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_settings_file_path() -> PathBuf {
    default_config_directory().join(SETTINGS_FILE_NAME)
}

impl Default for TorrentSettings {
    fn default() -> Self {
        Self {
            rom_directory: None,
            qbittorrent_container_rom_directory: None,
            torrent_library_directory: None,
            qbittorrent_container_torrent_library_directory: None,
            download_entire_torrent: false,
            file_link_mode: default_file_link_mode(),
            qbittorrent_host: default_localhost(),
            qbittorrent_port: default_qbittorrent_port(),
            qbittorrent_username: String::new(),
            qbittorrent_password: String::new(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            data_directory: None,
            media_directory: None,
            programs_directory: None,
            saves_directory: None,
            region_priority: Vec::new(),
            screenscraper: ScreenScraperSettings::default(),
            steamgriddb: SteamGridDBSettings::default(),
            igdb: IGDBSettings::default(),
            emumovies: EmuMoviesSettings::default(),
            torrent: TorrentSettings::default(),
            controller_mapping: ControllerMappingSettings::default(),
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
            directories::ProjectDirs::from("", "", db::APP_DATA_DIR)
                .map(|dirs| dirs.data_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        })
    }

    /// Get the media directory (images, videos, etc.)
    pub fn get_media_directory(&self) -> PathBuf {
        self.media_directory
            .clone()
            .unwrap_or_else(|| self.get_data_directory().join("media"))
    }

    /// Get the programs directory (emulators, cores, etc.)
    pub fn get_programs_directory(&self) -> PathBuf {
        self.programs_directory
            .clone()
            .unwrap_or_else(|| self.get_data_directory().join("programs"))
    }

    /// Get the canonical firmware store directory.
    pub fn get_firmware_directory(&self) -> PathBuf {
        self.get_data_directory().join("firmware")
    }

    /// Get the directory used to store imported firmware packages.
    pub fn get_firmware_packages_directory(&self) -> PathBuf {
        self.get_firmware_directory().join("packages")
    }

    /// Get the directory used for manual firmware drop-in folders.
    pub fn get_manual_firmware_directory(&self) -> PathBuf {
        self.get_firmware_directory().join("manual")
    }

    /// Get the saves directory (save game backups)
    pub fn get_saves_directory(&self) -> PathBuf {
        self.saves_directory
            .clone()
            .unwrap_or_else(|| self.get_data_directory().join("saves"))
    }

    /// Get the import directory for downloaded ROMs
    pub fn get_import_directory(&self) -> PathBuf {
        self.get_rom_directory()
    }

    /// Get the ROM directory for torrent downloads, organized by platform subdirs
    pub fn get_rom_directory(&self) -> PathBuf {
        self.torrent
            .rom_directory
            .clone()
            .unwrap_or_else(|| self.get_data_directory().join("roms"))
    }

    /// Get the torrent library directory for full-torrent downloads
    pub fn get_torrent_library_directory(&self) -> PathBuf {
        self.torrent
            .torrent_library_directory
            .clone()
            .unwrap_or_else(|| self.get_data_directory().join("torrent-library"))
    }
}

/// Decompress a zstd-compressed database file
fn decompress_database(compressed_path: &Path, output_path: &Path) -> Result<()> {
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
/// Returns the path to the uncompressed database if found/decompressed
fn find_or_decompress_database(
    db_name: &str,
    app_data_dir: &Path,
    resource_dir: Option<&Path>,
) -> Option<PathBuf> {
    let db_file = format!("{}.db", db_name);
    let zst_file = format!("{}.db.zst", db_name);

    // Target location for decompressed database
    let target_path = app_data_dir.join(&db_file);

    // If uncompressed database already exists in app data dir, use it
    if target_path.exists() {
        return Some(target_path);
    }

    // Possible locations for compressed database
    let possible_zst_paths: Vec<PathBuf> = [
        resource_dir.map(|p| p.join(&zst_file)),
        Some(PathBuf::from(format!("../db/{}", zst_file))), // Dev mode (from backend)
        Some(PathBuf::from(format!("./db/{}", zst_file))),  // Dev mode (from root)
        Some(PathBuf::from(format!(
            "/usr/share/{}/{}",
            db::APP_DATA_DIR,
            zst_file
        ))),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Also check for uncompressed in other locations (dev mode, system)
    let possible_db_paths: Vec<PathBuf> = [
        resource_dir.map(|p| p.join(&db_file)),
        Some(PathBuf::from(format!("../db/{}", db_file))), // Dev mode (from backend)
        Some(PathBuf::from(format!("./db/{}", db_file))),  // Dev mode (from root)
        Some(PathBuf::from(format!(
            "/usr/share/{}/{}",
            db::APP_DATA_DIR,
            db_file
        ))),
    ]
    .into_iter()
    .flatten()
    .collect();

    // First check if uncompressed exists anywhere
    for path in &possible_db_paths {
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

/// Initialize app state from explicit data/resource directories.
pub async fn initialize_app_state(
    state: &std::sync::Arc<tokio::sync::RwLock<AppState>>,
    app_data_dir: PathBuf,
    resource_dir: Option<PathBuf>,
) -> Result<()> {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    std::fs::create_dir_all(&app_data_dir)?;

    // User database path - only created when needed (first write operation)
    let user_db_path = app_data_dir.join(db::USER_DB_NAME);
    let settings_path = default_settings_file_path();

    // Initialize user database only if it already exists
    // This avoids creating empty database files
    let user_pool = if user_db_path.exists() {
        tracing::info!("Found user database at: {}", user_db_path.display());
        let pool = db::init_pool(&user_db_path).await?;
        crate::firmware::sync_builtin_rules(&pool)
            .await
            .map_err(anyhow::Error::msg)?;
        Some(pool)
    } else {
        tracing::info!("No user database yet (will be created on first write)");
        None
    };

    // Load settings from config TOML, migrating the legacy DB row if needed.
    let settings = match load_settings(&settings_path, user_pool.as_ref()).await {
        Ok(settings) => settings,
        Err(e) => {
            tracing::warn!(
                "Failed to load settings from {}: {}",
                settings_path.display(),
                e
            );
            AppSettings::default()
        }
    };

    // Find or decompress games database, then connect
    let games_db_pool = {
        let games_db_path =
            find_or_decompress_database(db::GAMES_DB_NAME, &app_data_dir, resource_dir.as_deref());

        match games_db_path {
            Some(path) => {
                tracing::info!("Found games database at: {}", path.display());
                let db_url = format!("sqlite:{}?mode=ro", path.display());
                match SqlitePoolOptions::new()
                    .max_connections(16)
                    .connect_with(SqliteConnectOptions::from_str(&db_url)?.read_only(true))
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Connected to games database (read-only)");
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
                tracing::info!("To enable, run: lunchbox-cli unified-build --download");
                None
            }
        }
    };

    // Find or decompress game_images database, then connect
    let images_db_pool = {
        let images_db_path =
            find_or_decompress_database(db::IMAGES_DB_NAME, &app_data_dir, resource_dir.as_deref());

        match images_db_path {
            Some(path) => {
                tracing::info!("Found images database at: {}", path.display());
                let db_url = format!("sqlite:{}?mode=ro", path.display());
                match SqlitePoolOptions::new()
                    .max_connections(16)
                    .connect_with(SqliteConnectOptions::from_str(&db_url)?.read_only(true))
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Connected to images database (read-only)");
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

    // Find or decompress emulators database, then connect
    let emulators_db_pool = {
        let emulators_db_path = find_or_decompress_database(
            db::EMULATORS_DB_NAME,
            &app_data_dir,
            resource_dir.as_deref(),
        );

        match emulators_db_path {
            Some(path) => {
                tracing::info!("Found emulators database at: {}", path.display());
                let db_url = format!("sqlite:{}?mode=ro", path.display());
                match SqlitePoolOptions::new()
                    .max_connections(16)
                    .connect_with(SqliteConnectOptions::from_str(&db_url)?.read_only(true))
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Connected to emulators database (read-only)");
                        Some(pool)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to connect to emulators database: {}", e);
                        None
                    }
                }
            }
            None => {
                tracing::info!("No emulators database found");
                None
            }
        }
    };

    // Find or decompress minerva database, then connect
    let minerva_db_pool = {
        let minerva_db_path = find_or_decompress_database(
            db::MINERVA_DB_NAME,
            &app_data_dir,
            resource_dir.as_deref(),
        );

        match minerva_db_path {
            Some(path) => {
                tracing::info!("Found minerva database at: {}", path.display());
                let db_url = format!("sqlite:{}?mode=ro", path.display());
                match SqlitePoolOptions::new()
                    .max_connections(16)
                    .connect_with(SqliteConnectOptions::from_str(&db_url)?.read_only(true))
                    .await
                {
                    Ok(pool) => {
                        tracing::info!("Connected to minerva database (read-only)");
                        Some(pool)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to connect to minerva database: {}", e);
                        None
                    }
                }
            }
            None => {
                tracing::info!("No minerva database found (ROM downloads will be disabled)");
                None
            }
        }
    };

    // Update state
    let mut state_guard = state.write().await;
    state_guard.db_pool = user_pool;
    state_guard.games_db_pool = games_db_pool;
    state_guard.images_db_pool = images_db_pool;
    state_guard.emulators_db_pool = emulators_db_pool;
    state_guard.minerva_db_pool = minerva_db_pool;
    state_guard.settings = settings;
    state_guard.user_db_path = Some(user_db_path);
    state_guard.settings_path = Some(settings_path);

    tracing::info!("App state initialized successfully");
    Ok(())
}

/// Ensure user database exists and is initialized
/// Call this before any write operation to the user database
pub async fn ensure_user_db(state: &mut AppState) -> Result<&SqlitePool> {
    if state.db_pool.is_some() {
        return Ok(state.db_pool.as_ref().unwrap());
    }

    let path = state
        .user_db_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("User database path not set"))?;

    tracing::info!("Creating user database at: {}", path.display());
    let pool = db::init_pool(path).await?;
    crate::firmware::sync_builtin_rules(&pool)
        .await
        .map_err(anyhow::Error::msg)?;
    state.db_pool = Some(pool);

    Ok(state.db_pool.as_ref().unwrap())
}

/// Load settings from the config TOML file and keyring.
///
/// If no TOML file exists yet, the old SQLite settings row is used once as a
/// migration source and then deleted after the TOML file is written.
pub async fn load_settings(
    settings_path: &Path,
    legacy_pool: Option<&SqlitePool>,
) -> Result<AppSettings> {
    let mut settings = if settings_path.exists() {
        load_settings_file(settings_path)?
    } else if let Some(pool) = legacy_pool {
        match load_legacy_settings_row(pool).await? {
            Some(settings) => {
                let mut migrated_settings = settings.clone();
                load_credentials_into_settings(&mut migrated_settings);
                save_settings(settings_path, &migrated_settings).await?;
                delete_legacy_settings_row(pool).await?;
                tracing::info!(
                    "Migrated settings from user database to {}",
                    settings_path.display()
                );
                migrated_settings
            }
            None => AppSettings::default(),
        }
    } else {
        AppSettings::default()
    };

    load_credentials_into_settings(&mut settings);
    Ok(settings)
}

fn load_settings_file(settings_path: &Path) -> Result<AppSettings> {
    let toml = std::fs::read_to_string(settings_path)?;
    Ok(toml::from_str(&toml)?)
}

async fn load_legacy_settings_row(pool: &SqlitePool) -> Result<Option<AppSettings>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'app_settings'")
            .fetch_optional(pool)
            .await?;

    if let Some((json,)) = row {
        Ok(Some(serde_json::from_str(&json)?))
    } else {
        Ok(None)
    }
}

async fn delete_legacy_settings_row(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM settings WHERE key = 'app_settings'")
        .execute(pool)
        .await?;
    Ok(())
}

fn load_credentials_into_settings(settings: &mut AppSettings) {
    let creds = crate::keyring_store::load_image_source_credentials();
    if !creds.steamgriddb_api_key.is_empty() {
        settings.steamgriddb.api_key = creds.steamgriddb_api_key;
    }
    if !creds.igdb_client_id.is_empty() {
        settings.igdb.client_id = creds.igdb_client_id;
    }
    if !creds.igdb_client_secret.is_empty() {
        settings.igdb.client_secret = creds.igdb_client_secret;
    }
    if !creds.emumovies_username.is_empty() {
        settings.emumovies.username = creds.emumovies_username;
    }
    if !creds.emumovies_password.is_empty() {
        settings.emumovies.password = creds.emumovies_password;
    }
    if !creds.screenscraper_dev_id.is_empty() {
        settings.screenscraper.dev_id = creds.screenscraper_dev_id;
    }
    if !creds.screenscraper_dev_password.is_empty() {
        settings.screenscraper.dev_password = creds.screenscraper_dev_password;
    }
    if creds.screenscraper_user_id.is_some() {
        settings.screenscraper.user_id = creds.screenscraper_user_id;
    }
    if creds.screenscraper_user_password.is_some() {
        settings.screenscraper.user_password = creds.screenscraper_user_password;
    }

    // Load qBittorrent password from keyring
    if let Ok(Some(v)) =
        crate::keyring_store::get_credential(crate::keyring_store::keys::QBITTORRENT_PASSWORD)
    {
        if !v.is_empty() {
            settings.torrent.qbittorrent_password = v;
        }
    }
}

/// Save settings to config TOML and credentials to keyring (if available).
pub async fn save_settings(settings_path: &Path, settings: &AppSettings) -> Result<()> {
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

    // Store qBittorrent password in keyring
    crate::keyring_store::store_credential(
        crate::keyring_store::keys::QBITTORRENT_PASSWORD,
        &settings.torrent.qbittorrent_password,
    )?;

    // If keyring is available, clear credentials from the file copy.
    // If not, store them in the file as fallback.
    let settings_for_file = if crate::keyring_store::is_keyring_available() {
        let mut s = settings.clone();

        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::STEAMGRIDDB_API_KEY,
            &settings.steamgriddb.api_key,
        ) {
            s.steamgriddb.api_key = String::new();
        }

        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::IGDB_CLIENT_ID,
            &settings.igdb.client_id,
        ) {
            s.igdb.client_id = String::new();
        }
        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::IGDB_CLIENT_SECRET,
            &settings.igdb.client_secret,
        ) {
            s.igdb.client_secret = String::new();
        }

        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::EMUMOVIES_USERNAME,
            &settings.emumovies.username,
        ) {
            s.emumovies.username = String::new();
        }
        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::EMUMOVIES_PASSWORD,
            &settings.emumovies.password,
        ) {
            s.emumovies.password = String::new();
        }

        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::SCREENSCRAPER_DEV_ID,
            &settings.screenscraper.dev_id,
        ) {
            s.screenscraper.dev_id = String::new();
        }
        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::SCREENSCRAPER_DEV_PASSWORD,
            &settings.screenscraper.dev_password,
        ) {
            s.screenscraper.dev_password = String::new();
        }
        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::SCREENSCRAPER_USER_ID,
            settings.screenscraper.user_id.as_deref().unwrap_or(""),
        ) {
            s.screenscraper.user_id = None;
        }
        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::SCREENSCRAPER_USER_PASSWORD,
            settings
                .screenscraper
                .user_password
                .as_deref()
                .unwrap_or(""),
        ) {
            s.screenscraper.user_password = None;
        }

        if crate::keyring_store::credential_matches(
            crate::keyring_store::keys::QBITTORRENT_PASSWORD,
            &settings.torrent.qbittorrent_password,
        ) {
            s.torrent.qbittorrent_password = String::new();
        }

        s
    } else {
        settings.clone()
    };

    write_settings_file(settings_path, &settings_for_file)?;

    Ok(())
}

/// Save settings when the credential fields did not change.
///
/// This keeps the credential fields exactly as they already exist in the TOML file
/// and avoids expensive keyring reads/writes for unrelated changes like
/// controller mapping updates.
pub async fn save_settings_preserving_credentials(
    settings_path: &Path,
    settings: &AppSettings,
) -> Result<()> {
    let mut settings_for_file = settings.clone();

    if settings_path.exists() {
        match load_settings_file(settings_path) {
            Ok(existing_file_settings) => {
                copy_credential_fields(&mut settings_for_file, &existing_file_settings);
            }
            Err(_) if crate::keyring_store::is_keyring_available() => {
                clear_credential_fields(&mut settings_for_file);
            }
            Err(e) => return Err(e),
        }
    } else if crate::keyring_store::is_keyring_available() {
        clear_credential_fields(&mut settings_for_file);
    }

    write_settings_file(settings_path, &settings_for_file)
}

fn write_settings_file(settings_path: &Path, settings: &AppSettings) -> Result<()> {
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let toml = toml::to_string_pretty(settings)?;
    std::fs::write(settings_path, toml)?;
    Ok(())
}

fn copy_credential_fields(target: &mut AppSettings, source: &AppSettings) {
    target.steamgriddb.api_key = source.steamgriddb.api_key.clone();
    target.igdb.client_id = source.igdb.client_id.clone();
    target.igdb.client_secret = source.igdb.client_secret.clone();
    target.emumovies.username = source.emumovies.username.clone();
    target.emumovies.password = source.emumovies.password.clone();
    target.screenscraper.dev_id = source.screenscraper.dev_id.clone();
    target.screenscraper.dev_password = source.screenscraper.dev_password.clone();
    target.screenscraper.user_id = source.screenscraper.user_id.clone();
    target.screenscraper.user_password = source.screenscraper.user_password.clone();
    target.torrent.qbittorrent_password = source.torrent.qbittorrent_password.clone();
}

fn clear_credential_fields(settings: &mut AppSettings) {
    settings.steamgriddb.api_key.clear();
    settings.igdb.client_id.clear();
    settings.igdb.client_secret.clear();
    settings.emumovies.username.clear();
    settings.emumovies.password.clear();
    settings.screenscraper.dev_id.clear();
    settings.screenscraper.dev_password.clear();
    settings.screenscraper.user_id = None;
    settings.screenscraper.user_password = None;
    settings.torrent.qbittorrent_password.clear();
}
