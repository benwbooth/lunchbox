use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rmpv::Value;
use rusqlite::{Connection, params};
use serde::Serialize;

use crate::ids::{normalize_key, stable_id};
use crate::source::{self, LibretroSourceManifest};

const PROVIDER_SLUG: &str = "libretro-database";
const RDB_MAGIC: &[u8; 8] = b"RARCHDB\0";
const RECORD_ID_SHIFT: i64 = 1_i64 << 32;

#[derive(Debug, Default, Clone, Serialize)]
pub struct LibretroImportStats {
    pub rdb_files: usize,
    pub rdb_records: usize,
    pub platforms_reused: usize,
    pub platforms_created: usize,
    pub titled_records: usize,
    pub hashes: usize,
    pub serial_identifiers: usize,
    pub records_without_title: usize,
    pub records_without_hash_or_serial: usize,
}

struct PlatformResolution {
    id: String,
    created: bool,
}

pub fn import(
    connection: &mut Connection,
    manifest_path: &Path,
    rdb_directory: &Path,
    import_timestamp: &str,
) -> Result<LibretroImportStats> {
    let manifest = source::load_libretro_manifest(manifest_path)?;
    source::verify_prepared_libretro(&manifest, rdb_directory)?;
    let provider_id: String = connection.query_row(
        "SELECT id FROM providers WHERE slug=?1",
        [PROVIDER_SLUG],
        |row| row.get(0),
    )?;
    let snapshot_id = stable_id(PROVIDER_SLUG, &manifest.revision);

    let transaction = connection.transaction()?;
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
            manifest.revision,
            manifest.archive_url,
            manifest.archive_sha256,
            manifest.archive_bytes,
            manifest.data_license,
            import_timestamp
        ],
    )?;

    // This replaces only the generated evidence pack. Canonical entities, user data, other
    // providers, and historical source-snapshot metadata remain untouched.
    transaction.execute("DELETE FROM libretro_databases", [])?;

    let mut rdb_files = list_rdb_files(rdb_directory)?;
    rdb_files.sort();
    let mut stats = LibretroImportStats::default();
    for (database_index, rdb_path) in rdb_files.into_iter().enumerate() {
        let database_id = i64::try_from(database_index + 1)?;
        let database_name = rdb_path
            .file_stem()
            .and_then(|value| value.to_str())
            .context("Libretro RDB filename is not valid UTF-8")?;
        let platform = ensure_platform(
            &transaction,
            &provider_id,
            database_name,
            &manifest,
            import_timestamp,
        )?;
        if platform.created {
            stats.platforms_created += 1;
        } else {
            stats.platforms_reused += 1;
        }

        let source_path = format!("rdb/{database_name}.rdb");
        let records = read_rdb(&rdb_path)
            .with_context(|| format!("reading Libretro database {source_path}"))?;
        transaction.execute(
            "INSERT INTO libretro_databases
             (id, source_snapshot_id, source_path, source_name, platform_id, record_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                database_id,
                snapshot_id,
                source_path,
                database_name,
                platform.id,
                i64::try_from(records.len())?
            ],
        )?;

        stats.rdb_files += 1;
        for (ordinal, fields) in records.into_iter().enumerate() {
            import_record(&transaction, database_id, ordinal, fields, &mut stats)
                .with_context(|| format!("importing {source_path} record {ordinal}"))?;
        }
    }

    transaction.commit()?;
    Ok(stats)
}

fn import_record(
    connection: &Connection,
    database_id: i64,
    ordinal: usize,
    fields: BTreeMap<String, Value>,
    stats: &mut LibretroImportStats,
) -> Result<()> {
    stats.rdb_records += 1;
    let ordinal = i64::try_from(ordinal)?;
    let record_id = database_id
        .checked_mul(RECORD_ID_SHIFT)
        .and_then(|prefix| prefix.checked_add(ordinal))
        .context("Libretro record identifier overflow")?;

    let title = string_field(&fields, "name")
        .or_else(|| string_field(&fields, "description"))
        .filter(|value| !value.trim().is_empty());
    let description = string_field(&fields, "description")
        .filter(|value| Some(value.as_str()) != title.as_deref());
    let file_name = string_field(&fields, "rom_name");
    let byte_size = unsigned_field(&fields, "size").and_then(|value| i64::try_from(value).ok());
    let region = string_field(&fields, "region");
    let release_date = release_date(&fields);
    let metadata_json = auxiliary_metadata_json(&fields)?;

    connection.execute(
        "INSERT INTO libretro_records
         (id, database_id, source_ordinal, title, description, file_name, byte_size,
          region, release_date, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            record_id,
            database_id,
            ordinal,
            title,
            description,
            file_name,
            byte_size,
            region,
            release_date,
            metadata_json
        ],
    )?;

    if title.is_some() {
        stats.titled_records += 1;
    } else {
        stats.records_without_title += 1;
    }

    let mut has_hash = false;
    for (algorithm, field, expected_length) in [
        ("crc32", "crc", 8),
        ("md5", "md5", 32),
        ("sha1", "sha1", 40),
    ] {
        let Some(digest) = hash_field(&fields, field) else {
            continue;
        };
        let digest = normalize_digest(algorithm, digest, expected_length)?;
        connection.execute(
            "INSERT INTO libretro_hashes (algorithm, digest, record_id)
             VALUES (?1, ?2, ?3)",
            params![algorithm, hex::decode(digest)?, record_id],
        )?;
        stats.hashes += 1;
        has_hash = true;
    }

    let serials: Vec<String> = string_fields(&fields, "serial")
        .into_iter()
        .map(normalize_identifier)
        .filter(|value| !value.is_empty())
        .collect();
    for serial in &serials {
        stats.serial_identifiers += connection.execute(
            "INSERT OR IGNORE INTO libretro_serials (identifier, record_id) VALUES (?1, ?2)",
            params![serial, record_id],
        )?;
    }
    if !has_hash && serials.is_empty() {
        stats.records_without_hash_or_serial += 1;
    }
    Ok(())
}

fn auxiliary_metadata_json(fields: &BTreeMap<String, Value>) -> Result<String> {
    let excluded = [
        "name",
        "description",
        "rom_name",
        "size",
        "crc",
        "md5",
        "sha1",
        "serial",
        "region",
        "releaseyear",
        "releasemonth",
        "releaseday",
    ];
    let mut metadata = serde_json::Map::new();
    for (key, value) in fields {
        if !excluded.contains(&key.as_str()) {
            metadata.insert(key.clone(), value_to_json(value)?);
        }
    }
    Ok(serde_json::to_string(&metadata)?)
}

fn ensure_platform(
    connection: &Connection,
    provider_id: &str,
    database_name: &str,
    manifest: &LibretroSourceManifest,
    timestamp: &str,
) -> Result<PlatformResolution> {
    let overridden_name = manifest.platform_overrides.get(database_name);
    let canonical_name = overridden_name.map_or(database_name, String::as_str);
    let normalized_name = normalize_key(canonical_name);
    if normalized_name.is_empty() {
        bail!("Libretro database has unusable platform name {database_name:?}");
    }
    let existing = connection.query_row(
        "SELECT id FROM platforms WHERE normalized_name=?1",
        [&normalized_name],
        |row| row.get::<_, String>(0),
    );
    let (id, created) = match existing {
        Ok(id) => (id, false),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let id = stable_id("platform", &normalized_name);
            connection.execute(
                "INSERT INTO platforms
                 (id, canonical_name, normalized_name, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'provisional', ?4, ?4)",
                params![id, canonical_name, normalized_name, timestamp],
            )?;
            (id, true)
        }
        Err(error) => return Err(error.into()),
    };
    connection.execute(
        "INSERT OR IGNORE INTO platform_aliases
         (platform_id, alias, normalized_alias, provider_id) VALUES (?1, ?2, ?3, ?4)",
        params![id, database_name, normalize_key(database_name), provider_id],
    )?;
    Ok(PlatformResolution { id, created })
}

fn read_rdb(path: &Path) -> Result<Vec<BTreeMap<String, Value>>> {
    let bytes = fs::read(path).with_context(|| format!("reading RDB file {}", path.display()))?;
    if bytes.len() < 18 || &bytes[..8] != RDB_MAGIC {
        bail!("invalid Libretro RDB header in {}", path.display());
    }
    let data_end = usize::try_from(u64::from_be_bytes(bytes[8..16].try_into()?))?;
    if !(16..bytes.len()).contains(&data_end) {
        bail!("invalid Libretro RDB data boundary in {}", path.display());
    }

    let record_bytes = &bytes[16..data_end];
    let mut cursor = Cursor::new(record_bytes);
    let mut records = Vec::new();
    let mut saw_sentinel = false;
    while usize::try_from(cursor.position())? < record_bytes.len() {
        let value = rmpv::decode::read_value(&mut cursor)
            .with_context(|| format!("decoding RDB record in {}", path.display()))?;
        if value.is_nil() {
            saw_sentinel = true;
            break;
        }
        records.push(value_to_fields(value)?);
    }
    if !saw_sentinel || usize::try_from(cursor.position())? != record_bytes.len() {
        bail!("RDB record stream has an invalid terminator boundary");
    }

    let mut footer = Cursor::new(&bytes[data_end..]);
    let footer_value = rmpv::decode::read_value(&mut footer)?;
    let footer_fields = value_to_fields(footer_value)?;
    let declared_count = unsigned_field(&footer_fields, "count")
        .context("Libretro RDB footer has no record count")?;
    if declared_count != records.len() as u64 {
        bail!(
            "Libretro RDB count mismatch in {}: footer={} parsed={}",
            path.display(),
            declared_count,
            records.len()
        );
    }
    Ok(records)
}

fn value_to_fields(value: Value) -> Result<BTreeMap<String, Value>> {
    let Value::Map(entries) = value else {
        bail!("Libretro RDB record is not a map");
    };
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let key = key
            .as_str()
            .context("Libretro RDB record contains a non-string field name")?
            .to_owned();
        if let Some(existing) = fields.get_mut(&key) {
            match existing {
                Value::Array(values) => values.push(value),
                _ => {
                    let first = std::mem::replace(existing, Value::Nil);
                    *existing = Value::Array(vec![first, value]);
                }
            }
        } else {
            fields.insert(key, value);
        }
    }
    Ok(fields)
}

fn value_to_json(value: &Value) -> Result<serde_json::Value> {
    Ok(match value {
        Value::Nil => serde_json::Value::Null,
        Value::Boolean(value) => (*value).into(),
        Value::Integer(value) => {
            if let Some(value) = value.as_i64() {
                value.into()
            } else {
                value
                    .as_u64()
                    .context("MessagePack integer cannot be represented as JSON")?
                    .into()
            }
        }
        Value::F32(value) => serde_json::json!(value),
        Value::F64(value) => serde_json::json!(value),
        Value::String(value) => value
            .as_str()
            .context("Libretro RDB contains invalid UTF-8 text")?
            .into(),
        Value::Binary(value) => serde_json::json!({"binary_hex": hex::encode(value)}),
        Value::Array(values) => serde_json::Value::Array(
            values
                .iter()
                .map(value_to_json)
                .collect::<Result<Vec<_>>>()?,
        ),
        Value::Map(entries) => {
            let mut object = serde_json::Map::new();
            for (key, value) in entries {
                let key = key
                    .as_str()
                    .context("Libretro RDB nested map contains a non-string key")?;
                object.insert(key.to_owned(), value_to_json(value)?);
            }
            serde_json::Value::Object(object)
        }
        Value::Ext(kind, bytes) => {
            serde_json::json!({"extension_type": kind, "binary_hex": hex::encode(bytes)})
        }
    })
}

fn string_field(fields: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    string_fields(fields, key).into_iter().next()
}

fn string_fields(fields: &BTreeMap<String, Value>, key: &str) -> Vec<String> {
    fn collect(value: &Value, output: &mut Vec<String>) {
        match value {
            Value::Array(values) => {
                for value in values {
                    collect(value, output);
                }
            }
            Value::String(value) => {
                if let Some(value) = value.as_str() {
                    output.push(value.to_owned());
                }
            }
            Value::Binary(value) => {
                if let Ok(value) = String::from_utf8(value.clone()) {
                    output.push(value);
                }
            }
            Value::Integer(value) => {
                if let Some(value) = value.as_i64() {
                    output.push(value.to_string());
                } else if let Some(value) = value.as_u64() {
                    output.push(value.to_string());
                }
            }
            _ => {}
        }
    }

    let mut values = Vec::new();
    if let Some(value) = fields.get(key) {
        collect(value, &mut values);
    }
    values
}

fn hash_field(fields: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    fn convert(value: &Value) -> Option<String> {
        match value {
            Value::Binary(value) => Some(hex::encode(value)),
            Value::String(value) => value.as_str().map(|value| {
                value
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .flat_map(char::to_lowercase)
                    .collect()
            }),
            _ => None,
        }
    }

    match fields.get(key)? {
        Value::Array(values) => values.iter().find_map(convert),
        value => convert(value),
    }
}

fn unsigned_field(fields: &BTreeMap<String, Value>, key: &str) -> Option<u64> {
    fn convert(value: &Value) -> Option<u64> {
        match value {
            Value::Integer(value) => value.as_u64(),
            Value::String(value) => value.as_str()?.parse().ok(),
            _ => None,
        }
    }
    match fields.get(key)? {
        Value::Array(values) => values.iter().find_map(convert),
        value => convert(value),
    }
}

fn normalize_digest(algorithm: &str, digest: String, expected_length: usize) -> Result<String> {
    if digest.len() > expected_length || !digest.bytes().all(|value| value.is_ascii_hexdigit()) {
        bail!("invalid {algorithm} digest {digest:?}");
    }
    Ok(format!("{digest:0>expected_length$}"))
}

fn normalize_identifier(value: String) -> String {
    value.trim().to_uppercase()
}

fn release_date(fields: &BTreeMap<String, Value>) -> Option<String> {
    let year = unsigned_field(fields, "releaseyear")?;
    let month = unsigned_field(fields, "releasemonth");
    let day = unsigned_field(fields, "releaseday");
    Some(match (month, day) {
        (Some(month @ 1..=12), Some(day @ 1..=31)) => {
            format!("{year:04}-{month:02}-{day:02}")
        }
        (Some(month @ 1..=12), _) => format!("{year:04}-{month:02}"),
        _ => format!("{year:04}"),
    })
}

fn list_rdb_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("rdb")
        {
            files.push(entry.path());
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_hashes_are_normalized() {
        let fields = BTreeMap::from([("crc".to_owned(), Value::Binary(vec![0xde, 0xad]))]);
        assert_eq!(hash_field(&fields, "crc").as_deref(), Some("dead"));
        assert_eq!(
            normalize_digest("crc32", "dead".to_owned(), 8).unwrap(),
            "0000dead"
        );
    }

    #[test]
    fn release_dates_retain_available_precision() {
        let fields = BTreeMap::from([
            ("releaseyear".to_owned(), Value::from(1998)),
            ("releasemonth".to_owned(), Value::from(11)),
            ("releaseday".to_owned(), Value::from(23)),
        ]);
        assert_eq!(release_date(&fields).as_deref(), Some("1998-11-23"));
    }
}
