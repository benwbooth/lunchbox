//! Transmission torrent client via RPC

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;

use super::TorrentClient;
use crate::torrent::{DownloadProgress, TorrentFileInfo};

pub struct TransmissionClient {
    host: String,
    port: u16,
    username: String,
    password: String,
}

impl TransmissionClient {
    pub fn new(settings: &crate::state::TorrentSettings) -> Self {
        Self {
            host: settings.transmission_host.clone(),
            port: settings.transmission_port,
            username: settings.transmission_username.clone(),
            password: settings.transmission_password.clone(),
        }
    }

    fn rpc_url(&self) -> String {
        format!("http://{}:{}/transmission/rpc", self.host, self.port)
    }

    /// Transmission requires a session ID header. First request gets 409 + session ID.
    async fn rpc_call(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;

        let mut request = client.post(&self.rpc_url()).json(body);
        if !self.username.is_empty() {
            request = request.basic_auth(&self.username, Some(&self.password));
        }

        let resp = request.send().await?;

        if resp.status().as_u16() == 409 {
            // Get session ID from header
            let session_id = resp
                .headers()
                .get("X-Transmission-Session-Id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            let mut retry = client
                .post(&self.rpc_url())
                .header("X-Transmission-Session-Id", &session_id)
                .json(body);
            if !self.username.is_empty() {
                retry = retry.basic_auth(&self.username, Some(&self.password));
            }
            let resp2 = retry.send().await.context("Transmission RPC retry failed")?;
            if !resp2.status().is_success() {
                bail!("Transmission RPC failed: HTTP {}", resp2.status());
            }
            return resp2.json().await.context("failed parsing Transmission response");
        }

        if !resp.status().is_success() {
            bail!("Transmission RPC failed: HTTP {}", resp.status());
        }

        resp.json().await.context("failed parsing Transmission response")
    }
}

#[async_trait]
impl TorrentClient for TransmissionClient {
    async fn test_connection(&self) -> Result<String> {
        let resp = self
            .rpc_call(&serde_json::json!({
                "method": "session-get",
                "arguments": {}
            }))
            .await?;

        let version = resp["arguments"]["version"]
            .as_str()
            .unwrap_or("unknown");
        Ok(format!("Connected to Transmission {version}"))
    }

    async fn add_torrent(
        &self,
        source: &str,
        download_dir: &Path,
        file_indices: Option<Vec<usize>>,
    ) -> Result<String> {
        let mut args = serde_json::json!({
            "filename": source,
            "download-dir": download_dir.display().to_string(),
        });

        if let Some(indices) = file_indices {
            args["files-wanted"] = serde_json::json!(indices);
        }

        let resp = self
            .rpc_call(&serde_json::json!({
                "method": "torrent-add",
                "arguments": args,
            }))
            .await?;

        let id = resp["arguments"]["torrent-added"]["id"]
            .as_i64()
            .or_else(|| resp["arguments"]["torrent-duplicate"]["id"].as_i64())
            .unwrap_or(0);

        Ok(format!("transmission:{id}"))
    }

    async fn get_progress(&self, job_id: &str) -> Result<Option<DownloadProgress>> {
        Ok(crate::torrent::get_progress(job_id))
    }

    async fn cancel(&self, job_id: &str) -> Result<()> {
        let id_str = job_id.strip_prefix("transmission:").unwrap_or(job_id);
        if let Ok(id) = id_str.parse::<i64>() {
            let _ = self
                .rpc_call(&serde_json::json!({
                    "method": "torrent-remove",
                    "arguments": {
                        "ids": [id],
                        "delete-local-data": true,
                    }
                }))
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
