use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};
use crc32fast::Hasher as Crc32;
use md5::Md5;
use rars::{ArchiveFamily, ArchiveReadOptions, ArchiveReader};
use rusqlite::{Connection, params};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::catalog;
use crate::settings::{self, SettingsStore};

const FALLBACK_ROM_EXTENSIONS: &[&str] = &[
    "7z", "a26", "a52", "a78", "atr", "atx", "bin", "car", "cas", "chd", "cia", "ciso", "col",
    "cso", "cue", "dsi", "fds", "fig", "gba", "gbc", "gcm", "gcz", "gen", "gg", "int", "iso",
    "j64", "jag", "lnx", "md", "m3u", "n64", "nds", "nes", "ngc", "ngp", "pbp", "pce", "pkg",
    "rar", "rvz", "sfc", "sgb", "sgx", "smc", "smd", "sms", "swc", "unf", "unh", "unif", "v64",
    "vec", "wad", "wbfs", "ws", "wsc", "xex", "xfd", "z64", "zip", "32x", "3ds",
];
const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "7z", "rar"];
const MANUAL_MATCH_METHOD: &str = "manual user selection";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchIdentity {
    pub game_uid: String,
    pub title: String,
    pub platform: String,
    pub launchbox_db_id: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchState {
    Exact(MatchIdentity),
    Reviewed(MatchIdentity),
    Ambiguous(usize),
    Unmatched,
    InventoryOnly,
    Error(String),
}

impl MatchState {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Exact(_) => "exact",
            Self::Reviewed(_) => "reviewed",
            Self::Ambiguous(_) => "ambiguous",
            Self::Unmatched => "unmatched",
            Self::InventoryOnly => "inventory_only",
            Self::Error(_) => "error",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualMatchCandidate {
    pub identity: MatchIdentity,
    pub matched_name: String,
}

#[derive(Clone, Debug)]
pub struct ScanResult {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub file_name: String,
    pub archive_member: String,
    pub archive_member_count: usize,
    pub display_title: String,
    pub platform: String,
    pub file_size: u64,
    pub modified_unix_ns: Option<i64>,
    pub crc32: String,
    pub md5: String,
    pub sha1: String,
    pub match_state: MatchState,
    pub match_method: String,
    pub selected: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ScanProgress {
    pub total_files: usize,
    pub scanned_files: usize,
    pub current_file: String,
    pub cache_reused_files: usize,
    pub content_read_files: usize,
}

#[derive(Clone, Debug)]
pub struct ScanOutput {
    pub root: PathBuf,
    pub platform_hint: String,
    pub extension_filter: Vec<String>,
    pub checksums_enabled: bool,
    pub results: Vec<ScanResult>,
    pub walk_errors: usize,
    pub cancelled: bool,
    pub cache_reused_files: usize,
    pub content_read_files: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RomImportProfile {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub platform_hint: String,
    pub extensions: Vec<String>,
    pub checksums_enabled: bool,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RomScanRunContext {
    pub batch_id: String,
    pub batch_position: usize,
    pub batch_total: usize,
    pub profile_id: String,
    pub profile_name: String,
    pub root: PathBuf,
    pub platform_hint: String,
    pub extensions: Vec<String>,
    pub checksums_enabled: bool,
    pub started_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RomScanRun {
    pub id: String,
    pub batch_id: String,
    pub batch_position: usize,
    pub batch_total: usize,
    pub profile_id: String,
    pub profile_name: String,
    pub root: PathBuf,
    pub platform_hint: String,
    pub extensions: Vec<String>,
    pub checksums_enabled: bool,
    pub outcome: String,
    pub started_at: i64,
    pub finished_at: i64,
    pub total_files: usize,
    pub exact_matches: usize,
    pub reviewed_matches: usize,
    pub other_results: usize,
    pub walk_errors: usize,
    pub cache_reused_files: usize,
    pub content_read_files: usize,
    pub error_text: String,
}

impl RomScanRunContext {
    pub fn one_time(
        root: PathBuf,
        platform_hint: String,
        extensions: Vec<String>,
        checksums_enabled: bool,
    ) -> Self {
        Self {
            batch_id: String::new(),
            batch_position: 0,
            batch_total: 0,
            profile_id: String::new(),
            profile_name: "One-time scan".to_owned(),
            root,
            platform_hint,
            extensions,
            checksums_enabled,
            started_at: settings::unix_timestamp(),
        }
    }

    pub fn for_profile(
        profile: &RomImportProfile,
        batch_id: String,
        batch_position: usize,
        batch_total: usize,
    ) -> Self {
        Self {
            batch_id,
            batch_position,
            batch_total,
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            root: profile.root.clone(),
            platform_hint: profile.platform_hint.clone(),
            extensions: profile.extensions.clone(),
            checksums_enabled: profile.checksums_enabled,
            started_at: settings::unix_timestamp(),
        }
    }
}

pub fn normalize_extension_filter(input: &str) -> Result<Vec<String>> {
    let mut extensions = input
        .split(|character: char| character == ',' || character == ';' || character.is_whitespace())
        .filter_map(|value| {
            let value = value
                .trim()
                .trim_start_matches("*.")
                .trim_start_matches('.');
            (!value.is_empty()).then(|| value.to_ascii_lowercase())
        })
        .collect::<Vec<_>>();
    if extensions.len() > 128 {
        bail!("a ROM scan profile can contain at most 128 extensions");
    }
    for extension in &extensions {
        if extension.len() > 24
            || !extension
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+'))
        {
            bail!(
                "ROM extension {extension:?} must contain 1-24 portable letters, numbers, _, -, or +"
            );
        }
    }
    extensions.sort_unstable();
    extensions.dedup();
    Ok(extensions)
}

pub fn format_extension_filter(extensions: &[String]) -> String {
    extensions
        .iter()
        .map(|extension| format!(".{extension}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Clone, Debug)]
struct Candidate {
    identity: MatchIdentity,
    md5: String,
}

struct MatchIndex {
    by_sha1: HashMap<String, Vec<Candidate>>,
    by_md5: HashMap<String, Vec<Candidate>>,
    extension_platforms: HashMap<String, Vec<String>>,
    extensions: HashSet<String>,
}

#[derive(Clone, Debug)]
enum ArchiveInspection {
    Empty,
    Members(Vec<ArchiveMemberInspection>),
}

#[derive(Clone, Debug)]
struct ArchiveMemberInspection {
    member_name: String,
    extension: String,
    size: u64,
    digests: Option<Digests>,
    error: Option<String>,
}

struct PreparedScanSource {
    identity_file_name: String,
    identity_file_size: u64,
    identity_extension: String,
    archive_member: String,
    archive_member_count: usize,
    digests: Option<Digests>,
    error: Option<String>,
}

struct HashCache {
    connection: Connection,
    root_id: String,
    seen_paths: HashSet<(String, Vec<u8>)>,
}

impl HashCache {
    fn new(connection: Connection, root: &Path) -> Self {
        let encoded_root = encode_path(root);
        let root_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, &encoded_root.bytes).to_string();
        Self {
            connection,
            root_id,
            seen_paths: HashSet::new(),
        }
    }

    fn mark_seen(&mut self, path: &Path) {
        let encoded = encode_path(path);
        self.seen_paths
            .insert((encoded.encoding.to_owned(), encoded.bytes));
    }

    fn load_sources(
        &mut self,
        path: &Path,
        metadata: &fs::Metadata,
        recognized_extensions: &HashSet<String>,
        archive: bool,
    ) -> Result<Option<Vec<PreparedScanSource>>> {
        let encoded = encode_path(path);
        self.seen_paths
            .insert((encoded.encoding.to_owned(), encoded.bytes.clone()));
        let fingerprint = metadata_fingerprint(metadata);
        let mut statement = self.connection.prepare(
            "SELECT archive_member, archive_member_count, member_size,
                    member_extension, crc32, md5, sha1
             FROM local_rom_hash_cache
             WHERE root_id=?1 AND path_encoding=?2 AND path_bytes=?3
               AND container_size=?4 AND metadata_fingerprint=?5
             ORDER BY archive_member",
        )?;
        let rows = statement.query_map(
            params![
                self.root_id,
                encoded.encoding,
                encoded.bytes,
                i64::try_from(metadata.len()).unwrap_or(i64::MAX),
                fingerprint,
            ],
            |row| {
                Ok(PreparedScanSource {
                    identity_file_name: row.get(0)?,
                    archive_member: row.get(0)?,
                    archive_member_count: row.get::<_, usize>(1)?,
                    identity_file_size: row.get::<_, u64>(2)?,
                    identity_extension: row.get(3)?,
                    digests: Some(Digests {
                        crc32: row.get(4)?,
                        md5: row.get(5)?,
                        sha1: row.get(6)?,
                    }),
                    error: None,
                })
            },
        )?;
        let mut sources = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        if sources.is_empty() {
            return Ok(None);
        }

        let valid = if archive {
            let member_count = sources[0].archive_member_count;
            member_count > 0
                && member_count == sources.len()
                && sources.iter().all(|source| {
                    source.archive_member_count == member_count
                        && !source.archive_member.is_empty()
                        && recognized_archive_member(&source.archive_member, recognized_extensions)
                            .is_some_and(|(_, extension)| extension == source.identity_extension)
                        && source.digests.as_ref().is_some_and(valid_digests)
                })
        } else {
            sources.len() == 1
                && sources[0].archive_member.is_empty()
                && sources[0].archive_member_count == 0
                && sources[0].identity_file_size == metadata.len()
                && sources[0].digests.as_ref().is_some_and(valid_digests)
        };
        if !valid {
            return Ok(None);
        }
        if !archive {
            sources[0].identity_file_name = path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default();
        }
        Ok(Some(sources))
    }

    fn save_sources(
        &mut self,
        path: &Path,
        metadata: &fs::Metadata,
        sources: &[PreparedScanSource],
    ) -> Result<()> {
        if sources.is_empty()
            || sources
                .iter()
                .any(|source| source.error.is_some() || source.digests.is_none())
        {
            return Ok(());
        }
        let encoded = encode_path(path);
        self.seen_paths
            .insert((encoded.encoding.to_owned(), encoded.bytes.clone()));
        let fingerprint = metadata_fingerprint(metadata);
        let container_size = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
        let now = settings::unix_timestamp();
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM local_rom_hash_cache
             WHERE root_id=?1 AND path_encoding=?2 AND path_bytes=?3",
            params![self.root_id, encoded.encoding, encoded.bytes],
        )?;
        for source in sources {
            let digests = source
                .digests
                .as_ref()
                .context("cacheable ROM source lost its content digests")?;
            transaction.execute(
                "INSERT INTO local_rom_hash_cache (
                     root_id, path_display, path_bytes, path_encoding,
                     container_size, metadata_fingerprint, archive_member,
                     archive_member_count, member_size, member_extension,
                     crc32, md5, sha1, updated_at
                 ) VALUES (
                     ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                     ?11, ?12, ?13, ?14
                 )",
                params![
                    self.root_id,
                    encoded.display,
                    encoded.bytes,
                    encoded.encoding,
                    container_size,
                    fingerprint,
                    source.archive_member,
                    i64::try_from(source.archive_member_count).unwrap_or(i64::MAX),
                    i64::try_from(source.identity_file_size).unwrap_or(i64::MAX),
                    source.identity_extension,
                    digests.crc32,
                    digests.md5,
                    digests.sha1,
                    now,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        let cached_paths = {
            let mut statement = self.connection.prepare(
                "SELECT DISTINCT path_encoding, path_bytes
                 FROM local_rom_hash_cache WHERE root_id=?1",
            )?;
            let rows = statement.query_map(params![self.root_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let stale_paths = cached_paths
            .into_iter()
            .filter(|path| !self.seen_paths.contains(path))
            .collect::<Vec<_>>();
        if stale_paths.is_empty() {
            return Ok(());
        }
        let transaction = self.connection.transaction()?;
        for (encoding, bytes) in stale_paths {
            transaction.execute(
                "DELETE FROM local_rom_hash_cache
                 WHERE root_id=?1 AND path_encoding=?2 AND path_bytes=?3",
                params![self.root_id, encoding, bytes],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}

fn valid_digests(digests: &Digests) -> bool {
    valid_hex_digest(&digests.crc32, 8)
        && valid_hex_digest(&digests.md5, 32)
        && valid_hex_digest(&digests.sha1, 40)
}

fn valid_hex_digest(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(unix)]
fn metadata_fingerprint(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;

    format!(
        "unix-v1:{}:{}:{}:{}:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

#[cfg(windows)]
fn metadata_fingerprint(metadata: &fs::Metadata) -> String {
    use std::os::windows::fs::MetadataExt;

    format!(
        "windows-v1:{:?}:{:?}:{}:{}:{}",
        metadata.volume_serial_number(),
        metadata.file_index(),
        metadata.file_size(),
        metadata.creation_time(),
        metadata.last_write_time(),
    )
}

#[cfg(not(any(unix, windows)))]
fn metadata_fingerprint(metadata: &fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    format!("portable-v1:{}:{modified}", metadata.len())
}

pub fn load_platform_names(discovery_path: &Path) -> Result<Vec<String>> {
    let connection = catalog::open_read_only(discovery_path, "Lunchbox discovery database")?;
    let mut statement =
        connection.prepare("SELECT name FROM platforms ORDER BY name COLLATE NOCASE")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn load_import_profiles(state_path: &Path) -> Result<Vec<RomImportProfile>> {
    let store = SettingsStore::at(state_path)?;
    let connection = store.connection()?;
    let mut statement = connection.prepare(
        "SELECT id, name, path_display, path_bytes, path_encoding,
                platform_hint, extensions_json, checksums_enabled, updated_at
         FROM rom_import_profiles
         ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([], |row| {
        let display = row.get::<_, String>(2)?;
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            decode_path(row.get(3)?, &row.get::<_, String>(4)?, display),
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, bool>(7)?,
            row.get::<_, i64>(8)?,
        ))
    })?;
    let mut profiles = Vec::new();
    for row in rows {
        let (id, name, root, platform_hint, extensions_json, checksums_enabled, updated_at) = row?;
        let raw_extensions = serde_json::from_str::<Vec<String>>(&extensions_json)
            .with_context(|| format!("decoding ROM scan profile {name:?}"))?;
        let extensions = normalize_extension_filter(&raw_extensions.join(","))?;
        if extensions != raw_extensions {
            bail!("ROM scan profile {name:?} has a non-canonical extension list");
        }
        validate_import_profile(&name, &root, &platform_hint, &extensions)?;
        profiles.push(RomImportProfile {
            id,
            name,
            root,
            platform_hint,
            extensions,
            checksums_enabled,
            updated_at,
        });
    }
    Ok(profiles)
}

pub fn save_import_profile(
    state_path: &Path,
    name: &str,
    root: &Path,
    platform_hint: &str,
    extensions_input: &str,
    checksums_enabled: bool,
) -> Result<RomImportProfile> {
    let name = name.trim();
    let platform_hint = platform_hint.trim();
    let extensions = normalize_extension_filter(extensions_input)?;
    validate_import_profile(name, root, platform_hint, &extensions)?;
    let normalized_name = name.to_lowercase();
    let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, normalized_name.as_bytes()).to_string();
    let encoded = encode_path(root);
    let updated_at = settings::unix_timestamp();
    let extensions_json = serde_json::to_string(&extensions)?;
    let store = SettingsStore::at(state_path)?;
    store.connection()?.execute(
        "INSERT INTO rom_import_profiles (
             id, name, path_display, path_bytes, path_encoding,
             platform_hint, extensions_json, checksums_enabled, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(name) DO UPDATE SET
             name=excluded.name, path_display=excluded.path_display,
             path_bytes=excluded.path_bytes,
             path_encoding=excluded.path_encoding, platform_hint=excluded.platform_hint,
             extensions_json=excluded.extensions_json,
             checksums_enabled=excluded.checksums_enabled,
             updated_at=excluded.updated_at",
        params![
            id,
            name,
            encoded.display,
            encoded.bytes,
            encoded.encoding,
            platform_hint,
            extensions_json,
            checksums_enabled,
            updated_at,
        ],
    )?;
    load_import_profiles(state_path)?
        .into_iter()
        .find(|profile| profile.id == id)
        .context("saved ROM scan profile was not readable")
}

pub fn delete_import_profile(state_path: &Path, id: &str) -> Result<()> {
    if Uuid::parse_str(id).is_err() {
        bail!("a valid stable ROM scan profile ID is required");
    }
    let store = SettingsStore::at(state_path)?;
    let removed = store
        .connection()?
        .execute("DELETE FROM rom_import_profiles WHERE id=?1", [id])?;
    if removed != 1 {
        bail!("the selected ROM scan profile no longer exists");
    }
    Ok(())
}

pub fn record_scan_run(
    state_path: &Path,
    context: &RomScanRunContext,
    scan_result: Result<&ScanOutput, &str>,
) -> Result<RomScanRun> {
    validate_scan_run_context(context)?;
    let finished_at = settings::unix_timestamp().max(context.started_at);
    let (
        outcome,
        total_files,
        exact_matches,
        reviewed_matches,
        other_results,
        walk_errors,
        cache_reused_files,
        content_read_files,
        error_text,
    ) = match scan_result {
        Ok(output) => {
            let total_files = output.results.len();
            let exact_matches = output
                .results
                .iter()
                .filter(|result| matches!(result.match_state, MatchState::Exact(_)))
                .count();
            let reviewed_matches = output
                .results
                .iter()
                .filter(|result| matches!(result.match_state, MatchState::Reviewed(_)))
                .count();
            (
                if output.cancelled {
                    "cancelled"
                } else {
                    "completed"
                },
                total_files,
                exact_matches,
                reviewed_matches,
                total_files
                    .saturating_sub(exact_matches)
                    .saturating_sub(reviewed_matches),
                output.walk_errors,
                output.cache_reused_files,
                output.content_read_files,
                String::new(),
            )
        }
        Err(error) => (
            "failed",
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            error.chars().take(4096).collect(),
        ),
    };
    let run = RomScanRun {
        id: Uuid::new_v4().to_string(),
        batch_id: context.batch_id.clone(),
        batch_position: context.batch_position,
        batch_total: context.batch_total,
        profile_id: context.profile_id.clone(),
        profile_name: context.profile_name.clone(),
        root: context.root.clone(),
        platform_hint: context.platform_hint.clone(),
        extensions: context.extensions.clone(),
        checksums_enabled: context.checksums_enabled,
        outcome: outcome.to_owned(),
        started_at: context.started_at,
        finished_at,
        total_files,
        exact_matches,
        reviewed_matches,
        other_results,
        walk_errors,
        cache_reused_files,
        content_read_files,
        error_text,
    };
    let encoded = encode_path(&run.root);
    let extensions_json = serde_json::to_string(&run.extensions)?;
    let mut connection = SettingsStore::at(state_path)?.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO rom_scan_runs (
             id, batch_id, batch_position, batch_total, profile_id, profile_name,
             path_display, path_bytes, path_encoding, platform_hint,
             extensions_json, checksums_enabled, outcome, started_at, finished_at,
             total_files, exact_matches, reviewed_matches, other_results,
             walk_errors, cache_reused_files, content_read_files, error_text
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
             ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
         )",
        params![
            run.id,
            run.batch_id,
            usize_to_i64(run.batch_position),
            usize_to_i64(run.batch_total),
            run.profile_id,
            run.profile_name,
            encoded.display,
            encoded.bytes,
            encoded.encoding,
            run.platform_hint,
            extensions_json,
            run.checksums_enabled,
            run.outcome,
            run.started_at,
            run.finished_at,
            usize_to_i64(run.total_files),
            usize_to_i64(run.exact_matches),
            usize_to_i64(run.reviewed_matches),
            usize_to_i64(run.other_results),
            usize_to_i64(run.walk_errors),
            usize_to_i64(run.cache_reused_files),
            usize_to_i64(run.content_read_files),
            run.error_text,
        ],
    )?;
    transaction.execute(
        "DELETE FROM rom_scan_runs
         WHERE id IN (
             SELECT id FROM rom_scan_runs
             ORDER BY finished_at DESC, id DESC
             LIMIT -1 OFFSET 200
         )",
        [],
    )?;
    transaction.commit()?;
    Ok(run)
}

pub fn load_scan_runs(state_path: &Path) -> Result<Vec<RomScanRun>> {
    let connection = SettingsStore::at(state_path)?.connection()?;
    let mut statement = connection.prepare(
        "SELECT id, batch_id, batch_position, batch_total, profile_id, profile_name,
                path_display, path_bytes, path_encoding, platform_hint,
                extensions_json, checksums_enabled, outcome, started_at, finished_at,
                total_files, exact_matches, reviewed_matches, other_results,
                walk_errors, cache_reused_files, content_read_files, error_text
         FROM rom_scan_runs
         ORDER BY finished_at DESC, id DESC
         LIMIT 200",
    )?;
    let rows = statement.query_map([], |row| {
        let display = row.get::<_, String>(6)?;
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            decode_path(row.get(7)?, &row.get::<_, String>(8)?, display),
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, bool>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, i64>(13)?,
            row.get::<_, i64>(14)?,
            row.get::<_, i64>(15)?,
            row.get::<_, i64>(16)?,
            row.get::<_, i64>(17)?,
            row.get::<_, i64>(18)?,
            row.get::<_, i64>(19)?,
            row.get::<_, i64>(20)?,
            row.get::<_, i64>(21)?,
            row.get::<_, String>(22)?,
        ))
    })?;
    let mut runs = Vec::new();
    for row in rows {
        let (
            id,
            batch_id,
            batch_position,
            batch_total,
            profile_id,
            profile_name,
            root,
            platform_hint,
            extensions_json,
            checksums_enabled,
            outcome,
            started_at,
            finished_at,
            total_files,
            exact_matches,
            reviewed_matches,
            other_results,
            walk_errors,
            cache_reused_files,
            content_read_files,
            error_text,
        ) = row?;
        let extensions = serde_json::from_str::<Vec<String>>(&extensions_json)
            .with_context(|| format!("decoding ROM scan history {id}"))?;
        let run = RomScanRun {
            id,
            batch_id,
            batch_position: nonnegative_usize(batch_position)?,
            batch_total: nonnegative_usize(batch_total)?,
            profile_id,
            profile_name,
            root,
            platform_hint,
            extensions,
            checksums_enabled,
            outcome,
            started_at,
            finished_at,
            total_files: nonnegative_usize(total_files)?,
            exact_matches: nonnegative_usize(exact_matches)?,
            reviewed_matches: nonnegative_usize(reviewed_matches)?,
            other_results: nonnegative_usize(other_results)?,
            walk_errors: nonnegative_usize(walk_errors)?,
            cache_reused_files: nonnegative_usize(cache_reused_files)?,
            content_read_files: nonnegative_usize(content_read_files)?,
            error_text,
        };
        validate_loaded_scan_run(&run)?;
        runs.push(run);
    }
    Ok(runs)
}

fn validate_scan_run_context(context: &RomScanRunContext) -> Result<()> {
    validate_import_profile(
        &context.profile_name,
        &context.root,
        &context.platform_hint,
        &context.extensions,
    )?;
    if !context.profile_id.is_empty() && Uuid::parse_str(&context.profile_id).is_err() {
        bail!("scan history requires a valid stable profile ID");
    }
    let batch_valid =
        (context.batch_id.is_empty() && context.batch_position == 0 && context.batch_total == 0)
            || (Uuid::parse_str(&context.batch_id).is_ok()
                && context.batch_position > 0
                && context.batch_position <= context.batch_total);
    if !batch_valid {
        bail!("scan history requires a coherent batch identity and position");
    }
    Ok(())
}

fn validate_loaded_scan_run(run: &RomScanRun) -> Result<()> {
    let context = RomScanRunContext {
        batch_id: run.batch_id.clone(),
        batch_position: run.batch_position,
        batch_total: run.batch_total,
        profile_id: run.profile_id.clone(),
        profile_name: run.profile_name.clone(),
        root: run.root.clone(),
        platform_hint: run.platform_hint.clone(),
        extensions: run.extensions.clone(),
        checksums_enabled: run.checksums_enabled,
        started_at: run.started_at,
    };
    validate_scan_run_context(&context)?;
    if !matches!(run.outcome.as_str(), "completed" | "cancelled" | "failed")
        || run.finished_at < run.started_at
        || run.exact_matches + run.reviewed_matches + run.other_results != run.total_files
        || run.error_text.chars().count() > 4096
    {
        bail!("ROM scan history row {} is internally inconsistent", run.id);
    }
    Ok(())
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn nonnegative_usize(value: i64) -> Result<usize> {
    usize::try_from(value).context("ROM scan history contains an invalid negative count")
}

fn validate_import_profile(
    name: &str,
    root: &Path,
    platform_hint: &str,
    extensions: &[String],
) -> Result<()> {
    if name.is_empty() || name.chars().count() > 80 || name.contains('\0') {
        bail!("a ROM scan profile name must contain 1-80 safe characters");
    }
    if root.as_os_str().is_empty() || !root.is_absolute() {
        bail!("a ROM scan profile requires an absolute local folder");
    }
    if platform_hint.chars().count() > 512 || platform_hint.contains('\0') {
        bail!("a ROM scan profile platform must contain at most 512 safe characters");
    }
    let encoded = encode_path(root);
    if encoded.display.is_empty() || encoded.display.len() > 8192 || encoded.bytes.len() > 32768 {
        bail!("the ROM scan profile folder path is too long to store safely");
    }
    if serde_json::to_string(extensions)?.len() > 8192 {
        bail!("the ROM scan profile extension list is too large");
    }
    Ok(())
}

pub fn search_manual_match_candidates(
    discovery_path: &Path,
    query: &str,
    platform: &str,
) -> Result<Vec<ManualMatchCandidate>> {
    let query = query.trim();
    if query.chars().count() < 2 {
        return Ok(Vec::new());
    }
    let platform = platform.trim();
    let connection = catalog::open_read_only(discovery_path, "Lunchbox discovery database")?;
    let mut statement = connection.prepare(
        "WITH candidates AS (
             SELECT g.id, g.title, p.name,
                    coalesce(g.launchbox_db_id, 0) AS launchbox_db_id,
                    '' AS matched_name
             FROM games g JOIN platforms p ON p.id=g.platform_id
             WHERE (?2='' OR p.name=?2)
               AND instr(lower(g.title), lower(?1)) > 0
             UNION ALL
             SELECT g.id, g.title, p.name,
                    coalesce(g.launchbox_db_id, 0) AS launchbox_db_id,
                    a.alternate_name
             FROM game_alternate_names a
             JOIN games g ON g.launchbox_db_id=a.launchbox_db_id
             JOIN platforms p ON p.id=g.platform_id
             WHERE (?2='' OR p.name=?2)
               AND instr(lower(a.alternate_name), lower(?1)) > 0
         )
         SELECT id, title, name, launchbox_db_id, matched_name
         FROM candidates
         ORDER BY CASE
                    WHEN lower(title)=lower(?1) THEN 0
                    WHEN lower(matched_name)=lower(?1) THEN 1
                    WHEN instr(lower(title), lower(?1))=1 THEN 2
                    ELSE 3
                  END,
                  length(title), title COLLATE NOCASE, name COLLATE NOCASE,
                  matched_name COLLATE NOCASE
         LIMIT 250",
    )?;
    let rows = statement.query_map(params![query, platform], |row| {
        Ok(ManualMatchCandidate {
            identity: MatchIdentity {
                game_uid: row.get(0)?,
                title: row.get(1)?,
                platform: row.get(2)?,
                launchbox_db_id: row.get(3)?,
            },
            matched_name: row.get(4)?,
        })
    })?;
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for candidate in rows {
        let candidate = candidate?;
        if seen.insert(candidate.identity.game_uid.clone()) {
            candidates.push(candidate);
            if candidates.len() == 100 {
                break;
            }
        }
    }
    Ok(candidates)
}

pub fn apply_manual_match(result: &mut ScanResult, candidate: &ManualMatchCandidate) -> Result<()> {
    if matches!(result.match_state, MatchState::Error(_)) {
        bail!("an unreadable ROM cannot be linked to a catalog game");
    }
    if candidate.identity.game_uid.trim().is_empty()
        || candidate.identity.title.trim().is_empty()
        || candidate.identity.platform.trim().is_empty()
        || candidate.identity.launchbox_db_id < 0
    {
        bail!("manual match candidate has an invalid catalog identity");
    }
    result.platform.clone_from(&candidate.identity.platform);
    result.match_state = MatchState::Reviewed(candidate.identity.clone());
    result.match_method = MANUAL_MATCH_METHOD.to_owned();
    result.selected = true;
    Ok(())
}

fn load_match_index(discovery_path: &Path) -> Result<MatchIndex> {
    let connection = catalog::open_read_only(discovery_path, "Lunchbox discovery database")?;
    let mut by_sha1: HashMap<String, Vec<Candidate>> = HashMap::new();
    let mut by_md5: HashMap<String, Vec<Candidate>> = HashMap::new();
    let mut statement = connection.prepare(
        "SELECT g.id, g.title, p.name, coalesce(g.launchbox_db_id, 0),
                upper(coalesce(g.libretro_sha1, '')), upper(coalesce(g.libretro_md5, ''))
         FROM games g JOIN platforms p ON p.id=g.platform_id
         WHERE nullif(g.libretro_sha1, '') IS NOT NULL
            OR nullif(g.libretro_md5, '') IS NOT NULL",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            MatchIdentity {
                game_uid: row.get(0)?,
                title: row.get(1)?,
                platform: row.get(2)?,
                launchbox_db_id: row.get(3)?,
            },
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    for row in rows {
        let (identity, sha1, md5) = row?;
        let candidate = Candidate {
            identity,
            md5: md5.clone(),
        };
        if !sha1.is_empty() {
            by_sha1.entry(sha1).or_default().push(candidate.clone());
        }
        if !md5.is_empty() {
            by_md5.entry(md5).or_default().push(candidate);
        }
    }

    let mut extensions = FALLBACK_ROM_EXTENSIONS
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<HashSet<_>>();
    let mut extension_platforms: HashMap<String, Vec<String>> = HashMap::new();
    let mut platform_statement =
        connection.prepare("SELECT name, coalesce(file_extensions, '') FROM platforms")?;
    let platforms = platform_statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for platform in platforms {
        let (name, values) = platform?;
        for extension in values.split(|character: char| {
            character == ',' || character == ';' || character.is_whitespace()
        }) {
            let extension = extension
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase();
            if extension.is_empty() {
                continue;
            }
            extensions.insert(extension.clone());
            let names = extension_platforms.entry(extension).or_default();
            if !names.contains(&name) {
                names.push(name.clone());
            }
        }
    }

    Ok(MatchIndex {
        by_sha1,
        by_md5,
        extension_platforms,
        extensions,
    })
}

pub(crate) struct PreparedScanner {
    index: MatchIndex,
}

impl PreparedScanner {
    pub(crate) fn new(discovery_path: &Path) -> Result<Self> {
        Ok(Self {
            index: load_match_index(discovery_path)?,
        })
    }

    pub(crate) fn scan(
        &self,
        root: &Path,
        platform_hint: &str,
        checksums_enabled: bool,
        cancelled: &AtomicBool,
        progress: Arc<dyn Fn(ScanProgress) + Send + Sync>,
    ) -> Result<ScanOutput> {
        scan_directory_with_index(
            &self.index,
            root,
            platform_hint,
            &[],
            checksums_enabled,
            cancelled,
            progress,
            None,
        )
    }

    pub(crate) fn scan_cached(
        &self,
        state_path: &Path,
        root: &Path,
        platform_hint: &str,
        checksums_enabled: bool,
        cancelled: &AtomicBool,
        progress: Arc<dyn Fn(ScanProgress) + Send + Sync>,
    ) -> Result<ScanOutput> {
        let store = SettingsStore::at(state_path)?;
        scan_directory_with_index(
            &self.index,
            root,
            platform_hint,
            &[],
            checksums_enabled,
            cancelled,
            progress,
            Some(store.connection()?),
        )
    }

    pub(crate) fn scan_cached_with_extensions(
        &self,
        state_path: &Path,
        root: &Path,
        platform_hint: &str,
        extension_filter: &[String],
        checksums_enabled: bool,
        cancelled: &AtomicBool,
        progress: Arc<dyn Fn(ScanProgress) + Send + Sync>,
    ) -> Result<ScanOutput> {
        let store = SettingsStore::at(state_path)?;
        scan_directory_with_index(
            &self.index,
            root,
            platform_hint,
            extension_filter,
            checksums_enabled,
            cancelled,
            progress,
            Some(store.connection()?),
        )
    }
}

pub fn scan_directory(
    discovery_path: &Path,
    root: &Path,
    platform_hint: &str,
    checksums_enabled: bool,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(ScanProgress) + Send + Sync>,
) -> Result<ScanOutput> {
    PreparedScanner::new(discovery_path)?.scan(
        root,
        platform_hint,
        checksums_enabled,
        cancelled,
        progress,
    )
}

pub fn scan_directory_cached(
    discovery_path: &Path,
    state_path: &Path,
    root: &Path,
    platform_hint: &str,
    checksums_enabled: bool,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(ScanProgress) + Send + Sync>,
) -> Result<ScanOutput> {
    PreparedScanner::new(discovery_path)?.scan_cached(
        state_path,
        root,
        platform_hint,
        checksums_enabled,
        cancelled,
        progress,
    )
}

pub fn scan_directory_cached_with_extensions(
    discovery_path: &Path,
    state_path: &Path,
    root: &Path,
    platform_hint: &str,
    extension_filter: &[String],
    checksums_enabled: bool,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(ScanProgress) + Send + Sync>,
) -> Result<ScanOutput> {
    PreparedScanner::new(discovery_path)?.scan_cached_with_extensions(
        state_path,
        root,
        platform_hint,
        extension_filter,
        checksums_enabled,
        cancelled,
        progress,
    )
}

pub(crate) fn seed_hash_cache_ui_probe() -> Result<()> {
    let discovery_path = catalog::requested_path("--games-database", "LUNCHBOX_GAMES_DATABASE")
        .context(
            "the hash-cache UI probe requires an explicit --games-database or LUNCHBOX_GAMES_DATABASE",
        )?;
    let state_path = catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .context(
            "the hash-cache UI probe requires an explicit --state-database or LUNCHBOX_STATE_DATABASE",
        )?;
    let root = catalog::requested_path("--import-directory", "LUNCHBOX_IMPORT_DIRECTORY")
        .context(
            "the hash-cache UI probe requires an explicit --import-directory or LUNCHBOX_IMPORT_DIRECTORY",
        )?;
    let output = scan_directory_cached(
        &discovery_path,
        &state_path,
        &root,
        "",
        true,
        &AtomicBool::new(false),
        Arc::new(|_| {}),
    )?;
    if output.cancelled || output.results.is_empty() {
        bail!(
            "the hash-cache seed must complete with at least one readable ROM result; cancelled={} results={}",
            output.cancelled,
            output.results.len()
        );
    }
    if output.cache_reused_files + output.content_read_files == 0 {
        bail!("the hash-cache seed did not inspect any cacheable ROM files");
    }
    println!(
        "LUNCHBOX_HASH_CACHE_SEEDED results={} reused={} read={}",
        output.results.len(),
        output.cache_reused_files,
        output.content_read_files
    );
    Ok(())
}

pub(crate) fn seed_import_profile_ui_probe() -> Result<()> {
    let state_path = catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .context(
            "the import-profile UI probe requires an explicit --state-database or LUNCHBOX_STATE_DATABASE",
        )?;
    let root = catalog::requested_path("--import-directory", "LUNCHBOX_IMPORT_DIRECTORY")
        .context(
            "the import-profile UI probe requires an explicit --import-directory or LUNCHBOX_IMPORT_DIRECTORY",
        )?;
    let profile = save_import_profile(
        &state_path,
        "Nintendo cartridge shelf",
        &root,
        "Nintendo Entertainment System",
        ".fds, .nes, .zip",
        true,
    )?;
    println!(
        "LUNCHBOX_IMPORT_PROFILE_SEEDED id={} name={:?} root={:?} extensions={}",
        profile.id,
        profile.name,
        profile.root,
        format_extension_filter(&profile.extensions),
    );
    Ok(())
}

pub(crate) fn seed_import_profile_batch_ui_probe() -> Result<()> {
    let state_path = catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .context(
            "the import-profile batch UI probe requires an explicit --state-database or LUNCHBOX_STATE_DATABASE",
        )?;
    let root = catalog::requested_path("--import-directory", "LUNCHBOX_IMPORT_DIRECTORY")
        .context(
            "the import-profile batch UI probe requires an explicit --import-directory or LUNCHBOX_IMPORT_DIRECTORY",
        )?;
    let cartridge = save_import_profile(
        &state_path,
        "Nintendo cartridge shelf",
        &root,
        "Nintendo Entertainment System",
        ".nes",
        true,
    )?;
    let archives = save_import_profile(
        &state_path,
        "Nintendo archive shelf",
        &root,
        "Nintendo Entertainment System",
        ".zip",
        true,
    )?;
    println!(
        "LUNCHBOX_IMPORT_PROFILE_BATCH_SEEDED profiles={:?},{:?} root={:?}",
        cartridge.name, archives.name, root,
    );
    Ok(())
}

fn scan_directory_with_index(
    index: &MatchIndex,
    root: &Path,
    platform_hint: &str,
    extension_filter: &[String],
    checksums_enabled: bool,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(ScanProgress) + Send + Sync>,
    cache_connection: Option<Connection>,
) -> Result<ScanOutput> {
    let root = root
        .canonicalize()
        .with_context(|| format!("opening ROM directory {}", root.display()))?;
    if !root.is_dir() {
        bail!("ROM import path is not a directory: {}", root.display());
    }
    let mut hash_cache = checksums_enabled
        .then(|| cache_connection.map(|connection| HashCache::new(connection, &root)))
        .flatten();
    let extension_filter = extension_filter.iter().cloned().collect::<HashSet<_>>();
    let recognized_extensions = index
        .extensions
        .union(&extension_filter)
        .cloned()
        .collect::<HashSet<_>>();
    let mut archive_member_extensions = extension_filter
        .iter()
        .filter(|extension| !ARCHIVE_EXTENSIONS.contains(&extension.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    archive_member_extensions.sort_unstable();
    let archive_member_filter = archive_member_extensions
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let mut paths = Vec::new();
    let mut walk_errors = 0usize;
    for entry in WalkDir::new(&root).follow_links(false).into_iter() {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                walk_errors = walk_errors.saturating_add(1);
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let extension = entry
            .path()
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase);
        if let Some(extension) = extension
            && recognized_extensions.contains(&extension)
        {
            if let Some(cache) = hash_cache.as_mut() {
                cache.mark_seen(entry.path());
            }
            if extension_filter.is_empty() || extension_filter.contains(&extension) {
                paths.push(entry.into_path());
            }
        }
    }
    paths.sort();
    progress(ScanProgress {
        total_files: paths.len(),
        ..ScanProgress::default()
    });

    let mut results = Vec::with_capacity(paths.len());
    let mut cache_reused_files = 0usize;
    let mut content_read_files = 0usize;
    for (position, path) in paths.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let file_name = path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        let metadata = match path.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                results.push(error_result(&root, path, error.to_string()));
                report_progress_with_cache(
                    &progress,
                    paths.len(),
                    position,
                    file_name,
                    cache_reused_files,
                    content_read_files,
                );
                continue;
            }
        };
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let archive = matches!(extension.as_str(), "zip" | "7z" | "rar");
        let cached_sources = if checksums_enabled {
            match hash_cache.as_mut() {
                Some(cache) => {
                    cache.load_sources(path, &metadata, &recognized_extensions, archive)?
                }
                None => None,
            }
        } else {
            None
        };
        let cache_hit = cached_sources.is_some();
        if cache_hit {
            cache_reused_files = cache_reused_files.saturating_add(1);
        } else if checksums_enabled {
            content_read_files = content_read_files.saturating_add(1);
        }
        let archive_inspection = if cache_hit || !archive {
            None
        } else {
            let inspection = match extension.as_str() {
                "zip" => inspect_zip(path, &recognized_extensions, checksums_enabled, cancelled),
                "7z" => {
                    inspect_seven_zip(path, &recognized_extensions, checksums_enabled, cancelled)
                }
                "rar" => inspect_rar(path, &recognized_extensions, checksums_enabled, cancelled),
                _ => unreachable!("archive extensions are exhaustively matched"),
            };
            match inspection {
                Ok(Some(inspection)) => Some(inspection),
                Ok(None) => break,
                Err(error) => {
                    results.push(error_result(&root, path, error.to_string()));
                    report_progress_with_cache(
                        &progress,
                        paths.len(),
                        position,
                        file_name,
                        cache_reused_files,
                        content_read_files,
                    );
                    continue;
                }
            }
        };
        let modified_unix_ns = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .and_then(|value| i64::try_from(value.as_nanos()).ok());
        let mut sources = if let Some(sources) = cached_sources {
            sources
        } else {
            match archive_inspection {
                Some(ArchiveInspection::Empty) => vec![PreparedScanSource {
                    identity_file_name: file_name.clone(),
                    identity_file_size: metadata.len(),
                    identity_extension: extension.clone(),
                    archive_member: String::new(),
                    archive_member_count: 0,
                    digests: None,
                    error: Some(format!(
                        "{} contains no recognized ROM members",
                        archive_kind_label(path)
                    )),
                }],
                Some(ArchiveInspection::Members(members)) => {
                    let member_count = members.len();
                    members
                        .into_iter()
                        .map(|member| PreparedScanSource {
                            identity_file_name: member.member_name.clone(),
                            identity_file_size: member.size,
                            identity_extension: member.extension,
                            archive_member: member.member_name,
                            archive_member_count: member_count,
                            digests: member.digests,
                            error: member.error,
                        })
                        .collect()
                }
                None => {
                    let digests = if checksums_enabled {
                        match hash_file(path, cancelled) {
                            Ok(Some(digests)) => Some(digests),
                            Ok(None) => break,
                            Err(error) => {
                                results.push(error_result(&root, path, error.to_string()));
                                report_progress_with_cache(
                                    &progress,
                                    paths.len(),
                                    position,
                                    file_name,
                                    cache_reused_files,
                                    content_read_files,
                                );
                                continue;
                            }
                        }
                    } else {
                        None
                    };
                    vec![PreparedScanSource {
                        identity_file_name: file_name.clone(),
                        identity_file_size: metadata.len(),
                        identity_extension: extension.clone(),
                        archive_member: String::new(),
                        archive_member_count: 0,
                        digests,
                        error: None,
                    }]
                }
            }
        };
        if checksums_enabled && !cache_hit {
            if sources
                .iter()
                .all(|source| source.error.is_none() && source.digests.is_some())
            {
                let unchanged = path.metadata().is_ok_and(|current| {
                    current.len() == metadata.len()
                        && metadata_fingerprint(&current) == metadata_fingerprint(&metadata)
                });
                if !unchanged {
                    results.push(error_result(
                        &root,
                        path,
                        "ROM changed while it was being read; scan it again".to_owned(),
                    ));
                    report_progress_with_cache(
                        &progress,
                        paths.len(),
                        position,
                        file_name,
                        cache_reused_files,
                        content_read_files,
                    );
                    continue;
                }
            }
            if let Some(cache) = hash_cache.as_mut() {
                cache.save_sources(path, &metadata, &sources)?;
            }
        }
        if archive && !archive_member_filter.is_empty() {
            sources.retain(|source| archive_member_filter.contains(&source.identity_extension));
            if sources.is_empty() {
                sources.push(PreparedScanSource {
                    identity_file_name: file_name.clone(),
                    identity_file_size: metadata.len(),
                    identity_extension: extension.clone(),
                    archive_member: String::new(),
                    archive_member_count: 0,
                    digests: None,
                    error: Some(format!(
                        "{} contains no members matching this scan profile ({})",
                        archive_kind_label(path),
                        format_extension_filter(&archive_member_extensions)
                    )),
                });
            }
        }
        for source in sources {
            let inferred_platform = if platform_hint.is_empty() {
                index
                    .extension_platforms
                    .get(&source.identity_extension)
                    .and_then(|names| (names.len() == 1).then(|| names[0].clone()))
            } else {
                Some(platform_hint.to_owned())
            };
            let (match_state, mut match_method) = if let Some(error) = source.error {
                (MatchState::Error(error), "read error".to_owned())
            } else if let Some(digests) = &source.digests {
                match_digest(index, &digests.sha1, &digests.md5, platform_hint)
            } else {
                (MatchState::InventoryOnly, "not checked".to_owned())
            };
            if !source.archive_member.is_empty() {
                match_method = format!("{} member {match_method}", archive_kind_label(path));
            }
            let platform = match &match_state {
                MatchState::Exact(identity) | MatchState::Reviewed(identity) => {
                    identity.platform.clone()
                }
                _ => inferred_platform.unwrap_or_else(|| "Unassigned".to_owned()),
            };
            results.push(ScanResult {
                path: path.clone(),
                relative_path: path.strip_prefix(&root).unwrap_or(path).to_path_buf(),
                file_name: file_name.clone(),
                archive_member: source.archive_member,
                archive_member_count: source.archive_member_count,
                display_title: display_title(&source.identity_file_name),
                platform,
                file_size: source.identity_file_size,
                modified_unix_ns,
                crc32: source
                    .digests
                    .as_ref()
                    .map(|value| value.crc32.clone())
                    .unwrap_or_default(),
                md5: source
                    .digests
                    .as_ref()
                    .map(|value| value.md5.clone())
                    .unwrap_or_default(),
                sha1: source
                    .digests
                    .as_ref()
                    .map(|value| value.sha1.clone())
                    .unwrap_or_default(),
                selected: source.archive_member_count <= 1
                    && matches!(match_state, MatchState::Exact(_)),
                match_state,
                match_method,
            });
        }
        if position % 8 == 0 || position + 1 == paths.len() {
            report_progress_with_cache(
                &progress,
                paths.len(),
                position,
                file_name,
                cache_reused_files,
                content_read_files,
            );
        }
    }

    let scan_cancelled = cancelled.load(Ordering::Relaxed);
    if !scan_cancelled && let Some(cache) = hash_cache.as_mut() {
        cache.finish()?;
    }

    Ok(ScanOutput {
        root,
        platform_hint: platform_hint.to_owned(),
        extension_filter: {
            let mut values = extension_filter.into_iter().collect::<Vec<_>>();
            values.sort_unstable();
            values
        },
        checksums_enabled,
        results,
        walk_errors,
        cancelled: scan_cancelled,
        cache_reused_files,
        content_read_files,
    })
}

#[derive(Clone, Debug)]
struct PersistedManualMatch {
    file_size: u64,
    md5: String,
    sha1: String,
    identity: MatchIdentity,
}

pub fn restore_manual_matches(state_path: &Path, output: &mut ScanOutput) -> Result<usize> {
    if !output.checksums_enabled || !state_path.is_file() {
        return Ok(0);
    }
    let store = SettingsStore::at(state_path)?;
    let connection = store.connection()?;
    let mut statement = connection.prepare(
        "SELECT CASE WHEN length(coalesce(source_archive_path_bytes, x'')) > 0
                     THEN source_archive_path_bytes ELSE path_bytes END,
                archive_member, file_size, upper(md5), upper(sha1),
                game_uid, launchbox_db_id, matched_title, platform
         FROM local_rom_files
         WHERE included=1 AND match_method=?1
           AND nullif(game_uid, '') IS NOT NULL
           AND nullif(md5, '') IS NOT NULL
           AND nullif(sha1, '') IS NOT NULL",
    )?;
    let rows = statement.query_map([MANUAL_MATCH_METHOD], |row| {
        let file_size = row.get::<_, i64>(2)?;
        Ok((
            (row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?),
            PersistedManualMatch {
                file_size: u64::try_from(file_size).unwrap_or_default(),
                md5: row.get(3)?,
                sha1: row.get(4)?,
                identity: MatchIdentity {
                    game_uid: row.get(5)?,
                    launchbox_db_id: row.get(6)?,
                    title: row.get(7)?,
                    platform: row.get(8)?,
                },
            },
        ))
    })?;
    let mut persisted: HashMap<(Vec<u8>, String), Option<PersistedManualMatch>> = HashMap::new();
    for row in rows {
        let (key, manual_match) = row?;
        match persisted.entry(key) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(Some(manual_match));
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(None);
            }
        }
    }

    let mut restored = 0usize;
    for result in &mut output.results {
        let key = (
            encode_path(&result.path).bytes,
            result.archive_member.clone(),
        );
        let Some(Some(manual_match)) = persisted.get(&key) else {
            continue;
        };
        if manual_match.file_size != result.file_size
            || result.md5.is_empty()
            || result.sha1.is_empty()
            || !manual_match.md5.eq_ignore_ascii_case(&result.md5)
            || !manual_match.sha1.eq_ignore_ascii_case(&result.sha1)
        {
            continue;
        }
        result.platform.clone_from(&manual_match.identity.platform);
        result.match_state = MatchState::Reviewed(manual_match.identity.clone());
        result.match_method = MANUAL_MATCH_METHOD.to_owned();
        result.selected = true;
        restored = restored.saturating_add(1);
    }
    Ok(restored)
}

pub(crate) fn archive_import_probe() -> Result<String> {
    let discovery_path = catalog::requested_discovery_database_path()
        .context("--games-database is required for the archive import probe")?;
    let root = catalog::requested_path("--import-directory", "LUNCHBOX_IMPORT_DIRECTORY")
        .context("--import-directory is required for the archive import probe")?;
    let cancelled = AtomicBool::new(false);
    let output = scan_directory(
        &discovery_path,
        &root,
        "",
        true,
        &cancelled,
        Arc::new(|_| {}),
    )?;
    let result = output
        .results
        .iter()
        .find(|result| {
            result.archive_member_count == 1 && matches!(result.match_state, MatchState::Exact(_))
        })
        .context("no single-ROM archive member received an exact checksum identity")?;
    let MatchState::Exact(identity) = &result.match_state else {
        unreachable!("probe predicate requires an exact identity");
    };
    Ok(format!(
        "archive={:?} member={:?} title={:?} platform={:?} method={:?}",
        result.file_name,
        result.archive_member,
        identity.title,
        identity.platform,
        result.match_method
    ))
}

pub(crate) fn multi_archive_import_probe() -> Result<String> {
    let discovery_path = catalog::requested_discovery_database_path()
        .context("--games-database is required for the multi-archive import probe")?;
    let root = catalog::requested_path("--import-directory", "LUNCHBOX_IMPORT_DIRECTORY")
        .context("--import-directory is required for the multi-archive import probe")?;
    let state_path = catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .context("--state-database is required for the multi-archive import probe")?;
    let store = SettingsStore::at(&state_path)?;
    let cancelled = AtomicBool::new(false);
    let mut output = scan_directory(
        &discovery_path,
        &root,
        "",
        true,
        &cancelled,
        Arc::new(|_| {}),
    )?;
    let archive_path = output
        .results
        .iter()
        .find(|result| {
            result.archive_member_count > 1 && !matches!(result.match_state, MatchState::Error(_))
        })
        .map(|result| result.path.clone())
        .context("no archive with multiple safe recognized ROM members was found")?;
    let initial_selection_count = output
        .results
        .iter()
        .filter(|result| result.path == archive_path && result.selected)
        .count();
    if initial_selection_count != 0 {
        bail!(
            "multi-ROM archive unexpectedly selected {initial_selection_count} members without review"
        );
    }

    let mut selected_count = 0usize;
    let mut exact_count = 0usize;
    for result in &mut output.results {
        let selected = result.path == archive_path
            && result.archive_member_count > 1
            && !matches!(result.match_state, MatchState::Error(_));
        result.selected = selected;
        if selected {
            selected_count = selected_count.saturating_add(1);
            exact_count = exact_count.saturating_add(usize::from(matches!(
                result.match_state,
                MatchState::Exact(_)
            )));
        }
    }
    if selected_count < 2 {
        bail!("multi-ROM archive exposed only {selected_count} selectable members");
    }
    if exact_count == 0 {
        bail!("multi-ROM archive exposed no exact checksum identities");
    }

    let imported = commit_scan_to(store.path(), &output)?;
    if imported != selected_count {
        bail!("imported {imported} members after {selected_count} were reviewed");
    }
    let source = encode_path(&archive_path);
    let connection = store.connection()?;
    let mut statement = connection.prepare(
        "SELECT archive_member, path_display, path_bytes, path_encoding,
                source_archive_path_display, source_archive_path_bytes,
                source_archive_path_encoding
         FROM local_rom_files
         WHERE included=1 AND source_archive_path_bytes=?1
         ORDER BY archive_member",
    )?;
    let rows = statement
        .query_map([&source.bytes], |row| {
            Ok((
                row.get::<_, String>(0)?,
                decode_path(row.get(2)?, &row.get::<_, String>(3)?, row.get(1)?),
                decode_path(row.get(5)?, &row.get::<_, String>(6)?, row.get(4)?),
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.len() != selected_count {
        bail!(
            "state retained {} materialized members after {selected_count} were imported",
            rows.len()
        );
    }
    let mut members = HashSet::new();
    let mut materialized_paths = HashSet::new();
    for (member, path, persisted_source) in &rows {
        if member.is_empty() || !members.insert(member) {
            bail!("state did not retain unique non-empty archive member identities");
        }
        if persisted_source != &archive_path {
            bail!("materialized member lost its exact source archive path");
        }
        if !path.is_file() || !materialized_paths.insert(path) {
            bail!("materialized members do not have distinct readable launch paths");
        }
    }
    let repeated = commit_scan_to(store.path(), &output)?;
    if repeated != selected_count {
        bail!("idempotent rescan imported {repeated} of {selected_count} members");
    }
    let retained = connection.query_row(
        "SELECT count(*) FROM local_rom_files
         WHERE included=1 AND source_archive_path_bytes=?1",
        [&source.bytes],
        |row| row.get::<_, usize>(0),
    )?;
    if retained != selected_count {
        bail!("idempotent rescan retained {retained} of {selected_count} members");
    }
    Ok(format!(
        "archive={:?} members={selected_count} exact={exact_count} cache_paths={} state={:?}",
        archive_path.file_name().unwrap_or_default(),
        materialized_paths.len(),
        state_path
    ))
}

pub(crate) fn manual_match_probe() -> Result<String> {
    let discovery_path = catalog::requested_discovery_database_path()
        .context("--games-database is required for the manual match probe")?;
    let root = catalog::requested_path("--import-directory", "LUNCHBOX_IMPORT_DIRECTORY")
        .context("--import-directory is required for the manual match probe")?;
    let state_path = catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .context("--state-database is required for the manual match probe")?;
    let query = catalog::requested_value("--manual-match-query", "LUNCHBOX_MANUAL_MATCH_QUERY")
        .context("--manual-match-query is required for the manual match probe")?;
    let platform =
        catalog::requested_value("--manual-match-platform", "LUNCHBOX_MANUAL_MATCH_PLATFORM")
            .context("--manual-match-platform is required for the manual match probe")?;
    let store = SettingsStore::at(&state_path)?;
    let cancelled = AtomicBool::new(false);
    let mut output = scan_directory(
        &discovery_path,
        &root,
        &platform,
        true,
        &cancelled,
        Arc::new(|_| {}),
    )?;
    for result in &mut output.results {
        result.selected = false;
    }
    let unmatched = output
        .results
        .iter()
        .enumerate()
        .filter(|(_, result)| matches!(result.match_state, MatchState::Unmatched))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if unmatched.len() != 1 {
        bail!(
            "manual match probe requires exactly one unmatched ROM, found {}",
            unmatched.len()
        );
    }
    let candidates = search_manual_match_candidates(&discovery_path, &query, &platform)?;
    let exact_candidates = candidates
        .iter()
        .filter(|candidate| {
            candidate.identity.title.eq_ignore_ascii_case(&query)
                && candidate.identity.platform == platform
        })
        .collect::<Vec<_>>();
    if exact_candidates.len() != 1 {
        bail!(
            "manual match probe requires one exact reviewed title/platform candidate, found {}",
            exact_candidates.len()
        );
    }
    let result_index = unmatched[0];
    let source_path = output.results[result_index].path.clone();
    let archive_member = output.results[result_index].archive_member.clone();
    let identity = exact_candidates[0].identity.clone();
    apply_manual_match(&mut output.results[result_index], exact_candidates[0])?;
    let imported = commit_scan_to(store.path(), &output)?;
    if imported != 1 {
        bail!("manual match probe imported {imported} ROMs instead of one");
    }

    let mut rescanned = scan_directory(
        &discovery_path,
        &root,
        &platform,
        true,
        &cancelled,
        Arc::new(|_| {}),
    )?;
    let restored = restore_manual_matches(store.path(), &mut rescanned)?;
    if restored != 1 {
        bail!("manual match probe restored {restored} reviewed links instead of one");
    }
    let result = rescanned
        .results
        .iter()
        .find(|result| result.path == source_path && result.archive_member == archive_member)
        .context("reviewed ROM disappeared during the verification rescan")?;
    let MatchState::Reviewed(restored_identity) = &result.match_state else {
        bail!("manual match did not restore as an explicitly reviewed identity");
    };
    if restored_identity != &identity
        || result.match_method != MANUAL_MATCH_METHOD
        || !result.selected
    {
        bail!("restored manual match lost identity, provenance, or selection state");
    }
    Ok(format!(
        "file={:?} title={:?} platform={:?} game_uid={:?} state={:?}",
        source_path.file_name().unwrap_or_default(),
        identity.title,
        identity.platform,
        identity.game_uid,
        state_path
    ))
}

fn report_progress_with_cache(
    progress: &Arc<dyn Fn(ScanProgress) + Send + Sync>,
    total_files: usize,
    position: usize,
    current_file: String,
    cache_reused_files: usize,
    content_read_files: usize,
) {
    progress(ScanProgress {
        total_files,
        scanned_files: position.saturating_add(1),
        current_file,
        cache_reused_files,
        content_read_files,
    });
}

#[derive(Clone, Debug)]
struct Digests {
    crc32: String,
    md5: String,
    sha1: String,
}

pub(crate) fn archive_kind_label(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("zip") => "ZIP",
        Some("7z") => "7z archive",
        Some("rar") => "RAR archive",
        _ => "archive",
    }
}

fn normalize_archive_member_name(name: &str) -> Option<String> {
    if name.is_empty() || name.contains('\0') {
        return None;
    }
    let normalized = name.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with("//") {
        return None;
    }
    let components = normalized.split('/').collect::<Vec<_>>();
    if components.is_empty()
        || components
            .iter()
            .any(|component| component.is_empty() || matches!(*component, "." | ".."))
        || components.first().is_some_and(|component| {
            let bytes = component.as_bytes();
            bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
        })
    {
        return None;
    }
    Some(components.join("/"))
}

fn recognized_archive_member(
    name: &str,
    recognized_extensions: &HashSet<String>,
) -> Option<(String, String)> {
    let member_name = normalize_archive_member_name(name)?;
    let mut components = member_name.split('/');
    if components
        .next()
        .is_some_and(|component| component == "__MACOSX")
    {
        return None;
    }
    let file_name = member_name.rsplit('/').next()?;
    if file_name.starts_with("._") {
        return None;
    }
    let (_, extension) = file_name.rsplit_once('.')?;
    let extension = extension.to_ascii_lowercase();
    if extension.is_empty()
        || ARCHIVE_EXTENSIONS.contains(&extension.as_str())
        || !recognized_extensions.contains(&extension)
    {
        return None;
    }
    Some((member_name, extension))
}

fn inspect_zip(
    path: &Path,
    recognized_extensions: &HashSet<String>,
    checksums_enabled: bool,
    cancelled: &AtomicBool,
) -> Result<Option<ArchiveInspection>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("reading ZIP directory {}", path.display()))?;
    let mut members = Vec::new();
    let mut names = HashSet::new();
    for member_index in 0..archive.len() {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let entry = archive
            .by_index(member_index)
            .with_context(|| format!("reading ZIP member #{member_index} in {}", path.display()))?;
        if entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            continue;
        }
        let Some((member_name, extension)) =
            recognized_archive_member(entry.name(), recognized_extensions)
        else {
            continue;
        };
        if !names.insert(member_name.clone()) {
            bail!("ZIP contains duplicate safe ROM member {member_name:?}");
        }
        members.push((member_index, member_name, extension, entry.size()));
    }

    if members.is_empty() {
        return Ok(Some(ArchiveInspection::Empty));
    }
    let mut inspected = Vec::with_capacity(members.len());
    for (member_index, member_name, extension, size) in members {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let mut entry = archive
            .by_index(member_index)
            .with_context(|| format!("reopening ZIP member {member_name} in {}", path.display()))?;
        let (digests, error) = if entry.encrypted() {
            (
                None,
                Some(format!(
                    "ZIP member {member_name} is encrypted and cannot be imported"
                )),
            )
        } else if checksums_enabled {
            match hash_reader(
                &mut entry,
                cancelled,
                &format!("ZIP member {member_name} in {}", path.display()),
            ) {
                Ok(Some(digests)) => (Some(digests), None),
                Ok(None) => return Ok(None),
                Err(error) => (None, Some(error.to_string())),
            }
        } else {
            (None, None)
        };
        inspected.push(ArchiveMemberInspection {
            member_name,
            extension,
            size,
            digests,
            error,
        });
    }
    Ok(Some(ArchiveInspection::Members(inspected)))
}

fn inspect_seven_zip(
    path: &Path,
    recognized_extensions: &HashSet<String>,
    checksums_enabled: bool,
    cancelled: &AtomicBool,
) -> Result<Option<ArchiveInspection>> {
    let mut archive = sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty())
        .with_context(|| format!("reading 7z directory {}", path.display()))?;
    let mut inspected = Vec::new();
    let mut member_indices = HashMap::new();
    for entry in &archive.archive().files {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        if entry.is_directory() || entry.is_anti_item {
            continue;
        }
        let Some((member_name, extension)) =
            recognized_archive_member(entry.name(), recognized_extensions)
        else {
            continue;
        };
        if member_indices.contains_key(&member_name) {
            bail!("7z archive contains duplicate safe ROM member {member_name:?}");
        }
        member_indices.insert(member_name.clone(), inspected.len());
        inspected.push(ArchiveMemberInspection {
            member_name,
            extension,
            size: entry.size(),
            digests: None,
            error: None,
        });
    }
    if inspected.is_empty() {
        return Ok(Some(ArchiveInspection::Empty));
    }
    if !checksums_enabled {
        return Ok(Some(ArchiveInspection::Members(inspected)));
    }

    let mut remaining = inspected.len();
    let mut scan_cancelled = false;
    archive
        .for_each_entries(|entry, reader| {
            if cancelled.load(Ordering::Relaxed) {
                scan_cancelled = true;
                return Ok(false);
            }
            let member_name = normalize_archive_member_name(entry.name());
            let Some(member_index) = member_name
                .as_ref()
                .and_then(|name| member_indices.get(name))
                .copied()
            else {
                std::io::copy(reader, &mut std::io::sink())?;
                return Ok(true);
            };
            let description = format!(
                "7z member {} in {}",
                inspected[member_index].member_name,
                path.display()
            );
            match hash_reader(reader, cancelled, &description)
                .map_err(|error| std::io::Error::other(error.to_string()))?
            {
                Some(digests) => inspected[member_index].digests = Some(digests),
                None => {
                    scan_cancelled = true;
                    return Ok(false);
                }
            }
            remaining = remaining.saturating_sub(1);
            Ok(remaining > 0)
        })
        .with_context(|| format!("decoding 7z members in {}", path.display()))?;
    if scan_cancelled {
        return Ok(None);
    }
    if remaining != 0 {
        bail!("7z archive did not decode every reviewed ROM member");
    }
    Ok(Some(ArchiveInspection::Members(inspected)))
}

#[derive(Clone)]
struct SharedRarDigestWriter(Arc<std::sync::Mutex<RarDigestState>>);

struct RarDigestState {
    expected_size: u64,
    written: u64,
    crc32: Crc32,
    md5: Md5,
    sha1: Sha1,
}

impl RarDigestState {
    fn new(expected_size: u64) -> Self {
        Self {
            expected_size,
            written: 0,
            crc32: Crc32::new(),
            md5: Md5::new(),
            sha1: Sha1::new(),
        }
    }

    fn finish(&mut self) -> Result<Digests> {
        if self.written != self.expected_size {
            bail!(
                "RAR member decoded {} bytes after its header declared {}",
                self.written,
                self.expected_size
            );
        }
        Ok(Digests {
            crc32: format!(
                "{:08X}",
                std::mem::replace(&mut self.crc32, Crc32::new()).finalize()
            ),
            md5: format!(
                "{:X}",
                std::mem::replace(&mut self.md5, Md5::new()).finalize()
            ),
            sha1: format!(
                "{:X}",
                std::mem::replace(&mut self.sha1, Sha1::new()).finalize()
            ),
        })
    }
}

impl Write for SharedRarDigestWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("RAR digest state was poisoned"))?;
        let written = state
            .written
            .checked_add(u64::try_from(bytes.len()).map_err(std::io::Error::other)?)
            .ok_or_else(|| std::io::Error::other("RAR member size overflow"))?;
        if written > state.expected_size {
            return Err(std::io::Error::other(
                "RAR member exceeded its declared uncompressed size",
            ));
        }
        state.crc32.update(bytes);
        state.md5.update(bytes);
        state.sha1.update(bytes);
        state.written = written;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn rar_member_is_symlink(member: &rars::ArchiveMember) -> bool {
    let unix_attributes = match member.meta.family {
        ArchiveFamily::Rar15To40 => matches!(member.meta.host_os, Some(3) | Some(5)),
        ArchiveFamily::Rar50Plus => member.meta.host_os == Some(1),
        ArchiveFamily::Rar13 => false,
        _ => false,
    };
    unix_attributes && member.meta.file_attr & 0o170000 == 0o120000
}

fn inspect_rar(
    path: &Path,
    recognized_extensions: &HashSet<String>,
    checksums_enabled: bool,
    cancelled: &AtomicBool,
) -> Result<Option<ArchiveInspection>> {
    let read_options = ArchiveReadOptions::new().with_rar50_buffered_decode_limit(64 * 1024 * 1024);
    let archive = ArchiveReader::read_path_with_options(path, read_options)
        .with_context(|| format!("reading RAR directory {}", path.display()))?;
    let mut inspected = Vec::new();
    let mut names = HashSet::new();
    let mut digest_writers_by_raw_name = HashMap::<Vec<u8>, SharedRarDigestWriter>::new();
    let mut digest_writers_by_member_name = HashMap::<String, SharedRarDigestWriter>::new();
    let mut unsupported_reason = None;

    for member in archive.members() {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        if member.meta.is_encrypted {
            unsupported_reason = Some("RAR archive contains encrypted members".to_owned());
        }
        if member.meta.is_split_before || member.meta.is_split_after {
            unsupported_reason = Some("RAR archive is part of a multi-volume set".to_owned());
        }
        if member.meta.is_directory || rar_member_is_symlink(&member) {
            continue;
        }
        let Ok(raw_name) = std::str::from_utf8(member.meta.name_bytes()) else {
            continue;
        };
        let Some((member_name, extension)) =
            recognized_archive_member(raw_name, recognized_extensions)
        else {
            continue;
        };
        if !names.insert(member_name.clone()) {
            bail!("RAR archive contains duplicate safe ROM member {member_name:?}");
        }
        let digest_writer = SharedRarDigestWriter(Arc::new(std::sync::Mutex::new(
            RarDigestState::new(member.meta.unpacked_size),
        )));
        digest_writers_by_raw_name.insert(member.meta.name_bytes().to_vec(), digest_writer.clone());
        digest_writers_by_member_name.insert(member_name.clone(), digest_writer);
        inspected.push(ArchiveMemberInspection {
            member_name,
            extension,
            size: member.meta.unpacked_size,
            digests: None,
            error: None,
        });
    }

    if inspected.is_empty() {
        return Ok(Some(ArchiveInspection::Empty));
    }
    if let Some(reason) = unsupported_reason {
        for member in &mut inspected {
            member.error = Some(format!(
                "{reason}; password and volume prompting is not supported"
            ));
        }
        return Ok(Some(ArchiveInspection::Members(inspected)));
    }
    if !checksums_enabled {
        return Ok(Some(ArchiveInspection::Members(inspected)));
    }

    archive
        .extract_to_with_options(read_options, |meta| {
            if let Some(writer) = digest_writers_by_raw_name.get(meta.name_bytes()) {
                Ok(Box::new(writer.clone()) as Box<dyn Write>)
            } else {
                Ok(Box::new(std::io::sink()) as Box<dyn Write>)
            }
        })
        .with_context(|| format!("decoding RAR members in {}", path.display()))?;
    if cancelled.load(Ordering::Relaxed) {
        return Ok(None);
    }
    for member in &mut inspected {
        let writer = digest_writers_by_member_name
            .get(&member.member_name)
            .with_context(|| format!("RAR member {} lost its digest state", member.member_name))?;
        member.digests = Some(
            writer
                .0
                .lock()
                .map_err(|_| anyhow::anyhow!("RAR digest state was poisoned"))?
                .finish()?,
        );
    }
    Ok(Some(ArchiveInspection::Members(inspected)))
}

fn hash_file(path: &Path, cancelled: &AtomicBool) -> Result<Option<Digests>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    hash_reader(&mut reader, cancelled, &path.display().to_string())
}

fn hash_reader(
    reader: &mut (impl Read + ?Sized),
    cancelled: &AtomicBool,
    description: &str,
) -> Result<Option<Digests>> {
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut crc32 = Crc32::new();
    let mut md5 = Md5::new();
    let mut sha1 = Sha1::new();
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("reading {description}"))?;
        if read == 0 {
            break;
        }
        let bytes = &buffer[..read];
        crc32.update(bytes);
        md5.update(bytes);
        sha1.update(bytes);
    }
    Ok(Some(Digests {
        crc32: format!("{:08X}", crc32.finalize()),
        md5: format!("{:X}", md5.finalize()),
        sha1: format!("{:X}", sha1.finalize()),
    }))
}

fn materialize_archive_member(state_path: &Path, result: &ScanResult) -> Result<PathBuf> {
    if result.archive_member.is_empty() || result.archive_member_count <= 1 {
        bail!("archive materialization requires a selected member from a multi-ROM archive");
    }
    match result
        .path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("zip") => materialize_zip_member(state_path, result),
        Some("7z") => materialize_seven_zip_member(state_path, result),
        Some("rar") => materialize_rar_member(state_path, result),
        extension => bail!("archive member materialization does not support {extension:?}"),
    }
}

fn materialize_zip_member(state_path: &Path, result: &ScanResult) -> Result<PathBuf> {
    let source = File::open(&result.path)
        .with_context(|| format!("opening source archive {}", result.path.display()))?;
    let mut archive = zip::ZipArchive::new(source)
        .with_context(|| format!("reading source archive {}", result.path.display()))?;
    let mut matching_indices = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .with_context(|| format!("reading ZIP member #{index} in {}", result.path.display()))?;
        if normalize_archive_member_name(entry.name()).as_deref()
            == Some(result.archive_member.as_str())
        {
            matching_indices.push(index);
        }
    }
    if matching_indices.len() != 1 {
        bail!(
            "ZIP member {:?} occurs {} times; one exact safe member is required",
            result.archive_member,
            matching_indices.len()
        );
    }
    let mut entry = archive
        .by_index(matching_indices[0])
        .with_context(|| format!("opening ZIP member {}", result.archive_member))?;
    if entry.encrypted() {
        bail!("ZIP member {} is encrypted", result.archive_member);
    }
    if entry.size() != result.file_size {
        bail!(
            "ZIP member {} changed size after review ({} -> {} bytes)",
            result.archive_member,
            result.file_size,
            entry.size()
        );
    }
    materialize_member_stream(state_path, result, &mut entry)
}

fn materialize_seven_zip_member(state_path: &Path, result: &ScanResult) -> Result<PathBuf> {
    let mut archive =
        sevenz_rust2::ArchiveReader::open(&result.path, sevenz_rust2::Password::empty())
            .with_context(|| format!("reading source archive {}", result.path.display()))?;
    let matches = archive
        .archive()
        .files
        .iter()
        .filter(|entry| {
            !entry.is_directory()
                && !entry.is_anti_item
                && normalize_archive_member_name(entry.name()).as_deref()
                    == Some(result.archive_member.as_str())
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        bail!(
            "7z member {:?} occurs {} times; one exact safe member is required",
            result.archive_member,
            matches.len()
        );
    }
    if matches[0].size() != result.file_size {
        bail!(
            "7z member {} changed size after review ({} -> {} bytes)",
            result.archive_member,
            result.file_size,
            matches[0].size()
        );
    }
    let mut materialized = None;
    archive
        .for_each_entries(|entry, reader| {
            if entry.is_directory()
                || entry.is_anti_item
                || normalize_archive_member_name(entry.name()).as_deref()
                    != Some(result.archive_member.as_str())
            {
                std::io::copy(reader, &mut std::io::sink())?;
                return Ok(true);
            }
            let path = materialize_member_stream(state_path, result, reader)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            materialized = Some(path);
            Ok(false)
        })
        .with_context(|| {
            format!(
                "decoding 7z member {} in {}",
                result.archive_member,
                result.path.display()
            )
        })?;
    materialized.context("reviewed 7z member was not decoded")
}

#[derive(Clone)]
struct SharedRarMaterializeWriter(Arc<std::sync::Mutex<RarMaterializeState>>);

struct RarMaterializeState {
    output: Option<File>,
    expected_size: u64,
    written: u64,
    crc32: Crc32,
    md5: Md5,
    sha1: Sha1,
    sha256: Sha256,
}

impl Write for SharedRarMaterializeWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("RAR materialization state was poisoned"))?;
        let written = state
            .written
            .checked_add(u64::try_from(bytes.len()).map_err(std::io::Error::other)?)
            .ok_or_else(|| std::io::Error::other("RAR member size overflow"))?;
        if written > state.expected_size {
            return Err(std::io::Error::other(
                "RAR member exceeded its reviewed uncompressed size",
            ));
        }
        state
            .output
            .as_mut()
            .ok_or_else(|| std::io::Error::other("RAR materialization output was closed"))?
            .write_all(bytes)?;
        state.crc32.update(bytes);
        state.md5.update(bytes);
        state.sha1.update(bytes);
        state.sha256.update(bytes);
        state.written = written;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("RAR materialization state was poisoned"))?;
        state
            .output
            .as_mut()
            .ok_or_else(|| std::io::Error::other("RAR materialization output was closed"))?
            .flush()
    }
}

fn materialize_rar_member(state_path: &Path, result: &ScanResult) -> Result<PathBuf> {
    let read_options = ArchiveReadOptions::new().with_rar50_buffered_decode_limit(64 * 1024 * 1024);
    let archive = ArchiveReader::read_path_with_options(&result.path, read_options)
        .with_context(|| format!("reading source archive {}", result.path.display()))?;
    let mut matching_names = Vec::new();
    let mut unsupported_reason = None;
    for member in archive.members() {
        if member.meta.is_encrypted {
            unsupported_reason = Some("RAR archive contains encrypted members");
        }
        if member.meta.is_split_before || member.meta.is_split_after {
            unsupported_reason = Some("RAR archive is part of a multi-volume set");
        }
        if member.meta.is_directory || rar_member_is_symlink(&member) {
            continue;
        }
        let Ok(raw_name) = std::str::from_utf8(member.meta.name_bytes()) else {
            continue;
        };
        if normalize_archive_member_name(raw_name).as_deref()
            == Some(result.archive_member.as_str())
        {
            matching_names.push((member.meta.name.clone(), member.meta.unpacked_size));
        }
    }
    if let Some(reason) = unsupported_reason {
        bail!("{reason}; password and volume prompting is not supported");
    }
    if matching_names.len() != 1 {
        bail!(
            "RAR member {:?} occurs {} times; one exact safe member is required",
            result.archive_member,
            matching_names.len()
        );
    }
    let (raw_name, unpacked_size) = matching_names.pop().expect("one RAR member was verified");
    if unpacked_size != result.file_size {
        bail!(
            "RAR member {} changed size after review ({} -> {} bytes)",
            result.archive_member,
            result.file_size,
            unpacked_size
        );
    }

    let (cache_root, temporary, output) = create_materialization_output(state_path)?;
    let shared = SharedRarMaterializeWriter(Arc::new(std::sync::Mutex::new(RarMaterializeState {
        output: Some(output),
        expected_size: result.file_size,
        written: 0,
        crc32: Crc32::new(),
        md5: Md5::new(),
        sha1: Sha1::new(),
        sha256: Sha256::new(),
    })));
    let outcome = (|| -> Result<PathBuf> {
        archive
            .extract_to_with_options(read_options, |meta| {
                if meta.name_bytes() == raw_name {
                    Ok(Box::new(shared.clone()) as Box<dyn Write>)
                } else {
                    Ok(Box::new(std::io::sink()) as Box<dyn Write>)
                }
            })
            .with_context(|| {
                format!(
                    "decoding RAR member {} in {}",
                    result.archive_member,
                    result.path.display()
                )
            })?;
        let mut state = shared
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("RAR materialization state was poisoned"))?;
        let output = state
            .output
            .take()
            .context("RAR materialization output disappeared")?;
        output
            .sync_all()
            .with_context(|| format!("syncing temporary ROM {}", temporary.display()))?;
        drop(output);
        let written = state.written;
        let crc32 = format!(
            "{:08X}",
            std::mem::replace(&mut state.crc32, Crc32::new()).finalize()
        );
        let md5 = format!(
            "{:X}",
            std::mem::replace(&mut state.md5, Md5::new()).finalize()
        );
        let sha1 = format!(
            "{:X}",
            std::mem::replace(&mut state.sha1, Sha1::new()).finalize()
        );
        let sha256 = format!(
            "{:x}",
            std::mem::replace(&mut state.sha256, Sha256::new()).finalize()
        );
        drop(state);
        publish_materialized_member(
            &cache_root,
            &temporary,
            result,
            written,
            &crc32,
            &md5,
            &sha1,
            &sha256,
        )
    })();
    if outcome.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    outcome
}

fn create_materialization_output(state_path: &Path) -> Result<(PathBuf, PathBuf, File)> {
    let state_parent = state_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let cache_root = state_parent.join("local-rom-cache");
    fs::create_dir_all(&cache_root)
        .with_context(|| format!("creating local ROM cache {}", cache_root.display()))?;
    let temporary = cache_root.join(format!(".{}.tmp", Uuid::new_v4()));
    let output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .with_context(|| format!("creating temporary ROM {}", temporary.display()))?;
    Ok((cache_root, temporary, output))
}

fn materialize_member_stream(
    state_path: &Path,
    result: &ScanResult,
    reader: &mut (impl Read + ?Sized),
) -> Result<PathBuf> {
    let archive_label = archive_kind_label(&result.path);
    let (cache_root, temporary, mut output) = create_materialization_output(state_path)?;
    let outcome = (|| -> Result<PathBuf> {
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut crc32 = Crc32::new();
        let mut md5 = Md5::new();
        let mut sha1 = Sha1::new();
        let mut sha256 = Sha256::new();
        let mut written = 0u64;
        loop {
            let read = reader.read(&mut buffer).with_context(|| {
                format!("reading {archive_label} member {}", result.archive_member)
            })?;
            if read == 0 {
                break;
            }
            let bytes = &buffer[..read];
            output
                .write_all(bytes)
                .with_context(|| format!("writing temporary ROM {}", temporary.display()))?;
            crc32.update(bytes);
            md5.update(bytes);
            sha1.update(bytes);
            sha256.update(bytes);
            written = written
                .checked_add(u64::try_from(read).context("ROM read size overflow")?)
                .context("materialized ROM size overflow")?;
        }
        output
            .sync_all()
            .with_context(|| format!("syncing temporary ROM {}", temporary.display()))?;
        drop(output);
        let crc32 = format!("{:08X}", crc32.finalize());
        let md5 = format!("{:X}", md5.finalize());
        let sha1 = format!("{:X}", sha1.finalize());
        let sha256 = format!("{:x}", sha256.finalize());
        publish_materialized_member(
            &cache_root,
            &temporary,
            result,
            written,
            &crc32,
            &md5,
            &sha1,
            &sha256,
        )
    })();
    if outcome.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    outcome
}

#[allow(clippy::too_many_arguments)]
fn publish_materialized_member(
    cache_root: &Path,
    temporary: &Path,
    result: &ScanResult,
    written: u64,
    crc32: &str,
    md5: &str,
    sha1: &str,
    sha256: &str,
) -> Result<PathBuf> {
    let archive_label = archive_kind_label(&result.path);
    if written != result.file_size {
        bail!(
            "{archive_label} member {} produced {written} bytes after {} were reviewed",
            result.archive_member,
            result.file_size
        );
    }
    for (label, reviewed, actual) in [
        ("CRC32", result.crc32.as_str(), crc32),
        ("MD5", result.md5.as_str(), md5),
        ("SHA-1", result.sha1.as_str(), sha1),
    ] {
        if !reviewed.is_empty() && reviewed != actual {
            bail!(
                "{archive_label} member {} changed {label} after review",
                result.archive_member
            );
        }
    }
    let extension = Path::new(&result.archive_member)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 16
                && value
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
        .context("reviewed archive member has no portable ROM extension")?
        .to_ascii_lowercase();
    let target_directory = cache_root.join(sha256);
    fs::create_dir_all(&target_directory).with_context(|| {
        format!(
            "creating ROM content directory {}",
            target_directory.display()
        )
    })?;
    let target = target_directory.join(format!("content.{extension}"));
    if target.exists() {
        if !target.is_file() || hash_sha256(&target)? != sha256 {
            bail!(
                "existing ROM cache target is not the reviewed content: {}",
                target.display()
            );
        }
        fs::remove_file(temporary)
            .with_context(|| format!("removing temporary ROM {}", temporary.display()))?;
        return Ok(target);
    }
    fs::rename(temporary, &target).with_context(|| {
        format!(
            "publishing reviewed {archive_label} member {} to {}",
            result.archive_member,
            target.display()
        )
    })?;
    Ok(target)
}

fn hash_sha256(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut sha256 = Sha256::new();
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("reading {}", path.display()))?;
        if read == 0 {
            break;
        }
        sha256.update(&buffer[..read]);
    }
    Ok(format!("{:x}", sha256.finalize()))
}

fn match_digest(
    index: &MatchIndex,
    sha1: &str,
    md5: &str,
    platform_hint: &str,
) -> (MatchState, String) {
    let (mut candidates, method) = if let Some(candidates) = index.by_sha1.get(sha1) {
        (
            candidates
                .iter()
                .filter(|candidate| candidate.md5.is_empty() || candidate.md5 == md5)
                .cloned()
                .collect::<Vec<_>>(),
            "SHA-1 + MD5",
        )
    } else if let Some(candidates) = index.by_md5.get(md5) {
        (candidates.clone(), "MD5")
    } else {
        return (MatchState::Unmatched, "strong checksum".to_owned());
    };
    if !platform_hint.is_empty() {
        candidates.retain(|candidate| candidate.identity.platform == platform_hint);
    }
    candidates.sort_by(|left, right| left.identity.game_uid.cmp(&right.identity.game_uid));
    candidates.dedup_by(|left, right| left.identity.game_uid == right.identity.game_uid);
    match candidates.len() {
        0 => (MatchState::Unmatched, method.to_owned()),
        1 => (
            MatchState::Exact(candidates.remove(0).identity),
            method.to_owned(),
        ),
        count => (MatchState::Ambiguous(count), method.to_owned()),
    }
}

fn error_result(root: &Path, path: &Path, error: String) -> ScanResult {
    let file_name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    ScanResult {
        path: path.to_path_buf(),
        relative_path: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
        file_name: file_name.clone(),
        archive_member: String::new(),
        archive_member_count: 0,
        display_title: display_title(&file_name),
        platform: "Unassigned".to_owned(),
        file_size: 0,
        modified_unix_ns: None,
        crc32: String::new(),
        md5: String::new(),
        sha1: String::new(),
        match_state: MatchState::Error(error),
        match_method: "read error".to_owned(),
        selected: false,
    }
}

fn display_title(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .map(|value| value.to_string_lossy())
        .unwrap_or_default();
    let mut result = String::with_capacity(stem.len());
    let mut round_depth = 0usize;
    let mut square_depth = 0usize;
    for character in stem.chars() {
        match character {
            '(' => round_depth = round_depth.saturating_add(1),
            ')' => round_depth = round_depth.saturating_sub(1),
            '[' => square_depth = square_depth.saturating_add(1),
            ']' => square_depth = square_depth.saturating_sub(1),
            _ if round_depth == 0 && square_depth == 0 => result.push(character),
            _ => {}
        }
    }
    let result = result.trim();
    if result.is_empty() {
        stem.into_owned()
    } else {
        result.to_owned()
    }
}

pub fn commit_scan(output: &ScanOutput) -> Result<usize> {
    let store = SettingsStore::open_default()?;
    commit_scan_to(store.path(), output)
}

fn commit_scan_to(state_path: &Path, output: &ScanOutput) -> Result<usize> {
    let root_encoded = encode_path(&output.root);
    let root_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, &root_encoded.bytes).to_string();
    let mut persisted_paths = Vec::with_capacity(output.results.len());
    for result in &output.results {
        let included = result.selected && !matches!(result.match_state, MatchState::Error(_));
        let path = if included && result.archive_member_count > 1 {
            materialize_archive_member(state_path, result)?
        } else {
            result.path.clone()
        };
        persisted_paths.push(path);
    }
    let mut connection = Connection::open(state_path)
        .with_context(|| format!("opening Lunchbox state {}", state_path.display()))?;
    connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;")?;
    let transaction = connection.transaction()?;
    let now = settings::unix_timestamp();
    transaction.execute(
        "INSERT INTO local_collection_roots (
             id, path_display, path_bytes, path_encoding, platform_hint,
             checksums_enabled, last_scanned_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
             path_display=excluded.path_display, path_bytes=excluded.path_bytes,
             path_encoding=excluded.path_encoding, platform_hint=excluded.platform_hint,
             checksums_enabled=excluded.checksums_enabled,
             last_scanned_at=excluded.last_scanned_at",
        params![
            root_id,
            root_encoded.display,
            root_encoded.bytes,
            root_encoded.encoding,
            output.platform_hint,
            output.checksums_enabled,
            now,
        ],
    )?;
    mark_scanned_scope_missing(&transaction, &root_id, &output.extension_filter)?;

    let mut imported = 0usize;
    for (result, persisted_path) in output.results.iter().zip(persisted_paths) {
        let path = encode_path(&persisted_path);
        let relative = encode_path(&result.relative_path);
        let source_archive = if persisted_path != result.path {
            encode_path(&result.path)
        } else {
            EncodedPath {
                display: String::new(),
                bytes: Vec::new(),
                encoding: "",
            }
        };
        let mut identity_bytes = root_id.as_bytes().to_vec();
        identity_bytes.push(0);
        identity_bytes.extend_from_slice(&relative.bytes);
        if result.archive_member_count > 1 {
            identity_bytes.push(0);
            identity_bytes.extend_from_slice(result.archive_member.as_bytes());
        }
        let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, &identity_bytes).to_string();
        let (game_uid, database_id, matched_title) = match &result.match_state {
            MatchState::Exact(identity) | MatchState::Reviewed(identity) => (
                Some(identity.game_uid.as_str()),
                identity.launchbox_db_id,
                Some(identity.title.as_str()),
            ),
            _ => (None, 0, None),
        };
        let included = result.selected && !matches!(result.match_state, MatchState::Error(_));
        imported = imported.saturating_add(usize::from(included));
        transaction.execute(
            "INSERT INTO local_rom_files (
                 id, root_id, path_display, path_bytes, path_encoding,
                 relative_path_display, relative_path_bytes, relative_path_encoding,
                 file_name, archive_member, source_archive_path_display,
                 source_archive_path_bytes, source_archive_path_encoding,
                 display_title, platform, file_size, modified_unix_ns,
                 crc32, md5, sha1, game_uid, launchbox_db_id, matched_title,
                 match_state, match_method, included, availability, imported_at
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                 ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24,
                 ?25, ?26, 'present', ?27
             )
             ON CONFLICT(id) DO UPDATE SET
                 path_display=excluded.path_display, path_bytes=excluded.path_bytes,
                 path_encoding=excluded.path_encoding,
                 relative_path_display=excluded.relative_path_display,
                 relative_path_bytes=excluded.relative_path_bytes,
                 relative_path_encoding=excluded.relative_path_encoding,
                 file_name=excluded.file_name, archive_member=excluded.archive_member,
                 source_archive_path_display=excluded.source_archive_path_display,
                 source_archive_path_bytes=excluded.source_archive_path_bytes,
                 source_archive_path_encoding=excluded.source_archive_path_encoding,
                 display_title=excluded.display_title,
                 platform=excluded.platform, file_size=excluded.file_size,
                 modified_unix_ns=excluded.modified_unix_ns, crc32=excluded.crc32,
                 md5=excluded.md5, sha1=excluded.sha1, game_uid=excluded.game_uid,
                 launchbox_db_id=excluded.launchbox_db_id,
                 matched_title=excluded.matched_title, match_state=excluded.match_state,
                 match_method=excluded.match_method, included=excluded.included,
                 availability='present', imported_at=excluded.imported_at",
            params![
                id,
                root_id,
                path.display,
                path.bytes,
                path.encoding,
                relative.display,
                relative.bytes,
                relative.encoding,
                result.file_name,
                result.archive_member,
                source_archive.display,
                source_archive.bytes,
                source_archive.encoding,
                result.display_title,
                result.platform,
                i64::try_from(result.file_size).unwrap_or(i64::MAX),
                result.modified_unix_ns,
                result.crc32,
                result.md5,
                result.sha1,
                game_uid,
                database_id,
                matched_title,
                result.match_state.key(),
                result.match_method,
                included,
                now,
            ],
        )?;
    }
    transaction.commit()?;
    Ok(imported)
}

fn mark_scanned_scope_missing(
    transaction: &rusqlite::Transaction<'_>,
    root_id: &str,
    extension_filter: &[String],
) -> Result<()> {
    if extension_filter.is_empty() {
        transaction.execute(
            "UPDATE local_rom_files SET availability='missing' WHERE root_id=?1",
            [root_id],
        )?;
        return Ok(());
    }
    let extensions = extension_filter.iter().collect::<HashSet<_>>();
    let affected = {
        let mut statement = transaction.prepare(
            "SELECT id, path_display, path_bytes, path_encoding,
                    source_archive_path_display, source_archive_path_bytes,
                    source_archive_path_encoding
             FROM local_rom_files WHERE root_id=?1",
        )?;
        let rows = statement.query_map([root_id], |row| {
            let path_display = row.get::<_, String>(1)?;
            let source_display = row.get::<_, String>(4)?;
            let path = if source_display.is_empty() {
                decode_path(row.get(2)?, &row.get::<_, String>(3)?, path_display)
            } else {
                decode_path(row.get(5)?, &row.get::<_, String>(6)?, source_display)
            };
            Ok((row.get::<_, String>(0)?, path))
        })?;
        rows.filter_map(|row| match row {
            Ok((id, path))
                if path
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(str::to_ascii_lowercase)
                    .is_some_and(|extension| extensions.contains(&extension)) =>
            {
                Some(Ok(id))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for id in affected {
        transaction.execute(
            "UPDATE local_rom_files SET availability='missing' WHERE id=?1",
            [id],
        )?;
    }
    Ok(())
}

pub(crate) struct EncodedPath {
    pub display: String,
    pub bytes: Vec<u8>,
    pub encoding: &'static str,
}

#[cfg(unix)]
pub(crate) fn encode_path(path: &Path) -> EncodedPath {
    use std::os::unix::ffi::OsStrExt;
    EncodedPath {
        display: path.to_string_lossy().into_owned(),
        bytes: path.as_os_str().as_bytes().to_vec(),
        encoding: "unix_bytes",
    }
}

#[cfg(windows)]
pub(crate) fn encode_path(path: &Path) -> EncodedPath {
    use std::os::windows::ffi::OsStrExt;
    EncodedPath {
        display: path.to_string_lossy().into_owned(),
        bytes: path
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect(),
        encoding: "windows_utf16le",
    }
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn encode_path(path: &Path) -> EncodedPath {
    let display = path.to_string_lossy().into_owned();
    EncodedPath {
        bytes: display.as_bytes().to_vec(),
        display,
        encoding: "utf8",
    }
}

#[cfg(unix)]
pub(crate) fn decode_path(bytes: Vec<u8>, encoding: &str, display: String) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    if encoding == "unix_bytes" {
        std::ffi::OsString::from_vec(bytes).into()
    } else {
        PathBuf::from(display)
    }
}

#[cfg(windows)]
pub(crate) fn decode_path(bytes: Vec<u8>, encoding: &str, display: String) -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    if encoding == "windows_utf16le" && bytes.len() % 2 == 0 {
        let words = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        std::ffi::OsString::from_wide(&words).into()
    } else {
        PathBuf::from(display)
    }
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn decode_path(_bytes: Vec<u8>, _encoding: &str, display: String) -> PathBuf {
    PathBuf::from(display)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn discovery_fixture(path: &Path, content: &[u8]) {
        let mut crc = Crc32::new();
        crc.update(content);
        let crc = format!("{:08X}", crc.finalize());
        let md5 = format!("{:X}", Md5::digest(content));
        let sha1 = format!("{:X}", Sha1::digest(content));
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE platforms (
                     id INTEGER PRIMARY KEY, name TEXT, file_extensions TEXT
                 );
                 CREATE TABLE games (
                     id TEXT PRIMARY KEY, title TEXT, platform_id INTEGER,
                     launchbox_db_id INTEGER, libretro_crc32 TEXT,
                     libretro_md5 TEXT, libretro_sha1 TEXT
                 );
                 CREATE TABLE game_alternate_names (
                     id INTEGER PRIMARY KEY, launchbox_db_id INTEGER,
                     alternate_name TEXT, region TEXT
                 );
                 INSERT INTO platforms VALUES (1, 'Nintendo Entertainment System', 'nes, zip');",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO game_alternate_names VALUES (1, 42, 'The Exact Alias', 'World')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO games VALUES ('game-1', 'Exact Game', 1, 42, ?1, ?2, ?3)",
                params![crc, md5, sha1],
            )
            .unwrap();
    }

    fn zip_fixture(path: &Path, members: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        for (name, content) in members {
            archive
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(content).unwrap();
        }
        archive.finish().unwrap();
    }

    fn seven_zip_fixture(path: &Path, members: &[(&str, &[u8])]) {
        let mut archive = sevenz_rust2::ArchiveWriter::create(path).unwrap();
        for (name, content) in members {
            archive
                .push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(*content))
                .unwrap();
        }
        archive.finish().unwrap();
    }

    #[test]
    fn extension_filters_are_portable_normalized_and_bounded() {
        assert_eq!(
            normalize_extension_filter(" *.NES, .zip;nes  Custom-1 ").unwrap(),
            vec!["custom-1", "nes", "zip"]
        );
        assert_eq!(
            format_extension_filter(&["nes".into(), "zip".into()]),
            ".nes, .zip"
        );
        assert!(normalize_extension_filter("bad/path").is_err());
        assert!(normalize_extension_filter(&"x,".repeat(129)).is_err());
    }

    #[test]
    fn named_import_profiles_round_trip_update_and_delete_exact_state() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path().join("state.db");
        let root = directory.path().join("ROM Library");
        fs::create_dir(&root).unwrap();

        let saved = save_import_profile(
            &state,
            "Nintendo shelf",
            &root,
            "Nintendo Entertainment System",
            "zip, .nes, NES",
            true,
        )
        .unwrap();
        assert_eq!(saved.extensions, vec!["nes", "zip"]);
        let restarted = load_import_profiles(&state).unwrap();
        assert_eq!(restarted, vec![saved.clone()]);

        let updated =
            save_import_profile(&state, "NINTENDO SHELF", &root, "", "fds, nes", false).unwrap();
        assert_eq!(updated.id, saved.id);
        assert_eq!(updated.name, "NINTENDO SHELF");
        assert_eq!(updated.extensions, vec!["fds", "nes"]);
        assert!(!updated.checksums_enabled);
        assert_eq!(load_import_profiles(&state).unwrap(), vec![updated.clone()]);

        delete_import_profile(&state, &updated.id).unwrap();
        assert!(load_import_profiles(&state).unwrap().is_empty());
        assert!(delete_import_profile(&state, &updated.id).is_err());
    }

    #[test]
    fn scan_history_preserves_profile_snapshots_outcomes_and_batch_identity() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path().join("state.db");
        let root = directory.path().join("ROM Library");
        fs::create_dir(&root).unwrap();
        let profile = save_import_profile(
            &state,
            "Nintendo shelf",
            &root,
            "Nintendo Entertainment System",
            "nes",
            true,
        )
        .unwrap();
        let batch_id = Uuid::new_v4().to_string();
        let context = RomScanRunContext::for_profile(&profile, batch_id.clone(), 1, 2);
        let output = ScanOutput {
            root: root.clone(),
            platform_hint: profile.platform_hint.clone(),
            extension_filter: profile.extensions.clone(),
            checksums_enabled: true,
            results: vec![
                ScanResult {
                    path: root.join("exact.nes"),
                    relative_path: PathBuf::from("exact.nes"),
                    file_name: "exact.nes".into(),
                    archive_member: String::new(),
                    archive_member_count: 0,
                    display_title: "Exact Game".into(),
                    platform: profile.platform_hint.clone(),
                    file_size: 4,
                    modified_unix_ns: Some(1),
                    crc32: "00000000".into(),
                    md5: "00000000000000000000000000000000".into(),
                    sha1: "0000000000000000000000000000000000000000".into(),
                    match_state: MatchState::Exact(MatchIdentity {
                        game_uid: "game-1".into(),
                        title: "Exact Game".into(),
                        platform: profile.platform_hint.clone(),
                        launchbox_db_id: 42,
                    }),
                    match_method: "SHA-1 + MD5".into(),
                    selected: true,
                },
                ScanResult {
                    path: root.join("unknown.nes"),
                    relative_path: PathBuf::from("unknown.nes"),
                    file_name: "unknown.nes".into(),
                    archive_member: String::new(),
                    archive_member_count: 0,
                    display_title: "Unknown".into(),
                    platform: profile.platform_hint.clone(),
                    file_size: 5,
                    modified_unix_ns: Some(2),
                    crc32: "11111111".into(),
                    md5: "11111111111111111111111111111111".into(),
                    sha1: "1111111111111111111111111111111111111111".into(),
                    match_state: MatchState::Unmatched,
                    match_method: "strong checksum".into(),
                    selected: false,
                },
            ],
            walk_errors: 1,
            cancelled: false,
            cache_reused_files: 1,
            content_read_files: 1,
        };
        let completed = record_scan_run(&state, &context, Ok(&output)).unwrap();
        assert_eq!(completed.outcome, "completed");
        assert_eq!(completed.exact_matches, 1);
        assert_eq!(completed.other_results, 1);
        assert_eq!(completed.batch_id, batch_id);

        let failed_context =
            RomScanRunContext::one_time(root.clone(), String::new(), Vec::new(), false);
        let failed = record_scan_run(&state, &failed_context, Err("offline root")).unwrap();
        assert_eq!(failed.outcome, "failed");
        assert_eq!(failed.error_text, "offline root");

        delete_import_profile(&state, &profile.id).unwrap();
        let restarted = load_scan_runs(&state).unwrap();
        assert_eq!(restarted.len(), 2);
        let completed = restarted
            .iter()
            .find(|run| run.outcome == "completed")
            .unwrap();
        assert_eq!(completed.profile_id, profile.id);
        assert_eq!(completed.profile_name, "Nintendo shelf");
        assert_eq!(completed.root, root);
        assert_eq!(completed.extensions, vec!["nes"]);
        assert_eq!(completed.batch_position, 1);
        assert_eq!(completed.batch_total, 2);
        assert!(restarted.iter().any(|run| run.outcome == "failed"));
    }

    #[test]
    fn scan_history_rejects_incoherent_batch_positions() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path().join("state.db");
        let root = directory.path().join("roms");
        fs::create_dir(&root).unwrap();
        let mut context = RomScanRunContext::one_time(root, String::new(), Vec::new(), true);
        context.batch_id = Uuid::new_v4().to_string();
        context.batch_position = 2;
        context.batch_total = 1;
        assert!(record_scan_run(&state, &context, Err("not reached")).is_err());
        assert!(load_scan_runs(&state).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn named_import_profiles_preserve_non_utf8_root_bytes() {
        use std::os::unix::ffi::OsStringExt;

        let directory = tempfile::tempdir().unwrap();
        let state = directory.path().join("state.db");
        let root = directory.path().join(std::ffi::OsString::from_vec(vec![
            b'r', b'o', b'm', b's', 0xff,
        ]));
        fs::create_dir(&root).unwrap();

        let saved = save_import_profile(
            &state,
            "Native path",
            &root,
            "Nintendo Entertainment System",
            "nes",
            true,
        )
        .unwrap();

        assert_eq!(saved.root, root);
        assert_eq!(load_import_profiles(&state).unwrap()[0].root, root);
    }

    #[test]
    fn extension_scopes_include_custom_files_and_only_requested_archive_members() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let root = directory.path().join("roms");
        fs::create_dir(&root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        fs::write(root.join("Loose.nes"), b"exact-rom").unwrap();
        fs::write(root.join("Ignored.sfc"), b"ignored").unwrap();
        fs::write(root.join("Homebrew.custom-1"), b"homebrew").unwrap();
        zip_fixture(
            &root.join("Mixed.zip"),
            &[("Disc.nes", b"exact-rom"), ("Disc.sfc", b"ignored")],
        );
        let extensions = normalize_extension_filter("custom-1, nes, zip").unwrap();
        let state = directory.path().join("state.db");
        let output = scan_directory_cached_with_extensions(
            &discovery,
            &state,
            &root,
            "",
            &extensions,
            false,
            &AtomicBool::new(false),
            Arc::new(|_| {}),
        )
        .unwrap();

        assert_eq!(output.extension_filter, extensions);
        assert_eq!(output.results.len(), 3);
        assert!(
            output
                .results
                .iter()
                .any(|result| result.file_name == "Loose.nes")
        );
        assert!(
            output
                .results
                .iter()
                .any(|result| result.file_name == "Homebrew.custom-1")
        );
        let archive = output
            .results
            .iter()
            .find(|result| result.file_name == "Mixed.zip")
            .unwrap();
        assert_eq!(archive.archive_member, "Disc.nes");
        assert_eq!(archive.archive_member_count, 2);
        assert!(!archive.selected);
        assert!(!output.results.iter().any(|result| {
            result.file_name == "Ignored.sfc" || result.archive_member == "Disc.sfc"
        }));
    }

    #[test]
    fn scoped_rescan_does_not_mark_other_extensions_missing() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let state = directory.path().join("state.db");
        let root = directory.path().join("roms");
        fs::create_dir(&root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        fs::write(root.join("Game.nes"), b"exact-rom").unwrap();
        fs::write(root.join("Game.sfc"), b"other").unwrap();
        SettingsStore::at(&state).unwrap();
        let mut initial = scan_directory(
            &discovery,
            &root,
            "",
            false,
            &AtomicBool::new(false),
            Arc::new(|_| {}),
        )
        .unwrap();
        initial
            .results
            .iter_mut()
            .for_each(|result| result.selected = true);
        assert_eq!(commit_scan_to(&state, &initial).unwrap(), 2);

        fs::remove_file(root.join("Game.nes")).unwrap();
        fs::remove_file(root.join("Game.sfc")).unwrap();
        let scoped = scan_directory_cached_with_extensions(
            &discovery,
            &state,
            &root,
            "",
            &["nes".into()],
            false,
            &AtomicBool::new(false),
            Arc::new(|_| {}),
        )
        .unwrap();
        assert!(scoped.results.is_empty());
        assert_eq!(commit_scan_to(&state, &scoped).unwrap(), 0);

        let connection = Connection::open(state).unwrap();
        let statuses = connection
            .prepare("SELECT file_name, availability FROM local_rom_files ORDER BY file_name")
            .unwrap()
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            statuses,
            vec![
                ("Game.nes".into(), "missing".into()),
                ("Game.sfc".into(), "present".into()),
            ]
        );
    }

    fn rar_fixture(path: &Path, members: &[(&str, &[u8])]) {
        let mut archive = rars::Builder::new(rars::ArchiveVersion::Rar50);
        for (name, content) in members {
            archive
                .add_bytes(name.as_bytes().to_vec(), content.to_vec(), None, None)
                .unwrap();
        }
        archive.write_to_path(path, None).unwrap();
    }

    #[test]
    fn manual_match_search_includes_alternate_titles_without_accepting_them() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        discovery_fixture(&discovery, b"exact-rom");

        let candidates = search_manual_match_candidates(
            &discovery,
            "exact alias",
            "Nintendo Entertainment System",
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].identity.game_uid, "game-1");
        assert_eq!(candidates[0].identity.title, "Exact Game");
        assert_eq!(candidates[0].matched_name, "The Exact Alias");
        assert!(
            search_manual_match_candidates(&discovery, "exact", "Another Platform")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn reviewed_manual_match_survives_only_an_exact_content_rescan() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let rom_path = rom_root.join("Mystery.nes");
        fs::write(&rom_path, b"unknown-rom").unwrap();

        let cancel = AtomicBool::new(false);
        let mut output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].match_state, MatchState::Unmatched);
        assert!(!output.results[0].selected);
        let candidate = search_manual_match_candidates(
            &discovery,
            "Exact Game",
            "Nintendo Entertainment System",
        )
        .unwrap()
        .remove(0);
        apply_manual_match(&mut output.results[0], &candidate).unwrap();
        assert!(matches!(
            output.results[0].match_state,
            MatchState::Reviewed(_)
        ));
        assert!(output.results[0].selected);

        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 1);
        let connection = Connection::open(&state).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT match_state || ':' || match_method FROM local_rom_files",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "reviewed:manual user selection"
        );

        let mut repeated =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(restore_manual_matches(&state, &mut repeated).unwrap(), 1);
        let MatchState::Reviewed(identity) = &repeated.results[0].match_state else {
            panic!("reviewed identity was not restored");
        };
        assert_eq!(identity.game_uid, "game-1");
        assert!(repeated.results[0].selected);

        fs::write(&rom_path, b"changed-rom").unwrap();
        let mut changed =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(restore_manual_matches(&state, &mut changed).unwrap(), 0);
        assert_eq!(changed.results[0].match_state, MatchState::Unmatched);
        assert!(!changed.results[0].selected);
    }

    #[test]
    fn single_rom_zip_member_receives_an_exact_checksum_identity() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        zip_fixture(
            &rom_root.join("Exact Game.zip"),
            &[
                ("__MACOSX/._Exact Game.nes", b"metadata"),
                ("docs/readme.txt", b"documentation"),
                ("roms/Exact Game (USA).nes", b"exact-rom"),
            ],
        );

        let cancel = AtomicBool::new(false);
        let output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 1);
        let result = &output.results[0];
        assert!(matches!(result.match_state, MatchState::Exact(_)));
        assert!(result.selected);
        assert_eq!(result.archive_member_count, 1);
        assert_eq!(result.archive_member, "roms/Exact Game (USA).nes");
        assert_eq!(result.display_title, "Exact Game");
        assert_eq!(result.file_size, 9);
        assert_eq!(result.platform, "Nintendo Entertainment System");
        assert_eq!(result.match_method, "ZIP member SHA-1 + MD5");

        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 1);
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 1);
        let connection = Connection::open(state).unwrap();
        let stored = connection
            .query_row(
                "SELECT file_name, display_title, file_size, game_uid, match_method
                 FROM local_rom_files",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            stored,
            (
                "Exact Game.zip".to_owned(),
                "Exact Game".to_owned(),
                9,
                "game-1".to_owned(),
                "ZIP member SHA-1 + MD5".to_owned(),
            )
        );
    }

    #[test]
    fn resumable_hash_cache_reuses_regular_and_archive_content_and_prunes_removed_files() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let state = directory.path().join("state.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let regular_path = rom_root.join("Exact Game.nes");
        let archive_path = rom_root.join("Exact Archive.zip");
        fs::write(&regular_path, b"exact-rom").unwrap();
        zip_fixture(&archive_path, &[("Exact Archive.nes", b"exact-rom")]);

        let cancel = AtomicBool::new(false);
        let first = scan_directory_cached(
            &discovery,
            &state,
            &rom_root,
            "",
            true,
            &cancel,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(first.cache_reused_files, 0);
        assert_eq!(first.content_read_files, 2);
        assert_eq!(first.results.len(), 2);
        assert!(
            first
                .results
                .iter()
                .all(|result| matches!(result.match_state, MatchState::Exact(_)))
        );

        let repeated = scan_directory_cached(
            &discovery,
            &state,
            &rom_root,
            "",
            true,
            &cancel,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(repeated.cache_reused_files, 2);
        assert_eq!(repeated.content_read_files, 0);
        assert_eq!(repeated.results.len(), 2);
        assert_eq!(repeated.results[0].sha1, first.results[0].sha1);
        assert_eq!(repeated.results[1].sha1, first.results[1].sha1);

        let inventory_only = scan_directory_cached(
            &discovery,
            &state,
            &rom_root,
            "",
            false,
            &cancel,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(inventory_only.cache_reused_files, 0);
        assert_eq!(inventory_only.content_read_files, 0);
        let after_inventory = scan_directory_cached(
            &discovery,
            &state,
            &rom_root,
            "",
            true,
            &cancel,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(after_inventory.cache_reused_files, 2);
        assert_eq!(after_inventory.content_read_files, 0);

        fs::remove_file(&regular_path).unwrap();
        fs::write(&regular_path, b"other-rom").unwrap();
        let changed = scan_directory_cached(
            &discovery,
            &state,
            &rom_root,
            "",
            true,
            &cancel,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(changed.cache_reused_files, 1);
        assert_eq!(changed.content_read_files, 1);
        let changed_regular = changed
            .results
            .iter()
            .find(|result| result.path == regular_path)
            .unwrap();
        assert_eq!(changed_regular.match_state, MatchState::Unmatched);

        fs::remove_file(&archive_path).unwrap();
        let pruned = scan_directory_cached(
            &discovery,
            &state,
            &rom_root,
            "",
            true,
            &cancel,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(pruned.cache_reused_files, 1);
        assert_eq!(pruned.content_read_files, 0);
        let connection = Connection::open(state).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM local_rom_hash_cache", [], |row| {
                    row.get::<_, usize>(0)
                })
                .unwrap(),
            1
        );
    }

    #[test]
    fn cancelled_hash_scan_reuses_every_completed_file_on_resume() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let state = directory.path().join("state.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        fs::write(rom_root.join("A.nes"), b"exact-rom").unwrap();
        fs::write(rom_root.join("B.nes"), b"another-rom").unwrap();

        let cancel = Arc::new(AtomicBool::new(false));
        let callback_cancel = Arc::clone(&cancel);
        let interrupted = scan_directory_cached(
            &discovery,
            &state,
            &rom_root,
            "",
            true,
            cancel.as_ref(),
            Arc::new(move |progress| {
                if progress.scanned_files == 1 {
                    callback_cancel.store(true, Ordering::Relaxed);
                }
            }),
        )
        .unwrap();
        assert!(interrupted.cancelled);
        assert_eq!(interrupted.results.len(), 1);
        assert_eq!(interrupted.cache_reused_files, 0);
        assert_eq!(interrupted.content_read_files, 1);

        let resumed_cancel = AtomicBool::new(false);
        let resumed = scan_directory_cached(
            &discovery,
            &state,
            &rom_root,
            "",
            true,
            &resumed_cancel,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert!(!resumed.cancelled);
        assert_eq!(resumed.results.len(), 2);
        assert_eq!(resumed.cache_reused_files, 1);
        assert_eq!(resumed.content_read_files, 1);
    }

    #[test]
    fn multi_rom_zip_members_are_reviewed_and_materialized_individually() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        zip_fixture(
            &rom_root.join("Collection.zip"),
            &[("One.nes", b"exact-rom"), ("Two.nes", b"another-rom")],
        );

        let cancel = AtomicBool::new(false);
        let mut output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 2);
        assert!(matches!(
            output.results[0].match_state,
            MatchState::Exact(_)
        ));
        assert!(!output.results[0].selected);
        assert_eq!(output.results[0].archive_member, "One.nes");
        assert_eq!(output.results[0].archive_member_count, 2);
        assert_eq!(output.results[0].match_method, "ZIP member SHA-1 + MD5");
        assert_eq!(output.results[1].match_state, MatchState::Unmatched);
        assert!(!output.results[1].selected);
        assert_eq!(output.results[1].archive_member, "Two.nes");
        assert_eq!(output.results[1].archive_member_count, 2);
        assert_eq!(output.results[1].platform, "Nintendo Entertainment System");

        output
            .results
            .iter_mut()
            .for_each(|result| result.selected = true);
        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 2);
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 2);
        let connection = Connection::open(&state).unwrap();
        let mut statement = connection
            .prepare(
                "SELECT path_display, archive_member, source_archive_path_display,
                        game_uid, included
                 FROM local_rom_files ORDER BY archive_member",
            )
            .unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, bool>(4)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1, "One.nes");
        assert_eq!(rows[0].2, rom_root.join("Collection.zip").to_string_lossy());
        assert_eq!(rows[0].3.as_deref(), Some("game-1"));
        assert!(rows[0].4);
        assert_eq!(fs::read(&rows[0].0).unwrap(), b"exact-rom");
        assert_eq!(rows[1].1, "Two.nes");
        assert_eq!(rows[1].2, rom_root.join("Collection.zip").to_string_lossy());
        assert_eq!(rows[1].3, None);
        assert!(rows[1].4);
        assert_eq!(fs::read(&rows[1].0).unwrap(), b"another-rom");
    }

    #[test]
    fn single_rom_seven_zip_member_receives_an_exact_checksum_identity() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let archive_path = rom_root.join("Exact Game.7z");
        seven_zip_fixture(
            &archive_path,
            &[
                ("__MACOSX/._Exact Game.nes", b"metadata"),
                ("docs/readme.txt", b"documentation"),
                ("roms/Exact Game (USA).nes", b"exact-rom"),
            ],
        );

        let cancel = AtomicBool::new(false);
        let output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 1);
        let result = &output.results[0];
        assert!(matches!(result.match_state, MatchState::Exact(_)));
        assert!(result.selected);
        assert_eq!(result.archive_member_count, 1);
        assert_eq!(result.archive_member, "roms/Exact Game (USA).nes");
        assert_eq!(result.file_size, 9);
        assert_eq!(result.platform, "Nintendo Entertainment System");
        assert_eq!(result.match_method, "7z archive member SHA-1 + MD5");

        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 1);
        let connection = Connection::open(state).unwrap();
        let stored_path = connection
            .query_row("SELECT path_display FROM local_rom_files", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap();
        assert_eq!(stored_path, archive_path.to_string_lossy());
    }

    #[test]
    fn multi_rom_seven_zip_members_are_reviewed_and_materialized_individually() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let archive_path = rom_root.join("Collection.7z");
        seven_zip_fixture(
            &archive_path,
            &[("One.nes", b"exact-rom"), ("Two.nes", b"another-rom")],
        );

        let cancel = AtomicBool::new(false);
        let mut output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 2);
        assert!(matches!(
            output.results[0].match_state,
            MatchState::Exact(_)
        ));
        assert!(!output.results[0].selected);
        assert_eq!(output.results[0].archive_member, "One.nes");
        assert_eq!(output.results[0].archive_member_count, 2);
        assert_eq!(
            output.results[0].match_method,
            "7z archive member SHA-1 + MD5"
        );
        assert_eq!(output.results[1].match_state, MatchState::Unmatched);
        assert!(!output.results[1].selected);
        assert_eq!(output.results[1].archive_member, "Two.nes");
        assert_eq!(output.results[1].archive_member_count, 2);

        output
            .results
            .iter_mut()
            .for_each(|result| result.selected = true);
        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 2);
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 2);
        let connection = Connection::open(&state).unwrap();
        let mut statement = connection
            .prepare(
                "SELECT path_display, archive_member, source_archive_path_display, game_uid
                 FROM local_rom_files ORDER BY archive_member",
            )
            .unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1, "One.nes");
        assert_eq!(rows[0].2, archive_path.to_string_lossy());
        assert_eq!(rows[0].3.as_deref(), Some("game-1"));
        assert_eq!(fs::read(&rows[0].0).unwrap(), b"exact-rom");
        assert_eq!(rows[1].1, "Two.nes");
        assert_eq!(rows[1].2, archive_path.to_string_lossy());
        assert_eq!(rows[1].3, None);
        assert_eq!(fs::read(&rows[1].0).unwrap(), b"another-rom");
    }

    #[test]
    fn reviewed_multi_rom_seven_zip_member_rejects_archive_changes_before_commit() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let archive_path = rom_root.join("Collection.7z");
        seven_zip_fixture(
            &archive_path,
            &[("One.nes", b"exact-rom"), ("Two.nes", b"another-rom")],
        );

        let cancel = AtomicBool::new(false);
        let mut output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        let exact = output
            .results
            .iter_mut()
            .find(|result| matches!(result.match_state, MatchState::Exact(_)))
            .unwrap();
        exact.selected = true;
        seven_zip_fixture(
            &archive_path,
            &[("One.nes", b"other-rom"), ("Two.nes", b"another-rom")],
        );

        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        let error = format!("{:#}", commit_scan_to(&state, &output).unwrap_err());
        assert!(error.contains("changed CRC32 after review"));
        let connection = Connection::open(&state).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM local_rom_files", [], |row| {
                    row.get::<_, usize>(0)
                })
                .unwrap(),
            0
        );
    }

    #[test]
    fn seven_zip_rejects_unsafe_member_paths() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        seven_zip_fixture(
            &rom_root.join("Unsafe.7z"),
            &[("../Escape.nes", b"exact-rom"), ("readme.txt", b"text")],
        );

        let cancel = AtomicBool::new(false);
        let output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 1);
        assert!(matches!(
            output.results[0].match_state,
            MatchState::Error(_)
        ));
        assert!(!output.results[0].selected);
        assert_eq!(output.results[0].archive_member_count, 0);
    }

    #[test]
    fn single_rom_rar_member_receives_an_exact_checksum_identity() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let archive_path = rom_root.join("Exact Game.rar");
        rar_fixture(
            &archive_path,
            &[
                ("__MACOSX/._Exact Game.nes", b"metadata"),
                ("docs/readme.txt", b"documentation"),
                ("roms/Exact Game (USA).nes", b"exact-rom"),
            ],
        );

        let cancel = AtomicBool::new(false);
        let output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 1);
        let result = &output.results[0];
        assert!(matches!(result.match_state, MatchState::Exact(_)));
        assert!(result.selected);
        assert_eq!(result.archive_member_count, 1);
        assert_eq!(result.archive_member, "roms/Exact Game (USA).nes");
        assert_eq!(result.file_size, 9);
        assert_eq!(result.platform, "Nintendo Entertainment System");
        assert_eq!(result.match_method, "RAR archive member SHA-1 + MD5");

        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 1);
        let connection = Connection::open(state).unwrap();
        let stored_path = connection
            .query_row("SELECT path_display FROM local_rom_files", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap();
        assert_eq!(stored_path, archive_path.to_string_lossy());
    }

    #[test]
    fn multi_rom_rar_members_are_reviewed_and_materialized_individually() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let archive_path = rom_root.join("Collection.rar");
        rar_fixture(
            &archive_path,
            &[("One.nes", b"exact-rom"), ("Two.nes", b"another-rom")],
        );

        let cancel = AtomicBool::new(false);
        let mut output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 2);
        assert!(matches!(
            output.results[0].match_state,
            MatchState::Exact(_)
        ));
        assert!(!output.results[0].selected);
        assert_eq!(output.results[0].archive_member, "One.nes");
        assert_eq!(output.results[0].archive_member_count, 2);
        assert_eq!(
            output.results[0].match_method,
            "RAR archive member SHA-1 + MD5"
        );
        assert_eq!(output.results[1].match_state, MatchState::Unmatched);
        assert!(!output.results[1].selected);

        output
            .results
            .iter_mut()
            .for_each(|result| result.selected = true);
        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 2);
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 2);
        let connection = Connection::open(&state).unwrap();
        let mut statement = connection
            .prepare(
                "SELECT path_display, archive_member, source_archive_path_display, game_uid
                 FROM local_rom_files ORDER BY archive_member",
            )
            .unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1, "One.nes");
        assert_eq!(rows[0].2, archive_path.to_string_lossy());
        assert_eq!(rows[0].3.as_deref(), Some("game-1"));
        assert_eq!(fs::read(&rows[0].0).unwrap(), b"exact-rom");
        assert_eq!(rows[1].1, "Two.nes");
        assert_eq!(rows[1].2, archive_path.to_string_lossy());
        assert_eq!(rows[1].3, None);
        assert_eq!(fs::read(&rows[1].0).unwrap(), b"another-rom");
    }

    #[test]
    fn reviewed_multi_rom_rar_member_rejects_archive_changes_before_commit() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let archive_path = rom_root.join("Collection.rar");
        rar_fixture(
            &archive_path,
            &[("One.nes", b"exact-rom"), ("Two.nes", b"another-rom")],
        );

        let cancel = AtomicBool::new(false);
        let mut output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        output
            .results
            .iter_mut()
            .find(|result| matches!(result.match_state, MatchState::Exact(_)))
            .unwrap()
            .selected = true;
        rar_fixture(
            &archive_path,
            &[("One.nes", b"other-rom"), ("Two.nes", b"another-rom")],
        );

        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        let error = format!("{:#}", commit_scan_to(&state, &output).unwrap_err());
        assert!(error.contains("changed CRC32 after review"));
        let connection = Connection::open(&state).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM local_rom_files", [], |row| {
                    row.get::<_, usize>(0)
                })
                .unwrap(),
            0
        );
    }

    #[test]
    fn encrypted_rar_is_an_actionable_error() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let archive_path = rom_root.join("Encrypted.rar");
        let mut archive = rars::Builder::new(rars::ArchiveVersion::Rar50)
            .store(true)
            .password(Some(b"private".to_vec()));
        archive
            .add_bytes(
                b"Exact Game.nes".to_vec(),
                b"exact-rom".to_vec(),
                None,
                None,
            )
            .unwrap();
        archive.write_to_path(&archive_path, None).unwrap();

        let cancel = AtomicBool::new(false);
        let output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 1);
        let MatchState::Error(error) = &output.results[0].match_state else {
            panic!("encrypted RAR was not rejected")
        };
        assert!(error.contains("encrypted"));
        assert!(!output.results[0].selected);
    }

    #[test]
    fn reviewed_multi_rom_member_rejects_archive_changes_before_commit() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let archive_path = rom_root.join("Collection.zip");
        zip_fixture(
            &archive_path,
            &[("One.nes", b"exact-rom"), ("Two.nes", b"another-rom")],
        );

        let cancel = AtomicBool::new(false);
        let mut output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        let exact = output
            .results
            .iter_mut()
            .find(|result| matches!(result.match_state, MatchState::Exact(_)))
            .unwrap();
        exact.selected = true;
        zip_fixture(
            &archive_path,
            &[("One.nes", b"other-rom"), ("Two.nes", b"another-rom")],
        );

        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        let error = commit_scan_to(&state, &output).unwrap_err().to_string();
        assert!(error.contains("changed CRC32 after review"));
        let connection = Connection::open(&state).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM local_rom_files", [], |row| {
                    row.get::<_, usize>(0)
                })
                .unwrap(),
            0
        );
        let cache_root = directory.path().join("local-rom-cache");
        assert_eq!(fs::read_dir(cache_root).unwrap().count(), 0);
    }

    #[test]
    fn zip_without_a_rom_member_is_an_actionable_error() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        zip_fixture(
            &rom_root.join("Documentation.zip"),
            &[("readme.txt", b"not a rom")],
        );

        let cancel = AtomicBool::new(false);
        let output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        let result = &output.results[0];
        assert!(matches!(result.match_state, MatchState::Error(_)));
        assert!(!result.selected);
        assert_eq!(result.match_method, "read error");
    }

    #[test]
    fn scans_and_commits_only_explicitly_selected_files() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        fs::write(rom_root.join("Exact Game (USA).nes"), b"exact-rom").unwrap();
        fs::write(rom_root.join("Mystery.nes"), b"unknown-rom").unwrap();
        discovery_fixture(&discovery, b"exact-rom");

        let cancel = AtomicBool::new(false);
        let mut output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(output.results.len(), 2);
        assert_eq!(
            output
                .results
                .iter()
                .filter(|result| matches!(result.match_state, MatchState::Exact(_)))
                .count(),
            1
        );
        output
            .results
            .iter_mut()
            .for_each(|result| result.selected = true);

        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 2);
        assert_eq!(commit_scan_to(&state, &output).unwrap(), 2);
        let connection = Connection::open(&state).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM local_rom_files WHERE included=1 AND availability='present'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            2
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM local_rom_files WHERE game_uid='game-1'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
        drop(connection);

        fs::remove_file(rom_root.join("Mystery.nes")).unwrap();
        let rescanned =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert_eq!(commit_scan_to(&state, &rescanned).unwrap(), 1);
        let connection = Connection::open(state).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM local_rom_files WHERE availability='present'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM local_rom_files WHERE availability='missing'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn cancellation_stops_before_hashing_more_files() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let rom_root = directory.path().join("roms");
        fs::create_dir(&rom_root).unwrap();
        fs::write(rom_root.join("Game.nes"), b"exact-rom").unwrap();
        discovery_fixture(&discovery, b"exact-rom");
        let cancel = AtomicBool::new(true);
        let output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        assert!(output.cancelled);
        assert!(output.results.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn native_path_round_trip_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_vec(vec![b'r', 0xff, b'm']));
        let encoded = encode_path(&path);
        assert_eq!(
            decode_path(encoded.bytes, encoded.encoding, encoded.display),
            path
        );
    }
}
