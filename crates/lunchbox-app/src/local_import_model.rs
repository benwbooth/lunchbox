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
        #[qproperty(bool, initialized)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, scanning)]
        #[qproperty(bool, importing)]
        #[qproperty(QString, directory)]
        #[qproperty(QString, message)]
        #[qproperty(QString, current_file)]
        #[qproperty(i32, total_files)]
        #[qproperty(i32, scanned_files)]
        #[qproperty(i32, matched_count)]
        #[qproperty(i32, unmatched_count)]
        #[qproperty(i32, result_count)]
        #[qproperty(i32, selected_count)]
        #[qproperty(i32, platform_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, collection_revision)]
        type LocalImportModel = super::LocalImportModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut LocalImportModel>);

        #[qinvokable]
        fn choose_directory(self: Pin<&mut LocalImportModel>, url: QUrl);

        #[qinvokable]
        fn start_scan(self: Pin<&mut LocalImportModel>, platform: QString, checksums_enabled: bool);

        #[qinvokable]
        fn cancel_scan(self: Pin<&mut LocalImportModel>);

        #[qinvokable]
        fn apply_result_filter(
            self: Pin<&mut LocalImportModel>,
            search: QString,
            status: QString,
            sort: QString,
            ascending: bool,
        );

        #[qinvokable]
        fn toggle_selected(self: Pin<&mut LocalImportModel>, index: i32);

        #[qinvokable]
        fn select_visible(self: Pin<&mut LocalImportModel>, mode: QString);

        #[qinvokable]
        fn import_selected(self: Pin<&mut LocalImportModel>);

        #[qinvokable]
        fn platform_name_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_file_name_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_archive_detail_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_path_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_title_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_platform_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_status_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_detail_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_size_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn result_selected_at(self: &LocalImportModel, index: i32) -> bool;
    }

    impl cxx_qt::Threading for LocalImportModel {}
}

use std::cmp::Ordering;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::catalog;
use crate::local_import::{self, MatchState, ScanOutput, ScanProgress, ScanResult};

pub struct LocalImportModelRust {
    initialized: bool,
    busy: bool,
    scanning: bool,
    importing: bool,
    directory: QString,
    message: QString,
    current_file: QString,
    total_files: i32,
    scanned_files: i32,
    matched_count: i32,
    unmatched_count: i32,
    result_count: i32,
    selected_count: i32,
    platform_count: i32,
    revision: i32,
    collection_revision: i32,
    platforms: Vec<String>,
    output: Option<ScanOutput>,
    visible_indices: Vec<usize>,
    cancel: Option<Arc<AtomicBool>>,
    generation: u64,
}

impl Default for LocalImportModelRust {
    fn default() -> Self {
        let directory = catalog::requested_path("--import-directory", "LUNCHBOX_IMPORT_DIRECTORY")
            .map(|path| qstring(path.to_string_lossy()))
            .unwrap_or_default();
        Self {
            initialized: false,
            busy: false,
            scanning: false,
            importing: false,
            directory,
            message: QString::from("Choose a ROM folder to scan your existing collection."),
            current_file: QString::default(),
            total_files: 0,
            scanned_files: 0,
            matched_count: 0,
            unmatched_count: 0,
            result_count: 0,
            selected_count: 0,
            platform_count: 0,
            revision: 0,
            collection_revision: 0,
            platforms: Vec::new(),
            output: None,
            visible_indices: Vec::new(),
            cancel: None,
            generation: 0,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

impl qobject::LocalImportModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        let Some(discovery_path) = catalog::requested_discovery_database_path() else {
            self.as_mut().set_message(qstring(
                "A discovery games database is required for local ROM import.",
            ));
            return;
        };
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Loading import platforms…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-import-platforms".into())
            .spawn(move || {
                let result = local_import::load_platform_names(&discovery_path)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_initialize(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start import initialization: {error}"
            )));
        }
    }

    fn finish_initialize(mut self: Pin<&mut Self>, result: Result<Vec<String>, String>) {
        self.as_mut().set_busy(false);
        match result {
            Ok(platforms) => {
                let count = platforms.len();
                self.as_mut().rust_mut().platforms = platforms;
                self.as_mut().set_platform_count(saturating_i32(count));
                self.as_mut().set_initialized(true);
                self.as_mut().set_message(qstring(
                    "Choose a ROM folder. Exact matches use SHA-1 and MD5; filenames are never accepted as identity.",
                ));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not load import platforms: {error}"))),
        }
    }

    pub fn choose_directory(mut self: Pin<&mut Self>, url: QUrl) {
        let Some(path) = url.to_local_file() else {
            self.as_mut()
                .set_message(qstring("Choose a local filesystem directory."));
            return;
        };
        self.as_mut().set_directory(path);
    }

    pub fn start_scan(mut self: Pin<&mut Self>, platform: QString, checksums_enabled: bool) {
        if *self.as_ref().busy() {
            return;
        }
        let root = PathBuf::from(self.as_ref().directory().to_string());
        if root.as_os_str().is_empty() {
            self.as_mut()
                .set_message(qstring("Choose a ROM directory before scanning."));
            return;
        }
        let Some(discovery_path) = catalog::requested_discovery_database_path() else {
            self.as_mut().set_message(qstring(
                "A discovery games database is required for local ROM import.",
            ));
            return;
        };

        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().cancel = Some(Arc::clone(&cancel));
        self.as_mut().rust_mut().output = None;
        self.as_mut().rust_mut().visible_indices.clear();
        self.as_mut().set_busy(true);
        self.as_mut().set_scanning(true);
        self.as_mut().set_total_files(0);
        self.as_mut().set_scanned_files(0);
        self.as_mut().set_matched_count(0);
        self.as_mut().set_unmatched_count(0);
        self.as_mut().set_result_count(0);
        self.as_mut().set_selected_count(0);
        self.as_mut().set_current_file(QString::default());
        self.as_mut()
            .set_message(qstring("Discovering supported ROM files…"));
        self.as_mut().bump_revision();

        let platform = platform.to_string();
        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress: Arc<dyn Fn(ScanProgress) + Send + Sync> = Arc::new(move |progress| {
            let _ = progress_thread.queue(move |mut model| {
                model.as_mut().update_scan_progress(generation, progress);
            });
        });
        let spawn = std::thread::Builder::new()
            .name("lunchbox-rom-scan".into())
            .spawn(move || {
                let result = local_import::scan_directory(
                    &discovery_path,
                    &root,
                    &platform,
                    checksums_enabled,
                    &cancel,
                    progress,
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_scan(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_scanning(false);
            self.as_mut().rust_mut().cancel = None;
            self.as_mut()
                .set_message(qstring(format!("Could not start ROM scanner: {error}")));
        }
    }

    fn update_scan_progress(mut self: Pin<&mut Self>, generation: u64, progress: ScanProgress) {
        if generation != self.as_ref().rust().generation || !*self.as_ref().scanning() {
            return;
        }
        self.as_mut()
            .set_total_files(saturating_i32(progress.total_files));
        self.as_mut()
            .set_scanned_files(saturating_i32(progress.scanned_files));
        self.as_mut()
            .set_current_file(qstring(progress.current_file));
        if progress.total_files > 0 {
            self.as_mut().set_message(qstring(format!(
                "Verifying ROM {} of {}…",
                progress.scanned_files, progress.total_files
            )));
        }
    }

    fn finish_scan(mut self: Pin<&mut Self>, generation: u64, result: Result<ScanOutput, String>) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        self.as_mut().set_scanning(false);
        self.as_mut().rust_mut().cancel = None;
        match result {
            Ok(output) => {
                let total = output.results.len();
                let matched = output
                    .results
                    .iter()
                    .filter(|result| matches!(result.match_state, MatchState::Exact(_)))
                    .count();
                let selected = output
                    .results
                    .iter()
                    .filter(|result| result.selected)
                    .count();
                let cancelled = output.cancelled;
                let walk_errors = output.walk_errors;
                self.as_mut().rust_mut().visible_indices = (0..total).collect();
                self.as_mut().rust_mut().output = Some(output);
                self.as_mut().set_total_files(saturating_i32(total));
                self.as_mut().set_scanned_files(saturating_i32(total));
                self.as_mut().set_matched_count(saturating_i32(matched));
                self.as_mut()
                    .set_unmatched_count(saturating_i32(total.saturating_sub(matched)));
                self.as_mut().set_result_count(saturating_i32(total));
                self.as_mut().set_selected_count(saturating_i32(selected));
                self.as_mut().set_current_file(QString::default());
                self.as_mut().bump_revision();
                self.as_mut().set_message(qstring(if cancelled {
                    format!(
                        "Scan cancelled after {total} files; completed results remain available for review."
                    )
                } else if walk_errors > 0 {
                    format!(
                        "Scanned {total} ROMs: {matched} exact matches. {walk_errors} filesystem entries could not be read."
                    )
                } else {
                    format!(
                        "Scanned {total} ROMs: {matched} exact matches and {} unmatched or ambiguous.",
                        total.saturating_sub(matched)
                    )
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("ROM scan failed: {error}"))),
        }
    }

    pub fn cancel_scan(mut self: Pin<&mut Self>) {
        if let Some(cancel) = self.as_ref().rust().cancel.as_ref() {
            cancel.store(true, AtomicOrdering::Relaxed);
            self.as_mut()
                .set_message(qstring("Cancelling after the current filesystem read…"));
        }
    }

    pub fn apply_result_filter(
        mut self: Pin<&mut Self>,
        search: QString,
        status: QString,
        sort: QString,
        ascending: bool,
    ) {
        let search = search.to_string().trim().to_lowercase();
        let status = status.to_string();
        let sort = sort.to_string();
        let mut indices = self
            .as_ref()
            .rust()
            .output
            .as_ref()
            .map(|output| {
                output
                    .results
                    .iter()
                    .enumerate()
                    .filter(|(_, result)| {
                        (search.is_empty()
                            || result.file_name.to_lowercase().contains(&search)
                            || result.archive_member.to_lowercase().contains(&search)
                            || result.display_title.to_lowercase().contains(&search)
                            || result.platform.to_lowercase().contains(&search)
                            || matched_title(result).to_lowercase().contains(&search))
                            && (status == "all" || result.match_state.key() == status)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if let Some(output) = self.as_ref().rust().output.as_ref() {
            indices.sort_by(|left, right| {
                let left = &output.results[*left];
                let right = &output.results[*right];
                let order = match sort.as_str() {
                    "platform" => left
                        .platform
                        .to_lowercase()
                        .cmp(&right.platform.to_lowercase())
                        .then_with(|| {
                            left.file_name
                                .to_lowercase()
                                .cmp(&right.file_name.to_lowercase())
                        }),
                    "size" => left.file_size.cmp(&right.file_size),
                    "status" => match_rank(&left.match_state)
                        .cmp(&match_rank(&right.match_state))
                        .then_with(|| {
                            left.file_name
                                .to_lowercase()
                                .cmp(&right.file_name.to_lowercase())
                        }),
                    _ => left
                        .file_name
                        .to_lowercase()
                        .cmp(&right.file_name.to_lowercase()),
                };
                if ascending { order } else { reverse(order) }
            });
        }
        let count = indices.len();
        self.as_mut().rust_mut().visible_indices = indices;
        self.as_mut().set_result_count(saturating_i32(count));
        self.as_mut().bump_revision();
    }

    pub fn toggle_selected(mut self: Pin<&mut Self>, index: i32) {
        let Some(result_index) = self.as_ref().visible_result_index(index) else {
            return;
        };
        if let Some(result) = self
            .as_mut()
            .rust_mut()
            .output
            .as_mut()
            .and_then(|output| output.results.get_mut(result_index))
            && !matches!(result.match_state, MatchState::Error(_))
        {
            result.selected = !result.selected;
        }
        self.as_mut().update_selected_count();
        self.as_mut().bump_revision();
    }

    pub fn select_visible(mut self: Pin<&mut Self>, mode: QString) {
        let mode = mode.to_string();
        let indices = self.as_ref().rust().visible_indices.clone();
        if let Some(output) = self.as_mut().rust_mut().output.as_mut() {
            for index in indices {
                if let Some(result) = output.results.get_mut(index) {
                    result.selected = match mode.as_str() {
                        "all" => !matches!(result.match_state, MatchState::Error(_)),
                        "exact" => matches!(result.match_state, MatchState::Exact(_)),
                        _ => false,
                    };
                }
            }
        }
        self.as_mut().update_selected_count();
        self.as_mut().bump_revision();
    }

    pub fn import_selected(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(output) = self.as_ref().rust().output.clone() else {
            self.as_mut()
                .set_message(qstring("Scan a ROM directory before importing."));
            return;
        };
        let selected = output
            .results
            .iter()
            .filter(|result| result.selected)
            .count();
        if selected == 0 {
            self.as_mut()
                .set_message(qstring("Select at least one readable ROM to import."));
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_importing(true);
        self.as_mut()
            .set_message(qstring(format!("Importing {selected} selected ROMs…")));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-rom-import".into())
            .spawn(move || {
                let result = local_import::commit_scan(&output).map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_import(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_importing(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start ROM import: {error}")));
        }
    }

    fn finish_import(mut self: Pin<&mut Self>, result: Result<usize, String>) {
        self.as_mut().set_busy(false);
        self.as_mut().set_importing(false);
        match result {
            Ok(count) => {
                let revision = self.as_ref().collection_revision().wrapping_add(1);
                self.as_mut().set_collection_revision(revision);
                self.as_mut().set_message(qstring(format!(
                    "Imported {count} ROM files. Exact matches now leave the Minerva ‘not installed’ view; unmatched files remain visible in My Collection."
                )));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("ROM import failed: {error}"))),
        }
    }

    pub fn platform_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().platforms.get(index))
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn result_file_name_at(&self, index: i32) -> QString {
        self.result_at(index)
            .map(|result| qstring(&result.file_name))
            .unwrap_or_default()
    }

    pub fn result_archive_detail_at(&self, index: i32) -> QString {
        self.result_at(index)
            .map(|result| {
                if !result.archive_member.is_empty() {
                    if result.archive_member_count > 1 {
                        qstring(format!(
                            "↳ {} · one of {} ROM members",
                            result.archive_member, result.archive_member_count
                        ))
                    } else {
                        qstring(format!("↳ {}", result.archive_member))
                    }
                } else if result.archive_member_count > 1 {
                    qstring(format!(
                        "{} ROM members · review required",
                        result.archive_member_count
                    ))
                } else {
                    QString::default()
                }
            })
            .unwrap_or_default()
    }

    pub fn result_path_at(&self, index: i32) -> QString {
        self.result_at(index)
            .map(|result| {
                if result.archive_member.is_empty() {
                    qstring(result.path.to_string_lossy())
                } else {
                    qstring(format!(
                        "{}  ›  {}",
                        result.path.to_string_lossy(),
                        result.archive_member
                    ))
                }
            })
            .unwrap_or_default()
    }

    pub fn result_title_at(&self, index: i32) -> QString {
        self.result_at(index)
            .map(|result| qstring(matched_title(result)))
            .unwrap_or_default()
    }

    pub fn result_platform_at(&self, index: i32) -> QString {
        self.result_at(index)
            .map(|result| qstring(&result.platform))
            .unwrap_or_default()
    }

    pub fn result_status_at(&self, index: i32) -> QString {
        self.result_at(index)
            .map(|result| {
                qstring(match &result.match_state {
                    MatchState::Exact(_) => "EXACT",
                    MatchState::Ambiguous(_) => "AMBIGUOUS",
                    MatchState::Unmatched => "UNMATCHED",
                    MatchState::InventoryOnly => "INVENTORY",
                    MatchState::Error(_) => "ERROR",
                })
            })
            .unwrap_or_default()
    }

    pub fn result_detail_at(&self, index: i32) -> QString {
        self.result_at(index)
            .map(|result| {
                qstring(match &result.match_state {
                    MatchState::Exact(identity) => {
                        format!("{} · {}", identity.title, result.match_method)
                    }
                    MatchState::Ambiguous(count) => {
                        format!("{count} catalog records share this checksum · kept local-only")
                    }
                    MatchState::Unmatched => {
                        "No strong checksum match · kept local-only".to_owned()
                    }
                    MatchState::InventoryOnly => {
                        if result.archive_member_count > 0 {
                            format!(
                                "{} member checksums disabled · local inventory only",
                                local_import::archive_kind_label(&result.path)
                            )
                        } else {
                            "Checksums disabled · local inventory only".to_owned()
                        }
                    }
                    MatchState::Error(error) => error.clone(),
                })
            })
            .unwrap_or_default()
    }

    pub fn result_size_at(&self, index: i32) -> QString {
        self.result_at(index)
            .map(|result| qstring(format_size(result.file_size)))
            .unwrap_or_default()
    }

    pub fn result_selected_at(&self, index: i32) -> bool {
        self.result_at(index).is_some_and(|result| result.selected)
    }

    fn result_at(&self, visible_index: i32) -> Option<&ScanResult> {
        self.visible_result_index(visible_index)
            .and_then(|index| self.rust().output.as_ref()?.results.get(index))
    }

    fn visible_result_index(&self, index: i32) -> Option<usize> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().visible_indices.get(index).copied())
    }

    fn update_selected_count(mut self: Pin<&mut Self>) {
        let count = self
            .as_ref()
            .rust()
            .output
            .as_ref()
            .map(|output| {
                output
                    .results
                    .iter()
                    .filter(|result| result.selected)
                    .count()
            })
            .unwrap_or_default();
        self.as_mut().set_selected_count(saturating_i32(count));
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
    }
}

fn matched_title(result: &ScanResult) -> &str {
    match &result.match_state {
        MatchState::Exact(identity) => &identity.title,
        _ => &result.display_title,
    }
}

fn match_rank(state: &MatchState) -> u8 {
    match state {
        MatchState::Exact(_) => 0,
        MatchState::Ambiguous(_) => 1,
        MatchState::Unmatched => 2,
        MatchState::InventoryOnly => 3,
        MatchState::Error(_) => 4,
    }
}

fn reverse(order: Ordering) -> Ordering {
    match order {
        Ordering::Less => Ordering::Greater,
        Ordering::Equal => Ordering::Equal,
        Ordering::Greater => Ordering::Less,
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_sizes_are_human_readable() {
        assert_eq!(format_size(42), "42 B");
        assert_eq!(format_size(1536), "1.5 KiB");
    }

    #[test]
    fn exact_matches_sort_before_unmatched_results() {
        assert!(
            match_rank(&MatchState::Exact(MatchIdentityFixture::identity()))
                < match_rank(&MatchState::Unmatched)
        );
    }

    struct MatchIdentityFixture;

    impl MatchIdentityFixture {
        fn identity() -> local_import::MatchIdentity {
            local_import::MatchIdentity {
                game_uid: "game".into(),
                title: "Game".into(),
                platform: "System".into(),
                launchbox_db_id: 1,
            }
        }
    }
}
