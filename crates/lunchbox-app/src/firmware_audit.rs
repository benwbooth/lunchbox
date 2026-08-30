use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use rusqlite::params;

use crate::firmware::FirmwareStatus;

const MAX_LOCAL_GAMES: usize = 100_000;
const UI_FIXTURE_ROOT_ID: &str = "firmware-audit-ui-probe-root";
const UI_FIXTURE_FILE_ID: &str = "firmware-audit-ui-probe-file";
const UI_FIXTURE_MARKER: &str = ".lunchbox-firmware-audit-fixture";
const NO_EXTERNAL_FIRMWARE: &str = "No external firmware";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirmwareAuditStatus {
    Missing,
    Manual,
    Optional,
    Ready,
    NoRuntime,
    Error,
}

impl FirmwareAuditStatus {
    pub fn key(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Manual => "manual",
            Self::Optional => "optional",
            Self::Ready => "ready",
            Self::NoRuntime => "runtime",
            Self::Error => "error",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Missing => "REQUIRED",
            Self::Manual => "MANUAL DUMP",
            Self::Optional => "OPTIONAL",
            Self::Ready => "READY",
            Self::NoRuntime => "NO EMULATOR",
            Self::Error => "REVIEW",
        }
    }

    pub fn severity(self) -> u8 {
        match self {
            Self::Missing => 0,
            Self::Manual => 1,
            Self::NoRuntime => 2,
            Self::Error => 3,
            Self::Optional => 4,
            Self::Ready => 5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FirmwareAuditEntry {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub emulator: String,
    pub package: String,
    pub source: String,
    pub status: FirmwareAuditStatus,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub struct FirmwareAuditProgress {
    pub current: usize,
    pub total: usize,
    pub platform: String,
}

#[derive(Clone, Debug, Default)]
pub struct FirmwareAuditOutput {
    pub entries: Vec<FirmwareAuditEntry>,
    pub game_count: usize,
    pub platform_count: usize,
    pub no_firmware_platform_count: usize,
    pub missing_count: usize,
    pub manual_count: usize,
    pub optional_count: usize,
    pub ready_count: usize,
    pub no_runtime_count: usize,
    pub error_count: usize,
    pub unavailable_game_count: usize,
    pub cancelled: bool,
}

pub fn seed_ui_probe() -> Result<PathBuf> {
    let state_path = crate::settings::state_database_path()?;
    let root = crate::catalog::requested_path(
        "--firmware-audit-fixture-root",
        "LUNCHBOX_FIRMWARE_AUDIT_FIXTURE_ROOT",
    )
    .unwrap_or_else(|| {
        state_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("firmware-audit-fixture")
    });
    if root.exists() && !root.join(UI_FIXTURE_MARKER).is_file() {
        anyhow::bail!(
            "firmware audit fixture root already exists without the Lunchbox fixture marker"
        );
    }
    std::fs::create_dir_all(&root)?;
    std::fs::write(
        root.join(UI_FIXTURE_MARKER),
        b"owned by the Lunchbox firmware audit UI probe\n",
    )?;
    let rom_path = root.join("Buck Rogers - Planet of Zoom.dsk");
    std::fs::write(&rom_path, b"Lunchbox firmware audit probe media\n")?;
    let root = root.canonicalize()?;
    let rom_path = rom_path.canonicalize()?;
    let root_path = crate::local_import::encode_path(&root);
    let rom_path_encoded = crate::local_import::encode_path(&rom_path);
    let relative = PathBuf::from("Buck Rogers - Planet of Zoom.dsk");
    let relative_encoded = crate::local_import::encode_path(&relative);
    let store = crate::settings::SettingsStore::at(&state_path)?;
    let mut connection = store.connection()?;
    let foreign_roots: i64 = connection.query_row(
        "SELECT count(*) FROM local_collection_roots WHERE id<>?1",
        [UI_FIXTURE_ROOT_ID],
        |row| row.get(0),
    )?;
    let foreign_files: i64 = connection.query_row(
        "SELECT count(*) FROM local_rom_files WHERE id<>?1",
        [UI_FIXTURE_FILE_ID],
        |row| row.get(0),
    )?;
    if foreign_roots != 0 || foreign_files != 0 {
        anyhow::bail!(
            "firmware audit UI probe requires an isolated --state-database (found {foreign_roots} foreign roots and {foreign_files} foreign files)"
        );
    }

    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO local_collection_roots (
             id, path_display, path_bytes, path_encoding, platform_hint,
             checksums_enabled, last_scanned_at
         ) VALUES (?1, ?2, ?3, ?4, 'Coleco ADAM', 0, ?5)
         ON CONFLICT(id) DO UPDATE SET
             path_display=excluded.path_display, path_bytes=excluded.path_bytes,
             path_encoding=excluded.path_encoding, platform_hint=excluded.platform_hint,
             checksums_enabled=0, last_scanned_at=excluded.last_scanned_at",
        params![
            UI_FIXTURE_ROOT_ID,
            root_path.display,
            root_path.bytes,
            root_path.encoding,
            crate::settings::unix_timestamp(),
        ],
    )?;
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
             ?10, 'Coleco ADAM', ?11, NULL, '', '', '', ?12, 0, ?10,
             'reviewed', 'firmware audit UI fixture', 1, 'present', ?13
         )
         ON CONFLICT(id) DO UPDATE SET
             root_id=excluded.root_id, path_display=excluded.path_display,
             path_bytes=excluded.path_bytes, path_encoding=excluded.path_encoding,
             relative_path_display=excluded.relative_path_display,
             relative_path_bytes=excluded.relative_path_bytes,
             relative_path_encoding=excluded.relative_path_encoding,
             file_name=excluded.file_name, display_title=excluded.display_title,
             platform=excluded.platform, file_size=excluded.file_size,
             game_uid=excluded.game_uid, matched_title=excluded.matched_title,
             included=1, availability='present', imported_at=excluded.imported_at",
        params![
            UI_FIXTURE_FILE_ID,
            UI_FIXTURE_ROOT_ID,
            rom_path_encoded.display,
            rom_path_encoded.bytes,
            rom_path_encoded.encoding,
            relative_encoded.display,
            relative_encoded.bytes,
            relative_encoded.encoding,
            "Buck Rogers - Planet of Zoom.dsk",
            "Buck Rogers: Planet of Zoom",
            i64::try_from(std::fs::metadata(&rom_path)?.len()).unwrap_or(i64::MAX),
            "local-file:firmware-audit-ui-probe",
            crate::settings::unix_timestamp(),
        ],
    )?;
    transaction.commit()?;
    drop(connection);
    store.set_platform_emulator_preference(
        "Coleco ADAM",
        "74ca593a-1d78-52e9-bc7f-c77608a3b09d",
        "standalone",
        "",
    )?;
    Ok(root)
}

#[derive(Clone, Debug)]
struct LocalGameTarget {
    game_uid: String,
    launchbox_db_id: i64,
    title: String,
    platform: String,
    rom_path: PathBuf,
}

pub fn audit_local_collection(
    catalog_database: &Path,
    state_database: &Path,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(FirmwareAuditProgress) + Send + Sync>,
) -> Result<FirmwareAuditOutput> {
    let (targets, unavailable_game_count) = load_local_targets(state_database)?;
    let game_count = targets.len().saturating_add(unavailable_game_count);
    let store = crate::settings::SettingsStore::at(state_database)?;
    let mut platform_keys = HashSet::<String>::new();
    let mut representatives = HashMap::<
        (String, String, String, String),
        (LocalGameTarget, Option<crate::settings::EmulatorPreference>),
    >::new();
    for target in targets {
        let platform_key = crate::catalog::normalize_platform_key(&target.platform);
        platform_keys.insert(platform_key.clone());
        let preference = store.emulator_preference(&target.game_uid, &target.platform)?;
        let key = (
            platform_key,
            preference
                .as_ref()
                .map(|value| value.emulator_id.clone())
                .unwrap_or_default(),
            preference
                .as_ref()
                .map(|value| value.runtime_kind.clone())
                .unwrap_or_default(),
            preference
                .as_ref()
                .map(|value| value.core_name.clone())
                .unwrap_or_default(),
        );
        representatives.entry(key).or_insert((target, preference));
    }
    let mut representatives = representatives.into_values().collect::<Vec<_>>();
    representatives.sort_by_cached_key(|(target, preference)| {
        (
            target.platform.to_lowercase(),
            preference
                .as_ref()
                .map(|value| value.emulator_id.to_lowercase())
                .unwrap_or_default(),
            preference
                .as_ref()
                .map(|value| value.core_name.to_lowercase())
                .unwrap_or_default(),
        )
    });
    let platform_count = platform_keys.len();
    let inspection_count = representatives.len();
    progress(FirmwareAuditProgress {
        total: inspection_count,
        ..FirmwareAuditProgress::default()
    });

    let mut output = FirmwareAuditOutput {
        game_count,
        platform_count,
        unavailable_game_count,
        ..FirmwareAuditOutput::default()
    };

    for (index, (target, preference)) in representatives.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            output.cancelled = true;
            break;
        }
        progress(FirmwareAuditProgress {
            current: index,
            total: inspection_count,
            platform: target.platform.clone(),
        });
        let result = (|| -> Result<Vec<FirmwareAuditEntry>> {
            let availability = crate::emulator::inspect_rom_launch_availability(
                &target.platform,
                &target.rom_path,
                catalog_database,
                preference.as_ref(),
            )?;
            let Some(selected_index) = availability.selected_index else {
                return Ok(vec![entry_for_runtime_problem(
                    &target,
                    FirmwareAuditStatus::NoRuntime,
                    if availability.requirement.is_empty() {
                        "No compatible emulator is installed for this platform.".to_owned()
                    } else {
                        format!("Install or configure one of: {}.", availability.requirement)
                    },
                )]);
            };
            let option = availability
                .options
                .get(selected_index)
                .context("the selected emulator option disappeared during the audit")?;
            let statuses = crate::firmware::statuses_for_options(
                catalog_database,
                &target.platform,
                &target.rom_path,
                std::slice::from_ref(option),
            )?
            .pop()
            .unwrap_or_default();
            if statuses.is_empty() {
                return Ok(vec![entry_for_no_firmware(&target, option.label())]);
            }
            Ok(statuses
                .into_iter()
                .map(|status| entry_for_status(&target, option.label(), status))
                .collect())
        })();

        match result {
            Ok(entries) if entries.len() == 1 && entries[0].package == NO_EXTERNAL_FIRMWARE => {
                output.no_firmware_platform_count =
                    output.no_firmware_platform_count.saturating_add(1);
                output.entries.extend(entries);
            }
            Ok(entries) => output.entries.extend(entries),
            Err(error) => output.entries.push(entry_for_runtime_problem(
                &target,
                FirmwareAuditStatus::Error,
                format!("Could not inspect this platform: {error:#}"),
            )),
        }
    }

    output.entries.sort_by(|left, right| {
        left.status
            .severity()
            .cmp(&right.status.severity())
            .then_with(|| {
                left.platform
                    .to_lowercase()
                    .cmp(&right.platform.to_lowercase())
            })
            .then_with(|| {
                left.emulator
                    .to_lowercase()
                    .cmp(&right.emulator.to_lowercase())
            })
            .then_with(|| {
                left.package
                    .to_lowercase()
                    .cmp(&right.package.to_lowercase())
            })
    });
    for entry in &output.entries {
        match entry.status {
            FirmwareAuditStatus::Missing => output.missing_count += 1,
            FirmwareAuditStatus::Manual => output.manual_count += 1,
            FirmwareAuditStatus::Optional => output.optional_count += 1,
            FirmwareAuditStatus::Ready => output.ready_count += 1,
            FirmwareAuditStatus::NoRuntime => output.no_runtime_count += 1,
            FirmwareAuditStatus::Error => output.error_count += 1,
        }
    }
    progress(FirmwareAuditProgress {
        current: inspection_count,
        total: inspection_count,
        platform: String::new(),
    });
    Ok(output)
}

fn entry_for_status(
    target: &LocalGameTarget,
    emulator: String,
    status: FirmwareStatus,
) -> FirmwareAuditEntry {
    let audit_status = classify_status(&status);
    let detail = match audit_status {
        FirmwareAuditStatus::Missing => {
            "Required firmware is not installed or synchronized for the selected emulator."
                .to_owned()
        }
        FirmwareAuditStatus::Manual => format!(
            "A user-supplied firmware dump is required. {}",
            status.notes.trim()
        ),
        FirmwareAuditStatus::Optional if status.supports_hle_fallback => {
            "The emulator can launch with HLE; installing this package may improve compatibility."
                .to_owned()
        }
        FirmwareAuditStatus::Optional => {
            "This optional firmware package is not currently installed.".to_owned()
        }
        FirmwareAuditStatus::Ready if status.target_strategy == "launch_scoped" => {
            "This firmware is resolved from the exact game layout at launch time.".to_owned()
        }
        FirmwareAuditStatus::Ready => {
            "The package is installed and ready for the selected emulator.".to_owned()
        }
        _ => status.notes.clone(),
    };
    let source = status.source_label().to_owned();
    FirmwareAuditEntry {
        game_uid: target.game_uid.clone(),
        launchbox_db_id: target.launchbox_db_id,
        title: target.title.clone(),
        platform: target.platform.clone(),
        emulator,
        package: status.package_name,
        source,
        status: audit_status,
        detail,
    }
}

fn entry_for_runtime_problem(
    target: &LocalGameTarget,
    status: FirmwareAuditStatus,
    detail: String,
) -> FirmwareAuditEntry {
    FirmwareAuditEntry {
        game_uid: target.game_uid.clone(),
        launchbox_db_id: target.launchbox_db_id,
        title: target.title.clone(),
        platform: target.platform.clone(),
        emulator: String::new(),
        package: String::new(),
        source: String::new(),
        status,
        detail,
    }
}

fn entry_for_no_firmware(target: &LocalGameTarget, emulator: String) -> FirmwareAuditEntry {
    FirmwareAuditEntry {
        game_uid: target.game_uid.clone(),
        launchbox_db_id: target.launchbox_db_id,
        title: target.title.clone(),
        platform: target.platform.clone(),
        emulator,
        package: NO_EXTERNAL_FIRMWARE.to_owned(),
        source: String::new(),
        status: FirmwareAuditStatus::Ready,
        detail: "No external BIOS or firmware package is defined for this emulator choice."
            .to_owned(),
    }
}

fn classify_status(status: &FirmwareStatus) -> FirmwareAuditStatus {
    if status.target_strategy == "manual_import" && !status.imported {
        FirmwareAuditStatus::Manual
    } else if status.needs_action() {
        FirmwareAuditStatus::Missing
    } else if !status.imported && (status.supports_hle_fallback || !status.required) {
        FirmwareAuditStatus::Optional
    } else {
        FirmwareAuditStatus::Ready
    }
}

fn load_local_targets(state_database: &Path) -> Result<(Vec<LocalGameTarget>, usize)> {
    if !state_database.is_file() {
        return Ok((Vec::new(), 0));
    }
    let connection = crate::catalog::open_read_only(state_database, "Lunchbox state database")?;
    let mut targets = Vec::new();
    let mut seen = HashSet::<String>::new();
    let mut unavailable = 0_usize;

    if table_exists(&connection, "installed_games")? {
        let mut statement = connection.prepare(
            "SELECT g.game_uid, g.launchbox_db_id, g.title, g.platform, g.file_path,
                    r.path_display, r.path_bytes, r.path_encoding
             FROM installed_games g
             LEFT JOIN installed_game_files f
               ON f.game_uid=g.game_uid AND f.is_primary=1
             LEFT JOIN installed_file_receipts r
               ON r.path_encoding=f.path_encoding AND r.path_bytes=f.path_bytes
             ORDER BY g.title COLLATE NOCASE, g.game_uid",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<Vec<u8>>>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })?;
        for row in rows {
            let (game_uid, database_id, title, platform, legacy_path, display, bytes, encoding) =
                row?;
            if !seen.insert(game_uid.clone()) {
                continue;
            }
            let path = match (display, bytes, encoding) {
                (Some(display), Some(bytes), Some(encoding)) => {
                    crate::local_import::decode_path(bytes, &encoding, display)
                }
                _ => PathBuf::from(legacy_path),
            };
            if !path.is_file() {
                unavailable = unavailable.saturating_add(1);
                continue;
            }
            targets.push(LocalGameTarget {
                game_uid,
                launchbox_db_id: database_id,
                title,
                platform,
                rom_path: path,
            });
        }
    }

    if table_exists(&connection, "local_rom_files")? {
        let mut statement = connection.prepare(
            "SELECT id, game_uid, launchbox_db_id, display_title, platform,
                    path_display, path_bytes, path_encoding
             FROM local_rom_files
             WHERE included=1 AND availability='present'
             ORDER BY display_title COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Vec<u8>>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;
        for row in rows {
            let (id, linked_uid, database_id, title, platform, display, bytes, encoding) = row?;
            let game_uid = linked_uid
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("local-file:{id}"));
            if !seen.insert(game_uid.clone()) {
                continue;
            }
            let path = crate::local_import::decode_path(bytes, &encoding, display);
            if !path.is_file() {
                unavailable = unavailable.saturating_add(1);
                continue;
            }
            targets.push(LocalGameTarget {
                game_uid,
                launchbox_db_id: database_id,
                title,
                platform,
                rom_path: path,
            });
        }
    }

    if seen.len() > MAX_LOCAL_GAMES {
        anyhow::bail!(
            "the local collection exceeds the {MAX_LOCAL_GAMES}-game firmware audit limit"
        );
    }
    Ok((targets, unavailable))
}

fn table_exists(connection: &rusqlite::Connection, name: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1",
            [name],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(
        required: bool,
        hle: bool,
        imported: bool,
        synced: bool,
        strategy: &str,
    ) -> FirmwareStatus {
        FirmwareStatus {
            rule_key: "rule".into(),
            source_id: "source".into(),
            source_transport: "manual".into(),
            source_url: String::new(),
            torrent_file: String::new(),
            path_prefix: String::new(),
            package_name: "bios.bin".into(),
            runtime_kind: "retroarch".into(),
            install_mode: "copy_archive".into(),
            required,
            supports_hle_fallback: hle,
            target_strategy: strategy.into(),
            imported,
            synced,
            runtime_path: String::new(),
            target_path: String::new(),
            notes: String::new(),
        }
    }

    #[test]
    fn required_manual_hle_and_ready_states_remain_distinct() {
        assert_eq!(
            classify_status(&status(true, false, false, false, "manual_import")),
            FirmwareAuditStatus::Manual
        );
        assert_eq!(
            classify_status(&status(true, false, false, false, "runtime_copy")),
            FirmwareAuditStatus::Missing
        );
        assert_eq!(
            classify_status(&status(true, true, false, false, "runtime_copy")),
            FirmwareAuditStatus::Optional
        );
        assert_eq!(
            classify_status(&status(true, false, true, true, "runtime_copy")),
            FirmwareAuditStatus::Ready
        );
    }
}
