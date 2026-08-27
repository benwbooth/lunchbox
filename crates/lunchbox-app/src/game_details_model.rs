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
        fn bundle_title_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn bundle_detail_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn file_name_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn file_detail_at(self: &GameDetailsModel, index: i32) -> QString;
    }

    impl cxx_qt::Threading for GameDetailsModel {}
}

use std::pin::Pin;

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
    bundle_count: i32,
    file_count: i32,
    selected_bundle: i32,
    detail_revision: i32,
    bundles: Vec<MinervaBundle>,
    files: Vec<TorrentFileCandidate>,
    details_generation: u64,
    torrent_generation: u64,
    database_id: i64,
}

impl Default for GameDetailsModelRust {
    fn default() -> Self {
        Self {
            panel_open: false,
            loading: false,
            torrent_loading: false,
            download_busy: false,
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
            bundle_count: 0,
            file_count: 0,
            selected_bundle: -1,
            detail_revision: 0,
            bundles: Vec::new(),
            files: Vec::new(),
            details_generation: 0,
            torrent_generation: 0,
            database_id: 0,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn count_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
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
        self.as_mut().rust_mut().details_generation =
            self.as_ref().rust().details_generation.wrapping_add(1);
        self.as_mut().rust_mut().torrent_generation =
            self.as_ref().rust().torrent_generation.wrapping_add(1);
        self.as_mut().set_loading(false);
        self.as_mut().set_torrent_loading(false);
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
        self.as_mut().rust_mut().database_id = 0;
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
        self.as_mut()
            .set_message(qstring("Adding the reviewed file to qBittorrent…"));

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
            .map(|file| qstring(&file.filename))
            .unwrap_or_default()
    }

    pub fn file_detail_at(&self, index: i32) -> QString {
        self.file(index)
            .map(|file| {
                let mut details = Vec::new();
                if index == 0 {
                    details.push("BEST MATCH".to_owned());
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
        },
    )?;
    store.upsert_job(&job)?;
    Ok(title)
}
