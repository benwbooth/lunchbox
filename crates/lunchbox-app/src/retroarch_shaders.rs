use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_UNPACKED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_FILES: usize = 20_000;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const RECEIPT_FILE: &str = ".lunchbox-pack.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShaderPack {
    id: &'static str,
    label: &'static str,
    preset_extension: &'static str,
    url: &'static str,
}

const SHADER_PACKS: [ShaderPack; 2] = [
    ShaderPack {
        id: "shaders_slang",
        label: "Slang",
        preset_extension: "slangp",
        url: "https://buildbot.libretro.com/assets/frontend/shaders_slang.zip",
    },
    ShaderPack {
        id: "shaders_glsl",
        label: "GLSL",
        preset_extension: "glslp",
        url: "https://buildbot.libretro.com/assets/frontend/shaders_glsl.zip",
    },
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShaderProgress {
    pub percent: i32,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderTargetStatus {
    pub path: PathBuf,
    pub slang_state: String,
    pub glsl_state: String,
    pub requires_confirmation: bool,
}

impl ShaderTargetStatus {
    pub fn detail(&self) -> String {
        format!("Slang: {} · GLSL: {}", self.slang_state, self.glsl_state)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShaderInventory {
    pub targets: Vec<ShaderTargetStatus>,
}

impl ShaderInventory {
    pub fn requires_confirmation(&self) -> bool {
        self.targets
            .iter()
            .any(|target| target.requires_confirmation)
    }

    pub fn installed_target_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| target.slang_state == "Managed" && target.glsl_state == "Managed")
            .count()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShaderInstallSummary {
    pub target_count: usize,
    pub file_count: usize,
    pub unpacked_bytes: u64,
    pub reused_archives: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PackReceipt {
    schema_version: u32,
    pack_id: String,
    source_url: String,
    archive_sha256: String,
    archive_bytes: u64,
    file_count: usize,
    unpacked_bytes: u64,
    installed_at: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ArchiveReceipt {
    schema_version: u32,
    pack_id: String,
    source_url: String,
    etag: String,
    last_modified: String,
    sha256: String,
    bytes: u64,
}

#[derive(Clone, Debug)]
struct RemoteMetadata {
    etag: String,
    last_modified: String,
    bytes: Option<u64>,
}

#[derive(Clone, Debug)]
struct ResolvedArchive {
    path: PathBuf,
    sha256: String,
    bytes: u64,
    reused: bool,
}

#[derive(Debug)]
struct PendingActivation {
    stage: PathBuf,
    target: PathBuf,
    backup: PathBuf,
    had_previous: bool,
    activated: bool,
}

pub fn inventory(target_override: Option<PathBuf>) -> Result<ShaderInventory> {
    let targets = match target_override {
        Some(path) => vec![path],
        None => discover_targets()?,
    };
    Ok(ShaderInventory {
        targets: targets
            .into_iter()
            .map(|path| inspect_target(path))
            .collect(),
    })
}

pub fn install(
    target_override: Option<PathBuf>,
    archive_directory: Option<PathBuf>,
    replace_unmanaged: bool,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(ShaderProgress) + Send + Sync>,
) -> Result<ShaderInstallSummary> {
    check_cancelled(cancelled)?;
    let inventory = inventory(target_override)?;
    if inventory.targets.is_empty() {
        bail!("could not determine a RetroArch shader directory on this computer");
    }
    if inventory.requires_confirmation() && !replace_unmanaged {
        bail!(
            "existing RetroArch shader packs were not installed by Lunchbox; confirm replacement before continuing"
        );
    }
    let cache = shader_cache_directory()?;
    fs::create_dir_all(&cache)?;
    let mut archives = Vec::with_capacity(SHADER_PACKS.len());
    for (index, pack) in SHADER_PACKS.iter().enumerate() {
        check_cancelled(cancelled)?;
        progress(ShaderProgress {
            percent: i32::try_from(index * 25).unwrap_or(0),
            message: format!("Preparing the {} shader pack…", pack.label),
        });
        let archive = if let Some(directory) = archive_directory.as_deref() {
            resolve_local_archive(pack, directory)?
        } else {
            resolve_official_archive(pack, &cache, cancelled, Arc::clone(&progress))?
        };
        archives.push((*pack, archive));
    }

    let operation = Uuid::new_v4().simple().to_string();
    let mut pending = Vec::new();
    let mut total_files = 0usize;
    let mut total_unpacked = 0u64;
    let stage_result = (|| -> Result<()> {
        for (target_index, target) in inventory.targets.iter().enumerate() {
            fs::create_dir_all(&target.path).with_context(|| {
                format!(
                    "creating RetroArch shader directory {}",
                    target.path.display()
                )
            })?;
            for (pack_index, (pack, archive)) in archives.iter().enumerate() {
                check_cancelled(cancelled)?;
                let stage = target
                    .path
                    .join(format!(".{}.lunchbox-stage-{operation}", pack.id));
                let target_path = target.path.join(pack.id);
                let backup = target
                    .path
                    .join(format!(".{}.lunchbox-backup-{operation}", pack.id));
                remove_path_if_present(&stage)?;
                remove_path_if_present(&backup)?;
                fs::create_dir(&stage).with_context(|| {
                    format!("creating shader staging directory {}", stage.display())
                })?;
                let overall_index = target_index * SHADER_PACKS.len() + pack_index;
                let overall_count = inventory.targets.len() * SHADER_PACKS.len();
                progress(ShaderProgress {
                    percent: 50
                        + i32::try_from(overall_index * 40 / overall_count.max(1)).unwrap_or(0),
                    message: format!(
                        "Validating and staging {} shaders for {}…",
                        pack.label,
                        target.path.display()
                    ),
                });
                let (file_count, unpacked_bytes) =
                    extract_pack(pack, &archive.path, &stage, cancelled)?;
                let receipt = PackReceipt {
                    schema_version: 1,
                    pack_id: pack.id.to_owned(),
                    source_url: pack.url.to_owned(),
                    archive_sha256: archive.sha256.clone(),
                    archive_bytes: archive.bytes,
                    file_count,
                    unpacked_bytes,
                    installed_at: unix_timestamp(),
                };
                write_json_atomic(&stage.join(RECEIPT_FILE), &receipt)?;
                total_files = total_files.saturating_add(file_count);
                total_unpacked = total_unpacked.saturating_add(unpacked_bytes);
                pending.push(PendingActivation {
                    stage,
                    target: target_path,
                    backup,
                    had_previous: false,
                    activated: false,
                });
            }
        }
        Ok(())
    })();
    if let Err(error) = stage_result {
        cleanup_pending(&pending);
        return Err(error);
    }
    check_cancelled(cancelled).inspect_err(|_| cleanup_pending(&pending))?;
    progress(ShaderProgress {
        percent: 94,
        message: "Publishing both shader packs atomically…".to_owned(),
    });
    activate_all(&mut pending)?;
    progress(ShaderProgress {
        percent: 100,
        message: "RetroArch shader packs are ready.".to_owned(),
    });
    Ok(ShaderInstallSummary {
        target_count: inventory.targets.len(),
        file_count: total_files,
        unpacked_bytes: total_unpacked,
        reused_archives: archives
            .iter()
            .filter(|(_, archive)| archive.reused)
            .count(),
    })
}

pub fn open_target(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("creating RetroArch shader directory {}", path.display()))?;
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    Command::new(program)
        .arg(path)
        .spawn()
        .with_context(|| format!("opening RetroArch shader directory {}", path.display()))?;
    Ok(())
}

fn discover_targets() -> Result<Vec<PathBuf>> {
    let base = BaseDirs::new().context("could not determine the user configuration directory")?;
    let mut candidates = Vec::new();
    if cfg!(target_os = "linux") {
        let native = base.config_dir().join("retroarch");
        let flatpak = base
            .home_dir()
            .join(".var/app/org.libretro.RetroArch/config/retroarch");
        for configuration in [&native, &flatpak] {
            if configuration.exists() {
                candidates.push(configuration.join("shaders"));
            }
        }
        if candidates.is_empty() {
            candidates.push(native.join("shaders"));
        }
    } else if cfg!(target_os = "windows") {
        candidates.push(base.data_local_dir().join("RetroArch/shaders"));
    } else if cfg!(target_os = "macos") {
        candidates.push(
            base.home_dir()
                .join("Library/Application Support/RetroArch/shaders"),
        );
    } else {
        bail!("RetroArch shader management is not supported on this host platform");
    }
    let mut seen = HashSet::new();
    candidates.retain(|path| seen.insert(path.clone()));
    Ok(candidates)
}

fn inspect_target(path: PathBuf) -> ShaderTargetStatus {
    let (slang_state, slang_external) = inspect_pack(&path, &SHADER_PACKS[0]);
    let (glsl_state, glsl_external) = inspect_pack(&path, &SHADER_PACKS[1]);
    ShaderTargetStatus {
        path,
        slang_state,
        glsl_state,
        requires_confirmation: slang_external || glsl_external,
    }
}

fn inspect_pack(root: &Path, pack: &ShaderPack) -> (String, bool) {
    let directory = root.join(pack.id);
    if !directory.exists() {
        return ("Not installed".to_owned(), false);
    }
    let receipt_path = directory.join(RECEIPT_FILE);
    let managed = fs::read(&receipt_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<PackReceipt>(&bytes).ok())
        .is_some_and(|receipt| {
            receipt.schema_version == 1
                && receipt.pack_id == pack.id
                && receipt.source_url == pack.url
                && receipt.file_count > 0
                && receipt.unpacked_bytes > 0
        });
    if managed {
        ("Managed".to_owned(), false)
    } else {
        ("Existing (external)".to_owned(), true)
    }
}

fn resolve_local_archive(pack: &ShaderPack, directory: &Path) -> Result<ResolvedArchive> {
    let path = directory.join(format!("{}.zip", pack.id));
    let metadata = fs::metadata(&path)
        .with_context(|| format!("reading local {} archive {}", pack.label, path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_ARCHIVE_BYTES {
        bail!("local {} archive has an invalid size", pack.label);
    }
    Ok(ResolvedArchive {
        sha256: sha256_file(&path)?,
        path,
        bytes: metadata.len(),
        reused: true,
    })
}

fn resolve_official_archive(
    pack: &ShaderPack,
    cache: &Path,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(ShaderProgress) + Send + Sync>,
) -> Result<ResolvedArchive> {
    let archive_path = cache.join(format!("{}.zip", pack.id));
    let receipt_path = cache.join(format!("{}.json", pack.id));
    let cached_receipt = fs::read(&receipt_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ArchiveReceipt>(&bytes).ok());
    let remote = remote_metadata(pack).ok();
    if let Some(receipt) = cached_receipt.as_ref()
        && cached_archive_matches(pack, &archive_path, receipt, remote.as_ref())?
    {
        progress(ShaderProgress {
            percent: 0,
            message: format!("Reusing the verified cached {} shader archive.", pack.label),
        });
        return Ok(ResolvedArchive {
            path: archive_path,
            sha256: receipt.sha256.clone(),
            bytes: receipt.bytes,
            reused: true,
        });
    }
    let remote = remote.context(format!(
        "could not check the current {} archive and no matching verified cache is available",
        pack.label
    ))?;
    if remote.bytes.is_some_and(|bytes| bytes > MAX_ARCHIVE_BYTES) {
        bail!(
            "{} shader archive exceeds the download safety limit",
            pack.label
        );
    }
    let temporary = cache.join(format!(
        ".{}.download-{}.zip",
        pack.id,
        Uuid::new_v4().simple()
    ));
    let downloaded = download_archive(pack, &temporary, cancelled, progress);
    let (bytes, sha256) = match downloaded {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
    };
    replace_file(&temporary, &archive_path)?;
    let receipt = ArchiveReceipt {
        schema_version: 1,
        pack_id: pack.id.to_owned(),
        source_url: pack.url.to_owned(),
        etag: remote.etag,
        last_modified: remote.last_modified,
        sha256: sha256.clone(),
        bytes,
    };
    write_json_atomic(&receipt_path, &receipt)?;
    Ok(ResolvedArchive {
        path: archive_path,
        sha256,
        bytes,
        reused: false,
    })
}

fn remote_metadata(pack: &ShaderPack) -> Result<RemoteMetadata> {
    let agent = http_agent()?;
    let response = agent
        .head(pack.url)
        .call()
        .with_context(|| format!("checking the current {} shader archive", pack.label))?;
    if !response.status().is_success() {
        bail!(
            "{} shader archive check returned HTTP {}",
            pack.label,
            response.status()
        );
    }
    let headers = response.headers();
    let text = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    };
    Ok(RemoteMetadata {
        etag: text("etag"),
        last_modified: text("last-modified"),
        bytes: headers
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok()),
    })
}

fn cached_archive_matches(
    pack: &ShaderPack,
    path: &Path,
    receipt: &ArchiveReceipt,
    remote: Option<&RemoteMetadata>,
) -> Result<bool> {
    if receipt.schema_version != 1
        || receipt.pack_id != pack.id
        || receipt.source_url != pack.url
        || receipt.bytes == 0
        || receipt.bytes > MAX_ARCHIVE_BYTES
    {
        return Ok(false);
    }
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(false);
    };
    if !metadata.is_file() || metadata.len() != receipt.bytes {
        return Ok(false);
    }
    if let Some(remote) = remote {
        let etag_matches = !remote.etag.is_empty() && remote.etag == receipt.etag;
        let modified_matches = !remote.last_modified.is_empty()
            && remote.last_modified == receipt.last_modified
            && remote.bytes == Some(receipt.bytes);
        if !etag_matches && !modified_matches {
            return Ok(false);
        }
    }
    Ok(sha256_file(path)? == receipt.sha256)
}

fn download_archive(
    pack: &ShaderPack,
    path: &Path,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(ShaderProgress) + Send + Sync>,
) -> Result<(u64, String)> {
    let agent = http_agent()?;
    let mut response = agent
        .get(pack.url)
        .call()
        .with_context(|| format!("downloading the {} shader archive", pack.label))?;
    if !response.status().is_success() {
        bail!(
            "{} shader download returned HTTP {}",
            pack.label,
            response.status()
        );
    }
    let expected = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if expected.is_some_and(|bytes| bytes > MAX_ARCHIVE_BYTES) {
        bail!(
            "{} shader archive exceeds the download safety limit",
            pack.label
        );
    }
    let mut reader = response.body_mut().as_reader();
    let mut output = File::create(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    let mut downloaded = 0u64;
    let mut last_progress = Instant::now() - Duration::from_secs(1);
    loop {
        check_cancelled(cancelled)?;
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        downloaded = downloaded
            .checked_add(u64::try_from(count).unwrap_or(u64::MAX))
            .context("shader download size overflow")?;
        if downloaded > MAX_ARCHIVE_BYTES {
            bail!(
                "{} shader archive exceeds the download safety limit",
                pack.label
            );
        }
        output.write_all(&buffer[..count])?;
        digest.update(&buffer[..count]);
        if last_progress.elapsed() >= Duration::from_millis(250) {
            let percent = expected
                .filter(|bytes| *bytes > 0)
                .map(|bytes| ((downloaded.saturating_mul(100) / bytes).min(100)) as i32)
                .unwrap_or(0);
            progress(ShaderProgress {
                percent: percent / 4,
                message: format!(
                    "Downloading {} shaders · {}{}",
                    pack.label,
                    format_bytes(downloaded),
                    expected
                        .map(|bytes| format!(" / {}", format_bytes(bytes)))
                        .unwrap_or_default()
                ),
            });
            last_progress = Instant::now();
        }
    }
    output.flush()?;
    output.sync_all()?;
    if expected.is_some_and(|bytes| bytes != downloaded) {
        bail!(
            "{} shader download ended at {} bytes; expected {}",
            pack.label,
            downloaded,
            expected.unwrap_or_default()
        );
    }
    Ok((downloaded, hex::encode(digest.finalize())))
}

fn http_agent() -> Result<ureq::Agent> {
    Ok(ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .http_status_as_error(false)
        .user_agent("Lunchbox/0.1 RetroArch shader manager")
        .build()
        .into())
}

fn extract_pack(
    pack: &ShaderPack,
    archive_path: &Path,
    output: &Path,
    cancelled: &AtomicBool,
) -> Result<(usize, u64)> {
    let file = File::open(archive_path)
        .with_context(|| format!("opening {} shader archive", pack.label))?;
    let mut archive =
        zip::ZipArchive::new(file).with_context(|| format!("reading {} shader ZIP", pack.label))?;
    if archive.len() == 0 || archive.len() > MAX_ARCHIVE_FILES {
        bail!("{} shader ZIP has an invalid file count", pack.label);
    }
    let mut paths = HashSet::new();
    let mut file_count = 0usize;
    let mut unpacked_bytes = 0u64;
    let mut has_preset = false;
    for index in 0..archive.len() {
        check_cancelled(cancelled)?;
        let mut entry = archive.by_index(index)?;
        let path = entry
            .enclosed_name()
            .context("shader ZIP contains an unsafe path")?;
        validate_archive_path(&path)?;
        if !paths.insert(path.clone()) {
            bail!("shader ZIP contains a duplicate path: {}", path.display());
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("shader ZIP contains a symbolic link: {}", path.display());
        }
        if entry.is_dir() {
            fs::create_dir_all(output.join(&path))?;
            continue;
        }
        unpacked_bytes = unpacked_bytes
            .checked_add(entry.size())
            .context("shader ZIP unpacked size overflow")?;
        if unpacked_bytes > MAX_UNPACKED_BYTES {
            bail!("{} shader ZIP exceeds the unpacked size limit", pack.label);
        }
        if path.extension().and_then(|value| value.to_str()) == Some(pack.preset_extension) {
            has_preset = true;
        }
        let target = output.join(&path);
        let parent = target
            .parent()
            .context("shader archive path has no parent")?;
        fs::create_dir_all(parent)?;
        let mut destination = File::create(&target)?;
        let copied = copy_cancelled(&mut entry, &mut destination, cancelled)?;
        if copied != entry.size() {
            bail!("shader ZIP member changed size while extracting");
        }
        destination.flush()?;
        file_count = file_count.saturating_add(1);
    }
    if !has_preset || file_count == 0 {
        bail!(
            "{} shader ZIP contains no .{} presets",
            pack.label,
            pack.preset_extension
        );
    }
    Ok((file_count, unpacked_bytes))
}

fn validate_archive_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        bail!("shader ZIP contains an invalid path");
    }
    for component in path.components() {
        match component {
            Component::Normal(value) if !value.is_empty() => {}
            _ => bail!("shader ZIP contains an unsafe path: {}", path.display()),
        }
    }
    Ok(())
}

fn copy_cancelled(
    input: &mut impl Read,
    output: &mut impl Write,
    cancelled: &AtomicBool,
) -> Result<u64> {
    let mut buffer = [0u8; 128 * 1024];
    let mut written = 0u64;
    loop {
        check_cancelled(cancelled)?;
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        output.write_all(&buffer[..count])?;
        written = written.saturating_add(count as u64);
    }
    Ok(written)
}

fn activate_all(pending: &mut [PendingActivation]) -> Result<()> {
    for index in 0..pending.len() {
        pending[index].had_previous = pending[index].target.exists();
        if pending[index].had_previous {
            if let Err(error) = fs::rename(&pending[index].target, &pending[index].backup) {
                let target = pending[index].target.display().to_string();
                rollback_activations(&mut pending[..index]);
                cleanup_stages(pending);
                return Err(error)
                    .with_context(|| format!("preserving existing shader pack {target}"));
            }
        }
        if let Err(error) = fs::rename(&pending[index].stage, &pending[index].target) {
            let mut restore_error = None;
            if pending[index].had_previous {
                restore_error = fs::rename(&pending[index].backup, &pending[index].target).err();
            }
            let target = pending[index].target.display().to_string();
            let backup = pending[index].backup.display().to_string();
            rollback_activations(&mut pending[..index]);
            cleanup_stages(pending);
            if let Some(restore_error) = restore_error {
                return Err(error).with_context(|| {
                    format!(
                        "publishing shader pack {target}; restoring its previous contents also failed ({restore_error}); the preserved contents remain at {backup}"
                    )
                });
            }
            return Err(error).with_context(|| format!("publishing shader pack {target}"));
        }
        pending[index].activated = true;
    }
    for activation in pending.iter() {
        if activation.had_previous {
            // Activation has committed at this point. A stale backup is preferable to
            // reporting a failed install after every new pack is already live.
            let _ = remove_path_if_present(&activation.backup);
        }
    }
    Ok(())
}

fn rollback_activations(pending: &mut [PendingActivation]) {
    for activation in pending.iter_mut().rev() {
        if activation.activated {
            let _ = remove_path_if_present(&activation.target);
            if activation.had_previous {
                let _ = fs::rename(&activation.backup, &activation.target);
            }
            activation.activated = false;
        }
        let _ = remove_path_if_present(&activation.stage);
    }
}

fn cleanup_pending(pending: &[PendingActivation]) {
    for activation in pending {
        let _ = remove_path_if_present(&activation.stage);
        let _ = remove_path_if_present(&activation.backup);
    }
}

fn cleanup_stages(pending: &[PendingActivation]) {
    for activation in pending {
        let _ = remove_path_if_present(&activation.stage);
    }
}

fn shader_cache_directory() -> Result<PathBuf> {
    ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.cache_dir().join("retroarch-shaders"))
        .context("could not determine the Lunchbox cache directory")
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("JSON destination has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".manifest-{}.json", Uuid::new_v4().simple()));
    let result = (|| -> Result<()> {
        let mut output = File::create(&temporary)?;
        serde_json::to_writer_pretty(&mut output, value)?;
        output.write_all(b"\n")?;
        output.flush()?;
        output.sync_all()?;
        replace_file(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn replace_file(source: &Path, target: &Path) -> Result<()> {
    let backup = target.with_extension(format!("backup-{}", Uuid::new_v4().simple()));
    let had_target = target.exists();
    if had_target {
        fs::rename(target, &backup)?;
    }
    if let Err(error) = fs::rename(source, target) {
        if had_target {
            let _ = fs::rename(&backup, target);
        }
        return Err(error).with_context(|| format!("publishing {}", target.display()));
    }
    if had_target {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn remove_path_if_present(path: &Path) -> Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut input = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        bail!("shader installation cancelled; existing packs were left unchanged");
    }
    Ok(())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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
    use tempfile::TempDir;

    fn write_pack(directory: &Path, pack: ShaderPack, extra: &[(&str, &[u8])]) {
        fs::create_dir_all(directory).unwrap();
        let path = directory.join(format!("{}.zip", pack.id));
        let file = File::create(path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                format!("presets/default.{}", pack.preset_extension),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"shader0 = path").unwrap();
        for (name, bytes) in extra {
            archive
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive.finish().unwrap();
    }

    fn fixture_archives(directory: &Path) {
        write_pack(
            directory,
            SHADER_PACKS[0],
            &[("presets/shaders/pass.slang", b"#version 450")],
        );
        write_pack(
            directory,
            SHADER_PACKS[1],
            &[("presets/shaders/pass.glsl", b"#version 330")],
        );
    }

    #[test]
    fn installs_both_packs_idempotently_and_preserves_custom_siblings() {
        let directory = TempDir::new().unwrap();
        let archives = directory.path().join("archives");
        let target = directory.path().join("shaders");
        fixture_archives(&archives);
        fs::create_dir_all(target.join("custom")).unwrap();
        fs::write(target.join("custom/preset.slangp"), b"mine").unwrap();
        let cancelled = AtomicBool::new(false);
        let progress: Arc<dyn Fn(ShaderProgress) + Send + Sync> = Arc::new(|_| {});

        let first = install(
            Some(target.clone()),
            Some(archives.clone()),
            false,
            &cancelled,
            Arc::clone(&progress),
        )
        .unwrap();
        assert_eq!(first.target_count, 1);
        assert_eq!(first.file_count, 4);
        assert_eq!(
            fs::read(target.join("custom/preset.slangp")).unwrap(),
            b"mine"
        );
        assert_eq!(
            inventory(Some(target.clone()))
                .unwrap()
                .installed_target_count(),
            1
        );

        fs::write(target.join("shaders_slang/stale.slangp"), b"stale").unwrap();
        install(
            Some(target.clone()),
            Some(archives),
            false,
            &cancelled,
            progress,
        )
        .unwrap();
        assert!(!target.join("shaders_slang/stale.slangp").exists());
        assert_eq!(
            fs::read(target.join("custom/preset.slangp")).unwrap(),
            b"mine"
        );
    }

    #[test]
    fn unmanaged_pack_requires_explicit_replacement() {
        let directory = TempDir::new().unwrap();
        let archives = directory.path().join("archives");
        let target = directory.path().join("shaders");
        fixture_archives(&archives);
        fs::create_dir_all(target.join("shaders_slang")).unwrap();
        fs::write(target.join("shaders_slang/user.slangp"), b"mine").unwrap();
        let cancelled = AtomicBool::new(false);
        let progress: Arc<dyn Fn(ShaderProgress) + Send + Sync> = Arc::new(|_| {});

        assert!(
            install(
                Some(target.clone()),
                Some(archives.clone()),
                false,
                &cancelled,
                Arc::clone(&progress),
            )
            .is_err()
        );
        assert_eq!(
            fs::read(target.join("shaders_slang/user.slangp")).unwrap(),
            b"mine"
        );
        install(
            Some(target.clone()),
            Some(archives),
            true,
            &cancelled,
            progress,
        )
        .unwrap();
        assert!(!target.join("shaders_slang/user.slangp").exists());
    }

    #[test]
    fn cancellation_and_unsafe_archives_never_publish_partial_packs() {
        let directory = TempDir::new().unwrap();
        let archives = directory.path().join("archives");
        let target = directory.path().join("shaders");
        fixture_archives(&archives);
        let cancelled = AtomicBool::new(true);
        assert!(
            install(
                Some(target.clone()),
                Some(archives.clone()),
                false,
                &cancelled,
                Arc::new(|_| {}),
            )
            .is_err()
        );
        assert!(!target.join("shaders_slang").exists());

        write_pack(
            &archives,
            SHADER_PACKS[0],
            &[("../escape.slang", b"unsafe")],
        );
        assert!(
            install(
                Some(target.clone()),
                Some(archives),
                false,
                &AtomicBool::new(false),
                Arc::new(|_| {}),
            )
            .is_err()
        );
        assert!(!target.join("shaders_slang").exists());
        assert!(!directory.path().join("escape.slang").exists());
    }

    #[test]
    fn activation_failure_rolls_back_earlier_packs() {
        let directory = TempDir::new().unwrap();
        let first_target = directory.path().join("first-target");
        let first_stage = directory.path().join("first-stage");
        let first_backup = directory.path().join("first-backup");
        let second_target = directory.path().join("second-target");
        let second_stage = directory.path().join("second-stage");
        let second_backup = directory.path().join("missing-parent/second-backup");
        fs::create_dir_all(&first_target).unwrap();
        fs::write(first_target.join("old"), b"old first").unwrap();
        fs::create_dir_all(&first_stage).unwrap();
        fs::write(first_stage.join("new"), b"new first").unwrap();
        fs::create_dir_all(&second_target).unwrap();
        fs::write(second_target.join("old"), b"old second").unwrap();
        fs::create_dir_all(&second_stage).unwrap();
        fs::write(second_stage.join("new"), b"new second").unwrap();
        let mut pending = vec![
            PendingActivation {
                stage: first_stage.clone(),
                target: first_target.clone(),
                backup: first_backup.clone(),
                had_previous: false,
                activated: false,
            },
            PendingActivation {
                stage: second_stage.clone(),
                target: second_target.clone(),
                backup: second_backup,
                had_previous: false,
                activated: false,
            },
        ];

        assert!(activate_all(&mut pending).is_err());
        assert_eq!(fs::read(first_target.join("old")).unwrap(), b"old first");
        assert_eq!(fs::read(second_target.join("old")).unwrap(), b"old second");
        assert!(!first_target.join("new").exists());
        assert!(!first_stage.exists());
        assert!(!first_backup.exists());
        assert!(!second_stage.exists());
    }
}
