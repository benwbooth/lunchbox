use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};
use crc32fast::Hasher as Crc32;
use md5::Md5;
use rusqlite::{Connection, params};
use sha1::{Digest, Sha1};
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
    Ambiguous(usize),
    Unmatched,
    InventoryOnly,
    Error(String),
}

impl MatchState {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Exact(_) => "exact",
            Self::Ambiguous(_) => "ambiguous",
            Self::Unmatched => "unmatched",
            Self::InventoryOnly => "inventory_only",
            Self::Error(_) => "error",
        }
    }
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
}

#[derive(Clone, Debug)]
pub struct ScanOutput {
    pub root: PathBuf,
    pub platform_hint: String,
    pub checksums_enabled: bool,
    pub results: Vec<ScanResult>,
    pub walk_errors: usize,
    pub cancelled: bool,
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
enum ZipInspection {
    Empty,
    Multiple {
        member_count: usize,
        inferred_platform: Option<String>,
    },
    Single {
        member_name: String,
        extension: String,
        size: u64,
        digests: Option<Digests>,
    },
}

pub fn load_platform_names(discovery_path: &Path) -> Result<Vec<String>> {
    let connection = catalog::open_read_only(discovery_path, "Lunchbox discovery database")?;
    let mut statement =
        connection.prepare("SELECT name FROM platforms ORDER BY name COLLATE NOCASE")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
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

pub fn scan_directory(
    discovery_path: &Path,
    root: &Path,
    platform_hint: &str,
    checksums_enabled: bool,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(ScanProgress) + Send + Sync>,
) -> Result<ScanOutput> {
    let root = root
        .canonicalize()
        .with_context(|| format!("opening ROM directory {}", root.display()))?;
    if !root.is_dir() {
        bail!("ROM import path is not a directory: {}", root.display());
    }
    let index = load_match_index(discovery_path)?;
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
        if extension
            .as_ref()
            .is_some_and(|extension| index.extensions.contains(extension))
        {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    progress(ScanProgress {
        total_files: paths.len(),
        ..ScanProgress::default()
    });

    let mut results = Vec::with_capacity(paths.len());
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
                report_progress(&progress, paths.len(), position, file_name);
                continue;
            }
        };
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let zip_inspection = if extension == "zip" {
            match inspect_zip(path, &index, checksums_enabled, cancelled) {
                Ok(Some(inspection)) => Some(inspection),
                Ok(None) => break,
                Err(error) => {
                    results.push(error_result(&root, path, error.to_string()));
                    report_progress(&progress, paths.len(), position, file_name);
                    continue;
                }
            }
        } else {
            None
        };
        let mut archive_member = String::new();
        let mut archive_member_count = 0usize;
        let mut identity_file_name = file_name.clone();
        let mut identity_file_size = metadata.len();
        let mut identity_extension = extension.clone();
        let mut archive_platform = None;
        let mut predetermined_match = None;
        let digests = match zip_inspection {
            Some(ZipInspection::Empty) => {
                predetermined_match = Some((
                    MatchState::Error("ZIP contains no recognized ROM members".to_owned()),
                    "archive inspection".to_owned(),
                ));
                None
            }
            Some(ZipInspection::Multiple {
                member_count,
                inferred_platform,
            }) => {
                archive_member_count = member_count;
                archive_platform = inferred_platform;
                predetermined_match = Some((
                    MatchState::InventoryOnly,
                    format!("ZIP contains {member_count} ROM members; review required"),
                ));
                None
            }
            Some(ZipInspection::Single {
                member_name,
                extension,
                size,
                digests,
            }) => {
                archive_member = member_name.clone();
                archive_member_count = 1;
                identity_file_name = member_name;
                identity_file_size = size;
                identity_extension = extension;
                digests
            }
            None if checksums_enabled => match hash_file(path, cancelled) {
                Ok(Some(digests)) => Some(digests),
                Ok(None) => break,
                Err(error) => {
                    results.push(error_result(&root, path, error.to_string()));
                    report_progress(&progress, paths.len(), position, file_name);
                    continue;
                }
            },
            None => None,
        };
        let inferred_platform = if platform_hint.is_empty() {
            archive_platform.or_else(|| {
                index
                    .extension_platforms
                    .get(&identity_extension)
                    .and_then(|names| (names.len() == 1).then(|| names[0].clone()))
            })
        } else {
            Some(platform_hint.to_owned())
        };
        let (match_state, mut match_method) = if let Some(value) = predetermined_match {
            value
        } else if let Some(digests) = &digests {
            match_digest(&index, &digests.sha1, &digests.md5, platform_hint)
        } else {
            (MatchState::InventoryOnly, "not checked".to_owned())
        };
        if !archive_member.is_empty() {
            match_method = format!("ZIP member {match_method}");
        }
        let platform = match &match_state {
            MatchState::Exact(identity) => identity.platform.clone(),
            _ => inferred_platform.unwrap_or_else(|| "Unassigned".to_owned()),
        };
        let modified_unix_ns = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .and_then(|value| i64::try_from(value.as_nanos()).ok());
        results.push(ScanResult {
            path: path.clone(),
            relative_path: path.strip_prefix(&root).unwrap_or(path).to_path_buf(),
            file_name: file_name.clone(),
            archive_member,
            archive_member_count,
            display_title: display_title(&identity_file_name),
            platform,
            file_size: identity_file_size,
            modified_unix_ns,
            crc32: digests
                .as_ref()
                .map(|value| value.crc32.clone())
                .unwrap_or_default(),
            md5: digests
                .as_ref()
                .map(|value| value.md5.clone())
                .unwrap_or_default(),
            sha1: digests
                .as_ref()
                .map(|value| value.sha1.clone())
                .unwrap_or_default(),
            selected: matches!(match_state, MatchState::Exact(_)),
            match_state,
            match_method,
        });
        if position % 8 == 0 || position + 1 == paths.len() {
            report_progress(&progress, paths.len(), position, file_name);
        }
    }

    Ok(ScanOutput {
        root,
        platform_hint: platform_hint.to_owned(),
        checksums_enabled,
        results,
        walk_errors,
        cancelled: cancelled.load(Ordering::Relaxed),
    })
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
        .context("no single-ROM ZIP member received an exact checksum identity")?;
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

fn report_progress(
    progress: &Arc<dyn Fn(ScanProgress) + Send + Sync>,
    total_files: usize,
    position: usize,
    current_file: String,
) {
    progress(ScanProgress {
        total_files,
        scanned_files: position.saturating_add(1),
        current_file,
    });
}

#[derive(Clone, Debug)]
struct Digests {
    crc32: String,
    md5: String,
    sha1: String,
}

fn inspect_zip(
    path: &Path,
    index: &MatchIndex,
    checksums_enabled: bool,
    cancelled: &AtomicBool,
) -> Result<Option<ZipInspection>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("reading ZIP directory {}", path.display()))?;
    let mut members = Vec::new();
    for member_index in 0..archive.len() {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let entry = archive
            .by_index(member_index)
            .with_context(|| format!("reading ZIP member #{member_index} in {}", path.display()))?;
        let Some(member_path) = entry.enclosed_name() else {
            continue;
        };
        if entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            continue;
        }
        let member_name = member_path.to_string_lossy().replace('\\', "/");
        let file_name = member_path
            .file_name()
            .map(|value| value.to_string_lossy())
            .unwrap_or_default();
        if member_name
            .split('/')
            .next()
            .is_some_and(|component| component == "__MACOSX")
            || file_name.starts_with("._")
        {
            continue;
        }
        let extension = member_path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if extension.is_empty()
            || ARCHIVE_EXTENSIONS.contains(&extension.as_str())
            || !index.extensions.contains(&extension)
        {
            continue;
        }
        members.push((member_index, member_name, extension, entry.size()));
    }

    match members.len() {
        0 => Ok(Some(ZipInspection::Empty)),
        1 => {
            let (member_index, member_name, extension, size) = members.remove(0);
            let digests = if checksums_enabled {
                let mut entry = archive.by_index(member_index).with_context(|| {
                    format!("reopening ZIP member {member_name} in {}", path.display())
                })?;
                if entry.encrypted() {
                    bail!(
                        "ZIP member {member_name} in {} is encrypted and cannot be verified",
                        path.display()
                    );
                }
                match hash_reader(
                    &mut entry,
                    cancelled,
                    &format!("ZIP member {member_name} in {}", path.display()),
                )? {
                    Some(digests) => Some(digests),
                    None => return Ok(None),
                }
            } else {
                None
            };
            Ok(Some(ZipInspection::Single {
                member_name,
                extension,
                size,
                digests,
            }))
        }
        member_count => {
            let mut platforms = HashSet::new();
            let mut all_members_have_one_platform = true;
            for (_, _, extension, _) in &members {
                let Some(names) = index.extension_platforms.get(extension) else {
                    all_members_have_one_platform = false;
                    break;
                };
                if names.len() != 1 {
                    all_members_have_one_platform = false;
                    break;
                }
                platforms.insert(names[0].clone());
            }
            let inferred_platform = (all_members_have_one_platform && platforms.len() == 1)
                .then(|| platforms.into_iter().next())
                .flatten();
            Ok(Some(ZipInspection::Multiple {
                member_count,
                inferred_platform,
            }))
        }
    }
}

fn hash_file(path: &Path, cancelled: &AtomicBool) -> Result<Option<Digests>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    hash_reader(&mut reader, cancelled, &path.display().to_string())
}

fn hash_reader(
    reader: &mut impl Read,
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
    transaction.execute(
        "UPDATE local_rom_files SET availability='missing' WHERE root_id=?1",
        [&root_id],
    )?;

    let mut imported = 0usize;
    for result in &output.results {
        let path = encode_path(&result.path);
        let relative = encode_path(&result.relative_path);
        let mut identity_bytes = root_id.as_bytes().to_vec();
        identity_bytes.push(0);
        identity_bytes.extend_from_slice(&relative.bytes);
        let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, &identity_bytes).to_string();
        let (game_uid, database_id, matched_title) = match &result.match_state {
            MatchState::Exact(identity) => (
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
                 file_name, display_title, platform, file_size, modified_unix_ns,
                 crc32, md5, sha1, game_uid, launchbox_db_id, matched_title,
                 match_state, match_method, included, availability, imported_at
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                 ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, 'present', ?23
             )
             ON CONFLICT(id) DO UPDATE SET
                 path_display=excluded.path_display, path_bytes=excluded.path_bytes,
                 path_encoding=excluded.path_encoding,
                 relative_path_display=excluded.relative_path_display,
                 relative_path_bytes=excluded.relative_path_bytes,
                 relative_path_encoding=excluded.relative_path_encoding,
                 file_name=excluded.file_name, display_title=excluded.display_title,
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
    use std::io::Write;

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
                 INSERT INTO platforms VALUES (1, 'Nintendo Entertainment System', 'nes, zip');",
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
    fn multi_rom_zip_is_kept_for_explicit_review_without_guessing_identity() {
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
        let output =
            scan_directory(&discovery, &rom_root, "", true, &cancel, Arc::new(|_| {})).unwrap();
        let result = &output.results[0];
        assert_eq!(result.match_state, MatchState::InventoryOnly);
        assert!(!result.selected);
        assert_eq!(result.archive_member_count, 2);
        assert!(result.archive_member.is_empty());
        assert_eq!(result.platform, "Nintendo Entertainment System");
        assert_eq!(
            result.match_method,
            "ZIP contains 2 ROM members; review required"
        );
        assert!(result.sha1.is_empty());
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
        assert_eq!(result.match_method, "archive inspection");
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
