use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, OptionalExtension, backup::Backup};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::write::SimpleFileOptions;

use crate::settings::SettingsStore;

const FORMAT_NAME: &str = "lunchbox-profile";
const SCHEMA_VERSION: u32 = 1;
const MANIFEST_NAME: &str = "profile.json";
const STATE_NAME: &str = "state.db";
const PENDING_DIRECTORY: &str = ".lunchbox-profile-restore.pending";
const APPLIED_NOTICE: &str = ".lunchbox-profile-restore-applied.json";
const MAX_ARCHIVE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 256 * 1024;
const MAX_ENTRIES: usize = 512;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSummary {
    pub collections: u64,
    pub installed_games: u64,
    pub customized_games: u64,
    pub import_profiles: u64,
    pub themes: u64,
    pub state_bytes: u64,
    pub created_at_unix: u64,
    pub app_version: String,
}

impl ProfileSummary {
    pub fn description(&self) -> String {
        format!(
            "{} collection{} · {} locally linked game{} · {} customized game{} · {} import profile{} · {} Couch Mode theme{} · {}",
            self.collections,
            plural(self.collections),
            self.installed_games,
            plural(self.installed_games),
            self.customized_games,
            plural(self.customized_games),
            self.import_profiles,
            plural(self.import_profiles),
            self.themes,
            plural(self.themes),
            format_bytes(self.state_bytes),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProfileManifest {
    format: String,
    schema_version: u32,
    created_at_unix: u64,
    app_version: String,
    summary: ProfileSummary,
    state: ProfileFile,
    themes: Vec<ProfileTheme>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProfileTheme {
    id: String,
    files: Vec<ProfileFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProfileFile {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AppliedNotice {
    applied_at_unix: u64,
    summary: ProfileSummary,
}

pub fn export_profile(state_path: &Path, destination: &Path) -> Result<ProfileSummary> {
    validate_destination(destination)?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .context("profile backup destination has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating backup directory {}", parent.display()))?;
    protect_managed_profile_paths(state_path, destination)?;

    let operation = Uuid::new_v4().simple().to_string();
    let snapshot = parent.join(format!(".lunchbox-profile-state-{operation}.db"));
    let temporary = parent.join(format!(".lunchbox-profile-archive-{operation}.tmp"));
    let result = (|| {
        create_database_snapshot(state_path, &snapshot)?;
        validate_database_read_only(&snapshot)?;

        let state = file_receipt(STATE_NAME, &snapshot, MAX_UNCOMPRESSED_BYTES)?;
        let themes = theme_receipts(state_path)?;
        let counts = profile_counts(&snapshot)?;
        let created_at_unix = unix_timestamp()?;
        let summary = ProfileSummary {
            collections: counts.0,
            installed_games: counts.1,
            customized_games: counts.2,
            import_profiles: counts.3,
            themes: u64::try_from(themes.len()).unwrap_or(u64::MAX),
            state_bytes: state.bytes,
            created_at_unix,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
        };
        let manifest = ProfileManifest {
            format: FORMAT_NAME.to_owned(),
            schema_version: SCHEMA_VERSION,
            created_at_unix,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            summary: summary.clone(),
            state,
            themes,
        };
        validate_manifest(&manifest)?;
        write_archive(&temporary, &snapshot, state_path, &manifest)?;
        inspect_profile(&temporary).context("verifying the completed profile archive")?;
        replace_file_atomically(&temporary, destination)?;
        Ok(summary)
    })();
    let _ = fs::remove_file(&snapshot);
    let _ = fs::remove_file(sidecar_path(&snapshot, "-wal"));
    let _ = fs::remove_file(sidecar_path(&snapshot, "-shm"));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn inspect_profile(source: &Path) -> Result<ProfileSummary> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("inspecting profile archive {}", source.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("profile archive must be a regular file");
    }
    if metadata.len() == 0 || metadata.len() > MAX_ARCHIVE_BYTES {
        bail!("profile archive must be non-empty and no larger than 2 GiB");
    }
    let file = File::open(source)
        .with_context(|| format!("opening profile archive {}", source.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("opening profile archive as ZIP")?;
    let (manifest, expected) = archive_manifest(&mut archive)?;
    verify_archive_entries(&mut archive, &expected)?;
    let inspection_parent = std::env::temp_dir();
    let inspection = inspection_parent.join(format!(
        "lunchbox-profile-inspect-{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir(&inspection).with_context(|| {
        format!(
            "creating profile inspection directory {}",
            inspection.display()
        )
    })?;
    let result = (|| {
        extract_verified_entries(&mut archive, &expected, &inspection)?;
        write_json_atomically(&inspection.join(MANIFEST_NAME), &manifest)?;
        validate_staged_restore(&inspection, &manifest)?;
        Ok(manifest.summary.clone())
    })();
    if let Err(error) = remove_exact_directory(&inspection_parent, &inspection)
        && result.is_ok()
    {
        return Err(error).context("cleaning the validated profile inspection");
    }
    result
}

pub fn stage_restore(state_path: &Path, source: &Path) -> Result<ProfileSummary> {
    let parent = state_parent(state_path)?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating state directory {}", parent.display()))?;
    let operation = Uuid::new_v4().simple().to_string();
    let staging = parent.join(format!(".lunchbox-profile-restore.staging-{operation}"));
    fs::create_dir(&staging)
        .with_context(|| format!("creating restore staging directory {}", staging.display()))?;

    let result = (|| {
        let metadata = fs::symlink_metadata(source)
            .with_context(|| format!("inspecting profile archive {}", source.display()))?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() == 0
            || metadata.len() > MAX_ARCHIVE_BYTES
        {
            bail!("profile archive must be a regular, non-empty file no larger than 2 GiB");
        }
        let file = File::open(source)
            .with_context(|| format!("opening profile archive {}", source.display()))?;
        let mut archive = zip::ZipArchive::new(file).context("opening profile archive as ZIP")?;
        let (manifest, expected) = archive_manifest(&mut archive)?;
        extract_verified_entries(&mut archive, &expected, &staging)?;
        fs::create_dir_all(staging.join("themes"))
            .context("creating the restored Couch Mode theme store")?;
        write_json_atomically(&staging.join(MANIFEST_NAME), &manifest)?;
        validate_staged_restore(&staging, &manifest)?;
        let pending = parent.join(PENDING_DIRECTORY);
        if metadata_if_exists(&pending)?.is_some() {
            validate_pending_directory(&pending)?;
        }
        replace_directory_atomically(&staging, &pending)?;
        Ok(manifest.summary)
    })();
    if result.is_err() && staging.exists() {
        let _ = remove_exact_directory(parent, &staging);
    }
    result
}

pub fn apply_pending_restore(state_path: &Path) -> Result<Option<ProfileSummary>> {
    let parent = state_parent(state_path)?;
    let pending = parent.join(PENDING_DIRECTORY);
    let Some(pending_metadata) = metadata_if_exists(&pending)? else {
        return Ok(None);
    };
    if !pending_metadata.is_dir() || pending_metadata.file_type().is_symlink() {
        bail!("pending profile restore is not an owned directory");
    }
    let manifest: ProfileManifest = read_json_bounded(&pending.join(MANIFEST_NAME))?;
    validate_staged_restore(&pending, &manifest)?;

    let operation = Uuid::new_v4().simple().to_string();
    let old_state = parent.join(format!(".lunchbox-profile-old-state-{operation}.db"));
    let themes = parent.join("themes");
    let old_themes = parent.join(format!(".lunchbox-profile-old-themes-{operation}"));
    let consumed_pending = parent.join(format!(".lunchbox-profile-applied-{operation}"));
    let pending_state = pending.join(STATE_NAME);
    let pending_themes = pending.join("themes");
    let current_state_metadata = metadata_if_exists(state_path)?;
    let current_theme_metadata = metadata_if_exists(&themes)?;
    let had_state = current_state_metadata.is_some();
    let had_themes = current_theme_metadata.is_some();

    if had_state {
        let metadata = current_state_metadata.as_ref().expect("checked above");
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            bail!("current state database is not a regular file");
        }
        checkpoint_current_state(state_path)?;
        fs::rename(state_path, &old_state).context("preserving current state before restore")?;
    }
    if had_themes {
        let metadata = current_theme_metadata.as_ref().expect("checked above");
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            if had_state {
                let _ = fs::rename(&old_state, state_path);
            }
            bail!("current Couch Mode theme store is not an owned directory");
        }
        if let Err(error) = fs::rename(&themes, &old_themes) {
            if had_state {
                let _ = fs::rename(&old_state, state_path);
            }
            return Err(error).context("preserving current themes before restore");
        }
    }

    let apply_result: Result<()> = (|| {
        fs::rename(&pending_state, state_path).context("activating restored state database")?;
        fs::rename(&pending_themes, &themes).context("activating restored Couch Mode themes")?;
        let store = SettingsStore::at(state_path).context("migrating restored state database")?;
        store
            .load()
            .context("loading restored application settings")?;
        validate_database_read_only(state_path)?;
        validate_theme_store(state_path, &manifest)?;
        fs::rename(&pending, &consumed_pending)
            .context("retiring the consumed pending profile restore")?;
        Ok(())
    })();

    if let Err(error) = apply_result {
        let _ = fs::remove_file(state_path);
        let _ = remove_sqlite_sidecars(state_path);
        if themes.exists() {
            let _ = remove_exact_directory(parent, &themes);
        }
        if had_state {
            let _ = fs::rename(&old_state, state_path);
        }
        if had_themes {
            let _ = fs::rename(&old_themes, &themes);
        }
        return Err(error).context("profile activation failed; the previous profile was restored");
    }

    if had_state && let Err(error) = fs::remove_file(&old_state) {
        eprintln!(
            "LUNCHBOX_PROFILE_RESTORE_CLEANUP_WARNING path={:?} error={error}",
            old_state
        );
    }
    if had_themes && let Err(error) = remove_exact_directory(parent, &old_themes) {
        eprintln!(
            "LUNCHBOX_PROFILE_RESTORE_CLEANUP_WARNING path={:?} error={error:#}",
            old_themes
        );
    }
    if let Err(error) = remove_exact_directory(parent, &consumed_pending) {
        eprintln!(
            "LUNCHBOX_PROFILE_RESTORE_CLEANUP_WARNING path={consumed_pending:?} error={error:#}"
        );
    }
    let notice = AppliedNotice {
        applied_at_unix: unix_timestamp()?,
        summary: manifest.summary.clone(),
    };
    if let Err(error) = write_json_atomically(&parent.join(APPLIED_NOTICE), &notice) {
        eprintln!("LUNCHBOX_PROFILE_RESTORE_NOTICE_WARNING error={error:#}");
    }
    Ok(Some(manifest.summary))
}

pub fn cancel_pending_restore(state_path: &Path) -> Result<bool> {
    let parent = state_parent(state_path)?;
    let pending = parent.join(PENDING_DIRECTORY);
    let metadata = match fs::symlink_metadata(&pending) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("inspecting pending restore {}", pending.display()));
        }
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("pending profile restore is not an owned directory");
    }
    validate_pending_directory(&pending)?;
    remove_exact_directory(parent, &pending)?;
    Ok(true)
}

pub fn take_applied_notice(state_path: &Path) -> Result<Option<ProfileSummary>> {
    let parent = state_parent(state_path)?;
    let path = parent.join(APPLIED_NOTICE);
    if metadata_if_exists(&path)?.is_none() {
        return Ok(None);
    }
    let notice: AppliedNotice = read_json_bounded(&path)?;
    if notice.applied_at_unix > unix_timestamp()?.saturating_add(300) {
        bail!("profile restore notice has an invalid timestamp");
    }
    fs::remove_file(&path).context("removing consumed profile restore notice")?;
    Ok(Some(notice.summary))
}

fn create_database_snapshot(source_path: &Path, destination: &Path) -> Result<()> {
    let store = SettingsStore::at(source_path)?;
    let source = store.connection()?;
    source
        .execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
        .context("checkpointing live state before profile backup")?;
    let mut destination_connection = Connection::open(destination)
        .with_context(|| format!("creating state snapshot {}", destination.display()))?;
    let backup = Backup::new(&source, &mut destination_connection)
        .context("starting online state backup")?;
    backup
        .run_to_completion(128, Duration::from_millis(2), None)
        .context("copying consistent state snapshot")?;
    drop(backup);
    destination_connection
        .execute_batch("PRAGMA journal_mode=DELETE; PRAGMA optimize;")
        .context("finalizing state snapshot")?;
    Ok(())
}

fn write_archive(
    destination: &Path,
    snapshot: &Path,
    state_path: &Path,
    manifest: &ProfileManifest,
) -> Result<()> {
    let output = File::create(destination)
        .with_context(|| format!("creating profile archive {}", destination.display()))?;
    let mut writer = zip::ZipWriter::new(output);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(9));
    let manifest_bytes = serde_json::to_vec_pretty(manifest)?;
    writer.start_file(MANIFEST_NAME, options)?;
    writer.write_all(&manifest_bytes)?;
    writer.start_file(STATE_NAME, options)?;
    copy_file_into(&mut writer, snapshot)?;

    let state_parent = state_parent(state_path)?;
    for theme in &manifest.themes {
        for receipt in &theme.files {
            writer.start_file(&receipt.path, options)?;
            copy_file_into(&mut writer, &state_parent.join(&receipt.path))?;
        }
    }
    let output = writer.finish().context("finishing profile ZIP archive")?;
    output
        .sync_all()
        .context("syncing completed profile archive")?;
    Ok(())
}

fn archive_manifest<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<(ProfileManifest, BTreeMap<String, ProfileFile>)> {
    if archive.is_empty() || archive.len() > MAX_ENTRIES {
        bail!("profile archive must contain between 1 and {MAX_ENTRIES} entries");
    }
    let manifest_bytes = {
        let mut entry = archive
            .by_name(MANIFEST_NAME)
            .context("profile archive is missing profile.json")?;
        if entry.is_dir() || entry.size() == 0 || entry.size() > MAX_MANIFEST_BYTES as u64 {
            bail!("profile.json must be a non-empty file no larger than 256 KiB");
        }
        read_zip_entry(&mut entry, MAX_MANIFEST_BYTES as u64)?
    };
    let manifest: ProfileManifest =
        serde_json::from_slice(&manifest_bytes).context("decoding profile.json")?;
    let expected = validate_manifest(&manifest)?;
    Ok((manifest, expected))
}

fn validate_manifest(manifest: &ProfileManifest) -> Result<BTreeMap<String, ProfileFile>> {
    if manifest.format != FORMAT_NAME || manifest.schema_version != SCHEMA_VERSION {
        bail!(
            "unsupported profile format or schema {}; expected {}",
            manifest.schema_version,
            SCHEMA_VERSION
        );
    }
    if manifest.app_version.trim().is_empty()
        || manifest.app_version.len() > 64
        || manifest.summary.app_version != manifest.app_version
        || manifest.summary.created_at_unix != manifest.created_at_unix
    {
        bail!("profile manifest has inconsistent application metadata");
    }
    if manifest.created_at_unix > unix_timestamp()?.saturating_add(300) {
        bail!("profile creation time is unreasonably far in the future");
    }
    if manifest.state.path != STATE_NAME || manifest.state.bytes == 0 {
        bail!("profile manifest must declare one non-empty state.db");
    }
    validate_receipt(&manifest.state)?;
    if manifest.summary.state_bytes != manifest.state.bytes
        || manifest.summary.themes != u64::try_from(manifest.themes.len()).unwrap_or(u64::MAX)
    {
        bail!("profile summary does not match its declared contents");
    }

    let mut expected = BTreeMap::new();
    let mut portable_paths = BTreeSet::new();
    portable_paths.insert(manifest.state.path.to_lowercase());
    expected.insert(manifest.state.path.clone(), manifest.state.clone());
    let mut theme_ids = BTreeSet::new();
    let mut total = manifest.state.bytes;
    for theme in &manifest.themes {
        crate::couch_theme::validate_theme_id(&theme.id)?;
        if !theme_ids.insert(theme.id.clone()) || theme.files.is_empty() {
            bail!("profile contains a duplicate or empty Couch Mode theme");
        }
        let prefix = format!("themes/{}/", theme.id);
        for file in &theme.files {
            validate_receipt(file)?;
            if !file.path.starts_with(&prefix) || !safe_relative_path(&file.path) {
                bail!("profile theme contains an unsafe or mismatched path");
            }
            if !portable_paths.insert(file.path.to_lowercase()) {
                bail!("profile manifest contains case-ambiguous paths");
            }
            total = total
                .checked_add(file.bytes)
                .context("profile uncompressed size overflow")?;
            if total > MAX_UNCOMPRESSED_BYTES {
                bail!("profile expands beyond the 4 GiB safety limit");
            }
            if expected.insert(file.path.clone(), file.clone()).is_some() {
                bail!("profile manifest contains duplicate paths");
            }
        }
    }
    Ok(expected)
}

fn verify_archive_entries<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    expected: &BTreeMap<String, ProfileFile>,
) -> Result<()> {
    if archive.len() != expected.len().saturating_add(1) {
        bail!("profile archive contains undeclared or missing entries");
    }
    let mut seen = BTreeSet::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        if name == MANIFEST_NAME {
            if !seen.insert(name) {
                bail!("profile archive contains duplicate profile.json entries");
            }
            continue;
        }
        let receipt = expected
            .get(&name)
            .with_context(|| format!("profile archive contains undeclared entry {name}"))?;
        verify_zip_entry(&mut entry, receipt)?;
        if !seen.insert(name) {
            bail!("profile archive contains duplicate or case-ambiguous entries");
        }
    }
    if expected.keys().any(|name| !seen.contains(name)) {
        bail!("profile archive is missing a declared entry");
    }
    Ok(())
}

fn extract_verified_entries<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    expected: &BTreeMap<String, ProfileFile>,
    staging: &Path,
) -> Result<()> {
    verify_archive_entries(archive, expected)?;
    for (name, receipt) in expected {
        let mut entry = archive
            .by_name(name)
            .with_context(|| format!("opening declared profile entry {name}"))?;
        let destination = staging.join(Path::new(name));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating restore directory {}", parent.display()))?;
        }
        let mut output = File::create(&destination)
            .with_context(|| format!("creating restored file {}", destination.display()))?;
        let copied = std::io::copy(
            &mut entry.by_ref().take(receipt.bytes.saturating_add(1)),
            &mut output,
        )?;
        if copied != receipt.bytes {
            bail!("profile entry {name} changed while extracting");
        }
        output.sync_all()?;
    }
    Ok(())
}

fn validate_staged_restore(root: &Path, manifest: &ProfileManifest) -> Result<()> {
    let expected = validate_manifest(manifest)?;
    validate_staged_inventory(root, &expected)?;
    for receipt in expected.values() {
        let path = root.join(&receipt.path);
        let actual = file_receipt(&receipt.path, &path, MAX_UNCOMPRESSED_BYTES)?;
        if actual.bytes != receipt.bytes || actual.sha256 != receipt.sha256 {
            bail!(
                "staged profile file {} does not match its receipt",
                receipt.path
            );
        }
    }
    let state = root.join(STATE_NAME);
    validate_database_read_only(&state)?;
    let counts = profile_counts(&state)?;
    if counts
        != (
            manifest.summary.collections,
            manifest.summary.installed_games,
            manifest.summary.customized_games,
            manifest.summary.import_profiles,
        )
    {
        bail!("profile summary does not match the restored database contents");
    }
    validate_database_migration(&state, root)?;
    validate_theme_store(&state, manifest)?;
    Ok(())
}

fn validate_pending_directory(pending: &Path) -> Result<()> {
    let manifest: ProfileManifest = read_json_bounded(&pending.join(MANIFEST_NAME))?;
    validate_staged_restore(pending, &manifest)
        .context("pending profile restore is not a complete owned staging directory")
}

fn validate_staged_inventory(root: &Path, expected: &BTreeMap<String, ProfileFile>) -> Result<()> {
    let mut expected_files = expected.keys().cloned().collect::<BTreeSet<_>>();
    expected_files.insert(MANIFEST_NAME.to_owned());
    let mut missing_files = expected_files.clone();
    let mut allowed_directories = BTreeSet::from(["themes".to_owned()]);
    for name in &expected_files {
        let mut parent = Path::new(name).parent();
        while let Some(directory) = parent.filter(|path| !path.as_os_str().is_empty()) {
            allowed_directories.insert(path_to_archive_name(directory)?);
            parent = directory.parent();
        }
    }

    let mut directories = vec![root.to_owned()];
    while let Some(directory) = directories.pop() {
        let entries = fs::read_dir(&directory)
            .with_context(|| format!("inspecting staged directory {}", directory.display()))?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                bail!("staged profile contains a symbolic link");
            }
            let relative = path
                .strip_prefix(root)
                .context("staged profile entry escaped its root")?;
            let name = path_to_archive_name(relative)?;
            if metadata.is_dir() {
                if !allowed_directories.contains(&name) {
                    bail!("staged profile contains undeclared directory {name}");
                }
                directories.push(path);
            } else if metadata.is_file() {
                if !expected_files.contains(&name) {
                    bail!("staged profile contains undeclared file {name}");
                }
                missing_files.remove(&name);
            } else {
                bail!("staged profile contains an unsupported filesystem entry");
            }
        }
    }
    if !missing_files.is_empty() {
        bail!("staged profile is missing one or more declared files");
    }
    Ok(())
}

fn validate_database_read_only(path: &Path) -> Result<()> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("opening restored state {} read-only", path.display()))?;
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        bail!("restored state database failed SQLite integrity check: {integrity}");
    }
    let foreign_key_failures: i64 =
        connection.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
    if foreign_key_failures != 0 {
        bail!("restored state database has {foreign_key_failures} foreign-key violations");
    }
    connection
        .query_row("SELECT id FROM app_settings WHERE id=1", [], |row| {
            row.get::<_, i64>(0)
        })
        .context("restored state database is not a Lunchbox profile")?;
    let secret_column: Option<(String, String)> = connection
        .query_row(
            "SELECT m.name, p.name
             FROM sqlite_schema AS m
             JOIN pragma_table_info(m.name) AS p
             WHERE m.type='table'
               AND (
                   lower(p.name) LIKE '%password%'
                   OR lower(p.name) LIKE '%secret%'
                   OR lower(p.name) LIKE '%api_key%'
                   OR lower(p.name) IN ('access_token', 'refresh_token')
               )
             LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((table, column)) = secret_column {
        bail!(
            "profile state contains forbidden credential column {table}.{column}; secrets must remain in the operating-system credential store"
        );
    }
    Ok(())
}

fn validate_database_migration(path: &Path, root: &Path) -> Result<()> {
    let copy = root.join(format!(".migration-check-{}.db", Uuid::new_v4().simple()));
    fs::copy(path, &copy).context("copying restored state for migration validation")?;
    let result = (|| {
        let store = SettingsStore::at(&copy)?;
        store.load()?;
        validate_database_read_only(&copy)
    })();
    let cleanup = remove_migration_copy(&copy);
    result.context("profile state cannot be migrated by this Lunchbox build")?;
    cleanup.context("cleaning the temporary profile migration check")
}

fn remove_migration_copy(path: &Path) -> Result<()> {
    for candidate in [
        path.to_owned(),
        sidecar_path(path, "-wal"),
        sidecar_path(path, "-shm"),
    ] {
        match fs::remove_file(&candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("removing migration file {}", candidate.display()));
            }
        }
    }
    Ok(())
}

fn validate_theme_store(state_path: &Path, manifest: &ProfileManifest) -> Result<()> {
    let catalog = crate::couch_theme::catalog_for_state(state_path);
    if let Some(warning) = catalog.warnings.first() {
        bail!("restored Couch Mode theme validation failed: {warning}");
    }
    let expected = manifest
        .themes
        .iter()
        .map(|theme| theme.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual = catalog
        .themes
        .iter()
        .filter(|theme| theme.installed)
        .map(|theme| theme.id.as_str())
        .collect::<BTreeSet<_>>();
    if actual != expected {
        bail!("restored Couch Mode theme inventory does not match the profile manifest");
    }
    let connection = Connection::open(state_path)?;
    let selected: String = connection.query_row(
        "SELECT COALESCE(
             (SELECT theme_id FROM couch_mode_preferences WHERE id=1),
             'lunchbox-default'
         )",
        [],
        |row| row.get(0),
    )?;
    if !catalog.themes.iter().any(|theme| theme.id == selected) {
        bail!("profile selects a Couch Mode theme that is not included in the backup");
    }
    Ok(())
}

fn theme_receipts(state_path: &Path) -> Result<Vec<ProfileTheme>> {
    let catalog = crate::couch_theme::catalog_for_state(state_path);
    if let Some(warning) = catalog.warnings.first() {
        bail!("cannot create a complete profile while an installed theme is invalid: {warning}");
    }
    let state_parent = state_parent(state_path)?;
    let mut themes = Vec::new();
    for theme in catalog.themes.iter().filter(|theme| theme.installed) {
        let root = state_parent.join("themes").join(&theme.id);
        let mut paths = vec![
            PathBuf::from("theme.json"),
            PathBuf::from(".lunchbox-theme-owned.json"),
        ];
        if let Some(background) = theme.background_path.as_deref() {
            paths.push(
                background
                    .strip_prefix(&root)
                    .context("installed theme background escaped its theme directory")?
                    .to_owned(),
            );
        }
        paths.sort();
        paths.dedup();
        let mut files = Vec::new();
        for relative in paths {
            let archive_path = Path::new("themes").join(&theme.id).join(&relative);
            let archive_name = path_to_archive_name(&archive_path)?;
            files.push(file_receipt(
                &archive_name,
                &root.join(relative),
                MAX_UNCOMPRESSED_BYTES,
            )?);
        }
        themes.push(ProfileTheme {
            id: theme.id.clone(),
            files,
        });
    }
    themes.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(themes)
}

fn profile_counts(path: &Path) -> Result<(u64, u64, u64, u64)> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok((
        query_count(&connection, "SELECT count(*) FROM user_collections")?,
        query_count(
            &connection,
            "SELECT count(DISTINCT game_uid) FROM installed_game_files",
        )?,
        query_count(&connection, "SELECT count(*) FROM game_metadata_overrides")?,
        query_count(&connection, "SELECT count(*) FROM rom_import_profiles")?,
    ))
}

fn query_count(connection: &Connection, sql: &str) -> Result<u64> {
    let value: i64 = connection.query_row(sql, [], |row| row.get(0))?;
    u64::try_from(value).context("profile database contains a negative record count")
}

fn file_receipt(name: &str, path: &Path, maximum: u64) -> Result<ProfileFile> {
    if !safe_relative_path(name) {
        bail!("profile entry path is unsafe: {name}");
    }
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting profile file {}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("profile file must be a regular file: {}", path.display());
    }
    if metadata.len() == 0 || metadata.len() > maximum {
        bail!("profile file has an invalid size: {}", path.display());
    }
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let copied = std::io::copy(&mut file, &mut digest)?;
    if copied != metadata.len() {
        bail!("profile file changed while hashing: {}", path.display());
    }
    Ok(ProfileFile {
        path: name.to_owned(),
        bytes: metadata.len(),
        sha256: hex::encode(digest.finalize()),
    })
}

fn verify_zip_entry<R: Read>(
    entry: &mut zip::read::ZipFile<'_, R>,
    receipt: &ProfileFile,
) -> Result<()> {
    if entry.is_dir()
        || entry.size() != receipt.bytes
        || entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
    {
        bail!("profile entry {} has invalid metadata", receipt.path);
    }
    let mut digest = Sha256::new();
    let copied = std::io::copy(
        &mut entry.by_ref().take(receipt.bytes.saturating_add(1)),
        &mut digest,
    )?;
    let sha256 = hex::encode(digest.finalize());
    if copied != receipt.bytes || sha256 != receipt.sha256 {
        bail!("profile entry {} failed its SHA-256 receipt", receipt.path);
    }
    Ok(())
}

fn validate_receipt(receipt: &ProfileFile) -> Result<()> {
    if receipt.bytes == 0
        || receipt.bytes > MAX_UNCOMPRESSED_BYTES
        || receipt.sha256.len() != 64
        || !receipt.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !safe_relative_path(&receipt.path)
    {
        bail!("profile manifest contains an invalid file receipt");
    }
    Ok(())
}

fn read_zip_entry<R: Read>(entry: &mut R, maximum: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    entry
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        bail!("profile archive entry exceeds its safety limit");
    }
    Ok(bytes)
}

fn read_json_bounded<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("inspecting {}", path.display()))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > MAX_MANIFEST_BYTES as u64
    {
        bail!("profile metadata must be a small regular file");
    }
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("decoding {}", path.display()))
}

fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path.parent().context("JSON destination has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".profile-json-{}.tmp", Uuid::new_v4().simple()));
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(value)?;
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        replace_file_atomically(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn replace_file_atomically(temporary: &Path, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .context("profile destination has no parent")?;
    if temporary.parent() != Some(parent) || temporary == target {
        bail!("profile publication paths do not share the exact destination directory");
    }
    let backup = parent.join(format!(".lunchbox-profile-old-{}", Uuid::new_v4().simple()));
    let target_metadata = metadata_if_exists(target)?;
    let had_target = target_metadata.is_some();
    if let Some(metadata) = target_metadata {
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            bail!("profile destination is not a regular file");
        }
        fs::rename(target, &backup).context("preserving existing profile archive")?;
    }
    if let Err(error) = fs::rename(temporary, target) {
        if had_target {
            let _ = fs::rename(&backup, target);
        }
        return Err(error).context("publishing profile archive");
    }
    if had_target && let Err(error) = fs::remove_file(&backup) {
        eprintln!("LUNCHBOX_PROFILE_BACKUP_CLEANUP_WARNING path={backup:?} error={error}");
    }
    Ok(())
}

fn replace_directory_atomically(temporary: &Path, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .context("restore destination has no parent")?;
    if temporary.parent() != Some(parent) || temporary == target {
        bail!("restore publication paths do not share the exact state directory");
    }
    let backup = parent.join(format!(
        ".lunchbox-profile-old-pending-{}",
        Uuid::new_v4().simple()
    ));
    let target_metadata = metadata_if_exists(target)?;
    let had_target = target_metadata.is_some();
    if let Some(metadata) = target_metadata {
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            bail!("pending restore destination is not an owned directory");
        }
        fs::rename(target, &backup).context("preserving previous pending restore")?;
    }
    if let Err(error) = fs::rename(temporary, target) {
        if had_target {
            let _ = fs::rename(&backup, target);
        }
        return Err(error).context("publishing pending profile restore");
    }
    if had_target && let Err(error) = remove_exact_directory(parent, &backup) {
        eprintln!("LUNCHBOX_PROFILE_RESTORE_CLEANUP_WARNING path={backup:?} error={error:#}");
    }
    Ok(())
}

fn remove_exact_directory(parent: &Path, target: &Path) -> Result<()> {
    if target.parent() != Some(parent) || target == parent {
        bail!("refusing to remove a directory outside the exact profile state root");
    }
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("inspecting directory {}", target.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("refusing to remove a non-directory profile path");
    }
    fs::remove_dir_all(target)
        .with_context(|| format!("removing profile directory {}", target.display()))
}

fn remove_sqlite_sidecars(path: &Path) -> Result<()> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = sidecar_path(path, suffix);
        match fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("removing SQLite sidecar {}", sidecar.display()));
            }
        }
    }
    Ok(())
}

fn checkpoint_current_state(path: &Path) -> Result<()> {
    let connection = Connection::open(path)
        .with_context(|| format!("opening current state {} before restore", path.display()))?;
    connection.busy_timeout(Duration::from_secs(2))?;
    let (busy, _remaining, _checkpointed): (i64, i64, i64) =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
    if busy != 0 {
        bail!("current state database is still busy; close other Lunchbox processes and retry");
    }
    drop(connection);
    remove_sqlite_sidecars(path)
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && !value.contains(':')
        && !value.contains('\0')
        && !Path::new(value).is_absolute()
        && value.split('/').all(portable_path_component)
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn portable_path_component(component: &str) -> bool {
    if component.is_empty()
        || component == "."
        || component == ".."
        || component.ends_with(['.', ' '])
    {
        return false;
    }
    let stem = component
        .split_once('.')
        .map_or(component, |(stem, _extension)| stem)
        .to_ascii_uppercase();
    !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

fn path_to_archive_name(path: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(value) = component else {
            bail!("profile path is not a safe relative path");
        };
        parts.push(
            value
                .to_str()
                .context("profile-managed theme paths must be valid Unicode")?,
        );
    }
    let value = parts.join("/");
    if !safe_relative_path(&value) {
        bail!("profile path is unsafe");
    }
    Ok(value)
}

fn copy_file_into<W: Write>(writer: &mut W, path: &Path) -> Result<()> {
    let mut file =
        File::open(path).with_context(|| format!("opening profile source {}", path.display()))?;
    std::io::copy(&mut file, writer)
        .with_context(|| format!("writing profile source {}", path.display()))?;
    Ok(())
}

fn validate_destination(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.file_name().is_none() {
        bail!("choose a profile backup file");
    }
    if let Some(metadata) = metadata_if_exists(path)?
        && (!metadata.is_file() || metadata.file_type().is_symlink())
    {
        bail!("profile backup destination must be a regular file");
    }
    Ok(())
}

fn protect_managed_profile_paths(state_path: &Path, destination: &Path) -> Result<()> {
    let state = resolve_existing_or_parent(state_path)?;
    let destination = resolve_existing_or_parent(destination)?;
    if destination == state
        || destination == resolve_existing_or_parent(&sidecar_path(state_path, "-wal"))?
        || destination == resolve_existing_or_parent(&sidecar_path(state_path, "-shm"))?
    {
        bail!("profile backup destination cannot replace the live state database");
    }
    let theme_root = state_parent(state_path)?.join("themes");
    let theme_root = resolve_existing_or_parent(&theme_root)?;
    if destination.starts_with(&theme_root) {
        bail!("profile backup destination cannot overwrite the managed Couch Mode theme store");
    }
    Ok(())
}

fn resolve_existing_or_parent(path: &Path) -> Result<PathBuf> {
    if metadata_if_exists(path)?.is_some() {
        return fs::canonicalize(path)
            .with_context(|| format!("resolving profile path {}", path.display()));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .context("profile path has no parent directory")?;
    let parent = fs::canonicalize(parent)
        .with_context(|| format!("resolving profile directory {}", parent.display()))?;
    Ok(parent.join(
        path.file_name()
            .context("profile path has no final component")?,
    ))
}

fn metadata_if_exists(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("inspecting profile path {}", path.display()))
        }
    }
}

fn state_parent(state_path: &Path) -> Result<&Path> {
    state_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .context("state database path has no parent directory")
}

fn unix_timestamp() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")
        .map(|duration| duration.as_secs())
}

fn plural(count: u64) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AppSettings;
    use tempfile::TempDir;

    fn install_theme_fixture(directory: &TempDir, state: &Path) {
        let package = directory.path().join("portable-theme.lunchbox-theme");
        let manifest = serde_json::json!({
            "schema_version": 1,
            "id": "portable-profile-theme",
            "name": "Portable Profile",
            "author": "Lunchbox Tests",
            "description": "A deterministic profile backup fixture.",
            "palette": {
                "background": "#101820",
                "panel": "#182430",
                "panel_raised": "#203040",
                "ink": "#f0f4f8",
                "muted": "#9aa8b8",
                "accent": "#ffb454",
                "accent_cool": "#62d6c6",
                "danger": "#ff7d72"
            },
            "hero_scrim_percent": 62,
            "card_radius": 16
        });
        let mut writer = zip::ZipWriter::new(File::create(&package).unwrap());
        writer
            .start_file("theme.json", SimpleFileOptions::default())
            .unwrap();
        writer
            .write_all(&serde_json::to_vec_pretty(&manifest).unwrap())
            .unwrap();
        writer.finish().unwrap();
        crate::couch_theme::install_package(&package, state).unwrap();

        let store = SettingsStore::at(state).unwrap();
        let mut preferences = store.load_couch_mode_preferences().unwrap();
        preferences.theme_id = "portable-profile-theme".to_owned();
        store.save_couch_mode_preferences(&preferences).unwrap();
    }

    fn state_fixture(directory: &TempDir) -> PathBuf {
        let path = directory.path().join("live/state.db");
        let store = SettingsStore::at(&path).unwrap();
        let mut settings = AppSettings::default();
        settings.qbittorrent_host = "backup.example.test".to_owned();
        settings.rom_directory = directory.path().join("portable roms");
        store.save(&settings).unwrap();
        let connection = store.connection().unwrap();
        connection
            .execute(
                "INSERT INTO user_collections (
                     id, name, description, kind, rules_json, created_at, updated_at
                 ) VALUES (
                     'backup-collection', 'Favorites to preserve',
                     'A portable profile fixture', 'manual', '', 1, 1
                 )",
                [],
            )
            .unwrap();
        path
    }

    #[test]
    fn profile_round_trip_is_staged_and_restores_exact_state() {
        let directory = TempDir::new().unwrap();
        let state = state_fixture(&directory);
        let archive = directory.path().join("portable.lunchbox-profile");
        let exported = export_profile(&state, &archive).unwrap();
        assert_eq!(exported.collections, 1);
        assert_eq!(inspect_profile(&archive).unwrap(), exported);

        let store = SettingsStore::at(&state).unwrap();
        let mut changed = store.load().unwrap();
        changed.qbittorrent_host = "changed.example.test".to_owned();
        store.save(&changed).unwrap();

        let staged = stage_restore(&state, &archive).unwrap();
        assert_eq!(staged, exported);
        assert_eq!(
            SettingsStore::at(&state)
                .unwrap()
                .load()
                .unwrap()
                .qbittorrent_host,
            "changed.example.test"
        );

        let applied = apply_pending_restore(&state).unwrap().unwrap();
        assert_eq!(applied, exported);
        assert_eq!(
            SettingsStore::at(&state)
                .unwrap()
                .load()
                .unwrap()
                .qbittorrent_host,
            "backup.example.test"
        );
        assert_eq!(take_applied_notice(&state).unwrap(), Some(exported));
        assert_eq!(take_applied_notice(&state).unwrap(), None);
    }

    #[test]
    fn profile_rejects_a_tampered_state_receipt() {
        let directory = TempDir::new().unwrap();
        let state = state_fixture(&directory);
        let archive = directory.path().join("portable.lunchbox-profile");
        export_profile(&state, &archive).unwrap();

        let file = File::open(&archive).unwrap();
        let mut input = zip::ZipArchive::new(file).unwrap();
        let manifest = {
            let mut entry = input.by_name(MANIFEST_NAME).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            bytes
        };
        let state_bytes = {
            let mut entry = input.by_name(STATE_NAME).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            bytes[0] ^= 0xff;
            bytes
        };
        let tampered = directory.path().join("tampered.lunchbox-profile");
        let mut writer = zip::ZipWriter::new(File::create(&tampered).unwrap());
        writer
            .start_file(MANIFEST_NAME, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&manifest).unwrap();
        writer
            .start_file(STATE_NAME, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&state_bytes).unwrap();
        writer.finish().unwrap();

        let error = inspect_profile(&tampered).unwrap_err().to_string();
        assert!(error.contains("SHA-256"), "{error}");
        assert!(stage_restore(&state, &tampered).is_err());
        assert!(!state.parent().unwrap().join(PENDING_DIRECTORY).exists());
    }

    #[test]
    fn profile_restores_verified_installed_themes_and_selection() {
        let directory = TempDir::new().unwrap();
        let state = state_fixture(&directory);
        install_theme_fixture(&directory, &state);
        let archive = directory.path().join("themed.lunchbox-profile");

        let exported = export_profile(&state, &archive).unwrap();
        assert_eq!(exported.themes, 1);
        crate::couch_theme::remove_installed_theme("portable-profile-theme", &state).unwrap();
        assert!(
            crate::couch_theme::catalog_for_state(&state)
                .themes
                .iter()
                .all(|theme| theme.id != "portable-profile-theme")
        );

        stage_restore(&state, &archive).unwrap();
        apply_pending_restore(&state).unwrap().unwrap();
        let catalog = crate::couch_theme::catalog_for_state(&state);
        assert!(catalog.warnings.is_empty());
        assert!(catalog.themes.iter().any(|theme| {
            theme.id == "portable-profile-theme" && theme.name == "Portable Profile"
        }));
        assert_eq!(
            SettingsStore::at(&state)
                .unwrap()
                .load_couch_mode_preferences()
                .unwrap()
                .theme_id,
            "portable-profile-theme"
        );
    }

    #[test]
    fn profile_refuses_managed_destinations_and_cancels_staged_restore() {
        let directory = TempDir::new().unwrap();
        let state = state_fixture(&directory);
        let error = export_profile(&state, &state).unwrap_err().to_string();
        assert!(error.contains("live state database"), "{error}");
        assert_eq!(
            SettingsStore::at(&state)
                .unwrap()
                .load()
                .unwrap()
                .qbittorrent_host,
            "backup.example.test"
        );

        let archive = directory.path().join("cancel.lunchbox-profile");
        export_profile(&state, &archive).unwrap();
        stage_restore(&state, &archive).unwrap();
        let pending = state.parent().unwrap().join(PENDING_DIRECTORY);
        assert!(pending.exists());
        fs::write(pending.join("themes/foreign.txt"), b"foreign data").unwrap();
        let error = format!("{:#}", cancel_pending_restore(&state).unwrap_err());
        assert!(error.contains("undeclared file"), "{error}");
        fs::remove_file(pending.join("themes/foreign.txt")).unwrap();
        assert!(cancel_pending_restore(&state).unwrap());
        assert!(!pending.exists());
        assert!(!cancel_pending_restore(&state).unwrap());
        assert_eq!(
            SettingsStore::at(&state)
                .unwrap()
                .load()
                .unwrap()
                .qbittorrent_host,
            "backup.example.test"
        );

        SettingsStore::at(&state)
            .unwrap()
            .connection()
            .unwrap()
            .execute("ALTER TABLE app_settings ADD COLUMN password TEXT", [])
            .unwrap();
        let leaked = directory.path().join("must-not-leak.lunchbox-profile");
        let error = export_profile(&state, &leaked).unwrap_err().to_string();
        assert!(error.contains("forbidden credential column"), "{error}");
        assert!(!leaked.exists());
    }
}
