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
        #[qproperty(bool, activity_busy)]
        #[qproperty(bool, activity_visible)]
        #[qproperty(i32, play_count)]
        #[qproperty(QString, play_time)]
        #[qproperty(QString, last_played)]
        #[qproperty(QString, completion_state)]
        #[qproperty(i32, activity_revision)]
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
        #[qproperty(QString, emulator_preference_scope)]
        #[qproperty(bool, firmware_busy)]
        #[qproperty(bool, firmware_needs_import)]
        #[qproperty(bool, firmware_can_download)]
        #[qproperty(bool, firmware_can_sync)]
        #[qproperty(i32, firmware_rule_count)]
        #[qproperty(i32, firmware_missing_count)]
        #[qproperty(i32, firmware_manual_count)]
        #[qproperty(i32, firmware_optional_count)]
        #[qproperty(QString, firmware_summary)]
        #[qproperty(QString, firmware_source_summary)]
        #[qproperty(QString, firmware_package_summary)]
        #[qproperty(QString, firmware_runtime_path)]
        #[qproperty(i32, bundle_count)]
        #[qproperty(i32, file_count)]
        #[qproperty(i32, selected_bundle)]
        #[qproperty(i32, local_file_count)]
        #[qproperty(i32, selected_local_file)]
        #[qproperty(i32, emulator_option_count)]
        #[qproperty(i32, selected_emulator_option)]
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
        fn select_local_file(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn select_emulator_option(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn save_game_emulator_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn save_platform_emulator_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn clear_game_emulator_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn clear_platform_emulator_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn open_firmware_directory(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn import_firmware_package(self: Pin<&mut GameDetailsModel>, url: QUrl);

        #[qinvokable]
        fn download_firmware(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn sync_firmware(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn save_completion_state(self: Pin<&mut GameDetailsModel>, state: QString);

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

        #[qinvokable]
        fn local_file_label_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn emulator_option_label_at(self: &GameDetailsModel, index: i32) -> QString;
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
use cxx_qt_lib::{QString, QUrl};

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
    activity_busy: bool,
    activity_visible: bool,
    play_count: i32,
    play_time: QString,
    last_played: QString,
    completion_state: QString,
    activity_revision: i32,
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
    emulator_preference_scope: QString,
    firmware_busy: bool,
    firmware_needs_import: bool,
    firmware_can_download: bool,
    firmware_can_sync: bool,
    firmware_rule_count: i32,
    firmware_missing_count: i32,
    firmware_manual_count: i32,
    firmware_optional_count: i32,
    firmware_summary: QString,
    firmware_source_summary: QString,
    firmware_package_summary: QString,
    firmware_runtime_path: QString,
    bundle_count: i32,
    file_count: i32,
    selected_bundle: i32,
    local_file_count: i32,
    selected_local_file: i32,
    emulator_option_count: i32,
    selected_emulator_option: i32,
    detail_revision: i32,
    bundles: Vec<MinervaBundle>,
    files: Vec<TorrentFileCandidate>,
    details_generation: u64,
    torrent_generation: u64,
    preparation_generation: u64,
    launch_generation: u64,
    activity_load_generation: u64,
    preparation_cancel: Option<Arc<AtomicBool>>,
    database_id: i64,
    local_file_path: PathBuf,
    local_file_paths: Vec<PathBuf>,
    rom_emulator_options: Vec<crate::emulator::RomEmulatorOption>,
    rom_firmware_statuses: Vec<Vec<crate::firmware::FirmwareStatus>>,
    pending_firmware_message: Option<String>,
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
            activity_busy: false,
            activity_visible: false,
            play_count: 0,
            play_time: QString::from("Never played"),
            last_played: QString::from("Never"),
            completion_state: QString::from("not_started"),
            activity_revision: 0,
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
            emulator_preference_scope: QString::default(),
            firmware_busy: false,
            firmware_needs_import: false,
            firmware_can_download: false,
            firmware_can_sync: false,
            firmware_rule_count: 0,
            firmware_missing_count: 0,
            firmware_manual_count: 0,
            firmware_optional_count: 0,
            firmware_summary: QString::default(),
            firmware_source_summary: QString::default(),
            firmware_package_summary: QString::default(),
            firmware_runtime_path: QString::default(),
            bundle_count: 0,
            file_count: 0,
            selected_bundle: -1,
            local_file_count: 0,
            selected_local_file: -1,
            emulator_option_count: 0,
            selected_emulator_option: -1,
            detail_revision: 0,
            bundles: Vec::new(),
            files: Vec::new(),
            details_generation: 0,
            torrent_generation: 0,
            preparation_generation: 0,
            launch_generation: 0,
            activity_load_generation: 0,
            preparation_cancel: None,
            database_id: 0,
            local_file_path: PathBuf::new(),
            local_file_paths: Vec::new(),
            rom_emulator_options: Vec::new(),
            rom_firmware_statuses: Vec::new(),
            pending_firmware_message: None,
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

fn format_play_time(seconds: i64, play_count: i64) -> String {
    let seconds = u64::try_from(seconds).unwrap_or_default();
    if seconds == 0 {
        return if play_count > 0 {
            "Under 1 min".to_owned()
        } else {
            "Never played".to_owned()
        };
    }
    if seconds < 60 {
        return "Under 1 min".to_owned();
    }
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if hours == 0 {
        format!("{minutes} min")
    } else if minutes == 0 {
        format!("{hours} hr")
    } else {
        format!("{hours} hr {minutes} min")
    }
}

fn format_last_played(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "Never".to_owned();
    }
    let elapsed = crate::settings::unix_timestamp().saturating_sub(timestamp);
    match elapsed {
        0..=59 => "Just now".to_owned(),
        60..=3_599 => format!("{} min ago", elapsed / 60),
        3_600..=86_399 => format!("{} hr ago", elapsed / 3_600),
        86_400..=172_799 => "Yesterday".to_owned(),
        _ => format!("{} days ago", elapsed / 86_400),
    }
}

fn has_cli_flag(flag: &str) -> bool {
    std::env::args_os().any(|argument| argument == flag)
}

fn is_local_launch_probe() -> bool {
    has_cli_flag("--rom-launch-probe") || has_cli_flag("--arcade-launch-probe")
}

fn is_emulator_launch_probe() -> bool {
    has_cli_flag("--exo-launch-probe") || is_local_launch_probe()
}

enum EmulatorDiscoveryResult {
    Prepared(crate::emulator::LaunchAvailability),
    Rom {
        availability: crate::emulator::RomLaunchAvailability,
        firmware_statuses: Vec<Vec<crate::firmware::FirmwareStatus>>,
    },
}

enum LaunchInput {
    Prepared {
        install: crate::exo_install::PreparedInstall,
        catalog_database: PathBuf,
    },
    Rom {
        path: PathBuf,
        platform: String,
        option: crate::emulator::RomEmulatorOption,
    },
}

struct LaunchStarted {
    game_id: String,
    emulator_name: String,
    process_id: u32,
    command_summary: String,
    tracking_warning: Option<String>,
    activity_recorded: bool,
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
        self.as_mut().set_firmware_busy(false);
        self.as_mut().rust_mut().pending_firmware_message = None;
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
        self.as_mut().set_activity_busy(false);
        self.as_mut().set_activity_visible(false);
        self.as_mut().set_play_count(0);
        self.as_mut().set_play_time(qstring("Never played"));
        self.as_mut().set_last_played(qstring("Never"));
        self.as_mut().set_completion_state(qstring("not_started"));
        self.as_mut().set_preparable(false);
        self.as_mut().set_prepared(false);
        self.as_mut().set_preparation_phase(QString::default());
        self.as_mut().set_preparation_file(QString::default());
        self.as_mut().set_prepared_summary(QString::default());
        self.as_mut().set_emulator_name(QString::default());
        self.as_mut().set_emulator_summary(QString::default());
        self.as_mut().set_launch_status(QString::default());
        self.as_mut()
            .set_emulator_preference_scope(QString::default());
        self.as_mut().set_firmware_busy(false);
        self.as_mut().rust_mut().pending_firmware_message = None;
        self.as_mut().clear_firmware_status();
        self.as_mut().set_can_launch(false);
        self.as_mut().set_game_running(false);
        self.as_mut().rust_mut().database_id = 0;
        self.as_mut().rust_mut().local_file_path = PathBuf::new();
        self.as_mut().rust_mut().local_file_paths.clear();
        self.as_mut().rust_mut().rom_emulator_options.clear();
        self.as_mut().rust_mut().rom_firmware_statuses.clear();
        self.as_mut().rust_mut().prepared_install = None;
        self.as_mut().rust_mut().bundles.clear();
        self.as_mut().rust_mut().files.clear();
        self.as_mut().set_bundle_count(0);
        self.as_mut().set_file_count(0);
        self.as_mut().set_selected_bundle(-1);
        self.as_mut().set_local_file_count(0);
        self.as_mut().set_selected_local_file(-1);
        self.as_mut().set_emulator_option_count(0);
        self.as_mut().set_selected_emulator_option(-1);
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
                let activity = details.activity.clone();
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
                let local_file_count = details.local_file_paths.len();
                let local_file_paths = details.local_file_paths.clone();
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
                self.as_mut().apply_play_activity(activity, details.local);
                self.as_mut().rust_mut().local_file_path = details.local_file_path;
                self.as_mut().rust_mut().local_file_paths = local_file_paths;
                self.as_mut()
                    .set_local_file_count(count_i32(local_file_count));
                self.as_mut()
                    .set_selected_local_file(if local_file_count == 0 { -1 } else { 0 });
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
                if prepared || (local_file_count > 0 && !preparable) {
                    self.as_mut().refresh_emulators();
                }
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not load game details: {error}"))),
        }
    }

    fn apply_play_activity(
        mut self: Pin<&mut Self>,
        activity: Option<crate::settings::PlayActivity>,
        local: bool,
    ) {
        self.as_mut()
            .set_activity_visible(local || activity.is_some());
        if let Some(activity) = activity {
            self.as_mut()
                .set_play_count(i32::try_from(activity.play_count.max(0)).unwrap_or(i32::MAX));
            self.as_mut().set_play_time(qstring(format_play_time(
                activity.total_play_time_seconds,
                activity.play_count,
            )));
            self.as_mut()
                .set_last_played(qstring(format_last_played(activity.last_played_at)));
            self.as_mut()
                .set_completion_state(qstring(activity.completion_state));
        } else {
            self.as_mut().set_play_count(0);
            self.as_mut().set_play_time(qstring("Never played"));
            self.as_mut().set_last_played(qstring("Never"));
            self.as_mut().set_completion_state(qstring("not_started"));
        }
    }

    pub fn save_completion_state(mut self: Pin<&mut Self>, state: QString) {
        if *self.as_ref().activity_busy() || !*self.as_ref().activity_visible() {
            return;
        }
        let state = state.to_string();
        if state == self.as_ref().completion_state().to_string() {
            return;
        }
        let previous = self.as_ref().completion_state().to_string();
        let game_id = self.as_ref().game_id().to_string();
        let title = self.as_ref().title().to_string();
        let platform = self.as_ref().platform().to_string();
        let database_id = self.as_ref().rust().database_id;
        let generation = self.as_ref().rust().details_generation;
        self.as_mut().set_activity_busy(true);
        self.as_mut().set_completion_state(qstring(&state));
        let completed_game_id = game_id.clone();
        let rollback = previous.clone();
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-completion-save".into())
            .spawn(move || {
                let result = crate::settings::SettingsStore::open_default()
                    .and_then(|store| {
                        store.set_completion_state(&game_id, database_id, &title, &platform, &state)
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_completion_save(
                        generation,
                        completed_game_id,
                        previous,
                        result,
                    );
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_activity_busy(false);
            self.as_mut().set_completion_state(qstring(rollback));
            self.as_mut().set_launch_status(qstring(format!(
                "Could not start completion-state save: {error}"
            )));
        }
    }

    fn finish_completion_save(
        mut self: Pin<&mut Self>,
        generation: u64,
        completed_game_id: String,
        previous: String,
        result: Result<(), String>,
    ) {
        if generation != self.as_ref().rust().details_generation
            || completed_game_id != self.as_ref().game_id().to_string()
        {
            return;
        }
        self.as_mut().set_activity_busy(false);
        match result {
            Ok(()) => {
                let revision = self.as_ref().activity_revision().wrapping_add(1);
                self.as_mut().set_activity_revision(revision);
                self.as_mut()
                    .set_launch_status(qstring("Saved this game's completion state."));
            }
            Err(error) => {
                self.as_mut().set_completion_state(qstring(previous));
                self.as_mut().set_launch_status(qstring(format!(
                    "Could not save completion state: {error}"
                )));
            }
        }
    }

    fn reload_play_activity(mut self: Pin<&mut Self>, notify_library: bool) {
        self.as_mut().rust_mut().activity_load_generation = self
            .as_ref()
            .rust()
            .activity_load_generation
            .wrapping_add(1);
        let generation = self.as_ref().rust().activity_load_generation;
        let details_generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        let completed_game_id = game_id.clone();
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-play-activity-load".into())
            .spawn(move || {
                let result = crate::settings::SettingsStore::open_default()
                    .and_then(|store| store.play_activity(&game_id))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_play_activity_reload(
                        generation,
                        details_generation,
                        completed_game_id,
                        notify_library,
                        result,
                    );
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_launch_status(qstring(format!(
                "Could not start play-activity refresh: {error}"
            )));
        }
    }

    fn finish_play_activity_reload(
        mut self: Pin<&mut Self>,
        generation: u64,
        details_generation: u64,
        completed_game_id: String,
        notify_library: bool,
        result: Result<Option<crate::settings::PlayActivity>, String>,
    ) {
        if generation != self.as_ref().rust().activity_load_generation
            || details_generation != self.as_ref().rust().details_generation
            || completed_game_id != self.as_ref().game_id().to_string()
        {
            return;
        }
        match result {
            Ok(activity) => {
                if is_emulator_launch_probe()
                    && let Some(activity) = activity.as_ref()
                {
                    let phase = if *self.as_ref().game_running() {
                        "started"
                    } else {
                        "finalized"
                    };
                    println!(
                        "LUNCHBOX_ACTIVITY_READY phase={phase} plays={} total_seconds={} completion={:?}",
                        activity.play_count,
                        activity.total_play_time_seconds,
                        activity.completion_state,
                    );
                }
                let local = *self.as_ref().local();
                self.as_mut().apply_play_activity(activity, local);
                if notify_library {
                    let revision = self.as_ref().activity_revision().wrapping_add(1);
                    self.as_mut().set_activity_revision(revision);
                }
            }
            Err(error) => self
                .as_mut()
                .set_launch_status(qstring(format!("Could not refresh play activity: {error}"))),
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
        let queue_message = match file.download_plan.as_ref() {
            Some(plan) if plan.is_optical_multidisc() => {
                "Adding the reviewed multi-disc set to qBittorrent…"
            }
            Some(plan) if plan.is_arcade_machine_layout() => {
                "Adding the reviewed arcade machine layout to qBittorrent…"
            }
            Some(_) => "Adding the reviewed eXo archive set to qBittorrent…",
            None => "Adding the reviewed file to qBittorrent…",
        };
        self.as_mut().set_message(qstring(queue_message));

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
        let prepared = self.as_ref().rust().prepared_install.clone();
        let selected_local_file = usize::try_from(*self.as_ref().selected_local_file())
            .ok()
            .and_then(|index| self.as_ref().rust().local_file_paths.get(index).cloned());
        if prepared.is_none() && selected_local_file.is_none() {
            self.as_mut().set_can_launch(false);
            self.as_mut().set_emulator_name(QString::default());
            self.as_mut().set_emulator_summary(QString::default());
            self.as_mut().set_launch_status(qstring(
                "No present local game file is available for emulator detection.",
            ));
            return;
        }
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
        let platform = self.as_ref().platform().to_string();
        self.as_mut().set_launch_discovery_busy(true);
        self.as_mut().set_can_launch(false);
        self.as_mut().set_emulator_name(QString::default());
        self.as_mut().set_emulator_summary(QString::default());
        self.as_mut().rust_mut().rom_emulator_options.clear();
        self.as_mut().rust_mut().rom_firmware_statuses.clear();
        self.as_mut().clear_firmware_status();
        self.as_mut().set_emulator_option_count(0);
        self.as_mut().set_selected_emulator_option(-1);
        self.as_mut()
            .set_emulator_preference_scope(QString::default());
        self.as_mut()
            .set_launch_status(qstring("Detecting compatible emulators…"));

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-emulator-discovery".into())
            .spawn(move || {
                let availability = (|| -> anyhow::Result<EmulatorDiscoveryResult> {
                    if let Some(prepared) = prepared {
                        return crate::emulator::inspect_launch_availability(
                            &prepared,
                            &catalog_database,
                        )
                        .map(EmulatorDiscoveryResult::Prepared);
                    }
                    let preference = crate::settings::SettingsStore::open_default()?
                        .emulator_preference(&game_id, &platform)?;
                    let rom_path = selected_local_file
                        .as_deref()
                        .context("selected local game file disappeared")?;
                    let availability = crate::emulator::inspect_rom_launch_availability(
                        &platform,
                        rom_path,
                        &catalog_database,
                        preference.as_ref(),
                    )?;
                    let firmware_statuses = crate::firmware::statuses_for_options(
                        &catalog_database,
                        &platform,
                        rom_path,
                        &availability.options,
                    )?;
                    Ok(EmulatorDiscoveryResult::Rom {
                        availability,
                        firmware_statuses,
                    })
                })()
                .map_err(|error| error.to_string());
                let completed_game_id = game_id.clone();
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_emulator_discovery(
                        generation,
                        completed_game_id,
                        availability,
                    );
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
        availability: Result<EmulatorDiscoveryResult, String>,
    ) {
        if generation != self.as_ref().rust().launch_generation
            || self.as_ref().game_id().to_string() != completed_game_id
        {
            return;
        }
        self.as_mut().set_launch_discovery_busy(false);
        match availability {
            Ok(EmulatorDiscoveryResult::Prepared(availability)) => {
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
            Ok(EmulatorDiscoveryResult::Rom {
                availability,
                firmware_statuses,
            }) => {
                let selected_index = availability.selected_index;
                let selected =
                    selected_index.and_then(|index| availability.options.get(index).cloned());
                let option_count = availability.options.len();
                self.as_mut().rust_mut().rom_emulator_options = availability.options;
                self.as_mut().rust_mut().rom_firmware_statuses = firmware_statuses;
                self.as_mut()
                    .set_emulator_option_count(count_i32(option_count));
                self.as_mut().set_selected_emulator_option(
                    selected_index
                        .and_then(|index| i32::try_from(index).ok())
                        .unwrap_or(-1),
                );
                self.as_mut()
                    .set_emulator_preference_scope(qstring(&availability.preference_scope));
                self.as_mut()
                    .set_launch_status(qstring(&availability.detail));
                if let Some(option) = selected {
                    self.as_mut().set_can_launch(true);
                    self.as_mut().set_emulator_name(qstring(option.label()));
                    self.as_mut()
                        .set_emulator_summary(qstring(option.summary()));
                    self.as_mut().update_selected_firmware(selected_index);
                } else {
                    self.as_mut().set_can_launch(false);
                    self.as_mut().set_emulator_name(qstring(
                        if availability.requirement.is_empty() {
                            "No compatible emulator".to_owned()
                        } else {
                            format!("Install {}", availability.requirement)
                        },
                    ));
                    self.as_mut().set_emulator_summary(QString::default());
                    self.as_mut().clear_firmware_status();
                }
            }
            Err(error) => {
                self.as_mut().set_can_launch(false);
                self.as_mut()
                    .set_launch_status(qstring(format!("Could not inspect emulators: {error}")));
            }
        }
        if let Some(message) = self.as_mut().rust_mut().pending_firmware_message.take() {
            self.as_mut().set_launch_status(qstring(message));
        }
    }

    pub fn select_local_file(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().launch_busy() || *self.as_ref().game_running() {
            return;
        }
        let Some(index) = usize::try_from(index).ok() else {
            return;
        };
        let Some(path) = self.as_ref().rust().local_file_paths.get(index).cloned() else {
            return;
        };
        self.as_mut()
            .set_selected_local_file(i32::try_from(index).unwrap_or(i32::MAX));
        self.as_mut().rust_mut().local_file_path = path;
        self.as_mut().invalidate_launch_state();
        self.as_mut().refresh_emulators();
    }

    pub fn select_emulator_option(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().launch_busy() || *self.as_ref().game_running() {
            return;
        }
        let Some(index) = usize::try_from(index).ok() else {
            return;
        };
        let Some(option) = self
            .as_ref()
            .rust()
            .rom_emulator_options
            .get(index)
            .cloned()
        else {
            return;
        };
        self.as_mut()
            .set_selected_emulator_option(i32::try_from(index).unwrap_or(i32::MAX));
        self.as_mut().set_can_launch(true);
        self.as_mut().set_emulator_name(qstring(option.label()));
        self.as_mut()
            .set_emulator_summary(qstring(option.summary()));
        self.as_mut().update_selected_firmware(Some(index));
        self.as_mut()
            .set_emulator_preference_scope(QString::default());
        self.as_mut().set_launch_status(qstring(
            "Emulator selected for this launch. Save it as a game or platform default if desired.",
        ));
    }

    pub fn save_game_emulator_preference(mut self: Pin<&mut Self>) {
        let Some(option) = self.selected_rom_emulator_option() else {
            return;
        };
        let game_id = self.as_ref().game_id().to_string();
        let result = crate::settings::SettingsStore::open_default().and_then(|store| {
            store.set_game_emulator_preference(
                &game_id,
                &option.emulator_id,
                option.runtime_kind.key(),
                &option.core_name,
            )
        });
        match result {
            Ok(()) => {
                self.as_mut().set_emulator_preference_scope(qstring("game"));
                self.as_mut().set_launch_status(qstring(format!(
                    "{} is now the default for this game.",
                    option.label()
                )));
            }
            Err(error) => self.as_mut().set_launch_status(qstring(format!(
                "Could not save the game emulator default: {error}"
            ))),
        }
    }

    pub fn save_platform_emulator_preference(mut self: Pin<&mut Self>) {
        let Some(option) = self.selected_rom_emulator_option() else {
            return;
        };
        let platform = self.as_ref().platform().to_string();
        let result = crate::settings::SettingsStore::open_default().and_then(|store| {
            store.set_platform_emulator_preference(
                &platform,
                &option.emulator_id,
                option.runtime_kind.key(),
                &option.core_name,
            )
        });
        match result {
            Ok(()) => {
                self.as_mut()
                    .set_emulator_preference_scope(qstring("platform"));
                self.as_mut().set_launch_status(qstring(format!(
                    "{} is now the default for {platform}.",
                    option.label()
                )));
            }
            Err(error) => self.as_mut().set_launch_status(qstring(format!(
                "Could not save the platform emulator default: {error}"
            ))),
        }
    }

    pub fn clear_game_emulator_preference(mut self: Pin<&mut Self>) {
        let game_id = self.as_ref().game_id().to_string();
        match crate::settings::SettingsStore::open_default()
            .and_then(|store| store.clear_game_emulator_preference(&game_id))
        {
            Ok(()) => self.as_mut().refresh_emulators(),
            Err(error) => self.as_mut().set_launch_status(qstring(format!(
                "Could not clear the game emulator default: {error}"
            ))),
        }
    }

    pub fn clear_platform_emulator_preference(mut self: Pin<&mut Self>) {
        let platform = self.as_ref().platform().to_string();
        match crate::settings::SettingsStore::open_default()
            .and_then(|store| store.clear_platform_emulator_preference(&platform))
        {
            Ok(()) => self.as_mut().refresh_emulators(),
            Err(error) => self.as_mut().set_launch_status(qstring(format!(
                "Could not clear the platform emulator default: {error}"
            ))),
        }
    }

    pub fn open_firmware_directory(mut self: Pin<&mut Self>) {
        let statuses = self.as_ref().selected_firmware_statuses();
        match crate::firmware::open_firmware_directory(&statuses) {
            Ok(path) => self.as_mut().set_launch_status(qstring(format!(
                "Opened firmware directory {}.",
                path.display()
            ))),
            Err(error) => self.as_mut().set_launch_status(qstring(format!(
                "Could not open the firmware directory: {error}"
            ))),
        }
    }

    pub fn import_firmware_package(mut self: Pin<&mut Self>, url: QUrl) {
        if *self.as_ref().firmware_busy() {
            return;
        }
        let Some(path) = url
            .to_local_file()
            .map(|path| PathBuf::from(path.to_string()))
        else {
            self.as_mut().set_launch_status(qstring(
                "Choose a firmware package from the local filesystem.",
            ));
            return;
        };
        let statuses = self.as_ref().selected_firmware_statuses();
        if !statuses
            .iter()
            .any(|status| status.target_strategy != "manual_import" && !status.imported)
        {
            self.as_mut().set_launch_status(qstring(
                "No unimported package is selected for this emulator.",
            ));
            return;
        }
        let generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        self.as_mut().set_firmware_busy(true);
        self.as_mut()
            .set_launch_status(qstring("Importing and verifying firmware package…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-firmware-import".into())
            .spawn(move || {
                let result = crate::firmware::import_and_sync(&statuses, &path)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_firmware_action(generation, game_id, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_firmware_busy(false);
            self.as_mut()
                .set_launch_status(qstring(format!("Could not start firmware import: {error}")));
        }
    }

    pub fn download_firmware(mut self: Pin<&mut Self>) {
        if *self.as_ref().firmware_busy() {
            return;
        }
        let statuses = self.as_ref().selected_firmware_statuses();
        if !statuses.iter().any(|status| {
            status.target_strategy != "manual_import"
                && !status.imported
                && status.source_transport != "manual"
        }) {
            self.as_mut().set_launch_status(qstring(
                "No downloadable firmware package is selected for this emulator.",
            ));
            return;
        }
        let generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        self.as_mut().set_firmware_busy(true);
        self.as_mut()
            .set_launch_status(qstring("Resolving the exact firmware source…"));
        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress_game_id = game_id.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-firmware-download".into())
            .spawn(move || {
                let result = crate::firmware::acquire_and_sync(&statuses, |message| {
                    let progress_game_id = progress_game_id.clone();
                    let _ = progress_thread.queue(move |mut model| {
                        model.as_mut().update_firmware_progress(
                            generation,
                            progress_game_id,
                            message,
                        );
                    });
                })
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_firmware_action(generation, game_id, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_firmware_busy(false);
            self.as_mut().set_launch_status(qstring(format!(
                "Could not start firmware download: {error}"
            )));
        }
    }

    pub fn sync_firmware(mut self: Pin<&mut Self>) {
        if *self.as_ref().firmware_busy() {
            return;
        }
        let statuses = self.as_ref().selected_firmware_statuses();
        if !statuses.iter().any(|status| {
            status.target_strategy != "manual_import"
                && status.imported
                && !status.target_path.is_empty()
        }) {
            self.as_mut().set_launch_status(qstring(
                "No imported package needs runtime sync for this emulator.",
            ));
            return;
        }
        let generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        self.as_mut().set_firmware_busy(true);
        self.as_mut()
            .set_launch_status(qstring("Verifying and syncing firmware…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-firmware-sync".into())
            .spawn(move || {
                let result =
                    crate::firmware::sync_imported(&statuses).map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_firmware_action(generation, game_id, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_firmware_busy(false);
            self.as_mut()
                .set_launch_status(qstring(format!("Could not start firmware sync: {error}")));
        }
    }

    fn finish_firmware_action(
        mut self: Pin<&mut Self>,
        generation: u64,
        game_id: String,
        result: Result<String, String>,
    ) {
        if generation != self.as_ref().rust().details_generation
            || game_id != self.as_ref().game_id().to_string()
        {
            return;
        }
        self.as_mut().set_firmware_busy(false);
        match result {
            Ok(message) => {
                self.as_mut().rust_mut().pending_firmware_message = Some(message);
                self.as_mut().refresh_emulators();
            }
            Err(error) => self.as_mut().set_launch_status(qstring(format!(
                "Could not import or sync firmware: {error}"
            ))),
        }
    }

    fn update_firmware_progress(
        mut self: Pin<&mut Self>,
        generation: u64,
        game_id: String,
        message: String,
    ) {
        if generation == self.as_ref().rust().details_generation
            && game_id == self.as_ref().game_id().to_string()
            && *self.as_ref().firmware_busy()
        {
            self.as_mut().set_launch_status(qstring(message));
        }
    }

    fn clear_firmware_status(mut self: Pin<&mut Self>) {
        self.as_mut().set_firmware_rule_count(0);
        self.as_mut().set_firmware_missing_count(0);
        self.as_mut().set_firmware_manual_count(0);
        self.as_mut().set_firmware_optional_count(0);
        self.as_mut().set_firmware_needs_import(false);
        self.as_mut().set_firmware_can_download(false);
        self.as_mut().set_firmware_can_sync(false);
        self.as_mut().set_firmware_summary(QString::default());
        self.as_mut()
            .set_firmware_source_summary(QString::default());
        self.as_mut()
            .set_firmware_package_summary(QString::default());
        self.as_mut().set_firmware_runtime_path(QString::default());
    }

    fn update_selected_firmware(mut self: Pin<&mut Self>, index: Option<usize>) {
        let statuses = index
            .and_then(|index| {
                self.as_ref()
                    .rust()
                    .rom_firmware_statuses
                    .get(index)
                    .cloned()
            })
            .unwrap_or_default();
        if statuses.is_empty() {
            self.as_mut().clear_firmware_status();
            return;
        }
        let missing = statuses
            .iter()
            .filter(|status| status.needs_action())
            .count();
        let manual = statuses
            .iter()
            .filter(|status| status.target_strategy == "manual_import" && !status.imported)
            .count();
        let optional = statuses
            .iter()
            .filter(|status| status.supports_hle_fallback && !status.imported)
            .count();
        let needs_import = statuses
            .iter()
            .any(|status| status.target_strategy != "manual_import" && !status.imported);
        let can_download = statuses.iter().any(|status| {
            status.target_strategy != "manual_import"
                && !status.imported
                && status.source_transport != "manual"
        });
        let can_sync = statuses.iter().any(|status| {
            status.target_strategy != "manual_import"
                && status.imported
                && !status.target_path.is_empty()
        });
        let mut sources = statuses
            .iter()
            .map(crate::firmware::FirmwareStatus::source_label)
            .collect::<Vec<_>>();
        sources.sort_unstable();
        sources.dedup();
        let mut packages = statuses
            .iter()
            .map(|status| status.package_name.as_str())
            .collect::<Vec<_>>();
        packages.sort_unstable();
        packages.dedup();
        let runtime_path = statuses
            .iter()
            .map(|status| status.runtime_path.as_str())
            .find(|path| !path.is_empty())
            .unwrap_or_default();
        self.as_mut()
            .set_firmware_rule_count(count_i32(statuses.len()));
        self.as_mut().set_firmware_missing_count(count_i32(missing));
        self.as_mut().set_firmware_manual_count(count_i32(manual));
        self.as_mut()
            .set_firmware_optional_count(count_i32(optional));
        if missing > 0 {
            self.as_mut().set_can_launch(false);
        }
        self.as_mut().set_firmware_needs_import(needs_import);
        self.as_mut().set_firmware_can_download(can_download);
        self.as_mut().set_firmware_can_sync(can_sync);
        self.as_mut()
            .set_firmware_summary(qstring(crate::firmware::summarize(&statuses)));
        self.as_mut()
            .set_firmware_source_summary(qstring(sources.join(" · ")));
        self.as_mut()
            .set_firmware_package_summary(qstring(packages.join(" · ")));
        self.as_mut()
            .set_firmware_runtime_path(qstring(runtime_path));
        if (has_cli_flag("--firmware-probe") || has_cli_flag("--firmware-ui-probe"))
            && !statuses.is_empty()
        {
            println!(
                "LUNCHBOX_FIRMWARE_READY rules={} missing={} manual={} optional={} launch_ready={} runtime={runtime_path:?}",
                statuses.len(),
                missing,
                manual,
                optional,
                *self.as_ref().can_launch()
            );
        }
    }

    fn selected_firmware_statuses(&self) -> Vec<crate::firmware::FirmwareStatus> {
        usize::try_from(*self.selected_emulator_option())
            .ok()
            .and_then(|index| self.rust().rom_firmware_statuses.get(index).cloned())
            .unwrap_or_default()
    }

    fn selected_rom_emulator_option(&self) -> Option<crate::emulator::RomEmulatorOption> {
        usize::try_from(*self.selected_emulator_option())
            .ok()
            .and_then(|index| self.rust().rom_emulator_options.get(index).cloned())
    }

    pub fn launch_game(mut self: Pin<&mut Self>) {
        if *self.as_ref().launch_busy() || *self.as_ref().game_running() {
            return;
        }
        if self
            .as_ref()
            .selected_firmware_statuses()
            .iter()
            .any(crate::firmware::FirmwareStatus::needs_action)
        {
            self.as_mut().set_can_launch(false);
            self.as_mut().set_launch_status(qstring(
                "Install or configure the required firmware before launching this game.",
            ));
            return;
        }
        let launch_input = if let Some(install) = self.as_ref().rust().prepared_install.clone() {
            let Some(catalog_database) = crate::catalog::requested_database_path() else {
                self.as_mut().set_launch_status(qstring(
                    "The canonical Lunchbox database is unavailable, so the emulator cannot be selected.",
                ));
                return;
            };
            LaunchInput::Prepared {
                install,
                catalog_database,
            }
        } else {
            let Some(path) = usize::try_from(*self.as_ref().selected_local_file())
                .ok()
                .and_then(|index| self.as_ref().rust().local_file_paths.get(index).cloned())
            else {
                self.as_mut()
                    .set_launch_status(qstring("Select a present local game file to launch."));
                return;
            };
            let Some(option) = self.selected_rom_emulator_option() else {
                self.as_mut().set_launch_status(qstring(
                    "Select an installed compatible emulator before launching this game.",
                ));
                return;
            };
            LaunchInput::Rom {
                path,
                platform: self.as_ref().platform().to_string(),
                option,
            }
        };

        self.as_mut().rust_mut().launch_generation =
            self.as_ref().rust().launch_generation.wrapping_add(1);
        let generation = self.as_ref().rust().launch_generation;
        let game_id = self.as_ref().game_id().to_string();
        let activity_title = self.as_ref().title().to_string();
        let activity_platform = self.as_ref().platform().to_string();
        let activity_database_id = self.as_ref().rust().database_id;
        let rom_probe = is_local_launch_probe();
        self.as_mut().set_launch_busy(true);
        self.as_mut().set_launch_status(qstring(
            "Building the exact launch plan and preparing writable runtime files…",
        ));

        let qt_thread = self.as_ref().qt_thread();
        let started_thread = qt_thread.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-emulator-launch".into())
            .spawn(move || {
                let launch = (|| -> anyhow::Result<(Result<(), String>, Option<String>, bool)> {
                    let plan = match &launch_input {
                        LaunchInput::Prepared {
                            install,
                            catalog_database,
                        } => crate::emulator::build_launch_plan(install, catalog_database)?,
                        LaunchInput::Rom {
                            path,
                            platform,
                            option,
                        } => crate::emulator::build_rom_launch_plan(path, platform, option)?,
                    };
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
                    let play_started = Instant::now();
                    let play_session =
                        crate::settings::SettingsStore::open_default().and_then(|store| {
                            store.begin_play_session(
                                &game_id,
                                activity_database_id,
                                &activity_title,
                                &activity_platform,
                                &emulator_name,
                            )
                        });
                    let activity_recorded = play_session.is_ok();
                    let tracking_warning = play_session
                        .as_ref()
                        .err()
                        .map(|error| format!("Play activity could not be recorded: {error}"));
                    let started_game_id = game_id.clone();
                    let started_warning = tracking_warning.clone();
                    let _ = started_thread.queue(move |mut model| {
                        model.as_mut().finish_launch_started(
                            generation,
                            LaunchStarted {
                                game_id: started_game_id,
                                emulator_name,
                                process_id,
                                command_summary,
                                tracking_warning: started_warning,
                                activity_recorded,
                            },
                        );
                    });
                    let probe_terminated = if rom_probe {
                        std::thread::sleep(Duration::from_millis(1800));
                        match child
                            .try_wait()
                            .context("checking the emulator probe process")?
                        {
                            Some(_) => false,
                            None => {
                                child
                                    .kill()
                                    .context("stopping the emulator probe process")?;
                                true
                            }
                        }
                    } else {
                        false
                    };
                    let status = child.wait();
                    crate::emulator::cleanup_after_launch(&cleanup_paths);
                    let status = status.context("waiting for the emulator process")?;
                    let outcome = if probe_terminated {
                        "terminated"
                    } else if status.success() {
                        "completed"
                    } else {
                        "failed"
                    };
                    let mut tracking_warning = tracking_warning;
                    if let Ok(play_session) = play_session
                        && let Err(error) =
                            crate::settings::SettingsStore::open_default().and_then(|store| {
                                let finalized = store.finish_play_session(
                                    &play_session.session_id,
                                    play_started.elapsed().as_secs(),
                                    outcome,
                                )?;
                                anyhow::ensure!(
                                    finalized,
                                    "the play session was no longer running"
                                );
                                Ok(())
                            })
                    {
                        tracking_warning =
                            Some(format!("Play duration could not be finalized: {error}"));
                    }
                    let exit = if status.success() || probe_terminated {
                        Ok(())
                    } else {
                        Err(format!("emulator exited with {status}"))
                    };
                    Ok((exit, tracking_warning, activity_recorded))
                })();
                match launch {
                    Ok((exit, tracking_warning, activity_recorded)) => {
                        let _ = qt_thread.queue(move |mut model| {
                            model.as_mut().finish_launch_exit(
                                generation,
                                game_id,
                                exit,
                                tracking_warning,
                                activity_recorded,
                            );
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

    fn finish_launch_started(mut self: Pin<&mut Self>, generation: u64, started: LaunchStarted) {
        if generation != self.as_ref().rust().launch_generation
            || self.as_ref().game_id().to_string() != started.game_id
        {
            return;
        }
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_game_running(true);
        self.as_mut()
            .set_emulator_name(qstring(&started.emulator_name));
        let mut status = format!(
            "Running with {} · process {}",
            started.emulator_name, started.process_id
        );
        if let Some(warning) = started.tracking_warning {
            status.push_str(&format!(" · {warning}"));
        }
        self.as_mut().set_launch_status(qstring(status));
        self.as_mut().set_message(qstring(format!(
            "Launched with {}. {}",
            started.emulator_name, started.command_summary
        )));
        if is_emulator_launch_probe() {
            println!(
                "LUNCHBOX_EMULATOR_STARTED name={:?} pid={} activity_recorded={} command={:?}",
                started.emulator_name,
                started.process_id,
                started.activity_recorded,
                started.command_summary,
            );
        }
        if started.activity_recorded {
            self.as_mut().reload_play_activity(true);
        }
    }

    fn finish_launch_exit(
        mut self: Pin<&mut Self>,
        generation: u64,
        completed_game_id: String,
        exit: Result<(), String>,
        tracking_warning: Option<String>,
        activity_recorded: bool,
    ) {
        if generation != self.as_ref().rust().launch_generation
            || self.as_ref().game_id().to_string() != completed_game_id
        {
            return;
        }
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_game_running(false);
        let base_status = match exit {
            Ok(()) => {
                if is_emulator_launch_probe() {
                    println!("LUNCHBOX_EMULATOR_EXITED status=success");
                }
                "The emulator session finished normally.".to_owned()
            }
            Err(error) => {
                if is_emulator_launch_probe() {
                    println!("LUNCHBOX_EMULATOR_EXITED status={error:?}");
                }
                format!("The emulator session ended: {error}")
            }
        };
        self.as_mut()
            .set_launch_status(qstring(match tracking_warning {
                Some(warning) => format!("{base_status} {warning}"),
                None => base_status,
            }));
        if activity_recorded {
            self.as_mut().reload_play_activity(true);
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
        self.as_mut()
            .set_launch_status(qstring(format!("Could not launch this game: {error}")));
        if is_emulator_launch_probe() {
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
                    } else if plan.is_exo_archive_set() {
                        qstring(format!("{} — eXo archive set", plan.display_name))
                    } else {
                        qstring(format!("{} — machine layout", plan.display_name))
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
                    details.push(
                        if plan.is_optical_multidisc() {
                            "MULTI-DISC"
                        } else if plan.is_exo_archive_set() {
                            "EXO ARCHIVE SET"
                        } else if plan.is_arcade_mame_layout() {
                            "MAME ROM + CHD"
                        } else if plan.is_arcade_hypseus_layout() {
                            "HYPSEUS BUNDLE"
                        } else {
                            "DAPHNE BUNDLE"
                        }
                        .to_owned(),
                    );
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
                } else if plan.is_exo_archive_set() {
                    qstring(format!(
                        "{} exact archives · {} total · shared dependencies retained for installation",
                        plan.members.len(),
                        game_details::format_bytes(plan.total_bytes())
                    ))
                } else if plan.is_arcade_mame_layout() {
                    qstring(format!(
                        "MAME ROM + CHD · {} total · staged in the exact set-name layout",
                        game_details::format_bytes(plan.total_bytes())
                    ))
                } else {
                    qstring(format!(
                        "{} required machine files · {} total · stages a launchable ROM/framefile/media bundle",
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
                            } else if plan.is_arcade_machine_layout() {
                                match member.role.as_str() {
                                    "mame-rom" => "MAME ROM set".to_owned(),
                                    "mame-chd" => "Laserdisc CHD".to_owned(),
                                    role if role.ends_with("-rom") => "Machine ROM".to_owned(),
                                    role if role.ends_with("-framefile") => {
                                        "Launch framefile".to_owned()
                                    }
                                    role if role.ends_with("-data") => "Frame data".to_owned(),
                                    role if role.ends_with("-video") => {
                                        "Laserdisc video".to_owned()
                                    }
                                    role if role.ends_with("-audio") => {
                                        "Laserdisc audio".to_owned()
                                    }
                                    role if role.ends_with("-ram") => {
                                        "Machine RAM image".to_owned()
                                    }
                                    _ => "Required machine file".to_owned(),
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

    pub fn local_file_label_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().local_file_paths.get(index))
            .map(|path| {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_else(|| path.as_os_str().to_string_lossy());
                if self.rust().local_file_paths.len() == 1 {
                    qstring(name)
                } else {
                    qstring(format!("{} · {}", name, path.display()))
                }
            })
            .unwrap_or_default()
    }

    pub fn emulator_option_label_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().rom_emulator_options.get(index))
            .map(|option| qstring(option.label()))
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

#[cfg(test)]
mod tests {
    use super::{format_last_played, format_play_time};

    #[test]
    fn activity_labels_stay_compact_in_the_details_card() {
        assert_eq!(format_play_time(0, 0), "Never played");
        assert_eq!(format_play_time(0, 1), "Under 1 min");
        assert_eq!(format_play_time(59, 1), "Under 1 min");
        assert_eq!(format_play_time(60, 1), "1 min");
        assert_eq!(format_play_time(3_600, 1), "1 hr");
        assert_eq!(format_play_time(3_900, 1), "1 hr 5 min");
        assert_eq!(format_last_played(0), "Never");
    }
}
