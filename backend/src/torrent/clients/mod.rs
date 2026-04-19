//! Torrent client implementations
//!
//! Each client implements adding torrents, tracking progress, and cancellation.
//! The `create_client` factory returns the qBittorrent Web UI implementation.

mod qbittorrent;

use anyhow::{bail, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

use super::{DownloadProgress, TorrentFileInfo};
use crate::state::AppSettings;

/// Trait for torrent client implementations
#[async_trait]
pub trait TorrentClient: Send + Sync {
    /// Test if this client is reachable and working
    async fn test_connection(&self) -> Result<String>;

    /// Add a torrent and start downloading.
    /// `source`: torrent URL or path to .torrent file
    /// `download_dir`: where to save downloaded files
    /// `file_indices`: specific files to download (None = all)
    /// Returns a job ID for tracking progress.
    async fn add_torrent(
        &self,
        source: &str,
        download_dir: &Path,
        file_indices: Option<Vec<usize>>,
    ) -> Result<String>;

    /// Get download progress for a job
    async fn get_progress(&self, job_id: &str) -> Result<Option<DownloadProgress>>;

    /// Cancel an active download
    async fn cancel(&self, job_id: &str) -> Result<()>;

    /// List files in a torrent without downloading (metadata only)
    async fn list_files(&self, torrent_bytes: &[u8]) -> Result<Vec<TorrentFileInfo>>;

    /// Get the path where a completed file was saved
    async fn get_downloaded_file_path(
        &self,
        job_id: &str,
        file_index: usize,
        download_dir: &Path,
    ) -> Result<Option<PathBuf>>;
}

/// Create the configured torrent client.
///
/// Lunchbox only supports qBittorrent Web UI for Minerva downloads.
pub fn create_client(settings: &AppSettings) -> Result<Box<dyn TorrentClient>> {
    if settings.torrent.qbittorrent_host.trim().is_empty() {
        bail!("qBittorrent host is required");
    }
    if settings.torrent.qbittorrent_username.trim().is_empty() {
        bail!("qBittorrent username is required");
    }
    if settings.torrent.qbittorrent_password.trim().is_empty() {
        bail!("qBittorrent password is required");
    }
    Ok(Box::new(qbittorrent::QBittorrentClient::new(settings)))
}
