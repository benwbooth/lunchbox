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
        #[qproperty(bool, matching)]
        #[qproperty(bool, profile_busy)]
        #[qproperty(bool, batch_scanning)]
        #[qproperty(QString, directory)]
        #[qproperty(QString, message)]
        #[qproperty(QString, match_message)]
        #[qproperty(QString, match_file_name)]
        #[qproperty(QString, match_archive_detail)]
        #[qproperty(QString, match_default_query)]
        #[qproperty(QString, match_default_platform)]
        #[qproperty(QString, current_file)]
        #[qproperty(i32, total_files)]
        #[qproperty(i32, scanned_files)]
        #[qproperty(i32, cache_reused_files)]
        #[qproperty(i32, content_read_files)]
        #[qproperty(i32, matched_count)]
        #[qproperty(i32, reviewed_count)]
        #[qproperty(i32, unmatched_count)]
        #[qproperty(i32, result_count)]
        #[qproperty(i32, selected_count)]
        #[qproperty(i32, platform_count)]
        #[qproperty(i32, profile_count)]
        #[qproperty(i32, profile_revision)]
        #[qproperty(i32, active_profile_index)]
        #[qproperty(i32, batch_profile_count)]
        #[qproperty(i32, batch_completed_count)]
        #[qproperty(QString, batch_profile_name)]
        #[qproperty(i32, history_count)]
        #[qproperty(i32, history_revision)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, match_revision)]
        #[qproperty(i32, match_candidate_count)]
        #[qproperty(i32, collection_revision)]
        type LocalImportModel = super::LocalImportModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut LocalImportModel>);

        #[qinvokable]
        fn choose_directory(self: Pin<&mut LocalImportModel>, url: QUrl);

        #[qinvokable]
        fn apply_profile(self: Pin<&mut LocalImportModel>, index: i32) -> bool;

        #[qinvokable]
        fn clear_active_profile(self: Pin<&mut LocalImportModel>);

        #[qinvokable]
        fn save_profile(
            self: Pin<&mut LocalImportModel>,
            name: QString,
            platform: QString,
            extensions: QString,
            checksums_enabled: bool,
        );

        #[qinvokable]
        fn delete_profile(self: Pin<&mut LocalImportModel>, index: i32);

        #[qinvokable]
        fn start_profile_scan_batch(self: Pin<&mut LocalImportModel>);

        #[qinvokable]
        fn start_scan(
            self: Pin<&mut LocalImportModel>,
            platform: QString,
            extensions: QString,
            checksums_enabled: bool,
        );

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
        fn begin_manual_match(self: Pin<&mut LocalImportModel>, index: i32);

        #[qinvokable]
        fn search_manual_matches(
            self: Pin<&mut LocalImportModel>,
            query: QString,
            platform: QString,
        );

        #[qinvokable]
        fn apply_manual_match(self: Pin<&mut LocalImportModel>, index: i32) -> bool;

        #[qinvokable]
        fn cancel_manual_match(self: Pin<&mut LocalImportModel>);

        #[qinvokable]
        fn match_candidate_title_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn match_candidate_platform_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn match_candidate_detail_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn platform_name_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn profile_name_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn profile_directory_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn profile_platform_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn profile_extensions_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn profile_checksums_at(self: &LocalImportModel, index: i32) -> bool;

        #[qinvokable]
        fn profile_detail_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_profile_name_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_directory_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_scope_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_outcome_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_summary_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_batch_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_finished_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_error_at(self: &LocalImportModel, index: i32) -> QString;

        #[qinvokable]
        fn history_profile_index_at(self: &LocalImportModel, index: i32) -> i32;

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
use crate::local_import::{
    self, ManualMatchCandidate, MatchState, RomImportProfile, RomScanRun, RomScanRunContext,
    ScanOutput, ScanProgress, ScanResult,
};

struct CompletedScan {
    scan: Result<ScanOutput, String>,
    history: Result<Vec<RomScanRun>, String>,
}

struct LocalImportInitialization {
    platforms: Vec<String>,
    profiles: Vec<RomImportProfile>,
    history: Vec<RomScanRun>,
}

struct ProfileBatchCompletion {
    history: Option<Vec<RomScanRun>>,
    processed: usize,
    total_files: usize,
    exact_matches: usize,
    reviewed_matches: usize,
    other_results: usize,
    failures: usize,
    cancelled: bool,
    warnings: Vec<String>,
}

pub struct LocalImportModelRust {
    initialized: bool,
    busy: bool,
    scanning: bool,
    importing: bool,
    matching: bool,
    profile_busy: bool,
    batch_scanning: bool,
    directory: QString,
    message: QString,
    match_message: QString,
    match_file_name: QString,
    match_archive_detail: QString,
    match_default_query: QString,
    match_default_platform: QString,
    current_file: QString,
    total_files: i32,
    scanned_files: i32,
    cache_reused_files: i32,
    content_read_files: i32,
    matched_count: i32,
    reviewed_count: i32,
    unmatched_count: i32,
    result_count: i32,
    selected_count: i32,
    platform_count: i32,
    profile_count: i32,
    profile_revision: i32,
    active_profile_index: i32,
    batch_profile_count: i32,
    batch_completed_count: i32,
    batch_profile_name: QString,
    history_count: i32,
    history_revision: i32,
    revision: i32,
    match_revision: i32,
    match_candidate_count: i32,
    collection_revision: i32,
    platforms: Vec<String>,
    profiles: Vec<RomImportProfile>,
    history: Vec<RomScanRun>,
    profile_root_override: Option<PathBuf>,
    output: Option<ScanOutput>,
    visible_indices: Vec<usize>,
    cancel: Option<Arc<AtomicBool>>,
    generation: u64,
    match_result_index: Option<usize>,
    match_candidates: Vec<ManualMatchCandidate>,
    match_generation: u64,
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
            matching: false,
            profile_busy: false,
            batch_scanning: false,
            directory,
            message: QString::from("Choose a ROM folder to scan your existing collection."),
            match_message: QString::from(
                "Search the catalog, then explicitly choose the record this ROM represents.",
            ),
            match_file_name: QString::default(),
            match_archive_detail: QString::default(),
            match_default_query: QString::default(),
            match_default_platform: QString::default(),
            current_file: QString::default(),
            total_files: 0,
            scanned_files: 0,
            cache_reused_files: 0,
            content_read_files: 0,
            matched_count: 0,
            reviewed_count: 0,
            unmatched_count: 0,
            result_count: 0,
            selected_count: 0,
            platform_count: 0,
            profile_count: 0,
            profile_revision: 0,
            active_profile_index: -1,
            batch_profile_count: 0,
            batch_completed_count: 0,
            batch_profile_name: QString::default(),
            history_count: 0,
            history_revision: 0,
            revision: 0,
            match_revision: 0,
            match_candidate_count: 0,
            collection_revision: 0,
            platforms: Vec::new(),
            profiles: Vec::new(),
            history: Vec::new(),
            profile_root_override: None,
            output: None,
            visible_indices: Vec::new(),
            cancel: None,
            generation: 0,
            match_result_index: None,
            match_candidates: Vec::new(),
            match_generation: 0,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn import_profile_probe() -> bool {
    std::env::args().any(|argument| argument == "--import-profile-ui-probe")
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
        let state_path = match crate::settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_message(qstring(format!(
                    "Could not locate Lunchbox state for ROM import: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_busy(true);
        if import_profile_probe() {
            println!("LUNCHBOX_IMPORT_PROFILE_MODEL initializing");
        }
        self.as_mut()
            .set_message(qstring("Loading import platforms…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-import-platforms".into())
            .spawn(move || {
                let result = (|| -> anyhow::Result<LocalImportInitialization> {
                    Ok(LocalImportInitialization {
                        platforms: local_import::load_platform_names(&discovery_path)?,
                        profiles: local_import::load_import_profiles(&state_path)?,
                        history: local_import::load_scan_runs(&state_path)?,
                    })
                })()
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

    fn finish_initialize(
        mut self: Pin<&mut Self>,
        result: Result<LocalImportInitialization, String>,
    ) {
        self.as_mut().set_busy(false);
        match result {
            Ok(initialization) => {
                let count = initialization.platforms.len();
                let profile_count = initialization.profiles.len();
                let history_count = initialization.history.len();
                self.as_mut().rust_mut().platforms = initialization.platforms;
                self.as_mut().rust_mut().profiles = initialization.profiles;
                self.as_mut().rust_mut().history = initialization.history;
                self.as_mut().set_platform_count(saturating_i32(count));
                self.as_mut()
                    .set_profile_count(saturating_i32(profile_count));
                self.as_mut()
                    .set_history_count(saturating_i32(history_count));
                self.as_mut().bump_profile_revision();
                self.as_mut().bump_history_revision();
                self.as_mut().set_initialized(true);
                if import_profile_probe() {
                    println!(
                        "LUNCHBOX_IMPORT_PROFILE_MODEL ready platforms={} profiles={}",
                        count, profile_count
                    );
                }
                self.as_mut().set_message(qstring(
                    "Choose a ROM folder. Exact matches use SHA-1 and MD5; filenames are never accepted as identity.",
                ));
            }
            Err(error) => {
                if import_profile_probe() {
                    eprintln!("LUNCHBOX_IMPORT_PROFILE_MODEL failed error={error}");
                }
                self.as_mut()
                    .set_message(qstring(format!("Could not load import platforms: {error}")));
            }
        }
    }

    pub fn choose_directory(mut self: Pin<&mut Self>, url: QUrl) {
        let Some(path) = url.to_local_file() else {
            self.as_mut()
                .set_message(qstring("Choose a local filesystem directory."));
            return;
        };
        self.as_mut().rust_mut().profile_root_override = None;
        self.as_mut().set_active_profile_index(-1);
        self.as_mut().set_directory(path);
    }

    pub fn apply_profile(mut self: Pin<&mut Self>, index: i32) -> bool {
        if *self.as_ref().busy() || *self.as_ref().profile_busy() {
            return false;
        }
        let Some(index) = usize::try_from(index).ok() else {
            return false;
        };
        let Some(profile) = self.as_ref().rust().profiles.get(index).cloned() else {
            return false;
        };
        self.as_mut().rust_mut().profile_root_override = Some(profile.root.clone());
        self.as_mut()
            .set_directory(qstring(profile.root.to_string_lossy()));
        self.as_mut()
            .set_active_profile_index(saturating_i32(index));
        self.as_mut().set_message(qstring(format!(
            "Loaded scan profile ‘{}’: {}.",
            profile.name,
            profile_scope_detail(&profile)
        )));
        if import_profile_probe() {
            println!(
                "LUNCHBOX_IMPORT_PROFILE_UI_READY name={:?} scope={:?} directory={:?}",
                profile.name,
                profile_scope_detail(&profile),
                profile.root,
            );
        }
        true
    }

    pub fn clear_active_profile(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || *self.as_ref().profile_busy() {
            return;
        }
        if *self.as_ref().active_profile_index() >= 0 {
            self.as_mut().set_active_profile_index(-1);
            self.as_mut().set_message(qstring(
                "Using a one-time scan scope. Save it as a profile if you want to reuse it.",
            ));
        }
    }

    pub fn save_profile(
        mut self: Pin<&mut Self>,
        name: QString,
        platform: QString,
        extensions: QString,
        checksums_enabled: bool,
    ) {
        if *self.as_ref().busy() || *self.as_ref().profile_busy() {
            return;
        }
        let root = self
            .as_ref()
            .rust()
            .profile_root_override
            .clone()
            .unwrap_or_else(|| PathBuf::from(self.as_ref().directory().to_string()));
        let name = name.to_string();
        let platform = platform.to_string();
        let extensions = extensions.to_string();
        let state_path = match crate::settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_message(qstring(format!(
                    "Could not locate Lunchbox state for scan profiles: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_profile_busy(true);
        self.as_mut()
            .set_message(qstring("Saving reusable ROM scan profile…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-import-profile-save".into())
            .spawn(move || {
                let result = (|| -> anyhow::Result<(Vec<RomImportProfile>, String, String)> {
                    let saved = local_import::save_import_profile(
                        &state_path,
                        &name,
                        &root,
                        &platform,
                        &extensions,
                        checksums_enabled,
                    )?;
                    let profiles = local_import::load_import_profiles(&state_path)?;
                    Ok((profiles, saved.id, saved.name))
                })()
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_profile_save(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start scan-profile save: {error}"
            )));
        }
    }

    fn finish_profile_save(
        mut self: Pin<&mut Self>,
        result: Result<(Vec<RomImportProfile>, String, String), String>,
    ) {
        self.as_mut().set_profile_busy(false);
        match result {
            Ok((profiles, saved_id, saved_name)) => {
                let selected = profiles
                    .iter()
                    .position(|profile| profile.id == saved_id)
                    .map(saturating_i32)
                    .unwrap_or(-1);
                let count = profiles.len();
                self.as_mut().rust_mut().profiles = profiles;
                self.as_mut().set_profile_count(saturating_i32(count));
                self.as_mut().set_active_profile_index(selected);
                self.as_mut().bump_profile_revision();
                self.as_mut().set_message(qstring(format!(
                    "Saved scan profile ‘{saved_name}’. Apply it whenever this collection needs another import."
                )));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not save scan profile: {error}"))),
        }
    }

    pub fn delete_profile(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() || *self.as_ref().profile_busy() {
            return;
        }
        let profile = usize::try_from(index)
            .ok()
            .and_then(|index| self.as_ref().rust().profiles.get(index).cloned());
        let Some(profile) = profile else {
            self.as_mut()
                .set_message(qstring("Choose a saved scan profile to remove."));
            return;
        };
        let state_path = match crate::settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_message(qstring(format!(
                    "Could not locate Lunchbox state for scan profiles: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_profile_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-import-profile-delete".into())
            .spawn(move || {
                let result = (|| -> anyhow::Result<(Vec<RomImportProfile>, String)> {
                    local_import::delete_import_profile(&state_path, &profile.id)?;
                    Ok((
                        local_import::load_import_profiles(&state_path)?,
                        profile.name,
                    ))
                })()
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_profile_delete(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start scan-profile removal: {error}"
            )));
        }
    }

    fn finish_profile_delete(
        mut self: Pin<&mut Self>,
        result: Result<(Vec<RomImportProfile>, String), String>,
    ) {
        self.as_mut().set_profile_busy(false);
        match result {
            Ok((profiles, removed_name)) => {
                let count = profiles.len();
                self.as_mut().rust_mut().profiles = profiles;
                self.as_mut().set_profile_count(saturating_i32(count));
                self.as_mut().set_active_profile_index(-1);
                self.as_mut().bump_profile_revision();
                self.as_mut().set_message(qstring(format!(
                    "Removed scan profile ‘{removed_name}’. No ROMs or collection records were changed."
                )));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not remove scan profile: {error}"))),
        }
    }

    pub fn start_profile_scan_batch(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || *self.as_ref().profile_busy() {
            return;
        }
        let profiles = self.as_ref().rust().profiles.clone();
        if profiles.is_empty() {
            self.as_mut().set_message(qstring(
                "Save at least one ROM scan profile before scanning every collection.",
            ));
            return;
        }
        let Some(discovery_path) = catalog::requested_discovery_database_path() else {
            self.as_mut().set_message(qstring(
                "A discovery games database is required for local ROM import.",
            ));
            return;
        };
        let state_path = match crate::settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_message(qstring(format!(
                    "Could not locate Lunchbox state for ROM import: {error}"
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
        self.as_mut().set_busy(true);
        self.as_mut().set_scanning(true);
        self.as_mut().set_batch_scanning(true);
        self.as_mut()
            .set_batch_profile_count(saturating_i32(profiles.len()));
        self.as_mut().set_batch_completed_count(0);
        self.as_mut().set_batch_profile_name(QString::default());
        self.as_mut().reset_result_summary();
        self.as_mut().set_message(qstring(format!(
            "Preparing to scan {} saved ROM collections…",
            profiles.len()
        )));

        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-rom-profile-batch".into())
            .spawn(move || {
                let result = (|| -> anyhow::Result<ProfileBatchCompletion> {
                    let scanner = local_import::PreparedScanner::new(&discovery_path)?;
                    let batch_id = uuid::Uuid::new_v4().to_string();
                    let profile_total = profiles.len();
                    let mut completion = ProfileBatchCompletion {
                        history: None,
                        processed: 0,
                        total_files: 0,
                        exact_matches: 0,
                        reviewed_matches: 0,
                        other_results: 0,
                        failures: 0,
                        cancelled: false,
                        warnings: Vec::new(),
                    };
                    for (index, profile) in profiles.iter().enumerate() {
                        if cancel.load(AtomicOrdering::Relaxed) {
                            completion.cancelled = true;
                            break;
                        }
                        let position = index + 1;
                        let profile_name = profile.name.clone();
                        let progress_thread = qt_thread.clone();
                        let progress_name = profile_name.clone();
                        let progress: Arc<dyn Fn(ScanProgress) + Send + Sync> =
                            Arc::new(move |progress| {
                                let name = progress_name.clone();
                                let _ = progress_thread.queue(move |mut model| {
                                    model.as_mut().update_profile_batch_progress(
                                        generation,
                                        position,
                                        profile_total,
                                        name,
                                        progress,
                                    );
                                });
                            });
                        progress(ScanProgress::default());
                        let scan = scanner
                            .scan_cached_with_extensions(
                                &state_path,
                                &profile.root,
                                &profile.platform_hint,
                                &profile.extensions,
                                profile.checksums_enabled,
                                &cancel,
                                progress,
                            )
                            .and_then(|mut output| {
                                local_import::restore_manual_matches(&state_path, &mut output)?;
                                Ok(output)
                            })
                            .map_err(|error| error.to_string());
                        let context = RomScanRunContext::for_profile(
                            profile,
                            batch_id.clone(),
                            position,
                            profile_total,
                        );
                        match local_import::record_scan_run(
                            &state_path,
                            &context,
                            scan.as_ref().map_err(String::as_str),
                        ) {
                            Ok(run) => {
                                completion.total_files =
                                    completion.total_files.saturating_add(run.total_files);
                                completion.exact_matches =
                                    completion.exact_matches.saturating_add(run.exact_matches);
                                completion.reviewed_matches = completion
                                    .reviewed_matches
                                    .saturating_add(run.reviewed_matches);
                                completion.other_results =
                                    completion.other_results.saturating_add(run.other_results);
                            }
                            Err(error) => completion.warnings.push(format!(
                                "Could not retain history for ‘{}’: {error}",
                                profile.name
                            )),
                        }
                        completion.processed = position;
                        if scan.as_ref().is_err() {
                            completion.failures = completion.failures.saturating_add(1);
                        }
                        if scan.as_ref().is_ok_and(|output| output.cancelled)
                            || cancel.load(AtomicOrdering::Relaxed)
                        {
                            completion.cancelled = true;
                            break;
                        }
                        // `scan` drops here before the next root. Only compact history is
                        // retained; a later review scan reuses the validated hash cache.
                    }
                    match local_import::load_scan_runs(&state_path) {
                        Ok(history) => completion.history = Some(history),
                        Err(error) => completion
                            .warnings
                            .push(format!("Could not reload ROM scan history: {error}")),
                    }
                    Ok(completion)
                })()
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_profile_scan_batch(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_scanning(false);
            self.as_mut().set_batch_scanning(false);
            self.as_mut().rust_mut().cancel = None;
            self.as_mut().set_message(qstring(format!(
                "Could not start saved-collection scan: {error}"
            )));
        }
    }

    fn update_profile_batch_progress(
        mut self: Pin<&mut Self>,
        generation: u64,
        position: usize,
        total: usize,
        profile_name: String,
        progress: ScanProgress,
    ) {
        if generation != self.as_ref().rust().generation || !*self.as_ref().batch_scanning() {
            return;
        }
        self.as_mut()
            .set_batch_completed_count(saturating_i32(position.saturating_sub(1)));
        self.as_mut().set_batch_profile_name(qstring(&profile_name));
        self.as_mut()
            .set_total_files(saturating_i32(progress.total_files));
        self.as_mut()
            .set_scanned_files(saturating_i32(progress.scanned_files));
        self.as_mut()
            .set_cache_reused_files(saturating_i32(progress.cache_reused_files));
        self.as_mut()
            .set_content_read_files(saturating_i32(progress.content_read_files));
        self.as_mut()
            .set_current_file(qstring(progress.current_file));
        self.as_mut().set_message(qstring(if progress.total_files > 0 {
            format!(
                "Collection {position} of {total} · {profile_name} · {} of {} files · {} cached",
                progress.scanned_files, progress.total_files, progress.cache_reused_files,
            )
        } else {
            format!("Collection {position} of {total} · discovering {profile_name}…")
        }));
    }

    fn finish_profile_scan_batch(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<ProfileBatchCompletion, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        self.as_mut().set_scanning(false);
        self.as_mut().rust_mut().cancel = None;
        self.as_mut().set_batch_profile_name(QString::default());
        match result {
            Ok(completion) => {
                self.as_mut()
                    .set_batch_completed_count(saturating_i32(completion.processed));
                if let Some(history) = completion.history {
                    let count = history.len();
                    self.as_mut().rust_mut().history = history;
                    self.as_mut().set_history_count(saturating_i32(count));
                    self.as_mut().bump_history_revision();
                }
                let status = if completion.cancelled {
                    format!(
                        "Stopped after {} of {} saved collections",
                        completion.processed,
                        *self.as_ref().batch_profile_count()
                    )
                } else {
                    format!("Scanned {} saved collections", completion.processed)
                };
                let mut message = format!(
                    "{status}: {} files · {} exact · {} reviewed · {} other",
                    completion.total_files,
                    completion.exact_matches,
                    completion.reviewed_matches,
                    completion.other_results,
                );
                if completion.failures > 0 {
                    message.push_str(&format!(" · {} failed", completion.failures));
                }
                if !completion.warnings.is_empty() {
                    message.push_str(". ");
                    message.push_str(&completion.warnings.join(" "));
                } else {
                    message
                        .push_str(". Open Scan history, then load a profile to review its files.");
                }
                self.as_mut().set_message(qstring(message));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Saved-collection scan could not start: {error}"
            ))),
        }
        // Publish the complete counters/history snapshot before notifying QML that
        // the batch is finished. Completion handlers must never observe stale state.
        self.as_mut().set_batch_scanning(false);
    }

    pub fn start_scan(
        mut self: Pin<&mut Self>,
        platform: QString,
        extensions: QString,
        checksums_enabled: bool,
    ) {
        if *self.as_ref().busy() || *self.as_ref().profile_busy() {
            return;
        }
        let root = self
            .as_ref()
            .rust()
            .profile_root_override
            .clone()
            .unwrap_or_else(|| PathBuf::from(self.as_ref().directory().to_string()));
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
        let state_path = match crate::settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_message(qstring(format!(
                    "Could not locate Lunchbox state for ROM import: {error}"
                )));
                return;
            }
        };
        let extension_filter =
            match local_import::normalize_extension_filter(&extensions.to_string()) {
                Ok(extensions) => extensions,
                Err(error) => {
                    self.as_mut()
                        .set_message(qstring(format!("Invalid ROM extension filter: {error}")));
                    return;
                }
            };
        let platform = platform.to_string();
        let profile = usize::try_from(*self.as_ref().active_profile_index())
            .ok()
            .and_then(|index| self.as_ref().rust().profiles.get(index).cloned())
            .filter(|profile| {
                profile.root == root
                    && profile.platform_hint == platform
                    && profile.extensions == extension_filter
                    && profile.checksums_enabled == checksums_enabled
            });
        let history_context = profile.map_or_else(
            || {
                RomScanRunContext::one_time(
                    root.clone(),
                    platform.clone(),
                    extension_filter.clone(),
                    checksums_enabled,
                )
            },
            |profile| RomScanRunContext::for_profile(&profile, String::new(), 0, 0),
        );

        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().cancel = Some(Arc::clone(&cancel));
        self.as_mut().rust_mut().output = None;
        self.as_mut().rust_mut().visible_indices.clear();
        self.as_mut().set_busy(true);
        self.as_mut().set_scanning(true);
        self.as_mut().set_batch_scanning(false);
        self.as_mut().set_batch_profile_count(0);
        self.as_mut().set_batch_completed_count(0);
        self.as_mut().set_batch_profile_name(QString::default());
        self.as_mut().set_total_files(0);
        self.as_mut().set_scanned_files(0);
        self.as_mut().set_cache_reused_files(0);
        self.as_mut().set_content_read_files(0);
        self.as_mut().set_matched_count(0);
        self.as_mut().set_reviewed_count(0);
        self.as_mut().set_unmatched_count(0);
        self.as_mut().set_result_count(0);
        self.as_mut().set_selected_count(0);
        self.as_mut().set_current_file(QString::default());
        self.as_mut()
            .set_message(qstring("Discovering supported ROM files…"));
        self.as_mut().bump_revision();

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
                let scan = (|| -> anyhow::Result<ScanOutput> {
                    let mut output = local_import::scan_directory_cached_with_extensions(
                        &discovery_path,
                        &state_path,
                        &root,
                        &platform,
                        &extension_filter,
                        checksums_enabled,
                        &cancel,
                        progress,
                    )?;
                    local_import::restore_manual_matches(&state_path, &mut output)?;
                    Ok(output)
                })()
                .map_err(|error| error.to_string());
                let history = local_import::record_scan_run(
                    &state_path,
                    &history_context,
                    scan.as_ref().map_err(String::as_str),
                )
                .and_then(|_| local_import::load_scan_runs(&state_path))
                .map_err(|error| error.to_string());
                let result = CompletedScan { scan, history };
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
            .set_cache_reused_files(saturating_i32(progress.cache_reused_files));
        self.as_mut()
            .set_content_read_files(saturating_i32(progress.content_read_files));
        self.as_mut()
            .set_current_file(qstring(progress.current_file));
        if progress.total_files > 0 {
            self.as_mut().set_message(qstring(format!(
                "Verifying ROM {} of {}…  {} cached · {} read",
                progress.scanned_files,
                progress.total_files,
                progress.cache_reused_files,
                progress.content_read_files,
            )));
        }
    }

    fn finish_scan(mut self: Pin<&mut Self>, generation: u64, result: CompletedScan) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        self.as_mut().set_scanning(false);
        self.as_mut().rust_mut().cancel = None;
        let history_warning = match result.history {
            Ok(history) => {
                let count = history.len();
                self.as_mut().rust_mut().history = history;
                self.as_mut().set_history_count(saturating_i32(count));
                self.as_mut().bump_history_revision();
                None
            }
            Err(error) => Some(error),
        };
        match result.scan {
            Ok(output) => {
                let total = output.results.len();
                let matched = output
                    .results
                    .iter()
                    .filter(|result| matches!(result.match_state, MatchState::Exact(_)))
                    .count();
                let reviewed = output
                    .results
                    .iter()
                    .filter(|result| matches!(result.match_state, MatchState::Reviewed(_)))
                    .count();
                let selected = output
                    .results
                    .iter()
                    .filter(|result| result.selected)
                    .count();
                let cancelled = output.cancelled;
                let walk_errors = output.walk_errors;
                let cache_reused_files = output.cache_reused_files;
                let content_read_files = output.content_read_files;
                let checksum_summary = output.checksums_enabled.then(|| {
                    format!(
                        " Reused hashes for {cache_reused_files} unchanged files; read {content_read_files} new or changed files."
                    )
                });
                self.as_mut().rust_mut().visible_indices = (0..total).collect();
                self.as_mut().rust_mut().output = Some(output);
                self.as_mut().set_total_files(saturating_i32(total));
                self.as_mut().set_scanned_files(saturating_i32(total));
                self.as_mut()
                    .set_cache_reused_files(saturating_i32(cache_reused_files));
                self.as_mut()
                    .set_content_read_files(saturating_i32(content_read_files));
                self.as_mut().set_matched_count(saturating_i32(matched));
                self.as_mut().set_reviewed_count(saturating_i32(reviewed));
                self.as_mut().set_unmatched_count(saturating_i32(
                    total.saturating_sub(matched).saturating_sub(reviewed),
                ));
                self.as_mut().set_result_count(saturating_i32(total));
                self.as_mut().set_selected_count(saturating_i32(selected));
                self.as_mut().set_current_file(QString::default());
                self.as_mut().bump_revision();
                let mut message = if cancelled {
                    format!(
                        "Scan cancelled after {total} files; completed results remain available for review."
                    )
                } else if walk_errors > 0 {
                    format!(
                        "Scanned {total} ROMs: {matched} exact and {reviewed} reviewed matches. {walk_errors} filesystem entries could not be read."
                    )
                } else {
                    format!(
                        "Scanned {total} ROMs: {matched} exact, {reviewed} reviewed, and {} unmatched or ambiguous.",
                        total.saturating_sub(matched).saturating_sub(reviewed)
                    )
                };
                if let Some(summary) = checksum_summary {
                    message.push_str(&summary);
                }
                if let Some(warning) = history_warning {
                    message.push_str(&format!(" Scan history could not be retained: {warning}"));
                }
                self.as_mut().set_message(qstring(message));
            }
            Err(error) => {
                let suffix = history_warning
                    .map(|warning| format!(" Scan history could not be retained: {warning}"))
                    .unwrap_or_default();
                self.as_mut()
                    .set_message(qstring(format!("ROM scan failed: {error}.{suffix}")));
            }
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
                    "Imported {count} ROM files. Exact and explicitly reviewed matches now leave the Minerva ‘not installed’ view; local-only files remain visible in My Collection."
                )));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("ROM import failed: {error}"))),
        }
    }

    pub fn begin_manual_match(mut self: Pin<&mut Self>, index: i32) {
        let Some(result_index) = self.as_ref().visible_result_index(index) else {
            return;
        };
        let (file_name, archive_detail, query, platform) = {
            let model = self.as_ref();
            let Some(result) = model
                .rust()
                .output
                .as_ref()
                .and_then(|output| output.results.get(result_index))
            else {
                return;
            };
            if matches!(result.match_state, MatchState::Error(_)) {
                return;
            }
            (
                result.file_name.clone(),
                result.archive_member.clone(),
                result.display_title.clone(),
                (result.platform != "Unassigned").then(|| result.platform.clone()),
            )
        };
        self.as_mut().rust_mut().match_generation =
            self.as_ref().rust().match_generation.wrapping_add(1);
        self.as_mut().rust_mut().match_result_index = Some(result_index);
        self.as_mut().rust_mut().match_candidates.clear();
        self.as_mut().set_matching(false);
        self.as_mut().set_match_candidate_count(0);
        self.as_mut().set_match_file_name(qstring(file_name));
        self.as_mut()
            .set_match_archive_detail(qstring(archive_detail));
        self.as_mut().set_match_default_query(qstring(query));
        self.as_mut()
            .set_match_default_platform(qstring(platform.unwrap_or_default()));
        self.as_mut().set_match_message(qstring(
            "Search suggestions never establish identity. Choose one catalog record explicitly.",
        ));
        self.as_mut().bump_match_revision();
    }

    pub fn search_manual_matches(mut self: Pin<&mut Self>, query: QString, platform: QString) {
        if self.as_ref().rust().match_result_index.is_none() {
            return;
        }
        let query = query.to_string().trim().to_owned();
        if query.chars().count() < 2 {
            self.as_mut().rust_mut().match_generation =
                self.as_ref().rust().match_generation.wrapping_add(1);
            self.as_mut().rust_mut().match_candidates.clear();
            self.as_mut().set_matching(false);
            self.as_mut().set_match_candidate_count(0);
            self.as_mut()
                .set_match_message(qstring("Enter at least two characters to search."));
            self.as_mut().bump_match_revision();
            return;
        }
        let Some(discovery_path) = catalog::requested_discovery_database_path() else {
            self.as_mut()
                .set_match_message(qstring("The discovery games database is unavailable."));
            return;
        };
        let platform = platform.to_string();
        self.as_mut().rust_mut().match_generation =
            self.as_ref().rust().match_generation.wrapping_add(1);
        let generation = self.as_ref().rust().match_generation;
        self.as_mut().rust_mut().match_candidates.clear();
        self.as_mut().set_matching(true);
        self.as_mut().set_match_candidate_count(0);
        self.as_mut()
            .set_match_message(qstring(format!("Searching the catalog for “{query}”…")));
        self.as_mut().bump_match_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-manual-rom-match".into())
            .spawn(move || {
                let result = local_import::search_manual_match_candidates(
                    &discovery_path,
                    &query,
                    &platform,
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_manual_match_search(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_matching(false);
            self.as_mut()
                .set_match_message(qstring(format!("Could not start catalog search: {error}")));
        }
    }

    fn finish_manual_match_search(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<Vec<ManualMatchCandidate>, String>,
    ) {
        if generation != self.as_ref().rust().match_generation {
            return;
        }
        self.as_mut().set_matching(false);
        match result {
            Ok(candidates) => {
                let count = candidates.len();
                self.as_mut().rust_mut().match_candidates = candidates;
                self.as_mut()
                    .set_match_candidate_count(saturating_i32(count));
                self.as_mut().set_match_message(qstring(if count == 0 {
                    "No catalog records matched. Try a broader title or all platforms.".to_owned()
                } else {
                    format!(
                        "Found {count} possible catalog records. Nothing is linked until you choose one."
                    )
                }));
            }
            Err(error) => self
                .as_mut()
                .set_match_message(qstring(format!("Catalog search failed: {error}"))),
        }
        self.as_mut().bump_match_revision();
    }

    pub fn apply_manual_match(mut self: Pin<&mut Self>, index: i32) -> bool {
        let candidate = {
            let model = self.as_ref();
            usize::try_from(index)
                .ok()
                .and_then(|index| model.rust().match_candidates.get(index))
                .cloned()
        };
        let Some(candidate) = candidate else {
            return false;
        };
        let Some(result_index) = self.as_ref().rust().match_result_index else {
            return false;
        };
        let mut model = self.as_mut();
        let mut rust = model.as_mut().rust_mut();
        let result = rust
            .output
            .as_mut()
            .and_then(|output| output.results.get_mut(result_index));
        let Some(result) = result else {
            return false;
        };
        if let Err(error) = local_import::apply_manual_match(result, &candidate) {
            self.as_mut()
                .set_match_message(qstring(format!("Could not link ROM: {error}")));
            return false;
        }
        let title = candidate.identity.title.clone();
        let platform = candidate.identity.platform.clone();
        self.as_mut().update_match_counts();
        self.as_mut().update_selected_count();
        self.as_mut().bump_revision();
        self.as_mut().set_message(qstring(format!(
            "Linked this ROM to {title} ({platform}) by explicit review. Import to save the decision."
        )));
        true
    }

    pub fn cancel_manual_match(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().match_generation =
            self.as_ref().rust().match_generation.wrapping_add(1);
        self.as_mut().rust_mut().match_result_index = None;
        self.as_mut().rust_mut().match_candidates.clear();
        self.as_mut().set_matching(false);
        self.as_mut().set_match_candidate_count(0);
        self.as_mut().bump_match_revision();
    }

    pub fn match_candidate_title_at(&self, index: i32) -> QString {
        self.match_candidate_at(index)
            .map(|candidate| qstring(&candidate.identity.title))
            .unwrap_or_default()
    }

    pub fn match_candidate_platform_at(&self, index: i32) -> QString {
        self.match_candidate_at(index)
            .map(|candidate| qstring(&candidate.identity.platform))
            .unwrap_or_default()
    }

    pub fn match_candidate_detail_at(&self, index: i32) -> QString {
        self.match_candidate_at(index)
            .map(|candidate| {
                if candidate.matched_name.is_empty() {
                    qstring(format!(
                        "Canonical title · catalog ID {}",
                        candidate.identity.game_uid
                    ))
                } else {
                    qstring(format!(
                        "Alternate title: {} · catalog ID {}",
                        candidate.matched_name, candidate.identity.game_uid
                    ))
                }
            })
            .unwrap_or_default()
    }

    fn match_candidate_at(&self, index: i32) -> Option<&ManualMatchCandidate> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().match_candidates.get(index))
    }

    pub fn platform_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().platforms.get(index))
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn profile_name_at(&self, index: i32) -> QString {
        self.profile_at(index)
            .map(|profile| qstring(&profile.name))
            .unwrap_or_default()
    }

    pub fn profile_directory_at(&self, index: i32) -> QString {
        self.profile_at(index)
            .map(|profile| qstring(profile.root.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn profile_platform_at(&self, index: i32) -> QString {
        self.profile_at(index)
            .map(|profile| qstring(&profile.platform_hint))
            .unwrap_or_default()
    }

    pub fn profile_extensions_at(&self, index: i32) -> QString {
        self.profile_at(index)
            .map(|profile| qstring(local_import::format_extension_filter(&profile.extensions)))
            .unwrap_or_default()
    }

    pub fn profile_checksums_at(&self, index: i32) -> bool {
        self.profile_at(index)
            .is_some_and(|profile| profile.checksums_enabled)
    }

    pub fn profile_detail_at(&self, index: i32) -> QString {
        self.profile_at(index)
            .map(|profile| qstring(profile_scope_detail(profile)))
            .unwrap_or_default()
    }

    pub fn history_profile_name_at(&self, index: i32) -> QString {
        self.history_at(index)
            .map(|run| qstring(&run.profile_name))
            .unwrap_or_default()
    }

    pub fn history_directory_at(&self, index: i32) -> QString {
        self.history_at(index)
            .map(|run| qstring(run.root.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn history_scope_at(&self, index: i32) -> QString {
        self.history_at(index)
            .map(|run| qstring(scan_run_scope_detail(run)))
            .unwrap_or_default()
    }

    pub fn history_outcome_at(&self, index: i32) -> QString {
        self.history_at(index)
            .map(|run| qstring(&run.outcome))
            .unwrap_or_default()
    }

    pub fn history_summary_at(&self, index: i32) -> QString {
        self.history_at(index)
            .map(|run| {
                qstring(if run.outcome == "failed" {
                    "No files were reviewed".to_owned()
                } else {
                    let mut summary = format!(
                        "{} files · {} exact · {} reviewed · {} other",
                        run.total_files, run.exact_matches, run.reviewed_matches, run.other_results,
                    );
                    if run.walk_errors > 0 {
                        summary.push_str(&format!(" · {} unreadable", run.walk_errors));
                    }
                    if run.checksums_enabled {
                        summary.push_str(&format!(
                            " · {} cached / {} read",
                            run.cache_reused_files, run.content_read_files,
                        ));
                    }
                    summary
                })
            })
            .unwrap_or_default()
    }

    pub fn history_batch_at(&self, index: i32) -> QString {
        self.history_at(index)
            .map(|run| {
                qstring(if run.batch_id.is_empty() {
                    "Single scan".to_owned()
                } else {
                    format!(
                        "Batch {} of {} · {}",
                        run.batch_position,
                        run.batch_total,
                        &run.batch_id[..8.min(run.batch_id.len())],
                    )
                })
            })
            .unwrap_or_default()
    }

    pub fn history_finished_at(&self, index: i32) -> QString {
        self.history_at(index)
            .map(|run| qstring(run.finished_at.to_string()))
            .unwrap_or_default()
    }

    pub fn history_error_at(&self, index: i32) -> QString {
        self.history_at(index)
            .map(|run| qstring(&run.error_text))
            .unwrap_or_default()
    }

    pub fn history_profile_index_at(&self, index: i32) -> i32 {
        let Some(profile_id) = self.history_at(index).map(|run| run.profile_id.as_str()) else {
            return -1;
        };
        if profile_id.is_empty() {
            return -1;
        }
        self.rust()
            .profiles
            .iter()
            .position(|profile| profile.id == profile_id)
            .map(saturating_i32)
            .unwrap_or(-1)
    }

    fn profile_at(&self, index: i32) -> Option<&RomImportProfile> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().profiles.get(index))
    }

    fn history_at(&self, index: i32) -> Option<&RomScanRun> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().history.get(index))
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
                    MatchState::Reviewed(_) => "REVIEWED",
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
                    MatchState::Reviewed(identity) => {
                        format!("{} · explicitly linked by you", identity.title)
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

    fn reset_result_summary(mut self: Pin<&mut Self>) {
        self.as_mut().set_total_files(0);
        self.as_mut().set_scanned_files(0);
        self.as_mut().set_cache_reused_files(0);
        self.as_mut().set_content_read_files(0);
        self.as_mut().set_matched_count(0);
        self.as_mut().set_reviewed_count(0);
        self.as_mut().set_unmatched_count(0);
        self.as_mut().set_result_count(0);
        self.as_mut().set_selected_count(0);
        self.as_mut().set_current_file(QString::default());
        self.as_mut().bump_revision();
    }

    fn update_match_counts(mut self: Pin<&mut Self>) {
        let (exact, reviewed, total) = self
            .as_ref()
            .rust()
            .output
            .as_ref()
            .map(|output| {
                (
                    output
                        .results
                        .iter()
                        .filter(|result| matches!(result.match_state, MatchState::Exact(_)))
                        .count(),
                    output
                        .results
                        .iter()
                        .filter(|result| matches!(result.match_state, MatchState::Reviewed(_)))
                        .count(),
                    output.results.len(),
                )
            })
            .unwrap_or_default();
        self.as_mut().set_matched_count(saturating_i32(exact));
        self.as_mut().set_reviewed_count(saturating_i32(reviewed));
        self.as_mut().set_unmatched_count(saturating_i32(
            total.saturating_sub(exact).saturating_sub(reviewed),
        ));
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
    }

    fn bump_match_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().match_revision().wrapping_add(1);
        self.as_mut().set_match_revision(revision);
    }

    fn bump_profile_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().profile_revision().wrapping_add(1);
        self.as_mut().set_profile_revision(revision);
    }

    fn bump_history_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().history_revision().wrapping_add(1);
        self.as_mut().set_history_revision(revision);
    }
}

fn profile_scope_detail(profile: &RomImportProfile) -> String {
    let platform = if profile.platform_hint.is_empty() {
        "auto-detect platform".to_owned()
    } else {
        profile.platform_hint.clone()
    };
    let extensions = if profile.extensions.is_empty() {
        "all supported extensions".to_owned()
    } else {
        local_import::format_extension_filter(&profile.extensions)
    };
    let checksums = if profile.checksums_enabled {
        "exact checksums"
    } else {
        "inventory only"
    };
    format!("{platform} · {extensions} · {checksums}")
}

fn scan_run_scope_detail(run: &RomScanRun) -> String {
    let platform = if run.platform_hint.is_empty() {
        "auto-detect platform"
    } else {
        run.platform_hint.as_str()
    };
    let extensions = if run.extensions.is_empty() {
        "all supported extensions".to_owned()
    } else {
        local_import::format_extension_filter(&run.extensions)
    };
    let checksums = if run.checksums_enabled {
        "exact checksums"
    } else {
        "inventory only"
    };
    format!("{platform} · {extensions} · {checksums}")
}

fn matched_title(result: &ScanResult) -> &str {
    match &result.match_state {
        MatchState::Exact(identity) | MatchState::Reviewed(identity) => &identity.title,
        _ => &result.display_title,
    }
}

fn match_rank(state: &MatchState) -> u8 {
    match state {
        MatchState::Exact(_) => 0,
        MatchState::Reviewed(_) => 1,
        MatchState::Ambiguous(_) => 2,
        MatchState::Unmatched => 3,
        MatchState::InventoryOnly => 4,
        MatchState::Error(_) => 5,
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
        assert!(
            match_rank(&MatchState::Reviewed(MatchIdentityFixture::identity()))
                < match_rank(&MatchState::Ambiguous(2))
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
