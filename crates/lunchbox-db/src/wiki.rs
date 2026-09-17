//! Hand-curated EmulationWiki recommendation rankings.
//!
//! The wiki's per-system comparison tables carry a "Recommended?" verdict and
//! an implicit preference order. This importer applies a pinned snapshot
//! (`sources/emulationwiki-recommendations.json`) to existing
//! `emulator_platforms` rows using exact name matches only: unknown
//! platforms, emulators, or cores fail the import instead of guessing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::ids::{sha256_file, stable_id};

pub const PROVIDER_SLUG: &str = "emulationwiki";
const PROVIDER_NAME: &str = "Emulation General Wiki";
const PROVIDER_HOMEPAGE: &str = "https://emulation.gametechwiki.com/";
const PROVIDER_LICENSE: &str = "CC BY-SA (version as published; attribution + share-alike)";
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
struct RecommendationSnapshot {
    schema_version: u32,
    retrieved_at: String,
    license: String,
    entries: Vec<RecommendationEntry>,
}

#[derive(Debug, Deserialize)]
struct RecommendationEntry {
    platform: String,
    emulator: String,
    core: String,
    rank: i64,
    verdict: String,
    note: String,
    page_url: String,
}

#[derive(Debug, Serialize)]
pub struct WikiRankStats {
    pub entries: usize,
    pub updated: usize,
}

pub fn import(
    connection: &mut Connection,
    source: &Path,
    import_timestamp: &str,
) -> Result<WikiRankStats> {
    let bytes = std::fs::read(source)
        .with_context(|| format!("opening wiki recommendation snapshot {}", source.display()))?;
    let snapshot: RecommendationSnapshot = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing wiki recommendation snapshot {}", source.display()))?;
    if snapshot.schema_version != SUPPORTED_SCHEMA_VERSION {
        bail!(
            "unsupported wiki recommendation schema version {}; expected {}",
            snapshot.schema_version,
            SUPPORTED_SCHEMA_VERSION
        );
    }
    if snapshot.retrieved_at.trim().is_empty() {
        bail!("wiki recommendation snapshot has no retrieval date");
    }
    let mut ranks_by_platform = BTreeMap::<String, BTreeSet<i64>>::new();
    for (position, entry) in snapshot.entries.iter().enumerate() {
        let row = position + 1;
        if entry.platform.trim().is_empty() || entry.emulator.trim().is_empty() {
            bail!("wiki recommendation row {row} names no platform or emulator");
        }
        if entry.rank < 1 {
            bail!("wiki recommendation row {row} has no positive rank");
        }
        if !matches!(entry.verdict.as_str(), "recommended" | "partial" | "not") {
            bail!("wiki recommendation row {row} has verdict {:?}", entry.verdict);
        }
        if !entry.page_url.starts_with("https://emulation.gametechwiki.com/") {
            bail!("wiki recommendation row {row} links outside the wiki");
        }
        if !ranks_by_platform
            .entry(entry.platform.clone())
            .or_default()
            .insert(entry.rank)
        {
            bail!(
                "wiki recommendations reuse rank {} for {:?}",
                entry.rank,
                entry.platform
            );
        }
    }

    let provider_id = stable_id("provider", PROVIDER_SLUG);
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO providers (id, slug, name, homepage, data_license, redistribution_policy)
         VALUES (?1, ?2, ?3, ?4, ?5, 'allowed')
         ON CONFLICT(slug) DO UPDATE SET
           name = excluded.name, homepage = excluded.homepage,
           data_license = excluded.data_license,
           redistribution_policy = excluded.redistribution_policy",
        params![
            provider_id,
            PROVIDER_SLUG,
            PROVIDER_NAME,
            PROVIDER_HOMEPAGE,
            PROVIDER_LICENSE
        ],
    )?;
    let snapshot_id = stable_id(PROVIDER_SLUG, &snapshot.retrieved_at);
    transaction.execute(
        "INSERT INTO source_snapshots
         (id, provider_id, revision, source_uri, content_sha256, content_bytes, data_license, imported_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(provider_id, revision) DO UPDATE SET
           source_uri=excluded.source_uri, content_sha256=excluded.content_sha256,
           content_bytes=excluded.content_bytes, data_license=excluded.data_license,
           imported_at=excluded.imported_at",
        params![
            snapshot_id,
            provider_id,
            snapshot.retrieved_at,
            source.display().to_string(),
            sha256_file(source).with_context(|| format!(
                "hashing wiki recommendation snapshot {}",
                source.display()
            ))?,
            i64::try_from(bytes.len()).unwrap_or(i64::MAX),
            PROVIDER_LICENSE,
            import_timestamp
        ],
    )?;

    let mut updated = 0_usize;
    for entry in &snapshot.entries {
        let platform_id: String = transaction
            .query_row(
                "SELECT id FROM platforms WHERE canonical_name = ?1",
                [&entry.platform],
                |row| row.get(0),
            )
            .with_context(|| {
                format!(
                    "wiki recommendations name an unknown platform {:?}; add it to the catalog first",
                    entry.platform
                )
            })?;
        let emulator_id: String = transaction
            .query_row(
                "SELECT id FROM emulators WHERE name = ?1",
                [&entry.emulator],
                |row| row.get(0),
            )
            .with_context(|| {
                format!(
                    "wiki recommendations name an unknown emulator {:?}; add it to the catalog first",
                    entry.emulator
                )
            })?;
        let changed = transaction.execute(
            "UPDATE emulator_platforms SET wiki_rank = ?1, wiki_verdict = ?2
             WHERE emulator_id = ?3 AND platform_id = ?4 AND core_name = ?5",
            params![
                entry.rank,
                entry.verdict,
                emulator_id,
                platform_id,
                entry.core.trim()
            ],
        )?;
        if changed != 1 {
            bail!(
                "wiki recommendations have no {:?} / {:?} / core {:?} mapping to rank; add the mapping first",
                entry.platform,
                entry.emulator,
                entry.core
            );
        }
        updated += 1;
    }
    transaction.commit()?;
    Ok(WikiRankStats {
        entries: snapshot.entries.len(),
        updated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_database() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE providers (
                   id TEXT PRIMARY KEY, slug TEXT NOT NULL UNIQUE, name TEXT NOT NULL,
                   homepage TEXT, data_license TEXT NOT NULL,
                   redistribution_policy TEXT NOT NULL
                 );
                 CREATE TABLE source_snapshots (
                   id TEXT PRIMARY KEY, provider_id TEXT NOT NULL REFERENCES providers(id),
                   revision TEXT NOT NULL, source_uri TEXT NOT NULL, content_sha256 TEXT NOT NULL,
                   content_bytes INTEGER NOT NULL, data_license TEXT NOT NULL, imported_at TEXT NOT NULL,
                   UNIQUE (provider_id, revision)
                 );
                 CREATE TABLE platforms (id TEXT PRIMARY KEY, canonical_name TEXT NOT NULL);
                 CREATE TABLE emulators (id TEXT PRIMARY KEY, name TEXT NOT NULL);
                 CREATE TABLE emulator_platforms (
                   emulator_id TEXT NOT NULL, platform_id TEXT NOT NULL,
                   core_name TEXT NOT NULL DEFAULT '', recommended INTEGER NOT NULL DEFAULT 1,
                   wiki_rank INTEGER, wiki_verdict TEXT,
                   PRIMARY KEY (emulator_id, platform_id, core_name)
                 );
                 CREATE UNIQUE INDEX emulator_platforms_wiki_rank
                   ON emulator_platforms(platform_id, wiki_rank) WHERE wiki_rank IS NOT NULL;
                 INSERT INTO platforms VALUES ('p-n64', 'Nintendo 64');
                 INSERT INTO emulators VALUES ('e-ares', 'ares'), ('e-gopher', 'Gopher64');
                 INSERT INTO emulator_platforms VALUES ('e-ares', 'p-n64', '', 1, NULL, NULL);
                 INSERT INTO emulator_platforms VALUES ('e-gopher', 'p-n64', '', 1, NULL, NULL);",
            )
            .unwrap();
        connection
    }

    fn snapshot_file(directory: &tempfile::TempDir, body: &str) -> std::path::PathBuf {
        let path = directory.path().join("wiki.json");
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn wiki_ranks_apply_to_exact_targets_only() {
        let directory = tempfile::tempdir().unwrap();
        let mut connection = test_database();
        let path = snapshot_file(
            &directory,
            r#"{"schema_version": 1, "retrieved_at": "2026-09-17", "license": "CC BY-SA",
                "entries": [
                  {"platform": "Nintendo 64", "emulator": "Gopher64", "core": "", "rank": 1,
                   "verdict": "recommended", "note": "Top pick.", "page_url": "https://emulation.gametechwiki.com/index.php/Nintendo_64_emulators"},
                  {"platform": "Nintendo 64", "emulator": "ares", "core": "", "rank": 2,
                   "verdict": "recommended", "note": "Accurate.", "page_url": "https://emulation.gametechwiki.com/index.php/Nintendo_64_emulators"}
                ]}"#,
        );
        let stats = import(&mut connection, &path, "2026-09-17T00:00:00Z").unwrap();
        assert_eq!((stats.entries, stats.updated), (2, 2));
        let rank: i64 = connection
            .query_row(
                "SELECT wiki_rank FROM emulator_platforms WHERE emulator_id = 'e-gopher'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rank, 1);
    }

    #[test]
    fn wiki_import_fails_closed_on_unknown_names_and_duplicate_ranks() {
        let directory = tempfile::tempdir().unwrap();
        let mut connection = test_database();
        let unknown = snapshot_file(
            &directory,
            r#"{"schema_version": 1, "retrieved_at": "2026-09-17", "license": "CC BY-SA",
                "entries": [
                  {"platform": "Nintendo 64", "emulator": "No Such Emulator", "core": "", "rank": 1,
                   "verdict": "recommended", "note": "", "page_url": "https://emulation.gametechwiki.com/index.php/Nintendo_64_emulators"}
                ]}"#,
        );
        assert!(import(&mut connection, &unknown, "2026-09-17T00:00:00Z").is_err());
        let duplicate = snapshot_file(
            &directory,
            r#"{"schema_version": 1, "retrieved_at": "2026-09-17", "license": "CC BY-SA",
                "entries": [
                  {"platform": "Nintendo 64", "emulator": "Gopher64", "core": "", "rank": 1,
                   "verdict": "recommended", "note": "", "page_url": "https://emulation.gametechwiki.com/index.php/Nintendo_64_emulators"},
                  {"platform": "Nintendo 64", "emulator": "ares", "core": "", "rank": 1,
                   "verdict": "recommended", "note": "", "page_url": "https://emulation.gametechwiki.com/index.php/Nintendo_64_emulators"}
                ]}"#,
        );
        assert!(import(&mut connection, &duplicate, "2026-09-17T00:00:00Z").is_err());
    }
}
