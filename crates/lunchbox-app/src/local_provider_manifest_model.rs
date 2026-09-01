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
        #[qproperty(i32, entry_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, catalog_revision)]
        #[qproperty(QString, message)]
        type LocalProviderManifestModel = super::LocalProviderManifestModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut LocalProviderManifestModel>);

        #[qinvokable]
        fn refresh(self: Pin<&mut LocalProviderManifestModel>);

        #[qinvokable]
        fn choose_and_import(self: Pin<&mut LocalProviderManifestModel>);

        #[qinvokable]
        fn import_path(self: Pin<&mut LocalProviderManifestModel>, source: QUrl);

        #[qinvokable]
        fn resync_at(self: Pin<&mut LocalProviderManifestModel>, index: i32);

        #[qinvokable]
        fn remove_at(self: Pin<&mut LocalProviderManifestModel>, index: i32);

        #[qinvokable]
        fn provider_id_at(self: &LocalProviderManifestModel, index: i32) -> QString;

        #[qinvokable]
        fn name_at(self: &LocalProviderManifestModel, index: i32) -> QString;

        #[qinvokable]
        fn version_at(self: &LocalProviderManifestModel, index: i32) -> QString;

        #[qinvokable]
        fn terms_url_at(self: &LocalProviderManifestModel, index: i32) -> QString;

        #[qinvokable]
        fn path_at(self: &LocalProviderManifestModel, index: i32) -> QString;

        #[qinvokable]
        fn path_url_at(self: &LocalProviderManifestModel, index: i32) -> QUrl;

        #[qinvokable]
        fn offer_count_at(self: &LocalProviderManifestModel, index: i32) -> i32;

        #[qinvokable]
        fn file_available_at(self: &LocalProviderManifestModel, index: i32) -> bool;
    }

    impl cxx_qt::Threading for LocalProviderManifestModel {}
}

use std::path::PathBuf;
use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::local_provider_manifest::{LocalProviderImportOutcome, LocalProviderManifestSummary};
use crate::settings::SettingsStore;

pub struct LocalProviderManifestModelRust {
    initialized: bool,
    busy: bool,
    entry_count: i32,
    revision: i32,
    catalog_revision: i32,
    message: QString,
    entries: Vec<LocalProviderManifestSummary>,
    generation: u64,
}

impl Default for LocalProviderManifestModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            entry_count: 0,
            revision: 0,
            catalog_revision: 0,
            message: QString::from(
                "Import a user-managed provider manifest to index local torrent catalogs without queueing ROM payloads.",
            ),
            entries: Vec::new(),
            generation: 0,
        }
    }
}

enum ManifestOperationResult {
    Imported(LocalProviderImportOutcome),
    Removed { display_name: String },
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::LocalProviderManifestModel {
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
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Refreshing local provider catalogs…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-provider-refresh".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| crate::local_provider_manifest::list_manifests(&store))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_refresh(generation, result, None);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start provider catalog refresh: {error}"
            )));
        }
    }

    pub fn choose_and_import(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let chosen = rfd::FileDialog::new()
            .set_title("Import local provider manifest")
            .add_filter("Lunchbox provider manifest", &["json"])
            .pick_file();
        if let Some(path) = chosen {
            self.as_mut().start_import(path);
        }
    }

    pub fn import_path(mut self: Pin<&mut Self>, source: QUrl) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(path) = source
            .to_local_file()
            .map(|path| PathBuf::from(path.to_string()))
        else {
            self.as_mut()
                .set_message(qstring("Choose a local provider manifest JSON file."));
            return;
        };
        self.as_mut().start_import(path);
    }

    pub fn resync_at(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(path) = self
            .as_ref()
            .entry(index)
            .map(|entry| entry.manifest_path.clone())
        else {
            self.as_mut()
                .set_message(qstring("That provider manifest is no longer listed."));
            return;
        };
        self.as_mut().start_import(path);
    }

    pub fn remove_at(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() {
            return;
        }
        let Some((provider_id, display_name)) = self
            .as_ref()
            .entry(index)
            .map(|entry| (entry.provider_id.clone(), entry.display_name.clone()))
        else {
            self.as_mut()
                .set_message(qstring("That provider manifest is no longer listed."));
            return;
        };
        self.as_mut().start_remove(provider_id, display_name);
    }

    fn start_import(mut self: Pin<&mut Self>, path: PathBuf) {
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Validating and indexing provider metadata…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-provider-import".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| {
                        crate::local_provider_manifest::import_manifest(&path, &store)
                    })
                    .map(ManifestOperationResult::Imported)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_operation(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start provider manifest import: {error}"
            )));
        }
    }

    fn start_remove(mut self: Pin<&mut Self>, provider_id: String, display_name: String) {
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring(format!("Removing {display_name} metadata…")));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-provider-remove".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| {
                        crate::local_provider_manifest::remove_manifest(&store, &provider_id)
                    })
                    .map(|_| ManifestOperationResult::Removed { display_name })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_operation(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start provider removal: {error}"
            )));
        }
    }

    fn finish_operation(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<ManifestOperationResult, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        match result {
            Ok(ManifestOperationResult::Imported(outcome)) => {
                let notice = format!(
                    "Indexed {} catalog{} from {} · version {}.",
                    outcome.offer_count,
                    if outcome.offer_count == 1 { "" } else { "s" },
                    outcome.display_name,
                    outcome.catalog_version
                );
                self.as_mut().bump_catalog_revision();
                self.as_mut().load_after_operation(generation, notice);
            }
            Ok(ManifestOperationResult::Removed { display_name }) => {
                self.as_mut().bump_catalog_revision();
                self.as_mut().load_after_operation(
                    generation,
                    format!(
                        "Removed {display_name} metadata. Torrent and ROM files were untouched."
                    ),
                );
            }
            Err(error) => {
                self.as_mut().set_busy(false);
                self.as_mut().set_message(qstring(format!(
                    "Provider catalog was not changed: {error}"
                )));
            }
        }
    }

    fn load_after_operation(mut self: Pin<&mut Self>, generation: u64, notice: String) {
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-provider-reload".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| crate::local_provider_manifest::list_manifests(&store))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_refresh(generation, result, Some(notice));
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Catalog changed, but the provider list could not reload: {error}"
            )));
        }
    }

    fn finish_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<Vec<LocalProviderManifestSummary>, String>,
        notice: Option<String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(entries) => {
                self.as_mut()
                    .set_entry_count(i32::try_from(entries.len()).unwrap_or(i32::MAX));
                self.as_mut().rust_mut().entries = entries;
                let message = notice.unwrap_or_else(|| match *self.as_ref().entry_count() {
                    0 => "No local provider manifests are installed.".to_owned(),
                    1 => "1 local provider manifest is installed.".to_owned(),
                    count => format!("{count} local provider manifests are installed."),
                });
                self.as_mut().set_message(qstring(message));
                let next = self.as_ref().revision().wrapping_add(1);
                self.as_mut().set_revision(next);
            }
            Err(error) => {
                self.as_mut().rust_mut().entries.clear();
                self.as_mut().set_entry_count(0);
                self.as_mut().set_message(qstring(format!(
                    "Could not load provider catalogs: {error}"
                )));
            }
        }
    }

    fn bump_catalog_revision(mut self: Pin<&mut Self>) {
        let next = self.as_ref().catalog_revision().wrapping_add(1);
        self.as_mut().set_catalog_revision(next);
    }

    pub fn provider_id_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.provider_id))
            .unwrap_or_default()
    }

    pub fn name_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.display_name))
            .unwrap_or_default()
    }

    pub fn version_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.catalog_version))
            .unwrap_or_default()
    }

    pub fn terms_url_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.terms_url))
            .unwrap_or_default()
    }

    pub fn path_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(entry.manifest_path.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn path_url_at(&self, index: i32) -> QUrl {
        self.entry(index)
            .map(|entry| QUrl::from_local_file(&qstring(entry.manifest_path.to_string_lossy())))
            .unwrap_or_default()
    }

    pub fn offer_count_at(&self, index: i32) -> i32 {
        self.entry(index)
            .map(|entry| i32::try_from(entry.offer_count).unwrap_or(i32::MAX))
            .unwrap_or(0)
    }

    pub fn file_available_at(&self, index: i32) -> bool {
        self.entry(index)
            .is_some_and(|entry| entry.manifest_path.is_file())
    }

    fn entry(&self, index: i32) -> Option<&LocalProviderManifestSummary> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().entries.get(index))
    }
}
