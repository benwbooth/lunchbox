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
        #[qproperty(bool, auditing)]
        #[qproperty(bool, repairing)]
        #[qproperty(QString, message)]
        #[qproperty(QString, phase)]
        #[qproperty(QString, scope)]
        #[qproperty(QString, artwork_kind)]
        #[qproperty(i32, progress_current)]
        #[qproperty(i32, progress_total)]
        #[qproperty(i32, examined_count)]
        #[qproperty(i32, covered_count)]
        #[qproperty(i32, missing_count)]
        #[qproperty(i32, repairable_count)]
        #[qproperty(i32, manual_review_count)]
        #[qproperty(i32, selected_count)]
        #[qproperty(i32, repaired_count)]
        #[qproperty(i32, unavailable_count)]
        #[qproperty(i32, failed_count)]
        #[qproperty(i32, repair_total)]
        #[qproperty(i32, repair_completed)]
        #[qproperty(i32, entry_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, media_revision)]
        #[qproperty(bool, recovery_available)]
        #[qproperty(i32, recovery_count)]
        #[qproperty(QString, recovery_summary)]
        type MediaAuditModel = super::MediaAuditModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut MediaAuditModel>);

        #[qinvokable]
        fn start_audit(self: Pin<&mut MediaAuditModel>, scope: QString, artwork_kind: QString);

        #[qinvokable]
        fn cancel(self: Pin<&mut MediaAuditModel>);

        #[qinvokable]
        fn apply_filter(
            self: Pin<&mut MediaAuditModel>,
            search: QString,
            status: QString,
            sort: QString,
        );

        #[qinvokable]
        fn toggle_selected(self: Pin<&mut MediaAuditModel>, index: i32);

        #[qinvokable]
        fn select_visible(self: Pin<&mut MediaAuditModel>, selected: bool);

        #[qinvokable]
        fn repair_selected(self: Pin<&mut MediaAuditModel>);

        #[qinvokable]
        fn resume_repair(self: Pin<&mut MediaAuditModel>);

        #[qinvokable]
        fn discard_recovery(self: Pin<&mut MediaAuditModel>);

        #[qinvokable]
        fn row_game_uid_at(self: &MediaAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_database_id_at(self: &MediaAuditModel, index: i32) -> i32;

        #[qinvokable]
        fn row_title_at(self: &MediaAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_platform_at(self: &MediaAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_local_at(self: &MediaAuditModel, index: i32) -> bool;

        #[qinvokable]
        fn row_downloadable_at(self: &MediaAuditModel, index: i32) -> bool;

        #[qinvokable]
        fn row_kind_label_at(self: &MediaAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_status_at(self: &MediaAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_status_label_at(self: &MediaAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_detail_at(self: &MediaAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_repairable_at(self: &MediaAuditModel, index: i32) -> bool;

        #[qinvokable]
        fn row_selected_at(self: &MediaAuditModel, index: i32) -> bool;

        #[qinvokable]
        fn report_ui_probe(self: &MediaAuditModel);

        #[qinvokable]
        fn verify_recovery_probe(self: &MediaAuditModel, dialog_visible: bool) -> bool;
    }

    impl cxx_qt::Threading for MediaAuditModel {}
}

use std::collections::{HashMap, HashSet, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::catalog;
use crate::media::{ArtworkKind, MediaFetchOutcome, MediaFetchQueue, MediaFetchRequest};
use crate::media_audit::{
    MediaAuditOutput, MediaAuditProgress, MissingMediaEntry, audit_missing_media,
    validate_audit_request,
};
use crate::media_repair_batch::{
    MediaRepairBatchSnapshot, MediaRepairLease, MediaRepairTarget, try_acquire_media_repair_lease,
};
use crate::settings::SettingsStore;

const MAX_INFLIGHT_REPAIRS: usize = 8;

pub struct MediaAuditModelRust {
    initialized: bool,
    busy: bool,
    auditing: bool,
    repairing: bool,
    message: QString,
    phase: QString,
    scope: QString,
    artwork_kind: QString,
    progress_current: i32,
    progress_total: i32,
    examined_count: i32,
    covered_count: i32,
    missing_count: i32,
    repairable_count: i32,
    manual_review_count: i32,
    selected_count: i32,
    repaired_count: i32,
    unavailable_count: i32,
    failed_count: i32,
    repair_total: i32,
    repair_completed: i32,
    entry_count: i32,
    revision: i32,
    media_revision: i32,
    recovery_available: bool,
    recovery_count: i32,
    recovery_summary: QString,
    output: Option<Arc<MediaAuditOutput>>,
    visible_indices: Vec<usize>,
    selected: HashSet<usize>,
    repaired: HashSet<usize>,
    unavailable: HashSet<usize>,
    errors: HashMap<usize, String>,
    search: String,
    status: String,
    sort: String,
    cancel: Option<Arc<AtomicBool>>,
    generation: u64,
    repair_generation: u64,
    repair_queue: Option<MediaFetchQueue>,
    repair_pending: VecDeque<usize>,
    repair_active: HashMap<String, usize>,
    repair_pump_busy: bool,
    repair_finalizing: bool,
    repair_successes: usize,
    repair_unavailable: usize,
    repair_failures: usize,
    repair_batch_id: Option<String>,
    repair_lease: Option<MediaRepairLease>,
    recovered_running_items: usize,
    ui_probe: bool,
}

impl Default for MediaAuditModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            auditing: false,
            repairing: false,
            message: QString::from(
                "Audit exact artwork coverage, review every miss, then repair only selected records.",
            ),
            phase: QString::default(),
            scope: QString::from("collection"),
            artwork_kind: QString::from("box-front"),
            progress_current: 0,
            progress_total: 0,
            examined_count: 0,
            covered_count: 0,
            missing_count: 0,
            repairable_count: 0,
            manual_review_count: 0,
            selected_count: 0,
            repaired_count: 0,
            unavailable_count: 0,
            failed_count: 0,
            repair_total: 0,
            repair_completed: 0,
            entry_count: 0,
            revision: 0,
            media_revision: 0,
            recovery_available: false,
            recovery_count: 0,
            recovery_summary: QString::default(),
            output: None,
            visible_indices: Vec::new(),
            selected: HashSet::new(),
            repaired: HashSet::new(),
            unavailable: HashSet::new(),
            errors: HashMap::new(),
            search: String::new(),
            status: "all".to_owned(),
            sort: "title".to_owned(),
            cancel: None,
            generation: 0,
            repair_generation: 0,
            repair_queue: None,
            repair_pending: VecDeque::new(),
            repair_active: HashMap::new(),
            repair_pump_busy: false,
            repair_finalizing: false,
            repair_successes: 0,
            repair_unavailable: 0,
            repair_failures: 0,
            repair_batch_id: None,
            repair_lease: None,
            recovered_running_items: 0,
            ui_probe: std::env::args().any(|argument| argument == "--media-audit-ui-probe"),
        }
    }
}

struct RecoveryLoad {
    snapshot: Option<MediaRepairBatchSnapshot>,
    recovered_running_items: usize,
    active_elsewhere: bool,
}

struct PreparedRepair {
    batch_id: String,
    lease: MediaRepairLease,
    provider_priority: Vec<String>,
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn entry_at(model: &qobject::MediaAuditModel, index: i32) -> Option<(usize, &MissingMediaEntry)> {
    let visible_index = usize::try_from(index).ok()?;
    let source_index = *model.rust().visible_indices.get(visible_index)?;
    let entry = model.rust().output.as_ref()?.entries.get(source_index)?;
    Some((source_index, entry))
}

fn source_status(model: &qobject::MediaAuditModel, source_index: usize) -> &'static str {
    if model.rust().repaired.contains(&source_index) {
        "repaired"
    } else if model.rust().errors.contains_key(&source_index) {
        "failed"
    } else if model.rust().unavailable.contains(&source_index) {
        "unavailable"
    } else if model
        .rust()
        .output
        .as_ref()
        .and_then(|output| output.entries.get(source_index))
        .is_some_and(|entry| entry.repairable)
    {
        "missing"
    } else {
        "review"
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "repaired" => "REPAIRED",
        "failed" => "FAILED",
        "unavailable" => "NOT FOUND",
        "review" => "REVIEW",
        _ => "MISSING",
    }
}

fn status_rank(status: &str) -> u8 {
    match status {
        "failed" => 0,
        "unavailable" => 1,
        "missing" => 2,
        "review" => 3,
        "repaired" => 4,
        _ => 5,
    }
}

fn output_from_repair_snapshot(
    snapshot: &MediaRepairBatchSnapshot,
) -> anyhow::Result<MediaAuditOutput> {
    let entries = snapshot
        .items
        .iter()
        .map(|item| {
            let kind = ArtworkKind::parse(&item.target.artwork_kind).ok_or_else(|| {
                anyhow::anyhow!(
                    "saved repair item has unsupported media category {}",
                    item.target.artwork_kind
                )
            })?;
            Ok(MissingMediaEntry {
                game_uid: item.target.game_uid.clone(),
                launchbox_db_id: item.target.launchbox_db_id,
                title: item.target.title.clone(),
                platform: item.target.platform.clone(),
                local: item.target.local,
                downloadable: item.target.downloadable,
                kind,
                repairable: true,
                detail: item.target.detail.clone(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let repairable_count = entries.len();
    Ok(MediaAuditOutput {
        entries,
        examined_count: repairable_count,
        repairable_count,
        ..MediaAuditOutput::default()
    })
}

impl qobject::MediaAuditModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_phase(qstring("Checking repair recovery"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-media-repair-recovery".into())
            .spawn(move || {
                let result = (|| {
                    let store = SettingsStore::open_default()?;
                    let Some(_lease) = try_acquire_media_repair_lease(&store)? else {
                        return Ok(RecoveryLoad {
                            snapshot: store.active_media_repair_batch()?,
                            recovered_running_items: 0,
                            active_elsewhere: true,
                        });
                    };
                    let recovered = store.recover_interrupted_media_repair_batch()?;
                    Ok(RecoveryLoad {
                        snapshot: recovered.as_ref().map(|batch| batch.snapshot.clone()),
                        recovered_running_items: recovered
                            .as_ref()
                            .map_or(0, |batch| batch.recovered_running_items),
                        active_elsewhere: false,
                    })
                })()
                .map_err(|error: anyhow::Error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_initialize(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_initialized(true);
            self.as_mut().set_message(qstring(format!(
                "Could not check for interrupted artwork repairs: {error}"
            )));
        }
    }

    fn finish_initialize(mut self: Pin<&mut Self>, result: Result<RecoveryLoad, String>) {
        self.as_mut().set_busy(false);
        self.as_mut().set_initialized(true);
        match result {
            Ok(RecoveryLoad {
                snapshot: Some(snapshot),
                recovered_running_items,
                active_elsewhere,
            }) => match output_from_repair_snapshot(&snapshot) {
                Ok(output) => {
                    self.as_mut().restore_repair_snapshot(
                        snapshot,
                        output,
                        recovered_running_items,
                        active_elsewhere,
                    );
                }
                Err(error) => self.as_mut().set_message(qstring(format!(
                    "Could not restore the interrupted artwork repair: {error:#}"
                ))),
            },
            Ok(_) => {}
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not check for interrupted artwork repairs: {error}"
            ))),
        }
    }

    fn restore_repair_snapshot(
        mut self: Pin<&mut Self>,
        snapshot: MediaRepairBatchSnapshot,
        output: MediaAuditOutput,
        recovered_running_items: usize,
        active_elsewhere: bool,
    ) {
        self.as_mut().rust_mut().output = Some(Arc::new(output));
        self.as_mut().rust_mut().selected.clear();
        self.as_mut().rust_mut().repaired.clear();
        self.as_mut().rust_mut().unavailable.clear();
        self.as_mut().rust_mut().errors.clear();
        for item in &snapshot.items {
            match item.status.as_str() {
                "pending" | "running" => {
                    self.as_mut().rust_mut().selected.insert(item.position);
                }
                "succeeded" => {
                    self.as_mut().rust_mut().repaired.insert(item.position);
                }
                "unavailable" => {
                    self.as_mut().rust_mut().unavailable.insert(item.position);
                }
                "failed" => {
                    self.as_mut()
                        .rust_mut()
                        .errors
                        .insert(item.position, item.outcome_detail.clone());
                }
                _ => {}
            }
        }
        let unfinished = snapshot.unfinished_count();
        let completed = snapshot.items.len().saturating_sub(unfinished);
        let summary = if active_elsewhere {
            format!(
                "Another Lunchbox process is repairing this batch. {unfinished} exact record{} remain.",
                if unfinished == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "Lunchbox found an interrupted repair with {unfinished} exact record{} remaining. Completed files were kept.",
                if unfinished == 1 { "" } else { "s" }
            )
        };
        let item_count = snapshot.items.len();
        let succeeded_count = snapshot.succeeded_count();
        let unavailable_count = self.as_ref().rust().unavailable.len();
        let failed_count = self.as_ref().rust().errors.len();
        self.as_mut().rust_mut().repair_batch_id = Some(snapshot.id.clone());
        self.as_mut().rust_mut().recovered_running_items = recovered_running_items;
        self.as_mut().set_scope(qstring(&snapshot.scope));
        self.as_mut()
            .set_artwork_kind(qstring(&snapshot.artwork_kind));
        self.as_mut().set_examined_count(saturating_i32(item_count));
        self.as_mut().set_missing_count(saturating_i32(item_count));
        self.as_mut()
            .set_repairable_count(saturating_i32(item_count));
        self.as_mut()
            .set_repaired_count(saturating_i32(succeeded_count));
        self.as_mut()
            .set_unavailable_count(saturating_i32(unavailable_count));
        self.as_mut().set_failed_count(saturating_i32(failed_count));
        self.as_mut().set_selected_count(saturating_i32(unfinished));
        self.as_mut().set_repair_total(saturating_i32(item_count));
        self.as_mut()
            .set_repair_completed(saturating_i32(completed));
        self.as_mut().set_recovery_available(true);
        self.as_mut().set_recovery_count(saturating_i32(unfinished));
        self.as_mut().set_recovery_summary(qstring(&summary));
        self.as_mut().set_phase(qstring("Repair interrupted"));
        self.as_mut().set_message(qstring(summary));
        self.as_mut().rebuild_visible();
    }

    pub fn start_audit(mut self: Pin<&mut Self>, scope: QString, artwork_kind: QString) {
        if *self.as_ref().busy() {
            return;
        }
        if *self.as_ref().recovery_available() {
            self.as_mut().set_message(qstring(
                "Resume or discard the interrupted artwork repair before starting another audit.",
            ));
            return;
        }
        let scope_text = scope.to_string();
        let kind_text = artwork_kind.to_string();
        let (scope, kind) = match validate_audit_request(&scope_text, &kind_text) {
            Ok(request) => request,
            Err(error) => {
                self.as_mut().set_message(qstring(error.to_string()));
                return;
            }
        };
        let Some(catalog_path) = catalog::requested_database_path() else {
            self.as_mut().set_message(qstring(
                "The Lunchbox catalog is required before running a media audit.",
            ));
            return;
        };

        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        let cancelled = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().cancel = Some(Arc::clone(&cancelled));
        self.as_mut().rust_mut().output = None;
        self.as_mut().rust_mut().visible_indices.clear();
        self.as_mut().rust_mut().selected.clear();
        self.as_mut().rust_mut().repaired.clear();
        self.as_mut().rust_mut().unavailable.clear();
        self.as_mut().rust_mut().errors.clear();
        self.as_mut().set_scope(qstring(&scope_text));
        self.as_mut().set_artwork_kind(qstring(&kind_text));
        self.as_mut().set_busy(true);
        self.as_mut().set_auditing(true);
        self.as_mut().set_repairing(false);
        self.as_mut().set_phase(qstring("Starting audit"));
        self.as_mut().set_progress_current(0);
        self.as_mut().set_progress_total(0);
        self.as_mut().set_examined_count(0);
        self.as_mut().set_covered_count(0);
        self.as_mut().set_missing_count(0);
        self.as_mut().set_repairable_count(0);
        self.as_mut().set_manual_review_count(0);
        self.as_mut().set_selected_count(0);
        self.as_mut().set_repaired_count(0);
        self.as_mut().set_unavailable_count(0);
        self.as_mut().set_failed_count(0);
        self.as_mut().set_repair_total(0);
        self.as_mut().set_repair_completed(0);
        self.as_mut().set_entry_count(0);
        self.as_mut().set_message(qstring(format!(
            "Auditing exact {} coverage across {}…",
            kind.label().to_lowercase(),
            scope.label()
        )));

        let media_root = crate::media::requested_media_directory();
        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress: Arc<dyn Fn(MediaAuditProgress) + Send + Sync> = Arc::new(move |progress| {
            let _ = progress_thread.queue(move |mut model| {
                model.as_mut().update_audit_progress(generation, progress);
            });
        });
        let spawn = std::thread::Builder::new()
            .name("lunchbox-media-audit".into())
            .spawn(move || {
                let result = audit_missing_media(
                    &catalog_path,
                    media_root,
                    scope,
                    kind,
                    &cancelled,
                    progress,
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_audit(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().rust_mut().cancel = None;
            self.as_mut().set_busy(false);
            self.as_mut().set_auditing(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start media audit: {error}")));
        }
    }

    pub fn cancel(mut self: Pin<&mut Self>) {
        if *self.as_ref().auditing() {
            if let Some(cancel) = self.as_ref().rust().cancel.as_ref() {
                cancel.store(true, Ordering::Relaxed);
                self.as_mut().set_message(qstring(
                    "Cancelling after the current bounded media-cache operation…",
                ));
            }
            return;
        }
        if *self.as_ref().repairing() {
            if self.as_ref().rust().repair_finalizing {
                self.as_mut().set_message(qstring(
                    "Finalizing the durable repair journal; completed provider work is already safe.",
                ));
                return;
            }
            let completed = *self.as_ref().repair_completed();
            let total = *self.as_ref().repair_total();
            let repaired = self.as_ref().rust().repair_successes;
            let generation = self.as_ref().rust().repair_generation.wrapping_add(1);
            self.as_mut().rust_mut().repair_generation = generation;
            self.as_mut().rust_mut().repair_queue = None;
            self.as_mut().rust_mut().repair_pending.clear();
            self.as_mut().rust_mut().repair_active.clear();
            self.as_mut().rust_mut().repair_pump_busy = false;
            self.as_mut().rust_mut().repair_finalizing = false;
            let batch_id = self.as_ref().rust().repair_batch_id.clone();
            let lease = self.as_mut().rust_mut().repair_lease.take();
            self.as_mut().rust_mut().repair_batch_id = None;
            self.as_mut().set_repairing(false);
            if let (Some(batch_id), Some(lease)) = (batch_id, lease) {
                self.as_mut().set_busy(true);
                self.as_mut().set_phase(qstring("Cancelling repair"));
                self.as_mut().set_message(qstring(
                    "Saving cancellation; completed artwork is being kept…",
                ));
                let qt_thread = self.as_ref().qt_thread();
                let spawn = std::thread::Builder::new()
                    .name("lunchbox-media-repair-cancel".into())
                    .spawn(move || {
                        let _lease = lease;
                        let result = SettingsStore::open_default()
                            .and_then(|store| {
                                store.cancel_media_repair_batch(
                                    &batch_id,
                                    "Cancelled by the user; completed artwork was kept.",
                                )
                            })
                            .map_err(|error| format!("{error:#}"));
                        let _ = qt_thread.queue(move |mut model| {
                            model.as_mut().finish_cancel_repair(
                                generation, completed, total, repaired, result,
                            );
                        });
                    });
                if let Err(error) = spawn {
                    self.as_mut().set_busy(false);
                    self.as_mut().set_phase(qstring("Repair stopped"));
                    self.as_mut().set_message(qstring(format!(
                        "Repair stopped, but the cancellation journal worker could not start: {error}"
                    )));
                }
            } else {
                self.as_mut().set_busy(false);
                self.as_mut().set_phase(qstring("Repair cancelled"));
                self.as_mut()
                    .set_message(qstring("Repair cancelled before provider work started."));
            }
        }
    }

    fn finish_cancel_repair(
        mut self: Pin<&mut Self>,
        generation: u64,
        completed: i32,
        total: i32,
        repaired: usize,
        result: Result<(), String>,
    ) {
        if generation != self.as_ref().rust().repair_generation {
            return;
        }
        self.as_mut().set_busy(false);
        self.as_mut().set_phase(qstring("Repair cancelled"));
        self.as_mut().set_message(qstring(match result {
            Ok(()) => format!(
                "Repair cancelled after {completed} of {total} selected records. Completed files were kept."
            ),
            Err(error) => format!(
                "Repair stopped after {completed} of {total} records, but its recovery journal could not be cancelled: {error}"
            ),
        }));
        if repaired > 0 {
            let revision = self.as_ref().media_revision().wrapping_add(1);
            self.as_mut().set_media_revision(revision);
        }
    }

    fn update_audit_progress(
        mut self: Pin<&mut Self>,
        generation: u64,
        progress: MediaAuditProgress,
    ) {
        if generation != self.as_ref().rust().generation || !*self.as_ref().auditing() {
            return;
        }
        self.as_mut().set_phase(qstring(progress.phase));
        self.as_mut()
            .set_progress_current(saturating_i32(progress.current));
        self.as_mut()
            .set_progress_total(saturating_i32(progress.total));
    }

    fn finish_audit(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<MediaAuditOutput, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().rust_mut().cancel = None;
        match result {
            Ok(output) => {
                let cancelled = output.cancelled;
                let examined = output.examined_count;
                let covered = output.covered_count;
                let missing = output.entries.len();
                let repairable = output.repairable_count;
                let manual = output.manual_review_count;
                let skipped = output.skipped_media_entries;
                let warning = output.warning.clone();
                self.as_mut().set_examined_count(saturating_i32(examined));
                self.as_mut().set_covered_count(saturating_i32(covered));
                self.as_mut().set_missing_count(saturating_i32(missing));
                self.as_mut()
                    .set_repairable_count(saturating_i32(repairable));
                self.as_mut()
                    .set_manual_review_count(saturating_i32(manual));
                self.as_mut().set_progress_current(saturating_i32(examined));
                self.as_mut().rust_mut().output = Some(Arc::new(output));
                self.as_mut().rebuild_visible();
                self.as_mut().set_phase(qstring(if cancelled {
                    "Audit cancelled"
                } else {
                    "Audit complete"
                }));
                let summary = if cancelled {
                    format!(
                        "Audit cancelled after {examined} games; partial results remain reviewable."
                    )
                } else if missing == 0 {
                    format!("All {examined} games have exact cached artwork in this category.")
                } else {
                    format!(
                        "Found {missing} missing across {examined} games: {repairable} exact LibRetro repairs and {manual} records needing provider review."
                    )
                };
                let skipped_notice = if skipped == 0 {
                    String::new()
                } else {
                    format!(" {skipped} unreadable cache entries were skipped.")
                };
                self.as_mut().set_message(qstring(match warning {
                    Some(warning) => {
                        format!("{summary}{skipped_notice} Media cache warning: {warning}")
                    }
                    None => format!("{summary}{skipped_notice}"),
                }));
                if self.as_ref().rust().ui_probe {
                    println!(
                        "LUNCHBOX_MEDIA_AUDIT_MODEL_READY examined={examined} covered={covered} missing={missing} repairable={repairable} manual={manual}"
                    );
                }
            }
            Err(error) => {
                self.as_mut().set_phase(qstring("Audit failed"));
                self.as_mut()
                    .set_message(qstring(format!("Media audit failed: {error}")));
            }
        }
        self.as_mut().set_busy(false);
        self.as_mut().set_auditing(false);
    }

    pub fn apply_filter(mut self: Pin<&mut Self>, search: QString, status: QString, sort: QString) {
        self.as_mut().rust_mut().search = search.to_string().trim().to_lowercase();
        self.as_mut().rust_mut().status = status.to_string();
        self.as_mut().rust_mut().sort = sort.to_string();
        self.as_mut().rebuild_visible();
    }

    fn rebuild_visible(mut self: Pin<&mut Self>) {
        let search = self.as_ref().rust().search.clone();
        let status = self.as_ref().rust().status.clone();
        let sort = self.as_ref().rust().sort.clone();
        let mut visible = self
            .as_ref()
            .rust()
            .output
            .as_ref()
            .map(|output| {
                output
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(index, entry)| {
                        let row_status = source_status(self.as_ref().get_ref(), *index);
                        (search.is_empty()
                            || entry.title.to_lowercase().contains(&search)
                            || entry.platform.to_lowercase().contains(&search)
                            || entry.detail.to_lowercase().contains(&search))
                            && match status.as_str() {
                                "all" => true,
                                "repairable" => entry.repairable && row_status != "repaired",
                                "review" => !entry.repairable,
                                value => row_status == value,
                            }
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if let Some(output) = self.as_ref().rust().output.as_ref() {
            visible.sort_by(|left, right| {
                let left_entry = &output.entries[*left];
                let right_entry = &output.entries[*right];
                match sort.as_str() {
                    "platform" => left_entry
                        .platform
                        .to_lowercase()
                        .cmp(&right_entry.platform.to_lowercase())
                        .then_with(|| {
                            left_entry
                                .title
                                .to_lowercase()
                                .cmp(&right_entry.title.to_lowercase())
                        }),
                    "status" => status_rank(source_status(self.as_ref().get_ref(), *left))
                        .cmp(&status_rank(source_status(self.as_ref().get_ref(), *right)))
                        .then_with(|| {
                            left_entry
                                .title
                                .to_lowercase()
                                .cmp(&right_entry.title.to_lowercase())
                        }),
                    _ => left_entry
                        .title
                        .to_lowercase()
                        .cmp(&right_entry.title.to_lowercase())
                        .then_with(|| left_entry.game_uid.cmp(&right_entry.game_uid)),
                }
            });
        }
        let count = visible.len();
        self.as_mut().rust_mut().visible_indices = visible;
        self.as_mut().set_entry_count(saturating_i32(count));
        self.as_mut().bump_revision();
    }

    pub fn toggle_selected(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() {
            return;
        }
        let Some((source_index, entry)) = entry_at(self.as_ref().get_ref(), index) else {
            return;
        };
        if !entry.repairable || self.as_ref().rust().repaired.contains(&source_index) {
            return;
        }
        if !self.as_mut().rust_mut().selected.insert(source_index) {
            self.as_mut().rust_mut().selected.remove(&source_index);
        }
        self.as_mut().sync_selected_count();
    }

    pub fn select_visible(mut self: Pin<&mut Self>, selected: bool) {
        if *self.as_ref().busy() {
            return;
        }
        let candidates = self.as_ref().rust().visible_indices.clone();
        for source_index in candidates {
            let selectable = self
                .as_ref()
                .rust()
                .output
                .as_ref()
                .and_then(|output| output.entries.get(source_index))
                .is_some_and(|entry| entry.repairable)
                && !self.as_ref().rust().repaired.contains(&source_index);
            if !selectable {
                continue;
            }
            if selected {
                self.as_mut().rust_mut().selected.insert(source_index);
            } else {
                self.as_mut().rust_mut().selected.remove(&source_index);
            }
        }
        self.as_mut().sync_selected_count();
    }

    fn sync_selected_count(mut self: Pin<&mut Self>) {
        let count = self.as_ref().rust().selected.len();
        self.as_mut().set_selected_count(saturating_i32(count));
        self.as_mut().bump_revision();
    }

    pub fn repair_selected(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy()
            || *self.as_ref().recovery_available()
            || self.as_ref().rust().selected.is_empty()
        {
            return;
        }
        if !crate::media::media_retrieval_enabled() {
            self.as_mut().set_message(qstring(
                "Artwork repair is unavailable in offline mode; the audit remains reviewable.",
            ));
            return;
        }
        let mut selected = self
            .as_ref()
            .rust()
            .selected
            .iter()
            .copied()
            .filter(|index| {
                self.as_ref()
                    .rust()
                    .output
                    .as_ref()
                    .and_then(|output| output.entries.get(*index))
                    .is_some_and(|entry| entry.repairable)
            })
            .collect::<Vec<_>>();
        selected.sort_unstable();
        if selected.is_empty() {
            return;
        }

        let output = self.as_ref().rust().output.clone();
        let Some(output) = output else {
            return;
        };
        self.as_mut().rust_mut().repair_generation =
            self.as_ref().rust().repair_generation.wrapping_add(1);
        let generation = self.as_ref().rust().repair_generation;
        let scope = self.as_ref().scope().to_string();
        let artwork_kind = self.as_ref().artwork_kind().to_string();
        let batch_id = uuid::Uuid::new_v4().to_string();
        self.as_mut().set_busy(true);
        self.as_mut().set_repairing(true);
        self.as_mut().set_phase(qstring("Preparing durable repair"));
        self.as_mut().set_message(qstring(format!(
            "Saving {} exact repair records before provider work starts…",
            selected.len()
        )));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-media-repair-prepare".into())
            .spawn(move || {
                let result = (|| {
                    let store = SettingsStore::open_default()?;
                    let lease = try_acquire_media_repair_lease(&store)?.ok_or_else(|| {
                        anyhow::anyhow!(
                            "artwork repair is already active in another Lunchbox process"
                        )
                    })?;
                    let targets = selected
                        .iter()
                        .filter_map(|index| output.entries.get(*index))
                        .map(|entry| MediaRepairTarget {
                            game_uid: entry.game_uid.clone(),
                            launchbox_db_id: entry.launchbox_db_id,
                            title: entry.title.clone(),
                            platform: entry.platform.clone(),
                            local: entry.local,
                            downloadable: entry.downloadable,
                            artwork_kind: entry.kind.key().to_owned(),
                            detail: entry.detail.clone(),
                        })
                        .collect::<Vec<_>>();
                    if targets.len() != selected.len() {
                        anyhow::bail!("one or more selected repair records disappeared");
                    }
                    store.begin_media_repair_batch(&batch_id, &scope, &artwork_kind, &targets)?;
                    let settings = store.load()?;
                    let provider_priority = crate::media::effective_provider_priority(
                        &settings.media_provider_priority,
                    );
                    Ok(PreparedRepair {
                        batch_id,
                        lease,
                        provider_priority,
                    })
                })()
                .map_err(|error: anyhow::Error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_prepare_repair(generation, selected, result, 0, 0, 0, 0);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_repairing(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the durable repair worker: {error}"
            )));
        }
    }

    pub fn resume_repair(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || !*self.as_ref().recovery_available() {
            return;
        }
        if !crate::media::media_retrieval_enabled() {
            self.as_mut().set_message(qstring(
                "Artwork repair is unavailable in offline mode. The interrupted batch remains saved.",
            ));
            return;
        }
        let Some(batch_id) = self.as_ref().rust().repair_batch_id.clone() else {
            self.as_mut()
                .set_message(qstring("The saved artwork repair identity is unavailable."));
            return;
        };
        let mut selected = self
            .as_ref()
            .rust()
            .selected
            .iter()
            .copied()
            .collect::<Vec<_>>();
        selected.sort_unstable();
        let successes_before = self.as_ref().rust().repaired.len();
        let unavailable_before = self.as_ref().rust().unavailable.len();
        let failures_before = self.as_ref().rust().errors.len();
        let completed_before = successes_before
            .saturating_add(unavailable_before)
            .saturating_add(failures_before);
        self.as_mut().rust_mut().repair_generation =
            self.as_ref().rust().repair_generation.wrapping_add(1);
        let generation = self.as_ref().rust().repair_generation;
        self.as_mut().set_busy(true);
        self.as_mut().set_repairing(true);
        self.as_mut().set_phase(qstring("Resuming durable repair"));
        self.as_mut()
            .set_message(qstring("Claiming the saved repair batch…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-media-repair-resume".into())
            .spawn(move || {
                let result = (|| {
                    let store = SettingsStore::open_default()?;
                    let lease = try_acquire_media_repair_lease(&store)?.ok_or_else(|| {
                        anyhow::anyhow!(
                            "this repair is active in another Lunchbox process; close that process before resuming here"
                        )
                    })?;
                    store.claim_media_repair_batch(&batch_id)?;
                    let settings = store.load()?;
                    let provider_priority =
                        crate::media::effective_provider_priority(&settings.media_provider_priority);
                    Ok(PreparedRepair {
                        batch_id,
                        lease,
                        provider_priority,
                    })
                })()
                .map_err(|error: anyhow::Error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_prepare_repair(
                        generation,
                        selected,
                        result,
                        completed_before,
                        successes_before,
                        unavailable_before,
                        failures_before,
                    );
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_repairing(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the repair recovery worker: {error}"
            )));
        }
    }

    pub fn discard_recovery(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || !*self.as_ref().recovery_available() {
            return;
        }
        let Some(batch_id) = self.as_ref().rust().repair_batch_id.clone() else {
            return;
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_phase(qstring("Discarding saved repair"));
        self.as_mut()
            .set_message(qstring("Discarding the interrupted repair journal…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-media-repair-discard".into())
            .spawn(move || {
                let result = (|| {
                    let store = SettingsStore::open_default()?;
                    let _lease = try_acquire_media_repair_lease(&store)?.ok_or_else(|| {
                        anyhow::anyhow!("the repair is active in another Lunchbox process")
                    })?;
                    store.cancel_media_repair_batch(&batch_id, "Discarded by the user.")
                })()
                .map_err(|error: anyhow::Error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_discard_recovery(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the repair discard worker: {error}"
            )));
        }
    }

    fn finish_discard_recovery(mut self: Pin<&mut Self>, result: Result<(), String>) {
        self.as_mut().set_busy(false);
        if let Err(error) = result {
            self.as_mut().set_message(qstring(format!(
                "Could not discard the saved artwork repair: {error}"
            )));
            return;
        }
        self.as_mut().rust_mut().repair_batch_id = None;
        self.as_mut().rust_mut().selected.clear();
        self.as_mut().set_selected_count(0);
        self.as_mut().set_recovery_available(false);
        self.as_mut().set_recovery_count(0);
        self.as_mut().set_recovery_summary(QString::default());
        self.as_mut().set_phase(qstring("Repair discarded"));
        self.as_mut().set_message(qstring(
            "The interrupted repair was discarded. Completed artwork remains cached.",
        ));
        self.as_mut().rebuild_visible();
    }

    fn finish_prepare_repair(
        mut self: Pin<&mut Self>,
        generation: u64,
        selected: Vec<usize>,
        result: Result<PreparedRepair, String>,
        completed_before: usize,
        successes_before: usize,
        unavailable_before: usize,
        failures_before: usize,
    ) {
        if generation != self.as_ref().rust().repair_generation {
            if let Ok(prepared) = result {
                let _ = std::thread::Builder::new()
                    .name("lunchbox-media-repair-stale-cleanup".into())
                    .spawn(move || {
                        let PreparedRepair {
                            batch_id,
                            lease,
                            provider_priority: _,
                        } = prepared;
                        let _lease = lease;
                        if let Ok(store) = SettingsStore::open_default() {
                            let _ = store.cancel_media_repair_batch(
                                &batch_id,
                                "Cancelled before provider work started.",
                            );
                        }
                    });
            }
            return;
        }
        match result {
            Ok(prepared) => {
                self.as_mut().set_recovery_available(false);
                self.as_mut().set_recovery_count(0);
                self.as_mut().set_recovery_summary(QString::default());
                self.as_mut().start_repair_queue(
                    selected,
                    prepared.batch_id,
                    prepared.lease,
                    prepared.provider_priority,
                    completed_before,
                    successes_before,
                    unavailable_before,
                    failures_before,
                );
            }
            Err(error) => {
                self.as_mut().set_busy(false);
                self.as_mut().set_repairing(false);
                self.as_mut().set_phase(qstring("Repair not started"));
                self.as_mut().set_message(qstring(format!(
                    "Could not prepare the durable artwork repair: {error}"
                )));
            }
        }
    }

    fn start_repair_queue(
        mut self: Pin<&mut Self>,
        selected: Vec<usize>,
        batch_id: String,
        lease: MediaRepairLease,
        provider_priority: Vec<String>,
        completed_before: usize,
        successes_before: usize,
        unavailable_before: usize,
        failures_before: usize,
    ) {
        self.as_mut().rust_mut().repair_generation =
            self.as_ref().rust().repair_generation.wrapping_add(1);
        let generation = self.as_ref().rust().repair_generation;
        let qt_thread = self.as_ref().qt_thread();
        let queue = match MediaFetchQueue::start(
            crate::media::requested_media_directory(),
            provider_priority,
            move |outcome| {
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_repair(generation, outcome);
                });
            },
        ) {
            Ok(queue) => queue,
            Err(error) => {
                let failed_batch_id = batch_id.clone();
                let _ = std::thread::Builder::new()
                    .name("lunchbox-media-repair-start-failure".into())
                    .spawn(move || {
                        let _lease = lease;
                        if let Ok(store) = SettingsStore::open_default() {
                            let _ = store.fail_media_repair_batch(
                                &failed_batch_id,
                                "The background artwork queue could not start.",
                            );
                        }
                    });
                self.as_mut().set_busy(false);
                self.as_mut().set_repairing(false);
                self.as_mut().set_phase(qstring("Repair not started"));
                self.as_mut().set_message(qstring(format!(
                    "Could not start the background artwork repair queue: {error}"
                )));
                return;
            }
        };
        for source_index in &selected {
            self.as_mut().rust_mut().errors.remove(source_index);
            self.as_mut().rust_mut().unavailable.remove(source_index);
        }
        self.as_mut().rust_mut().repair_queue = Some(queue);
        self.as_mut().rust_mut().repair_batch_id = Some(batch_id);
        self.as_mut().rust_mut().repair_lease = Some(lease);
        self.as_mut().rust_mut().repair_pending = selected.iter().copied().collect();
        self.as_mut().rust_mut().repair_active.clear();
        self.as_mut().rust_mut().repair_successes = successes_before;
        self.as_mut().rust_mut().repair_unavailable = unavailable_before;
        self.as_mut().rust_mut().repair_failures = failures_before;
        let repair_total = selected.len().saturating_add(completed_before);
        self.as_mut().set_repair_total(saturating_i32(repair_total));
        self.as_mut()
            .set_repair_completed(saturating_i32(completed_before));
        self.as_mut()
            .set_progress_current(saturating_i32(completed_before));
        self.as_mut()
            .set_progress_total(saturating_i32(repair_total));
        self.as_mut().set_busy(true);
        self.as_mut().set_repairing(true);
        self.as_mut()
            .set_phase(qstring("Repairing selected artwork"));
        self.as_mut().set_message(qstring(format!(
            "Repairing {} selected records with a durable bounded background queue…",
            selected.len()
        )));
        self.as_mut().pump_repairs();
    }

    fn pump_repairs(mut self: Pin<&mut Self>) {
        if !*self.as_ref().repairing()
            || self.as_ref().rust().repair_pump_busy
            || self.as_ref().rust().repair_finalizing
        {
            return;
        }
        let capacity =
            MAX_INFLIGHT_REPAIRS.saturating_sub(self.as_ref().rust().repair_active.len());
        if capacity == 0 {
            return;
        }
        if self.as_ref().rust().repair_pending.is_empty() {
            if self.as_ref().rust().repair_active.is_empty() {
                self.as_mut().finish_repair_batch();
            }
            return;
        }
        let source_indices = self
            .as_ref()
            .rust()
            .repair_pending
            .iter()
            .take(capacity)
            .copied()
            .collect::<Vec<_>>();
        let entries = source_indices
            .iter()
            .filter_map(|source_index| {
                self.as_ref()
                    .rust()
                    .output
                    .as_ref()
                    .and_then(|output| output.entries.get(*source_index))
                    .cloned()
                    .map(|entry| (*source_index, entry))
            })
            .collect::<Vec<_>>();
        if entries.len() != source_indices.len() {
            self.as_mut().stop_repair_and_fail_journal(
                "A selected repair record disappeared before it could be queued.".to_owned(),
            );
            return;
        }
        let Some(batch_id) = self.as_ref().rust().repair_batch_id.clone() else {
            self.as_mut().stop_repair_and_fail_journal(
                "The durable repair identity disappeared before queueing a record.".to_owned(),
            );
            return;
        };
        let game_uids = entries
            .iter()
            .map(|(_, entry)| entry.game_uid.clone())
            .collect::<Vec<_>>();
        self.as_mut().rust_mut().repair_pump_busy = true;
        let generation = self.as_ref().rust().repair_generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-media-repair-journal-claim".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| store.mark_media_repair_items_running(&batch_id, &game_uids))
                    .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_pump_repairs(generation, entries, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().rust_mut().repair_pump_busy = false;
            self.as_mut().stop_repair_and_fail_journal(format!(
                "Could not start the repair journal worker: {error}"
            ));
        }
    }

    fn finish_pump_repairs(
        mut self: Pin<&mut Self>,
        generation: u64,
        entries: Vec<(usize, MissingMediaEntry)>,
        result: Result<(), String>,
    ) {
        if generation != self.as_ref().rust().repair_generation || !*self.as_ref().repairing() {
            return;
        }
        self.as_mut().rust_mut().repair_pump_busy = false;
        if let Err(error) = result {
            self.as_mut().stop_repair_and_fail_journal(format!(
                "Could not save repair progress before provider work: {error}"
            ));
            return;
        }
        for (source_index, entry) in entries {
            if self.as_mut().rust_mut().repair_pending.pop_front() != Some(source_index) {
                self.as_mut().stop_repair_and_fail_journal(
                    "The durable repair queue changed while records were being claimed.".to_owned(),
                );
                return;
            }
            let request = MediaFetchRequest {
                request_id: entry.game_uid.clone(),
                database_id: entry.launchbox_db_id,
                title: entry.title.clone(),
                platform: entry.platform.clone(),
                requested_kind: entry.kind,
                force: true,
                exact_only: true,
            };
            let queued = self
                .as_ref()
                .rust()
                .repair_queue
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("repair queue stopped"))
                .and_then(|queue| queue.try_request(request));
            if let Err(error) = queued {
                self.as_mut().stop_repair_and_fail_journal(format!(
                    "The provider queue stopped before {} could start: {error}",
                    entry.title
                ));
                return;
            }
            self.as_mut()
                .rust_mut()
                .repair_active
                .insert(entry.game_uid, source_index);
        }
        self.as_mut().pump_repairs();
    }

    fn finish_repair(mut self: Pin<&mut Self>, generation: u64, outcome: MediaFetchOutcome) {
        if generation != self.as_ref().rust().repair_generation || !*self.as_ref().repairing() {
            return;
        }
        let request_id = match &outcome {
            MediaFetchOutcome::Found { request_id, .. }
            | MediaFetchOutcome::Missing { request_id, .. }
            | MediaFetchOutcome::Failed { request_id, .. } => request_id.clone(),
        };
        let Some(source_index) = self.as_ref().rust().repair_active.get(&request_id).copied()
        else {
            return;
        };
        let (terminal_status, detail) = match &outcome {
            MediaFetchOutcome::Found {
                requested_kind,
                fetched_kind,
                provider,
                ..
            } if requested_kind == fetched_kind => (
                "succeeded",
                format!("Cached exact {} from {provider}.", requested_kind.label()),
            ),
            MediaFetchOutcome::Found { .. } => (
                "failed",
                "The provider returned a different media category; no repair was accepted."
                    .to_owned(),
            ),
            MediaFetchOutcome::Missing { .. } => (
                "unavailable",
                "No configured provider returned this exact media category.".to_owned(),
            ),
            MediaFetchOutcome::Failed { error, .. } => ("failed", error.clone()),
        };
        let Some(batch_id) = self.as_ref().rust().repair_batch_id.clone() else {
            self.as_mut().stop_repair_and_fail_journal(
                "The durable repair identity disappeared before saving a result.".to_owned(),
            );
            return;
        };
        let generation = self.as_ref().rust().repair_generation;
        let qt_thread = self.as_ref().qt_thread();
        let saved_request_id = request_id.clone();
        let saved_detail = detail.clone();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-media-repair-journal-result".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| {
                        store.finish_media_repair_item(
                            &batch_id,
                            &saved_request_id,
                            terminal_status,
                            &saved_detail,
                        )
                    })
                    .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_saved_repair(
                        generation,
                        request_id,
                        source_index,
                        outcome,
                        detail,
                        result,
                    );
                });
            });
        if let Err(error) = spawn {
            self.as_mut().stop_repair_and_fail_journal(format!(
                "Could not start the repair result journal worker: {error}"
            ));
        }
    }

    fn finish_saved_repair(
        mut self: Pin<&mut Self>,
        generation: u64,
        request_id: String,
        source_index: usize,
        outcome: MediaFetchOutcome,
        detail: String,
        result: Result<(), String>,
    ) {
        if generation != self.as_ref().rust().repair_generation || !*self.as_ref().repairing() {
            return;
        }
        if let Err(error) = result {
            self.as_mut().stop_repair_and_fail_journal(format!(
                "Could not save the completed artwork result: {error}"
            ));
            return;
        }
        if self.as_mut().rust_mut().repair_active.remove(&request_id) != Some(source_index) {
            self.as_mut().stop_repair_and_fail_journal(
                "The active repair identity changed before its result was applied.".to_owned(),
            );
            return;
        }
        match outcome {
            MediaFetchOutcome::Found {
                requested_kind,
                fetched_kind,
                ..
            } if requested_kind == fetched_kind => {
                self.as_mut().rust_mut().repaired.insert(source_index);
                self.as_mut().rust_mut().selected.remove(&source_index);
                self.as_mut().rust_mut().repair_successes =
                    self.as_ref().rust().repair_successes.saturating_add(1);
            }
            MediaFetchOutcome::Found { .. } => {
                self.as_mut().rust_mut().errors.insert(source_index, detail);
                self.as_mut().rust_mut().repair_failures =
                    self.as_ref().rust().repair_failures.saturating_add(1);
            }
            MediaFetchOutcome::Missing { .. } => {
                self.as_mut().rust_mut().unavailable.insert(source_index);
                self.as_mut().rust_mut().repair_unavailable =
                    self.as_ref().rust().repair_unavailable.saturating_add(1);
            }
            MediaFetchOutcome::Failed { error, .. } => {
                self.as_mut().rust_mut().errors.insert(source_index, error);
                self.as_mut().rust_mut().repair_failures =
                    self.as_ref().rust().repair_failures.saturating_add(1);
            }
        }
        let complete = self.as_ref().repair_completed().saturating_add(1);
        let repaired_count = self.as_ref().rust().repaired.len();
        let unavailable_count = self.as_ref().rust().unavailable.len();
        let failed_count = self.as_ref().rust().errors.len();
        let selected_count = self.as_ref().rust().selected.len();
        self.as_mut().set_repair_completed(complete);
        self.as_mut().set_progress_current(complete);
        self.as_mut()
            .set_repaired_count(saturating_i32(repaired_count));
        self.as_mut()
            .set_unavailable_count(saturating_i32(unavailable_count));
        self.as_mut().set_failed_count(saturating_i32(failed_count));
        self.as_mut()
            .set_selected_count(saturating_i32(selected_count));
        self.as_mut().rebuild_visible();
        self.as_mut().pump_repairs();
    }

    fn finish_repair_batch(mut self: Pin<&mut Self>) {
        if self.as_ref().rust().repair_finalizing {
            return;
        }
        let repaired = self.as_ref().rust().repair_successes;
        let unavailable = self.as_ref().rust().repair_unavailable;
        let failed = self.as_ref().rust().repair_failures;
        let Some(batch_id) = self.as_ref().rust().repair_batch_id.clone() else {
            self.as_mut().stop_repair_and_fail_journal(
                "The durable repair identity disappeared before completion.".to_owned(),
            );
            return;
        };
        let completion_detail = format!(
            "Repair complete: {repaired} cached, {unavailable} unavailable, {failed} failed."
        );
        self.as_mut().rust_mut().repair_finalizing = true;
        self.as_mut().set_phase(qstring("Finalizing repair"));
        let generation = self.as_ref().rust().repair_generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-media-repair-finalize".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| {
                        store.complete_media_repair_batch(&batch_id, &completion_detail)
                    })
                    .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_repair_batch_saved(
                        generation,
                        repaired,
                        unavailable,
                        failed,
                        result,
                    );
                });
            });
        if let Err(error) = spawn {
            self.as_mut().rust_mut().repair_finalizing = false;
            self.as_mut().stop_repair_and_fail_journal(format!(
                "Could not start the repair finalization worker: {error}"
            ));
        }
    }

    fn finish_repair_batch_saved(
        mut self: Pin<&mut Self>,
        generation: u64,
        repaired: usize,
        unavailable: usize,
        failed: usize,
        result: Result<(), String>,
    ) {
        if generation != self.as_ref().rust().repair_generation || !*self.as_ref().repairing() {
            return;
        }
        self.as_mut().rust_mut().repair_finalizing = false;
        if let Err(error) = result {
            self.as_mut().stop_repair_and_fail_journal(format!(
                "Could not finalize the durable artwork repair: {error}"
            ));
            return;
        }
        self.as_mut().rust_mut().repair_queue = None;
        self.as_mut().rust_mut().repair_batch_id = None;
        self.as_mut().rust_mut().repair_lease = None;
        self.as_mut().set_busy(false);
        self.as_mut().set_repairing(false);
        self.as_mut().set_phase(qstring("Repair complete"));
        self.as_mut().set_message(qstring(format!(
            "Repair complete: {repaired} cached, {unavailable} unavailable across configured providers, {failed} failed. Every unresolved record remains available for review or retry."
        )));
        if repaired > 0 {
            let revision = self.as_ref().media_revision().wrapping_add(1);
            self.as_mut().set_media_revision(revision);
        }
        self.as_mut().rebuild_visible();
    }

    fn stop_repair_and_fail_journal(mut self: Pin<&mut Self>, message: String) {
        let batch_id = self.as_ref().rust().repair_batch_id.clone();
        let lease = self.as_mut().rust_mut().repair_lease.take();
        if let (Some(batch_id), Some(lease)) = (batch_id, lease) {
            let failure_detail = message.clone();
            let _ = std::thread::Builder::new()
                .name("lunchbox-media-repair-journal-failure".into())
                .spawn(move || {
                    let _lease = lease;
                    if let Ok(store) = SettingsStore::open_default() {
                        let _ = store.fail_media_repair_batch(&batch_id, &failure_detail);
                    }
                });
        }
        self.as_mut().rust_mut().repair_generation =
            self.as_ref().rust().repair_generation.wrapping_add(1);
        self.as_mut().rust_mut().repair_queue = None;
        self.as_mut().rust_mut().repair_pending.clear();
        self.as_mut().rust_mut().repair_active.clear();
        self.as_mut().rust_mut().repair_pump_busy = false;
        self.as_mut().rust_mut().repair_finalizing = false;
        self.as_mut().rust_mut().repair_batch_id = None;
        self.as_mut().set_busy(false);
        self.as_mut().set_repairing(false);
        self.as_mut().set_phase(qstring("Repair stopped"));
        self.as_mut().set_message(qstring(message));
        self.as_mut().rebuild_visible();
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
    }

    pub fn row_game_uid_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|(_, entry)| qstring(&entry.game_uid))
            .unwrap_or_default()
    }

    pub fn row_database_id_at(&self, index: i32) -> i32 {
        entry_at(self, index)
            .map(|(_, entry)| i32::try_from(entry.launchbox_db_id).unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn row_title_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|(_, entry)| qstring(&entry.title))
            .unwrap_or_default()
    }

    pub fn row_platform_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|(_, entry)| qstring(&entry.platform))
            .unwrap_or_default()
    }

    pub fn row_local_at(&self, index: i32) -> bool {
        entry_at(self, index).is_some_and(|(_, entry)| entry.local)
    }

    pub fn row_downloadable_at(&self, index: i32) -> bool {
        entry_at(self, index).is_some_and(|(_, entry)| entry.downloadable)
    }

    pub fn row_kind_label_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|(_, entry)| qstring(entry.kind.label()))
            .unwrap_or_default()
    }

    pub fn row_status_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|(source_index, _)| qstring(source_status(self, source_index)))
            .unwrap_or_default()
    }

    pub fn row_status_label_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|(source_index, _)| qstring(status_label(source_status(self, source_index))))
            .unwrap_or_default()
    }

    pub fn row_detail_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|(source_index, entry)| {
                if let Some(error) = self.rust().errors.get(&source_index) {
                    qstring(format!("Repair failed: {error}"))
                } else if self.rust().repaired.contains(&source_index) {
                    qstring(format!(
                        "Exact {} was validated and cached from a configured provider.",
                        entry.kind.label().to_lowercase()
                    ))
                } else if self.rust().unavailable.contains(&source_index) {
                    qstring(format!(
                        "No configured provider returned an exact {} for this reviewed title and platform.",
                        entry.kind.label().to_lowercase()
                    ))
                } else {
                    qstring(&entry.detail)
                }
            })
            .unwrap_or_default()
    }

    pub fn row_repairable_at(&self, index: i32) -> bool {
        entry_at(self, index).is_some_and(|(source_index, entry)| {
            entry.repairable && !self.rust().repaired.contains(&source_index)
        })
    }

    pub fn row_selected_at(&self, index: i32) -> bool {
        entry_at(self, index)
            .is_some_and(|(source_index, _)| self.rust().selected.contains(&source_index))
    }

    pub fn report_ui_probe(&self) {
        if self.rust().ui_probe {
            println!(
                "LUNCHBOX_MEDIA_AUDIT_UI_READY examined={} covered={} missing={} repairable={} manual={} rows={}",
                self.examined_count(),
                self.covered_count(),
                self.missing_count(),
                self.repairable_count(),
                self.manual_review_count(),
                self.entry_count(),
            );
        }
    }

    pub fn verify_recovery_probe(&self, dialog_visible: bool) -> bool {
        let summary = self.recovery_summary().to_string();
        let ready = dialog_visible
            && *self.initialized()
            && *self.recovery_available()
            && *self.recovery_count() == 1
            && self.rust().recovered_running_items == 1
            && *self.entry_count() == 1
            && self.row_title_at(0).to_string() == "Super Mario Bros."
            && summary.contains("interrupted");
        let evidence = format!(
            "visible={dialog_visible} initialized={} recovery={} count={} recovered_running={} rows={} title={:?}",
            *self.initialized(),
            *self.recovery_available(),
            *self.recovery_count(),
            self.rust().recovered_running_items,
            self.entry_count(),
            self.row_title_at(0).to_string(),
        );
        if ready {
            println!("LUNCHBOX_MEDIA_REPAIR_RECOVERY_UI_READY {evidence}");
        } else {
            eprintln!(
                "LUNCHBOX_MEDIA_REPAIR_RECOVERY_UI_FAILED {evidence} summary={summary:?} message={:?}",
                self.message().to_string()
            );
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::{status_label, status_rank};

    #[test]
    fn media_audit_statuses_have_stable_labels_and_priority() {
        assert_eq!(status_label("missing"), "MISSING");
        assert_eq!(status_label("review"), "REVIEW");
        assert_eq!(status_label("unavailable"), "NOT FOUND");
        assert!(status_rank("failed") < status_rank("missing"));
        assert!(status_rank("missing") < status_rank("repaired"));
    }
}
