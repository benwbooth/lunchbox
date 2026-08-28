use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::ids::{normalize_key, sha256_bytes, stable_id};

const PROVIDER_SLUG: &str = "lunchbox-emulator-catalog";
pub const INSTALL_SOURCES_JSON: &str =
    include_str!("../../../sources/emulator-install-sources.json");

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct EmulatorRow {
    platform: String,
    emulator_name: String,
    #[serde(default)]
    supported_os: String,
    #[serde(default)]
    homepage: String,
    #[serde(default)]
    winget_id: String,
    #[serde(default)]
    homebrew_formula: String,
    #[serde(default)]
    flatpak_id: String,
    #[serde(default)]
    retroarch_core: String,
    #[serde(default)]
    save_directory: String,
    #[serde(default)]
    save_extensions: String,
    #[serde(default)]
    notes: String,
}

#[derive(Debug, Deserialize)]
struct InstallSourceCatalog {
    schema_version: u32,
    reviewed_at: String,
    sources: Vec<InstallSource>,
}

#[derive(Debug, Deserialize)]
struct InstallSource {
    emulator: String,
    host: String,
    manager: String,
    package_id: String,
    #[serde(default = "empty_json_object")]
    metadata: serde_json::Value,
}

fn empty_json_object() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct EmulatorImportStats {
    pub source_rows: usize,
    pub unavailable_emulator_rows: usize,
    pub emulators: i64,
    pub emulator_aliases: i64,
    pub platforms: i64,
    pub platform_links: i64,
    pub host_system_links: i64,
    pub packages: i64,
    pub warnings: i64,
}

pub fn import(
    connection: &mut Connection,
    source: &Path,
    import_timestamp: &str,
) -> Result<EmulatorImportStats> {
    let provider_id: String = connection
        .query_row(
            "SELECT id FROM providers WHERE slug = ?1",
            [PROVIDER_SLUG],
            |row| row.get(0),
        )
        .context("emulator provider is not registered; initialize the schema first")?;

    let mut reader = csv::Reader::from_path(source)
        .with_context(|| format!("opening emulator source {}", source.display()))?;
    let transaction = connection.transaction()?;
    let mut source_rows = 0_usize;
    let mut unavailable_emulator_rows = 0_usize;

    for result in reader.deserialize::<EmulatorRow>() {
        let row = result
            .with_context(|| format!("parsing {} row {}", source.display(), source_rows + 2))?;
        source_rows += 1;
        validate_row(&row).with_context(|| {
            format!(
                "validating {} row {} ({:?} / {:?})",
                source.display(),
                source_rows + 1,
                row.platform,
                row.emulator_name
            )
        })?;
        let import_result = if row.emulator_name.trim().is_empty() {
            unavailable_emulator_rows += 1;
            import_unavailable_emulator_row(&transaction, &provider_id, &row, import_timestamp)
        } else {
            import_row(&transaction, &provider_id, &row, import_timestamp)
        };
        import_result.with_context(|| {
            format!(
                "importing {} row {} ({:?} / {:?})",
                source.display(),
                source_rows + 1,
                row.platform,
                row.emulator_name
            )
        })?;
    }

    import_install_sources(&transaction)?;

    if source_rows == 0 {
        bail!("emulator source is empty: {}", source.display());
    }

    transaction.commit()?;

    Ok(EmulatorImportStats {
        source_rows,
        unavailable_emulator_rows,
        emulators: count(connection, "emulators")?,
        emulator_aliases: count(connection, "emulator_aliases")?,
        platforms: count(connection, "platforms")?,
        platform_links: count(connection, "emulator_platforms")?,
        host_system_links: count(connection, "emulator_host_systems")?,
        packages: count(connection, "emulator_packages")?,
        warnings: connection.query_row(
            "SELECT count(*) FROM data_quality_issues WHERE severity = 'warning' AND status = 'open'",
            [],
            |row| row.get(0),
        )?,
    })
}

fn validate_row(row: &EmulatorRow) -> Result<()> {
    if row.emulator_name.trim().is_empty() {
        return Ok(());
    }

    if let Some(flatpak_id) = optional(&row.flatpak_id)
        && !flatpak_id.starts_with("snap:")
        && (!flatpak_id.contains('.')
            || !flatpak_id
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character)))
    {
        bail!(
            "Flatpak package ID {:?} is not a reverse-DNS application ID; this often means a CSV column shifted",
            row.flatpak_id
        );
    }

    for core in row
        .retroarch_core
        .split(';')
        .map(str::trim)
        .filter(|core| !core.is_empty())
    {
        if core.ends_with("_libretro")
            || !core.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '+')
            })
        {
            bail!(
                "RetroArch core {:?} is not a runtime core slug; this often means a CSV column shifted",
                row.retroarch_core
            );
        }
    }

    Ok(())
}

fn import_row(
    connection: &Connection,
    provider_id: &str,
    row: &EmulatorRow,
    timestamp: &str,
) -> Result<()> {
    let platform_name = row.platform.trim();
    let emulator_name = row.emulator_name.trim();
    if platform_name.is_empty() {
        bail!("emulator source contains a blank platform name");
    }

    let emulator_key = normalize_key(emulator_name);
    if emulator_key.is_empty() {
        bail!("emulator source contains an emulator name without usable identity characters");
    }

    let platform_id = ensure_platform(connection, provider_id, platform_name, timestamp)?;

    let emulator_id = stable_id("emulator", &emulator_key);
    let homepage = optional(&row.homepage);
    let notes = optional(&row.notes);
    let save_directory = optional(&row.save_directory);
    let save_extensions = optional(&row.save_extensions);
    connection.execute(
        "INSERT INTO emulators\n         (id, name, normalized_name, homepage, notes, save_directory, save_extensions, created_at, updated_at)\n         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)\n         ON CONFLICT(normalized_name) DO UPDATE SET\n           homepage = COALESCE(emulators.homepage, excluded.homepage),\n           notes = COALESCE(emulators.notes, excluded.notes),\n           save_directory = COALESCE(emulators.save_directory, excluded.save_directory),\n           save_extensions = COALESCE(emulators.save_extensions, excluded.save_extensions),\n           updated_at = excluded.updated_at",
        params![
            emulator_id,
            emulator_name,
            emulator_key,
            homepage,
            notes,
            save_directory,
            save_extensions,
            timestamp
        ],
    )?;

    let canonical_emulator: (String, String) = connection.query_row(
        "SELECT id, name FROM emulators WHERE normalized_name = ?1",
        [&emulator_key],
        |record| Ok((record.get(0)?, record.get(1)?)),
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO emulator_aliases (emulator_id, alias, normalized_alias)\n         VALUES (?1, ?2, ?3)",
        params![canonical_emulator.0, emulator_name, emulator_key],
    )?;
    if canonical_emulator.1 != emulator_name
        && canonical_emulator.1.eq_ignore_ascii_case(emulator_name)
    {
        record_warning(
            connection,
            "emulator_name_case_collision",
            Some("emulator"),
            Some(&canonical_emulator.0),
            serde_json::json!({
                "canonical_name": canonical_emulator.1,
                "source_name": emulator_name
            }),
            timestamp,
        )?;
    }

    for raw_host in row
        .supported_os
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let host = normalize_host(raw_host);
        connection.execute(
            "INSERT INTO host_systems (slug, display_name, family) VALUES (?1, ?2, ?3)\n             ON CONFLICT(slug) DO NOTHING",
            params![host.slug, host.display_name, host.family],
        )?;
        connection.execute(
            "INSERT OR IGNORE INTO emulator_host_systems (emulator_id, host_system_slug)\n             VALUES (?1, ?2)",
            params![canonical_emulator.0, host.slug],
        )?;
    }

    insert_package(
        connection,
        &canonical_emulator.0,
        "windows",
        "winget",
        &row.winget_id,
        &serde_json::json!({}),
    )?;
    insert_package(
        connection,
        &canonical_emulator.0,
        "macos",
        "homebrew",
        &row.homebrew_formula,
        &serde_json::json!({}),
    )?;
    if let Some(package) = optional(&row.flatpak_id)
        && !package.starts_with("snap:")
    {
        insert_package(
            connection,
            &canonical_emulator.0,
            "linux",
            "flatpak",
            package,
            &serde_json::json!({}),
        )?;
    }

    connection.execute(
        "INSERT INTO emulator_platforms (emulator_id, platform_id, core_name, recommended)\n         VALUES (?1, ?2, ?3, 1)\n         ON CONFLICT(emulator_id, platform_id, core_name) DO UPDATE SET recommended = 1",
        params![
            canonical_emulator.0,
            platform_id,
            row.retroarch_core.trim()
        ],
    )?;

    let payload = serde_json::to_string(row)?;
    let external_id = format!(
        "{}|{}|{}",
        platform_name,
        emulator_name,
        row.retroarch_core.trim()
    );
    let source_record_id = stable_id(PROVIDER_SLUG, &external_id);
    connection.execute(
        "INSERT INTO source_records\n         (id, provider_id, record_type, external_id, payload_json, payload_sha256, imported_at)\n         VALUES (?1, ?2, 'emulator', ?3, ?4, ?5, ?6)\n         ON CONFLICT(provider_id, record_type, external_id) DO UPDATE SET\n           payload_json = excluded.payload_json,\n           payload_sha256 = excluded.payload_sha256,\n           imported_at = excluded.imported_at",
        params![
            source_record_id,
            provider_id,
            external_id,
            payload,
            sha256_bytes(payload.as_bytes()),
            timestamp
        ],
    )?;
    insert_source_link(
        connection,
        &source_record_id,
        "emulator",
        &canonical_emulator.0,
        timestamp,
    )?;
    insert_source_link(
        connection,
        &source_record_id,
        "platform",
        &platform_id,
        timestamp,
    )?;

    Ok(())
}

fn import_unavailable_emulator_row(
    connection: &Connection,
    provider_id: &str,
    row: &EmulatorRow,
    timestamp: &str,
) -> Result<()> {
    let platform_name = row.platform.trim();
    if platform_name.is_empty() {
        bail!("emulator source contains a row without a platform or emulator name");
    }
    let platform_id = ensure_platform(connection, provider_id, platform_name, timestamp)?;
    let payload = serde_json::to_string(row)?;
    let external_id = format!("{platform_name}|no-emulator");
    let source_record_id = stable_id(PROVIDER_SLUG, &external_id);
    connection.execute(
        "INSERT INTO source_records\n         (id, provider_id, record_type, external_id, payload_json, payload_sha256, imported_at)\n         VALUES (?1, ?2, 'platform', ?3, ?4, ?5, ?6)\n         ON CONFLICT(provider_id, record_type, external_id) DO UPDATE SET\n           payload_json = excluded.payload_json,\n           payload_sha256 = excluded.payload_sha256,\n           imported_at = excluded.imported_at",
        params![
            source_record_id,
            provider_id,
            external_id,
            payload,
            sha256_bytes(payload.as_bytes()),
            timestamp
        ],
    )?;
    insert_source_link(
        connection,
        &source_record_id,
        "platform",
        &platform_id,
        timestamp,
    )?;
    record_warning(
        connection,
        "no_known_emulator",
        Some("platform"),
        Some(&platform_id),
        serde_json::json!({"platform": platform_name, "notes": optional(&row.notes)}),
        timestamp,
    )?;
    Ok(())
}

fn ensure_platform(
    connection: &Connection,
    provider_id: &str,
    platform_name: &str,
    timestamp: &str,
) -> Result<String> {
    let platform_key = normalize_key(platform_name);
    if platform_key.is_empty() {
        bail!("emulator source contains a platform name without usable identity characters");
    }
    let proposed_platform_id = stable_id("platform", &platform_key);
    connection.execute(
        "INSERT INTO platforms\n         (id, canonical_name, normalized_name, status, created_at, updated_at)\n         VALUES (?1, ?2, ?3, 'provisional', ?4, ?4)\n         ON CONFLICT(normalized_name) DO NOTHING",
        params![proposed_platform_id, platform_name, platform_key, timestamp],
    )?;
    let platform_id: String = connection.query_row(
        "SELECT id FROM platforms WHERE normalized_name = ?1",
        [&platform_key],
        |record| record.get(0),
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO platform_aliases\n         (platform_id, alias, normalized_alias, provider_id) VALUES (?1, ?2, ?3, ?4)",
        params![platform_id, platform_name, platform_key, provider_id],
    )?;
    Ok(platform_id)
}

fn insert_source_link(
    connection: &Connection,
    source_record_id: &str,
    entity_type: &str,
    entity_id: &str,
    timestamp: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO source_links\n         (source_record_id, entity_type, entity_id, confidence, method, status, decided_at)\n         VALUES (?1, ?2, ?3, 1000, 'source_native', 'accepted', ?4)\n         ON CONFLICT(source_record_id, entity_type, entity_id) DO UPDATE SET\n           confidence = excluded.confidence, method = excluded.method,\n           status = excluded.status, decided_at = excluded.decided_at",
        params![source_record_id, entity_type, entity_id, timestamp],
    )?;
    Ok(())
}

fn insert_package(
    connection: &Connection,
    emulator_id: &str,
    host: &str,
    manager: &str,
    package: &str,
    metadata: &serde_json::Value,
) -> Result<()> {
    if let Some(package) = optional(package) {
        connection.execute(
            "INSERT INTO emulator_packages
                 (emulator_id, host_system_slug, manager, package_id, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(emulator_id, host_system_slug, manager, package_id) DO UPDATE SET
               metadata_json=excluded.metadata_json",
            params![emulator_id, host, manager, package, metadata.to_string()],
        )?;
    }
    Ok(())
}

fn import_install_sources(connection: &Connection) -> Result<()> {
    let catalog: InstallSourceCatalog = serde_json::from_str(INSTALL_SOURCES_JSON)
        .context("parsing sources/emulator-install-sources.json")?;
    if catalog.schema_version != 1 {
        bail!(
            "unsupported emulator install source schema version {}",
            catalog.schema_version
        );
    }
    if catalog.reviewed_at.trim().is_empty() {
        bail!("emulator install source review date is required");
    }

    for source in catalog.sources {
        let emulator_key = normalize_key(&source.emulator);
        let emulator_id = connection
            .query_row(
                "SELECT id FROM emulators WHERE normalized_name=?1",
                [&emulator_key],
                |row| row.get::<_, String>(0),
            )
            .with_context(|| {
                format!(
                    "curated install source references unknown emulator {:?}",
                    source.emulator
                )
            })?;
        let host = normalize_host(&source.host);
        if !matches!(host.slug.as_str(), "linux" | "windows" | "macos") {
            bail!(
                "curated install source for {} uses unsupported host {}",
                source.emulator,
                source.host
            );
        }
        if !matches!(
            source.manager.as_str(),
            "flatpak" | "appimage" | "nix" | "github" | "direct" | "winget" | "homebrew"
        ) {
            bail!(
                "curated install source for {} uses unsupported manager {}",
                source.emulator,
                source.manager
            );
        }
        if source.package_id.trim().is_empty() || !source.metadata.is_object() {
            bail!(
                "curated install source for {} has an invalid package id or metadata object",
                source.emulator
            );
        }

        connection.execute(
            "INSERT INTO host_systems (slug, display_name, family) VALUES (?1, ?2, ?3)
             ON CONFLICT(slug) DO NOTHING",
            params![host.slug, host.display_name, host.family],
        )?;
        connection.execute(
            "INSERT OR IGNORE INTO emulator_host_systems (emulator_id, host_system_slug)
             VALUES (?1, ?2)",
            params![emulator_id, host.slug],
        )?;
        insert_package(
            connection,
            &emulator_id,
            &host.slug,
            &source.manager,
            &source.package_id,
            &source.metadata,
        )?;
    }
    Ok(())
}

fn record_warning(
    connection: &Connection,
    issue_type: &str,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    details: serde_json::Value,
    timestamp: &str,
) -> Result<()> {
    let key = format!(
        "{issue_type}|{}|{}|{details}",
        entity_type.unwrap_or(""),
        entity_id.unwrap_or("")
    );
    connection.execute(
        "INSERT INTO data_quality_issues\n         (id, severity, issue_type, entity_type, entity_id, details_json, created_at)\n         VALUES (?1, 'warning', ?2, ?3, ?4, ?5, ?6)\n         ON CONFLICT(id) DO UPDATE SET\n           details_json = excluded.details_json, created_at = excluded.created_at",
        params![
            stable_id("quality-issue", &key),
            issue_type,
            entity_type,
            entity_id,
            details.to_string(),
            timestamp
        ],
    )?;
    Ok(())
}

fn optional(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

struct HostSystem {
    slug: String,
    display_name: String,
    family: &'static str,
}

fn normalize_host(raw: &str) -> HostSystem {
    let key = normalize_key(raw);
    let (slug, display_name, family) = match key.as_str() {
        "windows" => ("windows", "Windows", "desktop"),
        "linux" => ("linux", "Linux", "desktop"),
        "macos" => ("macos", "macOS", "desktop"),
        "freebsd" => ("freebsd", "FreeBSD", "desktop"),
        "bsd" => ("bsd", "BSD", "desktop"),
        "haiku" => ("haiku", "Haiku", "desktop"),
        "amigaos" => ("amigaos", "AmigaOS", "desktop"),
        "morphos" => ("morphos", "MorphOS", "desktop"),
        "android" => ("android", "Android", "mobile"),
        "ios" => ("ios", "iOS", "mobile"),
        "tvos" => ("tvos", "tvOS", "mobile"),
        "web" => ("web", "Web", "web"),
        "switch" => ("switch", "Nintendo Switch", "console"),
        "3ds" => ("3ds", "Nintendo 3DS", "console"),
        "nintendo-ds" => ("nintendo-ds", "Nintendo DS", "console"),
        "vita" => ("vita", "PlayStation Vita", "console"),
        "wiiu" => ("wii-u", "Nintendo Wii U", "console"),
        _ => {
            return HostSystem {
                slug: key,
                display_name: raw.trim().to_owned(),
                family: "other",
            };
        }
    };
    HostSystem {
        slug: slug.to_owned(),
        display_name: display_name.to_owned(),
        family,
    }
}

fn count(connection: &Connection, table: &str) -> Result<i64> {
    const ALLOWED: [&str; 7] = [
        "emulators",
        "emulator_aliases",
        "platforms",
        "emulator_platforms",
        "emulator_host_systems",
        "emulator_packages",
        "data_quality_issues",
    ];
    if !ALLOWED.contains(&table) {
        bail!("unsupported table count: {table}");
    }
    Ok(
        connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })?,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn host_system_aliases_are_normalized() {
        assert_eq!(normalize_host("macOS").slug, "macos");
        assert_eq!(normalize_host("Nintendo DS").family, "console");
        assert_eq!(normalize_host("WiiU").slug, "wii-u");
    }

    #[test]
    fn package_and_host_tokens_deduplicate() {
        let values: BTreeSet<_> = "Windows;Linux;Windows"
            .split(';')
            .map(|item| normalize_host(item).slug)
            .collect();
        assert_eq!(
            values,
            BTreeSet::from(["linux".to_owned(), "windows".to_owned()])
        );
    }

    #[test]
    fn emulator_rows_reject_shifted_package_and_core_columns() {
        let valid = EmulatorRow {
            platform: "Nintendo 64".into(),
            emulator_name: "Mupen64Plus-Next".into(),
            flatpak_id: "org.libretro.RetroArch".into(),
            retroarch_core: "mupen64plus_next".into(),
            ..EmulatorRow::default()
        };
        validate_row(&valid).unwrap();

        let shifted_package = EmulatorRow {
            flatpak_id: "mupen64plus_next".into(),
            ..valid.clone()
        };
        assert!(validate_row(&shifted_package).is_err());

        let shifted_core = EmulatorRow {
            retroarch_core: "~/.config/retroarch/saves".into(),
            ..valid
        };
        assert!(validate_row(&shifted_core).is_err());
    }
}
