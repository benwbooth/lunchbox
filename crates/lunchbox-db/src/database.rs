use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};

use crate::audit;
use crate::emulators::{self, EmulatorImportStats};
use crate::ids::{sha256_bytes, sha256_file, stable_id};
use crate::libretro::{self, LibretroImportStats};
use crate::source;

const SCHEMA: &str = include_str!("../../../schema/001_initial.sql");
const MIGRATION_3: &str = include_str!("../../../schema/003_local_collection.sql");
const MIGRATION_4: &str = include_str!("../../../schema/004_emulator_install_sources.sql");
const MIGRATION_5: &str = include_str!("../../../schema/005_firmware_rules.sql");
const PROVIDER_REGISTRY_JSON: &str = include_str!("../../../sources/metadata-providers.json");

#[derive(Debug, Deserialize)]
struct ProviderRegistry {
    schema_version: u32,
    reviewed_at: String,
    providers: Vec<ProviderDefinition>,
}

#[derive(Debug, Deserialize)]
struct ProviderDefinition {
    slug: String,
    name: String,
    homepage: String,
    terms_url: String,
    data_license: String,
    redistribution_policy: String,
    role: String,
    decision: String,
}

#[derive(Debug, Serialize)]
pub struct BuildResult {
    pub database: PathBuf,
    pub database_bytes: u64,
    pub database_sha256: String,
    pub emulator_import: EmulatorImportStats,
    pub firmware_import: crate::firmware::FirmwareImportStats,
    pub libretro_import: LibretroImportStats,
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
    migrate_connection(&connection)?;
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
    connection.execute("UPDATE schema_migrations SET applied_at = ?1", [timestamp])?;
    Ok(())
}

fn migrate_connection(connection: &Connection) -> Result<()> {
    let current_version: i64 = connection
        .query_row(
            "SELECT coalesce(max(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .context("reading Lunchbox schema version")?;
    match current_version {
        2 => {
            connection
                .execute_batch(MIGRATION_3)
                .context("applying schema migration 3")?;
            connection
                .execute_batch(MIGRATION_4)
                .context("applying schema migration 4")?;
            connection
                .execute_batch(MIGRATION_5)
                .context("applying schema migration 5")?;
        }
        3 => {
            connection
                .execute_batch(MIGRATION_4)
                .context("applying schema migration 4")?;
            connection
                .execute_batch(MIGRATION_5)
                .context("applying schema migration 5")?;
        }
        4 => connection
            .execute_batch(MIGRATION_5)
            .context("applying schema migration 5")?,
        5 => {}
        0..=1 => bail!(
            "database schema version {current_version} is too old for an in-place migration; rebuild it from declared inputs"
        ),
        _ => bail!(
            "database schema version {current_version} is newer than this lunchbox-db build supports"
        ),
    }
    Ok(())
}

pub fn seed_providers(connection: &Connection) -> Result<()> {
    let registry = load_provider_registry()?;

    for provider in registry.providers {
        connection.execute(
            "INSERT INTO providers (id, slug, name, homepage, data_license, redistribution_policy)\n             VALUES (?1, ?2, ?3, ?4, ?5, ?6)\n             ON CONFLICT(slug) DO UPDATE SET\n               name = excluded.name, homepage = excluded.homepage,\n               data_license = excluded.data_license,\n               redistribution_policy = excluded.redistribution_policy",
            params![
                stable_id("provider", &provider.slug),
                provider.slug,
                provider.name,
                provider.homepage,
                provider.data_license,
                provider.redistribution_policy
            ],
        )?;
    }
    Ok(())
}

fn load_provider_registry() -> Result<ProviderRegistry> {
    let registry: ProviderRegistry = serde_json::from_str(PROVIDER_REGISTRY_JSON)
        .context("parsing sources/metadata-providers.json")?;
    if registry.schema_version != 1 {
        bail!(
            "unsupported metadata provider registry schema version: {}",
            registry.schema_version
        );
    }
    if registry.reviewed_at.trim().is_empty() {
        bail!("metadata provider registry reviewed_at must not be empty");
    }

    let allowed_policies = ["allowed", "runtime_only", "review_required", "internal"];
    let mut slugs = BTreeSet::new();
    for provider in &registry.providers {
        if provider.slug.trim().is_empty()
            || provider.name.trim().is_empty()
            || provider.homepage.trim().is_empty()
            || provider.terms_url.trim().is_empty()
            || provider.data_license.trim().is_empty()
            || provider.role.trim().is_empty()
            || provider.decision.trim().is_empty()
        {
            bail!("metadata provider definitions must not contain empty fields");
        }
        if !allowed_policies.contains(&provider.redistribution_policy.as_str()) {
            bail!(
                "invalid redistribution policy for {}: {}",
                provider.slug,
                provider.redistribution_policy
            );
        }
        if !slugs.insert(provider.slug.as_str()) {
            bail!("duplicate metadata provider slug: {}", provider.slug);
        }
    }
    Ok(registry)
}

pub fn build(
    database: &Path,
    emulator_source: &Path,
    libretro_manifest_path: &Path,
    libretro_rdb_directory: &Path,
    timestamp: &str,
) -> Result<BuildResult> {
    let parent = database.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let temporary = temporary_path(database, "building");
    let previous = temporary_path(database, "previous");
    remove_exact_file_if_present(&temporary)?;
    remove_exact_file_if_present(&previous)?;

    let mut connection = Connection::open(&temporary)
        .with_context(|| format!("creating temporary database {}", temporary.display()))?;
    configure(&connection)?;
    // This connection only writes a disposable build candidate. Atomic publication and the
    // post-build integrity audit provide the durability boundary, so journaling the import
    // itself only wastes time and disk space.
    connection.execute_batch(
        "PRAGMA journal_mode = OFF;\nPRAGMA synchronous = OFF;\nPRAGMA locking_mode = EXCLUSIVE;\nPRAGMA cache_size = -1048576;",
    )?;
    initialize_connection(&connection, timestamp)?;
    seed_providers(&connection)?;

    let emulator_import = emulators::import(&mut connection, emulator_source, timestamp)?;
    let firmware_import = crate::firmware::import(&mut connection)?;
    let emulator_source_hash = sha256_file(emulator_source)?;
    let libretro_manifest = source::load_libretro_manifest(libretro_manifest_path)?;
    let libretro_import = libretro::import(
        &mut connection,
        libretro_manifest_path,
        libretro_rdb_directory,
        timestamp,
    )?;
    let libretro_manifest_hash = sha256_file(libretro_manifest_path)?;

    let metadata = BTreeMap::from([
        ("build_timestamp", timestamp.to_owned()),
        ("emulator_source_sha256", emulator_source_hash),
        ("libretro_revision", libretro_manifest.revision),
        ("libretro_source_manifest_sha256", libretro_manifest_hash),
        (
            "metadata_provider_registry_sha256",
            sha256_bytes(PROVIDER_REGISTRY_JSON.as_bytes()),
        ),
        (
            "metadata_provider_registry_reviewed_at",
            load_provider_registry()?.reviewed_at,
        ),
        (
            "emulator_install_sources_sha256",
            sha256_bytes(emulators::INSTALL_SOURCES_JSON.as_bytes()),
        ),
        (
            "firmware_rules_sha256",
            sha256_bytes(crate::firmware::RULES_JSON.as_bytes()),
        ),
        ("schema_version", "5".to_owned()),
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
        firmware_import,
        libretro_import,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_registry_is_valid_and_seeds_idempotently() -> Result<()> {
        let registry = load_provider_registry()?;
        let connection = Connection::open_in_memory()?;
        configure(&connection)?;
        initialize_connection(&connection, "1970-01-01T00:00:00Z")?;

        seed_providers(&connection)?;
        seed_providers(&connection)?;

        let count: i64 =
            connection.query_row("SELECT count(*) FROM providers", [], |row| row.get(0))?;
        assert_eq!(count as usize, registry.providers.len());
        assert_eq!(
            connection.query_row(
                "SELECT redistribution_policy FROM providers WHERE slug='wikidata'",
                [],
                |row| row.get::<_, String>(0),
            )?,
            "allowed"
        );
        assert_eq!(
            connection.query_row(
                "SELECT redistribution_policy FROM providers WHERE slug='igdb'",
                [],
                |row| row.get::<_, String>(0),
            )?,
            "runtime_only"
        );

        Ok(())
    }

    #[test]
    fn schema_two_migrates_through_firmware_rules_once() -> Result<()> {
        let connection = Connection::open_in_memory()?;
        configure(&connection)?;
        initialize_connection(&connection, "1970-01-01T00:00:00Z")?;
        connection.execute_batch(
            "DROP TABLE firmware_rules;
             DROP TABLE firmware_sources;
             DROP TABLE local_files;
             DROP TABLE collection_roots;
             DROP TABLE emulator_packages;
             CREATE TABLE emulator_packages (
                 emulator_id TEXT NOT NULL REFERENCES emulators(id) ON DELETE CASCADE,
                 manager TEXT NOT NULL CHECK (
                     manager IN ('winget', 'homebrew', 'flatpak', 'snap', 'nix', 'other')
                 ),
                 package_id TEXT NOT NULL,
                 PRIMARY KEY (emulator_id, manager, package_id)
             ) STRICT, WITHOUT ROWID;
             DELETE FROM schema_migrations WHERE version IN (3, 4, 5);",
        )?;

        migrate_connection(&connection)?;
        migrate_connection(&connection)?;

        assert_eq!(
            connection.query_row(
                "SELECT count(*) FROM schema_migrations WHERE version=3",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            1
        );
        assert_eq!(
            connection.query_row(
                "SELECT count(*) FROM schema_migrations WHERE version=5",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            1
        );
        assert_eq!(
            connection.query_row(
                "SELECT count(*) FROM schema_migrations WHERE version=4",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            1
        );
        assert_eq!(
            connection.query_row(
                "SELECT count(*) FROM sqlite_schema
                 WHERE type='table' AND name IN ('collection_roots','local_files')",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            2
        );
        assert_eq!(
            connection.query_row(
                "SELECT count(*) FROM sqlite_schema
                 WHERE type='table' AND name IN ('firmware_sources','firmware_rules')",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            2
        );
        assert_eq!(
            connection.query_row(
                "SELECT count(*) FROM pragma_table_info('emulator_packages')
                 WHERE name IN ('host_system_slug','metadata_json')",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            2
        );
        Ok(())
    }
}
