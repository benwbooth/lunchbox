//! OpenDAL-backed cloud storage for save synchronization.
//!
//! Blobs and manifests are immutable and content-addressed. Each installation
//! updates only its own device-head pointer, avoiding a cross-provider global
//! compare-and-swap requirement (Google Drive and Dropbox do not expose the
//! conditional-write primitive that OneDrive does).

use crate::save_sync::{FileVersion, SaveManifest, SyncScope};
use anyhow::{Context, Result, ensure};
use opendal::layers::{RetryLayer, TimeoutLayer};
#[cfg(test)]
use opendal::services::Memory;
use opendal::services::{Dropbox, Gdrive, Onedrive};
use opendal::{ErrorKind, Operator, blocking};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const HEAD_SCHEMA: u32 = 1;
const PROFILE_SCHEMA: u32 = 1;
const MAX_GRAPH_MANIFESTS: usize = 512;
pub const DEFAULT_CLOUD_ROOT: &str = "/Lunchbox Save Sync";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudProvider {
    GoogleDrive,
    Dropbox,
    OneDrive,
}

impl CloudProvider {
    pub fn key(self) -> &'static str {
        match self {
            Self::GoogleDrive => "google_drive",
            Self::Dropbox => "dropbox",
            Self::OneDrive => "one_drive",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::GoogleDrive => "Google Drive",
            Self::Dropbox => "Dropbox",
            Self::OneDrive => "OneDrive",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        Ok(match value.trim() {
            "google_drive" => Self::GoogleDrive,
            "dropbox" => Self::Dropbox,
            "one_drive" => Self::OneDrive,
            _ => anyhow::bail!("unknown cloud-save provider"),
        })
    }
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudAuth {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

impl std::fmt::Debug for CloudAuth {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CloudAuth")
            .field(
                "access_token",
                &self.access_token.as_ref().map(|_| "[redacted]"),
            )
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("client_id", &self.client_id)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

impl CloudAuth {
    pub fn validate(&self, provider: CloudProvider) -> Result<()> {
        let has_access = nonempty(&self.access_token);
        let has_refresh = nonempty(&self.refresh_token);
        ensure!(
            has_access ^ has_refresh,
            "cloud authentication requires exactly one access or refresh token"
        );
        if has_refresh {
            ensure!(
                nonempty(&self.client_id),
                "refresh-token authentication requires a client ID"
            );
            if matches!(
                provider,
                CloudProvider::GoogleDrive | CloudProvider::Dropbox
            ) {
                ensure!(
                    nonempty(&self.client_secret),
                    "this provider requires a client secret with its refresh token"
                );
            }
        }
        for (label, value) in [
            ("access token", &self.access_token),
            ("refresh token", &self.refresh_token),
            ("client ID", &self.client_id),
            ("client secret", &self.client_secret),
        ] {
            if let Some(value) = value {
                ensure!(!value.trim().is_empty(), "{label} may not be blank");
                ensure!(value.len() <= 16_384, "{label} is unexpectedly large");
                ensure!(
                    !value.chars().any(char::is_control),
                    "{label} contains control characters"
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudProfile {
    pub schema: u32,
    pub provider: CloudProvider,
    pub device_id: String,
    pub automatic: bool,
    pub auth: CloudAuth,
}

impl CloudProfile {
    pub fn new(
        provider: CloudProvider,
        device_id: impl Into<String>,
        automatic: bool,
        auth: CloudAuth,
    ) -> Result<Self> {
        let profile = Self {
            schema: PROFILE_SCHEMA,
            provider,
            device_id: device_id.into(),
            automatic,
            auth,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == PROFILE_SCHEMA,
            "unsupported cloud-save profile schema"
        );
        validate_token("device ID", &self.device_id)?;
        self.auth.validate(self.provider)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceHead {
    pub schema: u32,
    pub scope: SyncScope,
    pub device_id: String,
    pub manifest_id: String,
    pub updated_unix_ms: i64,
}

impl DeviceHead {
    pub fn new(
        scope: SyncScope,
        device_id: impl Into<String>,
        manifest_id: impl Into<String>,
        updated_unix_ms: i64,
    ) -> Result<Self> {
        let head = Self {
            schema: HEAD_SCHEMA,
            scope,
            device_id: device_id.into(),
            manifest_id: manifest_id.into(),
            updated_unix_ms,
        };
        head.validate()?;
        Ok(head)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == HEAD_SCHEMA, "unsupported save head schema");
        self.scope.validate()?;
        validate_token("device ID", &self.device_id)?;
        validate_sha256(&self.manifest_id)?;
        ensure!(
            self.updated_unix_ms >= 0,
            "save head has a pre-epoch timestamp"
        );
        Ok(())
    }
}

/// A blocking storage facade suitable for the current synchronous settings
/// and launch pipeline. The owned runtime stays alive for the operator.
pub struct CloudStore {
    _runtime: Arc<tokio::runtime::Runtime>,
    operator: blocking::Operator,
    can_rename: bool,
}

impl CloudStore {
    pub fn connect(provider: CloudProvider, root: &str, auth: &CloudAuth) -> Result<Self> {
        validate_root(root)?;
        auth.validate(provider)?;
        opendal::install_default();

        let operator = match provider {
            CloudProvider::GoogleDrive => {
                let builder = configure_gdrive(Gdrive::default().root(root), auth);
                Operator::new(builder).context("configuring Google Drive save storage")?
            }
            CloudProvider::Dropbox => {
                let builder = configure_dropbox(Dropbox::default().root(root), auth);
                Operator::new(builder).context("configuring Dropbox save storage")?
            }
            CloudProvider::OneDrive => {
                let builder = configure_onedrive(Onedrive::default().root(root), auth);
                Operator::new(builder).context("configuring OneDrive save storage")?
            }
        };
        Self::from_operator(operator)
    }

    #[cfg(test)]
    pub(crate) fn memory() -> Result<Self> {
        Self::from_operator(Operator::new(Memory::default()).context("configuring memory storage")?)
    }

    fn from_operator(operator: Operator) -> Result<Self> {
        let operator = operator
            .layer(
                TimeoutLayer::default()
                    .with_timeout(Duration::from_secs(30))
                    .with_io_timeout(Duration::from_secs(60)),
            )
            .layer(RetryLayer::default().with_jitter());
        let capability = operator.info().capability();
        ensure!(
            capability.read
                && capability.write
                && capability.stat
                && capability.list
                && capability.delete,
            "cloud provider lacks required read/write/stat/list/delete capabilities"
        );
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("starting the cloud synchronization runtime")?,
        );
        let blocking = {
            let _guard = runtime.enter();
            blocking::Operator::new(operator).context("creating blocking cloud storage")?
        };
        Ok(Self {
            _runtime: runtime,
            operator: blocking,
            can_rename: capability.rename,
        })
    }

    /// Perform authenticated write/read/delete IO without touching save data.
    /// A successful builder construction alone does not prove the token or the
    /// provider account is usable.
    pub fn probe(&self) -> Result<()> {
        let path = format!("health/{}.probe", Uuid::new_v4().simple());
        let bytes = Uuid::new_v4().as_bytes().to_vec();
        let result = (|| {
            self.operator
                .write(&path, bytes.clone())
                .context("writing cloud-save connection probe")?;
            let readback = self
                .operator
                .read(&path)
                .context("reading cloud-save connection probe")?
                .to_vec();
            ensure!(readback == bytes, "cloud-save connection probe mismatch");
            Ok(())
        })();
        let cleanup = self
            .operator
            .delete(&path)
            .context("removing cloud-save connection probe");
        result.and(cleanup)
    }

    pub fn put_blob(&self, scope: &SyncScope, version: &FileVersion, bytes: &[u8]) -> Result<()> {
        scope.validate()?;
        ensure!(
            bytes.len() as u64 == version.size,
            "save blob size does not match manifest"
        );
        ensure!(
            hex::encode(Sha256::digest(bytes)) == version.sha256,
            "save blob hash does not match manifest"
        );
        let path = blob_path(scope, &version.sha256);
        self.put_immutable_verified(&path, bytes)
    }

    pub fn get_blob(&self, scope: &SyncScope, version: &FileVersion) -> Result<Vec<u8>> {
        scope.validate()?;
        let bytes = self
            .operator
            .read(&blob_path(scope, &version.sha256))
            .context("reading cloud save blob")?
            .to_vec();
        ensure!(
            bytes.len() as u64 == version.size,
            "cloud save blob size mismatch"
        );
        ensure!(
            hex::encode(Sha256::digest(&bytes)) == version.sha256,
            "cloud save blob hash mismatch"
        );
        Ok(bytes)
    }

    /// Stream a local artifact through a unique staging object, verify its
    /// expected content identity, and only then publish the immutable blob.
    /// This keeps whole-disk save images out of application memory.
    pub fn put_blob_file(
        &self,
        scope: &SyncScope,
        version: &FileVersion,
        local_path: &Path,
    ) -> Result<()> {
        scope.validate()?;
        let final_path = blob_path(scope, &version.sha256);
        if self
            .operator
            .exists(&final_path)
            .context("checking cloud save blob")?
        {
            return self.verify_remote_blob(scope, version);
        }

        let before = std::fs::metadata(local_path)
            .with_context(|| format!("reading local save metadata {}", local_path.display()))?;
        ensure!(
            before.is_file(),
            "local save artifact is not a regular file"
        );
        ensure!(
            before.len() == version.size,
            "local save size changed after planning"
        );
        let staging_path = format!(
            "{}/staging/{}",
            scope.remote_prefix(),
            Uuid::new_v4().simple()
        );
        let result = (|| {
            let mut source = File::open(local_path)
                .with_context(|| format!("opening local save {}", local_path.display()))?;
            let mut destination = self
                .operator
                .writer(&staging_path)
                .context("opening staged cloud save blob")?
                .into_std_write();
            let mut hasher = Sha256::new();
            let mut total = 0_u64;
            let mut buffer = vec![0_u8; 1024 * 1024];
            loop {
                let count = source
                    .read(&mut buffer)
                    .context("reading local save blob")?;
                if count == 0 {
                    break;
                }
                destination
                    .write_all(&buffer[..count])
                    .context("uploading staged cloud save blob")?;
                hasher.update(&buffer[..count]);
                total = total
                    .checked_add(count as u64)
                    .context("local save size overflow")?;
            }
            destination
                .close()
                .context("closing staged cloud save blob")?;
            let after = source
                .metadata()
                .context("rechecking local save metadata")?;
            ensure!(
                before.len() == after.len() && before.modified()? == after.modified()?,
                "local save changed during cloud upload"
            );
            ensure!(total == version.size, "uploaded save blob size mismatch");
            ensure!(
                hex::encode(hasher.finalize()) == version.sha256,
                "uploaded save blob hash mismatch"
            );
            self.verify_remote_path(&staging_path, version)?;

            // A concurrent writer can only target the same digest. Verify its
            // object rather than replacing it; otherwise atomically promote
            // this verified staging object.
            if self
                .operator
                .exists(&final_path)
                .context("rechecking cloud save blob")?
            {
                self.verify_remote_blob(scope, version)?;
            } else {
                if self.can_rename {
                    self.operator
                        .rename(&staging_path, &final_path)
                        .context("publishing immutable cloud save blob")?;
                } else {
                    self.copy_remote_path(&staging_path, &final_path)?;
                }
                self.verify_remote_blob(scope, version)?;
            }
            Ok(())
        })();
        let _ = self.operator.delete(&staging_path);
        result
    }

    /// Stream and verify a cloud artifact into a caller-owned destination.
    /// Callers should target a temporary file and publish it locally only
    /// after this returns successfully.
    pub fn copy_blob_to(
        &self,
        scope: &SyncScope,
        version: &FileVersion,
        destination: &mut impl Write,
    ) -> Result<()> {
        scope.validate()?;
        let mut source = self
            .operator
            .reader(&blob_path(scope, &version.sha256))
            .context("opening cloud save blob")?
            .into_std_read(..)
            .context("streaming cloud save blob")?;
        let mut hasher = Sha256::new();
        let mut total = 0_u64;
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let count = source
                .read(&mut buffer)
                .context("reading cloud save blob")?;
            if count == 0 {
                break;
            }
            destination
                .write_all(&buffer[..count])
                .context("writing staged local save blob")?;
            hasher.update(&buffer[..count]);
            total = total
                .checked_add(count as u64)
                .context("cloud save size overflow")?;
        }
        ensure!(total == version.size, "cloud save blob size mismatch");
        ensure!(
            hex::encode(hasher.finalize()) == version.sha256,
            "cloud save blob hash mismatch"
        );
        Ok(())
    }

    pub fn put_manifest(&self, manifest: &SaveManifest) -> Result<()> {
        manifest.validate()?;
        let bytes = serde_json::to_vec(manifest).context("encoding save manifest")?;
        self.put_immutable_verified(&manifest_path(&manifest.scope, &manifest.id), &bytes)
    }

    pub fn get_manifest(&self, scope: &SyncScope, id: &str) -> Result<SaveManifest> {
        scope.validate()?;
        validate_sha256(id)?;
        let bytes = self
            .operator
            .read(&manifest_path(scope, id))
            .context("reading cloud save manifest")?
            .to_vec();
        ensure!(
            bytes.len() <= 4 * 1024 * 1024,
            "cloud save manifest is too large"
        );
        let manifest: SaveManifest =
            serde_json::from_slice(&bytes).context("parsing cloud save manifest")?;
        manifest.validate()?;
        ensure!(
            &manifest.scope == scope,
            "cloud save manifest scope mismatch"
        );
        ensure!(manifest.id == id, "cloud save manifest path/ID mismatch");
        Ok(manifest)
    }

    pub fn set_device_head(&self, head: &DeviceHead) -> Result<()> {
        head.validate()?;
        // A newly connected device may adopt an already-published manifest
        // verbatim when its local inventory is identical. The immutable
        // manifest still has to exist and pass full identity validation.
        self.get_manifest(&head.scope, &head.manifest_id)?;
        let bytes = serde_json::to_vec(head).context("encoding save device head")?;
        let path = head_path(&head.scope, &head.device_id);
        self.operator
            .write(&path, bytes.clone())
            .context("writing save device head")?;
        let readback = self
            .operator
            .read(&path)
            .context("verifying save device head")?
            .to_vec();
        ensure!(
            readback == bytes,
            "save device head write verification failed"
        );
        Ok(())
    }

    pub fn device_heads(&self, scope: &SyncScope) -> Result<Vec<DeviceHead>> {
        scope.validate()?;
        let prefix = format!("{}/devices/", scope.remote_prefix());
        let entries = self
            .operator
            .list(&prefix)
            .context("listing cloud save heads")?;
        ensure!(
            entries.len() <= 256,
            "cloud save scope has too many device heads"
        );
        let mut heads = Vec::new();
        let mut devices = BTreeSet::new();
        for entry in entries {
            let path = entry.path();
            if !path.starts_with(&prefix) || !path.ends_with(".json") {
                continue;
            }
            let bytes = self
                .operator
                .read(path)
                .context("reading cloud save head")?
                .to_vec();
            ensure!(bytes.len() <= 64 * 1024, "cloud save head is too large");
            let head: DeviceHead =
                serde_json::from_slice(&bytes).context("parsing cloud save head")?;
            head.validate()?;
            ensure!(&head.scope == scope, "cloud save head scope mismatch");
            ensure!(
                path == head_path(scope, &head.device_id),
                "cloud save head path/device mismatch"
            );
            ensure!(
                devices.insert(head.device_id.clone()),
                "cloud save scope contains duplicate device heads"
            );
            heads.push(head);
        }
        heads.sort_by(|left, right| left.device_id.cmp(&right.device_id));
        Ok(heads)
    }

    /// Find the closest common ancestor by total parent-edge distance. This is
    /// used as the three-way base when two device heads diverge.
    pub fn nearest_common_ancestor(
        &self,
        scope: &SyncScope,
        left: &str,
        right: &str,
    ) -> Result<Option<SaveManifest>> {
        validate_sha256(left)?;
        validate_sha256(right)?;
        let left_ancestors = self.ancestor_distances(scope, left)?;
        let right_ancestors = self.ancestor_distances(scope, right)?;
        let id = left_ancestors
            .iter()
            .filter_map(|(id, left_distance)| {
                right_ancestors
                    .get(id)
                    .map(|right_distance| (id, left_distance + right_distance))
            })
            .min_by(|(left_id, left_distance), (right_id, right_distance)| {
                left_distance
                    .cmp(right_distance)
                    .then(left_id.cmp(right_id))
            })
            .map(|(id, _)| id.clone());
        id.map(|id| self.get_manifest(scope, &id)).transpose()
    }

    fn ancestor_distances(
        &self,
        scope: &SyncScope,
        start: &str,
    ) -> Result<BTreeMap<String, usize>> {
        let mut distances = BTreeMap::new();
        let mut queue = VecDeque::from([(start.to_owned(), 0_usize)]);
        while let Some((id, distance)) = queue.pop_front() {
            if distances.contains_key(&id) {
                continue;
            }
            ensure!(
                distances.len() < MAX_GRAPH_MANIFESTS,
                "save manifest ancestry exceeds {MAX_GRAPH_MANIFESTS} nodes"
            );
            let manifest = self.get_manifest(scope, &id)?;
            distances.insert(id, distance);
            for parent in manifest.parents {
                queue.push_back((parent, distance + 1));
            }
        }
        Ok(distances)
    }

    fn put_immutable_verified(&self, path: &str, bytes: &[u8]) -> Result<()> {
        match self.operator.read(path) {
            Ok(existing) => {
                ensure!(
                    existing.to_vec() == bytes,
                    "immutable cloud object already exists with different content"
                );
                return Ok(());
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("checking immutable cloud object"),
        }
        self.operator
            .write(path, bytes.to_vec())
            .context("writing immutable cloud object")?;
        let readback = self
            .operator
            .read(path)
            .context("verifying immutable cloud object")?
            .to_vec();
        ensure!(
            readback == bytes,
            "immutable cloud write verification failed"
        );
        Ok(())
    }

    fn verify_remote_blob(&self, scope: &SyncScope, version: &FileVersion) -> Result<()> {
        self.verify_remote_path(&blob_path(scope, &version.sha256), version)
    }

    fn copy_remote_path(&self, source_path: &str, destination_path: &str) -> Result<()> {
        let mut source = self
            .operator
            .reader(source_path)
            .context("opening staged cloud object")?
            .into_std_read(..)
            .context("streaming staged cloud object")?;
        let mut destination = self
            .operator
            .writer(destination_path)
            .context("opening immutable cloud object")?
            .into_std_write();
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let count = source
                .read(&mut buffer)
                .context("reading staged cloud object")?;
            if count == 0 {
                break;
            }
            destination
                .write_all(&buffer[..count])
                .context("publishing immutable cloud object")?;
        }
        destination
            .close()
            .context("closing immutable cloud object")?;
        Ok(())
    }

    fn verify_remote_path(&self, path: &str, version: &FileVersion) -> Result<()> {
        let mut source = self
            .operator
            .reader(path)
            .context("opening cloud save blob for verification")?
            .into_std_read(..)
            .context("streaming cloud save blob for verification")?;
        let mut hasher = Sha256::new();
        let mut total = 0_u64;
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let count = source
                .read(&mut buffer)
                .context("verifying cloud save blob")?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
            total = total
                .checked_add(count as u64)
                .context("cloud save size overflow")?;
        }
        ensure!(total == version.size, "cloud save blob size mismatch");
        ensure!(
            hex::encode(hasher.finalize()) == version.sha256,
            "cloud save blob hash mismatch"
        );
        Ok(())
    }
}

fn configure_gdrive(mut builder: Gdrive, auth: &CloudAuth) -> Gdrive {
    if let Some(token) = &auth.access_token {
        builder = builder.access_token(token);
    }
    if let Some(token) = &auth.refresh_token {
        builder = builder.refresh_token(token);
    }
    if let Some(client_id) = &auth.client_id {
        builder = builder.client_id(client_id);
    }
    if let Some(secret) = &auth.client_secret {
        builder = builder.client_secret(secret);
    }
    builder
}

fn configure_dropbox(mut builder: Dropbox, auth: &CloudAuth) -> Dropbox {
    if let Some(token) = &auth.access_token {
        builder = builder.access_token(token);
    }
    if let Some(token) = &auth.refresh_token {
        builder = builder.refresh_token(token);
    }
    if let Some(client_id) = &auth.client_id {
        builder = builder.client_id(client_id);
    }
    if let Some(secret) = &auth.client_secret {
        builder = builder.client_secret(secret);
    }
    builder
}

fn configure_onedrive(mut builder: Onedrive, auth: &CloudAuth) -> Onedrive {
    if let Some(token) = &auth.access_token {
        builder = builder.access_token(token);
    }
    if let Some(token) = &auth.refresh_token {
        builder = builder.refresh_token(token);
    }
    if let Some(client_id) = &auth.client_id {
        builder = builder.client_id(client_id);
    }
    if let Some(secret) = &auth.client_secret {
        builder = builder.client_secret(secret);
    }
    builder
}

fn blob_path(scope: &SyncScope, digest: &str) -> String {
    format!("{}/blobs/{digest}", scope.remote_prefix())
}

fn manifest_path(scope: &SyncScope, id: &str) -> String {
    format!("{}/manifests/{id}.json", scope.remote_prefix())
}

fn head_path(scope: &SyncScope, device_id: &str) -> String {
    format!("{}/devices/{device_id}.json", scope.remote_prefix())
}

fn nonempty(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|value| !value.trim().is_empty())
}

fn validate_root(root: &str) -> Result<()> {
    ensure!(root.len() <= 512, "cloud save root is too long");
    ensure!(
        !root.chars().any(char::is_control),
        "cloud save root contains control characters"
    );
    ensure!(
        root.split(['/', '\\']).all(|component| component != ".."),
        "cloud save root may not contain traversal"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_sync::{ArtifactKey, SavePurpose, SaveRoute};

    fn scope() -> SyncScope {
        SyncScope::new("duckstation", "linux").unwrap()
    }

    fn version(bytes: &[u8], modified: i64) -> FileVersion {
        FileVersion::from_bytes(bytes, modified)
    }

    fn manifest(parents: Vec<String>, device: &str, created: i64, bytes: &[u8]) -> SaveManifest {
        let file = version(bytes, created);
        SaveManifest::new(
            scope(),
            parents,
            device,
            created,
            BTreeMap::from([(
                ArtifactKey::new(
                    SaveRoute {
                        purpose: SavePurpose::Saves,
                        root_index: 0,
                    },
                    "game.sav",
                )
                .unwrap(),
                file,
            )]),
        )
        .unwrap()
    }

    #[test]
    fn auth_requires_one_token_and_provider_refresh_fields() {
        let neither = CloudAuth::default();
        assert!(neither.validate(CloudProvider::OneDrive).is_err());
        let both = CloudAuth {
            access_token: Some("access".into()),
            refresh_token: Some("refresh".into()),
            client_id: Some("client".into()),
            client_secret: Some("secret".into()),
        };
        assert!(both.validate(CloudProvider::Dropbox).is_err());
        let native_onedrive = CloudAuth {
            refresh_token: Some("refresh".into()),
            client_id: Some("client".into()),
            ..CloudAuth::default()
        };
        assert!(native_onedrive.validate(CloudProvider::OneDrive).is_ok());
        assert!(
            native_onedrive
                .validate(CloudProvider::GoogleDrive)
                .is_err()
        );
    }

    #[test]
    fn all_configured_provider_builders_initialize_without_network_io() {
        let auth = CloudAuth {
            access_token: Some("test-access-token".into()),
            ..CloudAuth::default()
        };
        for provider in [
            CloudProvider::GoogleDrive,
            CloudProvider::Dropbox,
            CloudProvider::OneDrive,
        ] {
            CloudStore::connect(provider, DEFAULT_CLOUD_ROOT, &auth).unwrap();
        }
    }

    #[test]
    fn profile_round_trip_preserves_settings_and_redacts_secrets_from_debug() {
        let profile = CloudProfile::new(
            CloudProvider::Dropbox,
            "desktop-a",
            true,
            CloudAuth {
                refresh_token: Some("refresh-secret".into()),
                client_id: Some("client-id".into()),
                client_secret: Some("client-secret".into()),
                ..CloudAuth::default()
            },
        )
        .unwrap();
        let encoded = serde_json::to_string(&profile).unwrap();
        let decoded: CloudProfile = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, profile);
        let debug = format!("{profile:?}");
        assert!(!debug.contains("refresh-secret"));
        assert!(!debug.contains("client-secret"));
        assert!(debug.contains("[redacted]"));
    }

    #[test]
    fn memory_store_round_trips_verified_blobs_manifests_and_heads() {
        let store = CloudStore::memory().unwrap();
        let bytes = b"save bytes";
        let file = version(bytes, 100);
        store.put_blob(&scope(), &file, bytes).unwrap();
        store.put_blob(&scope(), &file, bytes).unwrap();
        assert_eq!(store.get_blob(&scope(), &file).unwrap(), bytes);

        let manifest = manifest(Vec::new(), "device-a", 100, bytes);
        store.put_manifest(&manifest).unwrap();
        assert_eq!(
            store.get_manifest(&scope(), &manifest.id).unwrap(),
            manifest
        );
        let head = DeviceHead::new(scope(), "device-a", manifest.id.clone(), 101).unwrap();
        store.set_device_head(&head).unwrap();
        assert_eq!(store.device_heads(&scope()).unwrap(), vec![head]);
    }

    #[test]
    fn memory_store_streams_large_artifacts_through_staging() {
        let store = CloudStore::memory().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("disk.img");
        let bytes = vec![0x5a; 3 * 1024 * 1024 + 17];
        std::fs::write(&path, &bytes).unwrap();
        let file = version(&bytes, 100);
        store.put_blob_file(&scope(), &file, &path).unwrap();
        let mut downloaded = Vec::new();
        store
            .copy_blob_to(&scope(), &file, &mut downloaded)
            .unwrap();
        assert_eq!(downloaded, bytes);
    }

    #[test]
    fn immutable_objects_reject_different_content() {
        let store = CloudStore::memory().unwrap();
        store.put_immutable_verified("immutable", b"one").unwrap();
        assert!(store.put_immutable_verified("immutable", b"two").is_err());
    }

    #[test]
    fn ancestry_finds_the_nearest_shared_manifest() {
        let store = CloudStore::memory().unwrap();
        let base = manifest(Vec::new(), "device-a", 1, b"base");
        let left = manifest(vec![base.id.clone()], "device-a", 2, b"left");
        let right = manifest(vec![base.id.clone()], "device-b", 3, b"right");
        for manifest in [&base, &left, &right] {
            store.put_manifest(manifest).unwrap();
        }
        assert_eq!(
            store
                .nearest_common_ancestor(&scope(), &left.id, &right.id)
                .unwrap()
                .unwrap()
                .id,
            base.id
        );
    }

    #[test]
    fn head_cannot_reference_an_unpublished_manifest() {
        let store = CloudStore::memory().unwrap();
        let head = DeviceHead::new(scope(), "device-a", "a".repeat(64), 1).unwrap();
        assert!(store.set_device_head(&head).is_err());
    }
}
