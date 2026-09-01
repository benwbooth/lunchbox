#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qurl.h");
        type QString = cxx_qt_lib::QString;
        type QUrl = cxx_qt_lib::QUrl;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, busy)]
        #[qproperty(bool, auditing)]
        #[qproperty(bool, removing)]
        #[qproperty(QString, message)]
        #[qproperty(QString, current_root)]
        #[qproperty(QString, current_file)]
        #[qproperty(i32, total_roots)]
        #[qproperty(i32, completed_roots)]
        #[qproperty(i32, root_total_files)]
        #[qproperty(i32, root_scanned_files)]
        #[qproperty(i32, total_entry_count)]
        #[qproperty(i32, entry_count)]
        #[qproperty(i32, healthy_count)]
        #[qproperty(i32, issue_count)]
        #[qproperty(i32, missing_count)]
        #[qproperty(i32, changed_count)]
        #[qproperty(i32, duplicate_count)]
        #[qproperty(i32, untracked_count)]
        #[qproperty(i32, unavailable_count)]
        #[qproperty(i32, unreadable_count)]
        #[qproperty(i32, history_count)]
        #[qproperty(bool, comparison_available)]
        #[qproperty(i32, issue_delta)]
        #[qproperty(QString, comparison_summary)]
        #[qproperty(i32, selected_missing_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, collection_revision)]
        type LibraryAuditModel = super::LibraryAuditModelRust;

        #[qinvokable]
        fn start_audit(self: Pin<&mut LibraryAuditModel>);

        #[qinvokable]
        fn cancel_audit(self: Pin<&mut LibraryAuditModel>);

        #[qinvokable]
        fn apply_filter(
            self: Pin<&mut LibraryAuditModel>,
            search: QString,
            status: QString,
            sort: QString,
        );

        #[qinvokable]
        fn toggle_missing_selected(self: Pin<&mut LibraryAuditModel>, index: i32);

        #[qinvokable]
        fn select_visible_missing(self: Pin<&mut LibraryAuditModel>, selected: bool);

        #[qinvokable]
        fn remove_selected_missing(self: Pin<&mut LibraryAuditModel>);

        #[qinvokable]
        fn entry_title_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_file_name_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_archive_member_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_platform_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_path_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_root_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_status_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_status_label_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_detail_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn entry_removable_at(self: &LibraryAuditModel, index: i32) -> bool;

        #[qinvokable]
        fn entry_selected_at(self: &LibraryAuditModel, index: i32) -> bool;

        #[qinvokable]
        fn entry_file_url_at(self: &LibraryAuditModel, index: i32) -> QUrl;

        #[qinvokable]
        fn entry_root_url_at(self: &LibraryAuditModel, index: i32) -> QUrl;

        #[qinvokable]
        fn history_finished_at(self: &LibraryAuditModel, index: i32) -> i64;

        #[qinvokable]
        fn history_total_count_at(self: &LibraryAuditModel, index: i32) -> i32;

        #[qinvokable]
        fn history_issue_count_at(self: &LibraryAuditModel, index: i32) -> i32;

        #[qinvokable]
        fn history_summary_at(self: &LibraryAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn history_comparison_at(self: &LibraryAuditModel, index: i32) -> QString;
    }

    impl cxx_qt::Threading for LibraryAuditModel {}
}

use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::catalog;
use crate::library_audit::{
    self, LibraryAuditEntry, LibraryAuditOutput, LibraryAuditProgress, LibraryAuditStatus,
};

pub struct LibraryAuditModelRust {
    busy: bool,
    auditing: bool,
    removing: bool,
    message: QString,
    current_root: QString,
    current_file: QString,
    total_roots: i32,
    completed_roots: i32,
    root_total_files: i32,
    root_scanned_files: i32,
    total_entry_count: i32,
    entry_count: i32,
    healthy_count: i32,
    issue_count: i32,
    missing_count: i32,
    changed_count: i32,
    duplicate_count: i32,
    untracked_count: i32,
    unavailable_count: i32,
    unreadable_count: i32,
    history_count: i32,
    comparison_available: bool,
    issue_delta: i32,
    comparison_summary: QString,
    selected_missing_count: i32,
    revision: i32,
    collection_revision: i32,
    output: Option<Arc<LibraryAuditOutput>>,
    visible_indices: Vec<usize>,
    selected_missing: HashSet<String>,
    search: String,
    status: String,
    sort: String,
    cancel: Option<Arc<AtomicBool>>,
    generation: u64,
    pending_notice: String,
}

impl Default for LibraryAuditModelRust {
    fn default() -> Self {
        Self {
            busy: false,
            auditing: false,
            removing: false,
            message: QString::from(
                "Run an exact audit to verify tracked ROMs and discover maintenance issues.",
            ),
            current_root: QString::default(),
            current_file: QString::default(),
            total_roots: 0,
            completed_roots: 0,
            root_total_files: 0,
            root_scanned_files: 0,
            total_entry_count: 0,
            entry_count: 0,
            healthy_count: 0,
            issue_count: 0,
            missing_count: 0,
            changed_count: 0,
            duplicate_count: 0,
            untracked_count: 0,
            unavailable_count: 0,
            unreadable_count: 0,
            history_count: 0,
            comparison_available: false,
            issue_delta: 0,
            comparison_summary: qstring(
                "Run the audit twice to compare maintenance trends over time.",
            ),
            selected_missing_count: 0,
            revision: 0,
            collection_revision: 0,
            output: None,
            visible_indices: Vec::new(),
            selected_missing: HashSet::new(),
            search: String::new(),
            status: "issues".to_owned(),
            sort: "status".to_owned(),
            cancel: None,
            generation: 0,
            pending_notice: String::new(),
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

impl qobject::LibraryAuditModel {
    pub fn start_audit(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(discovery_path) = catalog::requested_discovery_database_path() else {
            self.as_mut().set_message(qstring(
                "The discovery games database is required for an exact library audit.",
            ));
            return;
        };
        let state_path = match crate::settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut()
                    .set_message(qstring(format!("Could not locate Lunchbox state: {error}")));
                return;
            }
        };

        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().cancel = Some(Arc::clone(&cancel));
        self.as_mut().set_busy(true);
        self.as_mut().set_auditing(true);
        self.as_mut().set_removing(false);
        self.as_mut().set_total_roots(0);
        self.as_mut().set_completed_roots(0);
        self.as_mut().set_root_total_files(0);
        self.as_mut().set_root_scanned_files(0);
        self.as_mut().set_current_root(QString::default());
        self.as_mut().set_current_file(QString::default());
        self.as_mut().set_message(qstring(
            "Preparing one exact index for every collection root…",
        ));

        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress: Arc<dyn Fn(LibraryAuditProgress) + Send + Sync> = Arc::new(move |progress| {
            let _ = progress_thread.queue(move |mut model| {
                model.as_mut().update_progress(generation, progress);
            });
        });
        let spawn = std::thread::Builder::new()
            .name("lunchbox-library-audit".into())
            .spawn(move || {
                let result = library_audit::audit_local_collection(
                    &discovery_path,
                    &state_path,
                    &cancel,
                    progress,
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_audit(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_auditing(false);
            self.as_mut().rust_mut().cancel = None;
            self.as_mut()
                .set_message(qstring(format!("Could not start library audit: {error}")));
        }
    }

    pub fn cancel_audit(mut self: Pin<&mut Self>) {
        if let Some(cancel) = self.as_ref().rust().cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
            self.as_mut()
                .set_message(qstring("Cancelling after the current content read…"));
        }
    }

    fn update_progress(mut self: Pin<&mut Self>, generation: u64, progress: LibraryAuditProgress) {
        if generation != self.as_ref().rust().generation || !*self.as_ref().auditing() {
            return;
        }
        self.as_mut()
            .set_total_roots(saturating_i32(progress.total_roots));
        self.as_mut()
            .set_completed_roots(saturating_i32(progress.completed_roots));
        self.as_mut()
            .set_current_root(qstring(progress.current_root));
        self.as_mut()
            .set_current_file(qstring(progress.current_file));
        self.as_mut()
            .set_root_total_files(saturating_i32(progress.root_total_files));
        self.as_mut()
            .set_root_scanned_files(saturating_i32(progress.root_scanned_files));
        if progress.total_roots > 0 {
            self.as_mut().set_message(qstring(format!(
                "Verifying root {} of {} · file {} of {}",
                progress
                    .completed_roots
                    .saturating_add(1)
                    .min(progress.total_roots),
                progress.total_roots,
                progress.root_scanned_files,
                progress.root_total_files
            )));
        }
    }

    fn finish_audit(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<LibraryAuditOutput, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().rust_mut().cancel = None;
        self.as_mut().set_current_root(QString::default());
        self.as_mut().set_current_file(QString::default());
        match result {
            Ok(output) => {
                let availability_changes = output.availability_changes;
                let warning_count = output.warnings.len();
                let cancelled = output.cancelled;
                let root_count = output.root_count;
                let scanned_root_count = output.scanned_root_count;
                let total_entries = output.entries.len();
                let issue_count = output.issue_count();
                self.as_mut().set_total_roots(saturating_i32(root_count));
                self.as_mut()
                    .set_completed_roots(saturating_i32(scanned_root_count));
                self.as_mut()
                    .set_total_entry_count(saturating_i32(total_entries));
                self.as_mut()
                    .set_healthy_count(saturating_i32(output.healthy_count));
                self.as_mut().set_issue_count(saturating_i32(issue_count));
                self.as_mut()
                    .set_missing_count(saturating_i32(output.missing_count));
                self.as_mut()
                    .set_changed_count(saturating_i32(output.changed_count));
                self.as_mut()
                    .set_duplicate_count(saturating_i32(output.duplicate_count));
                self.as_mut()
                    .set_untracked_count(saturating_i32(output.untracked_count));
                self.as_mut()
                    .set_unavailable_count(saturating_i32(output.unavailable_count));
                self.as_mut()
                    .set_unreadable_count(saturating_i32(output.unreadable_count));
                self.as_mut()
                    .set_history_count(saturating_i32(output.history.len()));
                if let (Some(current), Some(previous)) =
                    (output.history.first(), output.history.get(1))
                {
                    let issue_delta = signed_delta(current.issue_count(), previous.issue_count());
                    self.as_mut().set_comparison_available(true);
                    self.as_mut().set_issue_delta(issue_delta);
                    self.as_mut()
                        .set_comparison_summary(qstring(comparison_sentence(current, previous)));
                } else {
                    self.as_mut().set_comparison_available(false);
                    self.as_mut().set_issue_delta(0);
                    self.as_mut().set_comparison_summary(qstring(
                        "This is the first retained complete audit; the next run will show what changed.",
                    ));
                }
                self.as_mut().rust_mut().selected_missing.clear();
                self.as_mut().set_selected_missing_count(0);
                self.as_mut().rust_mut().output = Some(Arc::new(output));
                self.as_mut().rebuild_visible();

                let notice = std::mem::take(&mut self.as_mut().rust_mut().pending_notice);
                let summary = if root_count == 0 {
                    "No imported ROM roots yet. Use Import ROMs to add a local collection."
                        .to_owned()
                } else if cancelled {
                    format!(
                        "Audit cancelled after {scanned_root_count} of {root_count} complete roots; partial roots were discarded."
                    )
                } else if issue_count == 0 {
                    format!("All {total_entries} tracked ROMs passed exact verification.")
                } else {
                    format!(
                        "Found {issue_count} maintenance issues across {root_count} roots. Nothing was merged, relinked, or deleted automatically."
                    )
                };
                self.as_mut().set_message(qstring(format!(
                    "{}{}{}",
                    if notice.is_empty() {
                        String::new()
                    } else {
                        format!("{notice} ")
                    },
                    summary,
                    if warning_count == 0 {
                        String::new()
                    } else {
                        format!(" {warning_count} roots need attention.")
                    }
                )));
                if availability_changes > 0 {
                    let revision = self.as_ref().collection_revision().wrapping_add(1);
                    self.as_mut().set_collection_revision(revision);
                }
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Library audit failed: {error}"))),
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
        let mut indices = self
            .as_ref()
            .rust()
            .output
            .as_ref()
            .map(|output| {
                output
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| {
                        (search.is_empty()
                            || entry.title.to_lowercase().contains(&search)
                            || entry.file_name.to_lowercase().contains(&search)
                            || entry.archive_member.to_lowercase().contains(&search)
                            || entry.platform.to_lowercase().contains(&search)
                            || entry
                                .path
                                .to_string_lossy()
                                .to_lowercase()
                                .contains(&search)
                            || entry.detail.to_lowercase().contains(&search))
                            && match status.as_str() {
                                "all" => true,
                                "issues" => entry.status != LibraryAuditStatus::Healthy,
                                value => entry.status.key() == value,
                            }
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if let Some(output) = self.as_ref().rust().output.as_ref() {
            indices.sort_by(|left, right| {
                let left = &output.entries[*left];
                let right = &output.entries[*right];
                match sort.as_str() {
                    "title" => left
                        .title
                        .to_lowercase()
                        .cmp(&right.title.to_lowercase())
                        .then_with(|| left.path.cmp(&right.path)),
                    "root" => left
                        .root_path
                        .cmp(&right.root_path)
                        .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase())),
                    "path" => left.path.cmp(&right.path),
                    _ => status_rank(left.status)
                        .cmp(&status_rank(right.status))
                        .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase())),
                }
            });
        }
        let count = indices.len();
        self.as_mut().rust_mut().visible_indices = indices;
        self.as_mut().set_entry_count(saturating_i32(count));
        self.as_mut().bump_revision();
    }

    pub fn toggle_missing_selected(mut self: Pin<&mut Self>, index: i32) {
        let Some(entry) = self.as_ref().visible_entry(index).cloned() else {
            return;
        };
        if !entry.removable() {
            return;
        }
        let selected = &mut self.as_mut().rust_mut().selected_missing;
        if !selected.insert(entry.file_id.clone()) {
            selected.remove(&entry.file_id);
        }
        self.as_mut().update_selected_count();
        self.as_mut().bump_revision();
    }

    pub fn select_visible_missing(mut self: Pin<&mut Self>, selected: bool) {
        let file_ids = {
            let model = self.as_ref();
            let Some(output) = model.rust().output.as_ref() else {
                return;
            };
            model
                .rust()
                .visible_indices
                .iter()
                .filter_map(|index| output.entries.get(*index))
                .filter(|entry| entry.removable())
                .map(|entry| entry.file_id.clone())
                .collect::<Vec<_>>()
        };
        let selected_ids = &mut self.as_mut().rust_mut().selected_missing;
        for file_id in file_ids {
            if selected {
                selected_ids.insert(file_id);
            } else {
                selected_ids.remove(&file_id);
            }
        }
        self.as_mut().update_selected_count();
        self.as_mut().bump_revision();
    }

    pub fn remove_selected_missing(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let file_ids = self
            .as_ref()
            .rust()
            .selected_missing
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if file_ids.is_empty() {
            self.as_mut()
                .set_message(qstring("Select at least one missing record to remove."));
            return;
        }
        let state_path = match crate::settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut()
                    .set_message(qstring(format!("Could not locate Lunchbox state: {error}")));
                return;
            }
        };
        let selected = file_ids.len();
        self.as_mut().set_busy(true);
        self.as_mut().set_removing(true);
        self.as_mut().set_message(qstring(format!(
            "Removing {selected} missing records from Lunchbox state…"
        )));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-library-audit-cleanup".into())
            .spawn(move || {
                let result = library_audit::remove_missing_records(&state_path, &file_ids)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_remove(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_removing(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start missing-record cleanup: {error}"
            )));
        }
    }

    fn finish_remove(mut self: Pin<&mut Self>, result: Result<usize, String>) {
        self.as_mut().set_busy(false);
        self.as_mut().set_removing(false);
        match result {
            Ok(removed) => {
                self.as_mut().rust_mut().pending_notice = format!(
                    "Removed {removed} stale {} from Lunchbox state; no ROM data was touched.",
                    if removed == 1 { "record" } else { "records" }
                );
                self.as_mut().rust_mut().selected_missing.clear();
                self.as_mut().set_selected_missing_count(0);
                let revision = self.as_ref().collection_revision().wrapping_add(1);
                self.as_mut().set_collection_revision(revision);
                self.as_mut().start_audit();
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Missing-record cleanup failed: {error}"))),
        }
    }

    pub fn entry_title_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(&entry.title))
            .unwrap_or_default()
    }

    pub fn entry_file_name_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(&entry.file_name))
            .unwrap_or_default()
    }

    pub fn entry_archive_member_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(&entry.archive_member))
            .unwrap_or_default()
    }

    pub fn entry_platform_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(&entry.platform))
            .unwrap_or_default()
    }

    pub fn entry_path_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(entry.path.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn entry_root_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(entry.root_path.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn entry_status_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(entry.status.key()))
            .unwrap_or_default()
    }

    pub fn entry_status_label_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(entry.status.label()))
            .unwrap_or_default()
    }

    pub fn entry_detail_at(&self, index: i32) -> QString {
        self.visible_entry(index)
            .map(|entry| qstring(&entry.detail))
            .unwrap_or_default()
    }

    pub fn entry_removable_at(&self, index: i32) -> bool {
        self.visible_entry(index)
            .is_some_and(LibraryAuditEntry::removable)
    }

    pub fn entry_selected_at(&self, index: i32) -> bool {
        self.visible_entry(index)
            .is_some_and(|entry| self.rust().selected_missing.contains(&entry.file_id))
    }

    pub fn entry_file_url_at(&self, index: i32) -> QUrl {
        self.visible_entry(index)
            .map(|entry| QUrl::from_local_file(&qstring(entry.path.to_string_lossy())))
            .unwrap_or_default()
    }

    pub fn entry_root_url_at(&self, index: i32) -> QUrl {
        self.visible_entry(index)
            .map(|entry| QUrl::from_local_file(&qstring(entry.root_path.to_string_lossy())))
            .unwrap_or_default()
    }

    pub fn history_finished_at(&self, index: i32) -> i64 {
        self.history_run(index)
            .map(|run| run.finished_at)
            .unwrap_or_default()
    }

    pub fn history_total_count_at(&self, index: i32) -> i32 {
        self.history_run(index)
            .map(|run| saturating_i32(run.total_entry_count))
            .unwrap_or_default()
    }

    pub fn history_issue_count_at(&self, index: i32) -> i32 {
        self.history_run(index)
            .map(|run| saturating_i32(run.issue_count()))
            .unwrap_or_default()
    }

    pub fn history_summary_at(&self, index: i32) -> QString {
        self.history_run(index)
            .map(|run| {
                qstring(format!(
                    "{} healthy · {} missing · {} changed · {} duplicate · {} new · {} offline",
                    run.healthy_count,
                    run.missing_count,
                    run.changed_count.saturating_add(run.unreadable_count),
                    run.duplicate_count,
                    run.untracked_count,
                    run.unavailable_count,
                ))
            })
            .unwrap_or_default()
    }

    pub fn history_comparison_at(&self, index: i32) -> QString {
        let Ok(index) = usize::try_from(index) else {
            return QString::default();
        };
        let Some(output) = self.rust().output.as_ref() else {
            return QString::default();
        };
        let Some(current) = output.history.get(index) else {
            return QString::default();
        };
        output
            .history
            .get(index.saturating_add(1))
            .map(|previous| qstring(comparison_sentence(current, previous)))
            .unwrap_or_else(|| qstring("Oldest retained baseline"))
    }

    fn visible_entry(&self, index: i32) -> Option<&LibraryAuditEntry> {
        let index = usize::try_from(index).ok()?;
        let output_index = *self.rust().visible_indices.get(index)?;
        self.rust().output.as_ref()?.entries.get(output_index)
    }

    fn history_run(&self, index: i32) -> Option<&library_audit::LibraryAuditRun> {
        let index = usize::try_from(index).ok()?;
        self.rust().output.as_ref()?.history.get(index)
    }

    fn update_selected_count(mut self: Pin<&mut Self>) {
        let count = self.as_ref().rust().selected_missing.len();
        self.as_mut()
            .set_selected_missing_count(saturating_i32(count));
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
    }
}

fn signed_delta(current: usize, previous: usize) -> i32 {
    let current = i64::try_from(current).unwrap_or(i64::MAX);
    let previous = i64::try_from(previous).unwrap_or(i64::MAX);
    i32::try_from(current.saturating_sub(previous)).unwrap_or_else(|_| {
        if current >= previous {
            i32::MAX
        } else {
            i32::MIN
        }
    })
}

fn comparison_sentence(
    current: &library_audit::LibraryAuditRun,
    previous: &library_audit::LibraryAuditRun,
) -> String {
    let delta = signed_delta(current.issue_count(), previous.issue_count());
    let trend = match delta.cmp(&0) {
        std::cmp::Ordering::Less => format!(
            "{} fewer maintenance {} than the previous complete audit",
            delta.unsigned_abs(),
            if delta == -1 { "issue" } else { "issues" }
        ),
        std::cmp::Ordering::Greater => format!(
            "{} more maintenance {} than the previous complete audit",
            delta,
            if delta == 1 { "issue" } else { "issues" }
        ),
        std::cmp::Ordering::Equal => {
            "No net change in maintenance issues since the previous complete audit".to_owned()
        }
    };
    format!(
        "{trend} · missing {:+} · changed {:+} · duplicate {:+} · new {:+} · offline {:+}",
        signed_delta(current.missing_count, previous.missing_count),
        signed_delta(
            current
                .changed_count
                .saturating_add(current.unreadable_count),
            previous
                .changed_count
                .saturating_add(previous.unreadable_count),
        ),
        signed_delta(current.duplicate_count, previous.duplicate_count),
        signed_delta(current.untracked_count, previous.untracked_count),
        signed_delta(current.unavailable_count, previous.unavailable_count),
    )
}

fn status_rank(status: LibraryAuditStatus) -> u8 {
    match status {
        LibraryAuditStatus::Unavailable => 0,
        LibraryAuditStatus::Missing => 1,
        LibraryAuditStatus::Unreadable => 2,
        LibraryAuditStatus::Changed => 3,
        LibraryAuditStatus::Duplicate => 4,
        LibraryAuditStatus::Untracked => 5,
        LibraryAuditStatus::Healthy => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(issue_count: usize) -> library_audit::LibraryAuditRun {
        library_audit::LibraryAuditRun {
            id: uuid::Uuid::new_v4().to_string(),
            finished_at: 1,
            root_count: 1,
            scanned_root_count: 1,
            total_entry_count: 20,
            healthy_count: 20usize.saturating_sub(issue_count),
            missing_count: issue_count,
            changed_count: 0,
            duplicate_count: 0,
            untracked_count: 0,
            unavailable_count: 0,
            unreadable_count: 0,
            availability_changes: 0,
            warning_count: 0,
        }
    }

    #[test]
    fn audit_comparison_reports_better_equal_and_worse_trends_without_losing_signs() {
        let better = comparison_sentence(&run(2), &run(5));
        assert!(better.starts_with("3 fewer maintenance issues"));
        assert!(better.contains("missing -3"));

        let equal = comparison_sentence(&run(5), &run(5));
        assert!(equal.starts_with("No net change"));
        assert!(equal.contains("missing +0"));

        let worse = comparison_sentence(&run(7), &run(5));
        assert!(worse.starts_with("2 more maintenance issues"));
        assert!(worse.contains("missing +2"));
    }
}
