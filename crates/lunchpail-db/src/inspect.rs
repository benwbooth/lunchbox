use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::database;
use crate::ids::sha256_file;

#[derive(Debug, Serialize)]
pub struct ExistingDatabaseReport {
    pub legacy_catalog: LegacyCatalogAudit,
    pub openvgdb: OpenVgdbAudit,
    pub emulators: EmulatorDatabaseAudit,
    pub minerva: MinervaDatabaseAudit,
}

#[derive(Debug, Serialize)]
pub struct FileIdentity {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct LegacyCatalogAudit {
    pub file: FileIdentity,
    pub games: i64,
    pub platforms: i64,
    pub alternate_names: i64,
    pub rows_by_source: BTreeMap<String, i64>,
    pub launchbox_and_libretro_hash_rows: i64,
    pub exact_title_platform_duplicate_groups: i64,
    pub exact_title_platform_duplicate_rows: i64,
    pub exact_cross_source_duplicate_groups: i64,
    pub shared_openvgdb_id_groups: i64,
    pub shared_openvgdb_id_rows: i64,
    pub openvgdb_ids_spanning_platforms: i64,
    pub generated_ids_rebuild_stable: bool,
}

#[derive(Debug, Serialize)]
pub struct OpenVgdbAudit {
    pub file: FileIdentity,
    pub systems: i64,
    pub releases: i64,
    pub roms: i64,
    pub regions: i64,
    pub orphan_release_roms: i64,
    pub duplicate_crc_groups: i64,
    pub duplicate_sha1_groups: i64,
}

#[derive(Debug, Serialize)]
pub struct EmulatorDatabaseAudit {
    pub file: FileIdentity,
    pub emulators: i64,
    pub platform_links: i64,
    pub distinct_platforms: i64,
    pub orphan_platform_links: i64,
    pub case_insensitive_duplicate_name_groups: i64,
}

#[derive(Debug, Serialize)]
pub struct MinervaDatabaseAudit {
    pub file: FileIdentity,
    pub torrents: i64,
    pub torrent_platform_rows: i64,
    pub distinct_raw_platforms: i64,
    pub unmapped_platform_rows: i64,
    pub orphan_platform_links: i64,
}

pub fn inspect_existing(
    legacy_catalog: &Path,
    openvgdb: &Path,
    emulators: &Path,
    minerva: &Path,
) -> Result<ExistingDatabaseReport> {
    Ok(ExistingDatabaseReport {
        legacy_catalog: inspect_legacy_catalog(legacy_catalog)?,
        openvgdb: inspect_openvgdb(openvgdb)?,
        emulators: inspect_emulators(emulators)?,
        minerva: inspect_minerva(minerva)?,
    })
}

pub fn write_report(path: &Path, report: &ExistingDatabaseReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_file_name(format!(
        ".{}.building",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("existing-databases.json")
    ));
    fs::write(&temporary, serde_json::to_vec_pretty(report)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn inspect_legacy_catalog(path: &Path) -> Result<LegacyCatalogAudit> {
    let connection = database::open_read_only(path)?;
    let duplicate_groups = pair_query(
        &connection,
        "SELECT count(*), coalesce(sum(n),0) FROM (\n           SELECT platform_id, lower(trim(title)), count(*) n FROM games\n           GROUP BY 1,2 HAVING n > 1\n         )",
    )?;
    let shared_openvgdb = pair_query(
        &connection,
        "SELECT count(*), coalesce(sum(n),0) FROM (\n           SELECT openvgdb_release_id, count(*) n FROM games\n           WHERE openvgdb_release_id IS NOT NULL GROUP BY 1\n           HAVING count(DISTINCT metadata_source) > 1\n         )",
    )?;

    let mut rows_by_source = BTreeMap::new();
    let mut statement = connection.prepare(
        "SELECT coalesce(metadata_source, '(null)'), count(*) FROM games GROUP BY 1 ORDER BY 1",
    )?;
    let rows = statement.query_map([], |record| {
        Ok((record.get::<_, String>(0)?, record.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (source, count) = row?;
        rows_by_source.insert(source, count);
    }

    Ok(LegacyCatalogAudit {
        file: identify(path)?,
        games: scalar(&connection, "SELECT count(*) FROM games")?,
        platforms: scalar(&connection, "SELECT count(*) FROM platforms")?,
        alternate_names: scalar(&connection, "SELECT count(*) FROM game_alternate_names")?,
        rows_by_source,
        launchbox_and_libretro_hash_rows: scalar(
            &connection,
            "SELECT count(*) FROM games WHERE launchbox_db_id IS NOT NULL AND libretro_crc32 IS NOT NULL",
        )?,
        exact_title_platform_duplicate_groups: duplicate_groups.0,
        exact_title_platform_duplicate_rows: duplicate_groups.1,
        exact_cross_source_duplicate_groups: scalar(
            &connection,
            "SELECT count(*) FROM (\n               SELECT platform_id, lower(trim(title)) FROM games GROUP BY 1,2\n               HAVING count(*) > 1 AND count(DISTINCT metadata_source) > 1\n             )",
        )?,
        shared_openvgdb_id_groups: shared_openvgdb.0,
        shared_openvgdb_id_rows: shared_openvgdb.1,
        openvgdb_ids_spanning_platforms: scalar(
            &connection,
            "SELECT count(*) FROM (\n               SELECT openvgdb_release_id FROM games WHERE openvgdb_release_id IS NOT NULL\n               GROUP BY 1 HAVING count(DISTINCT platform_id) > 1\n             )",
        )?,
        generated_ids_rebuild_stable: false,
    })
}

fn inspect_openvgdb(path: &Path) -> Result<OpenVgdbAudit> {
    let connection = database::open_read_only(path)?;
    Ok(OpenVgdbAudit {
        file: identify(path)?,
        systems: scalar(&connection, "SELECT count(*) FROM SYSTEMS")?,
        releases: scalar(&connection, "SELECT count(*) FROM RELEASES")?,
        roms: scalar(&connection, "SELECT count(*) FROM ROMs")?,
        regions: scalar(&connection, "SELECT count(*) FROM REGIONS")?,
        orphan_release_roms: scalar(
            &connection,
            "SELECT count(*) FROM RELEASES r LEFT JOIN ROMs x ON x.romID=r.romID\n             WHERE r.romID IS NOT NULL AND x.romID IS NULL",
        )?,
        duplicate_crc_groups: scalar(
            &connection,
            "SELECT count(*) FROM (SELECT upper(romHashCRC) FROM ROMs\n             WHERE romHashCRC IS NOT NULL AND trim(romHashCRC)<>'' GROUP BY 1 HAVING count(*)>1)",
        )?,
        duplicate_sha1_groups: scalar(
            &connection,
            "SELECT count(*) FROM (SELECT upper(romHashSHA1) FROM ROMs\n             WHERE romHashSHA1 IS NOT NULL AND trim(romHashSHA1)<>'' GROUP BY 1 HAVING count(*)>1)",
        )?,
    })
}

fn inspect_emulators(path: &Path) -> Result<EmulatorDatabaseAudit> {
    let connection = database::open_read_only(path)?;
    Ok(EmulatorDatabaseAudit {
        file: identify(path)?,
        emulators: scalar(&connection, "SELECT count(*) FROM emulators")?,
        platform_links: scalar(&connection, "SELECT count(*) FROM platform_emulators")?,
        distinct_platforms: scalar(
            &connection,
            "SELECT count(DISTINCT platform_name) FROM platform_emulators",
        )?,
        orphan_platform_links: scalar(
            &connection,
            "SELECT count(*) FROM platform_emulators p LEFT JOIN emulators e ON e.id=p.emulator_id\n             WHERE e.id IS NULL",
        )?,
        case_insensitive_duplicate_name_groups: scalar(
            &connection,
            "SELECT count(*) FROM (SELECT lower(trim(name)) FROM emulators GROUP BY 1 HAVING count(*)>1)",
        )?,
    })
}

fn inspect_minerva(path: &Path) -> Result<MinervaDatabaseAudit> {
    let connection = database::open_read_only(path)?;
    Ok(MinervaDatabaseAudit {
        file: identify(path)?,
        torrents: scalar(&connection, "SELECT count(*) FROM minerva_torrents")?,
        torrent_platform_rows: scalar(
            &connection,
            "SELECT count(*) FROM minerva_torrent_platforms",
        )?,
        distinct_raw_platforms: scalar(
            &connection,
            "SELECT count(DISTINCT minerva_platform) FROM minerva_torrent_platforms",
        )?,
        unmapped_platform_rows: scalar(
            &connection,
            "SELECT count(*) FROM minerva_torrent_platforms WHERE lunchbox_platform_id IS NULL",
        )?,
        orphan_platform_links: scalar(
            &connection,
            "SELECT count(*) FROM minerva_torrent_platforms p\n             LEFT JOIN minerva_torrents t ON t.id=p.torrent_id WHERE t.id IS NULL",
        )?,
    })
}

fn identify(path: &Path) -> Result<FileIdentity> {
    Ok(FileIdentity {
        path: path.to_path_buf(),
        bytes: fs::metadata(path)
            .with_context(|| format!("reading metadata for {}", path.display()))?
            .len(),
        sha256: sha256_file(path)?,
    })
}

fn scalar(connection: &Connection, sql: &str) -> Result<i64> {
    Ok(connection.query_row(sql, [], |record| record.get(0))?)
}

fn pair_query(connection: &Connection, sql: &str) -> Result<(i64, i64)> {
    Ok(connection.query_row(sql, [], |record| Ok((record.get(0)?, record.get(1)?)))?)
}
