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
        #[qproperty(bool, refreshing)]
        #[qproperty(i32, job_count)]
        #[qproperty(i32, active_count)]
        #[qproperty(i32, finished_count)]
        #[qproperty(i32, failed_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, collection_revision)]
        #[qproperty(QString, message)]
        #[qproperty(QString, aggregate_speed)]
        #[qproperty(bool, history_busy)]
        #[qproperty(QString, history_title)]
        #[qproperty(QString, history_message)]
        #[qproperty(i32, history_count)]
        #[qproperty(i32, history_revision)]
        #[qproperty(bool, history_retryable)]
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
        fn retry_job(self: Pin<&mut DownloadQueueModel>, job_id: QString);

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

        #[qinvokable]
        fn job_can_retry(self: &DownloadQueueModel, index: i32) -> bool;

        #[qinvokable]
        fn job_can_import(self: &DownloadQueueModel, index: i32) -> bool;

        #[qinvokable]
        fn job_import_directory_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn job_index_for_game(self: &DownloadQueueModel, game_id: QString) -> i32;

        #[qinvokable]
        fn job_badge_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn load_history(self: Pin<&mut DownloadQueueModel>, job_id: QString);

        #[qinvokable]
        fn history_kind_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn history_label_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn history_detail_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn history_epoch_at(self: &DownloadQueueModel, index: i32) -> QString;

        #[qinvokable]
        fn report_recovery_ui_probe(self: &DownloadQueueModel);
    }

    impl cxx_qt::Threading for DownloadQueueModel {}
}

use std::collections::HashSet;
use std::pin::Pin;

use anyhow::{Context, Result, bail};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::ingest;
use crate::qbittorrent::{self, QbittorrentClient};
use crate::settings::{self, DownloadJob, DownloadJobEvent, NewDownloadJob, SettingsStore};

pub struct DownloadQueueModelRust {
    initialized: bool,
    busy: bool,
    refreshing: bool,
    job_count: i32,
    active_count: i32,
    finished_count: i32,
    failed_count: i32,
    revision: i32,
    collection_revision: i32,
    message: QString,
    aggregate_speed: QString,
    history_busy: bool,
    history_title: QString,
    history_message: QString,
    history_count: i32,
    history_revision: i32,
    history_retryable: bool,
    jobs: Vec<DownloadJob>,
    history_events: Vec<DownloadJobEvent>,
    history_generation: u64,
    refresh_generation: u64,
    recovery_ui_probe: bool,
}

impl Default for DownloadQueueModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            refreshing: false,
            job_count: 0,
            active_count: 0,
            finished_count: 0,
            failed_count: 0,
            revision: 0,
            collection_revision: 0,
            message: QString::from("No downloads yet."),
            aggregate_speed: QString::from("0 B/s"),
            history_busy: false,
            history_title: QString::default(),
            history_message: QString::from("Choose a download to inspect its recovery history."),
            history_count: 0,
            history_revision: 0,
            history_retryable: false,
            jobs: Vec::new(),
            history_events: Vec::new(),
            history_generation: 0,
            refresh_generation: 0,
            recovery_ui_probe: std::env::args()
                .any(|argument| argument == "--download-recovery-ui-probe"),
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
    notice: Option<String>,
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
        if *self.as_ref().busy() || *self.as_ref().refreshing() {
            return;
        }
        self.as_mut().rust_mut().refresh_generation =
            self.as_ref().rust().refresh_generation.wrapping_add(1);
        let generation = self.as_ref().rust().refresh_generation;
        self.as_mut().set_refreshing(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-download-refresh".into())
            .spawn(move || {
                let refreshed = refresh_jobs().map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_poll_refresh(generation, refreshed);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_refreshing(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start download refresh: {error}"
            )));
        }
    }

    fn finish_poll_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        refreshed: Result<QueueUpdate, String>,
    ) {
        if generation != self.as_ref().rust().refresh_generation {
            return;
        }
        self.as_mut().set_refreshing(false);
        match refreshed {
            Ok(update) => self.as_mut().replace_jobs(update),
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not refresh downloads: {error}"))),
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

    pub fn retry_job(mut self: Pin<&mut Self>, job_id: QString) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().invalidate_poll_refresh();
        let job_id = job_id.to_string();
        let Some(job) = self
            .as_ref()
            .rust()
            .jobs
            .iter()
            .find(|job| job.id == job_id)
            .cloned()
        else {
            self.as_mut()
                .set_message(qstring("The failed download is no longer available."));
            return;
        };
        if !can_retry(&job) {
            self.as_mut().set_message(qstring(
                "Only failed Lunchbox-owned downloads can enter recovery.",
            ));
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(format!(
            "Verifying and retrying {} without changing its reviewed files…",
            job.title
        )));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-download-recovery".into())
            .spawn(move || {
                let recovered = recover_failed_job(job).map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_refresh(recovered);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start download recovery: {error}"
            )));
        }
    }

    pub fn remove_job(mut self: Pin<&mut Self>, job_id: QString) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().invalidate_poll_refresh();
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
        self.as_mut().invalidate_poll_refresh();
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
        self.as_mut().invalidate_poll_refresh();
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
        let notice = update.notice.clone();
        let jobs = update.jobs;
        let active = jobs.iter().filter(|job| needs_refresh(job)).count();
        let finished = jobs.len().saturating_sub(active);
        let failed = jobs.iter().filter(|job| job.state == "failed").count();
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
            .set_failed_count(i32::try_from(failed).unwrap_or(i32::MAX));
        self.as_mut()
            .set_aggregate_speed(qstring(format_download_rate(aggregate_speed)));
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
        if update.imported_count > 0 {
            let collection_revision = self.as_ref().collection_revision().wrapping_add(1);
            self.as_mut().set_collection_revision(collection_revision);
        }
        self.as_mut()
            .set_message(qstring(if let Some(notice) = notice {
                notice
            } else if update.removed_count > 0 {
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

    fn invalidate_poll_refresh(mut self: Pin<&mut Self>) {
        if !*self.as_ref().refreshing() {
            return;
        }
        self.as_mut().rust_mut().refresh_generation =
            self.as_ref().rust().refresh_generation.wrapping_add(1);
        self.as_mut().set_refreshing(false);
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

    pub fn job_can_retry(&self, index: i32) -> bool {
        self.job(index).is_some_and(can_retry)
    }

    pub fn job_can_import(&self, index: i32) -> bool {
        self.job(index).is_some_and(|job| {
            qbittorrent::is_collection_import_job(job) && job.state == "imported"
        })
    }

    pub fn job_import_directory_at(&self, index: i32) -> QString {
        self.job(index)
            .filter(|job| qbittorrent::is_collection_import_job(job))
            .map(|job| qstring(job.local_target_path.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn job_index_for_game(&self, game_id: QString) -> i32 {
        relevant_job_index(&self.rust().jobs, &game_id.to_string())
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    pub fn job_badge_at(&self, index: i32) -> QString {
        self.job(index)
            .map(|job| qstring(job_badge(&job.state)))
            .unwrap_or_default()
    }

    pub fn load_history(mut self: Pin<&mut Self>, job_id: QString) {
        let job_id = job_id.to_string();
        let Some(job) = self
            .as_ref()
            .rust()
            .jobs
            .iter()
            .find(|job| job.id == job_id)
            .cloned()
        else {
            self.as_mut().rust_mut().history_events.clear();
            self.as_mut().set_history_count(0);
            self.as_mut()
                .set_history_message(qstring("The download record is no longer available."));
            return;
        };
        self.as_mut().rust_mut().history_generation =
            self.as_ref().rust().history_generation.wrapping_add(1);
        let generation = self.as_ref().rust().history_generation;
        self.as_mut().set_history_busy(true);
        self.as_mut().set_history_title(qstring(&job.title));
        self.as_mut().set_history_retryable(can_retry(&job));
        self.as_mut().set_history_message(qstring(
            "Loading durable state changes and recovery attempts…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-download-history-load".into())
            .spawn(move || {
                let events = SettingsStore::open_default()
                    .and_then(|store| store.download_job_events(&job.id, 128))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_history(generation, events);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_history_busy(false);
            self.as_mut()
                .set_history_message(qstring(format!("Could not load download history: {error}")));
        }
    }

    fn finish_history(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<Vec<DownloadJobEvent>, String>,
    ) {
        if generation != self.as_ref().rust().history_generation {
            return;
        }
        self.as_mut().set_history_busy(false);
        match result {
            Ok(events) => {
                let count = events.len();
                self.as_mut().rust_mut().history_events = events;
                self.as_mut()
                    .set_history_count(i32::try_from(count).unwrap_or(i32::MAX));
                self.as_mut().set_history_message(qstring(if count == 0 {
                    "No durable events predate this version; future state changes and recovery attempts will appear here.".to_owned()
                } else {
                    format!(
                        "{count} durable event{} · newest first · retained until this history record is removed",
                        if count == 1 { "" } else { "s" }
                    )
                }));
                let revision = self.as_ref().history_revision().wrapping_add(1);
                self.as_mut().set_history_revision(revision);
            }
            Err(error) => self
                .as_mut()
                .set_history_message(qstring(format!("Could not load download history: {error}"))),
        }
    }

    pub fn history_kind_at(&self, index: i32) -> QString {
        self.history_event(index)
            .map(|event| qstring(&event.kind))
            .unwrap_or_default()
    }

    pub fn history_label_at(&self, index: i32) -> QString {
        self.history_event(index)
            .map(|event| qstring(event_label(&event.kind)))
            .unwrap_or_default()
    }

    pub fn history_detail_at(&self, index: i32) -> QString {
        self.history_event(index)
            .map(|event| qstring(&event.message))
            .unwrap_or_default()
    }

    pub fn history_epoch_at(&self, index: i32) -> QString {
        self.history_event(index)
            .map(|event| qstring(event.created_at.to_string()))
            .unwrap_or_default()
    }

    pub fn report_recovery_ui_probe(&self) {
        if self.rust().recovery_ui_probe {
            println!(
                "LUNCHBOX_DOWNLOAD_RECOVERY_UI_READY failed={} history={} retryable={}",
                self.failed_count(),
                self.history_count(),
                self.job_can_retry(0)
            );
        }
    }

    fn job(&self, index: i32) -> Option<&DownloadJob> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().jobs.get(index))
    }

    fn history_event(&self, index: i32) -> Option<&DownloadJobEvent> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().history_events.get(index))
    }
}

fn refresh_jobs() -> Result<QueueUpdate> {
    let store = SettingsStore::open_default()?;
    if std::env::args().any(|argument| argument == "--download-recovery-ui-probe") {
        seed_download_recovery_probe(&store)?;
    }
    let settings = store.load()?;
    let mut jobs = store.jobs()?;
    if !jobs.iter().any(needs_refresh) {
        return Ok(QueueUpdate {
            jobs,
            imported_count: 0,
            removed_count: 0,
            notice: None,
        });
    }
    let mut imported_count = 0;
    let password = settings::load_password()?.unwrap_or_default();
    let client = QbittorrentClient::authenticated(&settings, &password)?;
    for job in jobs.iter_mut().filter(|job| is_active(&job.state)) {
        let mut durable_event = None;
        let snapshot = qbittorrent::job_exact_files(job)
            .and_then(|selection| client.snapshot(&job.info_hash, selection.as_deref()));
        match snapshot {
            Ok(snapshot) => {
                if !snapshot.client_save_path.trim().is_empty()
                    && snapshot.client_save_path != job.client_save_path
                {
                    job.client_save_path = snapshot.client_save_path.clone();
                    if qbittorrent::is_collection_import_job(job) {
                        let destination = qbittorrent::native_path_for_client_save_path(
                            &settings.torrent_library_directory,
                            &settings.qbittorrent_container_torrent_library_directory,
                            &job.client_save_path,
                        )?;
                        job.local_download_path = destination.clone();
                        job.local_target_path = destination;
                    } else {
                        job.local_download_path = qbittorrent::native_path_for_client_file(
                            &settings.torrent_library_directory,
                            &settings.qbittorrent_container_torrent_library_directory,
                            &job.client_save_path,
                            &job.torrent_file_path,
                        )?;
                    }
                }
                job.state = snapshot.state;
                job.progress = snapshot.progress;
                job.download_speed = snapshot.download_speed;
                job.downloaded_bytes = snapshot.downloaded_bytes;
                job.total_bytes = snapshot.total_bytes;
                job.message = snapshot.message;
                job.updated_at = settings::unix_timestamp();
                if job.state == "complete" {
                    if qbittorrent::is_collection_import_job(job) {
                        if job.local_target_path.is_dir() {
                            job.state = "imported".to_owned();
                            job.message = format!(
                                "Downloaded collection ready for review at {}",
                                job.local_target_path.display()
                            );
                            job.post_import_action =
                                if settings.seeding_policy == "pause_after_import" {
                                    "pause_pending".to_owned()
                                } else {
                                    "none".to_owned()
                                };
                            imported_count += 1;
                        } else {
                            job.message = format!(
                                "Download complete; waiting for the mapped collection folder at {}",
                                job.local_target_path.display()
                            );
                            durable_event = Some(("import_error", job.message.clone()));
                        }
                    } else {
                        match ingest::ingest_completed(&settings, &store, job) {
                            Ok(path) => {
                                job.state = "imported".to_owned();
                                job.message =
                                    format!("Ready in your library at {}", path.display());
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
                                durable_event = Some(("import_error", job.message.clone()));
                            }
                        }
                    }
                }
            }
            Err(error) => {
                if error.to_string().contains("is no longer in qBittorrent") {
                    match recover_managed_torrent(&store, job, &client) {
                        Ok(recreated) => {
                            job.state = "queued".to_owned();
                            job.download_speed = 0;
                            job.message = if recreated {
                                "Lunchbox restored the missing torrent from retained metadata; qBittorrent is rechecking the existing payload"
                                    .to_owned()
                            } else {
                                "Lunchbox restored the reviewed selection; qBittorrent is rechecking the existing payload"
                                    .to_owned()
                            };
                            durable_event = Some(("retry", job.message.clone()));
                        }
                        Err(recovery_error) => {
                            job.state = "failed".to_owned();
                            job.download_speed = 0;
                            job.message = recovery_failure_message(&recovery_error.to_string());
                        }
                    }
                } else {
                    job.message = format!("Could not refresh qBittorrent: {error}");
                }
                job.updated_at = settings::unix_timestamp();
            }
        }
        store.upsert_job(job)?;
        if let Some((kind, message)) = durable_event {
            store.record_download_job_event(&job.id, kind, &message)?;
        }
    }
    apply_pending_post_import_actions(&client, &store, &mut jobs)?;
    Ok(QueueUpdate {
        jobs,
        imported_count,
        removed_count: 0,
        notice: None,
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
            store.record_download_job_event(
                &job.id,
                if pause_result.is_ok() {
                    "post_import_applied"
                } else {
                    "post_import_error"
                },
                &job.message,
            )?;
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
        notice: None,
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
        notice: None,
    })
}

fn recover_failed_job(job: DownloadJob) -> Result<QueueUpdate> {
    let store = SettingsStore::open_default()?;
    let Some(mut current) = store
        .jobs()?
        .into_iter()
        .find(|candidate| candidate.id == job.id)
    else {
        bail!("the failed download record is no longer available");
    };
    if !can_retry(&current) {
        bail!("the download is no longer in a failed state");
    }
    let settings = store.load()?;
    let recovery = settings::load_password()
        .and_then(|password| {
            QbittorrentClient::authenticated(&settings, password.as_deref().unwrap_or_default())
        })
        .and_then(|client| recover_managed_torrent(&store, &current, &client));
    match recovery {
        Ok(recreated) => {
            let now = settings::unix_timestamp();
            let mut recovered = 0_usize;
            for related in store.jobs_for_info_hash(&current.info_hash)? {
                if !can_retry(&related) {
                    continue;
                }
                store.record_download_job_event(
                    &related.id,
                    "retry",
                    if recreated {
                        "Recovery recreated the missing Lunchbox-owned torrent from retained metadata, restored the exact reviewed file selection, and requested a data recheck."
                    } else {
                        "Recovery verified Lunchbox ownership, restored the exact reviewed file selection, and requested a data recheck."
                    },
                )?;
                let mut related = related;
                related.state = "queued".to_owned();
                related.message = if recreated {
                    "Lunchbox restored the missing torrent; qBittorrent is rechecking the existing payload"
                        .to_owned()
                } else {
                    "Recovery requested; qBittorrent is rechecking the existing payload".to_owned()
                };
                related.updated_at = now;
                store.upsert_job(&related)?;
                recovered = recovered.saturating_add(1);
            }
            Ok(QueueUpdate {
                jobs: store.jobs()?,
                imported_count: 0,
                removed_count: 0,
                notice: Some(format!(
                    "Recovery {} for {recovered} exact download record{}; reviewed file priorities and existing payloads were preserved.",
                    if recreated {
                        "recreated the missing torrent"
                    } else {
                        "started"
                    },
                    if recovered == 1 { "" } else { "s" }
                )),
            })
        }
        Err(error) => {
            current.message = recovery_failure_message(&error.to_string());
            current.updated_at = settings::unix_timestamp();
            store.upsert_job(&current)?;
            store.record_download_job_event(&current.id, "failed", &current.message)?;
            Ok(QueueUpdate {
                jobs: store.jobs()?,
                imported_count: 0,
                removed_count: 0,
                notice: Some(current.message),
            })
        }
    }
}

fn recover_managed_torrent(
    store: &SettingsStore,
    current: &DownloadJob,
    client: &QbittorrentClient,
) -> Result<bool> {
    let related = store.jobs_for_info_hash(&current.info_hash)?;
    let recoverable = related
        .iter()
        .filter(|job| is_active(&job.state) || job.state == "failed")
        .collect::<Vec<_>>();
    if recoverable.is_empty() {
        bail!("no active or failed Lunchbox download records remain for this torrent");
    }
    let client_save_path = recoverable
        .iter()
        .map(|job| job.client_save_path.trim())
        .find(|path| !path.is_empty())
        .context("the download record has no persisted qBittorrent save path")?;
    if recoverable.iter().any(|job| {
        let candidate = job.client_save_path.trim();
        !candidate.is_empty() && candidate != client_save_path
    }) {
        bail!("related download records disagree about the qBittorrent save path");
    }

    let mut whole_torrent = false;
    let mut reviewed_files = Vec::new();
    for job in recoverable {
        match qbittorrent::job_exact_files(job)? {
            Some(files) => reviewed_files.extend(files),
            None => whole_torrent = true,
        }
    }
    reviewed_files.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    reviewed_files.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);
    let selection = if whole_torrent {
        qbittorrent::DownloadSelection::All
    } else if reviewed_files.is_empty() {
        bail!("recovery has no exact reviewed torrent files");
    } else {
        qbittorrent::DownloadSelection::Exact(reviewed_files.clone())
    };
    let torrent_bytes = retained_or_recovered_torrent_bytes(store, current)?;
    client.recover_or_readd_owned(
        &torrent_bytes,
        &current.info_hash,
        client_save_path,
        &selection,
        &reviewed_files,
    )
}

fn retained_or_recovered_torrent_bytes(
    store: &SettingsStore,
    job: &DownloadJob,
) -> Result<Vec<u8>> {
    if let Some(bytes) = store.retained_torrent_bytes(&job.info_hash)? {
        return Ok(bytes);
    }
    let can_recover_source =
        job.source_kind == "minerva" || job.torrent_url.starts_with("registered-torrent:");
    if !can_recover_source {
        bail!(
            "this older download record predates retained torrent metadata; review its exact source once to bring it under Lunchbox lifecycle management"
        );
    }
    let bytes = crate::game_details::torrent_bytes_for_url(&job.torrent_url)
        .context("recovering exact torrent metadata from its recorded source")?;
    let retained = store.retain_torrent_metadata(&job.source_kind, &job.torrent_url, &bytes)?;
    if !retained.eq_ignore_ascii_case(&job.info_hash) {
        bail!("the recorded source now returns different torrent metadata");
    }
    Ok(bytes)
}

fn recovery_failure_message(error: &str) -> String {
    format!(
        "Recovery could not start: {error}. The reviewed selection, downloaded files, and target files were left untouched."
    )
}

fn can_retry(job: &DownloadJob) -> bool {
    job.state == "failed" && job.post_import_action != "pause_pending"
}

fn relevant_job_index(jobs: &[DownloadJob], game_id: &str) -> Option<usize> {
    jobs.iter()
        .position(|job| job.game_id == game_id && job.state != "cancelled")
}

fn job_badge(state: &str) -> &'static str {
    match state {
        "queued" => "QUEUED",
        "downloading" => "DOWNLOADING",
        "paused" => "PAUSED",
        "complete" => "INSTALLING",
        "imported" => "READY",
        "failed" => "ATTENTION",
        _ => "DOWNLOAD",
    }
}

fn event_label(kind: &str) -> &'static str {
    match kind {
        "queued" => "QUEUED",
        "downloading" => "DOWNLOADING",
        "paused" => "PAUSED",
        "complete" => "DOWNLOADED",
        "imported" => "IMPORTED",
        "failed" => "FAILED",
        "cancelled" => "CANCELLED",
        "retry" => "RECOVERY",
        "import_error" => "IMPORT RETRY",
        "post_import_error" => "SEEDING RETRY",
        "post_import_applied" => "SEEDING POLICY",
        _ => "EVENT",
    }
}

fn seed_download_recovery_probe(store: &SettingsStore) -> Result<()> {
    if store
        .jobs()?
        .iter()
        .any(|job| job.id == "download-recovery-ui-probe")
    {
        return Ok(());
    }
    let probe_root = store
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut job = DownloadJob::queued(NewDownloadJob {
        game_id: "download-recovery-game".into(),
        launchbox_db_id: 140,
        title: "Super Mario Bros.".into(),
        platform: "Nintendo Entertainment System".into(),
        source_kind: "minerva".into(),
        torrent_url: "https://example.invalid/reviewed-minerva-source.torrent".into(),
        torrent_file_index: Some(7),
        torrent_file_path: "Nintendo Entertainment System/Super Mario Bros. (USA).zip".into(),
        info_hash: "a".repeat(40),
        client_save_path: "downloads/lunchbox".into(),
        local_download_path: probe_root.join("downloads/Super Mario Bros. (USA).zip"),
        local_target_path: probe_root.join("roms/Super Mario Bros. (USA).zip"),
        download_plan: String::new(),
    });
    job.id = "download-recovery-ui-probe".into();
    job.created_at = 1_700_000_000;
    job.updated_at = 1_700_000_000;
    job.message = "Reviewed exact Minerva file queued in qBittorrent".into();
    store.upsert_job(&job)?;
    job.state = "downloading".into();
    job.progress = 0.42;
    job.downloaded_bytes = 352_321_536;
    job.total_bytes = 838_860_800;
    job.updated_at += 60;
    job.message = "Downloading exact reviewed file at 8.4 MiB/s".into();
    store.upsert_job(&job)?;
    job.state = "failed".into();
    job.download_speed = 0;
    job.updated_at += 60;
    job.message = "qBittorrent reported missingFiles; existing payload retained".into();
    store.upsert_job(&job)?;
    Ok(())
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

    use super::{
        can_retry, event_label, format_download_rate, is_active, job_badge, needs_refresh,
        ready_pause_hashes, recovery_failure_message, relevant_job_index,
        seed_download_recovery_probe,
    };
    use crate::settings::{DownloadJob, NewDownloadJob, SettingsStore};

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

    #[test]
    fn recovery_is_bounded_to_failed_jobs_and_has_actionable_labels() {
        let mut failed = job("failed", "abc123", "failed", "none");
        assert!(can_retry(&failed));
        failed.post_import_action = "pause_pending".into();
        assert!(!can_retry(&failed));
        assert!(!can_retry(&job("active", "abc123", "downloading", "none")));
        assert_eq!(event_label("retry"), "RECOVERY");
        assert_eq!(event_label("import_error"), "IMPORT RETRY");
        let message = recovery_failure_message("retained torrent metadata failed its receipt");
        assert!(message.contains("retained torrent metadata"));
        assert!(message.contains("downloaded files, and target files were left untouched"));
    }

    #[test]
    fn recovery_ui_fixture_is_idempotent_and_retains_exact_events() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        seed_download_recovery_probe(&store).unwrap();
        seed_download_recovery_probe(&store).unwrap();

        let jobs = store.jobs().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "download-recovery-ui-probe");
        assert_eq!(jobs[0].state, "failed");
        assert_eq!(jobs[0].torrent_file_index, Some(7));
        let events = store.download_job_events(&jobs[0].id, 16).unwrap();
        assert_eq!(
            events
                .iter()
                .map(|event| event.kind.as_str())
                .collect::<Vec<_>>(),
            vec!["failed", "downloading", "queued"]
        );
    }

    #[test]
    fn per_game_status_uses_the_newest_non_cancelled_job() {
        let cancelled = job("game", "new", "cancelled", "none");
        let complete = job("game", "current", "complete", "none");
        let unrelated = job("other", "other", "downloading", "none");
        let jobs = vec![cancelled, complete, unrelated];
        assert_eq!(relevant_job_index(&jobs, "game"), Some(1));
        assert_eq!(relevant_job_index(&jobs, "missing"), None);
        assert_eq!(job_badge("complete"), "INSTALLING");
        assert_eq!(job_badge("imported"), "READY");
        assert_eq!(job_badge("failed"), "ATTENTION");
    }
}
