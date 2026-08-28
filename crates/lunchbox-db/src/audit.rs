use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::database;
use crate::ids::sha256_bytes;

#[derive(Debug, Clone, Serialize)]
pub struct AuditCheck {
    pub name: String,
    pub severity: String,
    pub passed: bool,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    pub valid: bool,
    pub counts: BTreeMap<String, i64>,
    pub checks: Vec<AuditCheck>,
}

pub fn audit_path(path: &Path) -> Result<AuditReport> {
    let connection = database::open_read_only(path)?;
    audit_connection(&connection)
}

pub fn audit_connection(connection: &Connection) -> Result<AuditReport> {
    let mut counts = BTreeMap::new();
    for table in [
        "providers",
        "source_snapshots",
        "libretro_databases",
        "libretro_records",
        "libretro_hashes",
        "libretro_serials",
        "platforms",
        "games",
        "releases",
        "artifacts",
        "release_artifacts",
        "artifact_hashes",
        "release_identifiers",
        "source_records",
        "source_links",
        "entity_assertions",
        "acquisition_offers",
        "collection_roots",
        "local_files",
        "emulators",
        "emulator_platforms",
        "emulator_packages",
        "data_quality_issues",
    ] {
        counts.insert(table.to_owned(), table_count(connection, table)?);
    }

    let integrity: String =
        connection.query_row("PRAGMA integrity_check", [], |record| record.get(0))?;
    let foreign_key_failures = pragma_row_count(connection, "PRAGMA foreign_key_check")?;
    let missing_link_targets: i64 = connection.query_row(
        "SELECT\n           (SELECT count(*) FROM source_links s WHERE entity_type='platform' AND NOT EXISTS (SELECT 1 FROM platforms e WHERE e.id=s.entity_id)) +\n           (SELECT count(*) FROM source_links s WHERE entity_type='game' AND NOT EXISTS (SELECT 1 FROM games e WHERE e.id=s.entity_id)) +\n           (SELECT count(*) FROM source_links s WHERE entity_type='release' AND NOT EXISTS (SELECT 1 FROM releases e WHERE e.id=s.entity_id)) +\n           (SELECT count(*) FROM source_links s WHERE entity_type='artifact' AND NOT EXISTS (SELECT 1 FROM artifacts e WHERE e.id=s.entity_id)) +\n           (SELECT count(*) FROM source_links s WHERE entity_type='emulator' AND NOT EXISTS (SELECT 1 FROM emulators e WHERE e.id=s.entity_id)) +\n           (SELECT count(*) FROM source_links s WHERE entity_type='offer' AND NOT EXISTS (SELECT 1 FROM acquisition_offers e WHERE e.id=s.entity_id))",
        [],
        |record| record.get(0),
    )?;
    let duplicate_releases: i64 = connection.query_row(
        "SELECT count(*) FROM (\n           SELECT game_id, platform_id, coalesce(region,''), coalesce(languages,''),\n                  coalesce(version,''), coalesce(edition,''), coalesce(release_date,''), count(*) n\n           FROM releases GROUP BY 1,2,3,4,5,6,7 HAVING n > 1\n         )",
        [],
        |record| record.get(0),
    )?;
    let duplicate_selected_assertions: i64 = connection.query_row(
        "SELECT count(*) FROM (\n           SELECT entity_type, entity_id, field_name, count(*) n\n           FROM entity_assertions WHERE selected=1 GROUP BY 1,2,3 HAVING n > 1\n         )",
        [],
        |record| record.get(0),
    )?;
    let open_errors: i64 = connection.query_row(
        "SELECT count(*) FROM data_quality_issues WHERE severity='error' AND status='open'",
        [],
        |record| record.get(0),
    )?;
    let unresolved_records: i64 = connection.query_row(
        "SELECT count(*) FROM source_records r\n         WHERE NOT EXISTS (SELECT 1 FROM source_links l WHERE l.source_record_id=r.id AND l.status='accepted')",
        [],
        |record| record.get(0),
    )?;
    let pending_links: i64 = connection.query_row(
        "SELECT count(*) FROM source_links WHERE status='pending'",
        [],
        |record| record.get(0),
    )?;
    let invalid_hashes: i64 = connection.query_row(
        "SELECT count(*) FROM artifact_hashes WHERE\n           digest GLOB '*[^0-9a-f]*' OR\n           (algorithm='crc32' AND length(digest)<>8) OR\n           (algorithm='md5' AND length(digest)<>32) OR\n           (algorithm='sha1' AND length(digest)<>40) OR\n           (algorithm='sha256' AND length(digest)<>64)",
        [],
        |record| record.get(0),
    )?;
    let strong_hash_conflicts: i64 = connection.query_row(
        "SELECT count(*) FROM (\n           SELECT algorithm, digest FROM artifact_hashes\n           WHERE algorithm IN ('md5','sha1','sha256')\n           GROUP BY algorithm, digest HAVING count(DISTINCT artifact_id)>1\n         )",
        [],
        |record| record.get(0),
    )?;
    let crc_collisions: i64 = connection.query_row(
        "SELECT count(*) FROM (\n           SELECT digest FROM artifact_hashes WHERE algorithm='crc32'\n           GROUP BY digest HAVING count(DISTINCT artifact_id)>1\n         )",
        [],
        |record| record.get(0),
    )?;
    let invalid_libretro_snapshots: i64 = connection.query_row(
        "SELECT count(*) FROM libretro_databases d
         LEFT JOIN source_snapshots s ON s.id=d.source_snapshot_id
         LEFT JOIN providers p ON p.id=s.provider_id
         WHERE p.slug IS NULL OR p.slug<>'libretro-database'",
        [],
        |record| record.get(0),
    )?;
    let libretro_record_count_mismatches: i64 = connection.query_row(
        "SELECT count(*) FROM libretro_databases d
         WHERE d.record_count<>(SELECT count(*) FROM libretro_records r WHERE r.database_id=d.id)",
        [],
        |record| record.get(0),
    )?;
    let invalid_libretro_hashes: i64 = connection.query_row(
        "SELECT count(*) FROM libretro_hashes WHERE
           typeof(digest)<>'blob' OR
           (algorithm='crc32' AND length(digest)<>4) OR
           (algorithm='md5' AND length(digest)<>16) OR
           (algorithm='sha1' AND length(digest)<>20)",
        [],
        |record| record.get(0),
    )?;
    let libretro_records_without_title: i64 = connection.query_row(
        "SELECT count(*) FROM libretro_records WHERE title IS NULL OR trim(title)=''",
        [],
        |record| record.get(0),
    )?;
    let libretro_records_without_identity: i64 = connection.query_row(
        "SELECT count(*) FROM libretro_records r
         WHERE r.id NOT IN (
           SELECT record_id FROM libretro_hashes
           UNION
           SELECT record_id FROM libretro_serials
         )",
        [],
        |record| record.get(0),
    )?;
    let libretro_crc_disagreements: i64 = connection.query_row(
        "WITH per_record AS (
           SELECT record_id,
                  max(CASE WHEN algorithm='crc32' THEN hex(digest) END) crc,
                  max(CASE WHEN algorithm='md5' THEN hex(digest) END) md5,
                  max(CASE WHEN algorithm='sha1' THEN hex(digest) END) sha1
           FROM libretro_hashes GROUP BY record_id
         )
         SELECT count(*) FROM (
           SELECT coalesce('sha1:' || h.sha1, 'md5:' || h.md5) identity_key, r.byte_size
           FROM per_record h JOIN libretro_records r ON r.id=h.record_id
           WHERE h.crc IS NOT NULL AND (h.sha1 IS NOT NULL OR h.md5 IS NOT NULL)
           GROUP BY identity_key, r.byte_size HAVING count(DISTINCT h.crc)>1
         )",
        [],
        |record| record.get(0),
    )?;
    let snapshot_provider_mismatches: i64 = connection.query_row(
        "SELECT count(*) FROM source_records r JOIN source_snapshots s ON s.id=r.source_snapshot_id
         WHERE r.provider_id<>s.provider_id",
        [],
        |record| record.get(0),
    )?;
    let invalid_materialized_payloads = invalid_materialized_payloads(connection)?;
    let duplicate_platform_titles: i64 = connection.query_row(
        "SELECT count(*) FROM (
           SELECT d.platform_id, lower(trim(r.title)), count(*) n
           FROM libretro_records r JOIN libretro_databases d ON d.id=r.database_id
           WHERE r.title IS NOT NULL GROUP BY 1,2 HAVING n>1
         )",
        [],
        |record| record.get(0),
    )?;
    let invalid_local_file_sources: i64 = connection.query_row(
        "SELECT count(*) FROM local_files f
         LEFT JOIN source_records r ON r.id=f.source_record_id
         LEFT JOIN providers p ON p.id=r.provider_id
         WHERE p.slug IS NULL OR p.slug<>'lunchbox-local' OR r.record_type<>'artifact'
            OR NOT EXISTS (
              SELECT 1 FROM source_links l
              WHERE l.source_record_id=f.source_record_id
                AND l.entity_type='artifact' AND l.entity_id=f.artifact_id
                AND l.status='accepted' AND l.method='exact_hash'
            )",
        [],
        |record| record.get(0),
    )?;
    let offers_without_native_source: i64 = connection.query_row(
        "SELECT count(*) FROM acquisition_offers o
         WHERE NOT EXISTS (
           SELECT 1 FROM source_links l
           JOIN source_records r ON r.id=l.source_record_id
           WHERE l.entity_type='offer' AND l.entity_id=o.id
             AND l.status='accepted' AND r.provider_id=o.provider_id
         )",
        [],
        |record| record.get(0),
    )?;
    let missing_local_files: i64 = connection.query_row(
        "SELECT count(*) FROM local_files WHERE availability='missing'",
        [],
        |record| record.get(0),
    )?;
    let invalid_emulator_packages: i64 = connection.query_row(
        "SELECT count(*)
         FROM emulator_packages p
         LEFT JOIN emulator_host_systems h
           ON h.emulator_id=p.emulator_id AND h.host_system_slug=p.host_system_slug
         WHERE h.emulator_id IS NULL
            OR trim(p.package_id)=''
            OR instr(p.package_id, char(10))>0 OR instr(p.package_id, char(13))>0
            OR instr(p.package_id, char(9))>0
            OR (p.manager='winget' AND p.host_system_slug<>'windows')
            OR (p.manager='homebrew' AND p.host_system_slug<>'macos')
            OR (p.manager IN ('flatpak','appimage','nix','github','direct')
                AND p.host_system_slug<>'linux')
            OR (p.manager='flatpak' AND (instr(p.package_id,'.')=0 OR p.package_id LIKE '-%'))
            OR (p.manager='appimage' AND (
                instr(p.package_id,'/')=0
                OR coalesce(json_type(p.metadata_json,'$.asset_terms'),'')<>'array'
                OR coalesce(json_type(p.metadata_json,'$.update_strategy'),'')<>'text'
            ))
            OR (p.manager='direct' AND p.package_id NOT LIKE 'https://%')",
        [],
        |record| record.get(0),
    )?;

    let checks = vec![
        error_check("sqlite_integrity", integrity == "ok", integrity),
        error_check(
            "foreign_keys",
            foreign_key_failures == 0,
            foreign_key_failures.to_string(),
        ),
        error_check(
            "source_link_targets",
            missing_link_targets == 0,
            missing_link_targets.to_string(),
        ),
        error_check(
            "duplicate_release_fingerprints",
            duplicate_releases == 0,
            duplicate_releases.to_string(),
        ),
        error_check(
            "selected_field_uniqueness",
            duplicate_selected_assertions == 0,
            duplicate_selected_assertions.to_string(),
        ),
        error_check(
            "artifact_hash_format",
            invalid_hashes == 0,
            invalid_hashes.to_string(),
        ),
        error_check(
            "strong_hash_identity_conflicts",
            strong_hash_conflicts == 0,
            strong_hash_conflicts.to_string(),
        ),
        error_check(
            "libretro_source_snapshot_provenance",
            invalid_libretro_snapshots == 0,
            invalid_libretro_snapshots.to_string(),
        ),
        error_check(
            "libretro_declared_record_counts",
            libretro_record_count_mismatches == 0,
            libretro_record_count_mismatches.to_string(),
        ),
        error_check(
            "libretro_hash_format",
            invalid_libretro_hashes == 0,
            invalid_libretro_hashes.to_string(),
        ),
        error_check(
            "source_snapshot_provider_consistency",
            snapshot_provider_mismatches == 0,
            snapshot_provider_mismatches.to_string(),
        ),
        error_check(
            "materialized_source_payload_sha256",
            invalid_materialized_payloads == 0,
            invalid_materialized_payloads.to_string(),
        ),
        error_check(
            "local_file_source_provenance",
            invalid_local_file_sources == 0,
            invalid_local_file_sources.to_string(),
        ),
        error_check(
            "acquisition_offer_source_provenance",
            offers_without_native_source == 0,
            offers_without_native_source.to_string(),
        ),
        error_check(
            "host_scoped_emulator_install_sources",
            invalid_emulator_packages == 0,
            invalid_emulator_packages.to_string(),
        ),
        error_check(
            "open_error_quality_issues",
            open_errors == 0,
            open_errors.to_string(),
        ),
        info_check("unresolved_source_records", unresolved_records.to_string()),
        info_check("pending_identity_links", pending_links.to_string()),
        info_check(
            "crc32_values_shared_by_artifacts",
            crc_collisions.to_string(),
        ),
        info_check(
            "duplicate_platform_release_titles",
            duplicate_platform_titles.to_string(),
        ),
        info_check(
            "libretro_records_without_title",
            libretro_records_without_title.to_string(),
        ),
        info_check(
            "libretro_records_without_hash_or_serial",
            libretro_records_without_identity.to_string(),
        ),
        info_check(
            "libretro_strong_identity_crc_disagreements",
            libretro_crc_disagreements.to_string(),
        ),
        info_check("missing_local_files", missing_local_files.to_string()),
    ];
    let valid = checks
        .iter()
        .filter(|check| check.severity == "error")
        .all(|check| check.passed);

    Ok(AuditReport {
        valid,
        counts,
        checks,
    })
}

fn invalid_materialized_payloads(connection: &Connection) -> Result<i64> {
    let mut statement = connection.prepare(
        "SELECT payload_json, payload_sha256 FROM source_records
         WHERE payload_json IS NOT NULL OR payload_sha256 IS NOT NULL",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
        ))
    })?;
    let mut invalid = 0_i64;
    for row in rows {
        let (payload, expected_hash) = row?;
        match (payload, expected_hash) {
            (Some(payload), Some(expected_hash))
                if sha256_bytes(payload.as_bytes()) == expected_hash => {}
            _ => invalid += 1,
        }
    }
    Ok(invalid)
}

fn table_count(connection: &Connection, table: &str) -> Result<i64> {
    Ok(
        connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |record| {
            record.get(0)
        })?,
    )
}

fn pragma_row_count(connection: &Connection, pragma: &str) -> Result<i64> {
    let mut statement = connection.prepare(pragma)?;
    let mut rows = statement.query([])?;
    let mut count = 0_i64;
    while rows.next()?.is_some() {
        count += 1;
    }
    Ok(count)
}

fn error_check(name: &str, passed: bool, value: String) -> AuditCheck {
    AuditCheck {
        name: name.to_owned(),
        severity: "error".to_owned(),
        passed,
        value,
    }
}

fn info_check(name: &str, value: String) -> AuditCheck {
    AuditCheck {
        name: name.to_owned(),
        severity: "info".to_owned(),
        passed: true,
        value,
    }
}
