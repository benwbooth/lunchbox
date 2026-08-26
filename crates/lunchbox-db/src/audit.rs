use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::database;

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
        "platforms",
        "games",
        "releases",
        "artifacts",
        "source_records",
        "source_links",
        "acquisition_offers",
        "emulators",
        "emulator_platforms",
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
            "open_error_quality_issues",
            open_errors == 0,
            open_errors.to_string(),
        ),
        info_check("unresolved_source_records", unresolved_records.to_string()),
        info_check("pending_identity_links", pending_links.to_string()),
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
