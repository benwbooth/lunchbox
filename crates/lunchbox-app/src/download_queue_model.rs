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
        #[qproperty(i32, revision)]
        #[qproperty(i32, collection_revision)]
        #[qproperty(QString, message)]
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
    }

    impl cxx_qt::Threading for DownloadQueueModel {}
}

use std::pin::Pin;

use anyhow::Result;
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
    revision: i32,
    collection_revision: i32,
    message: QString,
    jobs: Vec<DownloadJob>,
}

impl Default for DownloadQueueModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            job_count: 0,
            active_count: 0,
            revision: 0,
            collection_revision: 0,
            message: QString::from("No downloads yet."),
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
        let active = jobs.iter().filter(|job| is_active(&job.state)).count();
        let count = jobs.len();
        self.as_mut().rust_mut().jobs = jobs;
        self.as_mut()
            .set_job_count(i32::try_from(count).unwrap_or(i32::MAX));
        self.as_mut()
            .set_active_count(i32::try_from(active).unwrap_or(i32::MAX));
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
        if update.imported_count > 0 {
            let collection_revision = self.as_ref().collection_revision().wrapping_add(1);
            self.as_mut().set_collection_revision(collection_revision);
        }
        self.as_mut().set_message(qstring(if count == 0 {
            "No downloads yet.".to_owned()
        } else if active == 0 {
            format!("{count} download records; none active")
        } else {
            format!("{active} active of {count} downloads")
        }));
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
                qstring(format!("{progress:.0}% · {bytes} · {}", job.message))
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
    if !jobs.iter().any(|job| is_active(&job.state)) {
        return Ok(QueueUpdate {
            jobs,
            imported_count: 0,
        });
    }
    let mut imported_count = 0;
    let password = settings::load_password()?.unwrap_or_default();
    let client = QbittorrentClient::authenticated(&settings, &password)?;
    for job in jobs.iter_mut().filter(|job| is_active(&job.state)) {
        match client.snapshot(&job.info_hash, job.torrent_file_index) {
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
    Ok(QueueUpdate {
        jobs,
        imported_count,
    })
}

fn control_job(mut job: DownloadJob, action: QueueAction) -> Result<QueueUpdate> {
    let store = SettingsStore::open_default()?;
    let settings = store.load()?;
    let password = settings::load_password()?.unwrap_or_default();
    let client = QbittorrentClient::authenticated(&settings, &password)?;
    let mut related_jobs = store.jobs_for_info_hash(&job.info_hash)?;
    match action {
        QueueAction::Pause => {
            client.pause(&job.info_hash)?;
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
            client.resume(&job.info_hash)?;
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
                client.cancel(&job.info_hash)?;
                job.message = "Cancelled; torrent removed and files preserved".to_owned();
            } else {
                let selection = qbittorrent::active_selection(&remaining, false, None)?;
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
    })
}

fn is_active(state: &str) -> bool {
    matches!(state, "queued" | "downloading" | "paused" | "complete")
}
