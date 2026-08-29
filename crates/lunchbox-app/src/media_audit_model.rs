#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
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
        type MediaAuditModel = super::MediaAuditModelRust;

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
use crate::media::{MediaFetchOutcome, MediaFetchQueue, MediaFetchRequest};
use crate::media_audit::{
    MediaAuditOutput, MediaAuditProgress, MissingMediaEntry, audit_missing_media,
    validate_audit_request,
};

const MAX_INFLIGHT_REPAIRS: usize = 8;

pub struct MediaAuditModelRust {
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
    repair_successes: usize,
    ui_probe: bool,
}

impl Default for MediaAuditModelRust {
    fn default() -> Self {
        Self {
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
            repair_successes: 0,
            ui_probe: std::env::args().any(|argument| argument == "--media-audit-ui-probe"),
        }
    }
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

impl qobject::MediaAuditModel {
    pub fn start_audit(mut self: Pin<&mut Self>, scope: QString, artwork_kind: QString) {
        if *self.as_ref().busy() {
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
            let completed = *self.as_ref().repair_completed();
            let total = *self.as_ref().repair_total();
            let repaired = self.as_ref().rust().repair_successes;
            let generation = self.as_ref().rust().repair_generation.wrapping_add(1);
            self.as_mut().rust_mut().repair_generation = generation;
            self.as_mut().rust_mut().repair_queue = None;
            self.as_mut().rust_mut().repair_pending.clear();
            self.as_mut().rust_mut().repair_active.clear();
            self.as_mut().set_busy(false);
            self.as_mut().set_repairing(false);
            self.as_mut().set_phase(qstring("Repair cancelled"));
            self.as_mut().set_message(qstring(format!(
                "Repair cancelled after {} of {} selected records. Completed files were kept.",
                completed, total
            )));
            if repaired > 0 {
                let revision = self.as_ref().media_revision().wrapping_add(1);
                self.as_mut().set_media_revision(revision);
            }
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
        if *self.as_ref().busy() || self.as_ref().rust().selected.is_empty() {
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

        self.as_mut().rust_mut().repair_generation =
            self.as_ref().rust().repair_generation.wrapping_add(1);
        let generation = self.as_ref().rust().repair_generation;
        let qt_thread = self.as_ref().qt_thread();
        let queue = match MediaFetchQueue::start(
            crate::media::requested_media_directory(),
            move |outcome| {
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_repair(generation, outcome);
                });
            },
        ) {
            Ok(queue) => queue,
            Err(error) => {
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
        self.as_mut().rust_mut().repair_pending = selected.iter().copied().collect();
        self.as_mut().rust_mut().repair_active.clear();
        self.as_mut().rust_mut().repair_successes = 0;
        self.as_mut()
            .set_repair_total(saturating_i32(selected.len()));
        self.as_mut().set_repair_completed(0);
        self.as_mut().set_progress_current(0);
        self.as_mut()
            .set_progress_total(saturating_i32(selected.len()));
        self.as_mut().set_busy(true);
        self.as_mut().set_repairing(true);
        self.as_mut()
            .set_phase(qstring("Repairing selected artwork"));
        self.as_mut().set_message(qstring(format!(
            "Repairing {} selected records with two bounded background workers…",
            selected.len()
        )));
        self.as_mut().pump_repairs();
    }

    fn pump_repairs(mut self: Pin<&mut Self>) {
        while self.as_ref().rust().repair_active.len() < MAX_INFLIGHT_REPAIRS {
            let Some(source_index) = self.as_mut().rust_mut().repair_pending.pop_front() else {
                break;
            };
            let Some(entry) = self
                .as_ref()
                .rust()
                .output
                .as_ref()
                .and_then(|output| output.entries.get(source_index))
                .cloned()
            else {
                continue;
            };
            let request = MediaFetchRequest {
                request_id: entry.game_uid.clone(),
                database_id: entry.launchbox_db_id,
                title: entry.title,
                platform: entry.platform,
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
            match queued {
                Ok(()) => {
                    self.as_mut()
                        .rust_mut()
                        .repair_active
                        .insert(entry.game_uid, source_index);
                }
                Err(error) => {
                    self.as_mut()
                        .rust_mut()
                        .errors
                        .insert(source_index, error.to_string());
                    let complete = self.as_ref().repair_completed().saturating_add(1);
                    self.as_mut().set_repair_completed(complete);
                }
            }
        }
        if self.as_ref().rust().repair_pending.is_empty()
            && self.as_ref().rust().repair_active.is_empty()
        {
            self.as_mut().finish_repair_batch();
        }
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
        let Some(source_index) = self.as_mut().rust_mut().repair_active.remove(&request_id) else {
            return;
        };
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
                self.as_mut().rust_mut().errors.insert(
                    source_index,
                    "The provider returned a different media category; no repair was accepted."
                        .to_owned(),
                );
            }
            MediaFetchOutcome::Missing { .. } => {
                self.as_mut().rust_mut().unavailable.insert(source_index);
            }
            MediaFetchOutcome::Failed { error, .. } => {
                self.as_mut().rust_mut().errors.insert(source_index, error);
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
        let repaired = self.as_ref().rust().repair_successes;
        let unavailable = self.as_ref().rust().unavailable.len();
        let failed = self.as_ref().rust().errors.len();
        self.as_mut().rust_mut().repair_queue = None;
        self.as_mut().set_busy(false);
        self.as_mut().set_repairing(false);
        self.as_mut().set_phase(qstring("Repair complete"));
        self.as_mut().set_message(qstring(format!(
            "Repair complete: {repaired} cached, {unavailable} not found at LibRetro, {failed} failed. Every unresolved record remains available for review or retry."
        )));
        if repaired > 0 {
            let revision = self.as_ref().media_revision().wrapping_add(1);
            self.as_mut().set_media_revision(revision);
        }
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
                        "Exact {} was validated and cached from LibRetro.",
                        entry.kind.label().to_lowercase()
                    ))
                } else if self.rust().unavailable.contains(&source_index) {
                    qstring(format!(
                        "LibRetro has no exact {} for this reviewed title and platform.",
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
