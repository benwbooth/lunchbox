use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;

use crate::audit;
use crate::emulators::{self, EmulatorImportStats};
use crate::ids::{sha256_file, stable_id};

const SCHEMA: &str = include_str!("../../../schema/001_initial.sql");

#[derive(Debug, Serialize)]
pub struct BuildResult {
    pub database: PathBuf,
    pub database_bytes: u64,
    pub database_sha256: String,
    pub emulator_import: EmulatorImportStats,
    pub audit: audit::AuditReport,
}

pub fn configure(connection: &Connection) -> Result<()> {
    connection.busy_timeout(std::time::Duration::from_secs(30))?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;\nPRAGMA trusted_schema = OFF;\nPRAGMA temp_store = MEMORY;",
    )?;
    Ok(())
}

pub fn open_existing(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("opening database {}", path.display()))?;
    configure(&connection)?;
    Ok(connection)
}

pub fn open_read_only(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("opening database read-only: {}", path.display()))?;
    configure(&connection)?;
    Ok(connection)
}

pub fn initialize_path(path: &Path) -> Result<()> {
    if path.exists() {
        bail!(
            "refusing to initialize over existing database: {}",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let connection = Connection::open(path)?;
    configure(&connection)?;
    initialize_connection(&connection, "1970-01-01T00:00:00Z")?;
    seed_providers(&connection)?;
    Ok(())
}

pub fn initialize_connection(connection: &Connection, timestamp: &str) -> Result<()> {
    connection.execute_batch(SCHEMA)?;
    connection.execute(
        "UPDATE schema_migrations SET applied_at = ?1 WHERE version = 1",
        [timestamp],
    )?;
    Ok(())
}

pub fn seed_providers(connection: &Connection) -> Result<()> {
    let providers = [
        (
            "lunchbox",
            "Lunchbox",
            "https://github.com/benwbooth/lunchbox",
            "MIT",
            "internal",
        ),
        (
            "lunchbox-emulator-catalog",
            "Lunchbox Emulator Catalog",
            "https://github.com/benwbooth/lunchbox",
            "MIT",
            "allowed",
        ),
        (
            "libretro-database",
            "Libretro Database",
            "https://github.com/libretro/libretro-database",
            "CC BY-SA 4.0",
            "allowed",
        ),
        (
            "openvgdb",
            "OpenVGDB",
            "https://github.com/OpenVGDB/OpenVGDB",
            "Unverified",
            "review_required",
        ),
        (
            "launchbox-games-db",
            "LaunchBox Games Database",
            "https://gamesdb.launchbox-app.com/",
            "No redistribution permission recorded",
            "review_required",
        ),
        (
            "igdb",
            "IGDB",
            "https://www.igdb.com/",
            "API terms",
            "runtime_only",
        ),
        (
            "minerva",
            "Minerva",
            "https://github.com/minerva-project/",
            "Provider-specific",
            "runtime_only",
        ),
    ];

    for (slug, name, homepage, license, policy) in providers {
        connection.execute(
            "INSERT INTO providers (id, slug, name, homepage, data_license, redistribution_policy)\n             VALUES (?1, ?2, ?3, ?4, ?5, ?6)\n             ON CONFLICT(slug) DO UPDATE SET\n               name = excluded.name, homepage = excluded.homepage,\n               data_license = excluded.data_license,\n               redistribution_policy = excluded.redistribution_policy",
            params![stable_id("provider", slug), slug, name, homepage, license, policy],
        )?;
    }
    Ok(())
}

pub fn build(database: &Path, emulator_source: &Path, timestamp: &str) -> Result<BuildResult> {
    let parent = database.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let temporary = temporary_path(database, "building");
    let previous = temporary_path(database, "previous");
    remove_exact_file_if_present(&temporary)?;
    remove_exact_file_if_present(&previous)?;

    let mut connection = Connection::open(&temporary)
        .with_context(|| format!("creating temporary database {}", temporary.display()))?;
    configure(&connection)?;
    initialize_connection(&connection, timestamp)?;
    seed_providers(&connection)?;

    let emulator_import = emulators::import(&mut connection, emulator_source, timestamp)?;
    let emulator_source_hash = sha256_file(emulator_source)?;

    let metadata = BTreeMap::from([
        ("build_timestamp", timestamp.to_owned()),
        ("emulator_source_sha256", emulator_source_hash),
        ("schema_version", "1".to_owned()),
    ]);
    for (key, value) in metadata {
        connection.execute(
            "INSERT INTO build_metadata (key, value) VALUES (?1, ?2)\n             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
    }

    connection.execute_batch("PRAGMA optimize; VACUUM;")?;
    let audit = audit::audit_connection(&connection)?;
    if !audit.valid {
        bail!("new database failed strict audit before publication");
    }
    drop(connection);

    if database.exists() {
        fs::rename(database, &previous).with_context(|| {
            format!(
                "moving previous database {} to {}",
                database.display(),
                previous.display()
            )
        })?;
    }

    if let Err(error) = fs::rename(&temporary, database) {
        if previous.exists() {
            let _ = fs::rename(&previous, database);
        }
        return Err(error).with_context(|| {
            format!(
                "installing completed database {} at {}",
                temporary.display(),
                database.display()
            )
        });
    }
    remove_exact_file_if_present(&previous)?;

    let final_audit = audit::audit_path(database)?;
    if !final_audit.valid {
        bail!("installed database failed strict audit");
    }

    Ok(BuildResult {
        database: database.to_path_buf(),
        database_bytes: fs::metadata(database)?.len(),
        database_sha256: sha256_file(database)?,
        emulator_import,
        audit: final_audit,
    })
}

fn temporary_path(database: &Path, suffix: &str) -> PathBuf {
    let file_name = database
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("lunchbox.db");
    database.with_file_name(format!(".{file_name}.{suffix}"))
}

fn remove_exact_file_if_present(path: &Path) -> Result<()> {
    if path.exists() {
        if !path.is_file() {
            bail!("refusing to remove non-file path: {}", path.display());
        }
        fs::remove_file(path)?;
    }
    Ok(())
}
