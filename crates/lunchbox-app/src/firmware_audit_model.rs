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
        #[qproperty(QString, message)]
        #[qproperty(QString, current_platform)]
        #[qproperty(i32, progress_current)]
        #[qproperty(i32, progress_total)]
        #[qproperty(i32, game_count)]
        #[qproperty(i32, platform_count)]
        #[qproperty(i32, no_firmware_platform_count)]
        #[qproperty(i32, missing_count)]
        #[qproperty(i32, manual_count)]
        #[qproperty(i32, optional_count)]
        #[qproperty(i32, ready_count)]
        #[qproperty(i32, no_runtime_count)]
        #[qproperty(i32, error_count)]
        #[qproperty(i32, unavailable_game_count)]
        #[qproperty(i32, entry_count)]
        #[qproperty(i32, revision)]
        type FirmwareAuditModel = super::FirmwareAuditModelRust;

        #[qinvokable]
        fn start_audit(self: Pin<&mut FirmwareAuditModel>);

        #[qinvokable]
        fn cancel(self: Pin<&mut FirmwareAuditModel>);

        #[qinvokable]
        fn apply_filter(
            self: Pin<&mut FirmwareAuditModel>,
            search: QString,
            status: QString,
            sort: QString,
        );

        #[qinvokable]
        fn row_game_uid_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_database_id_at(self: &FirmwareAuditModel, index: i32) -> i32;

        #[qinvokable]
        fn row_title_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_platform_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_emulator_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_package_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_source_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_status_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_status_label_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn row_detail_at(self: &FirmwareAuditModel, index: i32) -> QString;

        #[qinvokable]
        fn report_ui_probe(self: &FirmwareAuditModel);
    }

    impl cxx_qt::Threading for FirmwareAuditModel {}
}

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::firmware_audit::{FirmwareAuditEntry, FirmwareAuditOutput, FirmwareAuditProgress};

pub struct FirmwareAuditModelRust {
    busy: bool,
    message: QString,
    current_platform: QString,
    progress_current: i32,
    progress_total: i32,
    game_count: i32,
    platform_count: i32,
    no_firmware_platform_count: i32,
    missing_count: i32,
    manual_count: i32,
    optional_count: i32,
    ready_count: i32,
    no_runtime_count: i32,
    error_count: i32,
    unavailable_game_count: i32,
    entry_count: i32,
    revision: i32,
    output: Option<Arc<FirmwareAuditOutput>>,
    visible_indices: Vec<usize>,
    search: String,
    status: String,
    sort: String,
    cancel: Option<Arc<AtomicBool>>,
    generation: u64,
    ui_probe: bool,
}

impl Default for FirmwareAuditModelRust {
    fn default() -> Self {
        Self {
            busy: false,
            message: QString::from(
                "Scan My Collection to verify shared BIOS and firmware for each selected emulator.",
            ),
            current_platform: QString::default(),
            progress_current: 0,
            progress_total: 0,
            game_count: 0,
            platform_count: 0,
            no_firmware_platform_count: 0,
            missing_count: 0,
            manual_count: 0,
            optional_count: 0,
            ready_count: 0,
            no_runtime_count: 0,
            error_count: 0,
            unavailable_game_count: 0,
            entry_count: 0,
            revision: 0,
            output: None,
            visible_indices: Vec::new(),
            search: String::new(),
            status: "issues".to_owned(),
            sort: "status".to_owned(),
            cancel: None,
            generation: 0,
            ui_probe: std::env::args().any(|argument| argument == "--firmware-audit-ui-probe"),
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn human_join(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => format!(
            "{}, and {}",
            items[..items.len() - 1].join(", "),
            items.last().expect("non-empty issue summary")
        ),
    }
}

fn entry_at(model: &qobject::FirmwareAuditModel, index: i32) -> Option<&FirmwareAuditEntry> {
    let visible_index = usize::try_from(index).ok()?;
    let output_index = *model.rust().visible_indices.get(visible_index)?;
    model.rust().output.as_ref()?.entries.get(output_index)
}

fn filtered_indices(
    output: &FirmwareAuditOutput,
    search: &str,
    status: &str,
    sort: &str,
) -> Vec<usize> {
    let mut indices = output
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            let status_match = match status {
                "all" => true,
                "issues" => !matches!(
                    entry.status,
                    crate::firmware_audit::FirmwareAuditStatus::Ready
                        | crate::firmware_audit::FirmwareAuditStatus::Optional
                ),
                key => entry.status.key() == key,
            };
            status_match
                && (search.is_empty()
                    || entry.title.to_lowercase().contains(search)
                    || entry.platform.to_lowercase().contains(search)
                    || entry.emulator.to_lowercase().contains(search)
                    || entry.package.to_lowercase().contains(search)
                    || entry.source.to_lowercase().contains(search))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        let left = &output.entries[*left];
        let right = &output.entries[*right];
        match sort {
            "platform" => left
                .platform
                .to_lowercase()
                .cmp(&right.platform.to_lowercase())
                .then_with(|| {
                    left.package
                        .to_lowercase()
                        .cmp(&right.package.to_lowercase())
                }),
            "package" => left
                .package
                .to_lowercase()
                .cmp(&right.package.to_lowercase())
                .then_with(|| {
                    left.platform
                        .to_lowercase()
                        .cmp(&right.platform.to_lowercase())
                }),
            _ => left
                .status
                .severity()
                .cmp(&right.status.severity())
                .then_with(|| {
                    left.platform
                        .to_lowercase()
                        .cmp(&right.platform.to_lowercase())
                })
                .then_with(|| {
                    left.package
                        .to_lowercase()
                        .cmp(&right.package.to_lowercase())
                }),
        }
    });
    indices
}

impl qobject::FirmwareAuditModel {
    pub fn start_audit(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(catalog_database) = crate::catalog::requested_database_path() else {
            self.as_mut().set_message(qstring(
                "The canonical Lunchbox database is required for firmware and emulator rules.",
            ));
            return;
        };
        let state_database = match crate::settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_message(qstring(format!(
                    "Could not locate the local collection state: {error}"
                )));
                return;
            }
        };

        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().cancel = Some(Arc::clone(&cancel));
        self.as_mut().rust_mut().output = None;
        self.as_mut().rust_mut().visible_indices.clear();
        self.as_mut().set_entry_count(0);
        self.as_mut().set_progress_current(0);
        self.as_mut().set_progress_total(0);
        self.as_mut().set_current_platform(QString::default());
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Inspecting the installed emulator selected for every local platform…",
        ));

        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-firmware-audit".into())
            .spawn(move || {
                let progress: Arc<dyn Fn(FirmwareAuditProgress) + Send + Sync> =
                    Arc::new(move |progress| {
                        let _ = progress_thread.queue(move |mut model| {
                            model.as_mut().update_progress(generation, progress);
                        });
                    });
                let result = crate::firmware_audit::audit_local_collection(
                    &catalog_database,
                    &state_database,
                    &cancel,
                    progress,
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_audit(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut().rust_mut().cancel = None;
            self.as_mut().set_message(qstring(format!(
                "Could not start the firmware audit worker: {error}"
            )));
        }
    }

    pub fn cancel(mut self: Pin<&mut Self>) {
        if let Some(cancel) = self.as_ref().rust().cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
            self.as_mut()
                .set_message(qstring("Cancelling after the current platform inspection…"));
        }
    }

    fn update_progress(mut self: Pin<&mut Self>, generation: u64, progress: FirmwareAuditProgress) {
        if generation != self.as_ref().rust().generation || !*self.as_ref().busy() {
            return;
        }
        self.as_mut()
            .set_progress_current(saturating_i32(progress.current));
        self.as_mut()
            .set_progress_total(saturating_i32(progress.total));
        self.as_mut()
            .set_current_platform(qstring(progress.platform));
    }

    fn finish_audit(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<FirmwareAuditOutput, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        self.as_mut().rust_mut().cancel = None;
        self.as_mut().set_current_platform(QString::default());
        match result {
            Ok(output) => {
                self.as_mut()
                    .set_game_count(saturating_i32(output.game_count));
                self.as_mut()
                    .set_platform_count(saturating_i32(output.platform_count));
                self.as_mut().set_no_firmware_platform_count(saturating_i32(
                    output.no_firmware_platform_count,
                ));
                self.as_mut()
                    .set_missing_count(saturating_i32(output.missing_count));
                self.as_mut()
                    .set_manual_count(saturating_i32(output.manual_count));
                self.as_mut()
                    .set_optional_count(saturating_i32(output.optional_count));
                self.as_mut()
                    .set_ready_count(saturating_i32(output.ready_count));
                self.as_mut()
                    .set_no_runtime_count(saturating_i32(output.no_runtime_count));
                self.as_mut()
                    .set_error_count(saturating_i32(output.error_count));
                self.as_mut()
                    .set_unavailable_game_count(saturating_i32(output.unavailable_game_count));
                let cancelled = output.cancelled;
                let platform_count = output.platform_count;
                let missing = output.missing_count;
                let manual = output.manual_count;
                let runtime = output.no_runtime_count;
                let errors = output.error_count;
                let completed = *self.as_ref().progress_current();
                self.as_mut().rust_mut().output = Some(Arc::new(output));
                self.as_mut().rebuild_visible();
                self.as_mut().set_message(qstring(if cancelled {
                    format!(
                        "Audit cancelled. Retained verified results for {} platform{}.",
                        completed,
                        if completed == 1 { "" } else { "s" }
                    )
                } else if platform_count == 0 {
                    "My Collection has no present local games to inspect.".to_owned()
                } else if missing + manual + runtime + errors == 0 {
                    format!(
                        "Firmware readiness is clear across {platform_count} local platform{}.",
                        if platform_count == 1 { "" } else { "s" }
                    )
                } else {
                    let mut issues = Vec::new();
                    if missing > 0 {
                        issues.push(format!(
                            "{missing} missing package{}",
                            if missing == 1 { "" } else { "s" }
                        ));
                    }
                    if manual > 0 {
                        issues.push(format!(
                            "{manual} manual dump{}",
                            if manual == 1 { "" } else { "s" }
                        ));
                    }
                    if runtime > 0 {
                        issues.push(format!(
                            "{runtime} platform{} without a selected emulator",
                            if runtime == 1 { "" } else { "s" }
                        ));
                    }
                    if errors > 0 {
                        issues.push(format!(
                            "{errors} inspection error{}",
                            if errors == 1 { "" } else { "s" }
                        ));
                    }
                    format!("Review {}.", human_join(&issues))
                }));
            }
            Err(error) => {
                self.as_mut()
                    .set_message(qstring(format!("Firmware audit failed: {error}")));
            }
        }
    }

    pub fn apply_filter(mut self: Pin<&mut Self>, search: QString, status: QString, sort: QString) {
        self.as_mut().rust_mut().search = search.to_string().trim().to_lowercase();
        self.as_mut().rust_mut().status = match status.to_string().as_str() {
            "all" | "issues" | "missing" | "manual" | "optional" | "ready" | "runtime"
            | "error" => status.to_string(),
            _ => "issues".to_owned(),
        };
        self.as_mut().rust_mut().sort = match sort.to_string().as_str() {
            "status" | "platform" | "package" => sort.to_string(),
            _ => "status".to_owned(),
        };
        self.as_mut().rebuild_visible();
    }

    fn rebuild_visible(mut self: Pin<&mut Self>) {
        let Some(output) = self.as_ref().rust().output.as_ref().cloned() else {
            self.as_mut().rust_mut().visible_indices.clear();
            self.as_mut().set_entry_count(0);
            return;
        };
        let search = self.as_ref().rust().search.clone();
        let status = self.as_ref().rust().status.clone();
        let sort = self.as_ref().rust().sort.clone();
        let indices = filtered_indices(&output, &search, &status, &sort);
        let count = indices.len();
        self.as_mut().rust_mut().visible_indices = indices;
        self.as_mut().set_entry_count(saturating_i32(count));
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
    }

    pub fn row_game_uid_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(&entry.game_uid))
            .unwrap_or_default()
    }

    pub fn row_database_id_at(&self, index: i32) -> i32 {
        entry_at(self, index)
            .and_then(|entry| i32::try_from(entry.launchbox_db_id).ok())
            .unwrap_or_default()
    }

    pub fn row_title_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(&entry.title))
            .unwrap_or_default()
    }

    pub fn row_platform_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(&entry.platform))
            .unwrap_or_default()
    }

    pub fn row_emulator_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(&entry.emulator))
            .unwrap_or_default()
    }

    pub fn row_package_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(&entry.package))
            .unwrap_or_default()
    }

    pub fn row_source_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(&entry.source))
            .unwrap_or_default()
    }

    pub fn row_status_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(entry.status.key()))
            .unwrap_or_default()
    }

    pub fn row_status_label_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(entry.status.label()))
            .unwrap_or_default()
    }

    pub fn row_detail_at(&self, index: i32) -> QString {
        entry_at(self, index)
            .map(|entry| qstring(&entry.detail))
            .unwrap_or_default()
    }

    pub fn report_ui_probe(&self) {
        if self.rust().ui_probe {
            println!(
                "LUNCHBOX_FIRMWARE_AUDIT_UI_READY games={} platforms={} entries={} missing={} manual={} optional={} ready={} runtime={} unavailable={} message={:?}",
                self.game_count(),
                self.platform_count(),
                self.entry_count(),
                self.missing_count(),
                self.manual_count(),
                self.optional_count(),
                self.ready_count(),
                self.no_runtime_count(),
                self.unavailable_game_count(),
                self.message().to_string(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firmware_audit::{FirmwareAuditEntry, FirmwareAuditStatus};

    fn entry(status: FirmwareAuditStatus, platform: &str, package: &str) -> FirmwareAuditEntry {
        FirmwareAuditEntry {
            game_uid: "game".into(),
            launchbox_db_id: 1,
            title: "Game".into(),
            platform: platform.into(),
            emulator: "RetroArch".into(),
            package: package.into(),
            source: "Minerva".into(),
            status,
            detail: "Detail".into(),
        }
    }

    #[test]
    fn issue_filter_keeps_blocking_and_review_states_only() {
        let output = FirmwareAuditOutput {
            entries: vec![
                entry(FirmwareAuditStatus::Ready, "B", "ready"),
                entry(FirmwareAuditStatus::Optional, "A", "optional"),
                entry(FirmwareAuditStatus::Missing, "C", "missing"),
                entry(FirmwareAuditStatus::NoRuntime, "A", "runtime"),
            ],
            ..FirmwareAuditOutput::default()
        };
        let indices = filtered_indices(&output, "", "issues", "status");
        assert_eq!(indices.len(), 2);
        assert_eq!(output.entries[indices[0]].status.key(), "missing");
        assert_eq!(output.entries[indices[1]].status.key(), "runtime");
    }
}
