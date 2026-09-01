use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, Transaction, params};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::external_torrent;
use crate::settings::{
    RegisteredTorrentCatalog, RegisteredTorrentMember, SettingsStore, unix_timestamp,
    validate_registered_torrent_catalog,
};

const MANIFEST_FORMAT: &str = "lunchbox-local-torrent-provider";
const SCHEMA_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_OFFERS: usize = 256;
const MAX_TOTAL_METADATA_BYTES: usize = 64 * 1024 * 1024;
const MAX_TOTAL_MEMBERS: usize = 500_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDocument {
    format: String,
    schema_version: u32,
    provider: ManifestProvider,
    offers: Vec<ManifestOffer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestProvider {
    id: String,
    name: String,
    catalog_version: String,
    authorization: String,
    #[serde(default)]
    terms_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestOffer {
    id: String,
    platform: String,
    torrent_path: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    torrent_sha256: String,
    #[serde(default)]
    info_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocalProviderManifestSummary {
    pub provider_id: String,
    pub display_name: String,
    pub catalog_version: String,
    pub terms_url: String,
    pub manifest_path: PathBuf,
    pub manifest_sha256: String,
    pub offer_count: u32,
    pub imported_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocalProviderImportOutcome {
    pub provider_id: String,
    pub display_name: String,
    pub catalog_version: String,
    pub offer_count: usize,
}

struct PreparedManifest {
    provider_id: String,
    display_name: String,
    catalog_version: String,
    authorization: String,
    terms_url: String,
    manifest_path: PathBuf,
    manifest_sha256: String,
    imported_at: i64,
    offers: Vec<PreparedOffer>,
}

struct PreparedOffer {
    offer_id: String,
    catalog: RegisteredTorrentCatalog,
}

pub(crate) fn import_manifest(
    path: &Path,
    store: &SettingsStore,
) -> Result<LocalProviderImportOutcome> {
    let prepared = inspect_manifest(path)?;
    apply_manifest(store, &prepared)?;
    Ok(LocalProviderImportOutcome {
        provider_id: prepared.provider_id,
        display_name: prepared.display_name,
        catalog_version: prepared.catalog_version,
        offer_count: prepared.offers.len(),
    })
}

pub(crate) fn list_manifests(store: &SettingsStore) -> Result<Vec<LocalProviderManifestSummary>> {
    let connection = store.connection()?;
    let mut statement = connection.prepare(
        "SELECT provider_id, display_name, catalog_version, terms_url,
                manifest_path_display, manifest_path_bytes, manifest_path_encoding,
                manifest_sha256, offer_count, imported_at
         FROM local_provider_manifests
         ORDER BY display_name COLLATE NOCASE, provider_id",
    )?;
    let rows = statement.query_map([], |row| {
        let offer_count = row.get::<_, i64>(8)?;
        let display = row.get::<_, String>(4)?;
        Ok(LocalProviderManifestSummary {
            provider_id: row.get(0)?,
            display_name: row.get(1)?,
            catalog_version: row.get(2)?,
            terms_url: row.get(3)?,
            manifest_path: crate::local_import::decode_path(
                row.get(5)?,
                &row.get::<_, String>(6)?,
                display,
            ),
            manifest_sha256: row.get(7)?,
            offer_count: u32::try_from(offer_count).unwrap_or(u32::MAX),
            imported_at: row.get(9)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub(crate) fn remove_manifest(store: &SettingsStore, provider_id: &str) -> Result<usize> {
    validate_id(provider_id, "provider ID", 80)?;
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    let old_hashes = provider_hashes(&transaction, provider_id)?;
    let removed = transaction.execute(
        "DELETE FROM local_provider_manifests WHERE provider_id=?1",
        [provider_id],
    )?;
    if removed == 0 {
        bail!("the local provider manifest is no longer installed");
    }
    cleanup_unreferenced_catalogs(&transaction, provider_id, &old_hashes)?;
    transaction.commit()?;
    Ok(old_hashes.len())
}

fn inspect_manifest(path: &Path) -> Result<PreparedManifest> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("opening local provider manifest {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("a local provider manifest must be a regular JSON file, not a symlink");
    }
    if metadata.len() == 0 || metadata.len() > MAX_MANIFEST_BYTES {
        bail!("a local provider manifest must be between 1 byte and 1 MiB");
    }
    let manifest_path = path
        .canonicalize()
        .with_context(|| format!("resolving local provider manifest {}", path.display()))?;
    let manifest_root = manifest_path
        .parent()
        .context("local provider manifest has no parent directory")?
        .to_path_buf();
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or_default());
    File::open(&manifest_path)?
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| {
            format!(
                "reading local provider manifest {}",
                manifest_path.display()
            )
        })?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        bail!("a local provider manifest must be no larger than 1 MiB");
    }
    let document: ManifestDocument =
        serde_json::from_slice(&bytes).context("decoding local provider manifest JSON")?;
    validate_document_header(&document)?;

    let mut offer_ids = HashSet::with_capacity(document.offers.len());
    let mut info_hashes = HashSet::with_capacity(document.offers.len());
    let mut total_metadata_bytes = 0_usize;
    let mut total_members = 0_usize;
    let imported_at = unix_timestamp();
    let mut offers = Vec::with_capacity(document.offers.len());
    for entry in document.offers {
        validate_id(&entry.id, "offer ID", 120)?;
        if !offer_ids.insert(entry.id.to_ascii_lowercase()) {
            bail!("local provider manifest contains a duplicate offer ID");
        }
        validate_text(&entry.platform, "offer platform", 1, 512)?;
        let platform_key = crate::catalog::normalize_platform_key(&entry.platform);
        if platform_key.is_empty() || entry.platform == "Unassigned platform" {
            bail!("every local provider offer requires one exact platform");
        }
        validate_relative_portable_path(&entry.torrent_path)?;
        if !entry.name.is_empty() {
            validate_text(&entry.name, "offer name", 1, 120)?;
        }
        validate_optional_hash(&entry.torrent_sha256, 64, "torrent SHA-256")?;
        validate_optional_hash(&entry.info_hash, 40, "torrent info hash")?;

        let torrent_path = manifest_root.join(&entry.torrent_path);
        let torrent_metadata = fs::symlink_metadata(&torrent_path).with_context(|| {
            format!(
                "opening torrent {:?} for offer {}",
                entry.torrent_path, entry.id
            )
        })?;
        if torrent_metadata.file_type().is_symlink() || !torrent_metadata.is_file() {
            bail!("offer {} must reference a regular .torrent file", entry.id);
        }
        let torrent_path = torrent_path.canonicalize().with_context(|| {
            format!(
                "resolving torrent {:?} for offer {}",
                entry.torrent_path, entry.id
            )
        })?;
        if !torrent_path.starts_with(&manifest_root) {
            bail!("offer {} resolves outside the manifest directory", entry.id);
        }
        let mut offer = external_torrent::inspect_torrent_file(&torrent_path)
            .with_context(|| format!("reviewing local provider offer {}", entry.id))?;
        if !entry.torrent_sha256.is_empty()
            && !entry
                .torrent_sha256
                .eq_ignore_ascii_case(&offer.torrent_sha256)
        {
            bail!(
                "offer {} does not match its declared torrent SHA-256",
                entry.id
            );
        }
        if !entry.info_hash.is_empty() && !entry.info_hash.eq_ignore_ascii_case(&offer.info_hash) {
            bail!(
                "offer {} does not match its declared v1 info hash",
                entry.id
            );
        }
        if !info_hashes.insert(offer.info_hash.clone()) {
            bail!("local provider manifest repeats the same torrent info hash");
        }
        total_metadata_bytes = total_metadata_bytes
            .checked_add(offer.torrent_bytes.len())
            .context("local provider metadata size overflows")?;
        total_members = total_members
            .checked_add(offer.files.len())
            .context("local provider member count overflows")?;
        if total_metadata_bytes > MAX_TOTAL_METADATA_BYTES {
            bail!("local provider manifest exceeds the 64 MiB total metadata limit");
        }
        if total_members > MAX_TOTAL_MEMBERS {
            bail!("local provider manifest exceeds the 500000 member limit");
        }
        offer.source_label = if entry.name.is_empty() {
            format!("{} · {}", document.provider.name, entry.id)
        } else {
            format!("{} · {}", document.provider.name, entry.name)
        };
        let members = offer
            .files
            .iter()
            .map(|file| RegisteredTorrentMember {
                index: file.index,
                path: file.path.clone(),
                byte_size: file.byte_size,
                title_key: crate::game_details::torrent_member_title_key(&file.path),
            })
            .collect::<Vec<_>>();
        let catalog = RegisteredTorrentCatalog {
            platform: entry.platform,
            platform_key,
            source_kind: "file".to_owned(),
            source_label: offer.source_label,
            source_file_name: offer.source_file_name,
            torrent_name: offer.torrent_name,
            torrent_sha256: offer.torrent_sha256,
            info_hash: offer.info_hash,
            torrent_bytes: offer.torrent_bytes,
            total_bytes: offer.total_bytes,
            members,
            registered_at: imported_at,
        };
        validate_registered_torrent_catalog(&catalog)?;
        offers.push(PreparedOffer {
            offer_id: entry.id,
            catalog,
        });
    }

    Ok(PreparedManifest {
        provider_id: document.provider.id,
        display_name: document.provider.name,
        catalog_version: document.provider.catalog_version,
        authorization: document.provider.authorization,
        terms_url: document.provider.terms_url,
        manifest_path,
        manifest_sha256: hex::encode(Sha256::digest(bytes)),
        imported_at,
        offers,
    })
}

fn validate_document_header(document: &ManifestDocument) -> Result<()> {
    if document.format != MANIFEST_FORMAT || document.schema_version != SCHEMA_VERSION {
        bail!(
            "unsupported local provider manifest format {:?} schema {}",
            document.format,
            document.schema_version
        );
    }
    validate_id(&document.provider.id, "provider ID", 80)?;
    validate_text(&document.provider.name, "provider name", 1, 120)?;
    validate_text(&document.provider.catalog_version, "catalog version", 1, 80)?;
    if document.provider.authorization != "user-managed" {
        bail!("local manifests require authorization \"user-managed\"");
    }
    if !document.provider.terms_url.is_empty() {
        validate_text(&document.provider.terms_url, "terms URL", 1, 2048)?;
        let terms = url::Url::parse(&document.provider.terms_url)
            .context("parsing local provider terms URL")?;
        if terms.scheme() != "https"
            || terms.host_str().is_none()
            || !terms.username().is_empty()
            || terms.password().is_some()
        {
            bail!("local provider terms URL must be credential-free HTTPS");
        }
    }
    if !(1..=MAX_OFFERS).contains(&document.offers.len()) {
        bail!("a local provider manifest must contain between 1 and 256 offers");
    }
    Ok(())
}

fn validate_id(value: &str, label: &str, maximum: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > maximum
        || value.starts_with(['-', '.'])
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
    {
        bail!("{label} must use lowercase ASCII letters, digits, dots, dashes, or underscores");
    }
    Ok(())
}

fn validate_text(value: &str, label: &str, minimum: usize, maximum: usize) -> Result<()> {
    let count = value.chars().count();
    if !(minimum..=maximum).contains(&count) || value.trim() != value {
        bail!("{label} must contain between {minimum} and {maximum} trimmed characters");
    }
    if value.chars().any(char::is_control) {
        bail!("{label} contains control characters");
    }
    Ok(())
}

fn validate_optional_hash(value: &str, length: usize, label: &str) -> Result<()> {
    if !value.is_empty()
        && (value.len() != length || !value.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        bail!("{label} must be an optional {length}-character hexadecimal value");
    }
    Ok(())
}

fn validate_relative_portable_path(value: &str) -> Result<()> {
    validate_text(value, "torrent path", 1, 4096)?;
    if value.contains('\\') || Path::new(value).is_absolute() {
        bail!("torrent paths must be portable relative paths using forward slashes");
    }
    let components = Path::new(value).components().collect::<Vec<_>>();
    if components.is_empty()
        || components
            .iter()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("torrent paths cannot contain empty, current, or parent components");
    }
    for component in value.split('/') {
        if component.is_empty()
            || component.ends_with([' ', '.'])
            || component.chars().any(|character| {
                character.is_control()
                    || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
            })
        {
            bail!("torrent path contains a component that is not portable");
        }
        let stem = component.split('.').next().unwrap_or_default();
        let upper = stem.to_ascii_uppercase();
        if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || upper.strip_prefix("COM").is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
            || upper.strip_prefix("LPT").is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
        {
            bail!("torrent path uses a Windows-reserved component");
        }
    }
    if !value
        .rsplit('/')
        .next()
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".torrent"))
    {
        bail!("every local provider offer must reference a .torrent file");
    }
    Ok(())
}

fn apply_manifest(store: &SettingsStore, manifest: &PreparedManifest) -> Result<()> {
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    validate_same_version_is_immutable(&transaction, manifest)?;
    let old_hashes = provider_hashes(&transaction, &manifest.provider_id)?;
    let encoded = crate::local_import::encode_path(&manifest.manifest_path);
    if encoded.bytes.is_empty() || encoded.bytes.len() > 32768 {
        bail!("local provider manifest path exceeds the native-path limit");
    }
    transaction.execute(
        "INSERT INTO local_provider_manifests (
             provider_id, display_name, catalog_version, authorization, terms_url,
             manifest_path_display, manifest_path_bytes, manifest_path_encoding,
             manifest_sha256, offer_count, imported_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(provider_id) DO UPDATE SET
             display_name=excluded.display_name,
             catalog_version=excluded.catalog_version,
             authorization=excluded.authorization,
             terms_url=excluded.terms_url,
             manifest_path_display=excluded.manifest_path_display,
             manifest_path_bytes=excluded.manifest_path_bytes,
             manifest_path_encoding=excluded.manifest_path_encoding,
             manifest_sha256=excluded.manifest_sha256,
             offer_count=excluded.offer_count,
             imported_at=excluded.imported_at",
        params![
            manifest.provider_id,
            manifest.display_name,
            manifest.catalog_version,
            manifest.authorization,
            manifest.terms_url,
            encoded.display,
            encoded.bytes,
            encoded.encoding,
            manifest.manifest_sha256,
            i64::try_from(manifest.offers.len()).context("provider offer count is too large")?,
            manifest.imported_at,
        ],
    )?;
    transaction.execute(
        "DELETE FROM local_provider_manifest_offers WHERE provider_id=?1",
        [&manifest.provider_id],
    )?;
    for offer in &manifest.offers {
        upsert_provider_catalog(&transaction, &manifest.provider_id, &offer.catalog)?;
        transaction.execute(
            "INSERT INTO local_provider_manifest_offers (
                 provider_id, offer_id, info_hash, torrent_sha256, platform, platform_key
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                manifest.provider_id,
                offer.offer_id,
                offer.catalog.info_hash.to_ascii_lowercase(),
                offer.catalog.torrent_sha256.to_ascii_lowercase(),
                offer.catalog.platform,
                offer.catalog.platform_key,
            ],
        )?;
    }
    let new_hashes = manifest
        .offers
        .iter()
        .map(|offer| offer.catalog.info_hash.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let removed_hashes = old_hashes
        .into_iter()
        .filter(|hash| !new_hashes.contains(hash))
        .collect::<Vec<_>>();
    cleanup_unreferenced_catalogs(&transaction, &manifest.provider_id, &removed_hashes)?;
    transaction.commit()?;
    Ok(())
}

fn validate_same_version_is_immutable(
    transaction: &Transaction<'_>,
    manifest: &PreparedManifest,
) -> Result<()> {
    let current_version = transaction
        .query_row(
            "SELECT catalog_version FROM local_provider_manifests WHERE provider_id=?1",
            [&manifest.provider_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if current_version.as_deref() != Some(manifest.catalog_version.as_str()) {
        return Ok(());
    }
    let mut current = BTreeMap::new();
    let mut statement = transaction.prepare(
        "SELECT offer_id, info_hash, torrent_sha256, platform_key
         FROM local_provider_manifest_offers WHERE provider_id=?1",
    )?;
    let rows = statement.query_map([&manifest.provider_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (offer_id, info_hash, torrent_sha256, platform_key) = row?;
        current.insert(offer_id, (info_hash, torrent_sha256, platform_key));
    }
    let proposed = manifest
        .offers
        .iter()
        .map(|offer| {
            (
                offer.offer_id.clone(),
                (
                    offer.catalog.info_hash.to_ascii_lowercase(),
                    offer.catalog.torrent_sha256.to_ascii_lowercase(),
                    offer.catalog.platform_key.clone(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if current != proposed {
        bail!(
            "provider {} changed offers without changing catalog_version; bump the manifest version before resyncing",
            manifest.provider_id
        );
    }
    Ok(())
}

fn upsert_provider_catalog(
    transaction: &Transaction<'_>,
    provider_id: &str,
    catalog: &RegisteredTorrentCatalog,
) -> Result<()> {
    validate_registered_torrent_catalog(catalog)?;
    let existing = transaction
        .query_row(
            "SELECT id, torrent_sha256, platform_key, managed_by_provider_id
             FROM registered_torrent_catalogs WHERE info_hash=?1",
            [catalog.info_hash.to_ascii_lowercase()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let source_id = match existing {
        Some((_source_id, torrent_sha256, platform_key, manager)) if manager != provider_id => {
            if !torrent_sha256.eq_ignore_ascii_case(&catalog.torrent_sha256)
                || platform_key != catalog.platform_key
            {
                bail!(
                    "torrent {} is already registered with different metadata or platform provenance",
                    catalog.info_hash
                );
            }
            return Ok(());
        }
        Some((source_id, torrent_sha256, platform_key, _)) => {
            let shared_references: i64 = transaction.query_row(
                "SELECT count(*) FROM local_provider_manifest_offers
                 WHERE info_hash=?1 AND provider_id<>?2",
                params![catalog.info_hash.to_ascii_lowercase(), provider_id],
                |row| row.get(0),
            )?;
            if shared_references > 0
                && (!torrent_sha256.eq_ignore_ascii_case(&catalog.torrent_sha256)
                    || platform_key != catalog.platform_key)
            {
                bail!(
                    "torrent {} is shared by another provider and cannot change platform provenance",
                    catalog.info_hash
                );
            }
            transaction.execute(
                "UPDATE registered_torrent_catalogs SET
                     platform=?1, platform_key=?2, source_kind=?3, source_label=?4,
                     source_file_name=?5, torrent_name=?6, torrent_sha256=?7,
                     torrent_bytes=?8, file_count=?9, total_bytes=?10,
                     registered_at=?11, managed_by_provider_id=?12
                 WHERE id=?13",
                params![
                    catalog.platform,
                    catalog.platform_key,
                    catalog.source_kind,
                    catalog.source_label,
                    catalog.source_file_name,
                    catalog.torrent_name,
                    catalog.torrent_sha256.to_ascii_lowercase(),
                    catalog.torrent_bytes,
                    i64::try_from(catalog.members.len())
                        .context("torrent file count is too large")?,
                    i64::try_from(catalog.total_bytes).context("torrent payload is too large")?,
                    catalog.registered_at,
                    provider_id,
                    source_id,
                ],
            )?;
            source_id
        }
        None => {
            transaction.execute(
                "INSERT INTO registered_torrent_catalogs (
                     platform, platform_key, source_kind, source_label,
                     source_file_name, torrent_name, torrent_sha256, info_hash,
                     torrent_bytes, file_count, total_bytes, registered_at,
                     managed_by_provider_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    catalog.platform,
                    catalog.platform_key,
                    catalog.source_kind,
                    catalog.source_label,
                    catalog.source_file_name,
                    catalog.torrent_name,
                    catalog.torrent_sha256.to_ascii_lowercase(),
                    catalog.info_hash.to_ascii_lowercase(),
                    catalog.torrent_bytes,
                    i64::try_from(catalog.members.len())
                        .context("torrent file count is too large")?,
                    i64::try_from(catalog.total_bytes).context("torrent payload is too large")?,
                    catalog.registered_at,
                    provider_id,
                ],
            )?;
            transaction.last_insert_rowid()
        }
    };
    transaction.execute(
        "DELETE FROM registered_torrent_members WHERE source_id=?1",
        [source_id],
    )?;
    let mut insert = transaction.prepare(
        "INSERT INTO registered_torrent_members (
             source_id, file_index, file_path, byte_size, title_key
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for member in &catalog.members {
        insert.execute(params![
            source_id,
            i64::from(member.index),
            member.path,
            i64::try_from(member.byte_size).context("torrent member is too large")?,
            member.title_key,
        ])?;
    }
    Ok(())
}

fn provider_hashes(transaction: &Transaction<'_>, provider_id: &str) -> Result<Vec<String>> {
    let mut statement = transaction.prepare(
        "SELECT info_hash FROM local_provider_manifest_offers
         WHERE provider_id=?1 ORDER BY offer_id",
    )?;
    let rows = statement.query_map([provider_id], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn cleanup_unreferenced_catalogs(
    transaction: &Transaction<'_>,
    provider_id: &str,
    hashes: &[String],
) -> Result<()> {
    for info_hash in hashes {
        let manager = transaction
            .query_row(
                "SELECT managed_by_provider_id FROM registered_torrent_catalogs
                 WHERE info_hash=?1",
                [info_hash],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if manager.as_deref() != Some(provider_id) {
            continue;
        }
        let references: i64 = transaction.query_row(
            "SELECT count(*) FROM local_provider_manifest_offers WHERE info_hash=?1",
            [info_hash],
            |row| row.get(0),
        )?;
        if references == 0 {
            transaction.execute(
                "DELETE FROM registered_torrent_catalogs WHERE info_hash=?1",
                [info_hash],
            )?;
        } else {
            transaction.execute(
                "UPDATE registered_torrent_catalogs SET managed_by_provider_id=''
                 WHERE info_hash=?1",
                [info_hash],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_manifest(root: &Path, version: &str, offers: &[(&str, &str, &str)]) -> PathBuf {
        let path = root.join("provider.json");
        let offers = offers
            .iter()
            .map(|(id, platform, torrent_path)| {
                serde_json::json!({
                    "id": id,
                    "platform": platform,
                    "torrent_path": torrent_path
                })
            })
            .collect::<Vec<_>>();
        let document = serde_json::json!({
            "format": MANIFEST_FORMAT,
            "schema_version": SCHEMA_VERSION,
            "provider": {
                "id": "my-library",
                "name": "My Library",
                "catalog_version": version,
                "authorization": "user-managed",
                "terms_url": "https://example.invalid/terms"
            },
            "offers": offers
        });
        fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        path
    }

    fn write_probe(path: &Path) {
        let mut file = File::create(path).unwrap();
        file.write_all(&external_torrent::probe_fixture_torrent_bytes())
            .unwrap();
        file.sync_all().unwrap();
    }

    #[test]
    fn imports_resyncs_and_removes_a_local_provider_without_queueing_payload() {
        let directory = tempfile::tempdir().unwrap();
        let torrent = directory.path().join("switch.torrent");
        write_probe(&torrent);
        let manifest = write_manifest(
            directory.path(),
            "2026.09",
            &[("switch-volume", "Nintendo Switch", "switch.torrent")],
        );
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();

        let first = import_manifest(&manifest, &store).unwrap();
        assert_eq!(first.provider_id, "my-library");
        assert_eq!(first.offer_count, 1);
        assert!(store.jobs().unwrap().is_empty());
        let summaries = list_manifests(&store).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].manifest_path, manifest.canonicalize().unwrap());
        let sources = store
            .registered_torrent_sources_for_platform("nintendo-switch")
            .unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_label, "My Library · switch-volume");

        let repeated = import_manifest(&manifest, &store).unwrap();
        assert_eq!(repeated, first);
        assert_eq!(
            store
                .registered_torrent_sources_for_platform("nintendo-switch")
                .unwrap()
                .len(),
            1
        );

        assert_eq!(remove_manifest(&store, "my-library").unwrap(), 1);
        assert!(list_manifests(&store).unwrap().is_empty());
        assert!(
            store
                .registered_torrent_sources_for_platform("nintendo-switch")
                .unwrap()
                .is_empty()
        );
        assert!(torrent.is_file());
    }

    #[test]
    fn rejects_silent_offer_changes_and_accepts_an_explicit_version_bump() {
        let directory = tempfile::tempdir().unwrap();
        let first_torrent = directory.path().join("first.torrent");
        let second_torrent = directory.path().join("second.torrent");
        write_probe(&first_torrent);
        let mut second_bytes = external_torrent::probe_fixture_torrent_bytes();
        let tracker = b"https://tracker.example.invalid/announce";
        let replacement = b"https://tracker.example.invalid/changed!";
        let position = second_bytes
            .windows(tracker.len())
            .position(|window| window == tracker)
            .unwrap();
        second_bytes.splice(
            position..position + tracker.len(),
            replacement.iter().copied(),
        );
        fs::write(&second_torrent, second_bytes).unwrap();
        let manifest = write_manifest(
            directory.path(),
            "1",
            &[("volume", "Nintendo Switch", "first.torrent")],
        );
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        import_manifest(&manifest, &store).unwrap();

        write_manifest(
            directory.path(),
            "1",
            &[("volume", "Nintendo Switch", "second.torrent")],
        );
        let error = import_manifest(&manifest, &store).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("without changing catalog_version")
        );

        write_manifest(
            directory.path(),
            "2",
            &[("volume", "Nintendo Switch", "second.torrent")],
        );
        import_manifest(&manifest, &store).unwrap();
        assert_eq!(list_manifests(&store).unwrap()[0].catalog_version, "2");
    }

    #[test]
    fn manual_registration_relinquishes_provider_ownership_before_removal() {
        let directory = tempfile::tempdir().unwrap();
        let torrent = directory.path().join("switch.torrent");
        write_probe(&torrent);
        let manifest = write_manifest(
            directory.path(),
            "1",
            &[("switch-volume", "Nintendo Switch", "switch.torrent")],
        );
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        import_manifest(&manifest, &store).unwrap();

        let offer = external_torrent::inspect_torrent_file(&torrent).unwrap();
        let manual = RegisteredTorrentCatalog {
            platform: "Nintendo Switch".to_owned(),
            platform_key: "nintendo-switch".to_owned(),
            source_kind: "file".to_owned(),
            source_label: "Manually reviewed source".to_owned(),
            source_file_name: offer.source_file_name,
            torrent_name: offer.torrent_name,
            torrent_sha256: offer.torrent_sha256,
            info_hash: offer.info_hash,
            torrent_bytes: offer.torrent_bytes,
            total_bytes: offer.total_bytes,
            members: offer
                .files
                .iter()
                .map(|file| RegisteredTorrentMember {
                    index: file.index,
                    path: file.path.clone(),
                    byte_size: file.byte_size,
                    title_key: crate::game_details::torrent_member_title_key(&file.path),
                })
                .collect(),
            registered_at: unix_timestamp(),
        };
        store.register_torrent_catalog(&manual).unwrap();

        remove_manifest(&store, "my-library").unwrap();
        let sources = store
            .registered_torrent_sources_for_platform("nintendo-switch")
            .unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_label, "Manually reviewed source");
    }

    #[test]
    fn rejects_path_escape_unknown_fields_and_declared_hash_mismatch() {
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        write_probe(&outside.path().join("outside.torrent"));
        let manifest = write_manifest(
            directory.path(),
            "1",
            &[("escape", "Nintendo Switch", "../outside.torrent")],
        );
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        assert!(import_manifest(&manifest, &store).is_err());

        let invalid = serde_json::json!({
            "format": MANIFEST_FORMAT,
            "schema_version": SCHEMA_VERSION,
            "provider": {
                "id": "my-library",
                "name": "My Library",
                "catalog_version": "1",
                "authorization": "user-managed",
                "unexpected": true
            },
            "offers": []
        });
        fs::write(&manifest, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(import_manifest(&manifest, &store).is_err());

        let torrent = directory.path().join("source.torrent");
        write_probe(&torrent);
        let mismatch = serde_json::json!({
            "format": MANIFEST_FORMAT,
            "schema_version": SCHEMA_VERSION,
            "provider": {
                "id": "my-library",
                "name": "My Library",
                "catalog_version": "1",
                "authorization": "user-managed"
            },
            "offers": [{
                "id": "source",
                "platform": "Nintendo Switch",
                "torrent_path": "source.torrent",
                "torrent_sha256": "0".repeat(64)
            }]
        });
        fs::write(&manifest, serde_json::to_vec(&mismatch).unwrap()).unwrap();
        let error = import_manifest(&manifest, &store).unwrap_err();
        assert!(error.to_string().contains("declared torrent SHA-256"));
    }
}
