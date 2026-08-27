use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, params};

use crate::catalog;

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
    pub created_at: i64,
    pub updated_at: i64,
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
            created_at: now,
            updated_at: now,
        }
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
                        download_entire_torrent, file_link_mode
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
                 download_entire_torrent, file_link_mode
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
                 file_link_mode=excluded.file_link_mode",
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
                 created_at, updated_at
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                 ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20
             ) ON CONFLICT(id) DO UPDATE SET
                 state=excluded.state, progress=excluded.progress,
                 download_speed=excluded.download_speed,
                 downloaded_bytes=excluded.downloaded_bytes,
                 total_bytes=excluded.total_bytes, message=excluded.message,
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
                job.created_at,
                job.updated_at,
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
                    total_bytes, message, created_at, updated_at
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
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
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

    fn connection(&self) -> Result<Connection> {
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
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL
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
             ON user_collection_games(game_uid);",
    )?;
    if !column_exists(connection, "download_jobs", "launchbox_db_id")? {
        connection.execute(
            "ALTER TABLE download_jobs ADD COLUMN launchbox_db_id INTEGER NOT NULL DEFAULT 0",
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

fn validate_collection_description(description: &str) -> Result<String> {
    let description = description.trim();
    if description.chars().count() > 1000 {
        bail!("collection description must be at most 1000 characters");
    }
    Ok(description.to_owned())
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
