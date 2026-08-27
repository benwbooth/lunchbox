use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use lava_torrent::torrent::v1::Torrent;
use serde::Deserialize;
use ureq::unversioned::multipart::{Form, Part};

use crate::download_plan::DownloadPlan;
use crate::settings::{AppSettings, DownloadJob, NewDownloadJob};

const LUNCHBOX_CATEGORY: &str = "lunchbox";

#[derive(Clone, Debug)]
pub struct EnqueueRequest {
    pub game_id: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub torrent_url: String,
    pub torrent_bytes: Vec<u8>,
    pub selected_file_index: u32,
    pub selected_file_path: String,
    pub download_plan: Option<DownloadPlan>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TorrentSnapshot {
    pub state: String,
    pub progress: f64,
    pub download_speed: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct TorrentInfo {
    hash: String,
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
}

#[derive(Debug, Deserialize)]
struct TorrentFileInfo {
    index: u32,
    name: String,
    size: u64,
    progress: f64,
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

    pub fn add(
        &self,
        torrent_bytes: &[u8],
        info_hash: &str,
        save_path: &str,
        selection: &DownloadSelection,
        reviewed_files: &[(u32, String)],
    ) -> Result<Vec<(u32, String)>> {
        let existed = if let Some(existing) = self.torrent_info(info_hash)? {
            ensure_owned(&existing)?;
            true
        } else {
            self.ensure_category()?;
            let torrent_part = Part::bytes(torrent_bytes)
                .file_name("minerva.torrent")
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
                .context("adding the Minerva torrent to qBittorrent")?;
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
            false
        };

        let files = self.apply_selection(info_hash, selection, existed, true)?;
        reviewed_files
            .iter()
            .map(|(index, path)| {
                verified_file(&files, *index, path).map(|file| (file.index, file.name.clone()))
            })
            .collect()
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
            "error" => format!("qBittorrent reported {}", info.state),
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
        })
    }

    pub fn pause(&self, info_hash: &str) -> Result<()> {
        self.control_with_fallback("torrents/stop", "torrents/pause", info_hash)
    }

    pub fn pause_owned(&self, info_hash: &str) -> Result<()> {
        let info = self
            .torrent_info(info_hash)?
            .with_context(|| format!("torrent {info_hash} is no longer in qBittorrent"))?;
        ensure_owned(&info)?;
        self.pause(info_hash)
    }

    pub fn resume(&self, info_hash: &str) -> Result<()> {
        self.start(info_hash)
    }

    pub fn cancel(&self, info_hash: &str) -> Result<()> {
        let response = self
            .agent
            .post(self.endpoint("torrents/delete"))
            .header("Referer", &self.base_url)
            .send_form([("hashes", info_hash), ("deleteFiles", "false")])
            .context("removing torrent from qBittorrent")?;
        let (status, body) = response_text(response)?;
        require_success(status, &body, "qBittorrent cancel request")
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

pub fn test_connection(settings: &AppSettings, password: &str) -> Result<String> {
    QbittorrentClient::authenticated(settings, password)?.version()
}

pub fn enqueue(
    settings: &AppSettings,
    password: &str,
    store: &crate::settings::SettingsStore,
    request: EnqueueRequest,
) -> Result<DownloadJob> {
    settings.validate()?;
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

    let torrent = Torrent::read_from_bytes(&request.torrent_bytes)
        .map_err(|error| anyhow::anyhow!("could not parse selected torrent: {error}"))?;
    let info_hash = torrent.info_hash();
    let existing_jobs = store.jobs_for_info_hash(&info_hash)?;
    let mut download_plan = if settings.download_entire_torrent {
        None
    } else {
        request.download_plan
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
    let requested_files = if let Some(plan) = &download_plan {
        plan.selection()
            .into_iter()
            .map(|(index, path)| {
                Ok((
                    u32::try_from(index).context("torrent file index is too large")?,
                    path,
                ))
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![(
            request.selected_file_index,
            request.selected_file_path.clone(),
        )]
    };
    for job in &existing_jobs {
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
            existing_files.iter().any(|(_, existing_path)| {
                normalized_torrent_path(existing_path) == normalized_torrent_path(requested_path)
            })
        }) {
            bail!("this exact game download is already present in the download history");
        }
    }
    let native_download_path = settings.torrent_library_directory.join("lunchbox");
    std::fs::create_dir_all(&native_download_path).with_context(|| {
        format!(
            "creating native download directory {}",
            native_download_path.display()
        )
    })?;
    let client_save_path = client_path_join(
        &settings.qbittorrent_container_torrent_library_directory,
        "lunchbox",
    );

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
    let local_target_path = if let Some(plan) = &download_plan {
        settings
            .rom_directory
            .join(safe_path_component(&request.platform))
            .join(safe_path_component(&plan.display_name))
            .join(&plan.playlist_filename)
    } else {
        settings
            .rom_directory
            .join(safe_path_component(&request.platform))
            .join(file_name)
    };

    let client = QbittorrentClient::authenticated(settings, password)?;
    let actual_files = client.add(
        &request.torrent_bytes,
        &info_hash,
        &client_save_path,
        &selection,
        &requested_files,
    )?;
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
    let local_source_path =
        native_download_path.join(safe_torrent_relative_path(&actual_file_path)?);
    let serialized_plan = download_plan
        .map(|plan| serde_json::to_string(&plan).context("encoding download plan"))
        .transpose()?
        .unwrap_or_default();

    Ok(DownloadJob::queued(NewDownloadJob {
        game_id: request.game_id,
        launchbox_db_id: request.launchbox_db_id,
        title: request.title,
        platform: request.platform,
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
        "error".to_owned()
    } else if state.contains("pause") || state.contains("stop") {
        "paused".to_owned()
    } else if state.contains("queue") || state.contains("check") || state.contains("meta") {
        "queued".to_owned()
    } else {
        "downloading".to_owned()
    }
}

fn client_path_join(root: &str, child: &str) -> String {
    let root = root.trim_end_matches(['/', '\\']);
    let separator = if root.contains('\\') && !root.contains('/') {
        '\\'
    } else {
        '/'
    };
    format!("{root}{separator}{child}")
}

fn safe_path_component(value: &str) -> String {
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
    } else {
        cleaned.to_owned()
    }
}

fn safe_torrent_relative_path(value: &str) -> Result<PathBuf> {
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::mpsc;

    use super::*;
    use crate::download_plan::{DownloadPlan, DownloadPlanMember};

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
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_http_request(&mut stream);
                request_sender.send(request).unwrap();
                let cookie = if response.cookie {
                    "Set-Cookie: SID=lunchbox-test; Path=/\r\n"
                } else {
                    ""
                };
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n{cookie}Connection: close\r\n\r\n{}",
                    response.body.len(),
                    response.body
                );
                stream.write_all(reply.as_bytes()).unwrap();
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
    fn state_mapping_is_stable_across_qbittorrent_versions() {
        assert_eq!(normalized_state("stoppedDL", 0.2), "paused");
        assert_eq!(normalized_state("pausedDL", 0.2), "paused");
        assert_eq!(normalized_state("queuedDL", 0.2), "queued");
        assert_eq!(normalized_state("error", 0.2), "error");
        assert_eq!(normalized_state("uploading", 1.0), "complete");
    }

    #[test]
    fn unsafe_platform_characters_do_not_become_path_components() {
        assert_eq!(safe_path_component("Sony/PS2: USA"), "Sony_PS2_ USA");
        assert_eq!(safe_path_component("..."), "Unassigned platform");
    }

    #[test]
    fn active_selection_unions_files_from_a_shared_bundle() {
        let mut first = DownloadJob::queued(NewDownloadJob {
            game_id: "game-1".into(),
            launchbox_db_id: 1,
            title: "First".into(),
            platform: "Game Boy".into(),
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
                },
                DownloadPlanMember {
                    index: 4,
                    torrent_path: "Game (Disc 2).chd".into(),
                    target_relative_path: "Game (Disc 2).chd".into(),
                    byte_size: 40,
                    disc_index: Some(2),
                    playlist_entry: true,
                },
            ],
        };
        let mut job = DownloadJob::queued(NewDownloadJob {
            game_id: "game-1".into(),
            launchbox_db_id: 1,
            title: "Game".into(),
            platform: "Platform".into(),
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
