#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, initialized)]
        #[qproperty(bool, busy)]
        #[qproperty(i32, job_count)]
        #[qproperty(i32, active_count)]
        #[qproperty(i32, finished_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, collection_revision)]
        #[qproperty(QString, message)]
        #[qproperty(QString, aggregate_speed)]
        type DownloadQueueModel = super::DownloadQueueModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut DownloadQueueModel>);

        #[qinvokable]
        fn refresh(self: Pin<&mut DownloadQueueModel>);

        #[qinvokable]
        fn pause_job(self: Pin<&mut DownloadQueueModel>, index: i32);

        #[qinvokable]
        fn resume_job(self: Pin<&mut DownloadQueueModel>, index: i32);

        #[qinvokable]
        fn cancel_job(self: Pin<&mut DownloadQueueModel>, index: i32);

        #[qinvokable]
        fn remove_job(self: Pin<&mut DownloadQueueModel>, job_id: QString);

        #[qinvokable]
        fn clear_finished(self: Pin<&mut DownloadQueueModel>);

        #[qinvokable]
        fn job_id_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn job_title_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn job_platform_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn job_state_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn job_detail_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn job_progress_at(self: &DownloadQueueModel, index: i32) -> f64;

        #[qinvokable]
        fn job_can_pause(self: &DownloadQueueModel, index: i32) -> bool;

        #[qinvokable]
        fn job_can_resume(self: &DownloadQueueModel, index: i32) -> bool;

        #[qinvokable]
        fn job_can_cancel(self: &DownloadQueueModel, index: i32) -> bool;

        #[qinvokable]
        fn job_can_remove(self: &DownloadQueueModel, index: i32) -> bool;
    }

    impl cxx_qt::Threading for DownloadQueueModel {}
}

use std::collections::HashSet;
use std::pin::Pin;

use anyhow::{Result, bail};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::ingest;
use crate::qbittorrent::{self, QbittorrentClient};
use crate::settings::{self, DownloadJob, SettingsStore};

pub struct DownloadQueueModelRust {
    initialized: bool,
    busy: bool,
    job_count: i32,
    active_count: i32,
    finished_count: i32,
    revision: i32,
    collection_revision: i32,
    message: QString,
    aggregate_speed: QString,
    jobs: Vec<DownloadJob>,
}

impl Default for DownloadQueueModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            job_count: 0,
            active_count: 0,
            finished_count: 0,
            revision: 0,
            collection_revision: 0,
            message: QString::from("No downloads yet."),
            aggregate_speed: QString::from("0 B/s"),
            jobs: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
enum QueueAction {
    Pause,
    Resume,
    Cancel,
}

struct QueueUpdate {
    jobs: Vec<DownloadJob>,
    imported_count: usize,
    removed_count: usize,
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::DownloadQueueModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() {
            return;
        }
        self.as_mut().set_initialized(true);
        self.as_mut().refresh();
    }

    pub fn refresh(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-download-refresh".into())
            .spawn(move || {
                let refreshed = refresh_jobs().map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_refresh(refreshed);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start download refresh: {error}"
            )));
        }
    }

    fn finish_refresh(mut self: Pin<&mut Self>, refreshed: Result<QueueUpdate, String>) {
        self.as_mut().set_busy(false);
        match refreshed {
            Ok(update) => self.as_mut().replace_jobs(update),
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not refresh downloads: {error}"))),
        }
    }

    pub fn pause_job(self: Pin<&mut Self>, index: i32) {
        self.control_job(index, QueueAction::Pause);
    }

    pub fn resume_job(self: Pin<&mut Self>, index: i32) {
        self.control_job(index, QueueAction::Resume);
    }

    pub fn cancel_job(self: Pin<&mut Self>, index: i32) {
        self.control_job(index, QueueAction::Cancel);
    }

    pub fn remove_job(mut self: Pin<&mut Self>, job_id: QString) {
        if *self.as_ref().busy() {
            return;
        }
        let job_id = job_id.to_string();
        let active = self
            .as_ref()
            .rust()
            .jobs
            .iter()
            .find(|job| job.id == job_id)
            .map(needs_refresh);
        let Some(active) = active else {
            self.as_mut()
                .set_message(qstring("The download record is no longer available."));
            return;
        };
        if active {
            self.as_mut().set_message(qstring(
                "Active downloads, pending imports, and pending post-import actions cannot be removed from history.",
            ));
            return;
        }
        self.as_mut().start_history_cleanup(Some(job_id));
    }

    pub fn clear_finished(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || *self.as_ref().finished_count() == 0 {
            return;
        }
        self.as_mut().start_history_cleanup(None);
    }

    fn start_history_cleanup(mut self: Pin<&mut Self>, job_id: Option<String>) {
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(if job_id.is_some() {
            "Removing the finished download record…"
        } else {
            "Clearing finished download history…"
        }));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-download-history".into())
            .spawn(move || {
                let updated = cleanup_history(job_id).map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_refresh(updated);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start history cleanup: {error}")));
        }
    }

    fn control_job(mut self: Pin<&mut Self>, index: i32, action: QueueAction) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(job) = self.as_ref().job(index).cloned() else {
            self.as_mut()
                .set_message(qstring("The selected download is no longer available."));
            return;
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(match action {
            QueueAction::Pause => "Pausing download…",
            QueueAction::Resume => "Resuming download…",
            QueueAction::Cancel => "Cancelling download…",
        }));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-download-control".into())
            .spawn(move || {
                let controlled = control_job(job, action).map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_refresh(controlled);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start download control: {error}"
            )));
        }
    }

    fn replace_jobs(mut self: Pin<&mut Self>, update: QueueUpdate) {
        let jobs = update.jobs;
        let active = jobs.iter().filter(|job| needs_refresh(job)).count();
        let finished = jobs.len().saturating_sub(active);
        let aggregate_speed = jobs
            .iter()
            .filter(|job| job.state == "downloading")
            .fold(0_u64, |total, job| total.saturating_add(job.download_speed));
        let count = jobs.len();
        self.as_mut().rust_mut().jobs = jobs;
        self.as_mut()
            .set_job_count(i32::try_from(count).unwrap_or(i32::MAX));
        self.as_mut()
            .set_active_count(i32::try_from(active).unwrap_or(i32::MAX));
        self.as_mut()
            .set_finished_count(i32::try_from(finished).unwrap_or(i32::MAX));
        self.as_mut()
            .set_aggregate_speed(qstring(format_download_rate(aggregate_speed)));
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
        if update.imported_count > 0 {
            let collection_revision = self.as_ref().collection_revision().wrapping_add(1);
            self.as_mut().set_collection_revision(collection_revision);
        }
        self.as_mut()
            .set_message(qstring(if update.removed_count > 0 {
                format!(
                    "Removed {} finished download record{}; {count} remaining",
                    update.removed_count,
                    if update.removed_count == 1 { "" } else { "s" }
                )
            } else if count == 0 {
                "No downloads yet.".to_owned()
            } else if active == 0 {
                format!("{count} download records; none active")
            } else if aggregate_speed > 0 {
                format!(
                    "{active} active of {count} downloads · {}",
                    format_download_rate(aggregate_speed)
                )
            } else {
                format!("{active} active of {count} downloads")
            }));
    }

    pub fn job_id_at(&self, index: i32) -> QString {
        self.job(index)
            .map(|job| qstring(&job.id))
            .unwrap_or_default()
    }

    pub fn job_title_at(&self, index: i32) -> QString {
        self.job(index)
            .map(|job| qstring(&job.title))
            .unwrap_or_default()
    }

    pub fn job_platform_at(&self, index: i32) -> QString {
        self.job(index)
            .map(|job| qstring(&job.platform))
            .unwrap_or_default()
    }

    pub fn job_state_at(&self, index: i32) -> QString {
        self.job(index)
            .map(|job| qstring(job.state.to_uppercase()))
            .unwrap_or_default()
    }

    pub fn job_detail_at(&self, index: i32) -> QString {
        self.job(index)
            .map(|job| {
                let progress = (job.progress.clamp(0.0, 1.0) * 100.0).round();
                let bytes = if job.total_bytes > 0 {
                    format!(
                        "{} of {}",
                        crate::game_details::format_bytes(job.downloaded_bytes),
                        crate::game_details::format_bytes(job.total_bytes)
                    )
                } else {
                    "size pending".to_owned()
                };
                let rate = if job.state == "downloading" && job.download_speed > 0 {
                    format!(" · {}", format_download_rate(job.download_speed))
                } else {
                    String::new()
                };
                let plan = job
                    .parsed_download_plan()
                    .ok()
                    .flatten()
                    .map(|plan| {
                        if plan.is_optical_multidisc() {
                            format!("{}-disc set · ", plan.disc_count())
                        } else {
                            format!("{}-archive eXo set · ", plan.members.len())
                        }
                    })
                    .unwrap_or_default();
                qstring(format!(
                    "{plan}{progress:.0}% · {bytes}{rate} · {}",
                    job.message
                ))
            })
            .unwrap_or_default()
    }

    pub fn job_progress_at(&self, index: i32) -> f64 {
        self.job(index)
            .map(|job| job.progress.clamp(0.0, 1.0))
            .unwrap_or_default()
    }

    pub fn job_can_pause(&self, index: i32) -> bool {
        self.job(index)
            .is_some_and(|job| matches!(job.state.as_str(), "queued" | "downloading"))
    }

    pub fn job_can_resume(&self, index: i32) -> bool {
        self.job(index).is_some_and(|job| job.state == "paused")
    }

    pub fn job_can_cancel(&self, index: i32) -> bool {
        self.job(index).is_some_and(|job| is_active(&job.state))
    }

    pub fn job_can_remove(&self, index: i32) -> bool {
        self.job(index).is_some_and(|job| !needs_refresh(job))
    }

    fn job(&self, index: i32) -> Option<&DownloadJob> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().jobs.get(index))
    }
}

fn refresh_jobs() -> Result<QueueUpdate> {
    let store = SettingsStore::open_default()?;
    let settings = store.load()?;
    let mut jobs = store.jobs()?;
    if !jobs.iter().any(needs_refresh) {
        return Ok(QueueUpdate {
            jobs,
            imported_count: 0,
            removed_count: 0,
        });
    }
    let mut imported_count = 0;
    let password = settings::load_password()?.unwrap_or_default();
    let client = QbittorrentClient::authenticated(&settings, &password)?;
    for job in jobs.iter_mut().filter(|job| is_active(&job.state)) {
        let snapshot = qbittorrent::job_exact_files(job)
            .and_then(|selection| client.snapshot(&job.info_hash, selection.as_deref()));
        match snapshot {
            Ok(snapshot) => {
                job.state = snapshot.state;
                job.progress = snapshot.progress;
                job.download_speed = snapshot.download_speed;
                job.downloaded_bytes = snapshot.downloaded_bytes;
                job.total_bytes = snapshot.total_bytes;
                job.message = snapshot.message;
                job.updated_at = settings::unix_timestamp();
                if job.state == "complete" {
                    match ingest::ingest_completed(&settings, &store, job) {
                        Ok(path) => {
                            job.state = "imported".to_owned();
                            job.message = format!("Ready in your library at {}", path.display());
                            job.post_import_action =
                                if settings.seeding_policy == "pause_after_import" {
                                    "pause_pending".to_owned()
                                } else {
                                    "none".to_owned()
                                };
                            imported_count += 1;
                        }
                        Err(error) => {
                            job.message = format!(
                                "Download complete; import will retry automatically: {error}"
                            );
                        }
                    }
                }
            }
            Err(error) => {
                job.message = format!("Could not refresh qBittorrent: {error}");
                job.updated_at = settings::unix_timestamp();
            }
        }
        store.upsert_job(job)?;
    }
    apply_pending_post_import_actions(&client, &store, &mut jobs)?;
    Ok(QueueUpdate {
        jobs,
        imported_count,
        removed_count: 0,
    })
}

fn apply_pending_post_import_actions(
    client: &QbittorrentClient,
    store: &SettingsStore,
    jobs: &mut [DownloadJob],
) -> Result<()> {
    for info_hash in ready_pause_hashes(jobs) {
        let pause_result = client.pause_owned(&info_hash);
        for job in jobs.iter_mut().filter(|job| {
            job.info_hash.eq_ignore_ascii_case(&info_hash)
                && job.post_import_action == "pause_pending"
        }) {
            let ready_message = job
                .message
                .split(" · ")
                .next()
                .unwrap_or("Ready in your library");
            match &pause_result {
                Ok(()) => {
                    job.post_import_action = "pause_applied".to_owned();
                    job.message = format!("{ready_message} · Torrent paused after import");
                }
                Err(error) => {
                    job.message = format!(
                        "{ready_message} · Could not pause the torrent; will retry automatically: {error}"
                    );
                }
            }
            job.updated_at = settings::unix_timestamp();
            store.upsert_job(job)?;
        }
    }
    Ok(())
}

fn ready_pause_hashes(jobs: &[DownloadJob]) -> HashSet<String> {
    jobs.iter()
        .filter(|job| job.post_import_action == "pause_pending")
        .map(|job| job.info_hash.to_ascii_lowercase())
        .filter(|info_hash| {
            !jobs
                .iter()
                .any(|job| job.info_hash.eq_ignore_ascii_case(info_hash) && is_active(&job.state))
        })
        .collect()
}

fn control_job(mut job: DownloadJob, action: QueueAction) -> Result<QueueUpdate> {
    let store = SettingsStore::open_default()?;
    let settings = store.load()?;
    let password = settings::load_password()?.unwrap_or_default();
    let client = QbittorrentClient::authenticated(&settings, &password)?;
    let mut related_jobs = store.jobs_for_info_hash(&job.info_hash)?;
    match action {
        QueueAction::Pause => {
            client.pause_owned(&job.info_hash)?;
            for related in &mut related_jobs {
                if !matches!(related.state.as_str(), "queued" | "downloading" | "paused") {
                    continue;
                }
                related.state = "paused".to_owned();
                related.message = "Paused in qBittorrent".to_owned();
                related.updated_at = settings::unix_timestamp();
                store.upsert_job(related)?;
            }
        }
        QueueAction::Resume => {
            client.resume_owned(&job.info_hash)?;
            for related in &mut related_jobs {
                if !matches!(related.state.as_str(), "queued" | "downloading" | "paused") {
                    continue;
                }
                related.state = "queued".to_owned();
                related.message = "Resumed in qBittorrent".to_owned();
                related.updated_at = settings::unix_timestamp();
                store.upsert_job(related)?;
            }
        }
        QueueAction::Cancel => {
            let remaining = related_jobs
                .iter()
                .filter(|related| related.id != job.id && is_active(&related.state))
                .cloned()
                .collect::<Vec<_>>();
            if remaining.is_empty() {
                client.cancel_owned(&job.info_hash)?;
                job.message = "Cancelled; torrent removed and files preserved".to_owned();
            } else {
                let selection = qbittorrent::active_selection(&remaining, false, &[])?;
                let start_after = remaining.iter().any(|job| job.state != "paused");
                client.reconfigure(&job.info_hash, &selection, start_after)?;
                job.message =
                    "Cancelled this file; other downloads from the bundle continue".to_owned();
            }
            job.state = "cancelled".to_owned();
            job.updated_at = settings::unix_timestamp();
            store.upsert_job(&job)?;
        }
    }
    Ok(QueueUpdate {
        jobs: store.jobs()?,
        imported_count: 0,
        removed_count: 0,
    })
}

fn cleanup_history(job_id: Option<String>) -> Result<QueueUpdate> {
    let store = SettingsStore::open_default()?;
    let removed_count = if let Some(job_id) = job_id {
        if store.delete_job_record(&job_id)? {
            1
        } else {
            bail!("the record is active, pending import, or no longer exists");
        }
    } else {
        store.clear_finished_job_records()?
    };
    Ok(QueueUpdate {
        jobs: store.jobs()?,
        imported_count: 0,
        removed_count,
    })
}

fn format_download_rate(bytes_per_second: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let rate = bytes_per_second as f64;
    if rate >= GIB {
        format!("{:.1} GiB/s", rate / GIB)
    } else if rate >= MIB {
        format!("{:.1} MiB/s", rate / MIB)
    } else if rate >= KIB {
        format!("{:.1} KiB/s", rate / KIB)
    } else {
        format!("{bytes_per_second} B/s")
    }
}

fn is_active(state: &str) -> bool {
    matches!(state, "queued" | "downloading" | "paused" | "complete")
}

fn needs_refresh(job: &DownloadJob) -> bool {
    is_active(&job.state) || job.post_import_action == "pause_pending"
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use super::{format_download_rate, is_active, needs_refresh, ready_pause_hashes};
    use crate::settings::{DownloadJob, NewDownloadJob};

    fn job(game_id: &str, info_hash: &str, state: &str, action: &str) -> DownloadJob {
        let mut job = DownloadJob::queued(NewDownloadJob {
            game_id: game_id.into(),
            launchbox_db_id: 1,
            title: game_id.into(),
            platform: "Platform".into(),
            source_kind: "minerva".into(),
            torrent_url: "https://example.invalid/bundle.torrent".into(),
            torrent_file_index: Some(0),
            torrent_file_path: format!("{game_id}.zip"),
            info_hash: info_hash.into(),
            client_save_path: "/downloads".into(),
            local_download_path: PathBuf::from(format!("/downloads/{game_id}.zip")),
            local_target_path: PathBuf::from(format!("/roms/{game_id}.zip")),
            download_plan: String::new(),
        });
        job.state = state.into();
        job.post_import_action = action.into();
        job
    }

    #[test]
    fn download_rates_use_binary_units_and_active_states_protect_pending_work() {
        assert_eq!(format_download_rate(0), "0 B/s");
        assert_eq!(format_download_rate(1536), "1.5 KiB/s");
        assert_eq!(format_download_rate(5 * 1024 * 1024), "5.0 MiB/s");
        for state in ["queued", "downloading", "paused", "complete"] {
            assert!(is_active(state));
        }
        for state in ["imported", "cancelled", "failed"] {
            assert!(!is_active(state));
        }
    }

    #[test]
    fn durable_pause_waits_for_every_active_bundle_member() {
        let mut imported = job("imported", "ABC123", "imported", "pause_pending");
        let downloading = job("downloading", "abc123", "downloading", "none");
        assert!(needs_refresh(&imported));
        assert!(ready_pause_hashes(&[imported.clone(), downloading]).is_empty());

        imported.post_import_action = "pause_pending".into();
        let sibling = job("sibling", "abc123", "imported", "pause_pending");
        assert_eq!(
            ready_pause_hashes(&[imported, sibling]),
            HashSet::from(["abc123".to_owned()])
        );
    }
}
