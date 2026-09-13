//! End-to-end save synchronization coordinator and recoverable local apply.

use crate::save_cloud::{CloudStore, DeviceHead};
use crate::save_sync::{
    ArtifactKey, ConflictChoice, LocalInventory, RouteRoot, SaveManifest, SaveRoute, SyncAction,
    SyncActionKind, SyncPlan, SyncScope, merged_files, plan_three_way, resolve_conflicts,
    scan_local, validate_route_roots,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PreparedSync {
    pub scope: SyncScope,
    pub device_id: String,
    pub roots: Vec<RouteRoot>,
    pub local: LocalInventory,
    own_manifest_id: Option<String>,
    pub remote: Option<SaveManifest>,
    pub parent_ids: Vec<String>,
    pub plan: SyncPlan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AppliedSync {
    pub manifest_id: String,
    pub recovery_directory: Option<PathBuf>,
    pub actions: Vec<SyncAction>,
}

/// Return only remote history tips that are not already incorporated into the
/// local device head. Device heads can share a manifest or point at an older
/// ancestor; neither case needs a redundant choice in the UI.
pub fn remote_merge_candidates(
    store: &CloudStore,
    scope: &SyncScope,
    device_id: &str,
) -> Result<Vec<DeviceHead>> {
    Ok(load_sync_heads(store, scope, device_id)?.1)
}

fn load_sync_heads(
    store: &CloudStore,
    scope: &SyncScope,
    device_id: &str,
) -> Result<(Option<DeviceHead>, Vec<DeviceHead>)> {
    scope.validate()?;
    DeviceHead::new(scope.clone(), device_id, "a".repeat(64), 0)
        .context("validating local save-sync device ID")?;
    let heads = store.device_heads(scope)?;
    let own_head = heads
        .iter()
        .find(|head| head.device_id == device_id)
        .cloned();

    // Several devices may legitimately share one immutable manifest. One
    // representative is enough because selecting any of them merges the same
    // history. Prefer the most recently updated head for display.
    let mut by_manifest = BTreeMap::<String, DeviceHead>::new();
    for head in heads.into_iter().filter(|head| head.device_id != device_id) {
        let replace = by_manifest.get(&head.manifest_id).is_none_or(|current| {
            (head.updated_unix_ms, &head.device_id) > (current.updated_unix_ms, &current.device_id)
        });
        if replace {
            by_manifest.insert(head.manifest_id.clone(), head);
        }
    }
    let mut candidates = by_manifest.into_values().collect::<Vec<_>>();

    if let Some(own) = &own_head {
        let mut unmerged = Vec::new();
        for candidate in candidates {
            if !manifest_is_ancestor(store, scope, &candidate.manifest_id, &own.manifest_id)? {
                unmerged.push(candidate);
            }
        }
        candidates = unmerged;
    }

    // Keep only the frontier. If device B's history already includes device
    // A's head, merging B incorporates A too and should not prompt twice.
    let snapshot = candidates.clone();
    let mut frontier = Vec::new();
    for candidate in candidates {
        let mut represented_by_newer_tip = false;
        for other in &snapshot {
            if candidate.manifest_id != other.manifest_id
                && manifest_is_ancestor(store, scope, &candidate.manifest_id, &other.manifest_id)?
            {
                represented_by_newer_tip = true;
                break;
            }
        }
        if !represented_by_newer_tip {
            frontier.push(candidate);
        }
    }
    frontier.sort_by(|left, right| {
        right
            .updated_unix_ms
            .cmp(&left.updated_unix_ms)
            .then_with(|| left.device_id.cmp(&right.device_id))
    });
    Ok((own_head, frontier))
}

fn manifest_is_ancestor(
    store: &CloudStore,
    scope: &SyncScope,
    ancestor_id: &str,
    descendant_id: &str,
) -> Result<bool> {
    if ancestor_id == descendant_id {
        return Ok(true);
    }
    Ok(store
        .nearest_common_ancestor(scope, ancestor_id, descendant_id)?
        .is_some_and(|manifest| manifest.id == ancestor_id))
}

const RECOVERY_SCHEMA: u32 = 2;
const LEGACY_TERMINAL_RECOVERY_SCHEMA: u32 = 1;
const MAX_RECOVERY_JOURNAL_BYTES: u64 = 4 * 1024 * 1024;
const MAX_RECOVERY_MUTATIONS: usize = 4096;

#[derive(Deserialize)]
struct RecoveryJournalSchema {
    schema: u32,
}

/// Schema 1 never supported restart recovery and therefore recorded no head
/// linkage. It is accepted only for an already-complete directory produced by
/// the old writer; an incomplete schema-1 journal remains unsafe to replay.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTerminalRecoveryJournal {
    schema: u32,
    scope: SyncScope,
    device_id: String,
    created_unix_ms: i64,
    mutations: Vec<RecoveryMutation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryJournal {
    schema: u32,
    scope: SyncScope,
    device_id: String,
    created_unix_ms: i64,
    previous_manifest_id: Option<String>,
    next_manifest_id: String,
    mutations: Vec<RecoveryMutation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryMutation {
    key: ArtifactKey,
    kind: SyncActionKind,
    target: PathBuf,
    backup: Option<PathBuf>,
    staged_download: Option<PathBuf>,
}

pub fn prepare_sync(
    store: &CloudStore,
    scope: SyncScope,
    device_id: impl Into<String>,
    roots: Vec<RouteRoot>,
    remote_device_id: Option<&str>,
) -> Result<PreparedSync> {
    scope.validate()?;
    let device_id = device_id.into();
    DeviceHead::new(scope.clone(), &device_id, "a".repeat(64), 0)
        .context("validating local save-sync device ID")?;
    let local = scan_local(scope.clone(), &roots)?;
    let (own_head, candidates) = load_sync_heads(store, &scope, &device_id)?;
    let selected = match remote_device_id {
        Some(requested) => Some(
            candidates
                .iter()
                .find(|head| head.device_id == requested)
                .with_context(|| format!("no cloud save head for device {requested}"))?,
        ),
        None if candidates.len() == 1 => Some(&candidates[0]),
        None if candidates.len() > 1 => {
            let devices = candidates
                .iter()
                .map(|head| head.device_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            bail!("multiple remote save histories require a device selection: {devices}");
        }
        None => None,
    };

    let own_manifest = own_head
        .as_ref()
        .map(|head| store.get_manifest(&scope, &head.manifest_id))
        .transpose()?;
    let selected_manifest = selected
        .map(|head| store.get_manifest(&scope, &head.manifest_id))
        .transpose()?;

    let own_manifest_id = own_manifest.as_ref().map(|manifest| manifest.id.clone());
    let (base, remote, mut parent_ids) = match (own_manifest, selected_manifest) {
        (Some(own), Some(selected)) if own.id == selected.id => {
            (Some(own.clone()), Some(own.clone()), vec![own.id])
        }
        (Some(own), Some(selected)) => {
            let ancestor = store.nearest_common_ancestor(&scope, &own.id, &selected.id)?;
            if ancestor
                .as_ref()
                .is_some_and(|ancestor| ancestor.id == selected.id)
            {
                // The selected device is behind this installation. Its state is
                // already represented by our head; compare local files to ours.
                (Some(own.clone()), Some(own.clone()), vec![own.id])
            } else {
                let parents = vec![own.id.clone(), selected.id.clone()];
                (ancestor, Some(selected), parents)
            }
        }
        (Some(own), None) => (Some(own.clone()), Some(own.clone()), vec![own.id]),
        (None, Some(selected)) => (None, Some(selected.clone()), vec![selected.id]),
        (None, None) => (None, None, Vec::new()),
    };
    parent_ids.sort();
    parent_ids.dedup();
    let plan = plan_three_way(base.as_ref(), &local, remote.as_ref())?;
    Ok(PreparedSync {
        scope,
        device_id,
        roots,
        local,
        own_manifest_id,
        remote,
        parent_ids,
        plan,
    })
}

impl PreparedSync {
    pub fn apply(
        &self,
        store: &CloudStore,
        choices: &BTreeMap<ArtifactKey, ConflictChoice>,
        recovery_base: &Path,
        now_unix_ms: i64,
    ) -> Result<AppliedSync> {
        ensure!(now_unix_ms >= 0, "sync timestamp is before the Unix epoch");
        let recovered = recover_pending_sync_mutations(
            store,
            &self.scope,
            &self.device_id,
            &self.roots,
            recovery_base,
        )?;
        ensure!(
            recovered.is_empty(),
            "recovered an interrupted save sync from {}; review a fresh sync plan before making new changes",
            recovered
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        let current_own_manifest_id = store
            .device_heads(&self.scope)?
            .into_iter()
            .find(|head| head.device_id == self.device_id)
            .map(|head| head.manifest_id);
        ensure!(
            current_own_manifest_id == self.own_manifest_id,
            "cloud save history changed after sync planning; review a fresh plan"
        );
        let actions = resolve_conflicts(&self.plan, choices)?;
        if actions.is_empty()
            && let Some(remote) = &self.remote
        {
            if self.own_manifest_id.as_deref() == Some(remote.id.as_str()) {
                return Ok(AppliedSync {
                    manifest_id: remote.id.clone(),
                    recovery_directory: None,
                    actions,
                });
            }
            if self.own_manifest_id.is_none() {
                store.set_device_head(&DeviceHead::new(
                    self.scope.clone(),
                    self.device_id.clone(),
                    remote.id.clone(),
                    now_unix_ms,
                )?)?;
                return Ok(AppliedSync {
                    manifest_id: remote.id.clone(),
                    recovery_directory: None,
                    actions,
                });
            }
        }

        // Re-scan immediately before IO so a running emulator cannot make the
        // accepted plan stale without detection.
        let current = scan_local(self.scope.clone(), &self.roots)?;
        ensure!(
            current == self.local,
            "local save data changed after sync planning; review a fresh plan"
        );
        let roots = root_map(&self.roots)?;

        // Upload local winners first. Remote blobs are immutable, so this is
        // harmless if later local application fails.
        for action in &actions {
            if action.kind != SyncActionKind::Upload {
                continue;
            }
            let version = action
                .version
                .as_ref()
                .context("upload action has no file version")?;
            let path = artifact_path(&roots, &action.key)?;
            store.put_blob_file(&self.scope, version, &path)?;
        }

        let files = merged_files(&self.local, self.remote.as_ref(), &actions)?;
        let manifest = SaveManifest::new(
            self.scope.clone(),
            self.parent_ids.clone(),
            self.device_id.clone(),
            now_unix_ms,
            files,
        )?;
        let mut recovery_directory = None;
        let local_actions = actions
            .iter()
            .filter(|action| {
                matches!(
                    action.kind,
                    SyncActionKind::Download | SyncActionKind::DeleteLocal
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        if !local_actions.is_empty() {
            let recovery = prepare_local_mutations(
                store,
                &self.scope,
                &self.device_id,
                &roots,
                &local_actions,
                recovery_base,
                now_unix_ms,
                self.own_manifest_id.as_deref(),
                &manifest.id,
            )?;
            apply_local_mutations(&recovery)?;
            recovery_directory = Some(recovery.directory.clone());
        }

        store.put_manifest(&manifest)?;
        store.set_device_head(&DeviceHead::new(
            self.scope.clone(),
            self.device_id.clone(),
            manifest.id.clone(),
            now_unix_ms,
        )?)?;
        if let Some(directory) = &recovery_directory {
            write_synced(&directory.join("complete"), manifest.id.as_bytes())?;
        }
        Ok(AppliedSync {
            manifest_id: manifest.id,
            recovery_directory,
            actions,
        })
    }
}

struct PreparedLocalMutations {
    directory: PathBuf,
    mutations: Vec<RecoveryMutation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MutationRecoveryState {
    Applied,
    NotApplied,
}

struct ValidatedRecovery {
    directory: PathBuf,
    journal: RecoveryJournal,
    states: Vec<MutationRecoveryState>,
}

/// Resolve one interrupted local save transaction before any new sync writes.
///
/// A journal is rolled back only while this device's cloud head still matches
/// the head recorded before the transaction. If the intended head was already
/// published, every local mutation must match its staged result before the
/// journal is marked complete. Any other state is ambiguous and remains on
/// disk while synchronization fails closed.
pub fn recover_pending_sync_mutations(
    store: &CloudStore,
    scope: &SyncScope,
    device_id: &str,
    roots: &[RouteRoot],
    recovery_base: &Path,
) -> Result<Vec<PathBuf>> {
    ensure!(
        recovery_base.is_absolute(),
        "sync recovery base must be absolute"
    );
    ensure_existing_ancestors_without_symlink(recovery_base)?;
    let metadata = match std::fs::symlink_metadata(recovery_base) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).context("reading save-sync recovery base"),
    };
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "save-sync recovery base is not a physical directory"
    );

    let pending = pending_recovery_directories(store, recovery_base)?;
    ensure!(
        pending.len() <= 1,
        "multiple incomplete save-sync recovery journals require manual review: {}",
        pending
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let Some(directory) = pending.into_iter().next() else {
        return Ok(Vec::new());
    };
    let recovery = read_and_validate_recovery(&directory, scope, device_id, roots)?;
    let current_manifest_id = store
        .device_heads(&recovery.journal.scope)?
        .into_iter()
        .find(|head| head.device_id == recovery.journal.device_id)
        .map(|head| head.manifest_id);

    if current_manifest_id.as_deref() == Some(recovery.journal.next_manifest_id.as_str()) {
        for (mutation, state) in recovery.journal.mutations.iter().zip(&recovery.states) {
            ensure!(
                *state == MutationRecoveryState::Applied,
                "committed save-sync recovery target does not match its staged result: {}; recovery evidence retained at {}",
                mutation.target.display(),
                recovery.directory.display()
            );
        }
        write_synced(
            &recovery.directory.join("complete"),
            recovery.journal.next_manifest_id.as_bytes(),
        )?;
    } else if current_manifest_id.as_deref() == recovery.journal.previous_manifest_id.as_deref() {
        let applied = recovery
            .journal
            .mutations
            .iter()
            .zip(&recovery.states)
            .filter(|(_, state)| **state == MutationRecoveryState::Applied)
            .map(|(mutation, _)| mutation.clone())
            .collect::<Vec<_>>();
        rollback_mutations(&applied).with_context(|| {
            format!(
                "recovering interrupted save sync; recovery evidence retained at {}",
                recovery.directory.display()
            )
        })?;
        for mutation in &recovery.journal.mutations {
            ensure!(
                mutation_matches_original(mutation)?,
                "rolled-back save-sync target could not be verified: {}; recovery evidence retained at {}",
                mutation.target.display(),
                recovery.directory.display()
            );
        }
        write_synced(
            &recovery.directory.join("rolled-back"),
            recovery.journal.next_manifest_id.as_bytes(),
        )?;
    } else {
        bail!(
            "save-sync recovery journal has a stale cloud head; recovery evidence retained at {}",
            recovery.directory.display()
        );
    }
    Ok(vec![recovery.directory])
}

fn pending_recovery_directories(store: &CloudStore, recovery_base: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = Vec::new();
    for entry in std::fs::read_dir(recovery_base).context("listing save-sync recovery journals")? {
        let entry = entry.context("reading save-sync recovery directory entry")?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .context("reading save-sync recovery directory metadata")?;
        ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "unexpected non-directory in save-sync recovery base: {}",
            path.display()
        );
        let timestamp = recovery_directory_timestamp(&path)?;
        let complete = read_recovery_marker(&path.join("complete"))?;
        let rolled_back = read_recovery_marker(&path.join("rolled-back"))?;
        ensure!(
            !(complete.is_some() && rolled_back.is_some()),
            "save-sync recovery directory has conflicting completion markers: {}",
            path.display()
        );
        if complete.is_some() || rolled_back.is_some() {
            validate_terminal_recovery_journal(
                store,
                &path,
                complete.as_deref(),
                rolled_back.as_deref(),
            )?;
        } else {
            pending.push((timestamp, path));
        }
    }
    pending.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    Ok(pending.into_iter().map(|(_, path)| path).collect())
}

fn recovery_directory_timestamp(directory: &Path) -> Result<i64> {
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .context("save-sync recovery directory name is not UTF-8")?;
    let (timestamp, id) = name
        .split_once('-')
        .context("invalid save-sync recovery directory name")?;
    let timestamp = timestamp
        .parse::<i64>()
        .context("invalid save-sync recovery directory timestamp")?;
    ensure!(timestamp >= 0, "save-sync recovery timestamp is negative");
    ensure!(
        id.len() == 32 && Uuid::parse_str(id).is_ok(),
        "invalid save-sync recovery directory ID"
    );
    Ok(timestamp)
}

fn read_recovery_marker(path: &Path) -> Result<Option<String>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("reading save-sync recovery marker"),
    };
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() == 64,
        "invalid save-sync recovery marker: {}",
        path.display()
    );
    let value = std::fs::read_to_string(path).context("reading save-sync recovery marker")?;
    ensure_manifest_id(&value)?;
    Ok(Some(value))
}

fn parse_recovery_journal(directory: &Path) -> Result<RecoveryJournal> {
    let bytes = read_recovery_journal_bytes(directory)?;
    parse_recovery_journal_bytes(directory, &bytes)
}

fn read_recovery_journal_bytes(directory: &Path) -> Result<Vec<u8>> {
    let journal_path = directory.join("journal.json");
    let metadata = regular_file_metadata(&journal_path, "save-sync recovery journal")?;
    ensure!(
        metadata.len() <= MAX_RECOVERY_JOURNAL_BYTES,
        "save-sync recovery journal is too large"
    );
    std::fs::read(&journal_path).context("reading save-sync recovery journal")
}

fn parse_recovery_journal_bytes(directory: &Path, bytes: &[u8]) -> Result<RecoveryJournal> {
    let journal: RecoveryJournal =
        serde_json::from_slice(bytes).context("parsing save-sync recovery journal")?;
    ensure!(
        journal.schema == RECOVERY_SCHEMA,
        "unsupported save-sync recovery journal schema"
    );
    journal.scope.validate()?;
    ensure!(
        journal.created_unix_ms == recovery_directory_timestamp(directory)?,
        "save-sync recovery journal timestamp does not match its directory"
    );
    ensure!(
        !journal.mutations.is_empty() && journal.mutations.len() <= MAX_RECOVERY_MUTATIONS,
        "save-sync recovery journal has an invalid mutation count"
    );
    ensure_manifest_id(&journal.next_manifest_id)?;
    if let Some(previous) = &journal.previous_manifest_id {
        ensure_manifest_id(previous)?;
        ensure!(
            previous != &journal.next_manifest_id,
            "save-sync recovery journal did not advance the manifest"
        );
    }
    DeviceHead::new(
        journal.scope.clone(),
        &journal.device_id,
        journal.next_manifest_id.clone(),
        journal.created_unix_ms,
    )
    .context("validating save-sync recovery identity")?;
    Ok(journal)
}

fn validate_terminal_recovery_journal(
    store: &CloudStore,
    directory: &Path,
    complete: Option<&str>,
    rolled_back: Option<&str>,
) -> Result<()> {
    let marker = complete
        .or(rolled_back)
        .context("missing recovery marker")?;
    let bytes = read_recovery_journal_bytes(directory)?;
    let schema: RecoveryJournalSchema =
        serde_json::from_slice(&bytes).context("reading save-sync recovery journal schema")?;
    match schema.schema {
        RECOVERY_SCHEMA => {
            let journal = parse_recovery_journal_bytes(directory, &bytes)?;
            ensure!(
                marker == journal.next_manifest_id,
                "save-sync recovery marker does not match its journal: {}",
                directory.display()
            );
        }
        LEGACY_TERMINAL_RECOVERY_SCHEMA => {
            ensure!(
                complete.is_some() && rolled_back.is_none(),
                "legacy save-sync recovery journal has an impossible rollback marker: {}",
                directory.display()
            );
            let journal: LegacyTerminalRecoveryJournal = serde_json::from_slice(&bytes)
                .context("parsing legacy terminal save-sync recovery journal")?;
            ensure!(
                journal.schema == LEGACY_TERMINAL_RECOVERY_SCHEMA,
                "unsupported save-sync recovery journal schema"
            );
            journal.scope.validate()?;
            ensure!(
                journal.created_unix_ms == recovery_directory_timestamp(directory)?,
                "legacy save-sync recovery journal timestamp does not match its directory"
            );
            ensure!(
                !journal.mutations.is_empty() && journal.mutations.len() <= MAX_RECOVERY_MUTATIONS,
                "legacy save-sync recovery journal has an invalid mutation count"
            );
            DeviceHead::new(
                journal.scope.clone(),
                journal.device_id.clone(),
                marker,
                journal.created_unix_ms,
            )
            .context("validating legacy terminal save-sync recovery identity")?;
            // Schema 1 did not persist the intended manifest ID. The old
            // writer did, however, write `complete` only after publishing the
            // immutable manifest. Resolve it to establish linkage instead of
            // trusting the marker by shape alone.
            let manifest = store
                .get_manifest(&journal.scope, marker)
                .context("resolving legacy completed save-sync manifest")?;
            ensure!(
                manifest.device_id == journal.device_id
                    && manifest.created_unix_ms == journal.created_unix_ms,
                "legacy save-sync recovery marker does not match its journal: {}",
                directory.display()
            );
        }
        _ => bail!("unsupported save-sync recovery journal schema"),
    }
    Ok(())
}

fn read_and_validate_recovery(
    directory: &Path,
    expected_scope: &SyncScope,
    expected_device_id: &str,
    roots: &[RouteRoot],
) -> Result<ValidatedRecovery> {
    let journal = parse_recovery_journal(directory)?;
    ensure!(
        &journal.scope == expected_scope,
        "incomplete save-sync recovery journal belongs to a different scope: {}",
        directory.display()
    );
    ensure!(
        journal.device_id == expected_device_id,
        "incomplete save-sync recovery journal belongs to a different device: {}",
        directory.display()
    );
    let roots = root_map(roots)?;
    let mut keys = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut states = Vec::with_capacity(journal.mutations.len());
    for (index, mutation) in journal.mutations.iter().enumerate() {
        let route = mutation.key.route()?;
        let relative = mutation.key.relative_path()?;
        ensure!(
            ArtifactKey::new(route, relative)?.as_str() == mutation.key.as_str(),
            "save-sync recovery artifact key is not canonical"
        );
        ensure!(
            keys.insert(mutation.key.clone()),
            "duplicate artifact in save-sync recovery journal"
        );
        ensure!(
            targets.insert(mutation.target.clone()),
            "duplicate target in save-sync recovery journal"
        );
        let expected_target = artifact_path(&roots, &mutation.key)?;
        ensure!(
            mutation.target == expected_target,
            "save-sync recovery target no longer matches its route: {}",
            mutation.target.display()
        );
        validate_existing_path_chain(&roots, &mutation.key, &mutation.target)?;

        let expected_backup = directory.join("backups").join(mutation.key.as_str());
        if let Some(backup) = &mutation.backup {
            ensure!(
                backup == &expected_backup,
                "save-sync recovery backup escaped its journal directory"
            );
            regular_file_metadata(backup, "save-sync recovery backup")?;
        }
        let expected_staged = directory.join("downloads").join(format!("{index}.part"));
        match mutation.kind {
            SyncActionKind::Download => {
                let staged = mutation
                    .staged_download
                    .as_ref()
                    .context("download recovery mutation has no staged file")?;
                ensure!(
                    staged == &expected_staged,
                    "staged save download escaped its journal directory"
                );
                regular_file_metadata(staged, "staged save download")?;
            }
            SyncActionKind::DeleteLocal => {
                ensure!(
                    mutation.staged_download.is_none(),
                    "delete recovery mutation unexpectedly has a staged download"
                );
                ensure!(
                    mutation.backup.is_some(),
                    "delete recovery mutation has no original backup"
                );
            }
            _ => bail!("non-local action in save-sync recovery journal"),
        }
        states.push(classify_recovery_mutation(mutation)?);
    }
    Ok(ValidatedRecovery {
        directory: directory.to_path_buf(),
        journal,
        states,
    })
}

fn ensure_manifest_id(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "save-sync recovery manifest ID is not canonical SHA-256"
    );
    Ok(())
}

fn regular_file_metadata(path: &Path, label: &str) -> Result<std::fs::Metadata> {
    ensure_existing_ancestors_without_symlink(path)?;
    let metadata = std::fs::symlink_metadata(path).with_context(|| format!("reading {label}"))?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "{label} is not a physical regular file: {}",
        path.display()
    );
    Ok(metadata)
}

fn optional_regular_file(path: &Path, label: &str) -> Result<bool> {
    ensure_existing_ancestors_without_symlink(path)?;
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "{label} is not a physical regular file: {}",
                path.display()
            );
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("reading {label}")),
    }
}

fn classify_recovery_mutation(mutation: &RecoveryMutation) -> Result<MutationRecoveryState> {
    let target_exists = optional_regular_file(&mutation.target, "save-sync recovery target")?;
    match mutation.kind {
        SyncActionKind::Download => {
            let staged = mutation
                .staged_download
                .as_ref()
                .context("download recovery mutation has no staged file")?;
            if target_exists && files_equal(&mutation.target, staged)? {
                return Ok(MutationRecoveryState::Applied);
            }
            match &mutation.backup {
                Some(backup) if target_exists && files_equal(&mutation.target, backup)? => {
                    Ok(MutationRecoveryState::NotApplied)
                }
                None if !target_exists => Ok(MutationRecoveryState::NotApplied),
                _ => bail!(
                    "save-sync recovery target is stale or ambiguous: {}",
                    mutation.target.display()
                ),
            }
        }
        SyncActionKind::DeleteLocal => {
            let backup = mutation
                .backup
                .as_ref()
                .context("delete recovery mutation has no original backup")?;
            if !target_exists {
                Ok(MutationRecoveryState::Applied)
            } else if files_equal(&mutation.target, backup)? {
                Ok(MutationRecoveryState::NotApplied)
            } else {
                bail!(
                    "save-sync recovery target is stale or ambiguous: {}",
                    mutation.target.display()
                )
            }
        }
        _ => bail!("non-local action in save-sync recovery journal"),
    }
}

fn mutation_matches_original(mutation: &RecoveryMutation) -> Result<bool> {
    let target_exists = optional_regular_file(&mutation.target, "rolled-back save target")?;
    match &mutation.backup {
        Some(backup) => Ok(target_exists && files_equal(&mutation.target, backup)?),
        None => Ok(!target_exists),
    }
}

fn files_equal(left: &Path, right: &Path) -> Result<bool> {
    let left_metadata = regular_file_metadata(left, "save-sync comparison file")?;
    let right_metadata = regular_file_metadata(right, "save-sync comparison file")?;
    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }
    let mut left = File::open(left).context("opening save-sync comparison file")?;
    let mut right = File::open(right).context("opening save-sync comparison file")?;
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_count = left
            .read(&mut left_buffer)
            .context("reading save-sync comparison file")?;
        let right_count = right
            .read(&mut right_buffer)
            .context("reading save-sync comparison file")?;
        if left_count != right_count || left_buffer[..left_count] != right_buffer[..right_count] {
            return Ok(false);
        }
        if left_count == 0 {
            return Ok(true);
        }
    }
}

fn prepare_local_mutations(
    store: &CloudStore,
    scope: &SyncScope,
    device_id: &str,
    roots: &BTreeMap<SaveRoute, PathBuf>,
    actions: &[SyncAction],
    recovery_base: &Path,
    now_unix_ms: i64,
    previous_manifest_id: Option<&str>,
    next_manifest_id: &str,
) -> Result<PreparedLocalMutations> {
    ensure!(
        recovery_base.is_absolute(),
        "sync recovery base must be absolute"
    );
    ensure_directory_without_symlink(recovery_base)?;
    let directory = recovery_base.join(format!("{}-{}", now_unix_ms, Uuid::new_v4().simple()));
    std::fs::create_dir(&directory).context("creating save-sync recovery directory")?;
    let mut mutations = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        let target = artifact_path(roots, &action.key)?;
        validate_existing_path_chain(roots, &action.key, &target)?;
        let backup = if target.exists() {
            let path = directory.join("backups").join(action.key.as_str());
            copy_file_synced(&target, &path)?;
            Some(path)
        } else {
            None
        };
        let staged_download = if action.kind == SyncActionKind::Download {
            let version = action
                .version
                .as_ref()
                .context("download action has no file version")?;
            let path = directory.join("downloads").join(format!("{index}.part"));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).context("creating download staging directory")?;
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .context("creating staged local save")?;
            store.copy_blob_to(scope, version, &mut file)?;
            file.sync_all().context("syncing staged local save")?;
            Some(path)
        } else {
            None
        };
        mutations.push(RecoveryMutation {
            key: action.key.clone(),
            kind: action.kind,
            target,
            backup,
            staged_download,
        });
    }
    let journal = RecoveryJournal {
        schema: RECOVERY_SCHEMA,
        scope: scope.clone(),
        device_id: device_id.to_owned(),
        created_unix_ms: now_unix_ms,
        previous_manifest_id: previous_manifest_id.map(str::to_owned),
        next_manifest_id: next_manifest_id.to_owned(),
        mutations: mutations.clone(),
    };
    let journal_bytes =
        serde_json::to_vec_pretty(&journal).context("encoding sync recovery journal")?;
    write_synced(&directory.join("journal.json"), &journal_bytes)?;
    Ok(PreparedLocalMutations {
        directory,
        mutations,
    })
}

fn apply_local_mutations(prepared: &PreparedLocalMutations) -> Result<()> {
    let mut applied = Vec::new();
    for mutation in &prepared.mutations {
        let result = match mutation.kind {
            SyncActionKind::Download => publish_download(mutation),
            SyncActionKind::DeleteLocal => remove_local(mutation),
            _ => bail!("non-local action reached local mutation phase"),
        };
        if let Err(error) = result {
            let rollback = rollback_mutations(&applied);
            return match rollback {
                Ok(()) => Err(error.context("applying local save changes; prior changes rolled back")),
                Err(rollback_error) => Err(error.context(format!(
                    "applying local save changes; rollback also failed: {rollback_error:#}; recovery data: {}",
                    prepared.directory.display()
                ))),
            };
        }
        applied.push(mutation.clone());
    }
    Ok(())
}

fn publish_download(mutation: &RecoveryMutation) -> Result<()> {
    let staged = mutation
        .staged_download
        .as_ref()
        .context("download mutation is not staged")?;
    let parent = mutation
        .target
        .parent()
        .context("save target has no parent")?;
    ensure_directory_without_symlink(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .context("creating same-filesystem save replacement")?;
    let mut source = File::open(staged).context("opening staged save replacement")?;
    std::io::copy(&mut source, &mut temporary).context("copying staged save replacement")?;
    temporary.flush().context("flushing save replacement")?;
    temporary
        .as_file()
        .sync_all()
        .context("syncing save replacement")?;
    temporary
        .persist(&mutation.target)
        .map_err(|error| error.error)
        .context("publishing local save replacement")?;
    Ok(())
}

fn remove_local(mutation: &RecoveryMutation) -> Result<()> {
    if mutation.target.exists() {
        std::fs::remove_file(&mutation.target).context("removing remotely deleted local save")?;
    }
    Ok(())
}

fn rollback_mutations(mutations: &[RecoveryMutation]) -> Result<()> {
    let mut errors = Vec::new();
    for mutation in mutations.iter().rev() {
        let result = if let Some(backup) = &mutation.backup {
            restore_file(backup, &mutation.target)
        } else if mutation.kind == SyncActionKind::Download && mutation.target.exists() {
            std::fs::remove_file(&mutation.target).context("removing rolled-back downloaded save")
        } else {
            Ok(())
        };
        if let Err(error) = result {
            errors.push(format!("{}: {error:#}", mutation.target.display()));
        }
    }
    ensure!(errors.is_empty(), "{}", errors.join("; "));
    Ok(())
}

fn restore_file(backup: &Path, target: &Path) -> Result<()> {
    let parent = target.parent().context("save target has no parent")?;
    ensure_directory_without_symlink(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .context("creating same-filesystem save restoration")?;
    let mut source = File::open(backup).context("opening save recovery backup")?;
    std::io::copy(&mut source, &mut temporary).context("copying save recovery backup")?;
    temporary.flush().context("flushing save recovery backup")?;
    temporary
        .as_file()
        .sync_all()
        .context("syncing save recovery backup")?;
    temporary
        .persist(target)
        .map_err(|error| error.error)
        .context("restoring save recovery backup")?;
    Ok(())
}

fn copy_file_synced(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination.parent().context("backup path has no parent")?;
    std::fs::create_dir_all(parent).context("creating save backup directory")?;
    let mut input = File::open(source).context("opening local save for backup")?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .context("creating local save backup")?;
    std::io::copy(&mut input, &mut output).context("copying local save backup")?;
    output.flush().context("flushing local save backup")?;
    output.sync_all().context("syncing local save backup")?;
    Ok(())
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("writing {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing {}", path.display()))?;
    Ok(())
}

fn root_map(roots: &[RouteRoot]) -> Result<BTreeMap<SaveRoute, PathBuf>> {
    validate_route_roots(roots)?;
    let mut map = BTreeMap::new();
    for root in roots {
        ensure!(
            map.insert(root.route, root.path.clone()).is_none(),
            "duplicate save route"
        );
    }
    Ok(map)
}

fn artifact_path(roots: &BTreeMap<SaveRoute, PathBuf>, key: &ArtifactKey) -> Result<PathBuf> {
    let root = roots
        .get(&key.route()?)
        .context("sync action references an unavailable local route")?;
    let relative = key.relative_path()?;
    if relative.is_empty() {
        return Ok(root.clone());
    }
    Ok(relative
        .split('/')
        .fold(root.clone(), |path, part| path.join(part)))
}

fn validate_existing_path_chain(
    roots: &BTreeMap<SaveRoute, PathBuf>,
    key: &ArtifactKey,
    target: &Path,
) -> Result<()> {
    let root = roots
        .get(&key.route()?)
        .context("sync action references an unavailable local route")?;
    let root_metadata = match std::fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ensure_existing_ancestors_without_symlink(root)?;
            ensure!(
                target.starts_with(root),
                "save target escaped its route root"
            );
            return Ok(());
        }
        Err(error) => return Err(error).context("reading save root metadata"),
    };
    ensure_existing_ancestors_without_symlink(root)?;
    ensure!(
        !root_metadata.file_type().is_symlink(),
        "save root became a symbolic link"
    );
    if key.relative_path()?.is_empty() {
        ensure!(
            root_metadata.is_file(),
            "exact save root is no longer a file"
        );
        return Ok(());
    }
    ensure!(
        root_metadata.is_dir(),
        "directory save root is no longer a directory"
    );
    let mut current = root.clone();
    for part in key.relative_path()?.split('/') {
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "save target path contains a symbolic link"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error).context("reading save target metadata"),
        }
    }
    ensure!(
        target.starts_with(root),
        "save target escaped its route root"
    );
    Ok(())
}

fn ensure_directory_without_symlink(directory: &Path) -> Result<()> {
    if directory.exists() {
        ensure_existing_ancestors_without_symlink(directory)?;
        let metadata =
            std::fs::symlink_metadata(directory).context("reading directory metadata")?;
        ensure!(
            metadata.is_dir(),
            "expected a directory: {}",
            directory.display()
        );
        ensure!(
            !metadata.file_type().is_symlink(),
            "directory may not be a symbolic link: {}",
            directory.display()
        );
        return Ok(());
    }
    let parent = directory.parent().context("directory has no parent")?;
    ensure_directory_without_symlink(parent)?;
    std::fs::create_dir(directory)
        .with_context(|| format!("creating directory {}", directory.display()))?;
    Ok(())
}

fn ensure_existing_ancestors_without_symlink(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "save path must be absolute");
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                ensure!(
                    !metadata.file_type().is_symlink(),
                    "save path contains a symbolic link: {}",
                    current.display()
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error).context("reading save path ancestor metadata"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_sync::{FileVersion, SavePurpose, SaveRoute};

    fn scope() -> SyncScope {
        SyncScope::new("duckstation", "linux").unwrap()
    }

    fn roots(path: &Path) -> Vec<RouteRoot> {
        vec![RouteRoot {
            route: SaveRoute {
                purpose: SavePurpose::Saves,
                root_index: 0,
            },
            path: path.to_path_buf(),
            create_if_missing: true,
        }]
    }

    #[test]
    fn first_device_uploads_and_publishes_its_head() {
        let store = CloudStore::memory().unwrap();
        let local = tempfile::tempdir().unwrap();
        let recovery = tempfile::tempdir().unwrap();
        std::fs::write(local.path().join("game.sav"), b"device a").unwrap();
        let prepared =
            prepare_sync(&store, scope(), "device-a", roots(local.path()), None).unwrap();
        assert_eq!(prepared.plan.actions.len(), 1);
        assert_eq!(prepared.plan.actions[0].kind, SyncActionKind::Upload);
        let applied = prepared
            .apply(&store, &BTreeMap::new(), recovery.path(), 1000)
            .unwrap();
        assert!(applied.recovery_directory.is_none());
        let heads = store.device_heads(&scope()).unwrap();
        assert_eq!(heads.len(), 1);
        assert_eq!(heads[0].device_id, "device-a");
        assert_eq!(heads[0].manifest_id, applied.manifest_id);
    }

    #[test]
    fn remote_choice_replaces_local_file_and_keeps_recovery_copy() {
        let store = CloudStore::memory().unwrap();
        let device_a = tempfile::tempdir().unwrap();
        let device_b = tempfile::tempdir().unwrap();
        let recovery = tempfile::tempdir().unwrap();
        std::fs::write(device_a.path().join("game.sav"), b"remote winner").unwrap();
        prepare_sync(&store, scope(), "device-a", roots(device_a.path()), None)
            .unwrap()
            .apply(&store, &BTreeMap::new(), recovery.path(), 1000)
            .unwrap();

        std::fs::write(device_b.path().join("game.sav"), b"local loser").unwrap();
        let prepared =
            prepare_sync(&store, scope(), "device-b", roots(device_b.path()), None).unwrap();
        assert_eq!(prepared.plan.conflicts.len(), 1);
        let conflict = prepared.plan.conflicts[0].key.clone();
        let applied = prepared
            .apply(
                &store,
                &BTreeMap::from([(conflict.clone(), ConflictChoice::Remote)]),
                recovery.path(),
                2000,
            )
            .unwrap();
        assert_eq!(
            std::fs::read(device_b.path().join("game.sav")).unwrap(),
            b"remote winner"
        );
        let recovery_directory = applied.recovery_directory.unwrap();
        assert_eq!(
            std::fs::read(recovery_directory.join("backups").join(conflict.as_str())).unwrap(),
            b"local loser"
        );
        assert!(recovery_directory.join("journal.json").is_file());
        assert!(recovery_directory.join("complete").is_file());
    }

    #[test]
    fn stale_plan_is_rejected_before_upload_or_local_mutation() {
        let store = CloudStore::memory().unwrap();
        let local = tempfile::tempdir().unwrap();
        let recovery = tempfile::tempdir().unwrap();
        let path = local.path().join("game.sav");
        std::fs::write(&path, b"planned").unwrap();
        let prepared =
            prepare_sync(&store, scope(), "device-a", roots(local.path()), None).unwrap();
        std::fs::write(&path, b"changed after plan").unwrap();
        assert!(
            prepared
                .apply(&store, &BTreeMap::new(), recovery.path(), 1000)
                .is_err()
        );
        assert!(store.device_heads(&scope()).unwrap().is_empty());
    }

    #[test]
    fn concurrent_device_head_change_rejects_a_stale_plan() {
        let store = CloudStore::memory().unwrap();
        let local = tempfile::tempdir().unwrap();
        let recovery = tempfile::tempdir().unwrap();
        std::fs::write(local.path().join("game.sav"), b"stable").unwrap();
        let first = prepare_sync(&store, scope(), "device-a", roots(local.path()), None)
            .unwrap()
            .apply(&store, &BTreeMap::new(), recovery.path(), 1000)
            .unwrap();
        let prepared =
            prepare_sync(&store, scope(), "device-a", roots(local.path()), None).unwrap();
        let advanced = SaveManifest::new(
            scope(),
            vec![first.manifest_id],
            "device-a",
            2000,
            prepared.local.files.clone(),
        )
        .unwrap();
        store.put_manifest(&advanced).unwrap();
        store
            .set_device_head(&DeviceHead::new(scope(), "device-a", advanced.id, 2000).unwrap())
            .unwrap();

        let error = prepared
            .apply(&store, &BTreeMap::new(), recovery.path(), 3000)
            .unwrap_err()
            .to_string();
        assert!(error.contains("cloud save history changed after sync planning"));
    }

    #[test]
    fn identical_new_device_adopts_remote_manifest_and_publishes_its_head() {
        let store = CloudStore::memory().unwrap();
        let device_a = tempfile::tempdir().unwrap();
        let device_b = tempfile::tempdir().unwrap();
        let recovery = tempfile::tempdir().unwrap();
        for directory in [device_a.path(), device_b.path()] {
            std::fs::write(directory.join("game.sav"), b"same save").unwrap();
        }
        let original = prepare_sync(&store, scope(), "device-a", roots(device_a.path()), None)
            .unwrap()
            .apply(&store, &BTreeMap::new(), recovery.path(), 1000)
            .unwrap();
        let adopted = prepare_sync(&store, scope(), "device-b", roots(device_b.path()), None)
            .unwrap()
            .apply(&store, &BTreeMap::new(), recovery.path(), 2000)
            .unwrap();
        assert!(adopted.actions.is_empty());
        assert_eq!(adopted.manifest_id, original.manifest_id);
        let heads = store.device_heads(&scope()).unwrap();
        assert_eq!(heads.len(), 2);
        assert!(heads.iter().any(|head| head.device_id == "device-b"));
    }

    #[test]
    fn several_remote_heads_require_an_explicit_device_selection() {
        let store = CloudStore::memory().unwrap();
        let device_c = tempfile::tempdir().unwrap();
        let route = SaveRoute {
            purpose: SavePurpose::Saves,
            root_index: 0,
        };
        for (device_id, name, contents, timestamp) in [
            ("device-a", "a.sav", b"device a".as_slice(), 1000),
            ("device-b", "b.sav", b"device b".as_slice(), 2000),
        ] {
            let version = FileVersion::from_bytes(contents, timestamp);
            store.put_blob(&scope(), &version, contents).unwrap();
            let manifest = SaveManifest::new(
                scope(),
                Vec::new(),
                device_id,
                timestamp,
                BTreeMap::from([(ArtifactKey::new(route, name).unwrap(), version)]),
            )
            .unwrap();
            store.put_manifest(&manifest).unwrap();
            store
                .set_device_head(
                    &DeviceHead::new(scope(), device_id, manifest.id, timestamp).unwrap(),
                )
                .unwrap();
        }

        let error = prepare_sync(&store, scope(), "device-c", roots(device_c.path()), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("multiple remote save histories"));
        let selected = prepare_sync(
            &store,
            scope(),
            "device-c",
            roots(device_c.path()),
            Some("device-b"),
        )
        .unwrap();
        assert_eq!(selected.plan.actions.len(), 1);
        assert_eq!(selected.plan.actions[0].kind, SyncActionKind::Download);
    }

    #[test]
    fn ancestor_device_heads_collapse_to_the_unmerged_frontier() {
        let store = CloudStore::memory().unwrap();
        let device_a = tempfile::tempdir().unwrap();
        let device_b = tempfile::tempdir().unwrap();
        let device_c = tempfile::tempdir().unwrap();
        let recovery = tempfile::tempdir().unwrap();
        std::fs::write(device_a.path().join("a.sav"), b"from a").unwrap();
        let applied_a = prepare_sync(&store, scope(), "device-a", roots(device_a.path()), None)
            .unwrap()
            .apply(&store, &BTreeMap::new(), recovery.path(), 1000)
            .unwrap();

        std::fs::write(device_b.path().join("a.sav"), b"from a").unwrap();
        std::fs::write(device_b.path().join("b.sav"), b"from b").unwrap();
        let prepared_b =
            prepare_sync(&store, scope(), "device-b", roots(device_b.path()), None).unwrap();
        assert_eq!(prepared_b.parent_ids, vec![applied_a.manifest_id]);
        let applied_b = prepared_b
            .apply(&store, &BTreeMap::new(), recovery.path(), 2000)
            .unwrap();

        let candidates = remote_merge_candidates(&store, &scope(), "device-c").unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].manifest_id, applied_b.manifest_id);
        let prepared_c =
            prepare_sync(&store, scope(), "device-c", roots(device_c.path()), None).unwrap();
        assert_eq!(prepared_c.plan.actions.len(), 2);
        prepared_c
            .apply(&store, &BTreeMap::new(), recovery.path(), 3000)
            .unwrap();

        assert!(
            remote_merge_candidates(&store, &scope(), "device-c")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn new_device_downloads_into_a_missing_captured_save_directory() {
        let store = CloudStore::memory().unwrap();
        let device_a = tempfile::tempdir().unwrap();
        let device_b_parent = tempfile::tempdir().unwrap();
        let device_b = device_b_parent.path().join("not-created-yet");
        let recovery = tempfile::tempdir().unwrap();
        std::fs::write(device_a.path().join("game.sav"), b"remote save").unwrap();
        prepare_sync(&store, scope(), "device-a", roots(device_a.path()), None)
            .unwrap()
            .apply(&store, &BTreeMap::new(), recovery.path(), 1000)
            .unwrap();

        let prepared = prepare_sync(&store, scope(), "device-b", roots(&device_b), None).unwrap();
        assert_eq!(prepared.plan.actions.len(), 1);
        assert_eq!(prepared.plan.actions[0].kind, SyncActionKind::Download);
        prepared
            .apply(&store, &BTreeMap::new(), recovery.path(), 2000)
            .unwrap();
        assert_eq!(
            std::fs::read(device_b.join("game.sav")).unwrap(),
            b"remote save"
        );
    }

    #[test]
    fn restart_recovery_rolls_back_an_interrupted_multi_file_apply() {
        let store = CloudStore::memory().unwrap();
        let local = tempfile::tempdir().unwrap();
        let recovery_base = tempfile::tempdir().unwrap();
        std::fs::write(local.path().join("a.sav"), b"old a").unwrap();
        std::fs::write(local.path().join("b.sav"), b"old b").unwrap();
        let route = SaveRoute {
            purpose: SavePurpose::Saves,
            root_index: 0,
        };
        let roots = roots(local.path());
        let root_map = root_map(&roots).unwrap();
        let mut actions = Vec::new();
        for (name, contents) in [("a.sav", b"new a".as_slice()), ("b.sav", b"new b")] {
            let version = FileVersion::from_bytes(contents, 1000);
            store.put_blob(&scope(), &version, contents).unwrap();
            actions.push(SyncAction {
                key: ArtifactKey::new(route, name).unwrap(),
                kind: SyncActionKind::Download,
                version: Some(version),
            });
        }
        let prepared = prepare_local_mutations(
            &store,
            &scope(),
            "device-a",
            &root_map,
            &actions,
            recovery_base.path(),
            1000,
            None,
            &"b".repeat(64),
        )
        .unwrap();

        publish_download(&prepared.mutations[0]).unwrap();
        assert_eq!(std::fs::read(local.path().join("a.sav")).unwrap(), b"new a");
        assert_eq!(std::fs::read(local.path().join("b.sav")).unwrap(), b"old b");

        let recovered = recover_pending_sync_mutations(
            &store,
            &scope(),
            "device-a",
            &roots,
            recovery_base.path(),
        )
        .unwrap();
        assert_eq!(recovered, vec![prepared.directory.clone()]);
        assert_eq!(std::fs::read(local.path().join("a.sav")).unwrap(), b"old a");
        assert_eq!(std::fs::read(local.path().join("b.sav")).unwrap(), b"old b");
        assert!(prepared.directory.join("journal.json").is_file());
        assert!(prepared.directory.join("backups/saves/0/a.sav").is_file());
        assert!(prepared.directory.join("backups/saves/0/b.sav").is_file());
        assert!(prepared.directory.join("rolled-back").is_file());
        assert!(!prepared.directory.join("complete").exists());
        assert!(
            recover_pending_sync_mutations(
                &store,
                &scope(),
                "device-a",
                &roots,
                recovery_base.path(),
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn restart_recovery_malformed_journal_blocks_apply_without_discarding_evidence() {
        let store = CloudStore::memory().unwrap();
        let local = tempfile::tempdir().unwrap();
        let recovery_base = tempfile::tempdir().unwrap();
        std::fs::write(local.path().join("game.sav"), b"local").unwrap();
        let prepared =
            prepare_sync(&store, scope(), "device-a", roots(local.path()), None).unwrap();
        let directory = recovery_base
            .path()
            .join(format!("1000-{}", Uuid::new_v4().simple()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("journal.json"), b"{not-json").unwrap();

        let error = prepared
            .apply(&store, &BTreeMap::new(), recovery_base.path(), 2000)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("parsing save-sync recovery journal"),
            "{error}"
        );
        assert!(directory.join("journal.json").is_file());
        assert!(!directory.join("complete").exists());
        assert!(!directory.join("rolled-back").exists());
        assert!(store.device_heads(&scope()).unwrap().is_empty());
    }

    #[test]
    fn restart_recovery_stale_target_fails_closed_and_retains_backups() {
        let store = CloudStore::memory().unwrap();
        let local = tempfile::tempdir().unwrap();
        let recovery_base = tempfile::tempdir().unwrap();
        let target = local.path().join("game.sav");
        std::fs::write(&target, b"original").unwrap();
        let route = SaveRoute {
            purpose: SavePurpose::Saves,
            root_index: 0,
        };
        let roots = roots(local.path());
        let version = FileVersion::from_bytes(b"download", 1000);
        store.put_blob(&scope(), &version, b"download").unwrap();
        let prepared = prepare_local_mutations(
            &store,
            &scope(),
            "device-a",
            &root_map(&roots).unwrap(),
            &[SyncAction {
                key: ArtifactKey::new(route, "game.sav").unwrap(),
                kind: SyncActionKind::Download,
                version: Some(version),
            }],
            recovery_base.path(),
            1000,
            None,
            &"c".repeat(64),
        )
        .unwrap();
        std::fs::write(&target, b"external change").unwrap();

        let error = recover_pending_sync_mutations(
            &store,
            &scope(),
            "device-a",
            &roots,
            recovery_base.path(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("stale or ambiguous"), "{error}");
        assert_eq!(std::fs::read(&target).unwrap(), b"external change");
        assert!(prepared.directory.join("journal.json").is_file());
        assert!(
            prepared
                .directory
                .join("backups/saves/0/game.sav")
                .is_file()
        );
        assert!(!prepared.directory.join("complete").exists());
        assert!(!prepared.directory.join("rolled-back").exists());
    }

    #[test]
    fn restart_recovery_finishes_a_committed_journal_without_rolling_back() {
        let store = CloudStore::memory().unwrap();
        let local = tempfile::tempdir().unwrap();
        let recovery_base = tempfile::tempdir().unwrap();
        let target = local.path().join("game.sav");
        std::fs::write(&target, b"old").unwrap();
        let route = SaveRoute {
            purpose: SavePurpose::Saves,
            root_index: 0,
        };
        let key = ArtifactKey::new(route, "game.sav").unwrap();
        let version = FileVersion::from_bytes(b"committed", 1000);
        store.put_blob(&scope(), &version, b"committed").unwrap();
        let manifest = SaveManifest::new(
            scope(),
            Vec::new(),
            "device-a",
            1000,
            BTreeMap::from([(key.clone(), version.clone())]),
        )
        .unwrap();
        let roots = roots(local.path());
        let prepared = prepare_local_mutations(
            &store,
            &scope(),
            "device-a",
            &root_map(&roots).unwrap(),
            &[SyncAction {
                key,
                kind: SyncActionKind::Download,
                version: Some(version),
            }],
            recovery_base.path(),
            1000,
            None,
            &manifest.id,
        )
        .unwrap();
        apply_local_mutations(&prepared).unwrap();
        store.put_manifest(&manifest).unwrap();
        store
            .set_device_head(
                &DeviceHead::new(scope(), "device-a", manifest.id.clone(), 1000).unwrap(),
            )
            .unwrap();

        let recovered = recover_pending_sync_mutations(
            &store,
            &scope(),
            "device-a",
            &roots,
            recovery_base.path(),
        )
        .unwrap();
        assert_eq!(recovered, vec![prepared.directory.clone()]);
        assert_eq!(std::fs::read(target).unwrap(), b"committed");
        assert!(prepared.directory.join("complete").is_file());
        assert!(!prepared.directory.join("rolled-back").exists());
        assert!(prepared.directory.join("journal.json").is_file());
        assert!(
            prepared
                .directory
                .join("backups/saves/0/game.sav")
                .is_file()
        );

        std::fs::write(prepared.directory.join("complete"), "c".repeat(64)).unwrap();
        let error = recover_pending_sync_mutations(
            &store,
            &scope(),
            "device-a",
            &roots,
            recovery_base.path(),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("recovery marker does not match its journal"),
            "{error}"
        );
    }

    #[test]
    fn legacy_completed_journal_is_accepted_but_legacy_pending_journal_is_not_replayed() {
        let store = CloudStore::memory().unwrap();
        let local = tempfile::tempdir().unwrap();
        let recovery_base = tempfile::tempdir().unwrap();
        let directory = recovery_base
            .path()
            .join(format!("1000-{}", Uuid::new_v4().simple()));
        std::fs::create_dir(&directory).unwrap();
        let key = ArtifactKey::new(
            SaveRoute {
                purpose: SavePurpose::Saves,
                root_index: 0,
            },
            "game.sav",
        )
        .unwrap();
        let version = FileVersion::from_bytes(b"committed", 1000);
        store.put_blob(&scope(), &version, b"committed").unwrap();
        let manifest = SaveManifest::new(
            scope(),
            Vec::new(),
            "device-a",
            1000,
            BTreeMap::from([(key.clone(), version)]),
        )
        .unwrap();
        store.put_manifest(&manifest).unwrap();
        let mutation = RecoveryMutation {
            key,
            kind: SyncActionKind::Download,
            target: local.path().join("game.sav"),
            backup: None,
            staged_download: Some(directory.join("downloads/0.part")),
        };
        let journal = serde_json::json!({
            "schema": LEGACY_TERMINAL_RECOVERY_SCHEMA,
            "scope": scope(),
            "device_id": "device-a",
            "created_unix_ms": 1000,
            "mutations": [mutation],
        });
        std::fs::write(
            directory.join("journal.json"),
            serde_json::to_vec_pretty(&journal).unwrap(),
        )
        .unwrap();
        std::fs::write(directory.join("complete"), &manifest.id).unwrap();

        assert!(
            recover_pending_sync_mutations(
                &store,
                &scope(),
                "device-a",
                &roots(local.path()),
                recovery_base.path(),
            )
            .unwrap()
            .is_empty()
        );

        std::fs::remove_file(directory.join("complete")).unwrap();
        let error = recover_pending_sync_mutations(
            &store,
            &scope(),
            "device-a",
            &roots(local.path()),
            recovery_base.path(),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("parsing save-sync recovery journal"),
            "{error}"
        );
        assert!(directory.join("journal.json").is_file());
        assert!(!directory.join("complete").exists());
        assert!(!directory.join("rolled-back").exists());
    }
}
