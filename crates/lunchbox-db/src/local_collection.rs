use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};
use crc32fast::Hasher as Crc32;
use md5::Md5;
use rusqlite::{Connection, params};
use serde::Serialize;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::ids::{normalize_key, sha256_bytes, stable_id};

const PROVIDER_SLUG: &str = "lunchbox-local";

#[derive(Debug, Default, Clone, Serialize)]
pub struct LocalScanStats {
    pub root_id: String,
    pub files_seen: usize,
    pub files_filtered: usize,
    pub bytes_hashed: u64,
    pub files_with_libretro_strong_match: usize,
    pub platforms_inferred_from_hash: usize,
    pub ambiguous_hash_platforms: usize,
    pub files_without_platform: usize,
    pub present_files: i64,
    pub missing_files: i64,
}

#[derive(Debug)]
struct EncodedPath {
    encoding: &'static str,
    bytes: Vec<u8>,
    display: String,
    key: String,
}

#[derive(Debug)]
struct FileDigests {
    crc32: String,
    md5: String,
    sha1: String,
    sha256: String,
    md5_bytes: Vec<u8>,
    sha1_bytes: Vec<u8>,
}

#[derive(Debug, Default)]
struct LibretroMatches {
    records: usize,
    platforms: BTreeSet<String>,
}

pub fn scan(
    connection: &mut Connection,
    database_path: &Path,
    root: &Path,
    platform_selector: Option<&str>,
    extensions: &[String],
    scan_timestamp: &str,
) -> Result<LocalScanStats> {
    if !root.is_dir() {
        bail!(
            "local collection root is not a directory: {}",
            root.display()
        );
    }
    let canonical_root = fs::canonicalize(root)
        .with_context(|| format!("resolving collection root {}", root.display()))?;
    let canonical_database = fs::canonicalize(database_path).ok();
    let encoded_root = encode_path(&canonical_root);
    let root_id = stable_id("collection-root", &encoded_root.key);
    let provider_id: String = connection
        .query_row(
            "SELECT id FROM providers WHERE slug=?1",
            [PROVIDER_SLUG],
            |row| row.get(0),
        )
        .context("local collection provider is not registered; initialize the schema first")?;
    let explicit_platform = platform_selector
        .map(|selector| resolve_one_platform(connection, selector))
        .transpose()?;
    let extension_filter: BTreeSet<String> = extensions
        .iter()
        .map(|extension| extension.trim().trim_start_matches('.').to_lowercase())
        .filter(|extension| !extension.is_empty())
        .collect();

    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO collection_roots
         (id, path_key, path_display, path_bytes, path_encoding, platform_id, recursive,
          last_scan_started_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)
         ON CONFLICT(path_key) DO UPDATE SET
           path_display=excluded.path_display,
           path_bytes=excluded.path_bytes,
           path_encoding=excluded.path_encoding,
           platform_id=excluded.platform_id,
           recursive=excluded.recursive,
           last_scan_started_at=excluded.last_scan_started_at",
        params![
            root_id,
            encoded_root.key,
            encoded_root.display,
            encoded_root.bytes,
            encoded_root.encoding,
            explicit_platform,
            scan_timestamp
        ],
    )?;
    transaction.execute(
        "UPDATE local_files SET availability='missing' WHERE root_id=?1",
        [&root_id],
    )?;

    let mut stats = LocalScanStats {
        root_id: root_id.clone(),
        ..LocalScanStats::default()
    };
    for entry in WalkDir::new(&canonical_root)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = entry.with_context(|| {
            format!("walking local collection root {}", canonical_root.display())
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        if canonical_database
            .as_ref()
            .is_some_and(|database| entry.path() == database)
        {
            continue;
        }
        if !extension_filter.is_empty() && !has_extension(entry.path(), &extension_filter) {
            stats.files_filtered += 1;
            continue;
        }

        let relative_path = entry
            .path()
            .strip_prefix(&canonical_root)
            .with_context(|| {
                format!(
                    "making {} relative to {}",
                    entry.path().display(),
                    canonical_root.display()
                )
            })?;
        let encoded_relative = encode_path(relative_path);
        let metadata = entry
            .metadata()
            .with_context(|| format!("reading file metadata for {}", entry.path().display()))?;
        let byte_size = metadata.len();
        let byte_size_i64 = i64::try_from(byte_size)
            .with_context(|| format!("file is too large: {}", entry.path().display()))?;
        let modified_unix_ns = metadata.modified().ok().and_then(|modified| {
            modified
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| i64::try_from(duration.as_nanos()).ok())
        });
        let digests = hash_file(entry.path())?;
        stats.files_seen += 1;
        stats.bytes_hashed = stats.bytes_hashed.saturating_add(byte_size);

        let artifact_id = stable_id("artifact", &format!("sha256:{}", digests.sha256));
        let file_name = entry.file_name().to_string_lossy().into_owned();
        transaction.execute(
            "INSERT INTO artifacts
             (id, artifact_type, file_name, byte_size, status, created_at)
             VALUES (?1, ?2, ?3, ?4, 'provisional', ?5)
             ON CONFLICT(id) DO UPDATE SET byte_size=excluded.byte_size",
            params![
                artifact_id,
                artifact_type(entry.path()),
                file_name,
                byte_size_i64,
                scan_timestamp
            ],
        )?;
        for (algorithm, digest) in [
            ("crc32", digests.crc32.as_str()),
            ("md5", digests.md5.as_str()),
            ("sha1", digests.sha1.as_str()),
            ("sha256", digests.sha256.as_str()),
        ] {
            transaction.execute(
                "INSERT OR IGNORE INTO artifact_hashes (artifact_id, algorithm, digest)
                 VALUES (?1, ?2, ?3)",
                params![artifact_id, algorithm, digest],
            )?;
        }

        let matches = find_libretro_strong_matches(&transaction, &digests)?;
        if matches.records > 0 {
            stats.files_with_libretro_strong_match += 1;
        }
        let inferred_platform = if explicit_platform.is_none() && matches.platforms.len() == 1 {
            stats.platforms_inferred_from_hash += 1;
            matches.platforms.iter().next().cloned()
        } else {
            None
        };
        if matches.platforms.len() > 1 {
            stats.ambiguous_hash_platforms += 1;
        }
        let selected_platform = explicit_platform.clone().or(inferred_platform);
        if selected_platform.is_none() {
            stats.files_without_platform += 1;
        }

        let external_id = format!("{root_id}:{}", encoded_relative.key);
        let source_record_id = stable_id(PROVIDER_SLUG, &format!("artifact\0{external_id}"));
        let local_file_id = stable_id("local-file", &external_id);
        let game_id = stable_id("game", &format!("local\0{source_record_id}"));
        remove_replaced_links_and_releases(&transaction, &source_record_id)?;

        let payload = serde_json::to_string(&serde_json::json!({
            "root_id": root_id,
            "relative_path": encoded_relative.display,
            "relative_path_key": encoded_relative.key,
            "relative_path_bytes_hex": hex::encode(&encoded_relative.bytes),
            "path_encoding": encoded_relative.encoding,
            "byte_size": byte_size,
            "modified_unix_ns": modified_unix_ns,
            "hashes": {
                "crc32": digests.crc32,
                "md5": digests.md5,
                "sha1": digests.sha1,
                "sha256": digests.sha256,
            },
            "libretro_strong_match_records": matches.records,
            "libretro_platform_candidates": matches.platforms,
            "selected_platform_id": selected_platform,
        }))?;
        transaction.execute(
            "INSERT INTO source_records
             (id, provider_id, source_path, record_type, external_id, payload_json,
              payload_sha256, imported_at)
             VALUES (?1, ?2, ?3, 'artifact', ?4, ?5, ?6, ?7)
             ON CONFLICT(provider_id, record_type, external_id) DO UPDATE SET
               source_path=excluded.source_path,
               payload_json=excluded.payload_json,
               payload_sha256=excluded.payload_sha256,
               imported_at=excluded.imported_at",
            params![
                source_record_id,
                provider_id,
                encoded_relative.display,
                external_id,
                payload,
                sha256_bytes(payload.as_bytes()),
                scan_timestamp
            ],
        )?;
        transaction.execute(
            "INSERT INTO local_files
             (id, root_id, source_record_id, artifact_id, relative_path_key,
              relative_path_display, relative_path_bytes, path_encoding, archive_member,
              byte_size, modified_unix_ns, availability, first_seen_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '', ?9, ?10, 'present', ?11, ?11)
             ON CONFLICT(root_id, relative_path_key, archive_member) DO UPDATE SET
               source_record_id=excluded.source_record_id,
               artifact_id=excluded.artifact_id,
               relative_path_display=excluded.relative_path_display,
               relative_path_bytes=excluded.relative_path_bytes,
               path_encoding=excluded.path_encoding,
               byte_size=excluded.byte_size,
               modified_unix_ns=excluded.modified_unix_ns,
               availability='present',
               last_seen_at=excluded.last_seen_at",
            params![
                local_file_id,
                root_id,
                source_record_id,
                artifact_id,
                encoded_relative.key,
                encoded_relative.display,
                encoded_relative.bytes,
                encoded_relative.encoding,
                byte_size_i64,
                modified_unix_ns,
                scan_timestamp
            ],
        )?;
        insert_source_link(
            &transaction,
            &source_record_id,
            "artifact",
            &artifact_id,
            "exact_hash",
            "accepted",
            Some(scan_timestamp),
        )?;

        let title = game_title(relative_path);
        transaction.execute(
            "INSERT INTO games
             (id, canonical_title, sort_title, status, created_at, updated_at)
             VALUES (?1, ?2, ?2, 'provisional', ?3, ?3)
             ON CONFLICT(id) DO UPDATE SET updated_at=excluded.updated_at",
            params![game_id, title, scan_timestamp],
        )?;
        insert_source_link(
            &transaction,
            &source_record_id,
            "game",
            &game_id,
            "source_native",
            "accepted",
            Some(scan_timestamp),
        )?;

        for candidate in &matches.platforms {
            if selected_platform.as_ref() == Some(candidate) {
                continue;
            }
            insert_source_link(
                &transaction,
                &source_record_id,
                "platform",
                candidate,
                "exact_hash",
                "pending",
                None,
            )?;
        }
        if let Some(platform_id) = &selected_platform {
            let platform_method = if explicit_platform.is_some() {
                "source_native"
            } else {
                "exact_hash"
            };
            insert_source_link(
                &transaction,
                &source_record_id,
                "platform",
                platform_id,
                platform_method,
                "accepted",
                Some(scan_timestamp),
            )?;
            let release_id = stable_id(
                "release",
                &format!("local\0{source_record_id}\0{platform_id}"),
            );
            transaction.execute(
                "INSERT INTO releases
                 (id, game_id, platform_id, release_title, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'provisional', ?5, ?5)",
                params![release_id, game_id, platform_id, title, scan_timestamp],
            )?;
            transaction.execute(
                "INSERT INTO release_artifacts (release_id, artifact_id, role)
                 VALUES (?1, ?2, 'primary')",
                params![release_id, artifact_id],
            )?;
            insert_source_link(
                &transaction,
                &source_record_id,
                "release",
                &release_id,
                "source_native",
                "accepted",
                Some(scan_timestamp),
            )?;
        }
        update_platform_quality_issue(
            &transaction,
            &game_id,
            &source_record_id,
            explicit_platform.as_deref(),
            selected_platform.as_deref(),
            &matches.platforms,
            scan_timestamp,
        )?;
    }

    transaction.execute(
        "UPDATE collection_roots SET last_scan_completed_at=?2 WHERE id=?1",
        params![root_id, scan_timestamp],
    )?;
    stats.present_files = transaction.query_row(
        "SELECT count(*) FROM local_files WHERE root_id=?1 AND availability='present'",
        [&root_id],
        |row| row.get(0),
    )?;
    stats.missing_files = transaction.query_row(
        "SELECT count(*) FROM local_files WHERE root_id=?1 AND availability='missing'",
        [&root_id],
        |row| row.get(0),
    )?;
    transaction.commit()?;
    Ok(stats)
}

fn hash_file(path: &Path) -> Result<FileDigests> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut crc32 = Crc32::new();
    let mut md5 = Md5::new();
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("hashing {}", path.display()))?;
        if read == 0 {
            break;
        }
        let bytes = &buffer[..read];
        crc32.update(bytes);
        md5.update(bytes);
        sha1.update(bytes);
        sha256.update(bytes);
    }
    let md5_bytes = md5.finalize().to_vec();
    let sha1_bytes = sha1.finalize().to_vec();
    Ok(FileDigests {
        crc32: format!("{:08x}", crc32.finalize()),
        md5: hex::encode(&md5_bytes),
        sha1: hex::encode(&sha1_bytes),
        sha256: hex::encode(sha256.finalize()),
        md5_bytes,
        sha1_bytes,
    })
}

fn find_libretro_strong_matches(
    connection: &Connection,
    digests: &FileDigests,
) -> Result<LibretroMatches> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT h.record_id, d.platform_id
         FROM libretro_hashes h
         JOIN libretro_records r ON r.id=h.record_id
         JOIN libretro_databases d ON d.id=r.database_id
         WHERE (h.algorithm='sha1' AND h.digest=?1)
            OR (h.algorithm='md5' AND h.digest=?2)
         ORDER BY h.record_id, d.platform_id",
    )?;
    let rows = statement.query_map(params![digests.sha1_bytes, digests.md5_bytes], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut record_ids = BTreeSet::new();
    let mut platforms = BTreeSet::new();
    for row in rows {
        let (record_id, platform_id) = row?;
        record_ids.insert(record_id);
        platforms.insert(platform_id);
    }
    Ok(LibretroMatches {
        records: record_ids.len(),
        platforms,
    })
}

fn remove_replaced_links_and_releases(
    connection: &Connection,
    source_record_id: &str,
) -> Result<()> {
    let mut statement = connection.prepare(
        "SELECT entity_id FROM source_links
         WHERE source_record_id=?1 AND entity_type='release'",
    )?;
    let release_ids = statement
        .query_map([source_record_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    connection.execute(
        "DELETE FROM source_links
         WHERE source_record_id=?1 AND entity_type IN ('artifact','platform','release')",
        [source_record_id],
    )?;
    for release_id in release_ids {
        connection.execute("DELETE FROM releases WHERE id=?1", [release_id])?;
    }
    Ok(())
}

fn update_platform_quality_issue(
    connection: &Connection,
    game_id: &str,
    source_record_id: &str,
    explicit_platform: Option<&str>,
    selected_platform: Option<&str>,
    candidates: &BTreeSet<String>,
    timestamp: &str,
) -> Result<()> {
    connection.execute(
        "DELETE FROM data_quality_issues
         WHERE entity_type='game' AND entity_id=?1
           AND issue_type IN (
             'local_platform_unresolved',
             'local_platform_hash_ambiguous',
             'local_platform_hash_disagreement'
           )",
        [game_id],
    )?;
    let issue_type = if explicit_platform.is_some()
        && !candidates.is_empty()
        && explicit_platform.is_some_and(|platform| !candidates.contains(platform))
    {
        "local_platform_hash_disagreement"
    } else if selected_platform.is_some() {
        return Ok(());
    } else if candidates.len() > 1 {
        "local_platform_hash_ambiguous"
    } else {
        "local_platform_unresolved"
    };
    let details = serde_json::json!({
        "source_record_id": source_record_id,
        "platform_candidates": candidates,
    });
    connection.execute(
        "INSERT INTO data_quality_issues
         (id, severity, issue_type, entity_type, entity_id, details_json, created_at)
         VALUES (?1, 'warning', ?2, 'game', ?3, ?4, ?5)",
        params![
            stable_id(
                "quality-issue",
                &format!("{issue_type}\0{source_record_id}")
            ),
            issue_type,
            game_id,
            details.to_string(),
            timestamp
        ],
    )?;
    Ok(())
}

fn insert_source_link(
    connection: &Connection,
    source_record_id: &str,
    entity_type: &str,
    entity_id: &str,
    method: &str,
    status: &str,
    decided_at: Option<&str>,
) -> Result<()> {
    connection.execute(
        "INSERT INTO source_links
         (source_record_id, entity_type, entity_id, confidence, method, status, decided_at)
         VALUES (?1, ?2, ?3, 1000, ?4, ?5, ?6)
         ON CONFLICT(source_record_id, entity_type, entity_id) DO UPDATE SET
           confidence=excluded.confidence,
           method=excluded.method,
           status=excluded.status,
           decided_at=excluded.decided_at",
        params![
            source_record_id,
            entity_type,
            entity_id,
            method,
            status,
            decided_at
        ],
    )?;
    Ok(())
}

fn resolve_one_platform(connection: &Connection, selector: &str) -> Result<String> {
    let key = normalize_key(selector);
    if key.is_empty() {
        bail!("platform selector has no usable identity characters");
    }
    let mut statement = connection.prepare(
        "SELECT id FROM platforms WHERE normalized_name=?1
         UNION
         SELECT platform_id FROM platform_aliases WHERE normalized_alias=?1
         ORDER BY 1",
    )?;
    let matches = statement
        .query_map([key], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    match matches.as_slice() {
        [platform_id] => Ok(platform_id.clone()),
        [] => bail!("platform selector did not match exactly: {selector}"),
        _ => bail!("platform selector is ambiguous: {selector}"),
    }
}

fn has_extension(path: &Path, extensions: &BTreeSet<String>) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extensions.contains(&extension.to_lowercase()))
}

fn game_title(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|name| name.to_string_lossy().trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Untitled local game".to_owned())
}

fn artifact_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_lowercase();
    match extension.as_str() {
        "7z" | "gz" | "rar" | "tar" | "tgz" | "zip" => "archive",
        "ccd" | "chd" | "cso" | "cue" | "gcz" | "gdi" | "iso" | "m3u" | "rvz" | "wbfs" | "wux" => {
            "disc"
        }
        "3ds" | "a26" | "a52" | "a78" | "gb" | "gba" | "gbc" | "gen" | "j64" | "lnx" | "md"
        | "n64" | "nds" | "nes" | "ngc" | "pce" | "sfc" | "smc" | "v64" | "vec" | "ws" | "wsc"
        | "z64" => "rom",
        "appimage" | "com" | "elf" | "exe" | "nro" | "self" | "xex" => "executable",
        _ => "other",
    }
}

fn encode_path(path: &Path) -> EncodedPath {
    let (encoding, bytes) = native_path_bytes(path);
    let key = sha256_bytes(&[encoding.as_bytes(), b"\0", &bytes].concat());
    EncodedPath {
        encoding,
        bytes,
        display: path.to_string_lossy().into_owned(),
        key,
    }
}

#[cfg(unix)]
fn native_path_bytes(path: &Path) -> (&'static str, Vec<u8>) {
    use std::os::unix::ffi::OsStrExt;

    ("unix_bytes", path.as_os_str().as_bytes().to_vec())
}

#[cfg(windows)]
fn native_path_bytes(path: &Path) -> (&'static str, Vec<u8>) {
    use std::os::windows::ffi::OsStrExt;

    let bytes = path
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect();
    ("windows_utf16le", bytes)
}

#[cfg(not(any(unix, windows)))]
fn native_path_bytes(path: &Path) -> (&'static str, Vec<u8>) {
    ("utf8", path.to_string_lossy().as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::database;

    #[test]
    fn scan_is_idempotent_and_tracks_missing_files() -> Result<()> {
        let workspace = tempdir()?;
        let root = workspace.path().join("roms");
        fs::create_dir(&root)?;
        let rom = root.join("Example.GB");
        fs::write(&rom, b"abc")?;
        fs::write(root.join("notes.txt"), b"ignored")?;
        let database_path = workspace.path().join("collection.db");
        database::initialize_path(&database_path)?;
        let mut connection = database::open_existing(&database_path)?;
        let platform_id = stable_id("platform", "nintendo-game-boy");
        connection.execute(
            "INSERT INTO platforms
             (id, canonical_name, normalized_name, status, created_at, updated_at)
             VALUES (?1, 'Nintendo Game Boy', 'nintendo-game-boy', 'canonical', ?2, ?2)",
            params![platform_id, "1970-01-01T00:00:00Z"],
        )?;

        let extensions = vec!["gb".to_owned()];
        let first = scan(
            &mut connection,
            &database_path,
            &root,
            Some("Nintendo Game Boy"),
            &extensions,
            "1970-01-01T00:00:00Z",
        )?;
        let second = scan(
            &mut connection,
            &database_path,
            &root,
            Some("Nintendo Game Boy"),
            &extensions,
            "1970-01-01T00:00:00Z",
        )?;
        assert_eq!(first.files_seen, 1);
        assert_eq!(first.files_filtered, 1);
        assert_eq!(second.present_files, 1);
        for table in ["games", "releases", "artifacts", "local_files"] {
            assert_eq!(
                connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))?,
                1
            );
        }
        assert_eq!(
            connection.query_row(
                "SELECT digest FROM artifact_hashes WHERE algorithm='sha256'",
                [],
                |row| row.get::<_, String>(0),
            )?,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        fs::remove_file(&rom)?;
        let missing = scan(
            &mut connection,
            &database_path,
            &root,
            Some("Nintendo Game Boy"),
            &extensions,
            "1970-01-01T00:00:00Z",
        )?;
        assert_eq!(missing.present_files, 0);
        assert_eq!(missing.missing_files, 1);
        Ok(())
    }

    #[test]
    fn hash_file_calculates_all_identity_digests_in_one_pass() -> Result<()> {
        let workspace = tempdir()?;
        let path = workspace.path().join("sample.bin");
        fs::write(&path, b"abc")?;
        let digests = hash_file(&path)?;
        assert_eq!(digests.crc32, "352441c2");
        assert_eq!(digests.md5, "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(digests.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            digests.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        Ok(())
    }
}
