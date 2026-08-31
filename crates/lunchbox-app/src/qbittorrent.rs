use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use lava_torrent::torrent::v1::Torrent;
use serde::Deserialize;
use ureq::unversioned::multipart::{Form, Part};

use crate::download_plan::DownloadPlan;
use crate::settings::{AppSettings, DownloadJob, NewDownloadJob};

const LUNCHBOX_CATEGORY: &str = "lunchbox";
const MINIMUM_STORAGE_RESERVE_BYTES: u64 = 64 * 1024 * 1024;
const MAXIMUM_STORAGE_RESERVE_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct EnqueueRequest {
    pub game_id: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub source_kind: String,
    pub torrent_url: String,
    pub torrent_bytes: Vec<u8>,
    pub selected_file_index: u32,
    pub selected_file_path: String,
    pub download_plan: Option<DownloadPlan>,
    pub adopt_existing_torrent: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MagnetMetadataReview {
    pub torrent_bytes: Vec<u8>,
    pub info_hash: String,
    pub created_for_review: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadPreflight {
    pub can_queue: bool,
    pub blocked_reason: String,
    pub selection_kind: String,
    pub selected_bytes: u64,
    pub required_download_bytes: u64,
    pub required_install_bytes: u64,
    pub download_available_bytes: u64,
    pub install_available_bytes: u64,
    pub shared_filesystem: bool,
    pub uses_install_destination: bool,
    pub download_path: PathBuf,
    pub install_path: PathBuf,
    pub file_link_mode: String,
}

#[derive(Clone, Debug)]
struct ReviewedTorrentFile {
    index: u32,
    path: String,
    byte_size: u64,
}

struct AddedTorrent {
    files: Vec<(u32, String)>,
    client_save_path: String,
    existed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TorrentSnapshot {
    pub state: String,
    pub progress: f64,
    pub download_speed: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
    pub client_save_path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExistingTorrentSummary {
    pub info_hash: String,
    pub name: String,
    pub category: String,
    pub state: String,
    pub progress: f64,
    pub size: u64,
    pub save_path: String,
}

#[derive(Debug, Deserialize)]
struct TorrentInfo {
    hash: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    progress: f64,
    #[serde(default)]
    dlspeed: u64,
    #[serde(default)]
    downloaded: u64,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    save_path: String,
}

#[derive(Debug, Deserialize)]
struct TorrentFileInfo {
    index: u32,
    name: String,
    size: u64,
    progress: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QbittorrentConnectionDetails {
    pub version: String,
    pub default_save_path: String,
}

#[derive(Debug, Deserialize)]
struct QbittorrentPreferences {
    #[serde(default)]
    save_path: String,
}

pub struct QbittorrentClient {
    base_url: String,
    agent: ureq::Agent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DownloadSelection {
    All,
    Exact(Vec<(u32, String)>),
}

impl QbittorrentClient {
    pub fn authenticated(settings: &AppSettings, password: &str) -> Result<Self> {
        settings.validate()?;
        let client = Self {
            base_url: settings.qbittorrent_base_url(),
            agent: ureq::Agent::config_builder()
                .timeout_connect(Some(Duration::from_secs(4)))
                .timeout_global(Some(Duration::from_secs(12)))
                .http_status_as_error(false)
                .user_agent("Lunchbox/0.1 qBittorrent integration")
                .build()
                .into(),
        };
        let url = client.endpoint("auth/login");
        let response = client
            .agent
            .post(&url)
            .header("Referer", &client.base_url)
            .send_form([
                ("username", settings.qbittorrent_username.as_str()),
                ("password", password),
            ])
            .with_context(|| format!("connecting to qBittorrent at {}", client.base_url))?;
        let (status, body) = response_text(response)?;
        if status != 200 || body.trim() != "Ok." {
            bail!(
                "qBittorrent rejected the login (HTTP {status}): {}",
                concise_body(&body)
            );
        }
        Ok(client)
    }

    pub fn version(&self) -> Result<String> {
        let response = self
            .agent
            .get(self.endpoint("app/version"))
            .header("Referer", &self.base_url)
            .call()
            .context("requesting qBittorrent version")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent version request")?;
        let version = body.trim();
        if version.is_empty() {
            bail!("qBittorrent returned an empty version");
        }
        Ok(version.to_owned())
    }

    fn default_save_path(&self) -> Result<String> {
        let response = self
            .agent
            .get(self.endpoint("app/preferences"))
            .header("Referer", &self.base_url)
            .call()
            .context("requesting qBittorrent download preferences")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent preferences request")?;
        let preferences =
            serde_json::from_str::<QbittorrentPreferences>(&body).with_context(|| {
                format!("decoding qBittorrent preferences: {}", concise_body(&body))
            })?;
        Ok(preferences.save_path.trim().to_owned())
    }

    pub fn add(
        &self,
        torrent_bytes: &[u8],
        info_hash: &str,
        save_path: &str,
        selection: &DownloadSelection,
        reviewed_files: &[(u32, String)],
    ) -> Result<Vec<(u32, String)>> {
        self.add_with_placement(
            torrent_bytes,
            info_hash,
            save_path,
            selection,
            reviewed_files,
            false,
        )
        .map(|added| added.files)
    }

    fn add_with_placement(
        &self,
        torrent_bytes: &[u8],
        info_hash: &str,
        save_path: &str,
        selection: &DownloadSelection,
        reviewed_files: &[(u32, String)],
        adopt_existing_torrent: bool,
    ) -> Result<AddedTorrent> {
        let (existed, info) = if let Some(mut existing) = self.torrent_info(info_hash)? {
            if existing.category != LUNCHBOX_CATEGORY && adopt_existing_torrent {
                self.ensure_category()?;
                self.set_category(info_hash, LUNCHBOX_CATEGORY)?;
                existing.category = LUNCHBOX_CATEGORY.to_owned();
            } else {
                ensure_owned(&existing)?;
            }
            (true, existing)
        } else {
            self.ensure_category()?;
            let torrent_part = Part::bytes(torrent_bytes)
                .file_name("lunchbox-source.torrent")
                .mime_str("application/x-bittorrent")?;
            let form = Form::new()
                .part("torrents", torrent_part)
                .text("savepath", save_path)
                .text("category", LUNCHBOX_CATEGORY)
                .text("autoTMM", "false")
                .text("stopped", "true")
                .text("paused", "true")
                .text("contentLayout", "Original")
                .text("root_folder", "true");
            let response = self
                .agent
                .post(self.endpoint("torrents/add"))
                .header("Referer", &self.base_url)
                .send(form)
                .context("adding the reviewed torrent to qBittorrent")?;
            let (status, body) = response_text(response)?;
            require_success(status, &body, "qBittorrent add request")?;

            let mut added = None;
            for _ in 0..10 {
                if let Some(info) = self.torrent_info(info_hash)? {
                    added = Some(info);
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            let info = added.context("qBittorrent accepted the torrent but did not index it")?;
            ensure_owned(&info)?;
            (false, info)
        };

        let files = self.apply_selection(info_hash, selection, existed, true)?;
        let files = reviewed_files
            .iter()
            .map(|(index, path)| {
                verified_file(&files, *index, path).map(|file| (file.index, file.name.clone()))
            })
            .collect::<Result<Vec<_>>>()?;
        let client_save_path = if info.save_path.trim().is_empty() {
            if existed {
                bail!("qBittorrent did not report the existing torrent save path");
            }
            save_path.to_owned()
        } else {
            info.save_path.trim().to_owned()
        };
        Ok(AddedTorrent {
            files,
            client_save_path,
            existed,
        })
    }

    fn add_magnet_for_review(
        &self,
        magnet_uri: &str,
        info_hash: &str,
        save_path: &str,
    ) -> Result<bool> {
        if let Some(existing) = self.torrent_info(info_hash)? {
            ensure_owned(&existing)?;
            return Ok(false);
        }
        self.ensure_category()?;
        let response = self
            .agent
            .post(self.endpoint("torrents/add"))
            .header("Referer", &self.base_url)
            .send_form([
                ("urls", magnet_uri),
                ("savepath", save_path),
                ("category", LUNCHBOX_CATEGORY),
                ("autoTMM", "false"),
                ("stopped", "true"),
                ("paused", "true"),
                ("contentLayout", "Original"),
                ("root_folder", "true"),
            ])
            .context("adding the magnet for metadata review")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent magnet review request")?;
        Ok(true)
    }

    fn export_torrent(&self, info_hash: &str, maximum_bytes: u64) -> Result<Option<Vec<u8>>> {
        let mut response = self
            .agent
            .get(self.endpoint("torrents/export"))
            .header("Referer", &self.base_url)
            .query("hash", info_hash)
            .call()
            .context("requesting reviewed torrent metadata from qBittorrent")?;
        let status = response.status().as_u16();
        if matches!(status, 404 | 409) {
            return Ok(None);
        }
        if !(200..300).contains(&status) {
            let body = response
                .body_mut()
                .with_config()
                .limit(4096)
                .lossy_utf8(true)
                .read_to_string()
                .unwrap_or_default();
            bail!(
                "qBittorrent torrent export failed (HTTP {status}): {}",
                concise_body(&body)
            );
        }
        let bytes = response
            .body_mut()
            .with_config()
            .limit(maximum_bytes.saturating_add(1))
            .read_to_vec()
            .context("reading reviewed torrent metadata from qBittorrent")?;
        if bytes.is_empty() {
            return Ok(None);
        }
        if bytes.len() as u64 > maximum_bytes {
            bail!("qBittorrent returned torrent metadata larger than the review limit");
        }
        Ok(Some(bytes))
    }

    pub fn snapshot(
        &self,
        info_hash: &str,
        selected_files: Option<&[(u32, String)]>,
    ) -> Result<TorrentSnapshot> {
        let info = self
            .torrent_info(info_hash)?
            .with_context(|| format!("torrent {info_hash} is no longer in qBittorrent"))?;
        ensure_owned(&info)?;
        let (progress, downloaded_bytes, total_bytes) = if let Some(selected_files) = selected_files
        {
            selected_snapshot(&self.files(info_hash)?, selected_files)?
        } else {
            (info.progress, info.downloaded, info.size)
        };
        let state = normalized_state(&info.state, progress);
        let message = match state.as_str() {
            "complete" => "Download complete".to_owned(),
            "paused" => "Paused in qBittorrent".to_owned(),
            "failed" => format!("qBittorrent reported {}", info.state),
            "queued" => "Waiting in qBittorrent".to_owned(),
            _ => format!(
                "Downloading at {}/s",
                crate::game_details::format_bytes(info.dlspeed)
            ),
        };
        Ok(TorrentSnapshot {
            state,
            progress: progress.clamp(0.0, 1.0),
            download_speed: info.dlspeed,
            downloaded_bytes,
            total_bytes,
            message,
            client_save_path: info.save_path,
        })
    }

    fn existing_torrents(&self) -> Result<Vec<ExistingTorrentSummary>> {
        let response = self
            .agent
            .get(self.endpoint("torrents/info"))
            .header("Referer", &self.base_url)
            .query("sort", "name")
            .query("limit", "5000")
            .call()
            .context("requesting existing qBittorrent torrents")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent torrent list request")?;
        let mut torrents = serde_json::from_str::<Vec<TorrentInfo>>(&body)
            .with_context(|| format!("decoding qBittorrent torrent list: {}", concise_body(&body)))?
            .into_iter()
            .filter(|torrent| {
                torrent.hash.len() == 40
                    && torrent.hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            .map(|torrent| ExistingTorrentSummary {
                info_hash: torrent.hash.to_ascii_lowercase(),
                name: torrent.name,
                category: torrent.category,
                state: torrent.state,
                progress: torrent.progress.clamp(0.0, 1.0),
                size: torrent.size,
                save_path: torrent.save_path,
            })
            .collect::<Vec<_>>();
        torrents.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.info_hash.cmp(&right.info_hash))
        });
        Ok(torrents)
    }

    fn pause(&self, info_hash: &str) -> Result<()> {
        self.control_with_fallback("torrents/stop", "torrents/pause", info_hash)
    }

    pub fn pause_owned(&self, info_hash: &str) -> Result<()> {
        let info = self
            .torrent_info(info_hash)?
            .with_context(|| format!("torrent {info_hash} is no longer in qBittorrent"))?;
        ensure_owned(&info)?;
        self.pause(info_hash)
    }

    fn resume(&self, info_hash: &str) -> Result<()> {
        self.start(info_hash)
    }

    pub fn resume_owned(&self, info_hash: &str) -> Result<()> {
        let info = self
            .torrent_info(info_hash)?
            .with_context(|| format!("torrent {info_hash} is no longer in qBittorrent"))?;
        ensure_owned(&info)?;
        self.resume(info_hash)
    }

    pub fn recover_owned(&self, info_hash: &str) -> Result<()> {
        let info = self
            .torrent_info(info_hash)?
            .with_context(|| format!("torrent {info_hash} is no longer in qBittorrent"))?;
        ensure_owned(&info)?;
        self.recheck(info_hash)?;
        self.resume(info_hash)
    }

    pub fn recover_or_readd_owned(
        &self,
        torrent_bytes: &[u8],
        info_hash: &str,
        client_save_path: &str,
        selection: &DownloadSelection,
        reviewed_files: &[(u32, String)],
    ) -> Result<bool> {
        if client_save_path.trim().is_empty() {
            bail!("recovery requires the exact persisted qBittorrent save path");
        }
        let torrent = Torrent::read_from_bytes(torrent_bytes)
            .map_err(|error| anyhow::anyhow!("retained torrent metadata is invalid: {error}"))?;
        if !torrent.info_hash().eq_ignore_ascii_case(info_hash) {
            bail!("retained torrent metadata does not match the failed download");
        }
        let added = self.add_with_placement(
            torrent_bytes,
            info_hash,
            client_save_path,
            selection,
            reviewed_files,
            false,
        )?;
        self.recheck(info_hash)?;
        self.resume(info_hash)?;
        Ok(!added.existed)
    }

    fn cancel(&self, info_hash: &str) -> Result<()> {
        let response = self
            .agent
            .post(self.endpoint("torrents/delete"))
            .header("Referer", &self.base_url)
            .send_form([("hashes", info_hash), ("deleteFiles", "false")])
            .context("removing torrent from qBittorrent")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent cancel request")
    }

    pub fn cancel_owned(&self, info_hash: &str) -> Result<()> {
        let info = self
            .torrent_info(info_hash)?
            .with_context(|| format!("torrent {info_hash} is no longer in qBittorrent"))?;
        ensure_owned(&info)?;
        self.cancel(info_hash)
    }

    fn discard_review_if_present(&self, info_hash: &str) -> Result<()> {
        let Some(info) = self.torrent_info(info_hash)? else {
            return Ok(());
        };
        ensure_owned(&info)?;
        self.cancel(info_hash)
    }

    pub fn reconfigure(
        &self,
        info_hash: &str,
        selection: &DownloadSelection,
        start_after: bool,
    ) -> Result<()> {
        let info = self
            .torrent_info(info_hash)?
            .with_context(|| format!("torrent {info_hash} is no longer in qBittorrent"))?;
        ensure_owned(&info)?;
        self.apply_selection(info_hash, selection, true, start_after)
            .map(|_| ())
    }

    fn start(&self, info_hash: &str) -> Result<()> {
        self.control_with_fallback("torrents/start", "torrents/resume", info_hash)
    }

    fn recheck(&self, info_hash: &str) -> Result<()> {
        let response = self
            .agent
            .post(self.endpoint("torrents/recheck"))
            .header("Referer", &self.base_url)
            .send_form([("hashes", info_hash)])
            .context("requesting a qBittorrent data recheck")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent data recheck")
    }

    fn apply_selection(
        &self,
        info_hash: &str,
        selection: &DownloadSelection,
        stop_first: bool,
        start_after: bool,
    ) -> Result<Vec<TorrentFileInfo>> {
        let files = self.files(info_hash)?;
        let all_indices = files
            .iter()
            .map(|file| file.index.to_string())
            .collect::<Vec<_>>()
            .join("|");
        if all_indices.is_empty() {
            bail!("qBittorrent returned no files for torrent {info_hash}");
        }

        match selection {
            DownloadSelection::All => {}
            DownloadSelection::Exact(selected_files) => {
                if selected_files.is_empty() {
                    bail!("at least one exact torrent file must be selected");
                }
                for (file_index, expected_path) in selected_files {
                    verified_file(&files, *file_index, expected_path)?;
                }
            }
        }

        if stop_first {
            self.pause(info_hash)?;
        }
        match selection {
            DownloadSelection::All => self.set_file_priority(info_hash, &all_indices, 1)?,
            DownloadSelection::Exact(selected_files) => {
                let selected_indices = selected_files
                    .iter()
                    .map(|(index, _)| index.to_string())
                    .collect::<Vec<_>>()
                    .join("|");
                self.set_file_priority(info_hash, &all_indices, 0)?;
                self.set_file_priority(info_hash, &selected_indices, 7)?;
            }
        }
        if start_after {
            self.start(info_hash)?;
        }
        Ok(files)
    }

    fn control_with_fallback(&self, primary: &str, legacy: &str, info_hash: &str) -> Result<()> {
        let response = self
            .agent
            .post(self.endpoint(primary))
            .header("Referer", &self.base_url)
            .send_form([("hashes", info_hash)])
            .with_context(|| format!("calling qBittorrent {primary}"))?;
        let (status, body) = response_text(response)?;
        if status == 404 {
            let response = self
                .agent
                .post(self.endpoint(legacy))
                .header("Referer", &self.base_url)
                .send_form([("hashes", info_hash)])
                .with_context(|| format!("calling legacy qBittorrent {legacy}"))?;
            let (status, body) = response_text(response)?;
            return require_success(status, &body, "legacy qBittorrent control request");
        }
        require_success(status, &body, "qBittorrent control request")
    }

    fn torrent_info(&self, info_hash: &str) -> Result<Option<TorrentInfo>> {
        let response = self
            .agent
            .get(self.endpoint("torrents/info"))
            .header("Referer", &self.base_url)
            .query("hashes", info_hash)
            .call()
            .context("requesting qBittorrent torrent state")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent torrent state request")?;
        let torrents: Vec<TorrentInfo> = serde_json::from_str(&body).with_context(|| {
            format!(
                "decoding qBittorrent torrent state: {}",
                concise_body(&body)
            )
        })?;
        Ok(torrents
            .into_iter()
            .find(|torrent| torrent.hash.eq_ignore_ascii_case(info_hash)))
    }

    fn files(&self, info_hash: &str) -> Result<Vec<TorrentFileInfo>> {
        let response = self
            .agent
            .get(self.endpoint("torrents/files"))
            .header("Referer", &self.base_url)
            .query("hash", info_hash)
            .call()
            .context("requesting qBittorrent torrent files")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent torrent files request")?;
        serde_json::from_str(&body).with_context(|| {
            format!(
                "decoding qBittorrent torrent files: {}",
                concise_body(&body)
            )
        })
    }

    fn set_file_priority(&self, info_hash: &str, ids: &str, priority: u8) -> Result<()> {
        let priority = priority.to_string();
        let response = self
            .agent
            .post(self.endpoint("torrents/filePrio"))
            .header("Referer", &self.base_url)
            .send_form([
                ("hash", info_hash),
                ("id", ids),
                ("priority", priority.as_str()),
            ])
            .context("setting qBittorrent file priorities")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent file priority request")
    }

    fn set_category(&self, info_hash: &str, category: &str) -> Result<()> {
        let response = self
            .agent
            .post(self.endpoint("torrents/setCategory"))
            .header("Referer", &self.base_url)
            .send_form([("hashes", info_hash), ("category", category)])
            .context("assigning the imported torrent to Lunchbox")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent category assignment")
    }

    fn ensure_category(&self) -> Result<()> {
        let response = self
            .agent
            .get(self.endpoint("torrents/categories"))
            .header("Referer", &self.base_url)
            .call()
            .context("requesting qBittorrent categories")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent category request")?;
        let categories = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&body)
            .with_context(|| format!("decoding qBittorrent categories: {}", concise_body(&body)))?;
        if categories.keys().any(|name| name == LUNCHBOX_CATEGORY) {
            return Ok(());
        }

        let response = self
            .agent
            .post(self.endpoint("torrents/createCategory"))
            .header("Referer", &self.base_url)
            .send_form([("category", LUNCHBOX_CATEGORY), ("savePath", "")])
            .context("creating the Lunchbox qBittorrent category")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent category creation")
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/api/v2/{path}", self.base_url.trim_end_matches('/'))
    }
}

#[cfg(test)]
pub fn test_connection(settings: &AppSettings, password: &str) -> Result<String> {
    QbittorrentClient::authenticated(settings, password)?.version()
}

pub fn test_connection_details(
    settings: &AppSettings,
    password: &str,
) -> Result<QbittorrentConnectionDetails> {
    let client = QbittorrentClient::authenticated(settings, password)?;
    let version = client.version()?;
    let default_save_path = client.default_save_path().unwrap_or_default();
    Ok(QbittorrentConnectionDetails {
        version,
        default_save_path,
    })
}

pub fn existing_torrents(
    settings: &AppSettings,
    password: &str,
) -> Result<Vec<ExistingTorrentSummary>> {
    QbittorrentClient::authenticated(settings, password)?.existing_torrents()
}

pub fn export_existing_torrent(
    settings: &AppSettings,
    password: &str,
    info_hash: &str,
    maximum_bytes: u64,
) -> Result<Vec<u8>> {
    if info_hash.len() != 40 || !info_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("existing torrent selection requires a hexadecimal v1 info hash");
    }
    let client = QbittorrentClient::authenticated(settings, password)?;
    client
        .torrent_info(info_hash)?
        .with_context(|| format!("torrent {info_hash} is no longer loaded in qBittorrent"))?;
    client
        .export_torrent(info_hash, maximum_bytes)?
        .context("qBittorrent did not provide exportable .torrent metadata for this item")
}

pub fn adopt_existing_torrent(
    settings: &AppSettings,
    password: &str,
    info_hash: &str,
) -> Result<()> {
    let client = QbittorrentClient::authenticated(settings, password)?;
    let info = client
        .torrent_info(info_hash)?
        .with_context(|| format!("torrent {info_hash} is no longer loaded in qBittorrent"))?;
    if info.category == LUNCHBOX_CATEGORY {
        return Ok(());
    }
    client.ensure_category()?;
    client.set_category(info_hash, LUNCHBOX_CATEGORY)
}

pub fn inspect_magnet_metadata(
    settings: &AppSettings,
    password: &str,
    magnet_uri: &str,
    info_hash: &str,
    client_save_path: &str,
    maximum_bytes: u64,
) -> Result<MagnetMetadataReview> {
    if info_hash.len() != 40 || !info_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("magnet review requires a hexadecimal v1 info hash");
    }
    if client_save_path.trim().is_empty() || maximum_bytes == 0 {
        bail!("magnet review requires a bounded managed destination");
    }
    let client = QbittorrentClient::authenticated(settings, password)?;
    let created_for_review = client.add_magnet_for_review(
        magnet_uri,
        &info_hash.to_ascii_lowercase(),
        client_save_path,
    )?;
    let deadline = Instant::now() + Duration::from_secs(15);
    let result = loop {
        match client.export_torrent(info_hash, maximum_bytes) {
            Ok(Some(torrent_bytes)) => break Ok(torrent_bytes),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(200)),
            Ok(None) => {
                break Err(anyhow::anyhow!(
                    "qBittorrent did not receive magnet metadata within 15 seconds; leave qBittorrent running and try again"
                ));
            }
            Err(error) => break Err(error),
        }
    };
    let torrent_bytes = match result {
        Ok(bytes) => bytes,
        Err(error) => {
            if created_for_review {
                let _ = client.cancel_owned(info_hash);
            }
            return Err(error);
        }
    };
    let validation = (|| -> Result<()> {
        if created_for_review {
            client.pause_owned(info_hash)?;
        }
        let torrent = Torrent::read_from_bytes(&torrent_bytes).map_err(|error| {
            anyhow::anyhow!("qBittorrent exported invalid torrent metadata: {error}")
        })?;
        if !torrent.info_hash().eq_ignore_ascii_case(info_hash) {
            bail!("qBittorrent metadata does not match the reviewed magnet info hash");
        }
        Ok(())
    })();
    if let Err(error) = validation {
        if created_for_review {
            let _ = client.cancel_owned(info_hash);
        }
        return Err(error);
    }
    Ok(MagnetMetadataReview {
        torrent_bytes,
        info_hash: info_hash.to_ascii_lowercase(),
        created_for_review,
    })
}

pub fn discard_magnet_review(
    settings: &AppSettings,
    password: &str,
    info_hash: &str,
) -> Result<()> {
    QbittorrentClient::authenticated(settings, password)?.discard_review_if_present(info_hash)
}

pub fn inspect_enqueue(
    settings: &AppSettings,
    store: &crate::settings::SettingsStore,
    request: &EnqueueRequest,
) -> Result<DownloadPreflight> {
    settings.validate()?;
    validate_download_directories(settings)?;

    let torrent = Torrent::read_from_bytes(&request.torrent_bytes)
        .map_err(|error| anyhow::anyhow!("could not parse selected torrent: {error}"))?;
    let info_hash = torrent.info_hash();
    let inventory = reviewed_torrent_inventory(torrent)?;
    let inventory_by_index = inventory
        .iter()
        .map(|file| (file.index, file))
        .collect::<HashMap<_, _>>();

    let download_plan = if settings.download_entire_torrent {
        None
    } else {
        request.download_plan.clone()
    };
    if let Some(plan) = &download_plan {
        plan.validate()?;
        if plan.representative_index
            != usize::try_from(request.selected_file_index)
                .context("torrent file index is too large")?
        {
            bail!("download plan representative does not match the reviewed file");
        }
    }
    let requested_files = requested_files(request, download_plan.as_ref())?;
    let selected_bytes = reviewed_selection_bytes(&inventory_by_index, &requested_files)?;
    let total_bytes = inventory.iter().try_fold(0_u64, |total, file| {
        total
            .checked_add(file.byte_size)
            .context("torrent payload byte size overflows")
    })?;
    if total_bytes == 0 || selected_bytes == 0 {
        bail!("the reviewed download contains no payload bytes");
    }

    let existing_jobs = store.jobs_for_info_hash(&info_hash)?;
    let duplicate = duplicate_download_reason(settings, &existing_jobs, request, &requested_files)?;
    let already_selected = existing_selected_paths(&existing_jobs)?;
    let required_download_bytes = if settings.download_entire_torrent {
        if existing_jobs
            .iter()
            .any(|job| is_managed_download(&job.state) && job.torrent_file_index.is_none())
        {
            0
        } else {
            inventory.iter().try_fold(0_u64, |total, file| {
                if already_selected
                    .iter()
                    .any(|selected| same_reviewed_file_path(selected, &file.path))
                {
                    Ok(total)
                } else {
                    total
                        .checked_add(file.byte_size)
                        .context("torrent payload byte size overflows")
                }
            })?
        }
    } else {
        requested_files
            .iter()
            .try_fold(0_u64, |total, (index, path)| {
                if already_selected
                    .iter()
                    .any(|selected| same_reviewed_file_path(selected, path))
                {
                    Ok(total)
                } else {
                    let file = inventory_by_index
                        .get(index)
                        .context("reviewed torrent selection is no longer available")?;
                    total
                        .checked_add(file.byte_size)
                        .context("reviewed selection byte size overflows")
                }
            })?
    };

    let reviewed_relative_file = safe_torrent_relative_path(&request.selected_file_path)?;
    let file_name = reviewed_relative_file
        .file_name()
        .context("selected torrent file has no filename")?;
    let planned_install_path = planned_local_target_path(
        &settings.rom_directory,
        &request.platform,
        &info_hash,
        file_name,
        download_plan.as_ref(),
    )?;
    let download_path = managed_native_download_path(&settings.torrent_library_directory);
    let download_anchor = existing_ancestor(&download_path)?;
    let uses_install_destination = settings.file_link_mode != "leave_in_place"
        || download_plan
            .as_ref()
            .is_some_and(DownloadPlan::is_arcade_machine_layout);
    let install_path = if uses_install_destination {
        planned_install_path
    } else {
        download_path.join(&reviewed_relative_file)
    };
    let install_anchor = if uses_install_destination {
        existing_ancestor(&install_path)?
    } else {
        download_anchor.clone()
    };
    let download_available_bytes = fs2::available_space(&download_anchor).with_context(|| {
        format!(
            "checking available space for {}",
            settings.torrent_library_directory.display()
        )
    })?;
    let install_available_bytes = if uses_install_destination {
        fs2::available_space(&install_anchor)
            .with_context(|| format!("checking available space for {}", install_path.display()))?
    } else {
        download_available_bytes
    };
    let shared_filesystem = same_filesystem(&download_anchor, &install_anchor)?;
    let required_install_bytes = if settings.file_link_mode == "copy" {
        selected_bytes
    } else {
        0
    };

    let download_storage_required = if shared_filesystem {
        storage_with_reserve(
            required_download_bytes
                .checked_add(required_install_bytes)
                .context("combined storage requirement overflows")?,
        )
    } else {
        storage_with_reserve(required_download_bytes)
    };
    let install_storage_required = if shared_filesystem {
        0
    } else {
        storage_with_reserve(required_install_bytes)
    };
    let storage_problem = if download_available_bytes < download_storage_required {
        Some(format!(
            "Not enough free space in the torrent library. The reviewed download and safety reserve need {}, but only {} is available.",
            crate::game_details::format_bytes(download_storage_required),
            crate::game_details::format_bytes(download_available_bytes)
        ))
    } else if install_available_bytes < install_storage_required {
        Some(format!(
            "Not enough free space in the ROM library for copy-on-import. The install and safety reserve need {}, but only {} is available.",
            crate::game_details::format_bytes(install_storage_required),
            crate::game_details::format_bytes(install_available_bytes)
        ))
    } else {
        None
    };
    let destination_problem = if uses_install_destination {
        match std::fs::symlink_metadata(&install_path) {
            Ok(_) => Some(format!(
                "The exact destination already exists at {}. Lunchbox will not replace it; use Library Audit or choose the existing game instead.",
                install_path.display()
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("checking destination {}", install_path.display()));
            }
        }
    } else {
        None
    };
    let blocked_reason = duplicate
        .or(destination_problem)
        .or(storage_problem)
        .unwrap_or_default();
    let selection_kind = if settings.download_entire_torrent {
        "whole torrent"
    } else if download_plan.is_some() {
        "reviewed set"
    } else {
        "exact file"
    };

    Ok(DownloadPreflight {
        can_queue: blocked_reason.is_empty(),
        blocked_reason,
        selection_kind: selection_kind.to_owned(),
        selected_bytes,
        required_download_bytes,
        required_install_bytes,
        download_available_bytes,
        install_available_bytes,
        shared_filesystem,
        uses_install_destination,
        download_path,
        install_path,
        file_link_mode: settings.file_link_mode.clone(),
    })
}

fn validate_download_directories(settings: &AppSettings) -> Result<()> {
    if settings.torrent_library_directory.as_os_str().is_empty() {
        bail!("choose a native torrent library directory in Settings");
    }
    if settings
        .qbittorrent_container_torrent_library_directory
        .trim()
        .is_empty()
    {
        bail!("enter the qBittorrent torrent-library path in Settings");
    }
    if settings.rom_directory.as_os_str().is_empty() {
        bail!("choose a native ROM directory in Settings");
    }
    Ok(())
}

fn requested_files(
    request: &EnqueueRequest,
    download_plan: Option<&DownloadPlan>,
) -> Result<Vec<(u32, String)>> {
    if let Some(plan) = download_plan {
        plan.selection()
            .into_iter()
            .map(|(index, path)| {
                Ok((
                    u32::try_from(index).context("torrent file index is too large")?,
                    path,
                ))
            })
            .collect()
    } else {
        Ok(vec![(
            request.selected_file_index,
            request.selected_file_path.clone(),
        )])
    }
}

fn reviewed_torrent_inventory(torrent: Torrent) -> Result<Vec<ReviewedTorrentFile>> {
    if let Some(files) = torrent.files {
        files
            .into_iter()
            .enumerate()
            .map(|(index, file)| {
                Ok(ReviewedTorrentFile {
                    index: u32::try_from(index).context("torrent file index is too large")?,
                    path: file
                        .path
                        .components()
                        .map(|component| component.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                    byte_size: u64::try_from(file.length)
                        .context("torrent file has a negative byte length")?,
                })
            })
            .collect()
    } else {
        Ok(vec![ReviewedTorrentFile {
            index: 0,
            path: torrent.name,
            byte_size: u64::try_from(torrent.length)
                .context("torrent payload has a negative byte length")?,
        }])
    }
}

fn reviewed_selection_bytes(
    inventory: &HashMap<u32, &ReviewedTorrentFile>,
    requested_files: &[(u32, String)],
) -> Result<u64> {
    requested_files
        .iter()
        .try_fold(0_u64, |total, (index, path)| {
            let file = inventory
                .get(index)
                .with_context(|| format!("torrent file index {index} is no longer available"))?;
            if !reviewed_path_matches(&file.path, path) {
                bail!(
                    "torrent file index {index} is {}, not the reviewed file {path}",
                    file.path
                );
            }
            total
                .checked_add(file.byte_size)
                .context("reviewed selection byte size overflows")
        })
}

fn duplicate_download_reason(
    settings: &AppSettings,
    existing_jobs: &[DownloadJob],
    request: &EnqueueRequest,
    requested_files: &[(u32, String)],
) -> Result<Option<String>> {
    for job in existing_jobs {
        let present = is_managed_download(&job.state)
            || (job.state == "imported"
                && if settings.file_link_mode == "leave_in_place" {
                    job.local_download_path.is_file()
                } else {
                    job.local_target_path.is_file()
                });
        if job.game_id != request.game_id || !present {
            continue;
        }
        let existing_files = job_exact_files(job)?.unwrap_or_default();
        if requested_files.iter().any(|(_, requested_path)| {
            existing_files
                .iter()
                .any(|(_, existing_path)| same_reviewed_file_path(existing_path, requested_path))
        }) {
            return Ok(Some(
                "This exact game download is already queued, downloaded, or installed. Open Downloads or My Collection instead of creating a duplicate."
                    .to_owned(),
            ));
        }
    }
    Ok(None)
}

fn existing_selected_paths(existing_jobs: &[DownloadJob]) -> Result<Vec<String>> {
    let mut selected = Vec::new();
    for job in existing_jobs
        .iter()
        .filter(|job| is_managed_download(&job.state))
    {
        if let Some(files) = job_exact_files(job)? {
            selected.extend(files.into_iter().map(|(_, path)| path));
        }
    }
    selected
        .sort_by(|left, right| normalized_torrent_path(left).cmp(&normalized_torrent_path(right)));
    selected.dedup_by(|left, right| same_reviewed_file_path(left, right));
    Ok(selected)
}

fn existing_ancestor(path: &Path) -> Result<PathBuf> {
    let mut candidate = path.to_path_buf();
    loop {
        match candidate.metadata() {
            Ok(_) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("inspecting storage path {}", candidate.display()));
            }
        }
        let Some(parent) = candidate.parent() else {
            bail!("no existing filesystem contains {}", path.display());
        };
        candidate = parent.to_path_buf();
    }
}

fn storage_with_reserve(bytes: u64) -> u64 {
    if bytes == 0 {
        return 0;
    }
    let reserve = (bytes / 20)
        .max(MINIMUM_STORAGE_RESERVE_BYTES)
        .min(MAXIMUM_STORAGE_RESERVE_BYTES);
    bytes.saturating_add(reserve)
}

#[cfg(unix)]
fn same_filesystem(left: &Path, right: &Path) -> Result<bool> {
    use std::os::unix::fs::MetadataExt;

    Ok(left.metadata()?.dev() == right.metadata()?.dev())
}

#[cfg(windows)]
fn same_filesystem(left: &Path, right: &Path) -> Result<bool> {
    use std::path::Prefix;

    fn volume(path: &Path) -> Option<String> {
        match path.components().next()? {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                    Some((letter as char).to_ascii_uppercase().to_string())
                }
                Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => Some(format!(
                    "{}\\{}",
                    server.to_string_lossy().to_ascii_lowercase(),
                    share.to_string_lossy().to_ascii_lowercase()
                )),
                _ => None,
            },
            _ => None,
        }
    }

    Ok(volume(left).is_some_and(|left| Some(left) == volume(right)))
}

#[cfg(not(any(unix, windows)))]
fn same_filesystem(_left: &Path, _right: &Path) -> Result<bool> {
    Ok(false)
}

pub fn enqueue(
    settings: &AppSettings,
    password: &str,
    store: &crate::settings::SettingsStore,
    request: EnqueueRequest,
) -> Result<DownloadJob> {
    settings.validate()?;
    if !matches!(request.source_kind.as_str(), "minerva" | "manual_torrent") {
        bail!("unsupported download source kind {}", request.source_kind);
    }
    validate_download_directories(settings)?;
    let preflight = inspect_enqueue(settings, store, &request)?;
    if !preflight.can_queue {
        bail!("{}", preflight.blocked_reason);
    }

    let torrent = Torrent::read_from_bytes(&request.torrent_bytes)
        .map_err(|error| anyhow::anyhow!("could not parse selected torrent: {error}"))?;
    let info_hash = torrent.info_hash();
    let retained_info_hash = store.retain_torrent_metadata(
        &request.source_kind,
        &request.torrent_url,
        &request.torrent_bytes,
    )?;
    if !retained_info_hash.eq_ignore_ascii_case(&info_hash) {
        bail!("retained torrent metadata identity changed during enqueue");
    }
    let existing_jobs = store.jobs_for_info_hash(&info_hash)?;
    let mut download_plan = if settings.download_entire_torrent {
        None
    } else {
        request.download_plan.clone()
    };
    if let Some(plan) = &download_plan {
        plan.validate()?;
        if plan.representative_index
            != usize::try_from(request.selected_file_index)
                .context("torrent file index is too large")?
        {
            bail!("download plan representative does not match the reviewed file");
        }
    }
    let requested_files = requested_files(&request, download_plan.as_ref())?;
    let native_download_path = managed_native_download_path(&settings.torrent_library_directory);
    std::fs::create_dir_all(&native_download_path).with_context(|| {
        format!(
            "creating native download directory {}",
            native_download_path.display()
        )
    })?;
    let requested_client_save_path =
        managed_client_save_path(&settings.qbittorrent_container_torrent_library_directory);

    let progress_file_index = if settings.download_entire_torrent {
        None
    } else {
        Some(request.selected_file_index)
    };
    let selection = active_selection(
        &existing_jobs,
        settings.download_entire_torrent,
        &requested_files,
    )?;
    let reviewed_relative_file = safe_torrent_relative_path(&request.selected_file_path)?;
    let file_name = reviewed_relative_file
        .file_name()
        .context("selected torrent file has no filename")?;
    let local_target_path = planned_local_target_path(
        &settings.rom_directory,
        &request.platform,
        &info_hash,
        file_name,
        download_plan.as_ref(),
    )?;

    let client = QbittorrentClient::authenticated(settings, password)?;
    if request.adopt_existing_torrent
        && let Some(existing) = client.torrent_info(&info_hash)?
    {
        if existing.save_path.trim().is_empty() {
            bail!("qBittorrent did not report a save path for the torrent being imported");
        }
        let existing_files = client.files(&info_hash)?;
        for (index, reviewed_path) in &requested_files {
            let actual = verified_file(&existing_files, *index, reviewed_path)?;
            native_path_for_client_file(
                &settings.torrent_library_directory,
                &settings.qbittorrent_container_torrent_library_directory,
                &existing.save_path,
                &actual.name,
            )
            .context(
                "the existing torrent is outside the configured qBittorrent/native library mapping; update that mapping in Settings before importing it",
            )?;
        }
    }
    let added = client.add_with_placement(
        &request.torrent_bytes,
        &info_hash,
        &requested_client_save_path,
        &selection,
        &requested_files,
        request.adopt_existing_torrent,
    )?;
    let actual_files = added.files;
    if let Some(plan) = download_plan.as_mut() {
        for member in &mut plan.members {
            let member_index =
                u32::try_from(member.index).context("torrent file index is too large")?;
            member.torrent_path = actual_files
                .iter()
                .find(|(index, _)| *index == member_index)
                .map(|(_, path)| path.clone())
                .with_context(|| {
                    format!("qBittorrent did not return planned file index {member_index}")
                })?;
        }
        plan.validate()?;
    }
    let actual_file_path = actual_files
        .iter()
        .find(|(index, _)| *index == request.selected_file_index)
        .map(|(_, path)| path.clone())
        .context("qBittorrent did not return the reviewed representative file")?;
    let client_save_path = added.client_save_path;
    let local_source_path = native_path_for_client_file(
        &settings.torrent_library_directory,
        &settings.qbittorrent_container_torrent_library_directory,
        &client_save_path,
        &actual_file_path,
    )?;
    let serialized_plan = download_plan
        .map(|plan| serde_json::to_string(&plan).context("encoding download plan"))
        .transpose()?
        .unwrap_or_default();

    Ok(DownloadJob::queued(NewDownloadJob {
        game_id: request.game_id,
        launchbox_db_id: request.launchbox_db_id,
        title: request.title,
        platform: request.platform,
        source_kind: request.source_kind,
        torrent_url: request.torrent_url,
        torrent_file_index: progress_file_index,
        torrent_file_path: actual_file_path,
        info_hash,
        client_save_path,
        local_download_path: local_source_path,
        local_target_path,
        download_plan: serialized_plan,
    }))
}

fn planned_local_target_path(
    rom_directory: &Path,
    platform: &str,
    info_hash: &str,
    reviewed_file_name: &OsStr,
    download_plan: Option<&DownloadPlan>,
) -> Result<PathBuf> {
    if let Some(plan) = download_plan
        && plan.is_optical_multidisc()
    {
        return Ok(rom_directory
            .join(safe_path_component(platform))
            .join(safe_path_component(&plan.display_name))
            .join(&plan.playlist_filename));
    }
    if let Some(plan) = download_plan
        && plan.is_arcade_machine_layout()
    {
        let representative = plan
            .representative_member()
            .context("arcade machine plan representative is missing")?;
        return Ok(rom_directory.join(safe_path_component(platform)).join(
            safe_torrent_relative_path(&representative.target_relative_path)?,
        ));
    }
    if let Some(plan) = download_plan {
        let representative = plan
            .representative_member()
            .context("download plan representative is missing")?;
        return Ok(rom_directory
            .join(".lunchbox-pc-archives")
            .join(info_hash)
            .join(safe_torrent_relative_path(
                &representative.target_relative_path,
            )?));
    }
    Ok(rom_directory
        .join(safe_path_component(platform))
        .join(reviewed_file_name))
}

pub fn active_selection(
    existing_jobs: &[DownloadJob],
    new_job_downloads_all: bool,
    new_exact_files: &[(u32, String)],
) -> Result<DownloadSelection> {
    let active_jobs = existing_jobs
        .iter()
        .filter(|job| is_managed_download(&job.state))
        .collect::<Vec<_>>();
    if new_job_downloads_all
        || active_jobs
            .iter()
            .any(|job| job.torrent_file_index.is_none())
    {
        return Ok(DownloadSelection::All);
    }

    let mut selected = Vec::new();
    for job in active_jobs {
        selected.extend(job_exact_files(job)?.unwrap_or_default());
    }
    selected.extend_from_slice(new_exact_files);
    selected.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    selected.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);
    if selected.is_empty() {
        bail!("at least one exact torrent file must be selected");
    }
    Ok(DownloadSelection::Exact(selected))
}

pub fn job_exact_files(job: &DownloadJob) -> Result<Option<Vec<(u32, String)>>> {
    let Some(representative_index) = job.torrent_file_index else {
        return Ok(None);
    };
    if let Some(plan) = job.parsed_download_plan()? {
        return plan
            .selection()
            .into_iter()
            .map(|(index, path)| {
                Ok((
                    u32::try_from(index).context("torrent file index is too large")?,
                    path,
                ))
            })
            .collect::<Result<Vec<_>>>()
            .map(Some);
    }
    Ok(Some(vec![(
        representative_index,
        job.torrent_file_path.clone(),
    )]))
}

fn is_managed_download(state: &str) -> bool {
    matches!(state, "queued" | "downloading" | "paused" | "complete")
}

pub(crate) fn is_collection_import_job(job: &DownloadJob) -> bool {
    job.game_id.starts_with("torrent-import:")
        && job
            .torrent_url
            .starts_with("manual-collection-torrent:sha256:")
}

fn response_text(mut response: ureq::http::Response<ureq::Body>) -> Result<(u16, String)> {
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .read_to_string()
        .context("reading qBittorrent response")?;
    Ok((status, body))
}

fn require_success(status: u16, body: &str, operation: &str) -> Result<()> {
    if (200..300).contains(&status) {
        Ok(())
    } else {
        bail!("{operation} failed (HTTP {status}): {}", concise_body(body))
    }
}

fn concise_body(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        "no response body".to_owned()
    } else {
        body.chars().take(240).collect()
    }
}

fn ensure_owned(info: &TorrentInfo) -> Result<()> {
    if info.category == LUNCHBOX_CATEGORY {
        Ok(())
    } else {
        bail!(
            "torrent {} already exists outside the Lunchbox qBittorrent category; refusing to modify it",
            info.hash
        )
    }
}

fn normalized_state(qbittorrent_state: &str, progress: f64) -> String {
    if progress >= 0.999_999 {
        return "complete".to_owned();
    }
    let state = qbittorrent_state.to_ascii_lowercase();
    if state.contains("error") || state.contains("missing") {
        "failed".to_owned()
    } else if state.contains("pause") || state.contains("stop") {
        "paused".to_owned()
    } else if state.contains("queue") || state.contains("check") || state.contains("meta") {
        "queued".to_owned()
    } else {
        "downloading".to_owned()
    }
}

pub(crate) fn client_path_join(root: &str, child: &str) -> String {
    let root = root.trim_end_matches(['/', '\\']);
    let separator = if root.contains('\\') && !root.contains('/') {
        '\\'
    } else {
        '/'
    };
    format!("{root}{separator}{child}")
}

pub(crate) fn managed_native_download_path(root: &Path) -> PathBuf {
    root.join("lunchbox").join("roms")
}

pub(crate) fn managed_client_save_path(root: &str) -> String {
    client_path_join(&client_path_join(root, "lunchbox"), "roms")
}

pub(crate) fn managed_collection_client_save_path(
    root: &str,
    platform: &str,
    info_hash: &str,
) -> String {
    let sources = client_path_join(&client_path_join(root, "lunchbox"), "sources");
    let platform = client_path_join(&sources, &safe_path_component(platform));
    client_path_join(&platform, info_hash)
}

pub(crate) fn native_path_for_client_file(
    native_library_root: &Path,
    client_library_root: &str,
    client_save_path: &str,
    torrent_file_path: &str,
) -> Result<PathBuf> {
    let client_file_path = client_path_join(client_save_path, torrent_file_path);
    let relative = client_relative_path(client_library_root, &client_file_path)?;
    Ok(native_library_root.join(relative))
}

pub(crate) fn native_path_for_client_save_path(
    native_library_root: &Path,
    client_library_root: &str,
    client_save_path: &str,
) -> Result<PathBuf> {
    let relative = client_relative_path(client_library_root, client_save_path)?;
    Ok(native_library_root.join(relative))
}

fn client_relative_path(client_library_root: &str, client_path: &str) -> Result<PathBuf> {
    let root = client_library_root.replace('\\', "/");
    let root = root.trim().trim_end_matches('/');
    let path = client_path.replace('\\', "/");
    let path = path.trim();
    if root.is_empty() || path.is_empty() {
        bail!("qBittorrent path mapping requires non-empty client paths");
    }

    let windows_style = root.contains(':') || client_library_root.contains('\\');
    let prefix_matches = if windows_style {
        path.get(..root.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(root))
    } else {
        path.starts_with(root)
    };
    if !prefix_matches {
        bail!(
            "qBittorrent save path {client_path} is outside the configured client torrent library {client_library_root}"
        );
    }
    let suffix = &path[root.len()..];
    if !suffix.is_empty() && !suffix.starts_with('/') {
        bail!(
            "qBittorrent save path {client_path} is outside the configured client torrent library {client_library_root}"
        );
    }
    safe_torrent_relative_path(suffix.trim_start_matches('/'))
}

pub(crate) fn safe_path_component(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() {
        "Unassigned platform".to_owned()
    } else if windows_reserved_component(cleaned) {
        format!("_{cleaned}")
    } else {
        cleaned.to_owned()
    }
}

fn windows_reserved_component(value: &str) -> bool {
    let stem = value
        .split_once('.')
        .map_or(value, |(stem, _extension)| stem)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

pub(crate) fn safe_torrent_relative_path(value: &str) -> Result<PathBuf> {
    let normalized = value.replace('\\', "/");
    let path = Path::new(&normalized);
    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("torrent file path is not a safe relative path")
            }
        }
    }
    if safe.as_os_str().is_empty() {
        bail!("torrent file path is empty");
    }
    Ok(safe)
}

fn normalized_torrent_path(value: &str) -> String {
    value.replace('\\', "/").trim_start_matches('/').to_owned()
}

fn verified_file<'a>(
    files: &'a [TorrentFileInfo],
    file_index: u32,
    expected_path: &str,
) -> Result<&'a TorrentFileInfo> {
    let selected = files
        .iter()
        .find(|file| file.index == file_index)
        .with_context(|| {
            format!("torrent file index {file_index} does not exist in qBittorrent")
        })?;
    if !reviewed_path_matches(&selected.name, expected_path) {
        bail!(
            "qBittorrent file index {file_index} is {}, not the reviewed file {expected_path}",
            selected.name
        );
    }
    Ok(selected)
}

fn selected_snapshot(
    files: &[TorrentFileInfo],
    selected_files: &[(u32, String)],
) -> Result<(f64, u64, u64)> {
    if selected_files.is_empty() {
        bail!("download selection contains no files");
    }
    let mut downloaded_bytes = 0_u64;
    let mut total_bytes = 0_u64;
    for (index, path) in selected_files {
        let file = verified_file(files, *index, path)?;
        total_bytes = total_bytes.saturating_add(file.size);
        downloaded_bytes = downloaded_bytes
            .saturating_add((file.size as f64 * file.progress.clamp(0.0, 1.0)) as u64);
    }
    let progress = if total_bytes == 0 {
        0.0
    } else {
        downloaded_bytes as f64 / total_bytes as f64
    };
    Ok((progress, downloaded_bytes, total_bytes))
}

fn reviewed_path_matches(actual_path: &str, expected_path: &str) -> bool {
    let actual = normalized_torrent_path(actual_path);
    let expected = normalized_torrent_path(expected_path);
    if actual == expected {
        return true;
    }
    let Some(prefix) = actual.strip_suffix(&expected) else {
        return false;
    };
    let Some(root) = prefix.strip_suffix('/') else {
        return false;
    };
    !root.is_empty() && !root.contains('/')
}

fn same_reviewed_file_path(left: &str, right: &str) -> bool {
    reviewed_path_matches(left, right) || reviewed_path_matches(right, left)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::mpsc;

    use super::*;
    use crate::arcade_download::build_mame_laserdisc_plans;
    use crate::download_plan::{DownloadPlan, DownloadPlanMember, TorrentPlanFile};

    struct MockResponse {
        body: &'static str,
        cookie: bool,
    }

    fn mock_server(
        responses: Vec<MockResponse>,
    ) -> (
        SocketAddr,
        mpsc::Receiver<String>,
        std::thread::JoinHandle<()>,
    ) {
        mock_server_bytes(
            responses
                .into_iter()
                .map(|response| (response.body.as_bytes().to_vec(), response.cookie))
                .collect(),
        )
    }

    fn mock_server_bytes(
        responses: Vec<(Vec<u8>, bool)>,
    ) -> (
        SocketAddr,
        mpsc::Receiver<String>,
        std::thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            for (body, set_cookie) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_http_request(&mut stream);
                request_sender.send(request).unwrap();
                let cookie = if set_cookie {
                    "Set-Cookie: SID=lunchbox-test; Path=/\r\n"
                } else {
                    ""
                };
                let headers = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n{cookie}Connection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(headers.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        (address, request_receiver, worker)
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut expected_length = None;
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if expected_length.is_none()
                && let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
            {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let body_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                expected_length = Some(header_end + 4 + body_length);
            }
            if expected_length.is_some_and(|length| bytes.len() >= length) {
                break;
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn settings_for(address: SocketAddr) -> AppSettings {
        AppSettings {
            qbittorrent_host: address.ip().to_string(),
            qbittorrent_port: address.port(),
            qbittorrent_username: "lunchbox".to_owned(),
            ..AppSettings::default()
        }
    }

    #[test]
    fn connection_uses_qbittorrent_login_cookie_for_the_version_request() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: "v5.1.2",
                cookie: false,
            },
        ]);

        let version = test_connection(&settings_for(address), "secret").unwrap();

        assert_eq!(version, "v5.1.2");
        let login = requests.recv().unwrap();
        assert!(login.starts_with("POST /api/v2/auth/login HTTP/1.1"));
        assert!(login.contains("username=lunchbox&password=secret"));
        assert!(
            login
                .to_ascii_lowercase()
                .contains(&format!("referer: http://{address}").to_ascii_lowercase())
        );
        let version_request = requests.recv().unwrap();
        assert!(version_request.starts_with("GET /api/v2/app/version HTTP/1.1"));
        assert!(
            version_request
                .to_ascii_lowercase()
                .contains("cookie: sid=lunchbox-test")
        );
        worker.join().unwrap();
    }

    #[test]
    fn magnet_review_exports_verified_metadata_and_leaves_new_payload_stopped() {
        let torrent_bytes = crate::external_torrent::probe_fixture_torrent_bytes();
        let torrent = Torrent::read_from_bytes(&torrent_bytes).unwrap();
        let info_hash = torrent.info_hash();
        let owned_info =
            format!(r#"[{{"hash":"{info_hash}","category":"lunchbox","state":"stoppedDL"}}]"#);
        let (address, requests, worker) = mock_server_bytes(vec![
            (b"Ok.".to_vec(), true),
            (b"[]".to_vec(), false),
            (
                br#"{"lunchbox":{"name":"lunchbox","savePath":""}}"#.to_vec(),
                false,
            ),
            (Vec::new(), false),
            (torrent_bytes.clone(), false),
            (owned_info.into_bytes(), false),
            (Vec::new(), false),
        ]);
        let magnet = format!("magnet:?xt=urn:btih:{info_hash}&dn=Reviewed+Collection");

        let review = inspect_magnet_metadata(
            &settings_for(address),
            "secret",
            &magnet,
            &info_hash,
            "/downloads/lunchbox/imports/Test",
            16 * 1024 * 1024,
        )
        .unwrap();

        assert!(review.created_for_review);
        assert_eq!(review.info_hash, info_hash);
        assert_eq!(review.torrent_bytes, torrent_bytes);
        let captured = (0..7).map(|_| requests.recv().unwrap()).collect::<Vec<_>>();
        assert!(captured[3].starts_with("POST /api/v2/torrents/add HTTP/1.1"));
        assert!(captured[4].starts_with("GET /api/v2/torrents/export?hash="));
        assert!(captured[6].starts_with("POST /api/v2/torrents/stop HTTP/1.1"));
        worker.join().unwrap();
    }

    #[test]
    fn connection_details_include_the_clients_default_download_path() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: "v5.1.2",
                cookie: false,
            },
            MockResponse {
                body: r#"{"save_path":"/downloads/"}"#,
                cookie: false,
            },
        ]);

        let details = test_connection_details(&settings_for(address), "secret").unwrap();

        assert_eq!(details.version, "v5.1.2");
        assert_eq!(details.default_save_path, "/downloads/");
        assert!(
            requests
                .recv()
                .unwrap()
                .starts_with("POST /api/v2/auth/login HTTP/1.1")
        );
        assert!(
            requests
                .recv()
                .unwrap()
                .starts_with("GET /api/v2/app/version HTTP/1.1")
        );
        assert!(
            requests
                .recv()
                .unwrap()
                .starts_with("GET /api/v2/app/preferences HTTP/1.1")
        );
        worker.join().unwrap();
    }

    #[test]
    fn existing_torrent_import_list_is_bounded_sorted_and_v1_only() {
        let first_hash = "1111111111111111111111111111111111111111";
        let second_hash = "2222222222222222222222222222222222222222";
        let body = format!(
            r#"[{{"hash":"{second_hash}","name":"Zelda","category":"switch","state":"uploading","progress":1.0,"size":42,"save_path":"/games"}},{{"hash":"v2-only","name":"Ignored"}},{{"hash":"{first_hash}","name":"Mario","state":"downloading","progress":0.5,"size":84,"save_path":"/games"}}]"#
        );
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: Box::leak(body.into_boxed_str()),
                cookie: false,
            },
        ]);

        let torrents = existing_torrents(&settings_for(address), "secret").unwrap();

        assert_eq!(torrents.len(), 2);
        assert_eq!(torrents[0].name, "Mario");
        assert_eq!(torrents[1].name, "Zelda");
        let _login = requests.recv().unwrap();
        assert!(
            requests
                .recv()
                .unwrap()
                .starts_with("GET /api/v2/torrents/info?sort=name&limit=5000 HTTP/1.1")
        );
        worker.join().unwrap();
    }

    #[test]
    fn adopting_an_existing_torrent_assigns_the_lunchbox_category() {
        let info_hash = "1111111111111111111111111111111111111111";
        let info = format!(r#"[{{"hash":"{info_hash}","category":"personal"}}]"#);
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: Box::leak(info.into_boxed_str()),
                cookie: false,
            },
            MockResponse {
                body: r#"{"lunchbox":{"name":"lunchbox","savePath":""}}"#,
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
        ]);

        adopt_existing_torrent(&settings_for(address), "secret", info_hash).unwrap();

        let captured = (0..4).map(|_| requests.recv().unwrap()).collect::<Vec<_>>();
        assert!(captured[1].contains("torrents/info?hashes="));
        assert!(captured[2].starts_with("GET /api/v2/torrents/categories HTTP/1.1"));
        assert!(captured[3].starts_with("POST /api/v2/torrents/setCategory HTTP/1.1"));
        assert!(captured[3].contains("category=lunchbox"));
        worker.join().unwrap();
    }

    #[test]
    fn exporting_an_existing_torrent_is_read_only_and_identity_bounded() {
        let torrent_bytes = crate::external_torrent::probe_fixture_torrent_bytes();
        let info_hash = Torrent::read_from_bytes(&torrent_bytes)
            .unwrap()
            .info_hash();
        let info = format!(r#"[{{"hash":"{info_hash}","category":"personal"}}]"#);
        let (address, requests, worker) = mock_server_bytes(vec![
            (b"Ok.".to_vec(), true),
            (info.into_bytes(), false),
            (torrent_bytes.clone(), false),
        ]);

        let exported = export_existing_torrent(
            &settings_for(address),
            "secret",
            &info_hash,
            16 * 1024 * 1024,
        )
        .unwrap();

        assert_eq!(exported, torrent_bytes);
        let captured = (0..3).map(|_| requests.recv().unwrap()).collect::<Vec<_>>();
        assert!(captured[1].starts_with("GET /api/v2/torrents/info?hashes="));
        assert!(captured[2].starts_with("GET /api/v2/torrents/export?hash="));
        assert!(
            captured
                .iter()
                .all(|request| !request.contains("setCategory"))
        );
        worker.join().unwrap();
    }

    #[test]
    fn existing_lunchbox_category_is_left_unchanged() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: r#"{"lunchbox":{"name":"lunchbox","savePath":"/custom/path"}}"#,
                cookie: false,
            },
        ]);
        let client = QbittorrentClient::authenticated(&settings_for(address), "secret").unwrap();

        client.ensure_category().unwrap();

        let _login = requests.recv().unwrap();
        let categories = requests.recv().unwrap();
        assert!(categories.starts_with("GET /api/v2/torrents/categories HTTP/1.1"));
        worker.join().unwrap();
    }

    #[test]
    fn owned_pause_rechecks_category_and_uses_the_non_destructive_stop_endpoint() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: r#"[{"hash":"abc123","category":"lunchbox"}]"#,
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
        ]);
        let client = QbittorrentClient::authenticated(&settings_for(address), "secret").unwrap();

        client.pause_owned("ABC123").unwrap();

        let _login = requests.recv().unwrap();
        let ownership_check = requests.recv().unwrap();
        assert!(ownership_check.starts_with("GET /api/v2/torrents/info?hashes=ABC123 HTTP/1.1"));
        let pause = requests.recv().unwrap();
        assert!(pause.starts_with("POST /api/v2/torrents/stop HTTP/1.1"));
        assert!(pause.contains("hashes=ABC123"));
        assert!(!pause.contains("deleteFiles"));
        worker.join().unwrap();
    }

    #[test]
    fn owned_pause_rejects_an_unrelated_torrent_before_control() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: r#"[{"hash":"abc123","category":"personal"}]"#,
                cookie: false,
            },
        ]);
        let client = QbittorrentClient::authenticated(&settings_for(address), "secret").unwrap();

        assert!(client.pause_owned("abc123").is_err());

        let _login = requests.recv().unwrap();
        let ownership_check = requests.recv().unwrap();
        assert!(ownership_check.starts_with("GET /api/v2/torrents/info?hashes=abc123 HTTP/1.1"));
        worker.join().unwrap();
    }

    #[test]
    fn owned_resume_and_cancel_recheck_category_before_mutating() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: r#"[{"hash":"abc123","category":"lunchbox"}]"#,
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
            MockResponse {
                body: r#"[{"hash":"abc123","category":"lunchbox"}]"#,
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
        ]);
        let client = QbittorrentClient::authenticated(&settings_for(address), "secret").unwrap();

        client.resume_owned("ABC123").unwrap();
        client.cancel_owned("ABC123").unwrap();

        let _login = requests.recv().unwrap();
        let resume_ownership = requests.recv().unwrap();
        assert!(resume_ownership.starts_with("GET /api/v2/torrents/info?hashes=ABC123 HTTP/1.1"));
        let resume = requests.recv().unwrap();
        assert!(resume.starts_with("POST /api/v2/torrents/start HTTP/1.1"));
        let cancel_ownership = requests.recv().unwrap();
        assert!(cancel_ownership.starts_with("GET /api/v2/torrents/info?hashes=ABC123 HTTP/1.1"));
        let cancel = requests.recv().unwrap();
        assert!(cancel.starts_with("POST /api/v2/torrents/delete HTTP/1.1"));
        assert!(cancel.contains("hashes=ABC123&deleteFiles=false"));
        worker.join().unwrap();
    }

    #[test]
    fn owned_recovery_rechecks_data_then_resumes_without_changing_selection() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: r#"[{"hash":"abc123","category":"lunchbox","state":"missingFiles","progress":0.4}]"#,
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
        ]);
        let client = QbittorrentClient::authenticated(&settings_for(address), "secret").unwrap();

        client.recover_owned("ABC123").unwrap();

        let _login = requests.recv().unwrap();
        let ownership = requests.recv().unwrap();
        assert!(ownership.starts_with("GET /api/v2/torrents/info?hashes=ABC123 HTTP/1.1"));
        let recheck = requests.recv().unwrap();
        assert!(recheck.starts_with("POST /api/v2/torrents/recheck HTTP/1.1"));
        assert!(recheck.contains("hashes=ABC123"));
        let resume = requests.recv().unwrap();
        assert!(resume.starts_with("POST /api/v2/torrents/start HTTP/1.1"));
        assert!(resume.contains("hashes=ABC123"));
        assert!(!recheck.contains("filePrio"));
        worker.join().unwrap();
    }

    #[test]
    fn missing_owned_torrent_is_recreated_from_exact_retained_metadata() {
        let torrent_bytes = crate::external_torrent::probe_fixture_torrent_bytes();
        let info_hash = Torrent::read_from_bytes(&torrent_bytes)
            .unwrap()
            .info_hash();
        let owned_info = format!(
            r#"[{{"hash":"{info_hash}","category":"lunchbox","save_path":"/downloads/lunchbox/roms"}}]"#
        );
        let files = r#"[{"index":0,"name":"Game/Sample Game.rom","size":4,"progress":0.0},{"index":1,"name":"Game/Sample Game (Europe).rom","size":6,"progress":0.0}]"#;
        let (address, requests, worker) = mock_server_bytes(vec![
            (b"Ok.".to_vec(), true),
            (b"[]".to_vec(), false),
            (
                br#"{"lunchbox":{"name":"lunchbox","savePath":""}}"#.to_vec(),
                false,
            ),
            (Vec::new(), false),
            (owned_info.into_bytes(), false),
            (files.as_bytes().to_vec(), false),
            (Vec::new(), false),
            (Vec::new(), false),
            (Vec::new(), false),
            (Vec::new(), false),
            (Vec::new(), false),
        ]);
        let client = QbittorrentClient::authenticated(&settings_for(address), "secret").unwrap();

        let recreated = client
            .recover_or_readd_owned(
                &torrent_bytes,
                &info_hash,
                "/downloads/lunchbox/roms",
                &DownloadSelection::Exact(vec![(1, "Game/Sample Game (Europe).rom".to_owned())]),
                &[(1, "Game/Sample Game (Europe).rom".to_owned())],
            )
            .unwrap();

        assert!(recreated);
        let captured = (0..11)
            .map(|_| requests.recv().unwrap())
            .collect::<Vec<_>>();
        assert!(captured[1].starts_with("GET /api/v2/torrents/info?hashes="));
        assert!(captured[3].starts_with("POST /api/v2/torrents/add HTTP/1.1"));
        assert!(captured[3].contains("name=\"savepath\"\r\n\r\n/downloads/lunchbox/roms"));
        assert!(captured[6].contains("priority=0"));
        assert!(captured[7].contains("id=1&priority=7"));
        assert!(captured[9].starts_with("POST /api/v2/torrents/recheck HTTP/1.1"));
        assert!(captured[10].starts_with("POST /api/v2/torrents/start HTTP/1.1"));
        worker.join().unwrap();
    }

    #[test]
    fn exact_file_set_add_is_owned_verified_prioritized_and_started() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: "[]",
                cookie: false,
            },
            MockResponse {
                body: "{}",
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
            MockResponse {
                body: r#"[{"hash":"abc123","category":"lunchbox"}]"#,
                cookie: false,
            },
            MockResponse {
                body: r#"[{"index":0,"name":"Minerva Bundle/Other.zip","size":12,"progress":0.0},{"index":3,"name":"Minerva Bundle/Game/Game (Disc 1).chd","size":42,"progress":0.0},{"index":4,"name":"Minerva Bundle/Game/Game (Disc 2).chd","size":84,"progress":0.0}]"#,
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
            MockResponse {
                body: "",
                cookie: false,
            },
        ]);
        let client = QbittorrentClient::authenticated(&settings_for(address), "secret").unwrap();

        let actual_paths = client
            .add(
                b"test torrent bytes",
                "abc123",
                r"D:\Downloads\lunchbox",
                &DownloadSelection::Exact(vec![
                    (3, r"Game\Game (Disc 1).chd".to_owned()),
                    (4, r"Game\Game (Disc 2).chd".to_owned()),
                ]),
                &[
                    (3, r"Game\Game (Disc 1).chd".to_owned()),
                    (4, r"Game\Game (Disc 2).chd".to_owned()),
                ],
            )
            .unwrap();

        assert_eq!(
            actual_paths,
            vec![
                (3, "Minerva Bundle/Game/Game (Disc 1).chd".to_owned()),
                (4, "Minerva Bundle/Game/Game (Disc 2).chd".to_owned()),
            ]
        );
        let captured = (0..10)
            .map(|_| requests.recv().unwrap())
            .collect::<Vec<_>>();
        assert!(captured[1].starts_with("GET /api/v2/torrents/info?hashes=abc123 HTTP/1.1"));
        assert!(captured[2].starts_with("GET /api/v2/torrents/categories HTTP/1.1"));
        assert!(captured[3].starts_with("POST /api/v2/torrents/createCategory HTTP/1.1"));
        assert!(captured[3].contains("category=lunchbox&savePath="));
        assert!(captured[4].starts_with("POST /api/v2/torrents/add HTTP/1.1"));
        assert!(captured[4].contains("name=\"category\"\r\n\r\nlunchbox"));
        assert!(captured[4].contains("name=\"savepath\"\r\n\r\nD:\\Downloads\\lunchbox"));
        assert!(captured[4].contains("name=\"autoTMM\"\r\n\r\nfalse"));
        assert!(captured[4].contains("name=\"stopped\"\r\n\r\ntrue"));
        assert!(captured[4].contains("name=\"paused\"\r\n\r\ntrue"));
        assert!(captured[4].contains("name=\"contentLayout\"\r\n\r\nOriginal"));
        assert!(captured[4].contains("name=\"root_folder\"\r\n\r\ntrue"));
        assert!(captured[6].starts_with("GET /api/v2/torrents/files?hash=abc123 HTTP/1.1"));
        assert!(captured[7].contains("hash=abc123&id=0%7C3%7C4&priority=0"));
        assert!(captured[8].contains("hash=abc123&id=3%7C4&priority=7"));
        assert!(captured[9].starts_with("POST /api/v2/torrents/start HTTP/1.1"));
        worker.join().unwrap();
    }

    #[test]
    #[ignore = "requires a live qBittorrent instance and credential-file environment variables"]
    fn live_connection_reports_a_real_version_without_mutating_the_client() {
        let username_file = std::env::var_os("LUNCHBOX_QBIT_USERNAME_FILE").unwrap();
        let password_file = std::env::var_os("LUNCHBOX_QBIT_PASSWORD_FILE").unwrap();
        let username = std::fs::read_to_string(username_file).unwrap();
        let password = std::fs::read_to_string(password_file).unwrap();
        let settings = AppSettings {
            qbittorrent_username: username.trim().to_owned(),
            ..AppSettings::default()
        };
        let version = test_connection(&settings, password.trim()).unwrap();
        assert!(!version.trim().is_empty());
    }

    #[test]
    fn client_path_join_preserves_the_configured_path_style() {
        assert_eq!(
            client_path_join("/downloads/", "lunchbox"),
            "/downloads/lunchbox"
        );
        assert_eq!(
            client_path_join(r"D:\Downloads\", "lunchbox"),
            r"D:\Downloads\lunchbox"
        );
        assert_eq!(
            managed_client_save_path("/downloads/"),
            "/downloads/lunchbox/roms"
        );
        assert_eq!(
            managed_client_save_path(r"D:\Downloads\"),
            r"D:\Downloads\lunchbox\roms"
        );
        assert_eq!(
            managed_native_download_path(Path::new("/native/downloads")),
            PathBuf::from("/native/downloads/lunchbox/roms")
        );
    }

    #[test]
    fn authoritative_client_save_paths_map_to_the_native_library() {
        assert_eq!(
            native_path_for_client_file(
                Path::new("/mnt/stuff/Downloads"),
                "/downloads",
                "/downloads/roms/Nintendo Entertainment System",
                "Minerva_Myrient/No-Intro/Faxanadu (USA) (Rev 1).zip",
            )
            .unwrap(),
            PathBuf::from(
                "/mnt/stuff/Downloads/roms/Nintendo Entertainment System/Minerva_Myrient/No-Intro/Faxanadu (USA) (Rev 1).zip"
            )
        );
        assert_eq!(
            native_path_for_client_file(
                Path::new(r"E:\Downloads"),
                r"D:\Downloads",
                r"d:\downloads\Lunchbox\roms",
                r"Pack\Game.zip",
            )
            .unwrap(),
            PathBuf::from(r"E:\Downloads").join("Lunchbox/roms/Pack/Game.zip")
        );
        assert!(
            native_path_for_client_file(
                Path::new("/native/downloads"),
                "/downloads",
                "/other",
                "Game.zip",
            )
            .is_err()
        );
        assert!(
            native_path_for_client_file(
                Path::new("/native/downloads"),
                "/downloads",
                "/downloads-old",
                "Game.zip",
            )
            .is_err()
        );
    }

    #[test]
    fn snapshot_reports_qbittorrents_authoritative_save_path() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                body: "Ok.",
                cookie: true,
            },
            MockResponse {
                body: r#"[{"hash":"abc123","category":"lunchbox","state":"uploading","progress":1.0,"downloaded":42,"size":42,"save_path":"/downloads/legacy"}]"#,
                cookie: false,
            },
        ]);
        let client = QbittorrentClient::authenticated(&settings_for(address), "secret").unwrap();

        let snapshot = client.snapshot("ABC123", None).unwrap();

        assert_eq!(snapshot.state, "complete");
        assert_eq!(snapshot.client_save_path, "/downloads/legacy");
        let _login = requests.recv().unwrap();
        assert!(
            requests
                .recv()
                .unwrap()
                .starts_with("GET /api/v2/torrents/info?hashes=ABC123 HTTP/1.1")
        );
        worker.join().unwrap();
    }

    #[test]
    fn torrent_paths_cannot_escape_the_download_root() {
        assert_eq!(
            safe_torrent_relative_path(r"Game Boy\Game.zip").unwrap(),
            PathBuf::from("Game Boy/Game.zip")
        );
        assert!(safe_torrent_relative_path("../outside.zip").is_err());
        assert!(safe_torrent_relative_path("/outside.zip").is_err());
    }

    #[test]
    fn reviewed_path_allows_only_qbittorrents_single_root_directory() {
        assert!(reviewed_path_matches(
            "Minerva Bundle/Game Boy/Game.zip",
            r"Game Boy\Game.zip"
        ));
        assert!(reviewed_path_matches(
            "Game Boy/Game.zip",
            r"Game Boy\Game.zip"
        ));
        assert!(!reviewed_path_matches(
            "Unexpected/Nested/Game Boy/Game.zip",
            "Game Boy/Game.zip"
        ));
        assert!(!reviewed_path_matches(
            "Minerva Bundle/Game Boy/Other.zip",
            "Game Boy/Game.zip"
        ));
    }

    #[test]
    fn reviewed_file_identity_accepts_one_qbittorrent_root_in_either_direction() {
        assert!(same_reviewed_file_path(
            "Sample Pack/Game/Sample Game.rom",
            "Game/Sample Game.rom"
        ));
        assert!(same_reviewed_file_path(
            "Game/Sample Game.rom",
            "Sample Pack/Game/Sample Game.rom"
        ));
        assert!(!same_reviewed_file_path(
            "Other Pack/Game/Sample Game.rom",
            "Sample Pack/Game/Sample Game.rom"
        ));
    }

    #[test]
    fn managed_collection_platform_component_is_cross_platform_safe() {
        assert_eq!(safe_path_component("Nintendo/Switch"), "Nintendo_Switch");
        assert_eq!(safe_path_component("CON"), "_CON");
        assert_eq!(safe_path_component("lpt9.games"), "_lpt9.games");
        assert_eq!(safe_path_component(" .. "), "Unassigned platform");
    }

    #[test]
    fn storage_reserve_is_bounded_and_zero_cost_stays_zero() {
        assert_eq!(storage_with_reserve(0), 0);
        assert_eq!(storage_with_reserve(1), 1 + MINIMUM_STORAGE_RESERVE_BYTES);
        assert_eq!(
            storage_with_reserve(100 * 1024 * 1024 * 1024),
            100 * 1024 * 1024 * 1024 + MAXIMUM_STORAGE_RESERVE_BYTES
        );
    }

    #[test]
    fn enqueue_preflight_checks_native_destination_but_leave_in_place_does_not() {
        let directory = tempfile::tempdir().unwrap();
        let downloads = directory.path().join("downloads");
        let roms = directory.path().join("roms");
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::create_dir_all(roms.join("Test Platform")).unwrap();
        let store = crate::settings::SettingsStore::at(directory.path().join("state.db")).unwrap();
        let request = EnqueueRequest {
            game_id: "sample-game".to_owned(),
            launchbox_db_id: 42,
            title: "Sample Game".to_owned(),
            platform: "Test Platform".to_owned(),
            source_kind: "minerva".to_owned(),
            torrent_url: "https://example.invalid/sample.torrent".to_owned(),
            torrent_bytes: crate::external_torrent::probe_fixture_torrent_bytes(),
            selected_file_index: 0,
            selected_file_path: "Game/Sample Game.rom".to_owned(),
            download_plan: None,
            adopt_existing_torrent: false,
        };
        let mut settings = AppSettings {
            rom_directory: roms.clone(),
            torrent_library_directory: downloads.clone(),
            qbittorrent_container_torrent_library_directory: "/downloads".to_owned(),
            file_link_mode: "copy".to_owned(),
            ..AppSettings::default()
        };

        let ready = inspect_enqueue(&settings, &store, &request).unwrap();
        assert!(ready.can_queue, "{}", ready.blocked_reason);
        assert!(ready.uses_install_destination);
        assert_eq!(ready.required_download_bytes, 4);
        assert_eq!(ready.required_install_bytes, 4);
        assert_eq!(
            ready.install_path,
            roms.join("Test Platform/Sample Game.rom")
        );

        std::fs::write(&ready.install_path, b"existing").unwrap();
        let blocked = inspect_enqueue(&settings, &store, &request).unwrap();
        assert!(!blocked.can_queue);
        assert!(
            blocked
                .blocked_reason
                .contains("destination already exists")
        );

        settings.file_link_mode = "leave_in_place".to_owned();
        let leave_in_place = inspect_enqueue(&settings, &store, &request).unwrap();
        assert!(
            leave_in_place.can_queue,
            "{}",
            leave_in_place.blocked_reason
        );
        assert!(!leave_in_place.uses_install_destination);
        assert_eq!(leave_in_place.required_install_bytes, 0);
        assert!(leave_in_place.install_path.starts_with(downloads));
        let info_hash = Torrent::read_from_bytes(&request.torrent_bytes)
            .unwrap()
            .info_hash();
        assert_eq!(store.retained_torrent_bytes(&info_hash).unwrap(), None);
    }

    #[test]
    fn state_mapping_is_stable_across_qbittorrent_versions() {
        assert_eq!(normalized_state("stoppedDL", 0.2), "paused");
        assert_eq!(normalized_state("pausedDL", 0.2), "paused");
        assert_eq!(normalized_state("queuedDL", 0.2), "queued");
        assert_eq!(normalized_state("error", 0.2), "failed");
        assert_eq!(normalized_state("missingFiles", 0.2), "failed");
        assert_eq!(normalized_state("uploading", 1.0), "complete");
    }

    #[test]
    fn unsafe_platform_characters_do_not_become_path_components() {
        assert_eq!(safe_path_component("Sony/PS2: USA"), "Sony_PS2_ USA");
        assert_eq!(safe_path_component("..."), "Unassigned platform");
    }

    #[test]
    fn exo_plan_targets_a_per_torrent_shared_archive_cache() {
        let plan = DownloadPlan {
            version: 1,
            kind: "exo_archive_set".into(),
            display_name: "Prince of Persia (1990)".into(),
            playlist_filename: String::new(),
            representative_index: 3,
            members: vec![
                DownloadPlanMember {
                    index: 3,
                    torrent_path: "eXo/eXoDOS/Prince of Persia (1990).zip".into(),
                    target_relative_path: "eXo/eXoDOS/Prince of Persia (1990).zip".into(),
                    byte_size: 30,
                    disc_index: None,
                    playlist_entry: false,
                    role: "primary".into(),
                },
                DownloadPlanMember {
                    index: 4,
                    torrent_path: "Content/!DOSmetadata.zip".into(),
                    target_relative_path: "Content/!DOSmetadata.zip".into(),
                    byte_size: 40,
                    disc_index: None,
                    playlist_entry: false,
                    role: "metadata".into(),
                },
            ],
        };
        plan.validate().unwrap();

        assert_eq!(
            planned_local_target_path(
                Path::new("/roms"),
                "MS-DOS",
                "abc123",
                OsStr::new("Prince of Persia (1990).zip"),
                Some(&plan),
            )
            .unwrap(),
            PathBuf::from(
                "/roms/.lunchbox-pc-archives/abc123/eXo/eXoDOS/Prince of Persia (1990).zip"
            )
        );
    }

    #[test]
    fn arcade_machine_plan_targets_the_platform_launch_layout() {
        let files = vec![
            TorrentPlanFile {
                index: 1,
                filename: "Laserdisc Collection/MAME/ROMs/dlair.zip".into(),
                byte_size: 10,
            },
            TorrentPlanFile {
                index: 2,
                filename: "Laserdisc Collection/MAME/CHD/dlair/dlair.chd".into(),
                byte_size: 20,
            },
        ];
        let plan = build_mame_laserdisc_plans(&files, "Dragon's Lair", &["dlair".into()])
            .pop()
            .unwrap();
        assert_eq!(
            planned_local_target_path(
                Path::new("/roms"),
                "Arcade Laserdisc",
                "unused",
                OsStr::new("dlair.zip"),
                Some(&plan),
            )
            .unwrap(),
            PathBuf::from("/roms/Arcade Laserdisc/MAME/roms/dlair.zip")
        );
    }

    #[test]
    fn active_selection_unions_files_from_a_shared_bundle() {
        let mut first = DownloadJob::queued(NewDownloadJob {
            game_id: "game-1".into(),
            launchbox_db_id: 1,
            title: "First".into(),
            platform: "Game Boy".into(),
            source_kind: "minerva".into(),
            torrent_url: "https://example.invalid/bundle.torrent".into(),
            torrent_file_index: Some(3),
            torrent_file_path: "Game Boy/First.zip".into(),
            info_hash: "abc123".into(),
            client_save_path: "/downloads/lunchbox".into(),
            local_download_path: PathBuf::from("/downloads/lunchbox/Game Boy/First.zip"),
            local_target_path: PathBuf::from("/roms/Game Boy/First.zip"),
            download_plan: String::new(),
        });
        first.state = "downloading".into();
        let selection =
            active_selection(&[first], false, &[(7, "Game Boy/Second.zip".into())]).unwrap();
        assert_eq!(
            selection,
            DownloadSelection::Exact(vec![
                (3, "Game Boy/First.zip".into()),
                (7, "Game Boy/Second.zip".into())
            ])
        );
    }

    #[test]
    fn active_selection_keeps_every_member_of_a_persisted_plan() {
        let plan = DownloadPlan {
            version: 1,
            kind: "optical_multidisc".into(),
            display_name: "Game".into(),
            playlist_filename: "Game.m3u".into(),
            representative_index: 3,
            members: vec![
                DownloadPlanMember {
                    index: 3,
                    torrent_path: "Game (Disc 1).chd".into(),
                    target_relative_path: "Game (Disc 1).chd".into(),
                    byte_size: 30,
                    disc_index: Some(1),
                    playlist_entry: true,
                    role: "disc".into(),
                },
                DownloadPlanMember {
                    index: 4,
                    torrent_path: "Game (Disc 2).chd".into(),
                    target_relative_path: "Game (Disc 2).chd".into(),
                    byte_size: 40,
                    disc_index: Some(2),
                    playlist_entry: true,
                    role: "disc".into(),
                },
            ],
        };
        let mut job = DownloadJob::queued(NewDownloadJob {
            game_id: "game-1".into(),
            launchbox_db_id: 1,
            title: "Game".into(),
            platform: "Platform".into(),
            source_kind: "minerva".into(),
            torrent_url: "https://example.invalid/bundle.torrent".into(),
            torrent_file_index: Some(3),
            torrent_file_path: "Game (Disc 1).chd".into(),
            info_hash: "abc123".into(),
            client_save_path: "/downloads/lunchbox".into(),
            local_download_path: PathBuf::from("/downloads/lunchbox/Game (Disc 1).chd"),
            local_target_path: PathBuf::from("/roms/Platform/Game/Game.m3u"),
            download_plan: serde_json::to_string(&plan).unwrap(),
        });
        job.state = "downloading".into();

        assert_eq!(
            active_selection(&[job], false, &[]).unwrap(),
            DownloadSelection::Exact(vec![
                (3, "Game (Disc 1).chd".into()),
                (4, "Game (Disc 2).chd".into()),
            ])
        );
    }

    #[test]
    fn selected_snapshot_weights_progress_by_required_file_size() {
        let files = vec![
            TorrentFileInfo {
                index: 3,
                name: "Game (Disc 1).chd".into(),
                size: 100,
                progress: 1.0,
            },
            TorrentFileInfo {
                index: 4,
                name: "Game (Disc 2).chd".into(),
                size: 300,
                progress: 0.0,
            },
        ];
        assert_eq!(
            selected_snapshot(
                &files,
                &[
                    (3, "Game (Disc 1).chd".into()),
                    (4, "Game (Disc 2).chd".into()),
                ],
            )
            .unwrap(),
            (0.25, 100, 400)
        );
    }

    #[test]
    fn active_whole_torrent_job_keeps_the_shared_bundle_complete() {
        let whole = DownloadJob::queued(NewDownloadJob {
            game_id: "game-1".into(),
            launchbox_db_id: 1,
            title: "First".into(),
            platform: "Game Boy".into(),
            source_kind: "minerva".into(),
            torrent_url: "https://example.invalid/bundle.torrent".into(),
            torrent_file_index: None,
            torrent_file_path: "Game Boy/First.zip".into(),
            info_hash: "abc123".into(),
            client_save_path: "/downloads/lunchbox".into(),
            local_download_path: PathBuf::from("/downloads/lunchbox/Game Boy/First.zip"),
            local_target_path: PathBuf::from("/roms/Game Boy/First.zip"),
            download_plan: String::new(),
        });
        assert_eq!(
            active_selection(&[whole], false, &[(7, "Game Boy/Second.zip".into())]).unwrap(),
            DownloadSelection::All
        );
    }
}
