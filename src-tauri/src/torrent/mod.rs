//! Torrent client abstraction for downloading ROMs from Minerva Archive
//!
//! Supports multiple torrent clients:
//! - **embedded** (librqbit, requires `minerva-torrent` feature)
//! - **qbittorrent** (HTTP API)
//! - **transmission** (RPC)
//! - **deluge** (CLI via deluge-console)
//! - **rtorrent** (XML-RPC)
//! - **aria2** (JSON-RPC)
//! - **auto** (try embedded first, then probe external clients)

mod clients;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub use clients::create_client;

// ============================================================================
// Types (always available, no feature gate)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    FetchingTorrent,
    Downloading,
    Extracting,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub job_id: String,
    pub status: DownloadStatus,
    pub progress_percent: f64,
    pub download_speed: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub status_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFileInfo {
    pub index: usize,
    pub filename: String,
    pub size: u64,
}

// ============================================================================
// Progress tracking (global, shared across all client types)
// ============================================================================

static ACTIVE_DOWNLOADS: std::sync::OnceLock<std::sync::RwLock<HashMap<String, DownloadProgress>>> =
    std::sync::OnceLock::new();

fn downloads_map() -> &'static std::sync::RwLock<HashMap<String, DownloadProgress>> {
    ACTIVE_DOWNLOADS.get_or_init(|| std::sync::RwLock::new(HashMap::new()))
}

pub fn update_progress(
    job_id: &str,
    status: DownloadStatus,
    progress_percent: f64,
    download_speed: u64,
    downloaded_bytes: u64,
    total_bytes: u64,
    status_message: &str,
) {
    if let Ok(mut guard) = downloads_map().try_write() {
        guard.insert(
            job_id.to_string(),
            DownloadProgress {
                job_id: job_id.to_string(),
                status,
                progress_percent,
                download_speed,
                downloaded_bytes,
                total_bytes,
                status_message: status_message.to_string(),
            },
        );
    }
}

pub fn get_progress(job_id: &str) -> Option<DownloadProgress> {
    let guard = downloads_map().read().ok()?;
    guard.get(job_id).cloned()
}

pub fn clear_progress(job_id: &str) {
    if let Ok(mut guard) = downloads_map().write() {
        guard.remove(job_id);
    }
}

// ============================================================================
// Torrent metadata parsing (always available, uses lava_torrent)
// ============================================================================

/// Parse a .torrent file's metadata to extract the file listing.
/// Works without any torrent client — just reads the bencode metadata.
pub fn parse_torrent_metadata(torrent_bytes: &[u8]) -> Result<Vec<TorrentFileInfo>> {
    let torrent = lava_torrent::torrent::v1::Torrent::read_from_bytes(torrent_bytes)
        .map_err(|e| anyhow::anyhow!("failed to parse torrent file: {e}"))?;

    let mut files = Vec::new();

    if let Some(torrent_files) = torrent.files {
        for (idx, file) in torrent_files.iter().enumerate() {
            let filename = file
                .path
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            files.push(TorrentFileInfo {
                index: idx,
                filename,
                size: file.length as u64,
            });
        }
    } else {
        files.push(TorrentFileInfo {
            filename: torrent.name.clone(),
            size: torrent.length as u64,
            index: 0,
        });
    }

    Ok(files)
}

// ============================================================================
// HTTP helpers for fetching torrent files
// ============================================================================

/// Fetch a .torrent file from a URL with retry/backoff
pub async fn fetch_torrent_file(torrent_url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
        .build()?;

    let mut attempts = 0u32;
    loop {
        attempts += 1;
        let response = client.get(torrent_url).send().await?;
        let status = response.status();
        if status.is_success() {
            return Ok(response.bytes().await?.to_vec());
        }
        if (status.as_u16() == 429 || status.as_u16() == 503) && attempts <= 5 {
            let backoff = std::time::Duration::from_secs(2u64.pow(attempts));
            tracing::warn!("Rate limited ({status}), backing off {backoff:?}");
            tokio::time::sleep(backoff).await;
            continue;
        }
        bail!("HTTP {status} fetching torrent from {torrent_url}");
    }
}

// ============================================================================
// File linking utility
// ============================================================================

/// Link/copy a ROM file from source to the rom directory, organized by platform.
/// Returns the path to the linked/copied file.
pub fn link_rom_file(
    source: &Path,
    rom_dir: &Path,
    platform: &str,
    mode: &str,
) -> Result<PathBuf> {
    let target_dir = rom_dir.join(platform);
    std::fs::create_dir_all(&target_dir)?;
    let target = target_dir.join(source.file_name().unwrap_or_default());

    if target.exists() {
        return Ok(target);
    }

    match mode {
        "symlink" => {
            #[cfg(unix)]
            std::os::unix::fs::symlink(source, &target)?;
            #[cfg(windows)]
            std::fs::copy(source, &target)?;
        }
        "hardlink" => {
            std::fs::hard_link(source, &target)?;
        }
        "reflink" => {
            // Try reflink, fall back to copy
            if let Err(_) = reflink_copy(source, &target) {
                std::fs::copy(source, &target)?;
            }
        }
        "copy" => {
            std::fs::copy(source, &target)?;
        }
        "leave_in_place" => {
            // Don't create a link — the game_files entry will point to the source directly
            return Ok(source.to_path_buf());
        }
        _ => {
            // Default to symlink
            #[cfg(unix)]
            std::os::unix::fs::symlink(source, &target)?;
            #[cfg(windows)]
            std::fs::copy(source, &target)?;
        }
    }

    Ok(target)
}

/// Attempt a reflink (copy-on-write) copy. Falls back to regular copy.
fn reflink_copy(src: &Path, dst: &Path) -> Result<()> {
    // Reflink/CoW is filesystem-dependent. For now, fall back to regular copy.
    // Could use FICLONE ioctl on Linux/btrfs/xfs in the future.
    let _ = (src, dst);
    bail!("reflink not supported — will fall back to copy")
}
