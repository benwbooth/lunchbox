//! aria2 torrent client via JSON-RPC

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;

use super::TorrentClient;
use crate::torrent::{DownloadProgress, TorrentFileInfo};

pub struct Aria2Client {
    host: String,
    port: u16,
    secret: String,
}

impl Aria2Client {
    pub fn new(settings: &crate::state::TorrentSettings) -> Self {
        Self {
            host: settings.aria2_host.clone(),
            port: settings.aria2_port,
            secret: settings.aria2_secret.clone(),
        }
    }

    fn rpc_url(&self) -> String {
        format!("http://{}:{}/jsonrpc", self.host, self.port)
    }

    fn token_param(&self) -> String {
        if self.secret.is_empty() {
            String::new()
        } else {
            format!("token:{}", self.secret)
        }
    }

    async fn rpc_call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;

        let mut rpc_params = Vec::new();
        if !self.secret.is_empty() {
            rpc_params.push(serde_json::json!(self.token_param()));
        }
        if let serde_json::Value::Array(arr) = params {
            rpc_params.extend(arr);
        }

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "lunchbox",
            "method": method,
            "params": rpc_params,
        });

        let resp = client
            .post(&self.rpc_url())
            .json(&body)
            .send()
            .await
            .context("aria2 RPC request failed")?;

        if !resp.status().is_success() {
            bail!("aria2 RPC failed: HTTP {}", resp.status());
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(error) = json.get("error") {
            bail!("aria2 error: {}", error);
        }

        Ok(json)
    }
}

#[async_trait]
impl TorrentClient for Aria2Client {
    async fn test_connection(&self) -> Result<String> {
        let resp = self.rpc_call("aria2.getVersion", serde_json::json!([])).await?;
        let version = resp["result"]["version"]
            .as_str()
            .unwrap_or("unknown");
        Ok(format!("Connected to aria2 {version}"))
    }

    async fn add_torrent(
        &self,
        source: &str,
        download_dir: &Path,
        file_indices: Option<Vec<usize>>,
    ) -> Result<String> {
        let mut options = serde_json::json!({
            "dir": download_dir.display().to_string(),
        });

        if let Some(indices) = file_indices {
            // aria2 uses 1-based indices
            let select: Vec<String> = indices.iter().map(|i| (i + 1).to_string()).collect();
            options["select-file"] = serde_json::json!(select.join(","));
        }

        let resp = self
            .rpc_call(
                "aria2.addUri",
                serde_json::json!([[source], options]),
            )
            .await?;

        let gid = resp["result"].as_str().unwrap_or("unknown");
        Ok(format!("aria2:{gid}"))
    }

    async fn get_progress(&self, job_id: &str) -> Result<Option<DownloadProgress>> {
        Ok(crate::torrent::get_progress(job_id))
    }

    async fn cancel(&self, job_id: &str) -> Result<()> {
        let gid = job_id.strip_prefix("aria2:").unwrap_or(job_id);
        let _ = self
            .rpc_call("aria2.remove", serde_json::json!([gid]))
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
