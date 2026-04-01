//! Torrent client implementations
//!
//! Each client implements adding torrents, tracking progress, and cancellation.
//! The `create_client` factory selects the right implementation based on settings.

#[cfg(feature = "minerva-torrent")]
mod embedded;
mod qbittorrent;
mod transmission;
mod aria2;

use anyhow::{Result, bail};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

use super::{DownloadProgress, TorrentFileInfo};
use crate::state::TorrentSettings;

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

/// Create a torrent client based on settings
pub fn create_client(settings: &TorrentSettings) -> Result<Box<dyn TorrentClient>> {
    match settings.client.as_str() {
        "embedded" => create_embedded(settings),
        "qbittorrent" => Ok(Box::new(qbittorrent::QBittorrentClient::new(settings))),
        "transmission" => Ok(Box::new(transmission::TransmissionClient::new(settings))),
        "aria2" => Ok(Box::new(aria2::Aria2Client::new(settings))),
        "deluge" => bail!("Deluge client not yet implemented — use qBittorrent or Transmission"),
        "rtorrent" => bail!("rTorrent client not yet implemented — use qBittorrent or Transmission"),
        "auto" => create_auto(settings),
        other => bail!("Unknown torrent client: {other}"),
    }
}

fn create_embedded(_settings: &TorrentSettings) -> Result<Box<dyn TorrentClient>> {
    #[cfg(feature = "minerva-torrent")]
    {
        Ok(Box::new(embedded::EmbeddedClient::new()))
    }
    #[cfg(not(feature = "minerva-torrent"))]
    {
        bail!("Embedded torrent client not available (compile with minerva-torrent feature)")
    }
}

fn create_auto(settings: &TorrentSettings) -> Result<Box<dyn TorrentClient>> {
    // Try embedded first
    if let Ok(client) = create_embedded(settings) {
        return Ok(client);
    }
    // Fall back to qBittorrent if configured
    if !settings.qbittorrent_host.is_empty() {
        return Ok(Box::new(qbittorrent::QBittorrentClient::new(settings)));
    }
    // Fall back to Transmission
    if !settings.transmission_host.is_empty() {
        return Ok(Box::new(transmission::TransmissionClient::new(settings)));
    }
    // Fall back to aria2
    if !settings.aria2_host.is_empty() {
        return Ok(Box::new(aria2::Aria2Client::new(settings)));
    }
    // Default to embedded (will error if feature not enabled)
    create_embedded(settings)
}
