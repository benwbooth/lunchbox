use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};
use crc32fast::Hasher as Crc32;
use md5::Md5;
use rusqlite::{Connection, params};
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::catalog;
use crate::local_import::{
    MatchState, PreparedScanner, ScanProgress, ScanResult, decode_path, encode_path,
};
use crate::settings::{self, SettingsStore};

const MAX_AUDIT_RECORDS: usize = 500_000;
const MAX_AUDIT_HISTORY: usize = 100;
const FIXTURE_MARKER: &str = ".lunchbox-library-audit-fixture";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryAuditStatus {
    Healthy,
    Missing,
    Changed,
    Duplicate,
    Untracked,
    Unavailable,
    Unreadable,
}

impl LibraryAuditStatus {
    pub fn key(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Missing => "missing",
            Self::Changed => "changed",
            Self::Duplicate => "duplicate",
            Self::Untracked => "untracked",
            Self::Unavailable => "unavailable",
            Self::Unreadable => "unreadable",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "HEALTHY",
            Self::Missing => "MISSING",
            Self::Changed => "CHANGED",
            Self::Duplicate => "DUPLICATE",
            Self::Untracked => "NEW",
            Self::Unavailable => "ROOT OFFLINE",
            Self::Unreadable => "UNREADABLE",
        }
    }

    fn severity(self) -> u8 {
        match self {
            Self::Unavailable => 0,
            Self::Missing => 1,
            Self::Unreadable => 2,
            Self::Changed => 3,
            Self::Duplicate => 4,
            Self::Untracked => 5,
            Self::Healthy => 6,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LibraryAuditEntry {
    pub file_id: String,
    pub root_path: PathBuf,
    pub path: PathBuf,
    pub file_name: String,
    pub archive_member: String,
    pub title: String,
    pub platform: String,
    pub status: LibraryAuditStatus,
    pub detail: String,
}

impl LibraryAuditEntry {
    pub fn removable(&self) -> bool {
        self.status == LibraryAuditStatus::Missing && !self.file_id.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct LibraryAuditProgress {
    pub total_roots: usize,
    pub completed_roots: usize,
    pub current_root: String,
    pub root_total_files: usize,
    pub root_scanned_files: usize,
    pub current_file: String,
}

#[derive(Clone, Debug, Default)]
pub struct LibraryAuditOutput {
    pub entries: Vec<LibraryAuditEntry>,
    pub root_count: usize,
    pub scanned_root_count: usize,
    pub healthy_count: usize,
    pub missing_count: usize,
    pub changed_count: usize,
    pub duplicate_count: usize,
    pub untracked_count: usize,
    pub unavailable_count: usize,
    pub unreadable_count: usize,
    pub availability_changes: usize,
    pub warnings: Vec<String>,
    pub cancelled: bool,
    pub history: Vec<LibraryAuditRun>,
}

impl LibraryAuditOutput {
    pub fn issue_count(&self) -> usize {
        self.entries.len().saturating_sub(self.healthy_count)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryAuditRun {
    pub id: String,
    pub finished_at: i64,
    pub root_count: usize,
    pub scanned_root_count: usize,
    pub total_entry_count: usize,
    pub healthy_count: usize,
    pub missing_count: usize,
    pub changed_count: usize,
    pub duplicate_count: usize,
    pub untracked_count: usize,
    pub unavailable_count: usize,
    pub unreadable_count: usize,
    pub availability_changes: usize,
    pub warning_count: usize,
}

impl LibraryAuditRun {
    pub fn issue_count(&self) -> usize {
        self.total_entry_count.saturating_sub(self.healthy_count)
    }
}

#[derive(Clone, Debug)]
struct AuditRoot {
    id: String,
    path: PathBuf,
    display: String,
    platform_hint: String,
}

#[derive(Clone, Debug)]
struct PersistedAuditFile {
    id: String,
    root_id: String,
    path: PathBuf,
    file_name: String,
    archive_member: String,
    title: String,
    platform: String,
    file_size: u64,
    modified_unix_ns: Option<i64>,
    md5: String,
    sha1: String,
    included: bool,
    availability: String,
}

pub fn audit_local_collection(
    discovery_path: &Path,
    state_path: &Path,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(LibraryAuditProgress) + Send + Sync>,
) -> Result<LibraryAuditOutput> {
    let (roots, persisted) = load_audit_state(state_path)?;
    if persisted.len() > MAX_AUDIT_RECORDS {
        bail!("local library contains too many records to audit safely");
    }
    let root_count = roots.len();
    progress(LibraryAuditProgress {
        total_roots: root_count,
        ..LibraryAuditProgress::default()
    });
    if roots.is_empty() {
        return Ok(LibraryAuditOutput {
            root_count,
            ..LibraryAuditOutput::default()
        });
    }

    let scanner = PreparedScanner::new(discovery_path)?;
    let all_persisted_ids = persisted
        .iter()
        .map(|file| file.id.clone())
        .collect::<HashSet<_>>();
    let mut files_by_root = HashMap::<String, Vec<&PersistedAuditFile>>::new();
    for file in &persisted {
        files_by_root
            .entry(file.root_id.clone())
            .or_default()
            .push(file);
    }

    let mut entries = Vec::new();
    let mut successful_roots = Vec::<(String, HashSet<String>)>::new();
    let mut duplicate_candidates = HashMap::<String, Vec<usize>>::new();
    let mut warnings = Vec::new();
    let mut scanned_root_count = 0_usize;

    for (root_index, root) in roots.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let root_progress = Arc::clone(&progress);
        let root_display = root.display.clone();
        let scan_progress: Arc<dyn Fn(ScanProgress) + Send + Sync> =
            Arc::new(move |scan: ScanProgress| {
                root_progress(LibraryAuditProgress {
                    total_roots: root_count,
                    completed_roots: root_index,
                    current_root: root_display.clone(),
                    root_total_files: scan.total_files,
                    root_scanned_files: scan.scanned_files,
                    current_file: scan.current_file,
                });
            });
        let output = match scanner.scan(
            &root.path,
            &root.platform_hint,
            true,
            cancelled,
            scan_progress,
        ) {
            Ok(output) if !output.cancelled => output,
            Ok(_) => break,
            Err(error) => {
                let detail = format!("Collection root is unavailable: {error:#}");
                warnings.push(format!("{}: {error:#}", root.display));
                for file in files_by_root
                    .get(&root.id)
                    .into_iter()
                    .flat_map(|files| files.iter())
                    .filter(|file| file.included)
                {
                    entries.push(entry_from_persisted(
                        root,
                        file,
                        LibraryAuditStatus::Unavailable,
                        detail.clone(),
                    ));
                }
                continue;
            }
        };
        scanned_root_count = scanned_root_count.saturating_add(1);

        let current = output
            .results
            .iter()
            .map(|result| (scan_result_id(&root.id, result), result))
            .collect::<HashMap<_, _>>();
        let current_ids = current.keys().cloned().collect::<HashSet<_>>();
        successful_roots.push((root.id.clone(), current_ids));

        for file in files_by_root
            .get(&root.id)
            .into_iter()
            .flat_map(|files| files.iter())
            .filter(|file| file.included)
        {
            let Some(result) = current.get(&file.id) else {
                entries.push(entry_from_persisted(
                    root,
                    file,
                    LibraryAuditStatus::Missing,
                    "Not found during a complete scan of this collection root.".to_owned(),
                ));
                continue;
            };
            let (status, detail) = compare_file(file, result);
            let index = entries.len();
            entries.push(entry_from_scan(
                root,
                file.id.clone(),
                result,
                status,
                detail,
            ));
            if status == LibraryAuditStatus::Healthy && !result.sha1.is_empty() {
                duplicate_candidates
                    .entry(format!("sha1:{}", result.sha1.to_ascii_uppercase()))
                    .or_default()
                    .push(index);
            }
        }

        for (file_id, result) in current {
            if all_persisted_ids.contains(&file_id) {
                continue;
            }
            let detail = match &result.match_state {
                MatchState::Error(error) => format!("New file needs review: {error}"),
                _ => {
                    "New ROM discovered. Review it in Import ROMs before adding it to the library."
                        .to_owned()
                }
            };
            entries.push(entry_from_scan(
                root,
                String::new(),
                result,
                LibraryAuditStatus::Untracked,
                detail,
            ));
        }
    }

    for indices in duplicate_candidates
        .values()
        .filter(|indices| indices.len() > 1)
    {
        let count = indices.len();
        for index in indices {
            if let Some(entry) = entries.get_mut(*index) {
                entry.status = LibraryAuditStatus::Duplicate;
                entry.detail = format!(
                    "Exact SHA-1 content is shared by {count} tracked files. No records were merged or removed."
                );
            }
        }
    }

    let availability_changes = synchronize_availability(state_path, &persisted, &successful_roots)?;
    entries.sort_by(|left, right| {
        left.status
            .severity()
            .cmp(&right.status.severity())
            .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.archive_member.cmp(&right.archive_member))
    });

    let mut output = LibraryAuditOutput {
        root_count,
        scanned_root_count,
        entries,
        availability_changes,
        warnings,
        cancelled: cancelled.load(Ordering::Relaxed),
        ..LibraryAuditOutput::default()
    };
    for entry in &output.entries {
        match entry.status {
            LibraryAuditStatus::Healthy => output.healthy_count += 1,
            LibraryAuditStatus::Missing => output.missing_count += 1,
            LibraryAuditStatus::Changed => output.changed_count += 1,
            LibraryAuditStatus::Duplicate => output.duplicate_count += 1,
            LibraryAuditStatus::Untracked => output.untracked_count += 1,
            LibraryAuditStatus::Unavailable => output.unavailable_count += 1,
            LibraryAuditStatus::Unreadable => output.unreadable_count += 1,
        }
    }
    progress(LibraryAuditProgress {
        total_roots: root_count,
        completed_roots: scanned_root_count,
        ..LibraryAuditProgress::default()
    });
    output.history = if output.cancelled || output.root_count == 0 {
        load_audit_runs(state_path)?
    } else {
        record_audit_run(state_path, &output)?
    };
    Ok(output)
}

fn record_audit_run(
    state_path: &Path,
    output: &LibraryAuditOutput,
) -> Result<Vec<LibraryAuditRun>> {
    if output.cancelled || output.root_count == 0 {
        bail!("only completed audits with at least one root can be recorded");
    }
    let run = LibraryAuditRun {
        id: Uuid::new_v4().to_string(),
        finished_at: settings::unix_timestamp(),
        root_count: output.root_count,
        scanned_root_count: output.scanned_root_count,
        total_entry_count: output.entries.len(),
        healthy_count: output.healthy_count,
        missing_count: output.missing_count,
        changed_count: output.changed_count,
        duplicate_count: output.duplicate_count,
        untracked_count: output.untracked_count,
        unavailable_count: output.unavailable_count,
        unreadable_count: output.unreadable_count,
        availability_changes: output.availability_changes,
        warning_count: output.warnings.len(),
    };
    validate_audit_run(&run)?;

    let store = SettingsStore::at(state_path)?;
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO library_audit_runs (
             id, finished_at, root_count, scanned_root_count, total_entry_count,
             healthy_count, missing_count, changed_count, duplicate_count,
             untracked_count, unavailable_count, unreadable_count,
             availability_changes, warning_count
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
         )",
        params![
            run.id,
            run.finished_at,
            usize_to_i64(run.root_count),
            usize_to_i64(run.scanned_root_count),
            usize_to_i64(run.total_entry_count),
            usize_to_i64(run.healthy_count),
            usize_to_i64(run.missing_count),
            usize_to_i64(run.changed_count),
            usize_to_i64(run.duplicate_count),
            usize_to_i64(run.untracked_count),
            usize_to_i64(run.unavailable_count),
            usize_to_i64(run.unreadable_count),
            usize_to_i64(run.availability_changes),
            usize_to_i64(run.warning_count),
        ],
    )?;
    transaction.execute(
        "DELETE FROM library_audit_runs
         WHERE id IN (
             SELECT id FROM library_audit_runs
             ORDER BY finished_at DESC, sequence DESC
             LIMIT -1 OFFSET ?1
         )",
        [usize_to_i64(MAX_AUDIT_HISTORY)],
    )?;
    let history = load_audit_runs_on(&transaction)?;
    transaction.commit()?;
    Ok(history)
}

pub fn load_audit_runs(state_path: &Path) -> Result<Vec<LibraryAuditRun>> {
    let store = SettingsStore::at(state_path)?;
    let connection = store.connection()?;
    load_audit_runs_on(&connection)
}

fn load_audit_runs_on(connection: &Connection) -> Result<Vec<LibraryAuditRun>> {
    let mut statement = connection.prepare(
        "SELECT id, finished_at, root_count, scanned_root_count, total_entry_count,
                healthy_count, missing_count, changed_count, duplicate_count,
                untracked_count, unavailable_count, unreadable_count,
                availability_changes, warning_count
         FROM library_audit_runs
         ORDER BY finished_at DESC, sequence DESC
         LIMIT ?1",
    )?;
    let rows = statement.query_map([usize_to_i64(MAX_AUDIT_HISTORY)], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, i64>(9)?,
            row.get::<_, i64>(10)?,
            row.get::<_, i64>(11)?,
            row.get::<_, i64>(12)?,
            row.get::<_, i64>(13)?,
        ))
    })?;
    let mut history = Vec::new();
    for row in rows {
        let (
            id,
            finished_at,
            root_count,
            scanned_root_count,
            total_entry_count,
            healthy_count,
            missing_count,
            changed_count,
            duplicate_count,
            untracked_count,
            unavailable_count,
            unreadable_count,
            availability_changes,
            warning_count,
        ) = row?;
        let run = LibraryAuditRun {
            id,
            finished_at,
            root_count: nonnegative_usize(root_count)?,
            scanned_root_count: nonnegative_usize(scanned_root_count)?,
            total_entry_count: nonnegative_usize(total_entry_count)?,
            healthy_count: nonnegative_usize(healthy_count)?,
            missing_count: nonnegative_usize(missing_count)?,
            changed_count: nonnegative_usize(changed_count)?,
            duplicate_count: nonnegative_usize(duplicate_count)?,
            untracked_count: nonnegative_usize(untracked_count)?,
            unavailable_count: nonnegative_usize(unavailable_count)?,
            unreadable_count: nonnegative_usize(unreadable_count)?,
            availability_changes: nonnegative_usize(availability_changes)?,
            warning_count: nonnegative_usize(warning_count)?,
        };
        validate_audit_run(&run)?;
        history.push(run);
    }
    Ok(history)
}

fn validate_audit_run(run: &LibraryAuditRun) -> Result<()> {
    let status_total = run
        .healthy_count
        .saturating_add(run.missing_count)
        .saturating_add(run.changed_count)
        .saturating_add(run.duplicate_count)
        .saturating_add(run.untracked_count)
        .saturating_add(run.unavailable_count)
        .saturating_add(run.unreadable_count);
    if Uuid::parse_str(&run.id).is_err()
        || run.finished_at < 0
        || run.scanned_root_count > run.root_count
        || status_total != run.total_entry_count
    {
        bail!(
            "library audit history row {} is internally inconsistent",
            run.id
        );
    }
    Ok(())
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn nonnegative_usize(value: i64) -> Result<usize> {
    usize::try_from(value).context("library audit history contains an invalid negative count")
}

pub fn remove_missing_records(state_path: &Path, file_ids: &[String]) -> Result<usize> {
    if file_ids.is_empty() {
        return Ok(0);
    }
    if file_ids.len() > MAX_AUDIT_RECORDS {
        bail!("too many missing records were selected");
    }
    let mut unique = HashSet::with_capacity(file_ids.len());
    for file_id in file_ids {
        if Uuid::parse_str(file_id).is_err() || !unique.insert(file_id.clone()) {
            bail!("missing-record cleanup requires unique stable file IDs");
        }
    }
    let store = SettingsStore::at(state_path)?;
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    let mut removed = 0_usize;
    for file_id in unique {
        removed = removed.saturating_add(transaction.execute(
            "DELETE FROM local_rom_files
             WHERE id=?1 AND included=1 AND availability='missing'",
            [file_id],
        )?);
    }
    transaction.commit()?;
    Ok(removed)
}

fn load_audit_state(state_path: &Path) -> Result<(Vec<AuditRoot>, Vec<PersistedAuditFile>)> {
    let store = SettingsStore::at(state_path)?;
    let connection = store.connection()?;
    let mut root_statement = connection.prepare(
        "SELECT id, path_display, path_bytes, path_encoding, platform_hint
         FROM local_collection_roots ORDER BY path_display COLLATE NOCASE, id",
    )?;
    let root_rows = root_statement.query_map([], |row| {
        let display = row.get::<_, String>(1)?;
        Ok(AuditRoot {
            id: row.get(0)?,
            path: decode_path(row.get(2)?, &row.get::<_, String>(3)?, display.clone()),
            display,
            platform_hint: row.get(4)?,
        })
    })?;
    let roots = root_rows.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut file_statement = connection.prepare(
        "SELECT id, root_id,
                CASE WHEN source_archive_path_display<>''
                     THEN source_archive_path_display ELSE path_display END,
                CASE WHEN length(source_archive_path_bytes)>0
                     THEN source_archive_path_bytes ELSE path_bytes END,
                CASE WHEN source_archive_path_encoding<>''
                     THEN source_archive_path_encoding ELSE path_encoding END,
                file_name, archive_member,
                coalesce(nullif(matched_title, ''), display_title), platform,
                file_size, modified_unix_ns, upper(md5), upper(sha1),
                included, availability
         FROM local_rom_files ORDER BY root_id, id",
    )?;
    let file_rows = file_statement.query_map([], |row| {
        let display = row.get::<_, String>(2)?;
        Ok(PersistedAuditFile {
            id: row.get(0)?,
            root_id: row.get(1)?,
            path: decode_path(row.get(3)?, &row.get::<_, String>(4)?, display),
            file_name: row.get(5)?,
            archive_member: row.get(6)?,
            title: row.get(7)?,
            platform: row.get(8)?,
            file_size: u64::try_from(row.get::<_, i64>(9)?).unwrap_or_default(),
            modified_unix_ns: row.get(10)?,
            md5: row.get(11)?,
            sha1: row.get(12)?,
            included: row.get(13)?,
            availability: row.get(14)?,
        })
    })?;
    let files = file_rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok((roots, files))
}

fn compare_file(
    persisted: &PersistedAuditFile,
    current: &ScanResult,
) -> (LibraryAuditStatus, String) {
    if let MatchState::Error(error) = &current.match_state {
        return (
            LibraryAuditStatus::Unreadable,
            format!("The file exists but could not be verified: {error}"),
        );
    }
    if persisted.archive_member != current.archive_member {
        return (
            LibraryAuditStatus::Changed,
            "The archive member identity changed since import.".to_owned(),
        );
    }
    if !persisted.sha1.is_empty() && !persisted.sha1.eq_ignore_ascii_case(&current.sha1) {
        return (
            LibraryAuditStatus::Changed,
            "SHA-1 content no longer matches the imported ROM.".to_owned(),
        );
    }
    if persisted.sha1.is_empty()
        && !persisted.md5.is_empty()
        && !persisted.md5.eq_ignore_ascii_case(&current.md5)
    {
        return (
            LibraryAuditStatus::Changed,
            "MD5 content no longer matches the imported ROM.".to_owned(),
        );
    }
    if persisted.sha1.is_empty()
        && persisted.md5.is_empty()
        && (persisted.file_size != current.file_size
            || persisted.modified_unix_ns != current.modified_unix_ns)
    {
        return (
            LibraryAuditStatus::Changed,
            "Size or modification time changed; re-import to establish exact hashes.".to_owned(),
        );
    }
    (
        LibraryAuditStatus::Healthy,
        "Exact content and stored identity are intact.".to_owned(),
    )
}

fn entry_from_persisted(
    root: &AuditRoot,
    file: &PersistedAuditFile,
    status: LibraryAuditStatus,
    detail: String,
) -> LibraryAuditEntry {
    LibraryAuditEntry {
        file_id: file.id.clone(),
        root_path: root.path.clone(),
        path: file.path.clone(),
        file_name: file.file_name.clone(),
        archive_member: file.archive_member.clone(),
        title: file.title.clone(),
        platform: file.platform.clone(),
        status,
        detail,
    }
}

fn entry_from_scan(
    root: &AuditRoot,
    file_id: String,
    result: &ScanResult,
    status: LibraryAuditStatus,
    detail: String,
) -> LibraryAuditEntry {
    LibraryAuditEntry {
        file_id,
        root_path: root.path.clone(),
        path: result.path.clone(),
        file_name: result.file_name.clone(),
        archive_member: result.archive_member.clone(),
        title: result.display_title.clone(),
        platform: result.platform.clone(),
        status,
        detail,
    }
}

fn scan_result_id(root_id: &str, result: &ScanResult) -> String {
    let relative = encode_path(&result.relative_path);
    let mut identity = root_id.as_bytes().to_vec();
    identity.push(0);
    identity.extend_from_slice(&relative.bytes);
    if result.archive_member_count > 1 {
        identity.push(0);
        identity.extend_from_slice(result.archive_member.as_bytes());
    }
    Uuid::new_v5(&Uuid::NAMESPACE_URL, &identity).to_string()
}

fn synchronize_availability(
    state_path: &Path,
    persisted: &[PersistedAuditFile],
    successful_roots: &[(String, HashSet<String>)],
) -> Result<usize> {
    let current_by_root = successful_roots
        .iter()
        .map(|(root_id, current)| (root_id.as_str(), current))
        .collect::<HashMap<_, _>>();
    if current_by_root.is_empty() {
        return Ok(0);
    }
    let mut connection = Connection::open(state_path)?;
    let transaction = connection.transaction()?;
    let mut changed = 0_usize;
    for file in persisted {
        let Some(current) = current_by_root.get(file.root_id.as_str()) else {
            continue;
        };
        let desired = if current.contains(&file.id) {
            "present"
        } else {
            "missing"
        };
        if file.availability != desired {
            changed = changed.saturating_add(transaction.execute(
                "UPDATE local_rom_files SET availability=?2 WHERE id=?1",
                params![file.id, desired],
            )?);
        }
    }
    transaction.commit()?;
    Ok(changed)
}

pub fn library_audit_fixture_probe() -> Result<String> {
    let discovery_path = catalog::requested_discovery_database_path()
        .context("--database is required for the library audit fixture probe")?;
    let state_path = settings::state_database_path()?;
    let root = catalog::requested_path("--audit-fixture-root", "LUNCHBOX_AUDIT_FIXTURE_ROOT")
        .context("--audit-fixture-root is required for the library audit fixture probe")?;
    seed_fixture(&state_path, &root)?;
    let cancel = AtomicBool::new(false);
    let output = audit_local_collection(&discovery_path, &state_path, &cancel, Arc::new(|_| {}))?;
    if output.healthy_count != 1
        || output.missing_count != 1
        || output.changed_count != 1
        || output.duplicate_count != 2
        || output.untracked_count != 1
        || output.unavailable_count != 0
        || output.unreadable_count != 0
        || output.history.len() != 1
    {
        bail!(
            "audit fixture produced unexpected counts: healthy={} missing={} changed={} duplicate={} new={} unavailable={} unreadable={} history={}",
            output.healthy_count,
            output.missing_count,
            output.changed_count,
            output.duplicate_count,
            output.untracked_count,
            output.unavailable_count,
            output.unreadable_count,
            output.history.len()
        );
    }
    Ok(format!(
        "roots={} entries={} healthy={} missing={} changed={} duplicates={} new={} availability_changes={} history={} state={:?}",
        output.root_count,
        output.entries.len(),
        output.healthy_count,
        output.missing_count,
        output.changed_count,
        output.duplicate_count,
        output.untracked_count,
        output.availability_changes,
        output.history.len(),
        state_path
    ))
}

fn seed_fixture(state_path: &Path, root: &Path) -> Result<()> {
    if root.exists() && !root.join(FIXTURE_MARKER).is_file() {
        bail!("audit fixture root already exists without the Lunchbox fixture marker");
    }
    fs::create_dir_all(root)?;
    fs::write(
        root.join(FIXTURE_MARKER),
        b"owned by the Lunchbox audit probe\n",
    )?;
    let root = root.canonicalize()?;
    let root_encoded = encode_path(&root);
    let root_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, &root_encoded.bytes).to_string();
    let store = SettingsStore::at(state_path)?;
    let mut connection = store.connection()?;
    let foreign_root_count: i64 = connection.query_row(
        "SELECT count(*) FROM local_collection_roots WHERE id<>?1",
        [&root_id],
        |row| row.get(0),
    )?;
    if foreign_root_count != 0 {
        bail!("audit fixture state already contains a non-fixture collection root");
    }

    let fixtures: [(&str, &[u8], bool); 5] = [
        ("Healthy.nes", b"healthy-content", true),
        ("Duplicate One.nes", b"duplicate-content", true),
        ("Duplicate Two.nes", b"duplicate-content", true),
        ("Changed.nes", b"old-content", true),
        ("Missing.nes", b"missing-content", false),
    ];
    for (name, content, _) in fixtures {
        fs::write(root.join(name), content)?;
    }
    fs::write(root.join("Untracked.nes"), b"new-content")?;

    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO local_collection_roots (
             id, path_display, path_bytes, path_encoding, platform_hint,
             checksums_enabled, last_scanned_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
         ON CONFLICT(id) DO UPDATE SET
             path_display=excluded.path_display, path_bytes=excluded.path_bytes,
             path_encoding=excluded.path_encoding, platform_hint=excluded.platform_hint,
             checksums_enabled=1, last_scanned_at=excluded.last_scanned_at",
        params![
            root_id,
            root_encoded.display,
            root_encoded.bytes,
            root_encoded.encoding,
            "Nintendo Entertainment System",
            settings::unix_timestamp(),
        ],
    )?;
    transaction.execute("DELETE FROM local_rom_files WHERE root_id=?1", [&root_id])?;

    for (name, content, remains_present) in fixtures {
        let relative = PathBuf::from(name);
        let path = root.join(&relative);
        let encoded_path = encode_path(&path);
        let encoded_relative = encode_path(&relative);
        let file_id = fixture_file_id(&root_id, &encoded_relative.bytes);
        let (crc32, md5, sha1) = digests(content);
        let modified_unix_ns = path
            .metadata()?
            .modified()
            .ok()
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|value| i64::try_from(value.as_nanos()).ok());
        transaction.execute(
            "INSERT INTO local_rom_files (
                 id, root_id, path_display, path_bytes, path_encoding,
                 relative_path_display, relative_path_bytes, relative_path_encoding,
                 file_name, archive_member, source_archive_path_display,
                 source_archive_path_bytes, source_archive_path_encoding,
                 display_title, platform, file_size, modified_unix_ns,
                 crc32, md5, sha1, game_uid, launchbox_db_id, matched_title,
                 match_state, match_method, included, availability, imported_at
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, '', '', X'', '',
                 ?10, ?11, ?12, ?13, ?14, ?15, ?16, NULL, 0, NULL,
                 'inventory_only', 'audit fixture', 1, 'present', ?17
             )",
            params![
                file_id,
                root_id,
                encoded_path.display,
                encoded_path.bytes,
                encoded_path.encoding,
                encoded_relative.display,
                encoded_relative.bytes,
                encoded_relative.encoding,
                name,
                name.trim_end_matches(".nes"),
                "Nintendo Entertainment System",
                i64::try_from(content.len()).unwrap_or(i64::MAX),
                modified_unix_ns,
                crc32,
                md5,
                sha1,
                settings::unix_timestamp(),
            ],
        )?;
        if !remains_present {
            fs::remove_file(path)?;
        }
    }
    transaction.commit()?;
    fs::write(root.join("Changed.nes"), b"new-content")?;
    Ok(())
}

fn fixture_file_id(root_id: &str, relative_bytes: &[u8]) -> String {
    let mut identity = root_id.as_bytes().to_vec();
    identity.push(0);
    identity.extend_from_slice(relative_bytes);
    Uuid::new_v5(&Uuid::NAMESPACE_URL, &identity).to_string()
}

fn digests(content: &[u8]) -> (String, String, String) {
    let mut crc32 = Crc32::new();
    crc32.update(content);
    (
        format!("{:08X}", crc32.finalize()),
        format!("{:X}", Md5::digest(content)),
        format!("{:X}", Sha1::digest(content)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn discovery_fixture(path: &Path) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE platforms (
                     id INTEGER PRIMARY KEY, name TEXT, file_extensions TEXT
                 );
                 CREATE TABLE games (
                     id TEXT PRIMARY KEY, title TEXT, platform_id INTEGER,
                     launchbox_db_id INTEGER, libretro_crc32 TEXT,
                     libretro_md5 TEXT, libretro_sha1 TEXT
                 );
                 INSERT INTO platforms VALUES (
                     1, 'Nintendo Entertainment System', 'nes'
                 );",
            )
            .unwrap();
    }

    #[test]
    fn exact_audit_distinguishes_every_maintenance_state_and_cleans_only_missing() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let state = directory.path().join("state.db");
        let root = directory.path().join("roms");
        discovery_fixture(&discovery);
        seed_fixture(&state, &root).unwrap();

        let output = audit_local_collection(
            &discovery,
            &state,
            &AtomicBool::new(false),
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(output.healthy_count, 1);
        assert_eq!(output.missing_count, 1);
        assert_eq!(output.changed_count, 1);
        assert_eq!(output.duplicate_count, 2);
        assert_eq!(output.untracked_count, 1);
        assert_eq!(output.unavailable_count, 0);
        assert_eq!(output.availability_changes, 1);
        assert_eq!(output.history.len(), 1);
        assert_eq!(output.history[0].issue_count(), 5);
        assert_eq!(output.history[0].missing_count, 1);
        let preserved_files = [
            "Healthy.nes",
            "Duplicate One.nes",
            "Duplicate Two.nes",
            "Changed.nes",
            "Untracked.nes",
        ]
        .map(|name| (name, fs::read(root.join(name)).unwrap()));

        let missing = output
            .entries
            .iter()
            .find(|entry| entry.status == LibraryAuditStatus::Missing)
            .unwrap();
        assert!(missing.removable());
        assert_eq!(
            remove_missing_records(&state, std::slice::from_ref(&missing.file_id)).unwrap(),
            1
        );
        let repeated =
            remove_missing_records(&state, std::slice::from_ref(&missing.file_id)).unwrap();
        assert_eq!(repeated, 0);
        for (name, content) in preserved_files {
            assert_eq!(fs::read(root.join(name)).unwrap(), content);
        }
        assert!(!root.join("Missing.nes").exists());

        let refreshed = audit_local_collection(
            &discovery,
            &state,
            &AtomicBool::new(false),
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(refreshed.missing_count, 0);
        assert_eq!(refreshed.entries.len(), 5);
        assert_eq!(refreshed.history.len(), 2);
        assert_eq!(refreshed.history[0].missing_count, 0);
        assert_eq!(refreshed.history[0].issue_count(), 4);
        assert_eq!(refreshed.history[1].missing_count, 1);
        assert_eq!(refreshed.history[1].issue_count(), 5);
    }

    #[test]
    fn unavailable_root_is_not_synchronized_to_missing() {
        let directory = tempfile::tempdir().unwrap();
        let discovery = directory.path().join("games.db");
        let state = directory.path().join("state.db");
        let root = directory.path().join("roms");
        discovery_fixture(&discovery);
        seed_fixture(&state, &root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        let output = audit_local_collection(
            &discovery,
            &state,
            &AtomicBool::new(false),
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(output.unavailable_count, 5);
        assert_eq!(output.missing_count, 0);
        assert_eq!(output.availability_changes, 0);
        assert!(output.entries.iter().all(|entry| !entry.removable()));
    }

    #[test]
    fn audit_history_is_bounded_and_cancelled_runs_never_replace_the_baseline() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path().join("state.db");
        SettingsStore::at(&state).unwrap();
        let mut output = LibraryAuditOutput {
            root_count: 1,
            scanned_root_count: 1,
            entries: vec![LibraryAuditEntry {
                file_id: Uuid::new_v4().to_string(),
                root_path: directory.path().into(),
                path: directory.path().join("Game.rom"),
                file_name: "Game.rom".into(),
                archive_member: String::new(),
                title: "Game".into(),
                platform: "Platform".into(),
                status: LibraryAuditStatus::Healthy,
                detail: "Verified".into(),
            }],
            healthy_count: 1,
            ..LibraryAuditOutput::default()
        };
        for _ in 0..MAX_AUDIT_HISTORY.saturating_add(7) {
            output.history = record_audit_run(&state, &output).unwrap();
        }
        assert_eq!(output.history.len(), MAX_AUDIT_HISTORY);
        assert_eq!(load_audit_runs(&state).unwrap().len(), MAX_AUDIT_HISTORY);

        output.cancelled = true;
        assert!(record_audit_run(&state, &output).is_err());
        assert_eq!(load_audit_runs(&state).unwrap().len(), MAX_AUDIT_HISTORY);
    }
}
