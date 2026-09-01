use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use fs2::FileExt;
use rusqlite::{OptionalExtension, Transaction, params};

use crate::media::ArtworkKind;
use crate::settings::SettingsStore;

const MAX_MEDIA_REPAIR_ITEMS: usize = 350_000;
const MAX_MEDIA_REPAIR_HISTORY: usize = 20;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaRepairTarget {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub local: bool,
    pub downloadable: bool,
    pub artwork_kind: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaRepairItem {
    pub position: usize,
    pub target: MediaRepairTarget,
    pub status: String,
    pub outcome_detail: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaRepairBatchSnapshot {
    pub id: String,
    pub scope: String,
    pub artwork_kind: String,
    pub status: String,
    pub detail: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub items: Vec<MediaRepairItem>,
}

impl MediaRepairBatchSnapshot {
    pub fn unfinished_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| matches!(item.status.as_str(), "pending" | "running"))
            .count()
    }

    pub fn succeeded_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.status == "succeeded")
            .count()
    }
}

#[derive(Debug)]
pub struct RecoveredMediaRepairBatch {
    pub snapshot: MediaRepairBatchSnapshot,
    pub recovered_running_items: usize,
}

pub struct MediaRepairLease {
    file: File,
}

impl Drop for MediaRepairLease {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

pub fn try_acquire_media_repair_lease(store: &SettingsStore) -> Result<Option<MediaRepairLease>> {
    let path = media_repair_lock_path(store);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "creating missing-media repair lock directory {}",
                parent.display()
            )
        })?;
    }
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("opening missing-media repair lock {}", path.display()))?;
    match FileExt::try_lock_exclusive(&file) {
        Ok(()) => Ok(Some(MediaRepairLease { file })),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error).context("locking the missing-media repair service"),
    }
}

fn media_repair_lock_path(store: &SettingsStore) -> PathBuf {
    store
        .path()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("media-repair.lock")
}

impl SettingsStore {
    pub fn begin_media_repair_batch(
        &self,
        batch_id: &str,
        scope: &str,
        artwork_kind: &str,
        targets: &[MediaRepairTarget],
    ) -> Result<MediaRepairBatchSnapshot> {
        validate_batch_identity(batch_id, scope, artwork_kind)?;
        if targets.is_empty() || targets.len() > MAX_MEDIA_REPAIR_ITEMS {
            bail!(
                "a missing-media repair batch must contain 1 to {MAX_MEDIA_REPAIR_ITEMS} exact records"
            );
        }
        let mut game_uids = HashSet::with_capacity(targets.len());
        for target in targets {
            validate_target(target)?;
            if target.artwork_kind != artwork_kind {
                bail!("every missing-media repair item must use the batch media category");
            }
            if !game_uids.insert(target.game_uid.as_str()) {
                bail!(
                    "missing-media repair batch contains duplicate game {}",
                    target.game_uid
                );
            }
        }

        let now = unix_timestamp();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO media_repair_batches (
                 id, scope, artwork_kind, status, detail, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'running', '', ?4, ?4)",
            params![batch_id, scope, artwork_kind, now],
        )?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO media_repair_items (
                     batch_id, position, game_uid, launchbox_db_id, title,
                     platform, local, downloadable, artwork_kind, request_detail,
                     status, outcome_detail, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', '', ?11)",
            )?;
            for (position, target) in targets.iter().enumerate() {
                statement.execute(params![
                    batch_id,
                    i64::try_from(position).context("media repair position overflow")?,
                    target.game_uid,
                    target.launchbox_db_id,
                    target.title,
                    target.platform,
                    target.local,
                    target.downloadable,
                    target.artwork_kind,
                    target.detail,
                    now,
                ])?;
            }
        }
        prune_media_repair_history(&transaction)?;
        transaction.commit()?;
        self.media_repair_batch(batch_id)?
            .context("new missing-media repair batch disappeared")
    }

    pub fn active_media_repair_batch(&self) -> Result<Option<MediaRepairBatchSnapshot>> {
        let connection = self.connection()?;
        let id = connection
            .query_row(
                "SELECT id FROM media_repair_batches
                 WHERE status IN ('pending', 'running')
                 ORDER BY created_at DESC, rowid DESC
                 LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(id
            .map(|id| load_media_repair_batch(&connection, &id))
            .transpose()?)
    }

    pub fn recover_interrupted_media_repair_batch(
        &self,
    ) -> Result<Option<RecoveredMediaRepairBatch>> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let Some((batch_id, status)) = transaction
            .query_row(
                "SELECT id, status FROM media_repair_batches
                 WHERE status IN ('pending', 'running')
                 ORDER BY created_at DESC, rowid DESC
                 LIMIT 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
        else {
            return Ok(None);
        };

        let recovered_running_items = transaction.execute(
            "UPDATE media_repair_items
             SET status='pending',
                 outcome_detail='Lunchbox closed before this exact repair reported a result.',
                 updated_at=?2
             WHERE batch_id=?1 AND status='running'",
            params![batch_id, unix_timestamp()],
        )?;
        let unfinished: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM media_repair_items
             WHERE batch_id=?1 AND status IN ('pending', 'running')",
            [&batch_id],
            |row| row.get(0),
        )?;
        if unfinished == 0 {
            transaction.execute(
                "UPDATE media_repair_batches
                 SET status='completed',
                     detail='Every exact repair finished before restart.',
                     updated_at=?2
                 WHERE id=?1 AND status IN ('pending', 'running')",
                params![batch_id, unix_timestamp()],
            )?;
            transaction.commit()?;
            return Ok(None);
        }
        if status == "running" || recovered_running_items > 0 {
            transaction.execute(
                "UPDATE media_repair_batches
                 SET status='pending',
                     detail='Lunchbox closed before the batch finished. Review and resume the exact remaining records.',
                     updated_at=?2
                 WHERE id=?1 AND status IN ('pending', 'running')",
                params![batch_id, unix_timestamp()],
            )?;
        }
        transaction.commit()?;
        let snapshot = self
            .media_repair_batch(&batch_id)?
            .context("recovered missing-media repair batch disappeared")?;
        Ok(Some(RecoveredMediaRepairBatch {
            snapshot,
            recovered_running_items,
        }))
    }

    pub fn claim_media_repair_batch(&self, batch_id: &str) -> Result<MediaRepairBatchSnapshot> {
        uuid::Uuid::parse_str(batch_id).context("media repair batches require a UUID identity")?;
        let now = unix_timestamp();
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE media_repair_batches
             SET status='running', detail='', updated_at=?2
             WHERE id=?1 AND status='pending'
               AND EXISTS (
                   SELECT 1 FROM media_repair_items
                   WHERE batch_id=?1 AND status='pending'
               )",
            params![batch_id, now],
        )?;
        if changed != 1 {
            bail!("the recovered missing-media repair batch is no longer pending");
        }
        self.media_repair_batch(batch_id)?
            .context("claimed missing-media repair batch disappeared")
    }

    pub fn mark_media_repair_item_running(&self, batch_id: &str, game_uid: &str) -> Result<()> {
        self.mark_media_repair_items_running(batch_id, &[game_uid.to_owned()])
    }

    pub fn mark_media_repair_items_running(
        &self,
        batch_id: &str,
        game_uids: &[String],
    ) -> Result<()> {
        if game_uids.is_empty() || game_uids.len() > MAX_MEDIA_REPAIR_ITEMS {
            bail!("media repair queue claims require at least one bounded exact identity");
        }
        let mut seen = HashSet::with_capacity(game_uids.len());
        for game_uid in game_uids {
            validate_item_key(batch_id, game_uid)?;
            if !seen.insert(game_uid.as_str()) {
                bail!("media repair queue claim contains duplicate game {game_uid}");
            }
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        {
            let mut statement = transaction.prepare(
                "UPDATE media_repair_items
                 SET status='running', outcome_detail='', updated_at=?3
                 WHERE batch_id=?1 AND game_uid=?2 AND status='pending'
                   AND EXISTS (
                       SELECT 1 FROM media_repair_batches
                       WHERE id=?1 AND status='running'
                   )",
            )?;
            let now = unix_timestamp();
            for game_uid in game_uids {
                let changed = statement.execute(params![batch_id, game_uid, now])?;
                if changed != 1 {
                    bail!(
                        "the exact media repair item {game_uid} is not pending in the running batch"
                    );
                }
            }
        }
        transaction.execute(
            "UPDATE media_repair_batches SET updated_at=?2 WHERE id=?1",
            params![batch_id, unix_timestamp()],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn finish_media_repair_item(
        &self,
        batch_id: &str,
        game_uid: &str,
        status: &str,
        detail: &str,
    ) -> Result<()> {
        validate_item_key(batch_id, game_uid)?;
        if !matches!(status, "succeeded" | "unavailable" | "failed") {
            bail!("unsupported terminal media repair item status {status}");
        }
        validate_detail(detail)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE media_repair_items
             SET status=?3, outcome_detail=?4, updated_at=?5
             WHERE batch_id=?1 AND game_uid=?2 AND status='running'",
            params![batch_id, game_uid, status, detail, unix_timestamp()],
        )?;
        if changed != 1 {
            bail!("the exact media repair item is missing or already terminal");
        }
        transaction.execute(
            "UPDATE media_repair_batches SET updated_at=?2 WHERE id=?1",
            params![batch_id, unix_timestamp()],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn complete_media_repair_batch(&self, batch_id: &str, detail: &str) -> Result<()> {
        validate_batch_terminal_request(batch_id, detail)?;
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE media_repair_batches
             SET status='completed', detail=?2, updated_at=?3
             WHERE id=?1 AND status='running'
               AND NOT EXISTS (
                   SELECT 1 FROM media_repair_items
                   WHERE batch_id=?1 AND status IN ('pending', 'running')
               )",
            params![batch_id, detail, unix_timestamp()],
        )?;
        if changed != 1 {
            bail!("the media repair batch still has unfinished items or is already terminal");
        }
        Ok(())
    }

    pub fn cancel_media_repair_batch(&self, batch_id: &str, detail: &str) -> Result<()> {
        terminalize_media_repair_batch(self, batch_id, "cancelled", detail)
    }

    pub fn fail_media_repair_batch(&self, batch_id: &str, detail: &str) -> Result<()> {
        terminalize_media_repair_batch(self, batch_id, "failed", detail)
    }

    pub fn media_repair_batch(&self, batch_id: &str) -> Result<Option<MediaRepairBatchSnapshot>> {
        uuid::Uuid::parse_str(batch_id).context("media repair batches require a UUID identity")?;
        let connection = self.connection()?;
        Ok(load_media_repair_batch(&connection, batch_id).optional()?)
    }
}

fn terminalize_media_repair_batch(
    store: &SettingsStore,
    batch_id: &str,
    status: &str,
    detail: &str,
) -> Result<()> {
    validate_batch_terminal_request(batch_id, detail)?;
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE media_repair_items
         SET status='cancelled', outcome_detail=?2, updated_at=?3
         WHERE batch_id=?1 AND status IN ('pending', 'running')",
        params![batch_id, detail, unix_timestamp()],
    )?;
    let changed = transaction.execute(
        "UPDATE media_repair_batches
         SET status=?2, detail=?3, updated_at=?4
         WHERE id=?1 AND status IN ('pending', 'running')",
        params![batch_id, status, detail, unix_timestamp()],
    )?;
    if changed != 1 {
        bail!("the media repair batch is missing or already terminal");
    }
    transaction.commit()?;
    Ok(())
}

fn load_media_repair_batch(
    connection: &rusqlite::Connection,
    batch_id: &str,
) -> rusqlite::Result<MediaRepairBatchSnapshot> {
    let mut snapshot = connection.query_row(
        "SELECT id, scope, artwork_kind, status, detail, created_at, updated_at
         FROM media_repair_batches WHERE id=?1",
        [batch_id],
        |row| {
            Ok(MediaRepairBatchSnapshot {
                id: row.get(0)?,
                scope: row.get(1)?,
                artwork_kind: row.get(2)?,
                status: row.get(3)?,
                detail: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                items: Vec::new(),
            })
        },
    )?;
    let mut statement = connection.prepare(
        "SELECT position, game_uid, launchbox_db_id, title, platform, local,
                downloadable, artwork_kind, request_detail, status,
                outcome_detail, updated_at
         FROM media_repair_items
         WHERE batch_id=?1
         ORDER BY position",
    )?;
    snapshot.items = statement
        .query_map([batch_id], |row| {
            let position = row.get::<_, i64>(0)?;
            Ok(MediaRepairItem {
                position: usize::try_from(position).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Integer,
                        Box::new(error),
                    )
                })?,
                target: MediaRepairTarget {
                    game_uid: row.get(1)?,
                    launchbox_db_id: row.get(2)?,
                    title: row.get(3)?,
                    platform: row.get(4)?,
                    local: row.get(5)?,
                    downloadable: row.get(6)?,
                    artwork_kind: row.get(7)?,
                    detail: row.get(8)?,
                },
                status: row.get(9)?,
                outcome_detail: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(snapshot)
}

fn prune_media_repair_history(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute(
        "DELETE FROM media_repair_batches
         WHERE status NOT IN ('pending', 'running')
           AND id NOT IN (
               SELECT id FROM media_repair_batches
               WHERE status NOT IN ('pending', 'running')
               ORDER BY updated_at DESC, rowid DESC
               LIMIT ?1
           )",
        [
            i64::try_from(MAX_MEDIA_REPAIR_HISTORY)
                .context("media repair history limit overflow")?,
        ],
    )?;
    Ok(())
}

fn validate_batch_identity(batch_id: &str, scope: &str, artwork_kind: &str) -> Result<()> {
    uuid::Uuid::parse_str(batch_id).context("media repair batches require a UUID identity")?;
    if !matches!(scope, "collection" | "catalog") {
        bail!("unsupported media repair scope {scope}");
    }
    ArtworkKind::parse(artwork_kind)
        .with_context(|| format!("unsupported media repair category {artwork_kind}"))?;
    Ok(())
}

fn validate_target(target: &MediaRepairTarget) -> Result<()> {
    if target.game_uid.trim().is_empty()
        || target.game_uid.chars().count() > 512
        || target.game_uid.chars().any(char::is_control)
    {
        bail!("media repair targets require a bounded exact game identity");
    }
    if target.launchbox_db_id <= 0 {
        bail!("automatic media repair requires a positive exact catalog identity");
    }
    validate_bounded_printable("title", &target.title, 512)?;
    validate_bounded_printable("platform", &target.platform, 512)?;
    ArtworkKind::parse(&target.artwork_kind).with_context(|| {
        format!(
            "unsupported media repair target category {}",
            target.artwork_kind
        )
    })?;
    validate_detail(&target.detail)?;
    Ok(())
}

fn validate_item_key(batch_id: &str, game_uid: &str) -> Result<()> {
    uuid::Uuid::parse_str(batch_id).context("media repair batches require a UUID identity")?;
    if game_uid.trim().is_empty()
        || game_uid.chars().count() > 512
        || game_uid.chars().any(char::is_control)
    {
        bail!("media repair items require a bounded exact game identity");
    }
    Ok(())
}

fn validate_batch_terminal_request(batch_id: &str, detail: &str) -> Result<()> {
    uuid::Uuid::parse_str(batch_id).context("media repair batches require a UUID identity")?;
    validate_detail(detail)
}

fn validate_bounded_printable(label: &str, value: &str, maximum: usize) -> Result<()> {
    if value.trim().is_empty()
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        bail!("media repair {label} must contain bounded printable text");
    }
    Ok(())
}

fn validate_detail(detail: &str) -> Result<()> {
    if detail.chars().count() > 4096 || detail.contains('\0') {
        bail!("media repair details must be bounded text");
    }
    Ok(())
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

pub fn seed_media_repair_recovery_probe() -> Result<String> {
    let path = crate::catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .filter(|path| !path.as_os_str().is_empty())
        .context(
            "the media repair recovery UI probe requires an explicit --state-database or LUNCHBOX_STATE_DATABASE",
        )?;
    let store = SettingsStore::at(&path)?;
    if store.active_media_repair_batch()?.is_some() {
        bail!("the media repair recovery UI probe requires no active repair batch");
    }
    let batch_id = uuid::Uuid::new_v4().to_string();
    store.begin_media_repair_batch(
        &batch_id,
        "collection",
        "box-front",
        &[MediaRepairTarget {
            game_uid: "9697a5eb-e0b4-4f24-8d43-672701414ee7".to_owned(),
            launchbox_db_id: 140,
            title: "Super Mario Bros.".to_owned(),
            platform: "Nintendo Entertainment System".to_owned(),
            local: true,
            downloadable: false,
            artwork_kind: "box-front".to_owned(),
            detail: "No exact box front is cached.".to_owned(),
        }],
    )?;
    store.mark_media_repair_item_running(&batch_id, "9697a5eb-e0b4-4f24-8d43-672701414ee7")?;
    Ok(format!("batch={batch_id} state={path:?}"))
}

#[cfg(test)]
mod tests {
    use super::{MediaRepairTarget, try_acquire_media_repair_lease};
    use crate::settings::SettingsStore;

    fn target(game_uid: &str) -> MediaRepairTarget {
        MediaRepairTarget {
            game_uid: game_uid.to_owned(),
            launchbox_db_id: 140,
            title: format!("Game {game_uid}"),
            platform: "Nintendo Entertainment System".to_owned(),
            local: true,
            downloadable: false,
            artwork_kind: "box-front".to_owned(),
            detail: "No exact box front is cached.".to_owned(),
        }
    }

    #[test]
    fn interrupted_media_repair_is_exact_durable_and_terminal_once() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let batch_id = uuid::Uuid::new_v4().to_string();
        let created = store
            .begin_media_repair_batch(
                &batch_id,
                "collection",
                "box-front",
                &[target("one"), target("two")],
            )
            .unwrap();
        assert_eq!(created.status, "running");
        assert_eq!(created.unfinished_count(), 2);
        assert!(
            store
                .begin_media_repair_batch(
                    &uuid::Uuid::new_v4().to_string(),
                    "catalog",
                    "box-front",
                    &[target("other")],
                )
                .is_err()
        );

        store
            .mark_media_repair_items_running(&batch_id, &["one".to_owned(), "two".to_owned()])
            .unwrap();
        store
            .finish_media_repair_item(&batch_id, "one", "succeeded", "Cached exact artwork.")
            .unwrap();
        let recovered = store
            .recover_interrupted_media_repair_batch()
            .unwrap()
            .unwrap();
        assert_eq!(recovered.recovered_running_items, 1);
        assert_eq!(recovered.snapshot.status, "pending");
        assert_eq!(recovered.snapshot.succeeded_count(), 1);
        assert_eq!(recovered.snapshot.unfinished_count(), 1);

        store.claim_media_repair_batch(&batch_id).unwrap();
        store
            .mark_media_repair_item_running(&batch_id, "two")
            .unwrap();
        store
            .finish_media_repair_item(&batch_id, "two", "unavailable", "No exact provider file.")
            .unwrap();
        store
            .complete_media_repair_batch(&batch_id, "Repair complete.")
            .unwrap();
        assert!(store.active_media_repair_batch().unwrap().is_none());
        assert!(
            store
                .complete_media_repair_batch(&batch_id, "late completion")
                .is_err()
        );
    }

    #[test]
    fn media_repair_recovery_respects_a_live_cross_process_lease() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let first = try_acquire_media_repair_lease(&store).unwrap().unwrap();
        assert!(try_acquire_media_repair_lease(&store).unwrap().is_none());
        drop(first);
        assert!(try_acquire_media_repair_lease(&store).unwrap().is_some());
    }

    #[test]
    fn explicit_media_repair_cancellation_never_becomes_recoverable() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let batch_id = uuid::Uuid::new_v4().to_string();
        store
            .begin_media_repair_batch(&batch_id, "catalog", "box-front", &[target("one")])
            .unwrap();
        store
            .cancel_media_repair_batch(&batch_id, "Cancelled by the user.")
            .unwrap();
        assert!(store.active_media_repair_batch().unwrap().is_none());
        assert!(
            store
                .recover_interrupted_media_repair_batch()
                .unwrap()
                .is_none()
        );
    }
}
