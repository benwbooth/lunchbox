//! qBittorrent torrent client via HTTP API

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use reqwest::multipart;

use super::TorrentClient;
use crate::torrent::{DownloadProgress, DownloadStatus, TorrentFileInfo};

#[derive(Debug, Clone)]
struct PathMapping {
    host_root: PathBuf,
    container_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QbJobRef {
    hash: String,
    selected_indices: Option<Vec<usize>>,
}

pub struct QBittorrentClient {
    host: String,
    port: u16,
    username: String,
    password: String,
    path_mappings: Vec<PathMapping>,
}

impl QBittorrentClient {
    pub fn new(settings: &crate::state::AppSettings) -> Self {
        let mut path_mappings = Vec::new();
        let rom_host_root = settings.get_rom_directory();
        let rom_container_root = settings.torrent.qbittorrent_container_rom_directory.clone();
        let torrent_host_root = settings.get_torrent_library_directory();
        let torrent_container_root = settings
            .torrent
            .qbittorrent_container_torrent_library_directory
            .clone();

        Self::push_path_mapping(
            &mut path_mappings,
            rom_host_root.clone(),
            rom_container_root.clone(),
        );
        Self::push_path_mapping(
            &mut path_mappings,
            torrent_host_root.clone(),
            torrent_container_root.clone(),
        );
        Self::push_common_parent_mapping(
            &mut path_mappings,
            &rom_host_root,
            rom_container_root.as_deref(),
            &torrent_host_root,
            torrent_container_root.as_deref(),
        );

        Self {
            host: settings.torrent.qbittorrent_host.clone(),
            port: settings.torrent.qbittorrent_port,
            username: settings.torrent.qbittorrent_username.clone(),
            password: settings.torrent.qbittorrent_password.clone(),
            path_mappings,
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    fn push_path_mapping(
        path_mappings: &mut Vec<PathMapping>,
        host_root: PathBuf,
        container_root: Option<PathBuf>,
    ) {
        let Some(container_root) = container_root else {
            return;
        };
        if container_root.as_os_str().is_empty() {
            return;
        }
        path_mappings.push(PathMapping {
            host_root,
            container_root,
        });
    }

    fn push_common_parent_mapping(
        path_mappings: &mut Vec<PathMapping>,
        host_a: &Path,
        container_a: Option<&Path>,
        host_b: &Path,
        container_b: Option<&Path>,
    ) {
        let (Some(container_a), Some(container_b)) = (container_a, container_b) else {
            return;
        };
        let Some(host_common) = Self::common_parent(host_a, host_b) else {
            return;
        };
        let Some(container_common) = Self::common_parent(container_a, container_b) else {
            return;
        };

        if host_common == host_a
            || host_common == host_b
            || container_common == container_a
            || container_common == container_b
            || host_common.as_os_str().is_empty()
            || container_common.as_os_str().is_empty()
        {
            return;
        }

        if path_mappings.iter().any(|mapping| {
            mapping.host_root == host_common && mapping.container_root == container_common
        }) {
            return;
        }

        path_mappings.push(PathMapping {
            host_root: host_common,
            container_root: container_common,
        });
    }

    fn common_parent(a: &Path, b: &Path) -> Option<PathBuf> {
        let a_components = a.components().collect::<Vec<_>>();
        let b_components = b.components().collect::<Vec<_>>();
        let mut common = PathBuf::new();
        let mut matched = 0usize;

        for (left, right) in a_components.iter().zip(b_components.iter()) {
            if left != right {
                break;
            }
            common.push(left.as_os_str());
            matched += 1;
        }

        (matched > 0).then_some(common)
    }

    fn remap_path(
        &self,
        path: &Path,
        select_roots: impl Fn(&PathMapping) -> (&Path, &Path),
    ) -> PathBuf {
        let mut best_match: Option<(&Path, &Path, PathBuf, usize)> = None;

        for mapping in &self.path_mappings {
            let (from_root, to_root) = select_roots(mapping);
            let Ok(relative) = path.strip_prefix(from_root) else {
                continue;
            };
            let depth = from_root.components().count();
            let should_replace = best_match
                .as_ref()
                .map(|(_, _, _, best_depth)| depth > *best_depth)
                .unwrap_or(true);
            if should_replace {
                best_match = Some((from_root, to_root, relative.to_path_buf(), depth));
            }
        }

        if let Some((_, to_root, relative, _)) = best_match {
            if relative.as_os_str().is_empty() {
                to_root.to_path_buf()
            } else {
                to_root.join(relative)
            }
        } else {
            path.to_path_buf()
        }
    }

    fn map_host_path_to_container(&self, path: &Path) -> PathBuf {
        self.remap_path(path, |mapping| {
            (&mapping.host_root, &mapping.container_root)
        })
    }

    fn map_container_path_to_host(&self, path: &Path) -> PathBuf {
        self.remap_path(path, |mapping| {
            (&mapping.container_root, &mapping.host_root)
        })
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

    fn map_download_status(state: &str, progress: f64) -> DownloadStatus {
        match state {
            "error" | "missingFiles" => DownloadStatus::Failed,
            "uploading" | "stalledUP" | "queuedUP" | "forcedUP" | "pausedUP" | "checkingUP" => {
                DownloadStatus::Completed
            }
            "pausedDL" => DownloadStatus::Paused,
            "allocating" | "checkingDL" | "checkingResumeData" | "downloading" | "forcedDL"
            | "metaDL" | "forcedMetaDL" | "moving" | "queuedDL" | "stalledDL" | "unknown" => {
                DownloadStatus::Downloading
            }
            _ if progress >= 0.999 => DownloadStatus::Completed,
            _ => DownloadStatus::Downloading,
        }
    }

    fn status_message(
        name: &str,
        state: &str,
        progress_percent: f64,
        status: DownloadStatus,
    ) -> String {
        match status {
            DownloadStatus::Completed => format!("Download complete: {name}"),
            DownloadStatus::Failed => {
                if state.is_empty() {
                    format!("qBittorrent reported an error for {name}")
                } else {
                    format!("qBittorrent reported error state '{state}' for {name}")
                }
            }
            DownloadStatus::Cancelled => format!("Download cancelled: {name}"),
            DownloadStatus::Paused => format!("Download paused: {name}"),
            DownloadStatus::Downloading => match state {
                "allocating" => format!("Allocating disk space for {name}..."),
                "checkingDL" | "checkingResumeData" | "checkingUP" => {
                    format!("Checking existing data for {name}...")
                }
                "metaDL" | "forcedMetaDL" => format!("Fetching torrent metadata for {name}..."),
                "moving" => format!("Moving files for {name}..."),
                "queuedDL" => format!("Queued in qBittorrent: {name}"),
                "stalledDL" => format!("Waiting for peers for {name}..."),
                "unknown" => format!("Preparing {name} in qBittorrent..."),
                _ => format!("Downloading {name}: {progress_percent:.1}%"),
            },
            DownloadStatus::FetchingTorrent | DownloadStatus::Extracting => {
                format!("Downloading {name}: {progress_percent:.1}%")
            }
        }
    }

    fn encode_job_id(hash: &str, selected_indices: Option<&[usize]>) -> String {
        if let Some(selected_indices) = selected_indices {
            let mut sorted = selected_indices.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            let encoded = sorted
                .iter()
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join(",");
            format!("qbt:{hash}#files={encoded}")
        } else {
            format!("qbt:{hash}")
        }
    }

    fn parse_job_id(job_id: &str) -> QbJobRef {
        let raw = job_id.strip_prefix("qbt:").unwrap_or(job_id);
        let Some((hash, encoded_files)) = raw.split_once("#files=") else {
            return QbJobRef {
                hash: raw.to_string(),
                selected_indices: None,
            };
        };

        let mut selected_indices = encoded_files
            .split(',')
            .filter_map(|part| part.parse::<usize>().ok())
            .collect::<Vec<_>>();
        selected_indices.sort_unstable();
        selected_indices.dedup();

        QbJobRef {
            hash: hash.to_string(),
            selected_indices: if selected_indices.is_empty() {
                None
            } else {
                Some(selected_indices)
            },
        }
    }

    async fn fetch_existing_torrent(
        &self,
        client: &reqwest::Client,
        hash: &str,
    ) -> Result<Option<serde_json::Value>> {
        let resp = client
            .get(format!(
                "{}/api/v2/torrents/info?hashes={hash}",
                self.base_url()
            ))
            .send()
            .await
            .context("qBittorrent torrent info request failed")?;

        if !resp.status().is_success() {
            bail!(
                "qBittorrent torrent info request failed: HTTP {}",
                resp.status()
            );
        }

        let torrents = resp
            .json::<Vec<serde_json::Value>>()
            .await
            .context("Failed to decode qBittorrent torrent info response")?;
        Ok(torrents.into_iter().next())
    }

    async fn fetch_torrent_files_with_retry(
        &self,
        client: &reqwest::Client,
        hash: &str,
        attempts: usize,
    ) -> Result<Vec<serde_json::Value>> {
        for attempt in 0..attempts {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            let files_resp = client
                .get(format!(
                    "{}/api/v2/torrents/files?hash={hash}",
                    self.base_url()
                ))
                .send()
                .await;

            let Ok(files_resp) = files_resp else {
                continue;
            };
            if !files_resp.status().is_success() {
                continue;
            }

            let Ok(parsed_files) = files_resp.json::<Vec<serde_json::Value>>().await else {
                continue;
            };
            if !parsed_files.is_empty() {
                return Ok(parsed_files);
            }
        }

        bail!("qBittorrent never exposed the torrent file list");
    }

    async fn set_file_priority_batch(
        &self,
        client: &reqwest::Client,
        hash: &str,
        ids: &[usize],
        priority: &str,
    ) -> Result<()> {
        const CHUNK_SIZE: usize = 500;

        for chunk in ids.chunks(CHUNK_SIZE) {
            let id_list = chunk
                .iter()
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join("|");

            let resp = client
                .post(format!("{}/api/v2/torrents/filePrio", self.base_url()))
                .form(&[
                    ("hash", hash),
                    ("id", id_list.as_str()),
                    ("priority", priority),
                ])
                .send()
                .await
                .context("qBittorrent file priority request failed")?;

            if !resp.status().is_success() {
                bail!(
                    "qBittorrent file priority request failed: HTTP {}",
                    resp.status()
                );
            }
        }

        Ok(())
    }

    async fn apply_file_priorities(
        &self,
        client: &reqwest::Client,
        hash: &str,
        file_count: usize,
        wanted: &HashSet<usize>,
    ) -> Result<()> {
        let unwanted_ids = (0..file_count)
            .filter(|idx| !wanted.contains(idx))
            .collect::<Vec<_>>();
        if !unwanted_ids.is_empty() {
            self.set_file_priority_batch(client, hash, &unwanted_ids, "0")
                .await?;
        }

        let mut wanted_ids = wanted.iter().copied().collect::<Vec<_>>();
        wanted_ids.sort_unstable();
        if !wanted_ids.is_empty() {
            self.set_file_priority_batch(client, hash, &wanted_ids, "7")
                .await?;
        }

        Ok(())
    }

    fn existing_torrent_file_candidates(
        &self,
        torrent: &serde_json::Value,
        file_name: &str,
    ) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(save_path) = torrent["save_path"].as_str() {
            let save_path = self.map_container_path_to_host(&PathBuf::from(save_path));
            candidates.push(save_path.join(file_name));
        }

        if let Some(content_path) = torrent["content_path"].as_str() {
            let content_path = self.map_container_path_to_host(&PathBuf::from(content_path));
            if content_path.is_file() {
                candidates.push(content_path);
            } else {
                let nested_name = file_name
                    .split_once('/')
                    .map(|(_, rest)| rest)
                    .unwrap_or(file_name);
                candidates.push(content_path.join(nested_name));
            }
        }

        candidates
    }

    async fn requested_files_missing_on_disk(
        &self,
        client: &reqwest::Client,
        hash: &str,
        torrent: &serde_json::Value,
        requested_indices: &[usize],
    ) -> Result<bool> {
        let files = self.fetch_torrent_files_with_retry(client, hash, 3).await?;

        for index in requested_indices {
            let Some(file) = files.get(*index) else {
                return Ok(true);
            };
            let file_name = file["name"].as_str().unwrap_or("");
            if file_name.is_empty() {
                return Ok(true);
            }

            let exists = self
                .existing_torrent_file_candidates(torrent, file_name)
                .into_iter()
                .any(|candidate| candidate.exists());
            if !exists {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn active_selected_indices(files: &[serde_json::Value]) -> HashSet<usize> {
        files
            .iter()
            .enumerate()
            .filter_map(|(idx, file)| {
                let priority = file["priority"].as_i64().unwrap_or(0);
                let progress = file["progress"].as_f64().unwrap_or(0.0);
                if priority > 0 || progress >= 0.999 {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    async fn resume_existing_torrent(
        &self,
        client: &reqwest::Client,
        hash: &str,
        existing_torrent: &serde_json::Value,
        file_indices: Option<Vec<usize>>,
    ) -> Result<String> {
        let category = existing_torrent["category"].as_str().unwrap_or("");
        if category != "lunchbox" {
            let torrent_name = existing_torrent["name"]
                .as_str()
                .unwrap_or("unknown torrent");
            let save_path = existing_torrent["save_path"].as_str().unwrap_or("");
            let category_label = if category.is_empty() {
                "(none)"
            } else {
                category
            };
            bail!(
                "qBittorrent already has this torrent outside Lunchbox. Matching torrent: '{torrent_name}' (hash {hash}, category {category_label}, save path {save_path}). Minerva torrents often share the same display name, so use the hash to find the right one in qBittorrent, then either remove it or set its category to 'lunchbox'."
            );
        }

        let files = self
            .fetch_torrent_files_with_retry(client, hash, 15)
            .await?;
        let wanted = if let Some(ref requested_indices) = file_indices {
            let mut wanted = Self::active_selected_indices(&files);
            wanted.extend(requested_indices.iter().copied());
            wanted
        } else {
            (0..files.len()).collect::<HashSet<_>>()
        };

        self.apply_file_priorities(client, hash, files.len(), &wanted)
            .await?;
        self.start_torrent(client, hash).await?;

        Ok(Self::encode_job_id(hash, file_indices.as_deref()))
    }

    fn selection_display_name(
        file_name: &str,
        torrent_name: &str,
        selected_count: usize,
    ) -> String {
        if selected_count <= 1 {
            Path::new(file_name)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(file_name)
                .to_string()
        } else {
            format!("{torrent_name} ({selected_count} files)")
        }
    }
}

impl QBittorrentClient {
    async fn pause_torrent(&self, client: &reqwest::Client, hash: &str) -> Result<()> {
        let resp = client
            .post(format!("{}/api/v2/torrents/pause", self.base_url()))
            .form(&[("hashes", hash)])
            .send()
            .await
            .with_context(|| format!("qBittorrent pause request failed for torrent {hash}"))?;

        if !resp.status().is_success() {
            bail!("qBittorrent pause request failed: HTTP {}", resp.status());
        }

        Ok(())
    }

    async fn start_torrent(&self, client: &reqwest::Client, hash: &str) -> Result<()> {
        for endpoint in ["start", "resume"] {
            let resp = client
                .post(format!("{}/api/v2/torrents/{endpoint}", self.base_url()))
                .form(&[("hashes", hash)])
                .send()
                .await
                .with_context(|| {
                    format!("qBittorrent {endpoint} request failed for torrent {hash}")
                })?;

            if resp.status().is_success() {
                return Ok(());
            }

            if resp.status() != reqwest::StatusCode::NOT_FOUND {
                bail!(
                    "qBittorrent {endpoint} request failed: HTTP {}",
                    resp.status()
                );
            }
        }

        bail!(
            "qBittorrent does not support either /torrents/start or /torrents/resume for torrent {hash}"
        );
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
        if let Some(existing_torrent) = self.fetch_existing_torrent(&client, &info_hash).await? {
            let state = existing_torrent["state"].as_str().unwrap_or("");
            let missing_requested_files = if let Some(ref indices) = file_indices {
                self.requested_files_missing_on_disk(
                    &client,
                    &info_hash,
                    &existing_torrent,
                    indices,
                )
                .await?
            } else {
                false
            };
            if state == "error" || state == "missingFiles" || missing_requested_files {
                tracing::info!(
                    hash = %info_hash,
                    state,
                    missing_requested_files,
                    "Resetting qBittorrent torrent before add"
                );
                let _ = client
                    .post(format!("{}/api/v2/torrents/delete", self.base_url()))
                    .form(&[("hashes", info_hash.as_str()), ("deleteFiles", "false")])
                    .send()
                    .await;
                tokio::time::sleep(Duration::from_secs(1)).await;
            } else {
                return self
                    .resume_existing_torrent(&client, &info_hash, &existing_torrent, file_indices)
                    .await;
            }
        }

        // Add torrent paused (so we can set file priorities before it starts)
        let should_pause = file_indices.is_some();
        let qb_download_dir = self.map_host_path_to_container(download_dir);
        let part = multipart::Part::bytes(torrent_bytes)
            .file_name("torrent.torrent")
            .mime_str("application/x-bittorrent")?;
        let form = multipart::Form::new()
            .part("torrents", part)
            .text("savepath", qb_download_dir.display().to_string())
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

        let response_body = resp.text().await.unwrap_or_default();
        if response_body.to_ascii_lowercase().contains("fail") {
            if let Some(existing_torrent) = self.fetch_existing_torrent(&client, &info_hash).await?
            {
                return self
                    .resume_existing_torrent(&client, &info_hash, &existing_torrent, file_indices)
                    .await;
            }
            bail!("qBittorrent refused the torrent add request: {response_body}");
        }

        // Set file priorities if specific files requested
        if let Some(indices) = file_indices {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let files = self
                .fetch_torrent_files_with_retry(&client, &info_hash, 15)
                .await?;
            let wanted = indices.iter().copied().collect::<HashSet<_>>();
            self.apply_file_priorities(&client, &info_hash, files.len(), &wanted)
                .await?;
            self.start_torrent(&client, &info_hash).await?;

            return Ok(Self::encode_job_id(&info_hash, Some(&indices)));
        }

        Ok(Self::encode_job_id(&info_hash, None))
    }

    async fn get_progress(&self, job_id: &str) -> Result<Option<DownloadProgress>> {
        let job_ref = Self::parse_job_id(job_id);
        let hash = job_ref.hash.as_str();
        let client = self.authenticated_client().await?;
        let Some(torrent) = self.fetch_existing_torrent(&client, hash).await? else {
            return Ok(None);
        };

        let torrent_name = torrent["name"].as_str().unwrap_or("torrent");
        let raw_progress = torrent["progress"].as_f64().unwrap_or(0.0);
        let state = torrent["state"].as_str().unwrap_or("");
        let download_speed = torrent["dlspeed"].as_u64().unwrap_or(0);
        let torrent_total_bytes = torrent["total_size"]
            .as_u64()
            .or_else(|| torrent["size"].as_u64())
            .unwrap_or(0);
        let torrent_downloaded_bytes = torrent["downloaded"]
            .as_u64()
            .unwrap_or_else(|| ((raw_progress * torrent_total_bytes as f64).round()) as u64);

        let (display_name, progress_percent, downloaded_bytes, total_bytes, status) =
            if let Some(selected_indices) = job_ref.selected_indices.as_ref() {
                let files = self
                    .fetch_torrent_files_with_retry(&client, hash, 3)
                    .await?;
                let mut selected_display_name = None;
                let mut selected_total_bytes = 0_u64;
                let mut selected_downloaded_bytes = 0_u64;
                let mut matched_files = 0usize;
                let mut all_complete = true;
                let mut any_incomplete_selected = false;
                let mut any_active_selected = false;

                for &idx in selected_indices {
                    let Some(file) = files.get(idx) else {
                        all_complete = false;
                        continue;
                    };
                    matched_files += 1;
                    let file_name = file["name"].as_str().unwrap_or(torrent_name);
                    let file_size = file["size"].as_u64().unwrap_or(0);
                    let file_progress = file["progress"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
                    let file_priority = file["priority"].as_i64().unwrap_or(0);
                    selected_total_bytes += file_size;
                    selected_downloaded_bytes +=
                        ((file_size as f64) * file_progress).round() as u64;
                    if file_progress < 0.999 {
                        all_complete = false;
                        any_incomplete_selected = true;
                        if file_priority > 0 {
                            any_active_selected = true;
                        }
                    }
                    if selected_display_name.is_none() {
                        selected_display_name = Some(Self::selection_display_name(
                            file_name,
                            torrent_name,
                            selected_indices.len(),
                        ));
                    }
                }

                let selected_progress = if selected_total_bytes > 0 {
                    (selected_downloaded_bytes as f64 / selected_total_bytes as f64) * 100.0
                } else {
                    raw_progress * 100.0
                }
                .clamp(0.0, 100.0);

                let status = if matches!(state, "error" | "missingFiles") {
                    DownloadStatus::Failed
                } else if matched_files > 0 && all_complete {
                    DownloadStatus::Completed
                } else if matched_files > 0 && any_incomplete_selected && !any_active_selected {
                    DownloadStatus::Paused
                } else {
                    Self::map_download_status(state, raw_progress)
                };

                (
                    selected_display_name.unwrap_or_else(|| torrent_name.to_string()),
                    selected_progress,
                    if selected_total_bytes > 0 {
                        selected_downloaded_bytes
                    } else {
                        torrent_downloaded_bytes
                    },
                    if selected_total_bytes > 0 {
                        selected_total_bytes
                    } else {
                        torrent_total_bytes
                    },
                    status,
                )
            } else {
                let progress_percent = (raw_progress * 100.0).clamp(0.0, 100.0);
                (
                    torrent_name.to_string(),
                    progress_percent,
                    torrent_downloaded_bytes,
                    torrent_total_bytes,
                    Self::map_download_status(state, raw_progress),
                )
            };
        let status_message = Self::status_message(&display_name, state, progress_percent, status);

        Ok(Some(DownloadProgress {
            job_id: job_id.to_string(),
            status,
            progress_percent,
            download_speed,
            downloaded_bytes,
            total_bytes,
            status_message,
        }))
    }

    async fn pause(&self, job_id: &str) -> Result<()> {
        let job_ref = Self::parse_job_id(job_id);
        let client = self.authenticated_client().await?;

        if let Some(selected_indices) = job_ref.selected_indices {
            let Some(existing_torrent) =
                self.fetch_existing_torrent(&client, &job_ref.hash).await?
            else {
                return Ok(());
            };
            let files = self
                .fetch_torrent_files_with_retry(&client, &job_ref.hash, 3)
                .await?;
            let pause_ids = selected_indices
                .iter()
                .copied()
                .filter(|idx| {
                    files
                        .get(*idx)
                        .map(|file| file["progress"].as_f64().unwrap_or(0.0) < 0.999)
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>();
            if !pause_ids.is_empty() {
                self.set_file_priority_batch(&client, &job_ref.hash, &pause_ids, "0")
                    .await?;
            }

            let pause_set = selected_indices.iter().copied().collect::<HashSet<_>>();
            let remaining_selected = Self::active_selected_indices(&files)
                .into_iter()
                .filter(|idx| !pause_set.contains(idx))
                .collect::<Vec<_>>();
            if remaining_selected.is_empty()
                && existing_torrent["progress"].as_f64().unwrap_or(0.0) < 0.999
            {
                self.pause_torrent(&client, &job_ref.hash).await?;
            }
        } else {
            self.pause_torrent(&client, &job_ref.hash).await?;
        }

        Ok(())
    }

    async fn resume(&self, job_id: &str) -> Result<()> {
        let job_ref = Self::parse_job_id(job_id);
        let client = self.authenticated_client().await?;

        if let Some(selected_indices) = job_ref.selected_indices {
            let Some(_) = self.fetch_existing_torrent(&client, &job_ref.hash).await? else {
                return Ok(());
            };
            let files = self
                .fetch_torrent_files_with_retry(&client, &job_ref.hash, 3)
                .await?;
            let mut wanted = Self::active_selected_indices(&files);
            wanted.extend(selected_indices.iter().copied());
            self.apply_file_priorities(&client, &job_ref.hash, files.len(), &wanted)
                .await?;
        }

        self.start_torrent(&client, &job_ref.hash).await?;
        Ok(())
    }

    async fn cancel(&self, job_id: &str) -> Result<()> {
        let job_ref = Self::parse_job_id(job_id);
        if let Ok(client) = self.authenticated_client().await {
            if let Some(selected_indices) = job_ref.selected_indices {
                if let Some(existing_torrent) =
                    self.fetch_existing_torrent(&client, &job_ref.hash).await?
                {
                    let category = existing_torrent["category"].as_str().unwrap_or("");
                    if category == "lunchbox" {
                        let files = self
                            .fetch_torrent_files_with_retry(&client, &job_ref.hash, 3)
                            .await?;
                        let cancel_set = selected_indices.iter().copied().collect::<HashSet<_>>();
                        let cancel_ids = selected_indices
                            .iter()
                            .copied()
                            .filter(|idx| {
                                files
                                    .get(*idx)
                                    .map(|file| file["progress"].as_f64().unwrap_or(0.0) < 0.999)
                                    .unwrap_or(true)
                            })
                            .collect::<Vec<_>>();
                        if !cancel_ids.is_empty() {
                            self.set_file_priority_batch(&client, &job_ref.hash, &cancel_ids, "0")
                                .await?;
                        }

                        let remaining_selected = Self::active_selected_indices(&files)
                            .into_iter()
                            .filter(|idx| !cancel_set.contains(idx))
                            .collect::<Vec<_>>();
                        if remaining_selected.is_empty() {
                            let _ = client
                                .post(format!("{}/api/v2/torrents/delete", self.base_url()))
                                .form(&[
                                    ("hashes", job_ref.hash.as_str()),
                                    ("deleteFiles", "false"),
                                ])
                                .send()
                                .await;
                        }
                    }
                }
            } else {
                let _ = client
                    .post(format!("{}/api/v2/torrents/delete", self.base_url()))
                    .form(&[("hashes", job_ref.hash.as_str()), ("deleteFiles", "true")])
                    .send()
                    .await;
            }
        }
        Ok(())
    }

    async fn list_files(&self, torrent_bytes: &[u8]) -> Result<Vec<TorrentFileInfo>> {
        crate::torrent::parse_torrent_metadata(torrent_bytes)
    }

    async fn get_downloaded_file_path(
        &self,
        job_id: &str,
        file_index: usize,
        download_dir: &Path,
    ) -> Result<Option<PathBuf>> {
        let job_ref = Self::parse_job_id(job_id);
        let hash = job_ref.hash.as_str();
        let client = self.authenticated_client().await?;

        let Some(torrent) = self.fetch_existing_torrent(&client, hash).await? else {
            return Ok(None);
        };

        let files = self
            .fetch_torrent_files_with_retry(&client, hash, 3)
            .await?;
        let Some(file) = files.get(file_index) else {
            return Ok(None);
        };

        let file_name = file["name"].as_str().unwrap_or("");
        if file_name.is_empty() {
            return Ok(None);
        }

        let save_path = torrent["save_path"]
            .as_str()
            .map(PathBuf::from)
            .map(|path| self.map_container_path_to_host(&path))
            .unwrap_or_else(|| download_dir.to_path_buf());
        let candidate = save_path.join(file_name);
        if candidate.exists() {
            return Ok(Some(candidate));
        }

        if let Some(content_path) = torrent["content_path"].as_str() {
            let content_path = self.map_container_path_to_host(&PathBuf::from(content_path));
            if content_path.is_file() {
                if content_path.exists() {
                    return Ok(Some(content_path));
                }
            } else {
                let nested_name = file_name
                    .split_once('/')
                    .map(|(_, rest)| rest)
                    .unwrap_or(file_name);
                let candidate = content_path.join(nested_name);
                if candidate.exists() {
                    return Ok(Some(candidate));
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::{QBittorrentClient, QbJobRef};
    use crate::torrent::DownloadStatus;
    use std::path::PathBuf;

    fn client_with_mappings() -> QBittorrentClient {
        let mut settings = crate::state::AppSettings::default();
        settings.torrent.rom_directory = Some(PathBuf::from("/mnt/stuff/Downloads/roms"));
        settings.torrent.qbittorrent_container_rom_directory =
            Some(PathBuf::from("/downloads/roms"));
        settings.torrent.torrent_library_directory =
            Some(PathBuf::from("/mnt/stuff/Downloads/torrent-library"));
        settings
            .torrent
            .qbittorrent_container_torrent_library_directory =
            Some(PathBuf::from("/downloads/torrent-library"));
        QBittorrentClient::new(&settings)
    }

    #[test]
    fn unknown_state_is_treated_as_in_progress() {
        assert_eq!(
            QBittorrentClient::map_download_status("unknown", 0.0),
            DownloadStatus::Downloading
        );
    }

    #[test]
    fn paused_download_state_is_treated_as_paused() {
        assert_eq!(
            QBittorrentClient::map_download_status("pausedDL", 0.25),
            DownloadStatus::Paused
        );
    }

    #[test]
    fn missing_files_is_treated_as_failure() {
        assert_eq!(
            QBittorrentClient::map_download_status("missingFiles", 0.0),
            DownloadStatus::Failed
        );
    }

    #[test]
    fn completed_upload_state_is_treated_as_complete() {
        assert_eq!(
            QBittorrentClient::map_download_status("stalledUP", 1.0),
            DownloadStatus::Completed
        );
    }

    #[test]
    fn maps_host_rom_paths_into_container_paths() {
        let client = client_with_mappings();
        assert_eq!(
            client.map_host_path_to_container(&PathBuf::from(
                "/mnt/stuff/Downloads/roms/Nintendo Entertainment System"
            )),
            PathBuf::from("/downloads/roms/Nintendo Entertainment System")
        );
    }

    #[test]
    fn maps_container_torrent_library_paths_back_to_host_paths() {
        let client = client_with_mappings();
        assert_eq!(
            client.map_container_path_to_host(&PathBuf::from(
                "/downloads/torrent-library/Nintendo Entertainment System/test-job"
            )),
            PathBuf::from(
                "/mnt/stuff/Downloads/torrent-library/Nintendo Entertainment System/test-job"
            )
        );
    }

    #[test]
    fn maps_shared_container_parent_paths_back_to_host_paths() {
        let client = client_with_mappings();
        assert_eq!(
            client.map_container_path_to_host(&PathBuf::from(
                "/downloads/Minerva_Myrient/eXo/eXoDOS/Full Release/eXo/eXoDOS/Commander Blood (1994).zip"
            )),
            PathBuf::from(
                "/mnt/stuff/Downloads/Minerva_Myrient/eXo/eXoDOS/Full Release/eXo/eXoDOS/Commander Blood (1994).zip"
            )
        );
    }

    #[test]
    fn encodes_and_parses_selected_file_job_ids() {
        let job_id = QBittorrentClient::encode_job_id("abc123", Some(&[7, 2, 7]));
        assert_eq!(job_id, "qbt:abc123#files=2,7");
        assert_eq!(
            QBittorrentClient::parse_job_id(&job_id),
            QbJobRef {
                hash: "abc123".to_string(),
                selected_indices: Some(vec![2, 7]),
            }
        );
    }

    #[test]
    fn parses_full_torrent_job_ids() {
        assert_eq!(
            QBittorrentClient::parse_job_id("qbt:def456"),
            QbJobRef {
                hash: "def456".to_string(),
                selected_indices: None,
            }
        );
    }
}
