use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, params};

use crate::catalog;
use crate::download_plan::DownloadPlan;

const KEYRING_SERVICE: &str = "com.lunchbox.Lunchbox";
const KEYRING_ACCOUNT: &str = "qbittorrent-password";
static STORE_INITIALIZATION: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSettings {
    pub qbittorrent_host: String,
    pub qbittorrent_port: u16,
    pub qbittorrent_use_https: bool,
    pub qbittorrent_username: String,
    pub rom_directory: PathBuf,
    pub qbittorrent_container_rom_directory: String,
    pub torrent_library_directory: PathBuf,
    pub qbittorrent_container_torrent_library_directory: String,
    pub download_entire_torrent: bool,
    pub file_link_mode: String,
    pub seeding_policy: String,
    pub preferred_region: String,
    pub version_preference: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            qbittorrent_host: "127.0.0.1".to_owned(),
            qbittorrent_port: 8080,
            qbittorrent_use_https: false,
            qbittorrent_username: String::new(),
            rom_directory: PathBuf::new(),
            qbittorrent_container_rom_directory: String::new(),
            torrent_library_directory: PathBuf::new(),
            qbittorrent_container_torrent_library_directory: String::new(),
            download_entire_torrent: false,
            file_link_mode: "symlink".to_owned(),
            seeding_policy: "follow_client".to_owned(),
            preferred_region: "USA".to_owned(),
            version_preference: "latest".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryPreferences {
    pub hide_non_retail: bool,
    pub hide_adult: bool,
    pub artwork_type: String,
    pub grid_zoom: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserCollection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub game_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UserCollections {
    pub collections: Vec<UserCollection>,
    pub members: HashMap<String, HashSet<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmulatorPreference {
    pub emulator_id: String,
    pub runtime_kind: String,
    pub core_name: String,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ManagedEmulatorInstall {
    pub emulator_id: String,
    pub host_system_slug: String,
    pub manager: String,
    pub package_id: String,
    pub install_path: String,
    pub installed_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwarePackageReceipt {
    pub source_id: String,
    pub package_name: String,
    pub archive_path: String,
    pub extracted_root: String,
    pub sha256: String,
    pub file_count: i64,
    pub imported_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareFileReceipt {
    pub source_id: String,
    pub package_name: String,
    pub relative_path: String,
    pub store_path: String,
    pub sha256: String,
    pub file_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareInstallReceipt {
    pub rule_key: String,
    pub runtime_path: String,
    pub target_path: String,
    pub source_id: String,
    pub package_name: String,
    pub synced_at: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FirmwareDownloadReceipt {
    pub source_id: String,
    pub package_name: String,
    pub torrent_url: String,
    pub info_hash: String,
    pub file_index: u32,
    pub file_path: String,
    pub local_path: String,
    pub state: String,
    pub progress: f64,
    pub message: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayActivity {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub play_count: i64,
    pub total_play_time_seconds: i64,
    pub last_played_at: i64,
    pub first_played_at: i64,
    pub completion_state: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaySessionStart {
    pub session_id: String,
    pub started_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaPlaybackProgress {
    pub game_uid: String,
    pub media_key: String,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaTransfer {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub media_kind: String,
    pub provider: String,
    pub transport: String,
    pub source_url: String,
    pub remote_job_id: String,
    pub source_item_index: u32,
    pub source_item_path: String,
    pub local_download_path: PathBuf,
    pub local_target_path: PathBuf,
    pub state: String,
    pub progress: f64,
    pub download_speed: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for LibraryPreferences {
    fn default() -> Self {
        Self {
            hide_non_retail: true,
            hide_adult: false,
            artwork_type: "box-front".to_owned(),
            grid_zoom: 100,
        }
    }
}

impl LibraryPreferences {
    pub fn validate(&self) -> Result<()> {
        if crate::media::ArtworkKind::parse(&self.artwork_type).is_none() {
            bail!("unsupported artwork type {}", self.artwork_type);
        }
        if !(50..=200).contains(&self.grid_zoom) {
            bail!("grid zoom must be between 50 and 200 percent");
        }
        Ok(())
    }
}

impl AppSettings {
    pub fn validate(&self) -> Result<()> {
        if self.qbittorrent_host.trim().is_empty() {
            bail!("qBittorrent host is required");
        }
        if self.qbittorrent_port == 0 {
            bail!("qBittorrent port must be between 1 and 65535");
        }
        if !matches!(
            self.file_link_mode.as_str(),
            "symlink" | "hardlink" | "reflink" | "copy" | "leave_in_place"
        ) {
            bail!("unsupported file link mode {}", self.file_link_mode);
        }
        if !matches!(
            self.seeding_policy.as_str(),
            "follow_client" | "pause_after_import"
        ) {
            bail!("unsupported seeding policy {}", self.seeding_policy);
        }
        if !matches!(
            self.preferred_region.as_str(),
            "USA" | "Japan" | "Europe" | "World" | "Asia" | "any"
        ) {
            bail!("unsupported preferred region {}", self.preferred_region);
        }
        if !matches!(self.version_preference.as_str(), "latest" | "original") {
            bail!(
                "unsupported release version preference {}",
                self.version_preference
            );
        }
        Ok(())
    }

    pub fn qbittorrent_base_url(&self) -> String {
        let scheme = if self.qbittorrent_use_https {
            "https"
        } else {
            "http"
        };
        format!(
            "{scheme}://{}:{}",
            self.qbittorrent_host.trim(),
            self.qbittorrent_port
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadJob {
    pub id: String,
    pub game_id: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub torrent_url: String,
    pub torrent_file_index: Option<u32>,
    pub torrent_file_path: String,
    pub info_hash: String,
    pub client_save_path: String,
    pub local_download_path: PathBuf,
    pub local_target_path: PathBuf,
    pub state: String,
    pub progress: f64,
    pub download_speed: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
    pub post_import_action: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub download_plan: String,
}

pub(crate) struct NewDownloadJob {
    pub game_id: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub torrent_url: String,
    pub torrent_file_index: Option<u32>,
    pub torrent_file_path: String,
    pub info_hash: String,
    pub client_save_path: String,
    pub local_download_path: PathBuf,
    pub local_target_path: PathBuf,
    pub download_plan: String,
}

impl DownloadJob {
    pub(crate) fn queued(new_job: NewDownloadJob) -> Self {
        let now = unix_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            game_id: new_job.game_id,
            launchbox_db_id: new_job.launchbox_db_id,
            title: new_job.title,
            platform: new_job.platform,
            torrent_url: new_job.torrent_url,
            torrent_file_index: new_job.torrent_file_index,
            torrent_file_path: new_job.torrent_file_path,
            info_hash: new_job.info_hash,
            client_save_path: new_job.client_save_path,
            local_download_path: new_job.local_download_path,
            local_target_path: new_job.local_target_path,
            state: "queued".to_owned(),
            progress: 0.0,
            download_speed: 0,
            downloaded_bytes: 0,
            total_bytes: 0,
            message: "Queued in qBittorrent".to_owned(),
            post_import_action: "none".to_owned(),
            created_at: now,
            updated_at: now,
            download_plan: new_job.download_plan,
        }
    }

    pub fn parsed_download_plan(&self) -> Result<Option<DownloadPlan>> {
        if self.download_plan.trim().is_empty() {
            return Ok(None);
        }
        let plan: DownloadPlan = serde_json::from_str(&self.download_plan)
            .context("decoding persisted download plan")?;
        plan.validate()?;
        Ok(Some(plan))
    }
}

#[derive(Clone, Debug)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn open_default() -> Result<Self> {
        Self::at(state_database_path()?)
    }

    pub fn at(path: impl Into<PathBuf>) -> Result<Self> {
        let store = Self { path: path.into() };
        if let Some(parent) = store.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating state directory {}", parent.display()))?;
        }
        let _initialization = STORE_INITIALIZATION
            .lock()
            .map_err(|_| anyhow::anyhow!("Lunchbox state initialization lock was poisoned"))?;
        let connection = store.connection()?;
        let journal_mode =
            connection.query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            let enabled = connection
                .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))?;
            if !enabled.eq_ignore_ascii_case("wal") {
                bail!("could not enable WAL mode for the Lunchbox state database");
            }
        }
        migrate(&connection)?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<AppSettings> {
        let connection = self.connection()?;
        let settings = connection
            .query_row(
                "SELECT qbittorrent_host, qbittorrent_port, qbittorrent_use_https,
                        qbittorrent_username, rom_directory,
                        qbittorrent_container_rom_directory, torrent_library_directory,
                        qbittorrent_container_torrent_library_directory,
                        download_entire_torrent, file_link_mode, seeding_policy,
                        preferred_region, version_preference
                 FROM app_settings WHERE id=1",
                [],
                |row| {
                    let port = row.get::<_, i64>(1)?;
                    Ok(AppSettings {
                        qbittorrent_host: row.get(0)?,
                        qbittorrent_port: u16::try_from(port).unwrap_or_default(),
                        qbittorrent_use_https: row.get(2)?,
                        qbittorrent_username: row.get(3)?,
                        rom_directory: PathBuf::from(row.get::<_, String>(4)?),
                        qbittorrent_container_rom_directory: row.get(5)?,
                        torrent_library_directory: PathBuf::from(row.get::<_, String>(6)?),
                        qbittorrent_container_torrent_library_directory: row.get(7)?,
                        download_entire_torrent: row.get(8)?,
                        file_link_mode: row.get(9)?,
                        seeding_policy: row.get(10)?,
                        preferred_region: row.get(11)?,
                        version_preference: row.get(12)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default();
        settings.validate()?;
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<()> {
        settings.validate()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO app_settings (
                 id, qbittorrent_host, qbittorrent_port, qbittorrent_use_https,
                 qbittorrent_username, rom_directory,
                 qbittorrent_container_rom_directory, torrent_library_directory,
                 qbittorrent_container_torrent_library_directory,
                 download_entire_torrent, file_link_mode, seeding_policy,
                 preferred_region, version_preference
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                 qbittorrent_host=excluded.qbittorrent_host,
                 qbittorrent_port=excluded.qbittorrent_port,
                 qbittorrent_use_https=excluded.qbittorrent_use_https,
                 qbittorrent_username=excluded.qbittorrent_username,
                 rom_directory=excluded.rom_directory,
                 qbittorrent_container_rom_directory=excluded.qbittorrent_container_rom_directory,
                 torrent_library_directory=excluded.torrent_library_directory,
                 qbittorrent_container_torrent_library_directory=excluded.qbittorrent_container_torrent_library_directory,
                 download_entire_torrent=excluded.download_entire_torrent,
                 file_link_mode=excluded.file_link_mode,
                 seeding_policy=excluded.seeding_policy,
                 preferred_region=excluded.preferred_region,
                 version_preference=excluded.version_preference",
            params![
                settings.qbittorrent_host,
                i64::from(settings.qbittorrent_port),
                settings.qbittorrent_use_https,
                settings.qbittorrent_username,
                path_text(&settings.rom_directory),
                settings.qbittorrent_container_rom_directory,
                path_text(&settings.torrent_library_directory),
                settings.qbittorrent_container_torrent_library_directory,
                settings.download_entire_torrent,
                settings.file_link_mode,
                settings.seeding_policy,
                settings.preferred_region,
                settings.version_preference,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_library_preferences(&self) -> Result<LibraryPreferences> {
        let connection = self.connection()?;
        let preferences = connection
            .query_row(
                "SELECT hide_non_retail, hide_adult, artwork_type, grid_zoom
                 FROM library_preferences WHERE id=1",
                [],
                |row| {
                    Ok(LibraryPreferences {
                        hide_non_retail: row.get(0)?,
                        hide_adult: row.get(1)?,
                        artwork_type: row.get(2)?,
                        grid_zoom: row.get(3)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default();
        preferences.validate()?;
        Ok(preferences)
    }

    pub fn save_library_preferences(&self, preferences: &LibraryPreferences) -> Result<()> {
        preferences.validate()?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO library_preferences (
                 id, hide_non_retail, hide_adult, artwork_type, grid_zoom
             ) VALUES (1, ?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                 hide_non_retail=excluded.hide_non_retail,
                 hide_adult=excluded.hide_adult,
                 artwork_type=excluded.artwork_type,
                 grid_zoom=excluded.grid_zoom",
            params![
                preferences.hide_non_retail,
                preferences.hide_adult,
                preferences.artwork_type,
                preferences.grid_zoom,
            ],
        )?;
        Ok(())
    }

    pub fn favorite_game_ids(&self) -> Result<HashSet<String>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT game_uid FROM favorite_games")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
    }

    pub fn set_favorite(
        &self,
        game_uid: &str,
        launchbox_db_id: i64,
        title: &str,
        platform: &str,
        favorite: bool,
    ) -> Result<()> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to change favorites");
        }
        let connection = self.connection()?;
        if favorite {
            connection.execute(
                "INSERT INTO favorite_games (
                     game_uid, launchbox_db_id, title, platform, added_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(game_uid) DO UPDATE SET
                     launchbox_db_id=excluded.launchbox_db_id,
                     title=excluded.title,
                     platform=excluded.platform",
                params![game_uid, launchbox_db_id, title, platform, unix_timestamp()],
            )?;
        } else {
            connection.execute(
                "DELETE FROM favorite_games WHERE game_uid=?1",
                params![game_uid],
            )?;
        }
        Ok(())
    }

    pub fn play_activity(&self, game_uid: &str) -> Result<Option<PlayActivity>> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to load play activity");
        }
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT game_uid, launchbox_db_id, title, platform, play_count,
                        total_play_time_seconds, last_played_at, first_played_at,
                        completion_state
                 FROM game_activity WHERE game_uid=?1",
                [game_uid],
                play_activity_from_row,
            )
            .optional()?)
    }

    pub fn recent_play_activity(&self, limit: usize) -> Result<Vec<PlayActivity>> {
        let limit = i64::try_from(limit.min(500)).unwrap_or(500);
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT game_uid, launchbox_db_id, title, platform, play_count,
                    total_play_time_seconds, last_played_at, first_played_at,
                    completion_state
             FROM game_activity
             WHERE play_count > 0
             ORDER BY last_played_at DESC, game_uid
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit], play_activity_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn media_playback_progress(
        &self,
        game_uid: &str,
        media_key: &str,
    ) -> Result<Option<MediaPlaybackProgress>> {
        validate_media_playback_identity(game_uid, media_key)?;
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT game_uid, media_key, position_ms, duration_ms, updated_at
                 FROM media_playback_progress
                 WHERE game_uid=?1 AND media_key=?2",
                params![game_uid, media_key],
                |row| {
                    Ok(MediaPlaybackProgress {
                        game_uid: row.get(0)?,
                        media_key: row.get(1)?,
                        position_ms: row.get(2)?,
                        duration_ms: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn save_media_playback_progress(
        &self,
        game_uid: &str,
        media_key: &str,
        position_ms: i64,
        duration_ms: i64,
    ) -> Result<MediaPlaybackProgress> {
        validate_media_playback_identity(game_uid, media_key)?;
        if position_ms < 0 || duration_ms < 0 {
            bail!("media playback position and duration cannot be negative");
        }
        let position_ms = if duration_ms > 0 {
            position_ms.min(duration_ms)
        } else {
            position_ms
        };
        let updated_at = unix_timestamp();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO media_playback_progress (
                 game_uid, media_key, position_ms, duration_ms, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(game_uid, media_key) DO UPDATE SET
                 position_ms=excluded.position_ms,
                 duration_ms=excluded.duration_ms,
                 updated_at=excluded.updated_at",
            params![game_uid, media_key, position_ms, duration_ms, updated_at],
        )?;
        transaction.execute(
            "DELETE FROM media_playback_progress
             WHERE game_uid=?1 AND media_key<>?2",
            params![game_uid, media_key],
        )?;
        transaction.commit()?;
        Ok(MediaPlaybackProgress {
            game_uid: game_uid.to_owned(),
            media_key: media_key.to_owned(),
            position_ms,
            duration_ms,
            updated_at,
        })
    }

    pub fn media_transfer(
        &self,
        game_uid: &str,
        media_kind: &str,
    ) -> Result<Option<MediaTransfer>> {
        validate_media_transfer_identity(game_uid, media_kind)?;
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT game_uid, launchbox_db_id, title, platform, media_kind,
                        provider, transport, source_url, remote_job_id,
                        source_item_index, source_item_path,
                        local_download_path, local_target_path,
                        state, progress, download_speed, downloaded_bytes,
                        total_bytes, message, created_at, updated_at
                 FROM media_transfers WHERE game_uid=?1 AND media_kind=?2",
                params![game_uid, media_kind],
                media_transfer_from_row,
            )
            .optional()?)
    }

    pub fn media_transfers_for_remote_job(
        &self,
        provider: &str,
        remote_job_id: &str,
    ) -> Result<Vec<MediaTransfer>> {
        if provider.trim().is_empty()
            || provider.chars().count() > 128
            || remote_job_id.trim().is_empty()
            || remote_job_id.chars().count() > 512
        {
            bail!("media transfer requires bounded provider and remote job identities");
        }
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT game_uid, launchbox_db_id, title, platform, media_kind,
                    provider, transport, source_url, remote_job_id,
                    source_item_index, source_item_path,
                    local_download_path, local_target_path,
                    state, progress, download_speed, downloaded_bytes,
                    total_bytes, message, created_at, updated_at
             FROM media_transfers
             WHERE provider=?1 AND lower(remote_job_id)=lower(?2)
             ORDER BY created_at, game_uid, media_kind",
        )?;
        let rows =
            statement.query_map(params![provider, remote_job_id], media_transfer_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn upsert_media_transfer(&self, transfer: &MediaTransfer) -> Result<()> {
        validate_media_transfer(transfer)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO media_transfers (
                 game_uid, launchbox_db_id, title, platform, media_kind,
                 provider, transport, source_url, remote_job_id,
                 source_item_index, source_item_path,
                 local_download_path, local_target_path,
                 state, progress, download_speed, downloaded_bytes,
                 total_bytes, message, created_at, updated_at
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                 ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21
             )
             ON CONFLICT(game_uid, media_kind) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 title=excluded.title,
                 platform=excluded.platform,
                 provider=excluded.provider,
                 transport=excluded.transport,
                 source_url=excluded.source_url,
                 remote_job_id=excluded.remote_job_id,
                 source_item_index=excluded.source_item_index,
                 source_item_path=excluded.source_item_path,
                 local_download_path=excluded.local_download_path,
                 local_target_path=excluded.local_target_path,
                 state=excluded.state,
                 progress=excluded.progress,
                 download_speed=excluded.download_speed,
                 downloaded_bytes=excluded.downloaded_bytes,
                 total_bytes=excluded.total_bytes,
                 message=excluded.message,
                 updated_at=excluded.updated_at",
            params![
                transfer.game_uid,
                transfer.launchbox_db_id,
                transfer.title,
                transfer.platform,
                transfer.media_kind,
                transfer.provider,
                transfer.transport,
                transfer.source_url,
                transfer.remote_job_id,
                i64::from(transfer.source_item_index),
                transfer.source_item_path,
                path_text(&transfer.local_download_path),
                path_text(&transfer.local_target_path),
                transfer.state,
                transfer.progress,
                i64::try_from(transfer.download_speed).unwrap_or(i64::MAX),
                i64::try_from(transfer.downloaded_bytes).unwrap_or(i64::MAX),
                i64::try_from(transfer.total_bytes).unwrap_or(i64::MAX),
                transfer.message,
                transfer.created_at,
                transfer.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn begin_play_session(
        &self,
        game_uid: &str,
        launchbox_db_id: i64,
        title: &str,
        platform: &str,
        emulator: &str,
    ) -> Result<PlaySessionStart> {
        validate_play_identity(game_uid, title, platform)?;
        if emulator.trim().is_empty() {
            bail!("a play session requires an emulator identity");
        }
        let now = unix_timestamp();
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO game_activity (
                 game_uid, launchbox_db_id, title, platform, play_count,
                 total_play_time_seconds, last_played_at, first_played_at,
                 completion_state
             ) VALUES (?1, ?2, ?3, ?4, 1, 0, ?5, ?5, 'in_progress')
             ON CONFLICT(game_uid) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 title=excluded.title,
                 platform=excluded.platform,
                 play_count=game_activity.play_count + 1,
                 last_played_at=excluded.last_played_at,
                 first_played_at=CASE
                     WHEN game_activity.play_count=0 OR game_activity.first_played_at=0
                         THEN excluded.first_played_at
                     ELSE game_activity.first_played_at
                 END,
                 completion_state=CASE
                     WHEN game_activity.completion_state='not_started' THEN 'in_progress'
                     ELSE game_activity.completion_state
                 END",
            params![game_uid, launchbox_db_id, title, platform, now],
        )?;
        transaction.execute(
            "INSERT INTO play_sessions (
                 id, game_uid, launchbox_db_id, title, platform, emulator,
                 started_at, ended_at, duration_seconds, outcome
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, 0, 'running')",
            params![
                session_id,
                game_uid,
                launchbox_db_id,
                title,
                platform,
                emulator,
                now
            ],
        )?;
        transaction.commit()?;
        Ok(PlaySessionStart {
            session_id,
            started_at: now,
        })
    }

    pub fn finish_play_session(
        &self,
        session_id: &str,
        duration_seconds: u64,
        outcome: &str,
    ) -> Result<bool> {
        if session_id.trim().is_empty() {
            bail!("a play session identity is required");
        }
        if !matches!(outcome, "completed" | "failed" | "terminated") {
            bail!("unsupported play session outcome {outcome}");
        }
        let duration = i64::try_from(duration_seconds).unwrap_or(i64::MAX);
        let ended_at = unix_timestamp();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let game_uid = transaction
            .query_row(
                "SELECT game_uid FROM play_sessions WHERE id=?1 AND outcome='running'",
                [session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(game_uid) = game_uid else {
            transaction.rollback()?;
            return Ok(false);
        };
        let changed = transaction.execute(
            "UPDATE play_sessions
             SET ended_at=?2, duration_seconds=?3, outcome=?4
             WHERE id=?1 AND outcome='running'",
            params![session_id, ended_at, duration, outcome],
        )?;
        if changed == 1 {
            transaction.execute(
                "UPDATE game_activity
                 SET total_play_time_seconds=total_play_time_seconds + ?2
                 WHERE game_uid=?1",
                params![game_uid, duration],
            )?;
        }
        transaction.commit()?;
        Ok(changed == 1)
    }

    pub fn set_completion_state(
        &self,
        game_uid: &str,
        launchbox_db_id: i64,
        title: &str,
        platform: &str,
        completion_state: &str,
    ) -> Result<()> {
        validate_play_identity(game_uid, title, platform)?;
        validate_completion_state(completion_state)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO game_activity (
                 game_uid, launchbox_db_id, title, platform, play_count,
                 total_play_time_seconds, last_played_at, first_played_at,
                 completion_state
             ) VALUES (?1, ?2, ?3, ?4, 0, 0, 0, 0, ?5)
             ON CONFLICT(game_uid) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 title=excluded.title,
                 platform=excluded.platform,
                 completion_state=excluded.completion_state",
            params![game_uid, launchbox_db_id, title, platform, completion_state],
        )?;
        Ok(())
    }

    pub fn emulator_preference(
        &self,
        game_uid: &str,
        platform: &str,
    ) -> Result<Option<EmulatorPreference>> {
        let connection = self.connection()?;
        if !game_uid.trim().is_empty()
            && let Some(preference) = connection
                .query_row(
                    "SELECT emulator_id, runtime_kind, core_name
                     FROM game_emulator_preferences WHERE game_uid=?1",
                    [game_uid],
                    |row| {
                        Ok(EmulatorPreference {
                            emulator_id: row.get(0)?,
                            runtime_kind: row.get(1)?,
                            core_name: row.get(2)?,
                            scope: "game".to_owned(),
                        })
                    },
                )
                .optional()?
        {
            return Ok(Some(preference));
        }

        let platform_key = catalog::normalize_platform_key(platform);
        if platform_key.is_empty() {
            return Ok(None);
        }
        Ok(connection
            .query_row(
                "SELECT emulator_id, runtime_kind, core_name
                 FROM platform_emulator_preferences WHERE platform_key=?1",
                [platform_key],
                |row| {
                    Ok(EmulatorPreference {
                        emulator_id: row.get(0)?,
                        runtime_kind: row.get(1)?,
                        core_name: row.get(2)?,
                        scope: "platform".to_owned(),
                    })
                },
            )
            .optional()?)
    }

    pub fn managed_emulator_installs(&self) -> Result<Vec<ManagedEmulatorInstall>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT emulator_id, host_system_slug, manager, package_id, install_path,
                    installed_at, updated_at
             FROM managed_emulator_installs
             ORDER BY manager, package_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ManagedEmulatorInstall {
                emulator_id: row.get(0)?,
                host_system_slug: row.get(1)?,
                manager: row.get(2)?,
                package_id: row.get(3)?,
                install_path: row.get(4)?,
                installed_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn firmware_packages(&self) -> Result<Vec<FirmwarePackageReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT source_id, package_name, archive_path, extracted_root,
                    sha256, file_count, imported_at
             FROM firmware_packages
             ORDER BY source_id, package_name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(FirmwarePackageReceipt {
                source_id: row.get(0)?,
                package_name: row.get(1)?,
                archive_path: row.get(2)?,
                extracted_root: row.get(3)?,
                sha256: row.get(4)?,
                file_count: row.get(5)?,
                imported_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn firmware_files(
        &self,
        source_id: &str,
        package_name: &str,
    ) -> Result<Vec<FirmwareFileReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT source_id, package_name, relative_path, store_path,
                    sha256, file_size
             FROM firmware_files
             WHERE source_id=?1 AND package_name=?2
             ORDER BY relative_path",
        )?;
        let rows = statement.query_map(params![source_id, package_name], |row| {
            Ok(FirmwareFileReceipt {
                source_id: row.get(0)?,
                package_name: row.get(1)?,
                relative_path: row.get(2)?,
                store_path: row.get(3)?,
                sha256: row.get(4)?,
                file_size: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn firmware_installs(&self) -> Result<Vec<FirmwareInstallReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT rule_key, runtime_path, target_path, source_id,
                    package_name, synced_at
             FROM firmware_installs
             ORDER BY rule_key, runtime_path",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(FirmwareInstallReceipt {
                rule_key: row.get(0)?,
                runtime_path: row.get(1)?,
                target_path: row.get(2)?,
                source_id: row.get(3)?,
                package_name: row.get(4)?,
                synced_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn record_firmware_package(
        &self,
        receipt: &FirmwarePackageReceipt,
        files: &[FirmwareFileReceipt],
    ) -> Result<()> {
        validate_firmware_package_receipt(receipt)?;
        if usize::try_from(receipt.file_count).ok() != Some(files.len()) {
            bail!("firmware package receipt does not match its file manifest");
        }
        for file in files {
            validate_firmware_file_receipt(receipt, file)?;
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO firmware_packages (
                 source_id, package_name, archive_path, extracted_root,
                 sha256, file_count, imported_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(source_id, package_name) DO UPDATE SET
                 archive_path=excluded.archive_path,
                 extracted_root=excluded.extracted_root,
                 sha256=excluded.sha256,
                 file_count=excluded.file_count,
                 imported_at=excluded.imported_at",
            params![
                receipt.source_id,
                receipt.package_name,
                receipt.archive_path,
                receipt.extracted_root,
                receipt.sha256,
                receipt.file_count,
                receipt.imported_at,
            ],
        )?;
        transaction.execute(
            "DELETE FROM firmware_files WHERE source_id=?1 AND package_name=?2",
            params![receipt.source_id, receipt.package_name],
        )?;
        {
            let mut insert = transaction.prepare(
                "INSERT INTO firmware_files (
                     source_id, package_name, relative_path, store_path,
                     sha256, file_size
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for file in files {
                insert.execute(params![
                    file.source_id,
                    file.package_name,
                    file.relative_path,
                    file.store_path,
                    file.sha256,
                    file.file_size,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn record_firmware_install(&self, receipt: &FirmwareInstallReceipt) -> Result<()> {
        validate_firmware_install_receipt(receipt)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO firmware_installs (
                 rule_key, runtime_path, target_path, source_id,
                 package_name, synced_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(rule_key, runtime_path) DO UPDATE SET
                 target_path=excluded.target_path,
                 source_id=excluded.source_id,
                 package_name=excluded.package_name,
                 synced_at=excluded.synced_at",
            params![
                receipt.rule_key,
                receipt.runtime_path,
                receipt.target_path,
                receipt.source_id,
                receipt.package_name,
                receipt.synced_at,
            ],
        )?;
        Ok(())
    }

    pub fn firmware_downloads_for_info_hash(
        &self,
        info_hash: &str,
    ) -> Result<Vec<FirmwareDownloadReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT source_id, package_name, torrent_url, info_hash,
                    file_index, file_path, local_path, state, progress,
                    message, updated_at
             FROM firmware_downloads
             WHERE info_hash=?1
             ORDER BY source_id, package_name",
        )?;
        let rows = statement.query_map([info_hash], firmware_download_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn record_firmware_download(&self, receipt: &FirmwareDownloadReceipt) -> Result<()> {
        validate_firmware_download_receipt(receipt)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO firmware_downloads (
                 source_id, package_name, torrent_url, info_hash, file_index,
                 file_path, local_path, state, progress, message, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(source_id, package_name) DO UPDATE SET
                 torrent_url=excluded.torrent_url,
                 info_hash=excluded.info_hash,
                 file_index=excluded.file_index,
                 file_path=excluded.file_path,
                 local_path=excluded.local_path,
                 state=excluded.state,
                 progress=excluded.progress,
                 message=excluded.message,
                 updated_at=excluded.updated_at",
            params![
                receipt.source_id,
                receipt.package_name,
                receipt.torrent_url,
                receipt.info_hash,
                i64::from(receipt.file_index),
                receipt.file_path,
                receipt.local_path,
                receipt.state,
                receipt.progress,
                receipt.message,
                receipt.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn record_managed_emulator_install(
        &self,
        emulator_id: &str,
        host_system_slug: &str,
        manager: &str,
        package_id: &str,
        install_path: &str,
    ) -> Result<()> {
        validate_managed_emulator_install(emulator_id, host_system_slug, manager, package_id)?;
        let now = unix_timestamp();
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO managed_emulator_installs (
                 emulator_id, host_system_slug, manager, package_id, install_path,
                 installed_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(host_system_slug, manager, package_id) DO UPDATE SET
                 emulator_id=excluded.emulator_id,
                 install_path=excluded.install_path,
                 updated_at=excluded.updated_at",
            params![
                emulator_id,
                host_system_slug,
                manager,
                package_id,
                install_path,
                now
            ],
        )?;
        Ok(())
    }

    pub fn remove_managed_emulator_install(
        &self,
        host_system_slug: &str,
        manager: &str,
        package_id: &str,
    ) -> Result<()> {
        let connection = self.connection()?;
        connection.execute(
            "DELETE FROM managed_emulator_installs
             WHERE host_system_slug=?1 AND manager=?2 AND package_id=?3",
            params![host_system_slug, manager, package_id],
        )?;
        Ok(())
    }

    pub fn set_game_emulator_preference(
        &self,
        game_uid: &str,
        emulator_id: &str,
        runtime_kind: &str,
        core_name: &str,
    ) -> Result<()> {
        validate_emulator_preference(game_uid, emulator_id, runtime_kind, core_name)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO game_emulator_preferences (
                 game_uid, emulator_id, runtime_kind, core_name, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(game_uid) DO UPDATE SET
                 emulator_id=excluded.emulator_id,
                 runtime_kind=excluded.runtime_kind,
                 core_name=excluded.core_name,
                 updated_at=excluded.updated_at",
            params![
                game_uid,
                emulator_id,
                runtime_kind,
                core_name,
                unix_timestamp()
            ],
        )?;
        Ok(())
    }

    pub fn clear_game_emulator_preference(&self, game_uid: &str) -> Result<()> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to clear an emulator preference");
        }
        self.connection()?.execute(
            "DELETE FROM game_emulator_preferences WHERE game_uid=?1",
            [game_uid],
        )?;
        Ok(())
    }

    pub fn set_platform_emulator_preference(
        &self,
        platform: &str,
        emulator_id: &str,
        runtime_kind: &str,
        core_name: &str,
    ) -> Result<()> {
        let platform_key = catalog::normalize_platform_key(platform);
        validate_emulator_preference(&platform_key, emulator_id, runtime_kind, core_name)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO platform_emulator_preferences (
                 platform_key, platform_display, emulator_id,
                 runtime_kind, core_name, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(platform_key) DO UPDATE SET
                 platform_display=excluded.platform_display,
                 emulator_id=excluded.emulator_id,
                 runtime_kind=excluded.runtime_kind,
                 core_name=excluded.core_name,
                 updated_at=excluded.updated_at",
            params![
                platform_key,
                platform.trim(),
                emulator_id,
                runtime_kind,
                core_name,
                unix_timestamp()
            ],
        )?;
        Ok(())
    }

    pub fn clear_platform_emulator_preference(&self, platform: &str) -> Result<()> {
        let platform_key = catalog::normalize_platform_key(platform);
        if platform_key.is_empty() {
            bail!("a platform is required to clear an emulator preference");
        }
        self.connection()?.execute(
            "DELETE FROM platform_emulator_preferences WHERE platform_key=?1",
            [platform_key],
        )?;
        Ok(())
    }

    pub fn user_collections(&self) -> Result<UserCollections> {
        let connection = self.connection()?;
        let mut collection_statement = connection.prepare(
            "SELECT id, name, description
             FROM user_collections
             ORDER BY name COLLATE NOCASE, id",
        )?;
        let collection_rows = collection_statement.query_map([], |row| {
            Ok(UserCollection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                game_count: 0,
            })
        })?;
        let mut collections = collection_rows.collect::<rusqlite::Result<Vec<_>>>()?;

        let mut member_statement = connection.prepare(
            "SELECT collection_id, game_uid
             FROM user_collection_games
             ORDER BY collection_id, sort_order, game_uid",
        )?;
        let member_rows = member_statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut members = HashMap::<String, HashSet<String>>::new();
        for row in member_rows {
            let (collection_id, game_uid) = row?;
            members.entry(collection_id).or_default().insert(game_uid);
        }
        for collection in &mut collections {
            collection.game_count = members
                .get(&collection.id)
                .map(HashSet::len)
                .unwrap_or_default();
        }
        Ok(UserCollections {
            collections,
            members,
        })
    }

    pub fn create_collection(&self, name: &str, description: &str) -> Result<UserCollection> {
        let name = validate_collection_name(name)?;
        let description = validate_collection_description(description)?;
        let collection = UserCollection {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            game_count: 0,
        };
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO user_collections (
                     id, name, description, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![
                    collection.id,
                    collection.name,
                    collection.description,
                    unix_timestamp()
                ],
            )
            .with_context(|| format!("creating collection {}", collection.name))?;
        Ok(collection)
    }

    pub fn update_collection(&self, id: &str, name: &str, description: &str) -> Result<()> {
        let name = validate_collection_name(name)?;
        let description = validate_collection_description(description)?;
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "UPDATE user_collections
                 SET name=?2, description=?3, updated_at=?4
                 WHERE id=?1",
                params![id, name, description, unix_timestamp()],
            )
            .with_context(|| format!("updating collection {id}"))?;
        if changed == 0 {
            bail!("collection no longer exists");
        }
        Ok(())
    }

    pub fn delete_collection(&self, id: &str) -> Result<()> {
        let connection = self.connection()?;
        let changed = connection
            .execute("DELETE FROM user_collections WHERE id=?1", params![id])
            .with_context(|| format!("deleting collection {id}"))?;
        if changed == 0 {
            bail!("collection no longer exists");
        }
        Ok(())
    }

    pub fn set_collection_membership(
        &self,
        collection_id: &str,
        game_uid: &str,
        member: bool,
    ) -> Result<()> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to change collection membership");
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        if member {
            let next_order = transaction.query_row(
                "SELECT coalesce(max(sort_order), 0) + 1
                 FROM user_collection_games WHERE collection_id=?1",
                params![collection_id],
                |row| row.get::<_, i64>(0),
            )?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO user_collection_games (
                         collection_id, game_uid, sort_order, added_at
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![collection_id, game_uid, next_order, unix_timestamp()],
                )
                .with_context(|| format!("adding a game to collection {collection_id}"))?;
        } else {
            transaction.execute(
                "DELETE FROM user_collection_games
                 WHERE collection_id=?1 AND game_uid=?2",
                params![collection_id, game_uid],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn upsert_job(&self, job: &DownloadJob) -> Result<()> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO download_jobs (
                 id, game_id, launchbox_db_id, title, platform, torrent_url, torrent_file_index,
                 torrent_file_path, info_hash, client_save_path,
                 local_download_path, local_target_path, state, progress,
                 download_speed, downloaded_bytes, total_bytes, message,
                 post_import_action, created_at, updated_at, download_plan
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                 ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
             ) ON CONFLICT(id) DO UPDATE SET
                 state=excluded.state, progress=excluded.progress,
                 download_speed=excluded.download_speed,
                 downloaded_bytes=excluded.downloaded_bytes,
                 total_bytes=excluded.total_bytes, message=excluded.message,
                 post_import_action=excluded.post_import_action,
                 updated_at=excluded.updated_at",
            params![
                job.id,
                job.game_id,
                job.launchbox_db_id,
                job.title,
                job.platform,
                job.torrent_url,
                job.torrent_file_index.map(i64::from),
                job.torrent_file_path,
                job.info_hash,
                job.client_save_path,
                path_text(&job.local_download_path),
                path_text(&job.local_target_path),
                job.state,
                job.progress,
                u64_to_i64(job.download_speed),
                u64_to_i64(job.downloaded_bytes),
                u64_to_i64(job.total_bytes),
                job.message,
                job.post_import_action,
                job.created_at,
                job.updated_at,
                job.download_plan,
            ],
        )?;
        Ok(())
    }

    pub fn jobs(&self) -> Result<Vec<DownloadJob>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, game_id, launchbox_db_id, title, platform, torrent_url,
                    torrent_file_index, torrent_file_path, info_hash,
                    client_save_path, local_download_path, local_target_path,
                    state, progress, download_speed, downloaded_bytes,
                    total_bytes, message, post_import_action, created_at, updated_at,
                    download_plan
             FROM download_jobs ORDER BY created_at DESC, id",
        )?;
        let rows = statement.query_map([], |row| {
            let file_index = row.get::<_, Option<i64>>(6)?;
            Ok(DownloadJob {
                id: row.get(0)?,
                game_id: row.get(1)?,
                launchbox_db_id: row.get(2)?,
                title: row.get(3)?,
                platform: row.get(4)?,
                torrent_url: row.get(5)?,
                torrent_file_index: file_index.and_then(|value| u32::try_from(value).ok()),
                torrent_file_path: row.get(7)?,
                info_hash: row.get(8)?,
                client_save_path: row.get(9)?,
                local_download_path: PathBuf::from(row.get::<_, String>(10)?),
                local_target_path: PathBuf::from(row.get::<_, String>(11)?),
                state: row.get(12)?,
                progress: row.get(13)?,
                download_speed: i64_to_u64(row.get(14)?),
                downloaded_bytes: i64_to_u64(row.get(15)?),
                total_bytes: i64_to_u64(row.get(16)?),
                message: row.get(17)?,
                post_import_action: row.get(18)?,
                created_at: row.get(19)?,
                updated_at: row.get(20)?,
                download_plan: row.get(21)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn jobs_for_info_hash(&self, info_hash: &str) -> Result<Vec<DownloadJob>> {
        Ok(self
            .jobs()?
            .into_iter()
            .filter(|job| job.info_hash.eq_ignore_ascii_case(info_hash))
            .collect())
    }

    pub fn delete_job_record(&self, id: &str) -> Result<bool> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "DELETE FROM download_jobs
             WHERE id=?1
               AND state NOT IN ('queued', 'downloading', 'paused', 'complete')
               AND post_import_action != 'pause_pending'",
            params![id],
        )?;
        Ok(changed > 0)
    }

    pub fn clear_finished_job_records(&self) -> Result<usize> {
        let connection = self.connection()?;
        Ok(connection.execute(
            "DELETE FROM download_jobs
             WHERE state NOT IN ('queued', 'downloading', 'paused', 'complete')
               AND post_import_action != 'pause_pending'",
            [],
        )?)
    }

    pub fn record_installed(&self, job: &DownloadJob, path: &Path) -> Result<()> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO installed_games (
                 game_uid, launchbox_db_id, title, platform, file_path,
                 file_size, import_source, installed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'minerva', ?7)
             ON CONFLICT(game_uid) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 title=excluded.title, platform=excluded.platform,
                 file_path=excluded.file_path, file_size=excluded.file_size,
                 import_source=excluded.import_source,
                 installed_at=excluded.installed_at",
            params![
                job.game_id,
                job.launchbox_db_id,
                job.title,
                job.platform,
                path_text(path),
                path.metadata()
                    .ok()
                    .and_then(|metadata| i64::try_from(metadata.len()).ok()),
                unix_timestamp(),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn connection(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)
            .with_context(|| format!("opening Lunchbox state {}", self.path.display()))?;
        connection.busy_timeout(std::time::Duration::from_secs(2))?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA synchronous=NORMAL;
             PRAGMA trusted_schema=OFF;",
        )?;
        Ok(connection)
    }
}

pub fn state_database_path() -> Result<PathBuf> {
    if let Some(path) = catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Ok(path);
    }
    ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("state.db"))
        .context("could not determine the operating system application data directory")
}

pub fn load_password() -> Result<Option<String>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .context("opening the operating system credential store")?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error).context("reading qBittorrent password from credential store"),
    }
}

pub fn save_password(password: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .context("opening the operating system credential store")?;
    if password.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error).context("removing qBittorrent password"),
        }
    } else {
        entry
            .set_password(password)
            .context("saving qBittorrent password in credential store")
    }
}

fn migrate(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
             id INTEGER PRIMARY KEY CHECK (id=1),
             qbittorrent_host TEXT NOT NULL,
             qbittorrent_port INTEGER NOT NULL CHECK (qbittorrent_port BETWEEN 1 AND 65535),
             qbittorrent_use_https INTEGER NOT NULL CHECK (qbittorrent_use_https IN (0, 1)),
             qbittorrent_username TEXT NOT NULL,
             rom_directory TEXT NOT NULL,
             qbittorrent_container_rom_directory TEXT NOT NULL,
             torrent_library_directory TEXT NOT NULL,
             qbittorrent_container_torrent_library_directory TEXT NOT NULL,
             download_entire_torrent INTEGER NOT NULL CHECK (download_entire_torrent IN (0, 1)),
             file_link_mode TEXT NOT NULL CHECK (
                 file_link_mode IN ('symlink', 'hardlink', 'reflink', 'copy', 'leave_in_place')
             ),
             seeding_policy TEXT NOT NULL DEFAULT 'follow_client' CHECK (
                 seeding_policy IN ('follow_client', 'pause_after_import')
             ),
             preferred_region TEXT NOT NULL DEFAULT 'USA' CHECK (
                 preferred_region IN ('USA', 'Japan', 'Europe', 'World', 'Asia', 'any')
             ),
             version_preference TEXT NOT NULL DEFAULT 'latest' CHECK (
                 version_preference IN ('latest', 'original')
             )
         );
         CREATE TABLE IF NOT EXISTS library_preferences (
             id INTEGER PRIMARY KEY CHECK (id=1),
             hide_non_retail INTEGER NOT NULL CHECK (hide_non_retail IN (0, 1)),
             hide_adult INTEGER NOT NULL CHECK (hide_adult IN (0, 1)),
             artwork_type TEXT NOT NULL DEFAULT 'box-front' CHECK (
                 artwork_type IN (
                     'box-front', 'box-back', 'box-3d', 'screenshot',
                     'title-screen', 'fanart', 'clear-logo'
                 )
             ),
             grid_zoom INTEGER NOT NULL DEFAULT 100 CHECK (grid_zoom BETWEEN 50 AND 200)
         );
         CREATE TABLE IF NOT EXISTS download_jobs (
             id TEXT PRIMARY KEY,
             game_id TEXT NOT NULL,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             torrent_url TEXT NOT NULL,
             torrent_file_index INTEGER,
             torrent_file_path TEXT NOT NULL,
             info_hash TEXT NOT NULL,
             client_save_path TEXT NOT NULL,
             local_download_path TEXT NOT NULL,
             local_target_path TEXT NOT NULL,
             state TEXT NOT NULL,
             progress REAL NOT NULL CHECK (progress BETWEEN 0.0 AND 1.0),
             download_speed INTEGER NOT NULL,
             downloaded_bytes INTEGER NOT NULL,
             total_bytes INTEGER NOT NULL,
             message TEXT NOT NULL,
             post_import_action TEXT NOT NULL DEFAULT 'none' CHECK (
                 post_import_action IN ('none', 'pause_pending', 'pause_applied')
             ),
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL,
             download_plan TEXT NOT NULL DEFAULT ''
         );
         CREATE INDEX IF NOT EXISTS download_jobs_info_hash
             ON download_jobs(info_hash);
         CREATE TABLE IF NOT EXISTS installed_games (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             file_path TEXT NOT NULL,
             file_size INTEGER,
             import_source TEXT NOT NULL,
             installed_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS installed_games_launchbox_id
             ON installed_games(launchbox_db_id) WHERE launchbox_db_id > 0;
         CREATE TABLE IF NOT EXISTS local_collection_roots (
             id TEXT PRIMARY KEY,
             path_display TEXT NOT NULL,
             path_bytes BLOB NOT NULL,
             path_encoding TEXT NOT NULL CHECK (
                 path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
             ),
             platform_hint TEXT NOT NULL,
             checksums_enabled INTEGER NOT NULL CHECK (checksums_enabled IN (0, 1)),
             last_scanned_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS local_rom_files (
             id TEXT PRIMARY KEY,
             root_id TEXT NOT NULL REFERENCES local_collection_roots(id) ON DELETE CASCADE,
             path_display TEXT NOT NULL,
             path_bytes BLOB NOT NULL,
             path_encoding TEXT NOT NULL CHECK (
                 path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
             ),
             relative_path_display TEXT NOT NULL,
             relative_path_bytes BLOB NOT NULL,
             relative_path_encoding TEXT NOT NULL CHECK (
                 relative_path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
             ),
             file_name TEXT NOT NULL,
             display_title TEXT NOT NULL,
             platform TEXT NOT NULL,
             file_size INTEGER NOT NULL CHECK (file_size >= 0),
             modified_unix_ns INTEGER,
             crc32 TEXT NOT NULL,
             md5 TEXT NOT NULL,
             sha1 TEXT NOT NULL,
             game_uid TEXT,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             matched_title TEXT,
             match_state TEXT NOT NULL CHECK (
                 match_state IN ('exact', 'ambiguous', 'unmatched', 'inventory_only', 'error')
             ),
             match_method TEXT NOT NULL,
             included INTEGER NOT NULL CHECK (included IN (0, 1)),
             availability TEXT NOT NULL CHECK (availability IN ('present', 'missing')),
             imported_at INTEGER NOT NULL
         );
         CREATE UNIQUE INDEX IF NOT EXISTS local_rom_files_root_relative
             ON local_rom_files(root_id, relative_path_bytes);
         CREATE INDEX IF NOT EXISTS local_rom_files_game_uid
             ON local_rom_files(game_uid) WHERE game_uid IS NOT NULL;
         CREATE INDEX IF NOT EXISTS local_rom_files_visible
             ON local_rom_files(included, availability);
         CREATE TABLE IF NOT EXISTS favorite_games (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             added_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS favorite_games_launchbox_id
             ON favorite_games(launchbox_db_id) WHERE launchbox_db_id > 0;
         CREATE INDEX IF NOT EXISTS favorite_games_added_at
             ON favorite_games(added_at DESC);
         CREATE TABLE IF NOT EXISTS game_activity (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             play_count INTEGER NOT NULL DEFAULT 0 CHECK (play_count >= 0),
             total_play_time_seconds INTEGER NOT NULL DEFAULT 0 CHECK (
                 total_play_time_seconds >= 0
             ),
             last_played_at INTEGER NOT NULL DEFAULT 0 CHECK (last_played_at >= 0),
             first_played_at INTEGER NOT NULL DEFAULT 0 CHECK (first_played_at >= 0),
             completion_state TEXT NOT NULL DEFAULT 'not_started' CHECK (
                 completion_state IN (
                     'not_started', 'in_progress', 'completed', 'on_hold', 'abandoned'
                 )
             )
         );
         CREATE INDEX IF NOT EXISTS game_activity_recent
             ON game_activity(last_played_at DESC, game_uid) WHERE play_count > 0;
         CREATE INDEX IF NOT EXISTS game_activity_completion
             ON game_activity(completion_state, game_uid);
         CREATE TABLE IF NOT EXISTS play_sessions (
             id TEXT PRIMARY KEY,
             game_uid TEXT NOT NULL REFERENCES game_activity(game_uid) ON DELETE CASCADE,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             emulator TEXT NOT NULL,
             started_at INTEGER NOT NULL CHECK (started_at >= 0),
             ended_at INTEGER,
             duration_seconds INTEGER NOT NULL DEFAULT 0 CHECK (duration_seconds >= 0),
             outcome TEXT NOT NULL CHECK (
                 outcome IN ('running', 'completed', 'failed', 'terminated')
             ),
             CHECK (
                 (outcome='running' AND ended_at IS NULL)
                 OR (outcome<>'running' AND ended_at IS NOT NULL)
             )
         );
         CREATE INDEX IF NOT EXISTS play_sessions_game_started
             ON play_sessions(game_uid, started_at DESC);
         CREATE INDEX IF NOT EXISTS play_sessions_running
             ON play_sessions(outcome, started_at) WHERE outcome='running';
         CREATE TABLE IF NOT EXISTS media_playback_progress (
             game_uid TEXT NOT NULL,
             media_key TEXT NOT NULL,
             position_ms INTEGER NOT NULL CHECK (position_ms >= 0),
             duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
             updated_at INTEGER NOT NULL CHECK (updated_at >= 0),
             PRIMARY KEY (game_uid, media_key)
         );
         CREATE INDEX IF NOT EXISTS media_playback_progress_updated
             ON media_playback_progress(updated_at DESC, game_uid);
         CREATE TABLE IF NOT EXISTS media_transfers (
             game_uid TEXT NOT NULL,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             media_kind TEXT NOT NULL CHECK (media_kind IN ('manual', 'video')),
             provider TEXT NOT NULL CHECK (provider IN ('minerva', 'emumovies')),
             transport TEXT NOT NULL CHECK (transport IN ('torrent', 'http')),
             source_url TEXT NOT NULL,
             remote_job_id TEXT NOT NULL,
             source_item_index INTEGER NOT NULL CHECK (source_item_index >= 0),
             source_item_path TEXT NOT NULL,
             local_download_path TEXT NOT NULL,
             local_target_path TEXT NOT NULL,
             state TEXT NOT NULL CHECK (
                 state IN (
                     'queued', 'downloading', 'paused', 'complete',
                     'published', 'cancelled', 'error'
                 )
             ),
             progress REAL NOT NULL CHECK (progress BETWEEN 0.0 AND 1.0),
             download_speed INTEGER NOT NULL CHECK (download_speed >= 0),
             downloaded_bytes INTEGER NOT NULL CHECK (downloaded_bytes >= 0),
             total_bytes INTEGER NOT NULL CHECK (total_bytes >= 0),
             message TEXT NOT NULL,
             created_at INTEGER NOT NULL CHECK (created_at >= 0),
             updated_at INTEGER NOT NULL CHECK (updated_at >= 0),
             PRIMARY KEY (game_uid, media_kind)
         );
         CREATE INDEX IF NOT EXISTS media_transfers_remote_job
             ON media_transfers(provider, remote_job_id, state);
         CREATE INDEX IF NOT EXISTS media_transfers_updated
             ON media_transfers(updated_at DESC, game_uid);
         CREATE TABLE IF NOT EXISTS user_collections (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL COLLATE NOCASE UNIQUE,
             description TEXT NOT NULL,
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS user_collection_games (
             collection_id TEXT NOT NULL REFERENCES user_collections(id) ON DELETE CASCADE,
             game_uid TEXT NOT NULL,
             sort_order INTEGER NOT NULL,
             added_at INTEGER NOT NULL,
             PRIMARY KEY (collection_id, game_uid)
         );
         CREATE INDEX IF NOT EXISTS user_collection_games_order
             ON user_collection_games(collection_id, sort_order, game_uid);
         CREATE INDEX IF NOT EXISTS user_collection_games_game
             ON user_collection_games(game_uid);
         CREATE TABLE IF NOT EXISTS game_emulator_preferences (
             game_uid TEXT PRIMARY KEY,
             emulator_id TEXT NOT NULL,
             runtime_kind TEXT NOT NULL CHECK (
                 runtime_kind IN ('standalone', 'retroarch')
             ),
             core_name TEXT NOT NULL DEFAULT '',
             updated_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS platform_emulator_preferences (
             platform_key TEXT PRIMARY KEY,
             platform_display TEXT NOT NULL,
             emulator_id TEXT NOT NULL,
             runtime_kind TEXT NOT NULL CHECK (
                 runtime_kind IN ('standalone', 'retroarch')
             ),
             core_name TEXT NOT NULL DEFAULT '',
             updated_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS managed_emulator_installs (
             emulator_id TEXT NOT NULL,
             host_system_slug TEXT NOT NULL CHECK (
                 host_system_slug IN ('linux', 'windows', 'macos')
             ),
             manager TEXT NOT NULL CHECK (
                 manager IN ('flatpak', 'appimage', 'nix', 'github', 'direct', 'winget', 'homebrew', 'libretro')
             ),
             package_id TEXT NOT NULL,
             install_path TEXT NOT NULL,
             installed_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL,
             PRIMARY KEY (host_system_slug, manager, package_id)
         );
         CREATE TABLE IF NOT EXISTS firmware_packages (
             source_id TEXT NOT NULL,
             package_name TEXT NOT NULL,
             archive_path TEXT NOT NULL,
             extracted_root TEXT NOT NULL,
             sha256 TEXT NOT NULL CHECK (length(sha256)=64),
             file_count INTEGER NOT NULL CHECK (file_count >= 0),
             imported_at INTEGER NOT NULL,
             PRIMARY KEY (source_id, package_name)
         );
         CREATE TABLE IF NOT EXISTS firmware_files (
             source_id TEXT NOT NULL,
             package_name TEXT NOT NULL,
             relative_path TEXT NOT NULL,
             store_path TEXT NOT NULL,
             sha256 TEXT NOT NULL CHECK (length(sha256)=64),
             file_size INTEGER NOT NULL CHECK (file_size >= 0),
             PRIMARY KEY (source_id, package_name, relative_path),
             FOREIGN KEY (source_id, package_name)
                 REFERENCES firmware_packages(source_id, package_name)
                 ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS firmware_files_sha256
             ON firmware_files(sha256);
         CREATE TABLE IF NOT EXISTS firmware_downloads (
             source_id TEXT NOT NULL,
             package_name TEXT NOT NULL,
             torrent_url TEXT NOT NULL,
             info_hash TEXT NOT NULL,
             file_index INTEGER NOT NULL CHECK (file_index >= 0),
             file_path TEXT NOT NULL,
             local_path TEXT NOT NULL,
             state TEXT NOT NULL CHECK (
                 state IN ('queued', 'downloading', 'paused', 'complete', 'imported', 'failed')
             ),
             progress REAL NOT NULL CHECK (progress >= 0.0 AND progress <= 1.0),
             message TEXT NOT NULL,
             updated_at INTEGER NOT NULL,
             PRIMARY KEY (source_id, package_name)
         );
         CREATE INDEX IF NOT EXISTS firmware_downloads_info_hash
             ON firmware_downloads(info_hash, state);
         CREATE TABLE IF NOT EXISTS firmware_installs (
             rule_key TEXT NOT NULL,
             runtime_path TEXT NOT NULL,
             target_path TEXT NOT NULL,
             source_id TEXT NOT NULL,
             package_name TEXT NOT NULL,
             synced_at INTEGER NOT NULL,
             PRIMARY KEY (rule_key, runtime_path),
             FOREIGN KEY (source_id, package_name)
                 REFERENCES firmware_packages(source_id, package_name)
                 ON DELETE CASCADE
         );
         CREATE TABLE IF NOT EXISTS prepared_game_installs (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             platform TEXT NOT NULL,
             collection TEXT NOT NULL CHECK (
                 collection IN ('exodos', 'exowin3x', 'exowin9x')
             ),
             shortname TEXT NOT NULL,
             source_archive_path TEXT NOT NULL,
             companion_archive_path TEXT,
             metadata_archive_path TEXT NOT NULL,
             utility_archive_path TEXT,
             source_signature TEXT NOT NULL,
             install_root TEXT NOT NULL,
             launch_config_path TEXT NOT NULL,
             prepared_at INTEGER NOT NULL,
             last_used_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS prepared_game_installs_launchbox_id
             ON prepared_game_installs(launchbox_db_id) WHERE launchbox_db_id > 0;",
    )?;
    if !column_exists(connection, "download_jobs", "launchbox_db_id")? {
        connection.execute(
            "ALTER TABLE download_jobs ADD COLUMN launchbox_db_id INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !column_exists(connection, "download_jobs", "post_import_action")? {
        connection.execute(
            "ALTER TABLE download_jobs ADD COLUMN post_import_action TEXT NOT NULL DEFAULT 'none' CHECK (post_import_action IN ('none', 'pause_pending', 'pause_applied'))",
            [],
        )?;
    }
    if !column_exists(connection, "download_jobs", "download_plan")? {
        connection.execute(
            "ALTER TABLE download_jobs ADD COLUMN download_plan TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !column_exists(connection, "library_preferences", "artwork_type")? {
        connection.execute(
            "ALTER TABLE library_preferences ADD COLUMN artwork_type TEXT NOT NULL DEFAULT 'box-front'",
            [],
        )?;
    }
    if !column_exists(connection, "library_preferences", "grid_zoom")? {
        connection.execute(
            "ALTER TABLE library_preferences ADD COLUMN grid_zoom INTEGER NOT NULL DEFAULT 100",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "seeding_policy")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN seeding_policy TEXT NOT NULL DEFAULT 'follow_client' CHECK (seeding_policy IN ('follow_client', 'pause_after_import'))",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "preferred_region")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN preferred_region TEXT NOT NULL DEFAULT 'USA' CHECK (preferred_region IN ('USA', 'Japan', 'Europe', 'World', 'Asia', 'any'))",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "version_preference")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN version_preference TEXT NOT NULL DEFAULT 'latest' CHECK (version_preference IN ('latest', 'original'))",
            [],
        )?;
    }
    Ok(())
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_collection_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("collection name is required");
    }
    if name.chars().count() > 100 {
        bail!("collection name must be at most 100 characters");
    }
    Ok(name.to_owned())
}

fn validate_play_identity(game_uid: &str, title: &str, platform: &str) -> Result<()> {
    if game_uid.trim().is_empty() || title.trim().is_empty() || platform.trim().is_empty() {
        bail!("play activity requires a stable game identity, title, and platform");
    }
    if game_uid.chars().count() > 512
        || title.chars().count() > 1000
        || platform.chars().count() > 500
    {
        bail!("play activity identity fields exceed their safety limits");
    }
    Ok(())
}

fn validate_completion_state(completion_state: &str) -> Result<()> {
    if !matches!(
        completion_state,
        "not_started" | "in_progress" | "completed" | "on_hold" | "abandoned"
    ) {
        bail!("unsupported completion state {completion_state}");
    }
    Ok(())
}

fn validate_media_playback_identity(game_uid: &str, media_key: &str) -> Result<()> {
    if game_uid.trim().is_empty() || media_key.trim().is_empty() {
        bail!("media playback progress requires stable game and media identities");
    }
    if game_uid.chars().count() > 512 || media_key.chars().count() > 256 {
        bail!("media playback identity fields exceed their safety limits");
    }
    Ok(())
}

fn validate_media_transfer_identity(game_uid: &str, media_kind: &str) -> Result<()> {
    if game_uid.trim().is_empty() || game_uid.chars().count() > 512 {
        bail!("media transfer requires a bounded stable game identity");
    }
    if !matches!(media_kind, "manual" | "video") {
        bail!("unsupported media transfer kind {media_kind}");
    }
    Ok(())
}

fn validate_media_transfer(transfer: &MediaTransfer) -> Result<()> {
    validate_media_transfer_identity(&transfer.game_uid, &transfer.media_kind)?;
    if !matches!(transfer.provider.as_str(), "minerva" | "emumovies") {
        bail!("unsupported media transfer provider {}", transfer.provider);
    }
    if !matches!(transfer.transport.as_str(), "torrent" | "http") {
        bail!(
            "unsupported media transfer transport {}",
            transfer.transport
        );
    }
    if transfer.launchbox_db_id < 0 {
        bail!("media transfer LaunchBox ID cannot be negative");
    }
    for (label, value, limit) in [
        ("title", transfer.title.as_str(), 2_048),
        ("platform", transfer.platform.as_str(), 1_024),
        ("source URL", transfer.source_url.as_str(), 8_192),
        ("remote job identity", transfer.remote_job_id.as_str(), 512),
        (
            "source item path",
            transfer.source_item_path.as_str(),
            8_192,
        ),
        ("message", transfer.message.as_str(), 8_192),
    ] {
        if value.trim().is_empty() || value.chars().count() > limit {
            bail!("media transfer {label} is empty or exceeds its safety limit");
        }
    }
    if transfer.local_download_path.as_os_str().is_empty()
        || transfer.local_target_path.as_os_str().is_empty()
    {
        bail!("media transfer paths cannot be empty");
    }
    if !matches!(
        transfer.state.as_str(),
        "queued" | "downloading" | "paused" | "complete" | "published" | "cancelled" | "error"
    ) {
        bail!("unsupported media transfer state {}", transfer.state);
    }
    if !transfer.progress.is_finite() || !(0.0..=1.0).contains(&transfer.progress) {
        bail!("media transfer progress must be between zero and one");
    }
    if transfer.created_at < 0 || transfer.updated_at < 0 {
        bail!("media transfer timestamps cannot be negative");
    }
    Ok(())
}

fn media_transfer_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaTransfer> {
    let source_item_index = row.get::<_, i64>(9)?;
    let download_speed = row.get::<_, i64>(15)?;
    let downloaded_bytes = row.get::<_, i64>(16)?;
    let total_bytes = row.get::<_, i64>(17)?;
    Ok(MediaTransfer {
        game_uid: row.get(0)?,
        launchbox_db_id: row.get(1)?,
        title: row.get(2)?,
        platform: row.get(3)?,
        media_kind: row.get(4)?,
        provider: row.get(5)?,
        transport: row.get(6)?,
        source_url: row.get(7)?,
        remote_job_id: row.get(8)?,
        source_item_index: u32::try_from(source_item_index).unwrap_or_default(),
        source_item_path: row.get(10)?,
        local_download_path: PathBuf::from(row.get::<_, String>(11)?),
        local_target_path: PathBuf::from(row.get::<_, String>(12)?),
        state: row.get(13)?,
        progress: row.get(14)?,
        download_speed: u64::try_from(download_speed).unwrap_or_default(),
        downloaded_bytes: u64::try_from(downloaded_bytes).unwrap_or_default(),
        total_bytes: u64::try_from(total_bytes).unwrap_or_default(),
        message: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
    })
}

fn play_activity_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlayActivity> {
    Ok(PlayActivity {
        game_uid: row.get(0)?,
        launchbox_db_id: row.get(1)?,
        title: row.get(2)?,
        platform: row.get(3)?,
        play_count: row.get(4)?,
        total_play_time_seconds: row.get(5)?,
        last_played_at: row.get(6)?,
        first_played_at: row.get(7)?,
        completion_state: row.get(8)?,
    })
}

fn validate_collection_description(description: &str) -> Result<String> {
    let description = description.trim();
    if description.chars().count() > 1000 {
        bail!("collection description must be at most 1000 characters");
    }
    Ok(description.to_owned())
}

fn validate_emulator_preference(
    identity: &str,
    emulator_id: &str,
    runtime_kind: &str,
    core_name: &str,
) -> Result<()> {
    if identity.trim().is_empty() {
        bail!("a stable game or platform identity is required for an emulator preference");
    }
    if emulator_id.trim().is_empty() {
        bail!("an emulator identity is required for an emulator preference");
    }
    if !matches!(runtime_kind, "standalone" | "retroarch") {
        bail!("unsupported emulator runtime kind {runtime_kind}");
    }
    if runtime_kind == "retroarch" && core_name.trim().is_empty() {
        bail!("a RetroArch preference requires an exact core name");
    }
    if runtime_kind == "standalone" && !core_name.trim().is_empty() {
        bail!("a standalone emulator preference cannot include a RetroArch core");
    }
    Ok(())
}

fn validate_managed_emulator_install(
    emulator_id: &str,
    host_system_slug: &str,
    manager: &str,
    package_id: &str,
) -> Result<()> {
    if emulator_id.trim().is_empty() || package_id.trim().is_empty() {
        bail!("managed emulator installs require exact emulator and package identities");
    }
    if !matches!(host_system_slug, "linux" | "windows" | "macos") {
        bail!("unsupported emulator host {host_system_slug}");
    }
    if !matches!(
        manager,
        "flatpak" | "appimage" | "nix" | "github" | "direct" | "winget" | "homebrew" | "libretro"
    ) {
        bail!("unsupported emulator install manager {manager}");
    }
    Ok(())
}

fn validate_firmware_package_receipt(receipt: &FirmwarePackageReceipt) -> Result<()> {
    if receipt.source_id.trim().is_empty() || receipt.package_name.trim().is_empty() {
        bail!("firmware packages require exact source and package identities");
    }
    if receipt.archive_path.trim().is_empty() || receipt.extracted_root.trim().is_empty() {
        bail!("firmware package receipts require archive and extracted paths");
    }
    if receipt.sha256.len() != 64 || !receipt.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("firmware package receipts require a SHA-256 digest");
    }
    if receipt.file_count < 0 {
        bail!("firmware package file counts cannot be negative");
    }
    Ok(())
}

fn validate_firmware_file_receipt(
    package: &FirmwarePackageReceipt,
    file: &FirmwareFileReceipt,
) -> Result<()> {
    if file.source_id != package.source_id || file.package_name != package.package_name {
        bail!("firmware file manifest identities do not match their package");
    }
    if file.relative_path.trim().is_empty()
        || file.store_path.trim().is_empty()
        || Path::new(&file.relative_path).is_absolute()
        || Path::new(&file.relative_path)
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        bail!("firmware file manifests require safe relative paths");
    }
    if file.sha256.len() != 64
        || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || file.file_size < 0
    {
        bail!("firmware file manifests require a digest and non-negative size");
    }
    Ok(())
}

fn validate_firmware_install_receipt(receipt: &FirmwareInstallReceipt) -> Result<()> {
    if receipt.rule_key.trim().is_empty()
        || receipt.runtime_path.trim().is_empty()
        || receipt.target_path.trim().is_empty()
        || receipt.source_id.trim().is_empty()
        || receipt.package_name.trim().is_empty()
    {
        bail!("firmware install receipts require exact rule, package, and path identities");
    }
    Ok(())
}

fn firmware_download_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<FirmwareDownloadReceipt> {
    let file_index = row.get::<_, i64>(4)?;
    Ok(FirmwareDownloadReceipt {
        source_id: row.get(0)?,
        package_name: row.get(1)?,
        torrent_url: row.get(2)?,
        info_hash: row.get(3)?,
        file_index: u32::try_from(file_index).unwrap_or_default(),
        file_path: row.get(5)?,
        local_path: row.get(6)?,
        state: row.get(7)?,
        progress: row.get(8)?,
        message: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn validate_firmware_download_receipt(receipt: &FirmwareDownloadReceipt) -> Result<()> {
    if receipt.source_id.trim().is_empty()
        || receipt.package_name.trim().is_empty()
        || !receipt.torrent_url.starts_with("https://")
        || receipt.info_hash.trim().is_empty()
        || receipt.file_path.trim().is_empty()
        || receipt.local_path.trim().is_empty()
    {
        bail!("firmware downloads require exact source, torrent, file, and path identities");
    }
    if !matches!(
        receipt.state.as_str(),
        "queued" | "downloading" | "paused" | "complete" | "imported" | "failed"
    ) || !(0.0..=1.0).contains(&receipt.progress)
        || !receipt.progress.is_finite()
    {
        bail!("firmware download state is invalid");
    }
    Ok(())
}

pub(crate) fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_default()
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        (directory, store)
    }

    #[test]
    fn defaults_and_settings_round_trip_without_a_plaintext_password_column() {
        let (_directory, store) = store();
        assert_eq!(store.load().unwrap(), AppSettings::default());

        let expected = AppSettings {
            qbittorrent_host: "media.local".into(),
            qbittorrent_port: 9443,
            qbittorrent_use_https: true,
            qbittorrent_username: "lunchbox".into(),
            rom_directory: PathBuf::from("/native/roms"),
            qbittorrent_container_rom_directory: "/client/roms".into(),
            torrent_library_directory: PathBuf::from("/native/downloads"),
            qbittorrent_container_torrent_library_directory: "/downloads".into(),
            download_entire_torrent: true,
            file_link_mode: "hardlink".into(),
            seeding_policy: "pause_after_import".into(),
            preferred_region: "Japan".into(),
            version_preference: "original".into(),
        };
        store.save(&expected).unwrap();
        assert_eq!(store.load().unwrap(), expected);

        let connection = Connection::open(store.path()).unwrap();
        let mut statement = connection
            .prepare("PRAGMA table_info(app_settings)")
            .unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(!columns.iter().any(|column| column.contains("password")));
    }

    #[test]
    fn download_queue_is_durable_and_updates_in_place() {
        let (_directory, store) = store();
        let mut job = DownloadJob::queued(NewDownloadJob {
            game_id: "game-1".into(),
            launchbox_db_id: 42,
            title: "Game".into(),
            platform: "Platform".into(),
            torrent_url: "https://example.invalid/game.torrent".into(),
            torrent_file_index: Some(7),
            torrent_file_path: "Platform/Game.zip".into(),
            info_hash: "abc123".into(),
            client_save_path: "/downloads/lunchbox".into(),
            local_download_path: PathBuf::from("/native/downloads/lunchbox"),
            local_target_path: PathBuf::from("/native/roms/Platform/Game.zip"),
            download_plan: String::new(),
        });
        store.upsert_job(&job).unwrap();
        job.state = "downloading".into();
        job.progress = 0.5;
        job.download_speed = 1024;
        job.updated_at += 1;
        store.upsert_job(&job).unwrap();

        assert_eq!(store.jobs().unwrap(), vec![job]);
    }

    #[test]
    fn download_history_cleanup_never_removes_active_or_pending_import_jobs() {
        let (_directory, store) = store();
        let make_job = |game_id: &str, state: &str| {
            let mut job = DownloadJob::queued(NewDownloadJob {
                game_id: game_id.into(),
                launchbox_db_id: 42,
                title: game_id.into(),
                platform: "Platform".into(),
                torrent_url: "https://example.invalid/game.torrent".into(),
                torrent_file_index: Some(7),
                torrent_file_path: format!("Platform/{game_id}.zip"),
                info_hash: format!("hash-{game_id}"),
                client_save_path: "/downloads/lunchbox".into(),
                local_download_path: PathBuf::from("/native/downloads/lunchbox"),
                local_target_path: PathBuf::from(format!("/native/roms/Platform/{game_id}.zip")),
                download_plan: String::new(),
            });
            job.state = state.into();
            job
        };
        let mut pause_pending = make_job("pause-pending", "imported");
        pause_pending.post_import_action = "pause_pending".into();
        let jobs = [
            make_job("queued", "queued"),
            make_job("downloading", "downloading"),
            make_job("paused", "paused"),
            make_job("complete", "complete"),
            make_job("imported", "imported"),
            make_job("cancelled", "cancelled"),
            pause_pending,
        ];
        for job in &jobs {
            store.upsert_job(job).unwrap();
        }

        assert!(!store.delete_job_record(&jobs[0].id).unwrap());
        assert!(store.delete_job_record(&jobs[5].id).unwrap());
        assert!(!store.delete_job_record(&jobs[6].id).unwrap());
        assert_eq!(store.clear_finished_job_records().unwrap(), 1);
        assert_eq!(
            store
                .jobs()
                .unwrap()
                .into_iter()
                .map(|job| job.state)
                .collect::<HashSet<_>>(),
            HashSet::from([
                "queued".to_owned(),
                "downloading".to_owned(),
                "paused".to_owned(),
                "complete".to_owned(),
                "imported".to_owned(),
            ])
        );
    }

    #[test]
    fn library_content_filters_default_safely_and_round_trip() {
        let (_directory, store) = store();
        assert_eq!(
            store.load_library_preferences().unwrap(),
            LibraryPreferences::default()
        );

        let expected = LibraryPreferences {
            hide_non_retail: false,
            hide_adult: true,
            artwork_type: "screenshot".into(),
            grid_zoom: 135,
        };
        store.save_library_preferences(&expected).unwrap();
        assert_eq!(store.load_library_preferences().unwrap(), expected);
    }

    #[test]
    fn favorites_use_stable_game_identity_and_round_trip() {
        let (_directory, store) = store();
        assert!(store.favorite_game_ids().unwrap().is_empty());

        store
            .set_favorite("stable-game-uid", 42, "Game", "Platform", true)
            .unwrap();
        assert_eq!(
            store.favorite_game_ids().unwrap(),
            HashSet::from(["stable-game-uid".to_owned()])
        );

        store
            .set_favorite("stable-game-uid", 99, "Updated", "Platform", true)
            .unwrap();
        let connection = Connection::open(store.path()).unwrap();
        let metadata = connection
            .query_row(
                "SELECT launchbox_db_id, title FROM favorite_games WHERE game_uid=?1",
                ["stable-game-uid"],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .unwrap();
        assert_eq!(metadata, (99, "Updated".to_owned()));

        store
            .set_favorite("stable-game-uid", 99, "Updated", "Platform", false)
            .unwrap();
        assert!(store.favorite_game_ids().unwrap().is_empty());
        assert!(store.set_favorite("", 0, "", "", true).is_err());
    }

    #[test]
    fn emulator_preferences_resolve_game_before_platform_and_preserve_runtime() {
        let (_directory, store) = store();
        store
            .set_platform_emulator_preference(
                "Nintendo Entertainment System",
                "nestopia-id",
                "standalone",
                "",
            )
            .unwrap();
        assert_eq!(
            store
                .emulator_preference("game-one", "nintendo entertainment system")
                .unwrap(),
            Some(EmulatorPreference {
                emulator_id: "nestopia-id".into(),
                runtime_kind: "standalone".into(),
                core_name: String::new(),
                scope: "platform".into(),
            })
        );

        store
            .set_game_emulator_preference("game-one", "mesen-id", "retroarch", "mesen")
            .unwrap();
        assert_eq!(
            store
                .emulator_preference("game-one", "Nintendo Entertainment System")
                .unwrap(),
            Some(EmulatorPreference {
                emulator_id: "mesen-id".into(),
                runtime_kind: "retroarch".into(),
                core_name: "mesen".into(),
                scope: "game".into(),
            })
        );

        store.clear_game_emulator_preference("game-one").unwrap();
        assert_eq!(
            store
                .emulator_preference("game-one", "Nintendo Entertainment System")
                .unwrap()
                .unwrap()
                .scope,
            "platform"
        );
        store
            .clear_platform_emulator_preference("Nintendo Entertainment System")
            .unwrap();
        assert_eq!(
            store
                .emulator_preference("game-one", "Nintendo Entertainment System")
                .unwrap(),
            None
        );
        assert!(
            store
                .set_game_emulator_preference("game-one", "mesen-id", "retroarch", "")
                .is_err()
        );
    }

    #[test]
    fn managed_emulator_receipts_are_exact_idempotent_and_removable() {
        let (_directory, store) = store();
        store
            .record_managed_emulator_install(
                "mesen-id",
                "linux",
                "appimage",
                "SourMesen/Mesen2",
                "/managed/mesen",
            )
            .unwrap();
        store
            .record_managed_emulator_install(
                "mesen-id",
                "linux",
                "appimage",
                "SourMesen/Mesen2",
                "/managed/mesen/current",
            )
            .unwrap();

        let installs = store.managed_emulator_installs().unwrap();
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0].package_id, "SourMesen/Mesen2");
        assert_eq!(installs[0].install_path, "/managed/mesen/current");

        store
            .remove_managed_emulator_install("linux", "appimage", "SourMesen/Mesen2")
            .unwrap();
        assert!(store.managed_emulator_installs().unwrap().is_empty());
        assert!(
            store
                .record_managed_emulator_install("", "linux", "snap", "bad", "")
                .is_err()
        );
    }

    #[test]
    fn firmware_receipts_are_exact_and_idempotent() {
        let (_directory, store) = store();
        let mut package = FirmwarePackageReceipt {
            source_id: "minerva:retroarch-system-files".into(),
            package_name: "Sony - PlayStation (SwanStation).zip".into(),
            archive_path: "/store/firmware/playstation.zip".into(),
            extracted_root: "/store/firmware/playstation".into(),
            sha256: "a".repeat(64),
            file_count: 1,
            imported_at: 10,
        };
        let file = FirmwareFileReceipt {
            source_id: package.source_id.clone(),
            package_name: package.package_name.clone(),
            relative_path: "scph5501.bin".into(),
            store_path: "/store/firmware/playstation/scph5501.bin".into(),
            sha256: "b".repeat(64),
            file_size: 512 * 1024,
        };
        store
            .record_firmware_package(&package, std::slice::from_ref(&file))
            .unwrap();
        package.imported_at = 11;
        store
            .record_firmware_package(&package, std::slice::from_ref(&file))
            .unwrap();

        let install = FirmwareInstallReceipt {
            rule_key: "duckstation:DuckStation:Sony Playstation".into(),
            runtime_path: "/runtime/bios".into(),
            target_path: "/runtime/bios".into(),
            source_id: package.source_id.clone(),
            package_name: package.package_name.clone(),
            synced_at: 12,
        };
        store.record_firmware_install(&install).unwrap();
        store.record_firmware_install(&install).unwrap();

        assert_eq!(store.firmware_packages().unwrap(), vec![package]);
        assert_eq!(
            store
                .firmware_files(
                    "minerva:retroarch-system-files",
                    "Sony - PlayStation (SwanStation).zip"
                )
                .unwrap(),
            vec![file.clone()]
        );
        assert_eq!(store.firmware_installs().unwrap(), vec![install]);
        let mut download = FirmwareDownloadReceipt {
            source_id: "minerva:retroarch-system-files".into(),
            package_name: "Sony - PlayStation (SwanStation).zip".into(),
            torrent_url: "https://example.invalid/firmware.torrent".into(),
            info_hash: "abc123".into(),
            file_index: 42,
            file_path: "firmware/playstation.zip".into(),
            local_path: "/downloads/firmware/playstation.zip".into(),
            state: "downloading".into(),
            progress: 0.5,
            message: "Downloading".into(),
            updated_at: 13,
        };
        store.record_firmware_download(&download).unwrap();
        download.state = "complete".into();
        download.progress = 1.0;
        download.updated_at = 14;
        store.record_firmware_download(&download).unwrap();
        assert_eq!(
            store.firmware_downloads_for_info_hash("abc123").unwrap(),
            vec![download]
        );
        assert!(
            store
                .record_firmware_package(
                    &FirmwarePackageReceipt {
                        sha256: "bad".into(),
                        ..store.firmware_packages().unwrap().remove(0)
                    },
                    std::slice::from_ref(&file)
                )
                .is_err()
        );
        let connection = Connection::open(store.path()).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM firmware_files", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            1
        );
    }

    #[test]
    fn play_sessions_track_real_duration_and_finalize_once() {
        let (_directory, store) = store();
        assert!(store.play_activity("game-1").unwrap().is_none());
        store
            .set_completion_state(
                "game-1",
                42,
                "Metroid",
                "Nintendo Entertainment System",
                "not_started",
            )
            .unwrap();
        let session = store
            .begin_play_session(
                "game-1",
                42,
                "Metroid",
                "Nintendo Entertainment System",
                "RetroArch · fceumm",
            )
            .unwrap();
        assert!(
            store
                .finish_play_session(&session.session_id, 93, "completed")
                .unwrap()
        );
        assert!(
            !store
                .finish_play_session(&session.session_id, 93, "completed")
                .unwrap()
        );

        let activity = store.play_activity("game-1").unwrap().unwrap();
        assert_eq!(activity.play_count, 1);
        assert_eq!(activity.total_play_time_seconds, 93);
        assert_eq!(activity.completion_state, "in_progress");
        assert!(activity.first_played_at > 0);
        assert!(activity.last_played_at >= activity.first_played_at);
        assert_eq!(store.recent_play_activity(10).unwrap(), vec![activity]);

        assert!(
            store
                .set_completion_state(
                    "game-1",
                    42,
                    "Metroid",
                    "Nintendo Entertainment System",
                    "finished",
                )
                .is_err()
        );
        assert!(
            store
                .finish_play_session(&session.session_id, 0, "unknown")
                .is_err()
        );
    }

    #[test]
    fn media_progress_is_scoped_to_the_exact_video_and_replaces_stale_media() {
        let (_directory, store) = store();
        assert!(
            store
                .media_playback_progress("game-1", "sha256:first")
                .unwrap()
                .is_none()
        );
        let saved = store
            .save_media_playback_progress("game-1", "sha256:first", 31_500, 90_000)
            .unwrap();
        assert_eq!(saved.position_ms, 31_500);
        assert_eq!(
            store
                .media_playback_progress("game-1", "sha256:first")
                .unwrap(),
            Some(saved)
        );

        store
            .save_media_playback_progress("game-1", "sha256:replacement", 2_000, 80_000)
            .unwrap();
        assert!(
            store
                .media_playback_progress("game-1", "sha256:first")
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .save_media_playback_progress("game-1", "sha256:replacement", -1, 80_000)
                .is_err()
        );
    }

    #[test]
    fn media_transfers_are_durable_scoped_and_share_remote_jobs() {
        let (_directory, store) = store();
        let mut first = MediaTransfer {
            game_uid: "game-one".into(),
            launchbox_db_id: 42,
            title: "First Game".into(),
            platform: "Sega Pico".into(),
            media_kind: "manual".into(),
            provider: "minerva".into(),
            transport: "torrent".into(),
            source_url: "https://example.invalid/manuals.torrent".into(),
            remote_job_id: "ABCDEF012345".into(),
            source_item_index: 3,
            source_item_path: "Manuals/First Game.zip".into(),
            local_download_path: PathBuf::from("/downloads/Manuals/First Game.zip"),
            local_target_path: PathBuf::from("/media/game-one/minerva/manual.zip"),
            state: "queued".into(),
            progress: 0.0,
            download_speed: 0,
            downloaded_bytes: 0,
            total_bytes: 1_024,
            message: "Queued".into(),
            created_at: 10,
            updated_at: 10,
        };
        let second = MediaTransfer {
            game_uid: "game-two".into(),
            source_item_index: 7,
            source_item_path: "Manuals/Second Game.zip".into(),
            local_download_path: PathBuf::from("/downloads/Manuals/Second Game.zip"),
            local_target_path: PathBuf::from("/media/game-two/minerva/manual.zip"),
            created_at: 11,
            updated_at: 11,
            ..first.clone()
        };
        store.upsert_media_transfer(&first).unwrap();
        store.upsert_media_transfer(&second).unwrap();
        assert_eq!(
            store.media_transfer("game-one", "manual").unwrap(),
            Some(first.clone())
        );
        assert_eq!(
            store
                .media_transfers_for_remote_job("minerva", "abcdef012345")
                .unwrap(),
            vec![first.clone(), second]
        );

        first.state = "downloading".into();
        first.progress = 0.5;
        first.download_speed = 2_048;
        first.downloaded_bytes = 512;
        first.message = "Downloading".into();
        first.updated_at = 12;
        store.upsert_media_transfer(&first).unwrap();
        assert_eq!(
            store.media_transfer("game-one", "manual").unwrap(),
            Some(first)
        );

        let mut invalid = store.media_transfer("game-two", "manual").unwrap().unwrap();
        invalid.progress = f64::NAN;
        assert!(store.upsert_media_transfer(&invalid).is_err());
        assert!(store.media_transfer("game-one", "soundtrack").is_err());
    }

    #[test]
    fn named_collections_preserve_membership_and_cascade_on_delete() {
        let (_directory, store) = store();
        let collection = store
            .create_collection("  Couch Co-op  ", " Games for the living room ")
            .unwrap();
        assert_eq!(collection.name, "Couch Co-op");
        assert_eq!(collection.description, "Games for the living room");

        store
            .set_collection_membership(&collection.id, "game-one", true)
            .unwrap();
        store
            .set_collection_membership(&collection.id, "game-two", true)
            .unwrap();
        store
            .set_collection_membership(&collection.id, "game-one", true)
            .unwrap();
        let loaded = store.user_collections().unwrap();
        assert_eq!(loaded.collections[0].game_count, 2);
        assert_eq!(
            loaded.members[&collection.id],
            HashSet::from(["game-one".to_owned(), "game-two".to_owned()])
        );

        store
            .update_collection(&collection.id, "Arcade Night", "Favorites")
            .unwrap();
        assert_eq!(
            store.user_collections().unwrap().collections[0].name,
            "Arcade Night"
        );
        assert!(
            store
                .create_collection("arcade night", "duplicate")
                .is_err()
        );

        store
            .set_collection_membership(&collection.id, "game-one", false)
            .unwrap();
        assert_eq!(
            store.user_collections().unwrap().collections[0].game_count,
            1
        );
        store.delete_collection(&collection.id).unwrap();
        assert_eq!(
            store.user_collections().unwrap(),
            UserCollections::default()
        );
    }

    #[test]
    fn collection_validation_rejects_empty_and_unbounded_fields() {
        let (_directory, store) = store();
        assert!(store.create_collection(" ", "").is_err());
        assert!(store.create_collection(&"x".repeat(101), "").is_err());
        let collection = store.create_collection("Valid", "").unwrap();
        assert!(
            store
                .update_collection(&collection.id, "Valid", &"x".repeat(1001))
                .is_err()
        );
        assert!(
            store
                .set_collection_membership(&collection.id, "", true)
                .is_err()
        );
    }

    #[test]
    fn rejects_unknown_link_modes() {
        let settings = AppSettings {
            file_link_mode: "magic".into(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_unknown_seeding_policies() {
        let settings = AppSettings {
            seeding_policy: "delete_everything".into(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_unknown_release_preferences() {
        let settings = AppSettings {
            preferred_region: "Moon".into(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());

        let settings = AppSettings {
            version_preference: "random".into(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn older_state_database_migrates_to_safe_seeding_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE app_settings (
                     id INTEGER PRIMARY KEY,
                     qbittorrent_host TEXT NOT NULL,
                     qbittorrent_port INTEGER NOT NULL,
                     qbittorrent_use_https INTEGER NOT NULL,
                     qbittorrent_username TEXT NOT NULL,
                     rom_directory TEXT NOT NULL,
                     qbittorrent_container_rom_directory TEXT NOT NULL,
                     torrent_library_directory TEXT NOT NULL,
                     qbittorrent_container_torrent_library_directory TEXT NOT NULL,
                     download_entire_torrent INTEGER NOT NULL,
                     file_link_mode TEXT NOT NULL
                 );
                 INSERT INTO app_settings VALUES (
                     1, '127.0.0.1', 8080, 0, '', '', '', '', '', 0, 'symlink'
                 );
                 CREATE TABLE download_jobs (
                     id TEXT PRIMARY KEY,
                     game_id TEXT NOT NULL,
                     launchbox_db_id INTEGER NOT NULL DEFAULT 0,
                     title TEXT NOT NULL,
                     platform TEXT NOT NULL,
                     torrent_url TEXT NOT NULL,
                     torrent_file_index INTEGER,
                     torrent_file_path TEXT NOT NULL,
                     info_hash TEXT NOT NULL,
                     client_save_path TEXT NOT NULL,
                     local_download_path TEXT NOT NULL,
                     local_target_path TEXT NOT NULL,
                     state TEXT NOT NULL,
                     progress REAL NOT NULL,
                     download_speed INTEGER NOT NULL,
                     downloaded_bytes INTEGER NOT NULL,
                     total_bytes INTEGER NOT NULL,
                     message TEXT NOT NULL,
                     created_at INTEGER NOT NULL,
                     updated_at INTEGER NOT NULL
                 );",
            )
            .unwrap();
        drop(connection);

        let store = SettingsStore::at(&path).unwrap();
        let settings = store.load().unwrap();
        assert_eq!(settings.seeding_policy, "follow_client");
        assert_eq!(settings.preferred_region, "USA");
        assert_eq!(settings.version_preference, "latest");
        let connection = Connection::open(&path).unwrap();
        assert!(column_exists(&connection, "download_jobs", "post_import_action").unwrap());
        assert!(column_exists(&connection, "download_jobs", "download_plan").unwrap());
        assert!(column_exists(&connection, "app_settings", "preferred_region").unwrap());
        assert!(column_exists(&connection, "app_settings", "version_preference").unwrap());
        let prepared_table: Option<i64> = connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='prepared_game_installs'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(prepared_table, Some(1));
        for table in ["game_activity", "play_sessions"] {
            let migrated: Option<i64> = connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .optional()
                .unwrap();
            assert_eq!(migrated, Some(1), "missing migrated table {table}");
        }
    }

    #[test]
    fn concurrent_state_initialization_does_not_race_the_schema_migration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers = (0..8)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let store = SettingsStore::at(path).unwrap();
                    store.load().unwrap()
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            assert_eq!(worker.join().unwrap(), AppSettings::default());
        }
    }
}
