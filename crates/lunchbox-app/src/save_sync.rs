//! Provider-neutral save and save-state synchronization primitives.
//!
//! The engine deliberately separates content comparison from cloud transport
//! and UI. A sync is scoped to one exact emulator/runtime pair, compares both
//! sides to a common ancestor, and refuses to guess when both sides changed.
//! Modification times are carried for conflict presentation only; hashes are
//! the content identity.

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const MANIFEST_SCHEMA: u32 = 1;
const MAX_FILES: usize = 4096;
const MAX_DEPTH: usize = 8;
const MAX_FILE_SIZE: u64 = 16 * 1024 * 1024 * 1024;
const MAX_TOTAL_SIZE: u64 = 64 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncScope {
    pub emulator_slug: String,
    pub runtime_platform: String,
}

impl SyncScope {
    pub fn new(
        emulator_slug: impl Into<String>,
        runtime_platform: impl Into<String>,
    ) -> Result<Self> {
        let scope = Self {
            emulator_slug: emulator_slug.into(),
            runtime_platform: runtime_platform.into(),
        };
        scope.validate()?;
        Ok(scope)
    }

    pub fn validate(&self) -> Result<()> {
        validate_token("emulator slug", &self.emulator_slug)?;
        ensure!(
            matches!(
                self.runtime_platform.as_str(),
                "linux" | "linux-flatpak" | "macos" | "windows"
            ),
            "unsupported save-sync runtime platform {}",
            self.runtime_platform
        );
        Ok(())
    }

    pub fn remote_prefix(&self) -> String {
        format!("saves/v1/{}/{}", self.emulator_slug, self.runtime_platform)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavePurpose {
    Saves,
    States,
}

impl SavePurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Saves => "saves",
            Self::States => "states",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRoute {
    pub purpose: SavePurpose,
    pub root_index: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactKey(String);

impl ArtifactKey {
    pub fn new(route: SaveRoute, relative_path: &str) -> Result<Self> {
        validate_portable_relative_path(relative_path)?;
        let value = if relative_path.is_empty() {
            format!("{}/{}", route.purpose.as_str(), route.root_index)
        } else {
            format!(
                "{}/{}/{}",
                route.purpose.as_str(),
                route.root_index,
                relative_path
            )
        };
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn route(&self) -> Result<SaveRoute> {
        let mut parts = self.0.splitn(3, '/');
        let purpose = match parts.next() {
            Some("saves") => SavePurpose::Saves,
            Some("states") => SavePurpose::States,
            _ => bail!("invalid save artifact purpose in {}", self.0),
        };
        let root_index = parts
            .next()
            .context("save artifact is missing its root index")?
            .parse::<u16>()
            .context("save artifact has an invalid root index")?;
        if let Some(relative) = parts.next() {
            validate_portable_relative_path(relative)?;
            ensure!(
                !relative.is_empty(),
                "artifact path has an empty relative path"
            );
        }
        Ok(SaveRoute {
            purpose,
            root_index,
        })
    }

    pub fn relative_path(&self) -> Result<&str> {
        let mut parts = self.0.splitn(3, '/');
        let _ = parts.next();
        let _ = parts.next();
        let relative = parts.next().unwrap_or("");
        validate_portable_relative_path(relative)?;
        Ok(relative)
    }

    fn validate(&self) -> Result<()> {
        ensure!(self.0.len() <= 540, "artifact key is too long");
        let route = self.route()?;
        ensure!(
            Self::new(route, self.relative_path()?)?.0 == self.0,
            "artifact key is not canonical"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileVersion {
    pub sha256: String,
    pub size: u64,
    pub modified_unix_ms: i64,
}

impl FileVersion {
    pub fn from_bytes(bytes: &[u8], modified_unix_ms: i64) -> Self {
        Self {
            sha256: hex::encode(Sha256::digest(bytes)),
            size: bytes.len() as u64,
            modified_unix_ms,
        }
    }

    fn validate(&self) -> Result<()> {
        validate_sha256(&self.sha256)?;
        ensure!(self.size <= MAX_FILE_SIZE, "save artifact exceeds 16 GiB");
        ensure!(
            self.modified_unix_ms >= 0,
            "save artifact has a pre-epoch modification time"
        );
        Ok(())
    }

    fn same_content(&self, other: &Self) -> bool {
        self.sha256 == other.sha256 && self.size == other.size
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalInventory {
    pub scope: SyncScope,
    /// Only routes supplied by the platform resolver and safely scanned. A
    /// declared root that does not exist yet is an available empty route so a
    /// newly installed emulator can receive remote saves. Unresolved or
    /// unsupported roots are never supplied and therefore remain unavailable.
    pub available_routes: BTreeSet<SaveRoute>,
    pub files: BTreeMap<ArtifactKey, FileVersion>,
}

impl LocalInventory {
    pub fn validate(&self) -> Result<()> {
        self.scope.validate()?;
        validate_files(&self.files)?;
        for key in self.files.keys() {
            ensure!(
                self.available_routes.contains(&key.route()?),
                "artifact {} belongs to an unavailable local route",
                key.as_str()
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct RouteRoot {
    pub route: SaveRoute,
    pub path: PathBuf,
    /// True only for a fixed, machine-resolvable captured location whose
    /// absence means the emulator has not created it yet. Dynamic/user-chosen
    /// or removable paths must leave this false so absence stays unavailable.
    pub create_if_missing: bool,
}

/// Validate the local filesystem roots before any inventory or mutation.
///
/// Route IDs keep cloud artifacts distinct, but they do not make overlapping
/// local paths independent. Equal or nested roots could otherwise scan and
/// later mutate the same physical file through two different artifact keys.
pub fn validate_route_roots(roots: &[RouteRoot]) -> Result<()> {
    let mut routes = BTreeSet::new();
    for root in roots {
        ensure!(root.path.is_absolute(), "save root must be absolute");
        ensure!(
            !root
                .path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir)),
            "save root may not contain parent-directory components: {}",
            root.path.display()
        );
        ensure!(routes.insert(root.route), "duplicate save route");
    }

    for (index, left) in roots.iter().enumerate() {
        for right in &roots[index + 1..] {
            ensure!(
                !left.path.starts_with(&right.path) && !right.path.starts_with(&left.path),
                "save route roots overlap: {} and {}",
                left.path.display(),
                right.path.display()
            );
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveManifest {
    pub schema: u32,
    pub id: String,
    pub scope: SyncScope,
    pub parents: Vec<String>,
    pub device_id: String,
    pub created_unix_ms: i64,
    pub files: BTreeMap<ArtifactKey, FileVersion>,
}

#[derive(Serialize)]
struct ManifestBody<'a> {
    schema: u32,
    scope: &'a SyncScope,
    parents: &'a [String],
    device_id: &'a str,
    created_unix_ms: i64,
    files: &'a BTreeMap<ArtifactKey, FileVersion>,
}

impl SaveManifest {
    pub fn new(
        scope: SyncScope,
        mut parents: Vec<String>,
        device_id: impl Into<String>,
        created_unix_ms: i64,
        files: BTreeMap<ArtifactKey, FileVersion>,
    ) -> Result<Self> {
        parents.sort();
        parents.dedup();
        let mut manifest = Self {
            schema: MANIFEST_SCHEMA,
            id: String::new(),
            scope,
            parents,
            device_id: device_id.into(),
            created_unix_ms,
            files,
        };
        manifest.validate_body()?;
        manifest.id = manifest.computed_id()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_body()?;
        validate_sha256(&self.id)?;
        ensure!(self.id == self.computed_id()?, "save manifest ID mismatch");
        ensure!(
            !self.parents.iter().any(|parent| parent == &self.id),
            "save manifest cannot parent itself"
        );
        Ok(())
    }

    fn validate_body(&self) -> Result<()> {
        ensure!(
            self.schema == MANIFEST_SCHEMA,
            "unsupported save manifest schema {}",
            self.schema
        );
        self.scope.validate()?;
        validate_token("device ID", &self.device_id)?;
        ensure!(
            self.created_unix_ms >= 0,
            "save manifest has a pre-epoch timestamp"
        );
        ensure!(
            self.parents.len() <= 8,
            "save manifest has too many parents"
        );
        let mut unique = BTreeSet::new();
        for parent in &self.parents {
            validate_sha256(parent)?;
            ensure!(unique.insert(parent), "duplicate save manifest parent");
        }
        ensure!(
            self.parents.windows(2).all(|pair| pair[0] < pair[1]),
            "save manifest parents are not canonical"
        );
        validate_files(&self.files)
    }

    fn computed_id(&self) -> Result<String> {
        let bytes = serde_json::to_vec(&ManifestBody {
            schema: self.schema,
            scope: &self.scope,
            parents: &self.parents,
            device_id: &self.device_id,
            created_unix_ms: self.created_unix_ms,
            files: &self.files,
        })
        .context("encoding canonical save manifest")?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncActionKind {
    Upload,
    Download,
    DeleteLocal,
    DeleteRemote,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncAction {
    pub key: ArtifactKey,
    pub kind: SyncActionKind,
    /// Expected content for uploads/downloads. Delete actions carry `None`.
    pub version: Option<FileVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncConflict {
    pub key: ArtifactKey,
    pub local: Option<FileVersion>,
    pub remote: Option<FileVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncPlan {
    pub actions: Vec<SyncAction>,
    pub conflicts: Vec<SyncConflict>,
}

impl SyncPlan {
    pub fn requires_user_choice(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictChoice {
    Local,
    Remote,
}

pub fn plan_three_way(
    base: Option<&SaveManifest>,
    local: &LocalInventory,
    remote: Option<&SaveManifest>,
) -> Result<SyncPlan> {
    local.validate()?;
    for manifest in [base, remote].into_iter().flatten() {
        manifest.validate()?;
        ensure!(
            manifest.scope == local.scope,
            "save manifest scope does not match local runtime"
        );
    }

    let mut keys = BTreeSet::new();
    keys.extend(local.files.keys().cloned());
    if let Some(base) = base {
        keys.extend(base.files.keys().cloned());
    }
    if let Some(remote) = remote {
        keys.extend(remote.files.keys().cloned());
    }

    let mut plan = SyncPlan::default();
    for key in keys {
        if !local.available_routes.contains(&key.route()?) {
            // The runtime/root was not safely available. Treating this as an
            // empty directory would turn an unmounted disk into mass deletion.
            continue;
        }
        let ancestor = base.and_then(|manifest| manifest.files.get(&key));
        let local_version = local.files.get(&key);
        let remote_version = remote.and_then(|manifest| manifest.files.get(&key));

        if same_optional_content(local_version, remote_version) {
            continue;
        }
        let local_changed = !same_optional_content(local_version, ancestor);
        let remote_changed = !same_optional_content(remote_version, ancestor);
        match (local_changed, remote_changed) {
            (true, true) => plan.conflicts.push(SyncConflict {
                key,
                local: local_version.cloned(),
                remote: remote_version.cloned(),
            }),
            (true, false) => plan
                .actions
                .push(action_for_local(key, local_version.cloned())),
            (false, true) => plan
                .actions
                .push(action_for_remote(key, remote_version.cloned())),
            (false, false) => unreachable!("different content must have a changed side"),
        }
    }
    Ok(plan)
}

pub fn resolve_conflicts(
    plan: &SyncPlan,
    choices: &BTreeMap<ArtifactKey, ConflictChoice>,
) -> Result<Vec<SyncAction>> {
    let conflict_keys = plan
        .conflicts
        .iter()
        .map(|conflict| conflict.key.clone())
        .collect::<BTreeSet<_>>();
    ensure!(
        choices.keys().all(|key| conflict_keys.contains(key)),
        "conflict choices contain an unknown artifact"
    );
    ensure!(
        choices.len() == plan.conflicts.len(),
        "every sync conflict requires an explicit Local or Remote choice"
    );

    let mut actions = plan.actions.clone();
    for conflict in &plan.conflicts {
        match choices
            .get(&conflict.key)
            .context("sync conflict has no user choice")?
        {
            ConflictChoice::Local => actions.push(action_for_local(
                conflict.key.clone(),
                conflict.local.clone(),
            )),
            ConflictChoice::Remote => actions.push(action_for_remote(
                conflict.key.clone(),
                conflict.remote.clone(),
            )),
        }
    }
    actions.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(actions)
}

pub fn merged_files(
    local: &LocalInventory,
    remote: Option<&SaveManifest>,
    actions: &[SyncAction],
) -> Result<BTreeMap<ArtifactKey, FileVersion>> {
    local.validate()?;
    let mut files = remote
        .map(|manifest| manifest.files.clone())
        .unwrap_or_default();
    for (key, version) in &local.files {
        if local.available_routes.contains(&key.route()?) {
            files.insert(key.clone(), version.clone());
        }
    }
    for action in actions {
        match action.kind {
            SyncActionKind::Upload | SyncActionKind::Download => {
                files.insert(
                    action.key.clone(),
                    action
                        .version
                        .clone()
                        .context("copy action is missing its file version")?,
                );
            }
            SyncActionKind::DeleteLocal | SyncActionKind::DeleteRemote => {
                files.remove(&action.key);
            }
        }
    }
    validate_files(&files)?;
    Ok(files)
}

pub fn scan_local(scope: SyncScope, roots: &[RouteRoot]) -> Result<LocalInventory> {
    scope.validate()?;
    validate_route_roots(roots)?;
    let mut routes = BTreeSet::new();
    let mut files = BTreeMap::new();
    for root in roots {
        ensure_existing_ancestors_without_symlink(&root.path)?;
        routes.insert(root.route);
        let metadata = match std::fs::symlink_metadata(&root.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if !root.create_if_missing {
                    routes.remove(&root.route);
                }
                continue;
            }
            Err(error) => return Err(error).context("reading save root metadata"),
        };
        ensure!(
            !metadata.file_type().is_symlink(),
            "save root may not be a symbolic link: {}",
            root.path.display()
        );
        if metadata.is_file() {
            let key = ArtifactKey::new(root.route, "")?;
            files.insert(key, hash_stable_file(&root.path)?);
        } else if metadata.is_dir() {
            scan_directory(&root.path, &root.path, root.route, 0, &mut files)?;
        } else {
            bail!("save root is neither a file nor directory");
        }
    }
    let inventory = LocalInventory {
        scope,
        available_routes: routes,
        files,
    };
    inventory.validate()?;
    Ok(inventory)
}

fn ensure_existing_ancestors_without_symlink(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "save path contains a symbolic link: {}",
                current.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error).context("reading save path ancestor metadata"),
        }
    }
    Ok(())
}

fn scan_directory(
    root: &Path,
    directory: &Path,
    route: SaveRoute,
    depth: usize,
    files: &mut BTreeMap<ArtifactKey, FileVersion>,
) -> Result<()> {
    ensure!(
        depth <= MAX_DEPTH,
        "save directory nesting exceeds {MAX_DEPTH}"
    );
    let mut entries = std::fs::read_dir(directory)
        .with_context(|| format!("reading save directory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .with_context(|| format!("reading save path metadata {}", path.display()))?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "symbolic links are not allowed in save roots: {}",
            path.display()
        );
        if metadata.is_dir() {
            scan_directory(root, &path, route, depth + 1, files)?;
        } else if metadata.is_file() {
            ensure!(
                files.len() < MAX_FILES,
                "save inventory exceeds {MAX_FILES} files"
            );
            let relative = path
                .strip_prefix(root)
                .context("save file escaped its declared root")?;
            let portable = path_to_portable(relative)?;
            let key = ArtifactKey::new(route, &portable)?;
            ensure!(
                files.insert(key, hash_stable_file(&path)?).is_none(),
                "duplicate save artifact key"
            );
        } else {
            bail!("unsupported special file in save root: {}", path.display());
        }
    }
    Ok(())
}

fn hash_stable_file(path: &Path) -> Result<FileVersion> {
    let mut file =
        File::open(path).with_context(|| format!("opening save file {}", path.display()))?;
    let before = file.metadata().context("reading save file metadata")?;
    ensure!(
        before.len() <= MAX_FILE_SIZE,
        "save artifact exceeds 16 GiB"
    );
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).context("hashing save file")?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let after = file.metadata().context("rechecking save file metadata")?;
    ensure!(
        before.len() == after.len() && before.modified()? == after.modified()?,
        "save file changed during snapshot: {}",
        path.display()
    );
    let modified_unix_ms = after
        .modified()
        .context("save file has no modification time")?
        .duration_since(UNIX_EPOCH)
        .context("save file has a pre-epoch modification time")?
        .as_millis()
        .try_into()
        .context("save file modification time is out of range")?;
    Ok(FileVersion {
        sha256: hex::encode(hasher.finalize()),
        size: after.len(),
        modified_unix_ms,
    })
}

fn action_for_local(key: ArtifactKey, version: Option<FileVersion>) -> SyncAction {
    match version {
        Some(version) => SyncAction {
            key,
            kind: SyncActionKind::Upload,
            version: Some(version),
        },
        None => SyncAction {
            key,
            kind: SyncActionKind::DeleteRemote,
            version: None,
        },
    }
}

fn action_for_remote(key: ArtifactKey, version: Option<FileVersion>) -> SyncAction {
    match version {
        Some(version) => SyncAction {
            key,
            kind: SyncActionKind::Download,
            version: Some(version),
        },
        None => SyncAction {
            key,
            kind: SyncActionKind::DeleteLocal,
            version: None,
        },
    }
}

fn same_optional_content(left: Option<&FileVersion>, right: Option<&FileVersion>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.same_content(right),
        (None, None) => true,
        _ => false,
    }
}

fn validate_files(files: &BTreeMap<ArtifactKey, FileVersion>) -> Result<()> {
    ensure!(
        files.len() <= MAX_FILES,
        "save manifest exceeds {MAX_FILES} files"
    );
    let mut total = 0_u64;
    for (key, version) in files {
        key.validate()?;
        version.validate()?;
        total = total
            .checked_add(version.size)
            .context("save manifest size overflow")?;
    }
    ensure!(total <= MAX_TOTAL_SIZE, "save manifest exceeds 64 GiB");
    Ok(())
}

fn validate_sha256(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "expected a lowercase SHA-256 digest"
    );
    Ok(())
}

fn validate_token(label: &str, value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 80
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte)),
        "{label} must be a bounded portable identifier"
    );
    Ok(())
}

fn path_to_portable(path: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        let std::path::Component::Normal(component) = component else {
            bail!("save path contains a non-portable component");
        };
        parts.push(
            component
                .to_str()
                .context("save path is not valid UTF-8")?
                .to_owned(),
        );
    }
    let value = parts.join("/");
    validate_portable_relative_path(&value)?;
    ensure!(!value.is_empty(), "save path is empty");
    Ok(value)
}

fn validate_portable_relative_path(value: &str) -> Result<()> {
    ensure!(value.len() <= 512, "save path is too long");
    ensure!(
        !value.starts_with('/') && !value.contains('\\'),
        "save path is not relative POSIX form"
    );
    if value.is_empty() {
        return Ok(());
    }
    for part in value.split('/') {
        ensure!(
            !part.is_empty() && part != "." && part != "..",
            "save path contains traversal"
        );
        ensure!(
            !part.chars().any(|character| {
                character.is_control()
                    || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
            }),
            "save path contains characters forbidden on Windows"
        );
        ensure!(
            !part.ends_with([' ', '.']),
            "save path has a Windows-incompatible suffix"
        );
        let stem = part.split('.').next().unwrap_or(part).to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && matches!(&stem[..3], "COM" | "LPT")
                && matches!(stem.as_bytes()[3], b'1'..=b'9'));
        ensure!(!reserved, "save path uses a Windows-reserved name");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> SyncScope {
        SyncScope::new("duckstation", "linux").unwrap()
    }

    fn route() -> SaveRoute {
        SaveRoute {
            purpose: SavePurpose::Saves,
            root_index: 0,
        }
    }

    fn key(name: &str) -> ArtifactKey {
        ArtifactKey::new(route(), name).unwrap()
    }

    fn version(contents: &[u8], timestamp: i64) -> FileVersion {
        FileVersion::from_bytes(contents, timestamp)
    }

    fn inventory(files: BTreeMap<ArtifactKey, FileVersion>) -> LocalInventory {
        LocalInventory {
            scope: scope(),
            available_routes: BTreeSet::from([route()]),
            files,
        }
    }

    fn manifest(files: BTreeMap<ArtifactKey, FileVersion>) -> SaveManifest {
        SaveManifest::new(scope(), Vec::new(), "device-a", 1000, files).unwrap()
    }

    #[test]
    fn first_sync_uploads_and_downloads_without_guessing_differences() {
        let local_key = key("local.sav");
        let remote_key = key("remote.sav");
        let conflict_key = key("both.sav");
        let local = inventory(BTreeMap::from([
            (local_key.clone(), version(b"local", 10)),
            (conflict_key.clone(), version(b"local version", 20)),
        ]));
        let remote = manifest(BTreeMap::from([
            (remote_key.clone(), version(b"remote", 30)),
            (conflict_key.clone(), version(b"remote version", 40)),
        ]));
        let plan = plan_three_way(None, &local, Some(&remote)).unwrap();
        assert_eq!(plan.actions.len(), 2);
        assert!(
            plan.actions
                .iter()
                .any(|action| action.key == local_key && action.kind == SyncActionKind::Upload)
        );
        assert!(
            plan.actions
                .iter()
                .any(|action| action.key == remote_key && action.kind == SyncActionKind::Download)
        );
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].key, conflict_key);
        assert_eq!(
            plan.conflicts[0].local.as_ref().unwrap().modified_unix_ms,
            20
        );
        assert_eq!(
            plan.conflicts[0].remote.as_ref().unwrap().modified_unix_ms,
            40
        );
    }

    #[test]
    fn three_way_changes_and_deletions_follow_the_unchanged_side() {
        let remote_edit = key("remote-edit.sav");
        let remote_delete = key("remote-delete.sav");
        let local_edit = key("local-edit.sav");
        let local_delete = key("local-delete.sav");
        let old = version(b"old", 10);
        let base_files = BTreeMap::from([
            (remote_edit.clone(), old.clone()),
            (remote_delete.clone(), old.clone()),
            (local_edit.clone(), old.clone()),
            (local_delete.clone(), old.clone()),
        ]);
        let base = manifest(base_files.clone());
        let local = inventory(BTreeMap::from([
            (remote_edit.clone(), old.clone()),
            (local_edit.clone(), version(b"new local", 20)),
            (remote_delete.clone(), old.clone()),
        ]));
        let remote = manifest(BTreeMap::from([
            (remote_edit.clone(), version(b"new remote", 30)),
            (local_edit.clone(), old.clone()),
            (local_delete.clone(), old),
        ]));
        let plan = plan_three_way(Some(&base), &local, Some(&remote)).unwrap();
        assert!(plan.conflicts.is_empty());
        let kinds = plan
            .actions
            .iter()
            .map(|action| (action.key.clone(), action.kind))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(kinds[&remote_edit], SyncActionKind::Download);
        assert_eq!(kinds[&remote_delete], SyncActionKind::DeleteLocal);
        assert_eq!(kinds[&local_edit], SyncActionKind::Upload);
        assert_eq!(kinds[&local_delete], SyncActionKind::DeleteRemote);
    }

    #[test]
    fn both_changed_and_delete_versus_edit_are_conflicts() {
        let edited = key("edited.sav");
        let deleted = key("deleted.sav");
        let base = manifest(BTreeMap::from([
            (edited.clone(), version(b"old", 1)),
            (deleted.clone(), version(b"old", 1)),
        ]));
        let local = inventory(BTreeMap::from([(edited.clone(), version(b"local", 2))]));
        let remote = manifest(BTreeMap::from([
            (edited.clone(), version(b"remote", 3)),
            (deleted.clone(), version(b"remote edit", 4)),
        ]));
        let plan = plan_three_way(Some(&base), &local, Some(&remote)).unwrap();
        assert_eq!(plan.conflicts.len(), 2);
        assert!(plan.conflicts.iter().any(|conflict| conflict.key == edited));
        assert!(
            plan.conflicts
                .iter()
                .any(|conflict| conflict.key == deleted && conflict.local.is_none())
        );
    }

    #[test]
    fn conflict_resolution_requires_exact_local_or_remote_choices() {
        let artifact = key("game.sav");
        let plan = SyncPlan {
            actions: Vec::new(),
            conflicts: vec![SyncConflict {
                key: artifact.clone(),
                local: Some(version(b"local", 10)),
                remote: Some(version(b"remote", 20)),
            }],
        };
        assert!(resolve_conflicts(&plan, &BTreeMap::new()).is_err());
        let resolved = resolve_conflicts(
            &plan,
            &BTreeMap::from([(artifact.clone(), ConflictChoice::Remote)]),
        )
        .unwrap();
        assert_eq!(resolved[0].kind, SyncActionKind::Download);
        assert_eq!(resolved[0].version.as_ref().unwrap().modified_unix_ms, 20);
        let extra = key("extra.sav");
        assert!(
            resolve_conflicts(
                &plan,
                &BTreeMap::from([
                    (artifact, ConflictChoice::Local),
                    (extra, ConflictChoice::Remote),
                ])
            )
            .is_err()
        );
    }

    #[test]
    fn unavailable_routes_never_become_deletions_or_downloads() {
        let artifact = key("game.sav");
        let base = manifest(BTreeMap::from([(artifact.clone(), version(b"old", 1))]));
        let remote = manifest(BTreeMap::from([(artifact, version(b"new", 2))]));
        let local = LocalInventory {
            scope: scope(),
            available_routes: BTreeSet::new(),
            files: BTreeMap::new(),
        };
        assert_eq!(
            plan_three_way(Some(&base), &local, Some(&remote)).unwrap(),
            SyncPlan::default()
        );
    }

    #[test]
    fn content_identity_ignores_timestamps() {
        let artifact = key("game.sav");
        let local = inventory(BTreeMap::from([(artifact.clone(), version(b"same", 10))]));
        let remote = manifest(BTreeMap::from([(artifact, version(b"same", 999))]));
        assert_eq!(
            plan_three_way(None, &local, Some(&remote)).unwrap(),
            SyncPlan::default()
        );
    }

    #[test]
    fn manifest_ids_are_canonical_and_tamper_evident() {
        let files = BTreeMap::from([(key("game.sav"), version(b"save", 10))]);
        let left = SaveManifest::new(
            scope(),
            vec!["b".repeat(64), "a".repeat(64)],
            "device-a",
            100,
            files.clone(),
        )
        .unwrap();
        let right = SaveManifest::new(
            scope(),
            vec!["a".repeat(64), "b".repeat(64)],
            "device-a",
            100,
            files,
        )
        .unwrap();
        assert_eq!(left.id, right.id);
        let mut tampered = left;
        tampered.files.get_mut(&key("game.sav")).unwrap().size += 1;
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn scanner_hashes_files_and_rejects_symlinks() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("slot")).unwrap();
        std::fs::write(directory.path().join("slot/game.sav"), b"save").unwrap();
        let inventory = scan_local(
            scope(),
            &[RouteRoot {
                route: route(),
                path: directory.path().to_path_buf(),
                create_if_missing: false,
            }],
        )
        .unwrap();
        assert_eq!(
            inventory.files[&key("slot/game.sav")].sha256,
            hex::encode(Sha256::digest(b"save"))
        );

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                directory.path().join("slot/game.sav"),
                directory.path().join("linked.sav"),
            )
            .unwrap();
            assert!(
                scan_local(
                    scope(),
                    &[RouteRoot {
                        route: route(),
                        path: directory.path().to_path_buf(),
                        create_if_missing: false,
                    }],
                )
                .is_err()
            );
        }
    }

    #[test]
    fn scanner_rejects_equal_ancestor_and_descendant_route_roots() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("saves");
        let child = parent.join("states");
        std::fs::create_dir_all(&child).unwrap();
        let saves = SaveRoute {
            purpose: SavePurpose::Saves,
            root_index: 0,
        };
        let states = SaveRoute {
            purpose: SavePurpose::States,
            root_index: 0,
        };

        for (label, left, right) in [
            ("equal", parent.clone(), parent.clone()),
            ("ancestor", parent.clone(), child.clone()),
            ("descendant", child.clone(), parent.clone()),
        ] {
            let error = scan_local(
                scope(),
                &[
                    RouteRoot {
                        route: saves,
                        path: left,
                        create_if_missing: true,
                    },
                    RouteRoot {
                        route: states,
                        path: right,
                        create_if_missing: true,
                    },
                ],
            )
            .unwrap_err()
            .to_string();
            assert!(
                error.contains("save route roots overlap"),
                "{label}: {error}"
            );
        }
    }

    #[test]
    fn scanner_accepts_disjoint_sibling_route_roots() {
        let directory = tempfile::tempdir().unwrap();
        let saves = directory.path().join("saves");
        let states = directory.path().join("states");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::create_dir_all(&states).unwrap();
        std::fs::write(saves.join("game.sav"), b"save").unwrap();
        std::fs::write(states.join("game.state"), b"state").unwrap();

        let inventory = scan_local(
            scope(),
            &[
                RouteRoot {
                    route: SaveRoute {
                        purpose: SavePurpose::Saves,
                        root_index: 0,
                    },
                    path: saves,
                    create_if_missing: true,
                },
                RouteRoot {
                    route: SaveRoute {
                        purpose: SavePurpose::States,
                        root_index: 0,
                    },
                    path: states,
                    create_if_missing: true,
                },
            ],
        )
        .unwrap();

        assert_eq!(inventory.files.len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn scanner_rejects_a_symbolic_link_in_the_root_ancestry() {
        use std::os::unix::fs::symlink;

        let parent = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir(outside.path().join("saves")).unwrap();
        std::fs::write(outside.path().join("saves/game.sav"), b"outside").unwrap();
        symlink(outside.path(), parent.path().join("linked")).unwrap();
        let error = scan_local(
            scope(),
            &[RouteRoot {
                route: route(),
                path: parent.path().join("linked/saves"),
                create_if_missing: true,
            }],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("symbolic link"));
    }

    #[test]
    fn scanner_distinguishes_creatable_captured_roots_from_unavailable_paths() {
        let directory = tempfile::tempdir().unwrap();
        let creatable = SaveRoute {
            purpose: SavePurpose::Saves,
            root_index: 0,
        };
        let unavailable = SaveRoute {
            purpose: SavePurpose::States,
            root_index: 0,
        };
        let inventory = scan_local(
            scope(),
            &[
                RouteRoot {
                    route: creatable,
                    path: directory.path().join("fixed-save-directory"),
                    create_if_missing: true,
                },
                RouteRoot {
                    route: unavailable,
                    path: directory.path().join("user-selected-or-removable"),
                    create_if_missing: false,
                },
            ],
        )
        .unwrap();
        assert!(inventory.available_routes.contains(&creatable));
        assert!(!inventory.available_routes.contains(&unavailable));
    }

    #[test]
    fn portable_paths_reject_traversal_and_windows_incompatibilities() {
        for invalid in [
            "../save",
            "a//b",
            "slot\\save",
            "CON",
            "bad:name",
            "trail. ",
        ] {
            assert!(ArtifactKey::new(route(), invalid).is_err(), "{invalid}");
        }
        assert!(ArtifactKey::new(route(), "Game 1/slot-01.sav").is_ok());
        assert!(ArtifactKey::new(route(), "").is_ok());
    }
}
