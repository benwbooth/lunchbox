//! End-to-end save synchronization coordinator and recoverable local apply.

use crate::save_cloud::{CloudStore, DeviceHead};
use crate::save_sync::{
    ArtifactKey, ConflictChoice, LocalInventory, RouteRoot, SaveManifest, SaveRoute, SyncAction,
    SyncActionKind, SyncPlan, SyncScope, merged_files, plan_three_way, resolve_conflicts,
    scan_local,
};
use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
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

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct RecoveryJournal {
    schema: u32,
    scope: SyncScope,
    device_id: String,
    created_unix_ms: i64,
    mutations: Vec<RecoveryMutation>,
}

#[derive(Clone, Debug, Serialize)]
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
            )?;
            apply_local_mutations(&recovery)?;
            recovery_directory = Some(recovery.directory.clone());
        }

        let files = merged_files(&self.local, self.remote.as_ref(), &actions)?;
        let manifest = SaveManifest::new(
            self.scope.clone(),
            self.parent_ids.clone(),
            self.device_id.clone(),
            now_unix_ms,
            files,
        )?;
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

fn prepare_local_mutations(
    store: &CloudStore,
    scope: &SyncScope,
    device_id: &str,
    roots: &BTreeMap<SaveRoute, PathBuf>,
    actions: &[SyncAction],
    recovery_base: &Path,
    now_unix_ms: i64,
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
        schema: 1,
        scope: scope.clone(),
        device_id: device_id.to_owned(),
        created_unix_ms: now_unix_ms,
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
    let mut map = BTreeMap::new();
    for root in roots {
        ensure!(root.path.is_absolute(), "save root must be absolute");
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
}
