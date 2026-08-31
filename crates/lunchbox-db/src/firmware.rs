use std::collections::{BTreeSet, HashSet};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::ids::{normalize_key, sha256_bytes};

pub const RULES_JSON: &str = include_str!("../../../sources/firmware-rules.json");

#[derive(Debug, Deserialize)]
struct FirmwareCatalog {
    schema_version: u32,
    reviewed_at: String,
    provenance: FirmwareProvenance,
    acquisition_sources: Vec<FirmwareSource>,
    rules: Vec<FirmwareRule>,
}

#[derive(Debug, Deserialize)]
struct FirmwareProvenance {
    origin: String,
    license: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct FirmwareSource {
    id: String,
    transport: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    torrent_file: Option<String>,
    #[serde(default)]
    path_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FirmwareRule {
    rule_key: String,
    runtime_kind: String,
    runtime_name: String,
    platform_name: String,
    source: String,
    source_package_name: String,
    target_subdir: String,
    install_mode: String,
    required: bool,
    notes: String,
    supports_hle_fallback: bool,
    target_strategy: String,
}

#[derive(Debug, Default, Serialize)]
pub struct FirmwareImportStats {
    pub sources: i64,
    pub rules: i64,
    pub runtime_kinds: i64,
    pub platforms: i64,
    pub source_sha256: String,
}

pub fn import(connection: &mut Connection) -> Result<FirmwareImportStats> {
    let catalog: FirmwareCatalog =
        serde_json::from_str(RULES_JSON).context("parsing sources/firmware-rules.json")?;
    validate_catalog(&catalog)?;
    let known_cores = known_retroarch_cores(connection)?;
    let transaction = connection.transaction()?;

    transaction.execute("DELETE FROM firmware_rules", [])?;
    transaction.execute("DELETE FROM firmware_sources", [])?;

    for source in &catalog.acquisition_sources {
        transaction.execute(
            "INSERT INTO firmware_sources (
                 id, transport, locator_url, torrent_file, path_prefix, metadata_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, '{}')",
            params![
                source.id,
                source.transport,
                source.url,
                source.torrent_file,
                source.path_prefix
            ],
        )?;
    }

    let mut resolved_platforms = BTreeSet::new();
    let mut runtime_kinds = BTreeSet::new();
    for rule in &catalog.rules {
        let emulator_id = resolve_emulator_id(&transaction, rule)?;
        let platform_id = resolve_platform_id(&transaction, &rule.platform_name)?;
        if rule.runtime_kind == "retroarch" && !known_cores.contains(&rule.runtime_name) {
            bail!(
                "firmware rule {} references unknown canonical RetroArch core {}",
                rule.rule_key,
                rule.runtime_name
            );
        }
        transaction.execute(
            "INSERT INTO firmware_rules (
                 rule_key, runtime_kind, runtime_name, emulator_id, platform_id,
                 source_id, source_package_name, target_subdir, install_mode,
                 target_strategy, required, supports_hle_fallback, notes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                rule.rule_key,
                rule.runtime_kind,
                rule.runtime_name,
                emulator_id,
                platform_id,
                rule.source,
                rule.source_package_name,
                rule.target_subdir,
                rule.install_mode,
                rule.target_strategy,
                rule.required,
                rule.supports_hle_fallback,
                rule.notes
            ],
        )?;
        resolved_platforms.insert(platform_id);
        runtime_kinds.insert(rule.runtime_kind.as_str());
    }

    transaction.commit()?;
    Ok(FirmwareImportStats {
        sources: count(connection, "firmware_sources")?,
        rules: count(connection, "firmware_rules")?,
        runtime_kinds: i64::try_from(runtime_kinds.len()).unwrap_or(i64::MAX),
        platforms: i64::try_from(resolved_platforms.len()).unwrap_or(i64::MAX),
        source_sha256: sha256_bytes(RULES_JSON.as_bytes()),
    })
}

fn validate_catalog(catalog: &FirmwareCatalog) -> Result<()> {
    if catalog.schema_version != 1 {
        bail!(
            "unsupported firmware catalog schema version {}",
            catalog.schema_version
        );
    }
    if catalog.reviewed_at.trim().is_empty()
        || catalog.provenance.origin.trim().is_empty()
        || catalog.provenance.license != "MIT"
        || catalog.provenance.note.trim().is_empty()
    {
        bail!("firmware catalog provenance is incomplete");
    }
    if catalog.rules.is_empty() || catalog.acquisition_sources.is_empty() {
        bail!("firmware catalog must contain sources and rules");
    }

    let mut source_ids = HashSet::new();
    for source in &catalog.acquisition_sources {
        if !source_ids.insert(source.id.as_str()) || source.id.trim().is_empty() {
            bail!("duplicate or empty firmware source identity {}", source.id);
        }
        match source.transport.as_str() {
            "minerva"
                if source
                    .torrent_file
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                    && source
                        .path_prefix
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty()) =>
            {
                if source.url.is_some() {
                    bail!("Minerva firmware source {} must not use a URL", source.id);
                }
            }
            "github_release" | "direct"
                if source
                    .url
                    .as_deref()
                    .is_some_and(|value| value.starts_with("https://")) =>
            {
                if source.torrent_file.is_some() || source.path_prefix.is_some() {
                    bail!("HTTPS firmware source {} has Minerva fields", source.id);
                }
            }
            "manual" if source.url.is_none() && source.torrent_file.is_none() => {}
            _ => bail!("firmware source {} has invalid transport fields", source.id),
        }
    }

    let mut rule_keys = HashSet::new();
    let mut identities = HashSet::new();
    for rule in &catalog.rules {
        let values = [
            rule.rule_key.as_str(),
            rule.runtime_kind.as_str(),
            rule.runtime_name.as_str(),
            rule.platform_name.as_str(),
            rule.source.as_str(),
            rule.source_package_name.as_str(),
            rule.notes.as_str(),
        ];
        if values.iter().any(|value| value.trim().is_empty()) {
            bail!("firmware rule contains an empty required field");
        }
        if values
            .iter()
            .any(|value| value.chars().any(char::is_control))
        {
            bail!(
                "firmware rule {} contains a control character",
                rule.rule_key
            );
        }
        if !rule_keys.insert(rule.rule_key.as_str()) {
            bail!("duplicate firmware rule key {}", rule.rule_key);
        }
        if !source_ids.contains(rule.source.as_str()) {
            bail!(
                "firmware rule {} references unknown source {}",
                rule.rule_key,
                rule.source
            );
        }
        if !matches!(rule.install_mode.as_str(), "merge_tree" | "copy_archive") {
            bail!("firmware rule {} has invalid install mode", rule.rule_key);
        }
        if !matches!(
            rule.target_strategy.as_str(),
            "runtime_dir" | "launch_scoped" | "mame_rompath" | "manual_import" | "managed_import"
        ) {
            bail!(
                "firmware rule {} has invalid target strategy",
                rule.rule_key
            );
        }
        if rule.source.starts_with("manual:")
            != matches!(
                rule.target_strategy.as_str(),
                "manual_import" | "managed_import"
            )
        {
            bail!(
                "firmware rule {} has inconsistent manual target strategy",
                rule.rule_key
            );
        }
        if rule.supports_hle_fallback && rule.required {
            bail!(
                "firmware rule {} cannot require firmware and declare an HLE fallback",
                rule.rule_key
            );
        }
        let identity = (
            rule.runtime_kind.as_str(),
            rule.runtime_name.as_str(),
            normalize_key(&rule.platform_name),
            rule.source.as_str(),
            rule.source_package_name.as_str(),
            rule.target_subdir.as_str(),
            rule.install_mode.as_str(),
        );
        if !identities.insert(identity) {
            bail!("duplicate normalized firmware rule {}", rule.rule_key);
        }
    }
    Ok(())
}

fn resolve_emulator_id(connection: &Connection, rule: &FirmwareRule) -> Result<String> {
    let name = if rule.runtime_kind == "retroarch" {
        "RetroArch"
    } else {
        &rule.runtime_name
    };
    let key = normalize_key(name);
    connection
        .query_row(
            "SELECT id FROM emulators WHERE normalized_name=?1",
            [key],
            |row| row.get(0),
        )
        .optional()?
        .with_context(|| {
            format!(
                "firmware rule {} references unknown exact emulator {}",
                rule.rule_key, name
            )
        })
}

fn resolve_platform_id(connection: &Connection, name: &str) -> Result<String> {
    let key = normalize_key(name);
    let mut statement = connection.prepare(
        "SELECT id FROM platforms WHERE normalized_name=?1
         UNION
         SELECT platform_id FROM platform_aliases WHERE normalized_alias=?1",
    )?;
    let rows = statement.query_map([key], |row| row.get::<_, String>(0))?;
    let ids = rows.collect::<rusqlite::Result<BTreeSet<_>>>()?;
    match ids.len() {
        1 => Ok(ids
            .into_iter()
            .next()
            .expect("single exact platform identity")),
        0 => bail!("firmware catalog references unknown exact platform {name}"),
        _ => bail!("firmware platform alias {name} resolves to multiple identities"),
    }
}

fn known_retroarch_cores(connection: &Connection) -> Result<BTreeSet<String>> {
    let mut statement = connection.prepare(
        "SELECT core_name FROM emulator_platforms WHERE core_name<>'' ORDER BY core_name",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut cores = BTreeSet::new();
    for row in rows {
        for core in row?
            .split(';')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            cores.insert(core.to_owned());
        }
    }
    Ok(cores)
}

fn count(connection: &Connection, table: &str) -> Result<i64> {
    if !matches!(table, "firmware_sources" | "firmware_rules") {
        bail!("unsupported firmware table count {table}");
    }
    Ok(
        connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_catalog_has_unique_exact_rules_and_sources() {
        let catalog: FirmwareCatalog = serde_json::from_str(RULES_JSON).unwrap();
        validate_catalog(&catalog).unwrap();
        assert_eq!(catalog.rules.len(), 124);
        assert_eq!(catalog.acquisition_sources.len(), 19);
        assert!(catalog.rules.iter().any(|rule| {
            rule.runtime_kind == "retroarch"
                && rule.runtime_name == "swanstation"
                && rule.platform_name == "Sony Playstation"
        }));
        assert!(
            catalog.rules.iter().any(|rule| {
                rule.runtime_kind == "mame" && rule.target_strategy == "mame_rompath"
            })
        );
        assert!(catalog.rules.iter().any(|rule| {
            rule.runtime_kind == "demul" && rule.target_strategy == "manual_import"
        }));
        assert!(catalog.rules.iter().any(|rule| {
            rule.runtime_kind == "eden"
                && rule.platform_name == "Nintendo Switch"
                && rule.target_strategy == "managed_import"
        }));
    }
}
