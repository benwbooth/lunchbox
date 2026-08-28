#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, panel_open)]
        #[qproperty(bool, loading)]
        #[qproperty(bool, torrent_loading)]
        #[qproperty(bool, download_busy)]
        #[qproperty(bool, prepare_busy)]
        #[qproperty(bool, launch_discovery_busy)]
        #[qproperty(bool, launch_busy)]
        #[qproperty(bool, can_launch)]
        #[qproperty(bool, game_running)]
        #[qproperty(QString, game_id)]
        #[qproperty(QString, title)]
        #[qproperty(QString, platform)]
        #[qproperty(QString, description)]
        #[qproperty(QString, release_date)]
        #[qproperty(QString, developer)]
        #[qproperty(QString, publisher)]
        #[qproperty(QString, genre)]
        #[qproperty(QString, players)]
        #[qproperty(QString, rating)]
        #[qproperty(QString, esrb)]
        #[qproperty(QString, release_type)]
        #[qproperty(QString, notes)]
        #[qproperty(QString, message)]
        #[qproperty(bool, local)]
        #[qproperty(bool, downloadable)]
        #[qproperty(bool, preparable)]
        #[qproperty(bool, prepared)]
        #[qproperty(QString, preparation_phase)]
        #[qproperty(QString, preparation_file)]
        #[qproperty(QString, prepared_summary)]
        #[qproperty(QString, emulator_name)]
        #[qproperty(QString, emulator_summary)]
        #[qproperty(QString, launch_status)]
        #[qproperty(i32, bundle_count)]
        #[qproperty(i32, file_count)]
        #[qproperty(i32, selected_bundle)]
        #[qproperty(i32, detail_revision)]
        type GameDetailsModel = super::GameDetailsModelRust;

        #[qinvokable]
        fn select_game(
            self: Pin<&mut GameDetailsModel>,
            game_id: QString,
            title: QString,
            platform: QString,
            local: bool,
            downloadable: bool,
        );

        #[qinvokable]
        fn close_panel(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn load_bundle_files(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn queue_file(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn prepare_game(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn cancel_preparation(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn refresh_emulators(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn launch_game(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn bundle_title_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn bundle_detail_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn file_name_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn file_detail_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn file_has_download_plan(self: &GameDetailsModel, index: i32) -> bool;

        #[qinvokable]
        fn file_plan_summary_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn file_plan_kind_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn file_plan_members_at(self: &GameDetailsModel, index: i32) -> QString;
    }

    impl cxx_qt::Threading for GameDetailsModel {}
}

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

use anyhow::Context;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::game_details::{
    self, GameDetails, MinervaBundle, ReleasePreferences, TorrentFileCandidate,
};

pub struct GameDetailsModelRust {
    panel_open: bool,
    loading: bool,
    torrent_loading: bool,
    download_busy: bool,
    prepare_busy: bool,
    launch_discovery_busy: bool,
    launch_busy: bool,
    can_launch: bool,
    game_running: bool,
    game_id: QString,
    title: QString,
    platform: QString,
    description: QString,
    release_date: QString,
    developer: QString,
    publisher: QString,
    genre: QString,
    players: QString,
    rating: QString,
    esrb: QString,
    release_type: QString,
    notes: QString,
    message: QString,
    local: bool,
    downloadable: bool,
    preparable: bool,
    prepared: bool,
    preparation_phase: QString,
    preparation_file: QString,
    prepared_summary: QString,
    emulator_name: QString,
    emulator_summary: QString,
    launch_status: QString,
    bundle_count: i32,
    file_count: i32,
    selected_bundle: i32,
    detail_revision: i32,
    bundles: Vec<MinervaBundle>,
    files: Vec<TorrentFileCandidate>,
    details_generation: u64,
    torrent_generation: u64,
    preparation_generation: u64,
    launch_generation: u64,
    preparation_cancel: Option<Arc<AtomicBool>>,
    database_id: i64,
    local_file_path: PathBuf,
    prepared_install: Option<crate::exo_install::PreparedInstall>,
}

impl Default for GameDetailsModelRust {
    fn default() -> Self {
        Self {
            panel_open: false,
            loading: false,
            torrent_loading: false,
            download_busy: false,
            prepare_busy: false,
            launch_discovery_busy: false,
            launch_busy: false,
            can_launch: false,
            game_running: false,
            game_id: QString::default(),
            title: QString::default(),
            platform: QString::default(),
            description: QString::default(),
            release_date: QString::default(),
            developer: QString::default(),
            publisher: QString::default(),
            genre: QString::default(),
            players: QString::default(),
            rating: QString::default(),
            esrb: QString::default(),
            release_type: QString::default(),
            notes: QString::default(),
            message: QString::from("Select a game to inspect it."),
            local: false,
            downloadable: false,
            preparable: false,
            prepared: false,
            preparation_phase: QString::default(),
            preparation_file: QString::default(),
            prepared_summary: QString::default(),
            emulator_name: QString::default(),
            emulator_summary: QString::default(),
            launch_status: QString::default(),
            bundle_count: 0,
            file_count: 0,
            selected_bundle: -1,
            detail_revision: 0,
            bundles: Vec::new(),
            files: Vec::new(),
            details_generation: 0,
            torrent_generation: 0,
            preparation_generation: 0,
            launch_generation: 0,
            preparation_cancel: None,
            database_id: 0,
            local_file_path: PathBuf::new(),
            prepared_install: None,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn count_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn has_cli_flag(flag: &str) -> bool {
    std::env::args_os().any(|argument| argument == flag)
}

impl qobject::GameDetailsModel {
    pub fn select_game(
        mut self: Pin<&mut Self>,
        game_id: QString,
        title: QString,
        platform: QString,
        local: bool,
        downloadable: bool,
    ) {
        let game_id_string = game_id.to_string();
        let title_string = title.to_string();
        let platform_string = platform.to_string();
        self.as_mut().invalidate_preparation();
        self.as_mut().invalidate_launch_state();
        self.as_mut().rust_mut().details_generation =
            self.as_ref().rust().details_generation.wrapping_add(1);
        self.as_mut().rust_mut().torrent_generation =
            self.as_ref().rust().torrent_generation.wrapping_add(1);
        let generation = self.as_ref().rust().details_generation;

        self.as_mut().set_panel_open(true);
        self.as_mut().set_loading(true);
        self.as_mut().set_torrent_loading(false);
        self.as_mut().set_game_id(game_id);
        self.as_mut().set_title(title);
        self.as_mut().set_platform(platform);
        self.as_mut().set_local(local);
        self.as_mut().set_downloadable(downloadable);
        self.as_mut().clear_details();
        self.as_mut()
            .set_message(qstring("Loading game details and Minerva sources…"));

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-game-details".into())
            .spawn(move || {
                let loaded = game_details::load(
                    &game_id_string,
                    &title_string,
                    &platform_string,
                    local,
                    downloadable,
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_game_details(generation, loaded);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_loading(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start details worker: {error}")));
        }
    }

    pub fn close_panel(mut self: Pin<&mut Self>) {
        self.as_mut().invalidate_preparation();
        self.as_mut().invalidate_launch_state();
        self.as_mut().rust_mut().details_generation =
            self.as_ref().rust().details_generation.wrapping_add(1);
        self.as_mut().rust_mut().torrent_generation =
            self.as_ref().rust().torrent_generation.wrapping_add(1);
        self.as_mut().set_loading(false);
        self.as_mut().set_torrent_loading(false);
        self.as_mut().set_prepare_busy(false);
        self.as_mut().set_launch_discovery_busy(false);
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_panel_open(false);
    }

    fn clear_details(mut self: Pin<&mut Self>) {
        self.as_mut().set_description(QString::default());
        self.as_mut().set_release_date(QString::default());
        self.as_mut().set_developer(QString::default());
        self.as_mut().set_publisher(QString::default());
        self.as_mut().set_genre(QString::default());
        self.as_mut().set_players(QString::default());
        self.as_mut().set_rating(QString::default());
        self.as_mut().set_esrb(QString::default());
        self.as_mut().set_release_type(QString::default());
        self.as_mut().set_notes(QString::default());
        self.as_mut().set_preparable(false);
        self.as_mut().set_prepared(false);
        self.as_mut().set_preparation_phase(QString::default());
        self.as_mut().set_preparation_file(QString::default());
        self.as_mut().set_prepared_summary(QString::default());
        self.as_mut().set_emulator_name(QString::default());
        self.as_mut().set_emulator_summary(QString::default());
        self.as_mut().set_launch_status(QString::default());
        self.as_mut().set_can_launch(false);
        self.as_mut().set_game_running(false);
        self.as_mut().rust_mut().database_id = 0;
        self.as_mut().rust_mut().local_file_path = PathBuf::new();
        self.as_mut().rust_mut().prepared_install = None;
        self.as_mut().rust_mut().bundles.clear();
        self.as_mut().rust_mut().files.clear();
        self.as_mut().set_bundle_count(0);
        self.as_mut().set_file_count(0);
        self.as_mut().set_selected_bundle(-1);
        self.as_mut().bump_revision();
    }

    fn finish_game_details(
        mut self: Pin<&mut Self>,
        generation: u64,
        loaded: Result<GameDetails, String>,
    ) {
        if generation != self.as_ref().rust().details_generation {
            return;
        }
        self.as_mut().set_loading(false);
        match loaded {
            Ok(details) => {
                let preparable = crate::exo_install::is_preparable_archive(
                    &details.platform,
                    &details.local_file_path,
                );
                let prepared_summary = details
                    .prepared_install
                    .as_ref()
                    .map(|prepared| {
                        format!(
                            "{} prepared · {}",
                            prepared.collection.display_name(),
                            prepared.launch_config_path.display()
                        )
                    })
                    .unwrap_or_default();
                let prepared = details.prepared_install.is_some();
                let prepared_install = details.prepared_install.clone();
                self.as_mut().set_description(qstring(&details.description));
                self.as_mut()
                    .set_release_date(qstring(&details.release_date));
                self.as_mut().set_developer(qstring(&details.developer));
                self.as_mut().set_publisher(qstring(&details.publisher));
                self.as_mut().set_genre(qstring(&details.genre));
                self.as_mut().set_players(qstring(&details.players));
                self.as_mut().set_rating(qstring(&details.rating));
                self.as_mut().set_esrb(qstring(&details.esrb));
                self.as_mut()
                    .set_release_type(qstring(&details.release_type));
                self.as_mut().set_notes(qstring(&details.notes));
                self.as_mut().rust_mut().database_id = details.database_id;
                self.as_mut().set_local(details.local);
                self.as_mut().set_downloadable(details.downloadable);
                self.as_mut().rust_mut().local_file_path = details.local_file_path;
                self.as_mut().rust_mut().prepared_install = prepared_install;
                self.as_mut().set_preparable(preparable);
                self.as_mut().set_prepared(prepared);
                self.as_mut()
                    .set_prepared_summary(qstring(prepared_summary));
                let bundle_count = details.bundles.len();
                self.as_mut().rust_mut().bundles = details.bundles;
                self.as_mut().set_bundle_count(count_i32(bundle_count));
                self.as_mut().bump_revision();
                let message = if details.local {
                    "Installed in your collection".to_owned()
                } else if bundle_count == 0 {
                    "No exact Minerva platform source was found for this game.".to_owned()
                } else {
                    format!("{bundle_count} Minerva source bundles available")
                };
                self.as_mut().set_message(qstring(message));
                if prepared {
                    self.as_mut().refresh_emulators();
                }
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not load game details: {error}"))),
        }
    }

    pub fn load_bundle_files(mut self: Pin<&mut Self>, index: i32) {
        let Some(bundle_index) = usize::try_from(index).ok() else {
            self.as_mut().set_message(qstring(
                "The selected Minerva source is no longer available.",
            ));
            return;
        };
        let bundle = self.as_ref().rust().bundles.get(bundle_index).cloned();
        let Some(bundle) = bundle else {
            self.as_mut().set_message(qstring(
                "The selected Minerva source is no longer available.",
            ));
            return;
        };
        if *self.as_ref().torrent_loading() {
            return;
        }
        self.as_mut().rust_mut().torrent_generation =
            self.as_ref().rust().torrent_generation.wrapping_add(1);
        let generation = self.as_ref().rust().torrent_generation;
        let title = self.as_ref().title().to_string();
        self.as_mut().rust_mut().files.clear();
        self.as_mut().set_file_count(0);
        self.as_mut().set_selected_bundle(index);
        self.as_mut().set_torrent_loading(true);
        self.as_mut()
            .set_message(qstring("Fetching and matching torrent contents…"));
        self.as_mut().bump_revision();

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-torrent-metadata".into())
            .spawn(move || {
                let loaded = (|| {
                    let settings = crate::settings::SettingsStore::open_default()?.load()?;
                    game_details::load_torrent_files(
                        &bundle,
                        &title,
                        &ReleasePreferences {
                            preferred_region: settings.preferred_region,
                            version_preference: settings.version_preference,
                        },
                    )
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_torrent_files(generation, loaded);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_torrent_loading(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start torrent metadata worker: {error}"
            )));
        }
    }

    fn finish_torrent_files(
        mut self: Pin<&mut Self>,
        generation: u64,
        loaded: Result<Vec<TorrentFileCandidate>, String>,
    ) {
        if generation != self.as_ref().rust().torrent_generation {
            return;
        }
        self.as_mut().set_torrent_loading(false);
        match loaded {
            Ok(files) => {
                let count = files.len();
                self.as_mut().rust_mut().files = files;
                self.as_mut().set_file_count(count_i32(count));
                self.as_mut().bump_revision();
                self.as_mut().set_message(qstring(if count == 0 {
                    "No title candidates were found in this bundle. Try another source."
                        .to_owned()
                } else {
                    format!("{count} candidate files ranked by title, region, and release version; review the exact filename before downloading")
                }));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not inspect torrent contents: {error}"
            ))),
        }
    }

    pub fn queue_file(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().download_busy() || *self.as_ref().torrent_loading() {
            return;
        }
        let Some(file) = self.as_ref().file(index).cloned() else {
            self.as_mut().set_message(qstring(
                "The selected torrent file is no longer available. Inspect the source again.",
            ));
            return;
        };
        let selected_bundle = *self.as_ref().selected_bundle();
        let Some(bundle) = self.as_ref().bundle(selected_bundle).cloned() else {
            self.as_mut().set_message(qstring(
                "The selected Minerva source is no longer available.",
            ));
            return;
        };
        let game_id = self.as_ref().game_id().to_string();
        let completed_game_id = game_id.clone();
        let launchbox_db_id = self.as_ref().rust().database_id;
        let title = self.as_ref().title().to_string();
        let platform = self.as_ref().platform().to_string();
        self.as_mut().set_download_busy(true);
        self.as_mut().set_message(qstring(
            if file
                .download_plan
                .as_ref()
                .is_some_and(|plan| plan.is_optical_multidisc())
            {
                "Adding the reviewed multi-disc set to qBittorrent…"
            } else if file.download_plan.is_some() {
                "Adding the reviewed eXo archive set to qBittorrent…"
            } else {
                "Adding the reviewed file to qBittorrent…"
            },
        ));

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-minerva-enqueue".into())
            .spawn(move || {
                let queued =
                    queue_download(game_id, launchbox_db_id, title, platform, bundle, file)
                        .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_queue(completed_game_id, queued);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_download_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start download worker: {error}")));
        }
    }

    pub fn prepare_game(mut self: Pin<&mut Self>) {
        if *self.as_ref().prepare_busy() {
            return;
        }
        if !*self.as_ref().preparable() {
            self.as_mut().set_message(qstring(
                "This installed file is not an exact eXoDOS, eXoWin3x, or eXoWin9x game archive.",
            ));
            return;
        }
        let archive_path = self.as_ref().rust().local_file_path.clone();
        if !archive_path.is_file() {
            self.as_mut().set_preparable(false);
            self.as_mut().set_message(qstring(format!(
                "The installed archive is no longer available: {}",
                archive_path.display()
            )));
            return;
        }

        self.as_mut().rust_mut().preparation_generation =
            self.as_ref().rust().preparation_generation.wrapping_add(1);
        let generation = self.as_ref().rust().preparation_generation;
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().preparation_cancel = Some(Arc::clone(&cancel));
        let game_id = self.as_ref().game_id().to_string();
        let completed_game_id = game_id.clone();
        let launchbox_db_id = self.as_ref().rust().database_id;
        let platform = self.as_ref().platform().to_string();
        self.as_mut().set_prepare_busy(true);
        self.as_mut()
            .set_preparation_phase(qstring("Preparing install"));
        self.as_mut().set_preparation_file(QString::default());
        self.as_mut().set_message(qstring(
            "Preparing the exact eXo game, metadata, and shared runtime assets…",
        ));

        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-exo-prepare".into())
            .spawn(move || {
                let mut last_update = Instant::now()
                    .checked_sub(Duration::from_secs(1))
                    .unwrap_or_else(Instant::now);
                let mut last_phase = String::new();
                let prepared = (|| -> anyhow::Result<crate::exo_install::PreparedInstall> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    crate::exo_install::prepare_install(
                        &settings,
                        &store,
                        &game_id,
                        launchbox_db_id,
                        &platform,
                        &archive_path,
                        &cancel,
                        |progress| {
                            let phase_changed = progress.phase != last_phase;
                            if phase_changed || last_update.elapsed() >= Duration::from_millis(100)
                            {
                                last_phase.clone_from(&progress.phase);
                                last_update = Instant::now();
                                let progress_generation = generation;
                                let _ = progress_thread.queue(move |mut model| {
                                    model
                                        .as_mut()
                                        .update_preparation_progress(progress_generation, progress);
                                });
                            }
                        },
                    )
                })()
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_preparation(generation, completed_game_id, prepared);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_prepare_busy(false);
            self.as_mut().rust_mut().preparation_cancel = None;
            self.as_mut().set_message(qstring(format!(
                "Could not start eXo preparation worker: {error}"
            )));
        }
    }

    pub fn cancel_preparation(mut self: Pin<&mut Self>) {
        if let Some(cancel) = self.as_ref().rust().preparation_cancel.as_ref() {
            cancel.store(true, AtomicOrdering::Relaxed);
            self.as_mut().set_preparation_phase(qstring("Cancelling…"));
            self.as_mut().set_message(qstring(
                "Cancelling eXo preparation and removing the private staging directory…",
            ));
        }
    }

    pub fn refresh_emulators(mut self: Pin<&mut Self>) {
        if *self.as_ref().launch_discovery_busy() || *self.as_ref().launch_busy() {
            return;
        }
        let Some(prepared) = self.as_ref().rust().prepared_install.clone() else {
            self.as_mut().set_can_launch(false);
            self.as_mut().set_emulator_name(QString::default());
            self.as_mut().set_emulator_summary(QString::default());
            self.as_mut().set_launch_status(qstring(
                "Prepare this PC install before detecting emulators.",
            ));
            return;
        };
        let Some(catalog_database) = crate::catalog::requested_database_path() else {
            self.as_mut().set_can_launch(false);
            self.as_mut().set_launch_status(qstring(
                "The canonical Lunchbox database is unavailable, so emulator metadata cannot be loaded.",
            ));
            return;
        };

        self.as_mut().rust_mut().launch_generation =
            self.as_ref().rust().launch_generation.wrapping_add(1);
        let generation = self.as_ref().rust().launch_generation;
        let game_id = self.as_ref().game_id().to_string();
        self.as_mut().set_launch_discovery_busy(true);
        self.as_mut().set_can_launch(false);
        self.as_mut().set_emulator_name(QString::default());
        self.as_mut().set_emulator_summary(QString::default());
        self.as_mut()
            .set_launch_status(qstring("Detecting compatible emulators…"));

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-emulator-discovery".into())
            .spawn(move || {
                let availability =
                    crate::emulator::inspect_launch_availability(&prepared, &catalog_database)
                        .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_emulator_discovery(generation, game_id, availability);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_launch_discovery_busy(false);
            self.as_mut().set_launch_status(qstring(format!(
                "Could not start emulator detection: {error}"
            )));
        }
    }

    fn finish_emulator_discovery(
        mut self: Pin<&mut Self>,
        generation: u64,
        completed_game_id: String,
        availability: Result<crate::emulator::LaunchAvailability, String>,
    ) {
        if generation != self.as_ref().rust().launch_generation
            || self.as_ref().game_id().to_string() != completed_game_id
        {
            return;
        }
        self.as_mut().set_launch_discovery_busy(false);
        match availability {
            Ok(availability) => {
                self.as_mut()
                    .set_launch_status(qstring(&availability.detail));
                if let Some(emulator) = availability.emulator {
                    self.as_mut().set_can_launch(true);
                    self.as_mut().set_emulator_name(qstring(&emulator.name));
                    self.as_mut()
                        .set_emulator_summary(qstring(emulator.executable.summary()));
                } else {
                    self.as_mut().set_can_launch(false);
                    self.as_mut().set_emulator_name(qstring(format!(
                        "Install {}",
                        availability.requirement
                    )));
                    self.as_mut().set_emulator_summary(QString::default());
                }
            }
            Err(error) => {
                self.as_mut().set_can_launch(false);
                self.as_mut()
                    .set_launch_status(qstring(format!("Could not inspect emulators: {error}")));
            }
        }
    }

    pub fn launch_game(mut self: Pin<&mut Self>) {
        if *self.as_ref().launch_busy() || *self.as_ref().game_running() {
            return;
        }
        let Some(prepared) = self.as_ref().rust().prepared_install.clone() else {
            self.as_mut()
                .set_launch_status(qstring("Prepare this PC install before launching it."));
            return;
        };
        let Some(catalog_database) = crate::catalog::requested_database_path() else {
            self.as_mut().set_launch_status(qstring(
                "The canonical Lunchbox database is unavailable, so the emulator cannot be selected.",
            ));
            return;
        };

        self.as_mut().rust_mut().launch_generation =
            self.as_ref().rust().launch_generation.wrapping_add(1);
        let generation = self.as_ref().rust().launch_generation;
        let game_id = self.as_ref().game_id().to_string();
        self.as_mut().set_launch_busy(true);
        self.as_mut().set_launch_status(qstring(
            "Building the exact launch plan and preparing writable runtime files…",
        ));

        let qt_thread = self.as_ref().qt_thread();
        let started_thread = qt_thread.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-emulator-launch".into())
            .spawn(move || {
                let launch = (|| -> anyhow::Result<Result<(), String>> {
                    let plan = crate::emulator::build_launch_plan(&prepared, &catalog_database)?;
                    let emulator_name = plan.emulator_name.clone();
                    let command_summary = plan.command_summary();
                    let cleanup_paths = plan.cleanup_paths.clone();
                    let mut child = match crate::emulator::spawn_launch_plan(&plan) {
                        Ok(child) => child,
                        Err(error) => {
                            crate::emulator::cleanup_after_launch(&cleanup_paths);
                            return Err(error);
                        }
                    };
                    let process_id = child.id();
                    let started_game_id = game_id.clone();
                    let _ = started_thread.queue(move |mut model| {
                        model.as_mut().finish_launch_started(
                            generation,
                            started_game_id,
                            emulator_name,
                            process_id,
                            command_summary,
                        );
                    });
                    let status = child.wait();
                    crate::emulator::cleanup_after_launch(&cleanup_paths);
                    let status = status.context("waiting for the emulator process")?;
                    Ok(if status.success() {
                        Ok(())
                    } else {
                        Err(format!("emulator exited with {status}"))
                    })
                })();
                match launch {
                    Ok(exit) => {
                        let _ = qt_thread.queue(move |mut model| {
                            model.as_mut().finish_launch_exit(generation, game_id, exit);
                        });
                    }
                    Err(error) => {
                        let error = error.to_string();
                        let _ = qt_thread.queue(move |mut model| {
                            model
                                .as_mut()
                                .finish_launch_failure(generation, game_id, error);
                        });
                    }
                }
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_launch_busy(false);
            self.as_mut().set_launch_status(qstring(format!(
                "Could not start emulator launch worker: {error}"
            )));
        }
    }

    fn finish_launch_started(
        mut self: Pin<&mut Self>,
        generation: u64,
        completed_game_id: String,
        emulator_name: String,
        process_id: u32,
        command_summary: String,
    ) {
        if generation != self.as_ref().rust().launch_generation
            || self.as_ref().game_id().to_string() != completed_game_id
        {
            return;
        }
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_game_running(true);
        self.as_mut().set_emulator_name(qstring(&emulator_name));
        self.as_mut().set_launch_status(qstring(format!(
            "Running with {emulator_name} · process {process_id}"
        )));
        self.as_mut().set_message(qstring(format!(
            "Launched with {emulator_name}. {command_summary}"
        )));
        if has_cli_flag("--exo-launch-probe") {
            println!(
                "LUNCHBOX_EMULATOR_STARTED name={emulator_name:?} pid={process_id} command={command_summary:?}"
            );
        }
    }

    fn finish_launch_exit(
        mut self: Pin<&mut Self>,
        generation: u64,
        completed_game_id: String,
        exit: Result<(), String>,
    ) {
        if generation != self.as_ref().rust().launch_generation
            || self.as_ref().game_id().to_string() != completed_game_id
        {
            return;
        }
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_game_running(false);
        match exit {
            Ok(()) => {
                self.as_mut()
                    .set_launch_status(qstring("The emulator session finished normally."));
                if has_cli_flag("--exo-launch-probe") {
                    println!("LUNCHBOX_EMULATOR_EXITED status=success");
                }
            }
            Err(error) => {
                self.as_mut()
                    .set_launch_status(qstring(format!("The emulator session ended: {error}")));
                if has_cli_flag("--exo-launch-probe") {
                    println!("LUNCHBOX_EMULATOR_EXITED status={error:?}");
                }
            }
        }
    }

    fn finish_launch_failure(
        mut self: Pin<&mut Self>,
        generation: u64,
        completed_game_id: String,
        error: String,
    ) {
        if generation != self.as_ref().rust().launch_generation
            || self.as_ref().game_id().to_string() != completed_game_id
        {
            return;
        }
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_game_running(false);
        self.as_mut().set_launch_status(qstring(format!(
            "Could not launch this prepared install: {error}"
        )));
        if has_cli_flag("--exo-launch-probe") {
            eprintln!("LUNCHBOX_EMULATOR_FAILED error={error:?}");
        }
    }

    fn update_preparation_progress(
        mut self: Pin<&mut Self>,
        generation: u64,
        progress: crate::exo_install::PreparationProgress,
    ) {
        if generation != self.as_ref().rust().preparation_generation
            || !*self.as_ref().prepare_busy()
        {
            return;
        }
        self.as_mut()
            .set_preparation_phase(qstring(&progress.phase));
        self.as_mut()
            .set_preparation_file(qstring(&progress.current_file));
        let detail = if progress.current_file.is_empty() {
            progress.phase
        } else if progress.completed_bytes == 0 {
            format!("{} · {}", progress.phase, progress.current_file)
        } else {
            format!(
                "{} · {} · {}",
                progress.phase,
                progress.current_file,
                game_details::format_bytes(progress.completed_bytes)
            )
        };
        self.as_mut().set_message(qstring(detail));
    }

    fn finish_preparation(
        mut self: Pin<&mut Self>,
        generation: u64,
        completed_game_id: String,
        prepared: Result<crate::exo_install::PreparedInstall, String>,
    ) {
        if generation != self.as_ref().rust().preparation_generation
            || self.as_ref().game_id().to_string() != completed_game_id
        {
            return;
        }
        self.as_mut().set_prepare_busy(false);
        self.as_mut().rust_mut().preparation_cancel = None;
        match prepared {
            Ok(prepared) => {
                let reused = prepared.reused;
                let summary = format!(
                    "{} prepared · {}",
                    prepared.collection.display_name(),
                    prepared.launch_config_path.display()
                );
                self.as_mut().set_prepared(true);
                self.as_mut().rust_mut().prepared_install = Some(prepared.clone());
                self.as_mut().set_preparation_phase(qstring("Ready"));
                self.as_mut()
                    .set_preparation_file(qstring(prepared.launch_config_path.to_string_lossy()));
                self.as_mut().set_prepared_summary(qstring(&summary));
                self.as_mut().set_message(qstring(if reused {
                    format!("Reused the verified prepared install. {summary}")
                } else {
                    format!("Prepared install completed atomically. {summary}")
                }));
                self.as_mut().refresh_emulators();
            }
            Err(error) => {
                self.as_mut().set_preparation_phase(qstring("Not prepared"));
                self.as_mut().set_message(qstring(
                    if error.to_ascii_lowercase().contains("cancelled") {
                        "eXo preparation was cancelled; no partial install was published."
                            .to_owned()
                    } else {
                        format!("Could not prepare eXo install: {error}")
                    },
                ));
            }
        }
    }

    fn invalidate_preparation(mut self: Pin<&mut Self>) {
        if let Some(cancel) = self.as_ref().rust().preparation_cancel.as_ref() {
            cancel.store(true, AtomicOrdering::Relaxed);
        }
        self.as_mut().rust_mut().preparation_cancel = None;
        self.as_mut().rust_mut().preparation_generation =
            self.as_ref().rust().preparation_generation.wrapping_add(1);
        self.as_mut().set_prepare_busy(false);
    }

    fn invalidate_launch_state(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().launch_generation =
            self.as_ref().rust().launch_generation.wrapping_add(1);
        self.as_mut().set_launch_discovery_busy(false);
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_can_launch(false);
        self.as_mut().set_game_running(false);
    }

    fn finish_queue(
        mut self: Pin<&mut Self>,
        completed_game_id: String,
        queued: Result<String, String>,
    ) {
        self.as_mut().set_download_busy(false);
        if self.as_ref().game_id().to_string() != completed_game_id {
            return;
        }
        match queued {
            Ok(title) => self.as_mut().set_message(qstring(format!(
                "{title} was added to Downloads. Progress is persisted across restarts."
            ))),
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not queue download: {error}"))),
        }
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().detail_revision().wrapping_add(1);
        self.as_mut().set_detail_revision(revision);
    }

    pub fn bundle_title_at(&self, index: i32) -> QString {
        self.bundle(index)
            .map(|bundle| {
                let source = if bundle.collection.trim().is_empty() {
                    "Minerva"
                } else {
                    &bundle.collection
                };
                qstring(format!("{source} · {}", bundle.provider_platform))
            })
            .unwrap_or_default()
    }

    pub fn bundle_detail_at(&self, index: i32) -> QString {
        self.bundle(index)
            .map(|bundle| {
                qstring(format!(
                    "{} files · {} · {}",
                    bundle.rom_count,
                    game_details::format_bytes(bundle.total_size),
                    bundle.match_kind.label()
                ))
            })
            .unwrap_or_default()
    }

    pub fn file_name_at(&self, index: i32) -> QString {
        self.file(index)
            .map(|file| {
                if let Some(plan) = &file.download_plan {
                    if plan.is_optical_multidisc() {
                        qstring(format!(
                            "{} — {}-disc set",
                            plan.display_name,
                            plan.disc_count()
                        ))
                    } else {
                        qstring(format!("{} — eXo archive set", plan.display_name))
                    }
                } else {
                    qstring(&file.filename)
                }
            })
            .unwrap_or_default()
    }

    pub fn file_detail_at(&self, index: i32) -> QString {
        self.file(index)
            .map(|file| {
                let mut details = Vec::new();
                if index == 0 {
                    details.push("BEST MATCH".to_owned());
                }
                if let Some(plan) = &file.download_plan {
                    details.push(if plan.is_optical_multidisc() {
                        "MULTI-DISC".to_owned()
                    } else {
                        "EXO ARCHIVE SET".to_owned()
                    });
                    details.push(format!("{} required files", plan.members.len()));
                }
                if !file.region.is_empty() {
                    details.push(file.region.clone());
                }
                if !file.version.is_empty() {
                    details.push(file.version.clone());
                }
                details.push(format!("{:.0}% title match", file.match_score * 100.0));
                details.push(game_details::format_bytes(file.byte_size));
                details.push(format!("torrent file #{}", file.index));
                qstring(details.join(" · "))
            })
            .unwrap_or_default()
    }

    pub fn file_has_download_plan(&self, index: i32) -> bool {
        self.file(index)
            .is_some_and(|file| file.download_plan.is_some())
    }

    pub fn file_plan_summary_at(&self, index: i32) -> QString {
        self.file(index)
            .and_then(|file| file.download_plan.as_ref())
            .map(|plan| {
                if plan.is_optical_multidisc() {
                    let companion_count = plan.members.len().saturating_sub(plan.disc_count());
                    qstring(format!(
                        "{} discs · {} companion file{} · {} total · creates {}",
                        plan.disc_count(),
                        companion_count,
                        if companion_count == 1 { "" } else { "s" },
                        game_details::format_bytes(plan.total_bytes()),
                        plan.playlist_filename
                    ))
                } else {
                    qstring(format!(
                        "{} exact archives · {} total · shared dependencies retained for installation",
                        plan.members.len(),
                        game_details::format_bytes(plan.total_bytes())
                    ))
                }
            })
            .unwrap_or_default()
    }

    pub fn file_plan_kind_at(&self, index: i32) -> QString {
        self.file(index)
            .and_then(|file| file.download_plan.as_ref())
            .map(|plan| qstring(&plan.kind))
            .unwrap_or_default()
    }

    pub fn file_plan_members_at(&self, index: i32) -> QString {
        self.file(index)
            .and_then(|file| file.download_plan.as_ref())
            .map(|plan| {
                qstring(
                    plan.members
                        .iter()
                        .map(|member| {
                            let role = if plan.is_exo_archive_set() {
                                match member.role.as_str() {
                                    "primary" => "Game archive".to_owned(),
                                    "game-data" => "Game data".to_owned(),
                                    "metadata" => "Launch metadata".to_owned(),
                                    "utilities" => "Shared utilities".to_owned(),
                                    _ => "Required archive".to_owned(),
                                }
                            } else if member.playlist_entry {
                                format!("Disc {}", member.disc_index.unwrap_or_default())
                            } else {
                                format!("Disc {} companion", member.disc_index.unwrap_or_default())
                            };
                            format!(
                                "{role}  ·  {}  ·  {}",
                                member.torrent_path,
                                game_details::format_bytes(member.byte_size)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            })
            .unwrap_or_default()
    }

    fn bundle(&self, index: i32) -> Option<&MinervaBundle> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().bundles.get(index))
    }

    fn file(&self, index: i32) -> Option<&TorrentFileCandidate> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().files.get(index))
    }
}

fn queue_download(
    game_id: String,
    launchbox_db_id: i64,
    title: String,
    platform: String,
    bundle: MinervaBundle,
    file: TorrentFileCandidate,
) -> anyhow::Result<String> {
    let store = crate::settings::SettingsStore::open_default()?;
    let settings = store.load()?;
    let password = crate::settings::load_password()?.unwrap_or_default();
    let torrent_bytes = game_details::torrent_bytes(&bundle)?;
    let file_index = u32::try_from(file.index)
        .map_err(|_| anyhow::anyhow!("torrent file index is too large"))?;
    let job = crate::qbittorrent::enqueue(
        &settings,
        &password,
        &store,
        crate::qbittorrent::EnqueueRequest {
            game_id,
            launchbox_db_id,
            title: title.clone(),
            platform,
            torrent_url: bundle.torrent_url,
            torrent_bytes,
            selected_file_index: file_index,
            selected_file_path: file.filename,
            download_plan: file.download_plan,
        },
    )?;
    store.upsert_job(&job)?;
    Ok(title)
}
