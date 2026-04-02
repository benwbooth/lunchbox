//! qBittorrent torrent client via HTTP API

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use reqwest::multipart;

use super::TorrentClient;
use crate::torrent::{DownloadProgress, TorrentFileInfo};

pub struct QBittorrentClient {
    host: String,
    port: u16,
    username: String,
    password: String,
}

impl QBittorrentClient {
    pub fn new(settings: &crate::state::TorrentSettings) -> Self {
        Self {
            host: settings.qbittorrent_host.clone(),
            port: settings.qbittorrent_port,
            username: settings.qbittorrent_username.clone(),
            password: settings.qbittorrent_password.clone(),
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    async fn authenticated_client(&self) -> Result<reqwest::Client> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .build()?;

        let login_resp = client
            .post(format!("{}/api/v2/auth/login", self.base_url()))
            .form(&[
                ("username", self.username.as_str()),
                ("password", self.password.as_str()),
            ])
            .send()
            .await
            .context("qBittorrent login request failed")?;

        if !login_resp.status().is_success() {
            bail!("qBittorrent login failed: HTTP {}", login_resp.status());
        }

        let body = login_resp.text().await.unwrap_or_default();
        if !body.to_lowercase().contains("ok") {
            bail!("qBittorrent rejected credentials");
        }

        Ok(client)
    }
}

#[async_trait]
impl TorrentClient for QBittorrentClient {
    async fn test_connection(&self) -> Result<String> {
        let client = self.authenticated_client().await?;
        let resp = client
            .get(format!("{}/api/v2/app/version", self.base_url()))
            .send()
            .await?;
        let version = resp.text().await.unwrap_or_default();
        Ok(format!("Connected to qBittorrent {version}"))
    }

    async fn add_torrent(
        &self,
        source: &str,
        download_dir: &Path,
        file_indices: Option<Vec<usize>>,
    ) -> Result<String> {
        let client = self.authenticated_client().await?;

        // Download the torrent file first so we can upload it
        let torrent_bytes = crate::torrent::fetch_torrent_file(source).await?;

        // Parse to get the info hash for later reference
        let info_hash = torrent_info_hash(&torrent_bytes);

        // Add torrent paused (so we can set file priorities before it starts)
        let should_pause = file_indices.is_some();
        let part = multipart::Part::bytes(torrent_bytes)
            .file_name("torrent.torrent")
            .mime_str("application/x-bittorrent")?;
        let form = multipart::Form::new()
            .part("torrents", part)
            .text("savepath", download_dir.display().to_string())
            .text("category", "lunchbox")
            .text("paused", if should_pause { "true" } else { "false" });

        let resp = client
            .post(format!("{}/api/v2/torrents/add", self.base_url()))
            .multipart(form)
            .send()
            .await
            .context("qBittorrent add torrent failed")?;

        if !resp.status().is_success() {
            bail!("qBittorrent add torrent failed: HTTP {}", resp.status());
        }

        // Set file priorities if specific files requested
        if let Some(indices) = file_indices {
            // Wait for qBittorrent to process the torrent metadata
            tokio::time::sleep(Duration::from_secs(3)).await;

            let hash = &info_hash;

            // Get file count
            let files_resp = client
                .get(format!("{}/api/v2/torrents/files?hash={hash}", self.base_url()))
                .send()
                .await;

            if let Ok(files_resp) = files_resp {
                if let Ok(files) = files_resp.json::<Vec<serde_json::Value>>().await {
                    let wanted: std::collections::HashSet<usize> = indices.into_iter().collect();

                    // Set all unwanted files to priority 0 in bulk
                    let unwanted_ids: Vec<String> = (0..files.len())
                        .filter(|i| !wanted.contains(i))
                        .map(|i| i.to_string())
                        .collect();

                    if !unwanted_ids.is_empty() {
                        // qBittorrent accepts pipe-separated IDs
                        let _ = client
                            .post(format!("{}/api/v2/torrents/filePrio", self.base_url()))
                            .form(&[
                                ("hash", hash.as_str()),
                                ("id", &unwanted_ids.join("|")),
                                ("priority", "0"),
                            ])
                            .send()
                            .await;
                    }

                    // Set wanted files to high priority
                    let wanted_ids: Vec<String> = wanted.iter().map(|i| i.to_string()).collect();
                    if !wanted_ids.is_empty() {
                        let _ = client
                            .post(format!("{}/api/v2/torrents/filePrio", self.base_url()))
                            .form(&[
                                ("hash", hash.as_str()),
                                ("id", &wanted_ids.join("|")),
                                ("priority", "7"),
                            ])
                            .send()
                            .await;
                    }
                }
            }

            // Resume the torrent now that priorities are set
            let _ = client
                .post(format!("{}/api/v2/torrents/resume", self.base_url()))
                .form(&[("hashes", hash.as_str())])
                .send()
                .await;
        }

        Ok(format!("qbt:{info_hash}"))
    }

    async fn get_progress(&self, job_id: &str) -> Result<Option<DownloadProgress>> {
        Ok(crate::torrent::get_progress(job_id))
    }

    async fn cancel(&self, job_id: &str) -> Result<()> {
        let hash = job_id.strip_prefix("qbt:").unwrap_or(job_id);
        if let Ok(client) = self.authenticated_client().await {
            let _ = client
                .post(format!("{}/api/v2/torrents/delete", self.base_url()))
                .form(&[("hashes", hash), ("deleteFiles", "true")])
                .send()
                .await;
        }
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
        Ok(None)
    }
}

/// Extract the info hash from a .torrent file for qBittorrent API calls
fn torrent_info_hash(torrent_bytes: &[u8]) -> String {
    use sha1::Digest;

    // Parse bencode to find the "info" dictionary and hash it
    if let Ok(torrent) = lava_torrent::torrent::v1::Torrent::read_from_bytes(torrent_bytes) {
        return torrent.info_hash().to_lowercase();
    }

    // Fallback: hash the whole torrent bytes (not ideal but better than nothing)
    let hash = sha1::Sha1::digest(torrent_bytes);
    hex::encode(hash)
}
