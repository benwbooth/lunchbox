#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, results_json)]
        #[qproperty(QString, details_json)]
        #[qproperty(QString, package_json)]
        #[qproperty(QString, message)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, applying)]
        type PatchCatalogModel = super::PatchCatalogModelRust;
        #[qinvokable]
        fn select_game(self: Pin<&mut PatchCatalogModel>, game: QString, platform: QString);
        #[qinvokable]
        fn search(
            self: Pin<&mut PatchCatalogModel>,
            source: QString,
            query: QString,
            kind: QString,
        );
        #[qinvokable]
        fn show_entry(self: Pin<&mut PatchCatalogModel>, index: i32);
        #[qinvokable]
        fn download(self: Pin<&mut PatchCatalogModel>, index: i32);
        #[qinvokable]
        fn use_variant(
            self: Pin<&mut PatchCatalogModel>,
            index: i32,
            rom_path: QString,
            enable: bool,
        );
        #[qinvokable]
        fn save_plaza_key(self: Pin<&mut PatchCatalogModel>, key: QString);
        #[qinvokable]
        fn cancel(self: Pin<&mut PatchCatalogModel>);
        #[qsignal]
        fn installed(self: Pin<&mut PatchCatalogModel>);
    }
    impl cxx_qt::Threading for PatchCatalogModel {}
}
use crate::{
    game_mods::Profile,
    patch_catalog::{self as catalog, Entry, Package},
};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub struct PatchCatalogModelRust {
    results_json: QString,
    details_json: QString,
    package_json: QString,
    message: QString,
    busy: bool,
    applying: bool,
    game: String,
    platform: String,
    generation: u64,
    cancel: Arc<AtomicBool>,
    results: Vec<Entry>,
    selected: Option<Entry>,
    package: Option<Package>,
}
impl Default for PatchCatalogModelRust {
    fn default() -> Self {
        Self {
            results_json: "[]".into(),
            details_json: "null".into(),
            package_json: "null".into(),
            message: QString::default(),
            busy: false,
            applying: false,
            game: String::new(),
            platform: String::new(),
            generation: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            results: vec![],
            selected: None,
            package: None,
        }
    }
}
impl Drop for PatchCatalogModelRust {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
enum Outcome {
    Search(catalog::SearchReport),
    Details(Entry),
    Package(Package),
    Retry(Package, String),
    Installed(bool),
    Key,
}
impl qobject::PatchCatalogModel {
    pub fn select_game(mut self: Pin<&mut Self>, game: QString, platform: QString) {
        if self.rust().game == game.to_string() && self.rust().platform == platform.to_string() {
            return;
        }
        self.rust().cancel.store(true, Ordering::Relaxed);
        self.as_mut().rust_mut().generation += 1;
        self.as_mut().rust_mut().game = game.to_string();
        self.as_mut().rust_mut().platform = platform.to_string();
        self.as_mut().rust_mut().results.clear();
        self.as_mut().rust_mut().selected = None;
        self.as_mut().rust_mut().package = None;
        self.as_mut().set_results_json("[]".into());
        self.as_mut().set_details_json("null".into());
        self.as_mut().set_package_json("null".into());
        self.as_mut().set_message(QString::default());
        // Keep the global busy lock until the cancelled worker exits.
    }
    pub fn cancel(mut self: Pin<&mut Self>) {
        self.rust().cancel.store(true, Ordering::Relaxed);
        self.as_mut()
            .set_message("Cancelling patch operation…".into());
    }
    fn work(
        mut self: Pin<&mut Self>,
        status: &str,
        task: impl FnOnce(Arc<AtomicBool>, &mut dyn FnMut(u64, Option<u64>)) -> anyhow::Result<Outcome>
        + Send
        + 'static,
    ) {
        if *self.busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_message(status.into());
        let generation = self.rust().generation;
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().cancel = Arc::clone(&cancel);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let progress_thread = thread.clone();
            let mut last = Instant::now() - Duration::from_secs(1);
            let result = task(Arc::clone(&cancel), &mut |bytes, total| {
                if last.elapsed() < Duration::from_millis(200) {
                    return;
                }
                last = Instant::now();
                let message = match total {
                    Some(total) => format!(
                        "Downloading patch package: {:.1} / {:.1} MiB",
                        bytes as f64 / 1048576.,
                        total as f64 / 1048576.
                    ),
                    None => format!(
                        "Downloading patch package: {:.1} MiB",
                        bytes as f64 / 1048576.
                    ),
                };
                let _ = progress_thread.queue(move |mut model| {
                    if model.rust().generation == generation {
                        model.as_mut().set_message(message.into());
                    }
                });
            });
            let _ = thread.queue(move |mut model| {
                model.as_mut().set_busy(false);
                model.as_mut().set_applying(false);
                if model.rust().generation != generation { return; }
                match result {
                    Ok(Outcome::Search(report)) => {
                        let entries = report.entries;
                        let count = entries.len();
                        model.as_mut().set_results_json(serde_json::to_string(&entries).unwrap().into());
                        model.as_mut().rust_mut().results = entries;
                        model.as_mut().set_message(if !report.note.is_empty() { report.note.into() } else if count == 0 { "No matching entries. Try an alternate/Japanese title or another source.".into() } else { format!("{count} results. Choose one to review its requirements and files.").into() });
                    }
                    Ok(Outcome::Details(entry)) => {
                        model.as_mut().set_details_json(serde_json::to_string(&entry).unwrap().into());
                        model.as_mut().rust_mut().selected = Some(entry);
                        model.as_mut().set_message("Review the game, language and required ROM before downloading.".into());
                    }
                    Ok(Outcome::Package(package)) => {
                        model.as_mut().set_package_json(package.json().to_string().into());
                        model.as_mut().rust_mut().package = Some(package);
                        model.as_mut().set_message("Downloaded. Choose the correct variant; alternatives are not automatically stacked.".into());
                    }
                    Ok(Outcome::Retry(package, error)) => {
                        model.as_mut().set_package_json(package.json().to_string().into());
                        model.as_mut().rust_mut().package = Some(package);
                        model.as_mut().set_message(error.into());
                    }
                    Ok(Outcome::Installed(enabled)) => {
                        model.as_mut().set_package_json("null".into());
                        model.as_mut().set_message(if enabled { "Patched copy built and enabled. Press Play to use it; the original is unchanged. Patched saves are separate.".into() } else { "Patch imported, disabled. Manage it under Settings & mappings → Patches & cheats.".into() });
                        model.as_mut().installed();
                    }
                    Ok(Outcome::Key) => model.as_mut().set_message("Romhack Plaza connection saved in the OS credential store. Select that source and search to check access.".into()),
                    Err(error) => model.as_mut().set_message(QString::from(format!("{error:#}"))),
                }
            });
        });
    }
    pub fn search(mut self: Pin<&mut Self>, source: QString, query: QString, kind: QString) {
        if *self.busy() || self.rust().game.is_empty() {
            return;
        }
        self.as_mut().rust_mut().selected = None;
        self.as_mut().rust_mut().package = None;
        self.as_mut().rust_mut().results.clear();
        self.as_mut().set_results_json("[]".into());
        self.as_mut().set_details_json("null".into());
        self.as_mut().set_package_json("null".into());
        let platform = self.rust().platform.clone();
        let (source, query, kind) = (source.to_string(), query.to_string(), kind.to_string());
        self.work("Searching community patches…", move |cancel, _| {
            catalog::search_report(&source, &query, &platform, &kind, &cancel).map(Outcome::Search)
        });
    }
    pub fn show_entry(mut self: Pin<&mut Self>, index: i32) {
        if *self.busy() {
            return;
        }
        let Some(entry) = usize::try_from(index)
            .ok()
            .and_then(|i| self.rust().results.get(i))
            .cloned()
        else {
            return;
        };
        self.as_mut().rust_mut().package = None;
        self.as_mut().rust_mut().selected = None;
        self.as_mut().set_details_json("null".into());
        self.as_mut().set_package_json("null".into());
        self.work(
            "Loading patch details and available packages…",
            move |cancel, _| catalog::details(entry, &cancel).map(Outcome::Details),
        );
    }
    pub fn download(mut self: Pin<&mut Self>, index: i32) {
        if *self.busy() || index < 0 {
            return;
        }
        let Some(entry) = self.rust().selected.clone() else {
            return;
        };
        self.as_mut().rust_mut().package = None;
        self.as_mut().set_package_json("null".into());
        self.work("Downloading patch package…", move |cancel, progress| {
            catalog::download(entry, index as usize, &cancel, progress).map(Outcome::Package)
        });
    }
    pub fn save_plaza_key(self: Pin<&mut Self>, key: QString) {
        let key = key.to_string();
        self.work("Saving source connection…", move |_, _| {
            catalog::save_key(&key)?;
            Ok(Outcome::Key)
        });
    }
    pub fn use_variant(mut self: Pin<&mut Self>, index: i32, rom_path: QString, enable: bool) {
        if *self.busy() || index < 0 {
            return;
        }
        let Some(package) = self.as_mut().rust_mut().package.take() else {
            return;
        };
        let game = self.rust().game.clone();
        let source = std::path::PathBuf::from(rom_path.to_string());
        self.as_mut().set_applying(true);
        self.as_mut().set_package_json("null".into());
        self.work(
            if enable {
                "Checking the ROM and building its patched copy…"
            } else {
                "Importing selected patch…"
            },
            move |cancel, _| {
                let result = (|| -> anyhow::Result<()> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let mut profile = Profile::load(&store, &game)?;
                    let before = serde_json::to_string(&profile)?;
                    let mut patch = catalog::import_variant(&package, index as usize)?;
                    patch.enabled = enable;
                    if let Some(existing) = profile
                        .patches
                        .iter_mut()
                        .find(|p| p.sha256 == patch.sha256)
                    {
                        anyhow::ensure!(enable || !existing.enabled,
                            "This patch is already enabled. Manage it under Settings & mappings → Patches & cheats.");
                        // Refresh provenance/requirements even if this file was
                        // previously imported manually, without disabling it.
                        patch.enabled |= existing.enabled;
                        *existing = patch;
                    } else {
                        profile.patches.push(patch);
                    }
                    if enable {
                        anyhow::ensure!(
                            source.is_file(),
                            "Install/select the required ROM before applying this patch"
                        );
                        let media = crate::rom_launch_preparation::prepare_for_launch(
                            &source, false, &cancel,
                        )?;
                        struct Cleanup(Vec<std::path::PathBuf>);
                        impl Drop for Cleanup {
                            fn drop(&mut self) {
                                crate::emulator::cleanup_after_launch(&self.0);
                            }
                        }
                        let _cleanup = Cleanup(media.cleanup_paths.clone());
                        crate::game_mods::prepare(&media.path, &profile, &game, &cancel)?;
                    }
                    anyhow::ensure!(!cancel.load(Ordering::Relaxed), "Patch operation cancelled");
                    anyhow::ensure!(
                        serde_json::to_string(&Profile::load(&store, &game)?)? == before,
                        "The game's patch list changed during preparation. Please try again."
                    );
                    profile.save(&store, &game)?;
                    Ok(())
                })();
                Ok(match result {
                    Ok(()) => Outcome::Installed(enable),
                    Err(error) => Outcome::Retry(package, format!("{error:#}")),
                })
            },
        );
    }
}
