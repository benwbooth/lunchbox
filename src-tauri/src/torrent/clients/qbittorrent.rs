//! qBittorrent torrent client via HTTP API

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;

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
            .timeout(Duration::from_secs(15))
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

        let resp = client
            .post(format!("{}/api/v2/torrents/add", self.base_url()))
            .form(&[
                ("urls", source),
                ("savepath", &download_dir.display().to_string()),
                ("category", "lunchbox"),
            ])
            .send()
            .await
            .context("qBittorrent add torrent failed")?;

        if !resp.status().is_success() {
            bail!("qBittorrent add torrent failed: HTTP {}", resp.status());
        }

        // qBittorrent doesn't return a torrent ID directly — use the source hash
        let job_id = format!("qbt:{}", hash_source(source));

        // If file_indices specified, set file priority after a brief delay
        if let Some(indices) = file_indices {
            tokio::time::sleep(Duration::from_secs(2)).await;
            // Set unwanted files to priority 0, wanted to priority 7
            let hash = hash_source(source);
            if let Ok(files_resp) = client
                .get(format!("{}/api/v2/torrents/files?hash={hash}", self.base_url()))
                .send()
                .await
            {
                if let Ok(files) = files_resp.json::<Vec<serde_json::Value>>().await {
                    let wanted: std::collections::HashSet<usize> = indices.into_iter().collect();
                    for (i, _) in files.iter().enumerate() {
                        let priority = if wanted.contains(&i) { "7" } else { "0" };
                        let _ = client
                            .post(format!("{}/api/v2/torrents/filePrio", self.base_url()))
                            .form(&[
                                ("hash", hash.as_str()),
                                ("id", &i.to_string()),
                                ("priority", priority),
                            ])
                            .send()
                            .await;
                    }
                }
            }
        }

        Ok(job_id)
    }

    async fn get_progress(&self, job_id: &str) -> Result<Option<DownloadProgress>> {
        // Check local progress map first
        Ok(crate::torrent::get_progress(job_id))
    }

    async fn cancel(&self, job_id: &str) -> Result<()> {
        let hash = job_id.strip_prefix("qbt:").unwrap_or(job_id);
        let client = self.authenticated_client().await?;
        let _ = client
            .post(format!("{}/api/v2/torrents/delete", self.base_url()))
            .form(&[("hashes", hash), ("deleteFiles", "true")])
            .send()
            .await;
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

fn hash_source(source: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(source.as_bytes());
    hex::encode(&hash[..8])
}
