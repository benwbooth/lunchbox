use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use uuid::Uuid;

use crate::catalog;
use crate::local_import::{self, MatchIdentity};
use crate::settings::{SettingsStore, unix_timestamp};

const MANUAL_MATCH_METHOD: &str = "manual user selection";
const MAX_IDENTITY_OPERATION_FILES: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedGameFile {
    pub id: String,
    pub path_display: String,
    pub archive_member: String,
    pub file_name: String,
    pub display_title: String,
    pub platform: String,
    pub file_size: u64,
    pub sha1: String,
    pub md5: String,
    pub game_uid: Option<String>,
    pub launchbox_db_id: i64,
    pub matched_title: Option<String>,
    pub match_state: String,
    pub match_method: String,
    pub availability: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityEvent {
    pub id: String,
    pub kind: String,
    pub operation_id: Option<String>,
    pub operation_kind: String,
    pub operation_item_count: usize,
    pub file_id: String,
    pub file_path_display: String,
    pub archive_member: String,
    pub from_game_uid: Option<String>,
    pub from_title: Option<String>,
    pub from_platform: String,
    pub to_game_uid: Option<String>,
    pub to_title: Option<String>,
    pub to_platform: String,
    pub reverts_event_id: Option<String>,
    pub occurred_at: i64,
    pub reverted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityOperation {
    pub id: String,
    pub kind: String,
    pub events: Vec<IdentityEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredIdentity {
    game_uid: Option<String>,
    launchbox_db_id: i64,
    title: Option<String>,
    platform: String,
    match_state: String,
    match_method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredEvent {
    event: IdentityEvent,
    file_size: Option<u64>,
    sha1: String,
    md5: String,
    from: StoredIdentity,
    to: StoredIdentity,
}

pub fn imported_files_for_game(state_path: &Path, game_uid: &str) -> Result<Vec<ImportedGameFile>> {
    validate_game_uid(game_uid)?;
    let store = SettingsStore::at(state_path)?;
    let connection = store.connection()?;
    let mut statement = connection.prepare(
        "SELECT id, path_display, archive_member, file_name, display_title, platform,
                file_size, sha1, md5, game_uid, launchbox_db_id, matched_title,
                match_state, match_method, availability
         FROM local_rom_files
         WHERE game_uid=?1 AND included=1
         ORDER BY availability='missing', file_name COLLATE NOCASE, archive_member COLLATE NOCASE, id",
    )?;
    statement
        .query_map([game_uid], imported_file_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("loading imported ROM identities")
}

pub fn search_targets(
    discovery_path: &Path,
    query: &str,
    platform: &str,
) -> Result<Vec<local_import::ManualMatchCandidate>> {
    local_import::search_manual_match_candidates(discovery_path, query, platform)
}

#[cfg(test)]
pub fn relink_imported_file(
    state_path: &Path,
    discovery_path: &Path,
    file_id: &str,
    expected_game_uid: &str,
    target_game_uid: &str,
) -> Result<IdentityEvent> {
    let operation = relink_imported_files(
        state_path,
        discovery_path,
        &[file_id.to_owned()],
        expected_game_uid,
        target_game_uid,
    )?;
    operation
        .events
        .into_iter()
        .next()
        .context("the ROM identity operation produced no event")
}

pub fn relink_imported_files(
    state_path: &Path,
    discovery_path: &Path,
    file_ids: &[String],
    expected_game_uid: &str,
    target_game_uid: &str,
) -> Result<IdentityOperation> {
    if file_ids.is_empty() {
        bail!("select at least one imported ROM identity");
    }
    if file_ids.len() > MAX_IDENTITY_OPERATION_FILES {
        bail!(
            "one identity operation may contain at most {MAX_IDENTITY_OPERATION_FILES} imported ROMs"
        );
    }
    let mut unique_file_ids = HashSet::with_capacity(file_ids.len());
    for file_id in file_ids {
        validate_uuid(file_id, "imported file identity")?;
        if !unique_file_ids.insert(file_id.as_str()) {
            bail!("the identity operation repeats an imported ROM");
        }
    }
    validate_game_uid(expected_game_uid)?;
    validate_game_uid(target_game_uid)?;
    if expected_game_uid == target_game_uid {
        bail!("this ROM is already linked to the selected catalog game");
    }
    let target = load_exact_catalog_target(discovery_path, target_game_uid)?;
    let store = SettingsStore::at(state_path)?;
    let mut connection = store.connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let source_file_count: usize = transaction
        .query_row(
            "SELECT count(*) FROM local_rom_files WHERE game_uid=?1 AND included=1",
            [expected_game_uid],
            |row| row.get::<_, i64>(0),
        )?
        .try_into()
        .context("the imported ROM count is too large")?;
    let target_file_count: usize = transaction
        .query_row(
            "SELECT count(*) FROM local_rom_files WHERE game_uid=?1 AND included=1",
            [target_game_uid],
            |row| row.get::<_, i64>(0),
        )?
        .try_into()
        .context("the target ROM count is too large")?;
    let operation_kind = if file_ids.len() < source_file_count {
        "split"
    } else if file_ids.len() > 1 || target_file_count > 0 {
        "merge"
    } else {
        "relink"
    };
    let operation_id = Uuid::new_v4().to_string();
    let mut current_files = Vec::with_capacity(file_ids.len());
    for file_id in file_ids {
        let current = transaction
            .query_row(
                "SELECT id, path_display, archive_member, file_name, display_title, platform,
                        file_size, sha1, md5, game_uid, launchbox_db_id, matched_title,
                        match_state, match_method, availability
                 FROM local_rom_files WHERE id=?1 AND included=1",
                [file_id],
                imported_file_from_row,
            )
            .optional()?
            .context("an imported ROM is no longer available for identity maintenance")?;
        if current.game_uid.as_deref() != Some(expected_game_uid) {
            bail!(
                "an imported ROM identity changed; reopen the manager before changing this group"
            );
        }
        current_files.push(current);
    }
    let to = StoredIdentity {
        game_uid: Some(target.game_uid.clone()),
        launchbox_db_id: target.launchbox_db_id,
        title: Some(target.title.clone()),
        platform: target.platform.clone(),
        match_state: "reviewed".to_owned(),
        match_method: MANUAL_MATCH_METHOD.to_owned(),
    };
    let mut events = Vec::with_capacity(current_files.len());
    for current in &current_files {
        let from = identity_from_file(current);
        let changed = transaction.execute(
            "UPDATE local_rom_files
             SET game_uid=?2, launchbox_db_id=?3, matched_title=?4, platform=?5,
                 match_state=?6, match_method=?7
             WHERE id=?1 AND game_uid=?8",
            params![
                current.id,
                to.game_uid,
                to.launchbox_db_id,
                to.title,
                to.platform,
                to.match_state,
                to.match_method,
                expected_game_uid,
            ],
        )?;
        if changed != 1 {
            bail!("an imported ROM identity changed before the group could be committed");
        }
        events.push(insert_event(
            &transaction,
            "relink",
            current,
            &from,
            &to,
            None,
            Some(&operation_id),
            operation_kind,
            current_files.len(),
        )?);
    }
    transaction.commit()?;
    Ok(IdentityOperation {
        id: operation_id,
        kind: operation_kind.to_owned(),
        events,
    })
}

pub fn identity_history_for_file(state_path: &Path, file_id: &str) -> Result<Vec<IdentityEvent>> {
    validate_uuid(file_id, "imported file identity")?;
    let store = SettingsStore::at(state_path)?;
    let connection = store.connection()?;
    let mut statement = connection.prepare(
        "SELECT e.id, e.event_kind, e.file_id, e.file_path_display, e.archive_member,
                e.from_game_uid, e.from_title, e.from_platform,
                e.to_game_uid, e.to_title, e.to_platform, e.reverts_event_id,
                e.occurred_at,
                EXISTS(SELECT 1 FROM collection_identity_events u
                       WHERE u.reverts_event_id=e.id),
                e.operation_id, coalesce(e.operation_kind, e.event_kind),
                CASE WHEN e.operation_id IS NULL THEN 1 ELSE (
                    SELECT count(*) FROM collection_identity_events grouped
                    WHERE grouped.operation_id=e.operation_id
                      AND grouped.event_kind=e.event_kind
                ) END
         FROM collection_identity_events e
         WHERE e.file_id=?1
         ORDER BY e.sequence DESC LIMIT 100",
    )?;
    statement
        .query_map([file_id], event_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("loading ROM identity history")
}

pub fn undo_relink(state_path: &Path, event_id: &str) -> Result<IdentityEvent> {
    validate_uuid(event_id, "identity event")?;
    let store = SettingsStore::at(state_path)?;
    let mut connection = store.connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let stored = load_stored_event(&transaction, event_id)?
        .context("the identity event no longer exists")?;
    if stored.event.kind != "relink" || stored.event.reverts_event_id.is_some() {
        bail!("only an original ROM relink can be undone");
    }
    let stored_events = if let Some(operation_id) = stored.event.operation_id.as_deref() {
        load_stored_operation(&transaction, operation_id)?
    } else {
        vec![stored]
    };
    if stored_events.is_empty() {
        bail!("the ROM identity operation no longer has any events");
    }
    let mut current_files = Vec::with_capacity(stored_events.len());
    for item in &stored_events {
        if item.event.kind != "relink" || item.event.reverts_event_id.is_some() {
            bail!("only an original ROM identity operation can be undone");
        }
        let already_reverted: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM collection_identity_events WHERE reverts_event_id=?1)",
            [&item.event.id],
            |row| row.get(0),
        )?;
        if already_reverted {
            bail!("this ROM identity operation has already been undone");
        }
        let current = transaction
            .query_row(
                "SELECT id, path_display, archive_member, file_name, display_title, platform,
                        file_size, sha1, md5, game_uid, launchbox_db_id, matched_title,
                        match_state, match_method, availability
                 FROM local_rom_files WHERE id=?1 AND included=1",
                [&item.event.file_id],
                imported_file_from_row,
            )
            .optional()?
            .context(
                "an imported ROM no longer exists, so the identity operation cannot be undone",
            )?;
        if current.path_display != item.event.file_path_display
            || current.archive_member != item.event.archive_member
            || item.file_size.is_some_and(|size| current.file_size != size)
            || current.sha1 != item.sha1
            || current.md5 != item.md5
            || identity_from_file(&current) != item.to
        {
            bail!(
                "an imported ROM changed after this identity operation; undo was stopped before changing any records"
            );
        }
        current_files.push(current);
    }
    let undo_operation_id = Uuid::new_v4().to_string();
    let mut undo_events = Vec::with_capacity(stored_events.len());
    for (item, current) in stored_events.iter().zip(&current_files) {
        let changed = transaction.execute(
            "UPDATE local_rom_files
             SET game_uid=?2, launchbox_db_id=?3, matched_title=?4, platform=?5,
                 match_state=?6, match_method=?7
             WHERE id=?1",
            params![
                current.id,
                item.from.game_uid,
                item.from.launchbox_db_id,
                item.from.title,
                item.from.platform,
                item.from.match_state,
                item.from.match_method,
            ],
        )?;
        if changed != 1 {
            bail!("an imported ROM disappeared before undo could be committed");
        }
        undo_events.push(insert_event(
            &transaction,
            "undo",
            current,
            &item.to,
            &item.from,
            Some(&item.event.id),
            Some(&undo_operation_id),
            "undo",
            stored_events.len(),
        )?);
    }
    transaction.commit()?;
    let requested_index = stored_events
        .iter()
        .position(|item| item.event.id == event_id)
        .unwrap_or_default();
    undo_events
        .into_iter()
        .nth(requested_index)
        .context("the ROM identity undo produced no event")
}

fn load_exact_catalog_target(discovery_path: &Path, game_uid: &str) -> Result<MatchIdentity> {
    let connection = catalog::open_read_only(discovery_path, "Lunchbox discovery database")?;
    connection
        .query_row(
            "SELECT g.id, g.title, p.name, coalesce(g.launchbox_db_id, 0)
             FROM games g JOIN platforms p ON p.id=g.platform_id
             WHERE g.id=?1",
            [game_uid],
            |row| {
                Ok(MatchIdentity {
                    game_uid: row.get(0)?,
                    title: row.get(1)?,
                    platform: row.get(2)?,
                    launchbox_db_id: row.get(3)?,
                })
            },
        )
        .optional()?
        .context("the selected catalog game no longer exists")
}

fn imported_file_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ImportedGameFile> {
    let file_size: i64 = row.get(6)?;
    Ok(ImportedGameFile {
        id: row.get(0)?,
        path_display: row.get(1)?,
        archive_member: row.get(2)?,
        file_name: row.get(3)?,
        display_title: row.get(4)?,
        platform: row.get(5)?,
        file_size: u64::try_from(file_size).unwrap_or_default(),
        sha1: row.get(7)?,
        md5: row.get(8)?,
        game_uid: row.get(9)?,
        launchbox_db_id: row.get(10)?,
        matched_title: row.get(11)?,
        match_state: row.get(12)?,
        match_method: row.get(13)?,
        availability: row.get(14)?,
    })
}

fn identity_from_file(file: &ImportedGameFile) -> StoredIdentity {
    StoredIdentity {
        game_uid: file.game_uid.clone(),
        launchbox_db_id: file.launchbox_db_id,
        title: file.matched_title.clone(),
        platform: file.platform.clone(),
        match_state: file.match_state.clone(),
        match_method: file.match_method.clone(),
    }
}

fn insert_event(
    transaction: &rusqlite::Transaction<'_>,
    kind: &str,
    file: &ImportedGameFile,
    from: &StoredIdentity,
    to: &StoredIdentity,
    reverts_event_id: Option<&str>,
    operation_id: Option<&str>,
    operation_kind: &str,
    operation_item_count: usize,
) -> Result<IdentityEvent> {
    let id = Uuid::new_v4().to_string();
    let occurred_at = unix_timestamp();
    let file_size = i64::try_from(file.file_size).context("the imported ROM is too large")?;
    transaction.execute(
        "INSERT INTO collection_identity_events (
             id, event_kind, operation_id, operation_kind,
             file_id, file_path_display, archive_member, file_size, sha1, md5,
             from_game_uid, from_launchbox_db_id, from_title, from_platform,
             from_match_state, from_match_method,
             to_game_uid, to_launchbox_db_id, to_title, to_platform,
             to_match_state, to_match_method, reverts_event_id, occurred_at
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
             ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
         )",
        params![
            id,
            kind,
            operation_id,
            operation_kind,
            file.id,
            file.path_display,
            file.archive_member,
            file_size,
            file.sha1,
            file.md5,
            from.game_uid,
            from.launchbox_db_id,
            from.title,
            from.platform,
            from.match_state,
            from.match_method,
            to.game_uid,
            to.launchbox_db_id,
            to.title,
            to.platform,
            to.match_state,
            to.match_method,
            reverts_event_id,
            occurred_at,
        ],
    )?;
    Ok(IdentityEvent {
        id,
        kind: kind.to_owned(),
        operation_id: operation_id.map(str::to_owned),
        operation_kind: operation_kind.to_owned(),
        operation_item_count,
        file_id: file.id.clone(),
        file_path_display: file.path_display.clone(),
        archive_member: file.archive_member.clone(),
        from_game_uid: from.game_uid.clone(),
        from_title: from.title.clone(),
        from_platform: from.platform.clone(),
        to_game_uid: to.game_uid.clone(),
        to_title: to.title.clone(),
        to_platform: to.platform.clone(),
        reverts_event_id: reverts_event_id.map(str::to_owned),
        occurred_at,
        reverted: false,
    })
}

fn load_stored_event(
    transaction: &rusqlite::Transaction<'_>,
    event_id: &str,
) -> Result<Option<StoredEvent>> {
    transaction
        .query_row(
            "SELECT id, event_kind, file_id, file_path_display, archive_member, file_size, sha1, md5,
                    from_game_uid, from_launchbox_db_id, from_title, from_platform,
                    from_match_state, from_match_method,
                    to_game_uid, to_launchbox_db_id, to_title, to_platform,
                    to_match_state, to_match_method, reverts_event_id, occurred_at,
                    operation_id, coalesce(operation_kind, event_kind),
                    CASE WHEN operation_id IS NULL THEN 1 ELSE (
                        SELECT count(*) FROM collection_identity_events grouped
                        WHERE grouped.operation_id=collection_identity_events.operation_id
                          AND grouped.event_kind=collection_identity_events.event_kind
                    ) END
             FROM collection_identity_events WHERE id=?1",
            [event_id],
            stored_event_from_row,
        )
        .optional()
        .context("loading the exact ROM identity event")
}

fn load_stored_operation(
    transaction: &rusqlite::Transaction<'_>,
    operation_id: &str,
) -> Result<Vec<StoredEvent>> {
    validate_uuid(operation_id, "identity operation")?;
    let mut statement = transaction.prepare(
        "SELECT id, event_kind, file_id, file_path_display, archive_member, file_size, sha1, md5,
                from_game_uid, from_launchbox_db_id, from_title, from_platform,
                from_match_state, from_match_method,
                to_game_uid, to_launchbox_db_id, to_title, to_platform,
                to_match_state, to_match_method, reverts_event_id, occurred_at,
                operation_id, coalesce(operation_kind, event_kind),
                (SELECT count(*) FROM collection_identity_events grouped
                 WHERE grouped.operation_id=collection_identity_events.operation_id
                   AND grouped.event_kind=collection_identity_events.event_kind)
         FROM collection_identity_events
         WHERE operation_id=?1 AND event_kind='relink'
         ORDER BY sequence",
    )?;
    statement
        .query_map([operation_id], stored_event_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("loading the complete ROM identity operation")
}

fn stored_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredEvent> {
    let kind: String = row.get(1)?;
    let reverts_event_id: Option<String> = row.get(20)?;
    let file_size = row
        .get::<_, Option<i64>>(5)?
        .and_then(|size| size.try_into().ok());
    Ok(StoredEvent {
        event: IdentityEvent {
            id: row.get(0)?,
            kind,
            operation_id: row.get(22)?,
            operation_kind: row.get(23)?,
            operation_item_count: row.get::<_, i64>(24)?.try_into().unwrap_or_default(),
            file_id: row.get(2)?,
            file_path_display: row.get(3)?,
            archive_member: row.get(4)?,
            from_game_uid: row.get(8)?,
            from_title: row.get(10)?,
            from_platform: row.get(11)?,
            to_game_uid: row.get(14)?,
            to_title: row.get(16)?,
            to_platform: row.get(17)?,
            reverts_event_id,
            occurred_at: row.get(21)?,
            reverted: false,
        },
        file_size,
        sha1: row.get(6)?,
        md5: row.get(7)?,
        from: StoredIdentity {
            game_uid: row.get(8)?,
            launchbox_db_id: row.get(9)?,
            title: row.get(10)?,
            platform: row.get(11)?,
            match_state: row.get(12)?,
            match_method: row.get(13)?,
        },
        to: StoredIdentity {
            game_uid: row.get(14)?,
            launchbox_db_id: row.get(15)?,
            title: row.get(16)?,
            platform: row.get(17)?,
            match_state: row.get(18)?,
            match_method: row.get(19)?,
        },
    })
}

fn event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IdentityEvent> {
    Ok(IdentityEvent {
        id: row.get(0)?,
        kind: row.get(1)?,
        operation_id: row.get(14)?,
        operation_kind: row.get(15)?,
        operation_item_count: row.get::<_, i64>(16)?.try_into().unwrap_or_default(),
        file_id: row.get(2)?,
        file_path_display: row.get(3)?,
        archive_member: row.get(4)?,
        from_game_uid: row.get(5)?,
        from_title: row.get(6)?,
        from_platform: row.get(7)?,
        to_game_uid: row.get(8)?,
        to_title: row.get(9)?,
        to_platform: row.get(10)?,
        reverts_event_id: row.get(11)?,
        occurred_at: row.get(12)?,
        reverted: row.get(13)?,
    })
}

fn validate_uuid(value: &str, label: &str) -> Result<()> {
    Uuid::parse_str(value).with_context(|| format!("{label} must be a UUID"))?;
    Ok(())
}

fn validate_game_uid(game_uid: &str) -> Result<()> {
    validate_uuid(game_uid, "stable game identity")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    const SOURCE_ID: &str = "11111111-1111-4111-8111-111111111111";
    const TARGET_ID: &str = "22222222-2222-4222-8222-222222222222";
    const THIRD_ID: &str = "33333333-3333-4333-8333-333333333333";
    const FILE_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    const SECOND_FILE_ID: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
    const ROOT_ID: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path().join("state.db");
        let discovery = directory.path().join("games.db");
        let store = SettingsStore::at(&state).unwrap();
        let connection = store.connection().unwrap();
        connection
            .execute(
                "INSERT INTO local_collection_roots (
                 id, path_display, path_bytes, path_encoding, platform_hint,
                 checksums_enabled, last_scanned_at
             ) VALUES (?1, '/roms', X'2F726F6D73', 'unix_bytes', 'Source Platform', 1, 1)",
                [ROOT_ID],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO local_rom_files (
                 id, root_id, path_display, path_bytes, path_encoding,
                 relative_path_display, relative_path_bytes, relative_path_encoding,
                 file_name, archive_member, display_title, platform, file_size,
                 modified_unix_ns, crc32, md5, sha1, game_uid, launchbox_db_id,
                 matched_title, match_state, match_method, included, availability, imported_at
             ) VALUES (
                 ?1, ?2, '/roms/game.bin', X'2F726F6D732F67616D652E62696E', 'unix_bytes',
                 'game.bin', X'67616D652E62696E', 'unix_bytes', 'game.bin', '',
                 'Source Game', 'Source Platform', 4096, 1, '12345678',
                 '11111111111111111111111111111111',
                 '1111111111111111111111111111111111111111', ?3, 10,
                 'Source Game', 'exact', 'SHA-1 + MD5', 1, 'present', 1
             )",
                params![FILE_ID, ROOT_ID, SOURCE_ID],
            )
            .unwrap();

        let games = Connection::open(&discovery).unwrap();
        games
            .execute_batch(
                "CREATE TABLE platforms (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE games (
                 id TEXT PRIMARY KEY, title TEXT NOT NULL, platform_id INTEGER NOT NULL,
                 launchbox_db_id INTEGER
             );
             CREATE TABLE game_alternate_names (
                 id INTEGER PRIMARY KEY, launchbox_db_id INTEGER,
                 alternate_name TEXT, region TEXT
             );
             INSERT INTO platforms VALUES (1, 'Source Platform');
             INSERT INTO platforms VALUES (2, 'Target Platform');",
            )
            .unwrap();
        games
            .execute(
                "INSERT INTO games VALUES (?1, 'Source Game', 1, 10)",
                [SOURCE_ID],
            )
            .unwrap();
        games
            .execute(
                "INSERT INTO games VALUES (?1, 'Target Game', 2, 20)",
                [TARGET_ID],
            )
            .unwrap();
        games
            .execute(
                "INSERT INTO games VALUES (?1, 'Third Game', 2, 30)",
                [THIRD_ID],
            )
            .unwrap();
        drop(games);
        (directory, state, discovery)
    }

    fn insert_second_source_file(state: &Path) {
        let store = SettingsStore::at(state).unwrap();
        store
            .connection()
            .unwrap()
            .execute(
                "INSERT INTO local_rom_files (
                 id, root_id, path_display, path_bytes, path_encoding,
                 relative_path_display, relative_path_bytes, relative_path_encoding,
                 file_name, archive_member, display_title, platform, file_size,
                 modified_unix_ns, crc32, md5, sha1, game_uid, launchbox_db_id,
                 matched_title, match_state, match_method, included, availability, imported_at
             ) VALUES (
                 ?1, ?2, '/roms/game-alt.bin', X'2F726F6D732F67616D652D616C742E62696E', 'unix_bytes',
                 'game-alt.bin', X'67616D652D616C742E62696E', 'unix_bytes', 'game-alt.bin', '',
                 'Source Game Alt', 'Source Platform', 8192, 2, '87654321',
                 '22222222222222222222222222222222',
                 '2222222222222222222222222222222222222222', ?3, 10,
                 'Source Game', 'exact', 'SHA-1 + MD5', 1, 'present', 2
             )",
                params![SECOND_FILE_ID, ROOT_ID, SOURCE_ID],
            )
            .unwrap();
    }

    #[test]
    fn relink_preserves_rom_evidence_and_writes_auditable_event() {
        let (_directory, state, discovery) = fixture();
        let before = imported_files_for_game(&state, SOURCE_ID)
            .unwrap()
            .remove(0);
        let event =
            relink_imported_file(&state, &discovery, FILE_ID, SOURCE_ID, TARGET_ID).unwrap();
        let after = imported_files_for_game(&state, TARGET_ID)
            .unwrap()
            .remove(0);
        assert_eq!(event.from_game_uid.as_deref(), Some(SOURCE_ID));
        assert_eq!(event.to_game_uid.as_deref(), Some(TARGET_ID));
        assert_eq!(after.path_display, before.path_display);
        assert_eq!(after.sha1, before.sha1);
        assert_eq!(after.md5, before.md5);
        assert_eq!(after.file_size, before.file_size);
        assert_eq!(after.platform, "Target Platform");
        assert_eq!(after.matched_title.as_deref(), Some("Target Game"));
        assert_eq!(after.match_state, "reviewed");
        assert_eq!(after.match_method, MANUAL_MATCH_METHOD);
        assert_eq!(
            identity_history_for_file(&state, FILE_ID).unwrap(),
            vec![event]
        );
    }

    #[test]
    fn undo_restores_exact_prior_identity_and_cannot_repeat() {
        let (_directory, state, discovery) = fixture();
        let relink =
            relink_imported_file(&state, &discovery, FILE_ID, SOURCE_ID, TARGET_ID).unwrap();
        let undo = undo_relink(&state, &relink.id).unwrap();
        let restored = imported_files_for_game(&state, SOURCE_ID)
            .unwrap()
            .remove(0);
        assert_eq!(restored.platform, "Source Platform");
        assert_eq!(restored.matched_title.as_deref(), Some("Source Game"));
        assert_eq!(restored.match_state, "exact");
        assert_eq!(restored.match_method, "SHA-1 + MD5");
        assert_eq!(undo.reverts_event_id.as_deref(), Some(relink.id.as_str()));
        assert!(
            undo_relink(&state, &relink.id)
                .unwrap_err()
                .to_string()
                .contains("already")
        );
        let history = identity_history_for_file(&state, FILE_ID).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].kind, "undo");
        assert!(history[1].reverted);
    }

    #[test]
    fn stale_state_stops_relink_and_undo_without_overwriting_newer_work() {
        let (_directory, state, discovery) = fixture();
        assert!(
            relink_imported_file(&state, &discovery, FILE_ID, TARGET_ID, THIRD_ID)
                .unwrap_err()
                .to_string()
                .contains("changed")
        );
        let relink =
            relink_imported_file(&state, &discovery, FILE_ID, SOURCE_ID, TARGET_ID).unwrap();
        let store = SettingsStore::at(&state).unwrap();
        store
            .connection()
            .unwrap()
            .execute(
                "UPDATE local_rom_files
             SET game_uid=?2, launchbox_db_id=30, matched_title='Third Game'
             WHERE id=?1",
                params![FILE_ID, THIRD_ID],
            )
            .unwrap();
        assert!(
            undo_relink(&state, &relink.id)
                .unwrap_err()
                .to_string()
                .contains("changed")
        );
        assert_eq!(imported_files_for_game(&state, THIRD_ID).unwrap().len(), 1);
    }

    #[test]
    fn grouped_merge_and_undo_are_atomic_and_preserve_every_file() {
        let (_directory, state, discovery) = fixture();
        insert_second_source_file(&state);
        let before = imported_files_for_game(&state, SOURCE_ID).unwrap();
        let operation = relink_imported_files(
            &state,
            &discovery,
            &[FILE_ID.to_owned(), SECOND_FILE_ID.to_owned()],
            SOURCE_ID,
            TARGET_ID,
        )
        .unwrap();
        assert_eq!(operation.kind, "merge");
        assert_eq!(operation.events.len(), 2);
        assert!(
            operation
                .events
                .iter()
                .all(|event| event.operation_id.as_deref() == Some(operation.id.as_str()))
        );
        assert!(
            operation
                .events
                .iter()
                .all(|event| event.operation_item_count == 2)
        );
        let merged = imported_files_for_game(&state, TARGET_ID).unwrap();
        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged
                .iter()
                .map(|file| (&file.id, &file.sha1))
                .collect::<Vec<_>>(),
            before
                .iter()
                .map(|file| (&file.id, &file.sha1))
                .collect::<Vec<_>>()
        );

        let undo = undo_relink(&state, &operation.events[1].id).unwrap();
        assert_eq!(undo.operation_kind, "undo");
        assert_eq!(undo.operation_item_count, 2);
        assert_eq!(imported_files_for_game(&state, SOURCE_ID).unwrap().len(), 2);
        assert!(
            imported_files_for_game(&state, TARGET_ID)
                .unwrap()
                .is_empty()
        );
        assert!(
            undo_relink(&state, &operation.events[0].id)
                .unwrap_err()
                .to_string()
                .contains("already")
        );
    }

    #[test]
    fn grouped_split_and_undo_fail_closed_before_any_partial_change() {
        let (_directory, state, discovery) = fixture();
        insert_second_source_file(&state);
        let split = relink_imported_files(
            &state,
            &discovery,
            &[FILE_ID.to_owned()],
            SOURCE_ID,
            TARGET_ID,
        )
        .unwrap();
        assert_eq!(split.kind, "split");
        assert_eq!(imported_files_for_game(&state, SOURCE_ID).unwrap().len(), 1);
        assert_eq!(imported_files_for_game(&state, TARGET_ID).unwrap().len(), 1);
        undo_relink(&state, &split.events[0].id).unwrap();
        assert_eq!(imported_files_for_game(&state, SOURCE_ID).unwrap().len(), 2);

        let merge = relink_imported_files(
            &state,
            &discovery,
            &[FILE_ID.to_owned(), SECOND_FILE_ID.to_owned()],
            SOURCE_ID,
            TARGET_ID,
        )
        .unwrap();
        SettingsStore::at(&state)
            .unwrap()
            .connection()
            .unwrap()
            .execute(
                "UPDATE local_rom_files
                 SET game_uid=?2, launchbox_db_id=30, matched_title='Third Game'
                 WHERE id=?1",
                params![SECOND_FILE_ID, THIRD_ID],
            )
            .unwrap();
        let error = undo_relink(&state, &merge.events[0].id).unwrap_err();
        assert!(error.to_string().contains("before changing any records"));
        assert_eq!(imported_files_for_game(&state, SOURCE_ID).unwrap().len(), 0);
        assert_eq!(imported_files_for_game(&state, TARGET_ID).unwrap().len(), 1);
        assert_eq!(imported_files_for_game(&state, THIRD_ID).unwrap().len(), 1);
    }
}
