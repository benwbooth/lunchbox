//! Embedded torrent client using librqbit

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, Session, SessionOptions,
    SessionPersistenceConfig, api::TorrentIdOrHash,
};
use tokio::sync::OnceCell;

use super::TorrentClient;
use crate::torrent::{DownloadProgress, DownloadStatus, TorrentFileInfo, update_progress};

static TORRENT_SESSION: OnceCell<Arc<Session>> = OnceCell::const_new();

pub struct EmbeddedClient;

impl EmbeddedClient {
    pub fn new() -> Self {
        Self
    }

    async fn get_session(download_dir: &Path) -> Result<Arc<Session>> {
        TORRENT_SESSION
            .get_or_try_init(|| async {
                tokio::fs::create_dir_all(download_dir).await?;
                let persistence_dir = download_dir.join(".rqbit-session");
                tokio::fs::create_dir_all(&persistence_dir).await?;

                let mut options = SessionOptions::default();
                options.fastresume = true;
                options.persistence = Some(SessionPersistenceConfig::Json {
                    folder: Some(persistence_dir),
                });

                let session = Session::new_with_opts(download_dir.to_path_buf(), options)
                    .await
                    .context("failed initializing librqbit session")?;
                Ok::<Arc<Session>, anyhow::Error>(session)
            })
            .await
            .cloned()
    }
}

#[async_trait]
impl TorrentClient for EmbeddedClient {
    async fn test_connection(&self) -> Result<String> {
        Ok("Embedded torrent client (librqbit) is available".to_string())
    }

    async fn add_torrent(
        &self,
        source: &str,
        download_dir: &Path,
        file_indices: Option<Vec<usize>>,
    ) -> Result<String> {
        let session = Self::get_session(download_dir).await?;

        let torrent_source = if source.starts_with("http://") || source.starts_with("https://") || source.starts_with("magnet:") {
            AddTorrent::from_url(source.to_string())
        } else {
            let bytes = tokio::fs::read(source).await?;
            AddTorrent::from_bytes(bytes)
        };

        let add_opts = AddTorrentOptions {
            paused: false,
            overwrite: true,
            output_folder: Some(download_dir.display().to_string()),
            only_files: file_indices.clone(),
            ..Default::default()
        };

        let response = session
            .add_torrent(torrent_source, Some(add_opts))
            .await
            .context("failed adding torrent")?;

        let (torrent_id, handle) = match response {
            AddTorrentResponse::Added(id, handle) => (id, handle),
            AddTorrentResponse::AlreadyManaged(id, handle) => {
                if let Some(ref indices) = file_indices {
                    let set: HashSet<usize> = indices.iter().copied().collect();
                    let _ = session.update_only_files(&handle, &set).await;
                }
                let _ = session.unpause(&handle).await;
                (id, handle)
            }
            AddTorrentResponse::ListOnly(_) => {
                bail!("torrent returned list-only response unexpectedly");
            }
        };

        // Wait for metadata
        tokio::time::timeout(Duration::from_secs(90), handle.wait_until_initialized())
            .await
            .context("timed out waiting for torrent metadata")?
            .context("torrent initialization failed")?;

        Ok(format!("embedded:{}", torrent_id.to_string()))
    }

    async fn get_progress(&self, job_id: &str) -> Result<Option<DownloadProgress>> {
        Ok(crate::torrent::get_progress(job_id))
    }

    async fn cancel(&self, job_id: &str) -> Result<()> {
        update_progress(job_id, DownloadStatus::Cancelled, 0.0, 0, 0, 0, "Cancelled");
        Ok(())
    }

    async fn list_files(&self, torrent_bytes: &[u8]) -> Result<Vec<TorrentFileInfo>> {
        crate::torrent::parse_torrent_metadata(torrent_bytes)
    }

    async fn get_downloaded_file_path(
        &self,
        _job_id: &str,
        _file_index: usize,
        _download_dir: &Path,
    ) -> Result<Option<PathBuf>> {
        // For embedded client, the file path is resolved by the download handler
        Ok(None)
    }
}
