use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde::Serialize;

use crate::database;
use crate::ids::{normalize_key, sha256_bytes, sha256_file, stable_id};

const PROVIDER_SLUG: &str = "minerva";

#[derive(Debug, Default, Clone, Serialize)]
pub struct MinervaImportStats {
    pub torrents: usize,
    pub platform_rows: usize,
    pub mapped_platform_rows: usize,
    pub ambiguous_platform_rows: usize,
    pub unmapped_platform_rows: usize,
    pub offers: i64,
    pub source_snapshot_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct MinervaPlatformPayload {
    minerva_platform: String,
    legacy_lunchbox_platform_id: Option<i64>,
    legacy_lunchbox_platform_name: Option<String>,
    rom_count: i64,
    resolved_platform_ids: Vec<String>,
}

pub fn import(
    connection: &mut Connection,
    catalog: &Path,
    import_timestamp: &str,
) -> Result<MinervaImportStats> {
    let provider: (String, String) = connection
        .query_row(
            "SELECT id, data_license FROM providers WHERE slug=?1",
            [PROVIDER_SLUG],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .context("Minerva provider is not registered; initialize the schema first")?;
    let source_hash = sha256_file(catalog)
        .with_context(|| format!("hashing Minerva catalog {}", catalog.display()))?;
    let source_bytes = fs::metadata(catalog)
        .with_context(|| format!("reading Minerva catalog metadata {}", catalog.display()))?
        .len();
    let source_bytes = i64::try_from(source_bytes).context("Minerva catalog is too large")?;
    let revision = format!("sha256:{source_hash}");
    let snapshot_id = stable_id("source-snapshot", &format!("{PROVIDER_SLUG}\0{revision}"));
    let source_uri = format!(
        "local-file:{}",
        catalog
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("minerva.db")
    );

    let source = database::open_read_only(catalog)?;
    validate_catalog(&source)?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO source_snapshots
         (id, provider_id, revision, source_uri, content_sha256, content_bytes, data_license, imported_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(provider_id, revision) DO UPDATE SET
           source_uri=excluded.source_uri,
           content_sha256=excluded.content_sha256,
           content_bytes=excluded.content_bytes,
           data_license=excluded.data_license,
           imported_at=excluded.imported_at",
        params![
            snapshot_id,
            provider.0,
            revision,
            source_uri,
            source_hash,
            source_bytes,
            provider.1,
            import_timestamp
        ],
    )?;

    let mut torrent_statement = source.prepare(
        "SELECT id, torrent_file, torrent_url, collection, rom_count, total_size
         FROM minerva_torrents ORDER BY torrent_file",
    )?;
    let torrents = torrent_statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    let mut platform_statement = source.prepare(
        "SELECT minerva_platform, lunchbox_platform_id, lunchbox_platform_name, rom_count
         FROM minerva_torrent_platforms WHERE torrent_id=?1 ORDER BY minerva_platform",
    )?;

    let mut stats = MinervaImportStats {
        source_snapshot_sha256: source_hash.clone(),
        ..MinervaImportStats::default()
    };
    for torrent in torrents {
        let (source_id, torrent_file, torrent_url, collection, rom_count, total_size) = torrent?;
        if torrent_file.trim().is_empty() || torrent_url.trim().is_empty() {
            bail!("Minerva torrent {source_id} has a blank file or URL");
        }
        if rom_count < 0 || total_size < 0 {
            bail!("Minerva torrent {source_id} has a negative count or size");
        }

        let platform_rows = platform_statement.query_map([source_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        let mut platforms = Vec::new();
        let mut resolved_platforms = BTreeMap::<String, String>::new();
        for platform_row in platform_rows {
            let (raw_name, legacy_id, legacy_name, platform_rom_count) = platform_row?;
            if raw_name.trim().is_empty() || platform_rom_count < 0 {
                bail!("Minerva torrent {source_id} has an invalid platform row");
            }
            stats.platform_rows += 1;
            let lookup_name = legacy_name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(&raw_name);
            let resolved = resolve_exact_platforms(&transaction, lookup_name)?;
            match resolved.len() {
                0 => stats.unmapped_platform_rows += 1,
                1 => stats.mapped_platform_rows += 1,
                _ => stats.ambiguous_platform_rows += 1,
            }
            for (id, canonical_name) in &resolved {
                resolved_platforms.insert(id.clone(), canonical_name.clone());
            }
            platforms.push(MinervaPlatformPayload {
                minerva_platform: raw_name,
                legacy_lunchbox_platform_id: legacy_id,
                legacy_lunchbox_platform_name: legacy_name,
                rom_count: platform_rom_count,
                resolved_platform_ids: resolved.into_iter().map(|(id, _)| id).collect(),
            });
        }

        let platform_hint = if resolved_platforms.len() == 1 {
            resolved_platforms.values().next().cloned()
        } else if platforms.len() == 1 {
            Some(platforms[0].minerva_platform.clone())
        } else {
            None
        };
        let display_name = torrent_display_name(&torrent_file);
        let payload = serde_json::to_string(&serde_json::json!({
            "record_kind": "torrent_bundle",
            "source_id": source_id,
            "torrent_file": torrent_file,
            "torrent_url": torrent_url,
            "collection": collection,
            "rom_count": rom_count,
            "total_size": total_size,
            "platforms": platforms,
        }))?;
        let external_id = format!("bundle:{torrent_file}");
        let offer_id = stable_id("offer", &format!("{PROVIDER_SLUG}\0{external_id}"));
        let source_record_id = stable_id(PROVIDER_SLUG, &format!("offer\0{external_id}"));

        transaction.execute(
            "INSERT INTO acquisition_offers
             (id, provider_id, external_id, display_name, platform_hint, byte_size,
              availability, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unknown', ?7)
             ON CONFLICT(provider_id, external_id) DO UPDATE SET
               display_name=excluded.display_name,
               platform_hint=excluded.platform_hint,
               byte_size=excluded.byte_size,
               availability=excluded.availability,
               metadata_json=excluded.metadata_json",
            params![
                offer_id,
                provider.0,
                external_id,
                display_name,
                platform_hint,
                total_size,
                payload
            ],
        )?;
        transaction.execute(
            "INSERT INTO source_records
             (id, provider_id, source_snapshot_id, source_path, source_ordinal, record_type,
              external_id, payload_json, payload_sha256, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'offer', ?6, ?7, ?8, ?9)
             ON CONFLICT(provider_id, record_type, external_id) DO UPDATE SET
               source_snapshot_id=excluded.source_snapshot_id,
               source_path=excluded.source_path,
               source_ordinal=excluded.source_ordinal,
               payload_json=excluded.payload_json,
               payload_sha256=excluded.payload_sha256,
               imported_at=excluded.imported_at",
            params![
                source_record_id,
                provider.0,
                snapshot_id,
                torrent_file,
                source_id,
                external_id,
                payload,
                sha256_bytes(payload.as_bytes()),
                import_timestamp
            ],
        )?;
        insert_source_link(
            &transaction,
            &source_record_id,
            "offer",
            &offer_id,
            "source_native",
            "accepted",
            Some(import_timestamp),
        )?;
        for platform_id in resolved_platforms.keys() {
            insert_source_link(
                &transaction,
                &source_record_id,
                "platform",
                platform_id,
                "exact_name",
                "accepted",
                Some(import_timestamp),
            )?;
        }
        stats.torrents += 1;
    }

    transaction.execute(
        "DELETE FROM acquisition_offers
         WHERE id IN (
           SELECT l.entity_id FROM source_links l
           JOIN source_records r ON r.id=l.source_record_id
           WHERE r.provider_id=?1 AND r.record_type='offer'
             AND r.source_snapshot_id<>?2
             AND json_extract(r.payload_json, '$.record_kind')='torrent_bundle'
             AND l.entity_type='offer'
         )",
        params![provider.0, snapshot_id],
    )?;
    transaction.execute(
        "DELETE FROM source_records
         WHERE provider_id=?1 AND record_type='offer' AND source_snapshot_id<>?2
           AND json_extract(payload_json, '$.record_kind')='torrent_bundle'",
        params![provider.0, snapshot_id],
    )?;

    let ending_hash = sha256_file(catalog)
        .with_context(|| format!("rechecking Minerva catalog {}", catalog.display()))?;
    if ending_hash != source_hash {
        bail!(
            "Minerva catalog changed during import: {}",
            catalog.display()
        );
    }

    transaction.commit()?;
    stats.offers = connection.query_row(
        "SELECT count(*) FROM acquisition_offers o
         WHERE o.provider_id=?1 AND EXISTS (
           SELECT 1 FROM source_links l JOIN source_records r ON r.id=l.source_record_id
           WHERE l.entity_type='offer' AND l.entity_id=o.id
             AND json_extract(r.payload_json, '$.record_kind')='torrent_bundle'
         )",
        [provider.0],
        |row| row.get(0),
    )?;
    Ok(stats)
}

fn validate_catalog(connection: &Connection) -> Result<()> {
    for table in ["minerva_torrents", "minerva_torrent_platforms"] {
        let exists: i64 = connection.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )?;
        if exists != 1 {
            bail!("Minerva catalog is missing required table {table}");
        }
    }
    Ok(())
}

fn resolve_exact_platforms(
    connection: &Connection,
    platform_name: &str,
) -> Result<Vec<(String, String)>> {
    let key = normalize_key(platform_name);
    if key.is_empty() {
        return Ok(Vec::new());
    }
    let mut statement = connection.prepare(
        "SELECT id, canonical_name FROM platforms WHERE normalized_name=?1
         UNION
         SELECT p.id, p.canonical_name FROM platform_aliases a
         JOIN platforms p ON p.id=a.platform_id WHERE a.normalized_alias=?1
         ORDER BY 1",
    )?;
    let rows = statement.query_map([key], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn torrent_display_name(torrent_file: &str) -> String {
    let file_name = torrent_file.rsplit('/').next().unwrap_or(torrent_file);
    file_name
        .strip_suffix(".torrent")
        .unwrap_or(file_name)
        .strip_prefix("Minerva_Myrient - ")
        .or_else(|| {
            file_name
                .strip_suffix(".torrent")
                .unwrap_or(file_name)
                .strip_prefix("Minerva_Myrient_v0.3 - ")
        })
        .unwrap_or_else(|| file_name.strip_suffix(".torrent").unwrap_or(file_name))
        .to_owned()
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

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn import_is_idempotent_and_only_accepts_exact_platforms() -> Result<()> {
        let catalog = NamedTempFile::new()?;
        let source = Connection::open(catalog.path())?;
        source.execute_batch(
            "CREATE TABLE minerva_torrents (
               id INTEGER PRIMARY KEY, torrent_file TEXT NOT NULL UNIQUE,
               torrent_url TEXT NOT NULL, collection TEXT,
               rom_count INTEGER DEFAULT 0, total_size INTEGER DEFAULT 0
             );
             CREATE TABLE minerva_torrent_platforms (
               torrent_id INTEGER NOT NULL, minerva_platform TEXT NOT NULL,
               lunchbox_platform_id INTEGER, lunchbox_platform_name TEXT,
               rom_count INTEGER DEFAULT 0,
               PRIMARY KEY (torrent_id, minerva_platform)
             );
             INSERT INTO minerva_torrents VALUES
               (1, 'set/Minerva_Myrient - Nintendo - Game Boy.torrent',
                'https://example.invalid/gb.torrent', 'No-Intro', 2, 42),
               (2, 'set/Unknown.torrent', 'https://example.invalid/x.torrent', NULL, 1, 7);
             INSERT INTO minerva_torrent_platforms VALUES
               (1, 'Nintendo - Game Boy', 1, 'Nintendo Game Boy', 2),
               (2, 'An Unmapped Platform', NULL, NULL, 1);",
        )?;
        drop(source);

        let mut target = Connection::open_in_memory()?;
        database::configure(&target)?;
        database::initialize_connection(&target, "1970-01-01T00:00:00Z")?;
        database::seed_providers(&target)?;
        let platform_id = stable_id("platform", "nintendo-game-boy");
        target.execute(
            "INSERT INTO platforms
             (id, canonical_name, normalized_name, status, created_at, updated_at)
             VALUES (?1, 'Nintendo Game Boy', 'nintendo-game-boy', 'canonical', ?2, ?2)",
            params![platform_id, "1970-01-01T00:00:00Z"],
        )?;

        let first = import(&mut target, catalog.path(), "1970-01-01T00:00:00Z")?;
        let second = import(&mut target, catalog.path(), "1970-01-01T00:00:00Z")?;
        assert_eq!(first.torrents, 2);
        assert_eq!(first.mapped_platform_rows, 1);
        assert_eq!(first.unmapped_platform_rows, 1);
        assert_eq!(second.offers, 2);
        assert_eq!(
            target.query_row("SELECT count(*) FROM source_records", [], |row| row
                .get::<_, i64>(0))?,
            2
        );
        assert_eq!(
            target.query_row(
                "SELECT count(*) FROM source_links WHERE entity_type='platform' AND status='accepted'",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            1
        );
        Ok(())
    }

    #[test]
    fn provider_paths_are_not_interpreted_as_host_paths() {
        assert_eq!(
            torrent_display_name("remote/path/Minerva_Myrient - Sega - Saturn.torrent"),
            "Sega - Saturn"
        );
    }
}
