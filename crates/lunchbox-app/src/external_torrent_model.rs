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
        #[qproperty(bool, existing_loading)]
        #[qproperty(bool, existing_filter_busy)]
        #[qproperty(bool, importing_existing)]
        #[qproperty(bool, filter_busy)]
        #[qproperty(bool, ready)]
        #[qproperty(bool, collection_mode)]
        #[qproperty(bool, register_for_platform)]
        #[qproperty(i32, file_count)]
        #[qproperty(i32, filtered_file_count)]
        #[qproperty(i32, filtered_revision)]
        #[qproperty(i32, selected_index)]
        #[qproperty(i32, selected_file_count)]
        #[qproperty(bool, suggested_selection)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, queued_revision)]
        #[qproperty(i32, registered_revision)]
        #[qproperty(i32, existing_count)]
        #[qproperty(i32, existing_filtered_count)]
        #[qproperty(i32, existing_filter_revision)]
        #[qproperty(i32, existing_revision)]
        #[qproperty(i32, existing_selected_count)]
        #[qproperty(i32, batch_source_count)]
        #[qproperty(i32, batch_valid_count)]
        #[qproperty(i32, batch_error_count)]
        #[qproperty(i32, batch_revision)]
        #[qproperty(QString, game_title)]
        #[qproperty(QString, game_platform)]
        #[qproperty(QString, source_file_name)]
        #[qproperty(QString, source_kind)]
        #[qproperty(QString, torrent_name)]
        #[qproperty(QString, info_hash)]
        #[qproperty(QString, total_size)]
        #[qproperty(QString, download_scope)]
        #[qproperty(QString, file_filter)]
        #[qproperty(QString, selection_summary)]
        #[qproperty(QString, message)]
        #[qproperty(QString, queued_job_id)]
        #[qproperty(QString, existing_message)]
        #[qproperty(QString, existing_selected_info_hash)]
        type ExternalTorrentModel = super::ExternalTorrentModelRust;

        #[qinvokable]
        fn begin_review(
            self: Pin<&mut ExternalTorrentModel>,
            game_uid: QString,
            launchbox_db_id: i32,
            title: QString,
            platform: QString,
        );

        #[qinvokable]
        fn begin_collection_review(self: Pin<&mut ExternalTorrentModel>, platform: QString);

        #[qinvokable]
        fn set_collection_platform(self: Pin<&mut ExternalTorrentModel>, platform: QString);

        #[qinvokable]
        fn inspect_file(self: Pin<&mut ExternalTorrentModel>, url: QUrl);

        #[qinvokable]
        fn inspect_magnet(self: Pin<&mut ExternalTorrentModel>, magnet_uri: QString);

        #[qinvokable]
        fn refresh_existing_torrents(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn inspect_existing_torrent(self: Pin<&mut ExternalTorrentModel>, index: i32);

        #[qinvokable]
        fn toggle_existing_torrent(self: Pin<&mut ExternalTorrentModel>, index: i32);

        #[qinvokable]
        fn set_existing_torrent_selected(
            self: Pin<&mut ExternalTorrentModel>,
            index: i32,
            selected: bool,
        );

        #[qinvokable]
        fn clear_existing_selection(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn inspect_selected_existing_torrents(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn filter_existing_torrents(self: Pin<&mut ExternalTorrentModel>, query: QString);

        #[qinvokable]
        fn inspect_probe_fixture(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn select_file(self: Pin<&mut ExternalTorrentModel>, index: i32);

        #[qinvokable]
        fn filter_files(self: Pin<&mut ExternalTorrentModel>, query: QString);

        #[qinvokable]
        fn set_platform_registration(self: Pin<&mut ExternalTorrentModel>, enabled: bool);

        #[qinvokable]
        fn queue_selected(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn register_source(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn clear(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn file_path_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn file_size_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn filtered_file_index_at(self: &ExternalTorrentModel, row: i32) -> i32;

        #[qinvokable]
        fn filtered_row_for_file_index(self: &ExternalTorrentModel, index: i32) -> i32;

        #[qinvokable]
        fn file_selected_at(self: &ExternalTorrentModel, index: i32) -> bool;

        #[qinvokable]
        fn file_match_label_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn existing_name_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn existing_detail_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn existing_info_hash_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn existing_filtered_index_at(self: &ExternalTorrentModel, row: i32) -> i32;

        #[qinvokable]
        fn existing_filtered_row_for_info_hash(
            self: &ExternalTorrentModel,
            info_hash: QString,
        ) -> i32;

        #[qinvokable]
        fn existing_selected_at(self: &ExternalTorrentModel, index: i32) -> bool;

        #[qinvokable]
        fn batch_name_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn batch_detail_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn batch_info_hash_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn batch_valid_at(self: &ExternalTorrentModel, index: i32) -> bool;
    }

    impl cxx_qt::Threading for ExternalTorrentModel {}
}

use std::pin::Pin;
use std::sync::Arc;

use anyhow::Context;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::download_plan::DownloadPlan;
use crate::external_torrent::{
    self, CanonicalGameAssociation, CollectionTorrentAssociation, ManualTorrentOffer,
    ReviewedTorrentFile,
};
use crate::game_details::{ReleasePreferences, TorrentFileCandidate};

pub struct ExternalTorrentModelRust {
    busy: bool,
    existing_loading: bool,
    existing_filter_busy: bool,
    importing_existing: bool,
    filter_busy: bool,
    ready: bool,
    collection_mode: bool,
    register_for_platform: bool,
    file_count: i32,
    filtered_file_count: i32,
    filtered_revision: i32,
    selected_index: i32,
    selected_file_count: i32,
    suggested_selection: bool,
    revision: i32,
    queued_revision: i32,
    registered_revision: i32,
    existing_count: i32,
    existing_filtered_count: i32,
    existing_filter_revision: i32,
    existing_revision: i32,
    existing_selected_count: i32,
    batch_source_count: i32,
    batch_valid_count: i32,
    batch_error_count: i32,
    batch_revision: i32,
    game_title: QString,
    game_platform: QString,
    source_file_name: QString,
    source_kind: QString,
    torrent_name: QString,
    info_hash: QString,
    total_size: QString,
    download_scope: QString,
    file_filter: QString,
    selection_summary: QString,
    message: QString,
    queued_job_id: QString,
    existing_message: QString,
    existing_selected_info_hash: QString,
    association: Option<CanonicalGameAssociation>,
    collection_association: Option<CollectionTorrentAssociation>,
    offer: Option<ManualTorrentOffer>,
    display_order: Arc<Vec<usize>>,
    searchable_paths: Arc<Vec<String>>,
    filtered_file_indices: Vec<usize>,
    ranked_file_indices: Vec<usize>,
    selected_file_indices: Vec<usize>,
    selected_download_plan: Option<DownloadPlan>,
    generation: u64,
    filter_generation: u64,
    existing_generation: u64,
    existing_filter_generation: u64,
    existing_torrents: Vec<crate::qbittorrent::ExistingTorrentSummary>,
    existing_searchable_text: Arc<Vec<String>>,
    existing_filtered_indices: Vec<usize>,
    existing_selected_info_hashes: Vec<String>,
    batch_reviews: Vec<BatchSourceReview>,
    probe_fixture_path: Option<std::path::PathBuf>,
}

impl Default for ExternalTorrentModelRust {
    fn default() -> Self {
        Self {
            busy: false,
            existing_loading: false,
            existing_filter_busy: false,
            importing_existing: false,
            filter_busy: false,
            ready: false,
            collection_mode: false,
            register_for_platform: false,
            file_count: 0,
            filtered_file_count: 0,
            filtered_revision: 0,
            selected_index: -1,
            selected_file_count: 0,
            suggested_selection: false,
            revision: 0,
            queued_revision: 0,
            registered_revision: 0,
            existing_count: 0,
            existing_filtered_count: 0,
            existing_filter_revision: 0,
            existing_revision: 0,
            existing_selected_count: 0,
            batch_source_count: 0,
            batch_valid_count: 0,
            batch_error_count: 0,
            batch_revision: 0,
            game_title: QString::default(),
            game_platform: QString::default(),
            source_file_name: QString::default(),
            source_kind: QString::from("file"),
            torrent_name: QString::default(),
            info_hash: QString::default(),
            total_size: QString::default(),
            download_scope: QString::from("Selected file only"),
            file_filter: QString::default(),
            selection_summary: QString::default(),
            message: QString::from(
                "Choose a lawful .torrent file, then review its exact contents before anything is downloaded.",
            ),
            queued_job_id: QString::default(),
            existing_message: QString::from("Checking qBittorrent for torrents you can import…"),
            existing_selected_info_hash: QString::default(),
            association: None,
            collection_association: None,
            offer: None,
            display_order: Arc::new(Vec::new()),
            searchable_paths: Arc::new(Vec::new()),
            filtered_file_indices: Vec::new(),
            ranked_file_indices: Vec::new(),
            selected_file_indices: Vec::new(),
            selected_download_plan: None,
            generation: 0,
            filter_generation: 0,
            existing_generation: 0,
            existing_filter_generation: 0,
            existing_torrents: Vec::new(),
            existing_searchable_text: Arc::new(Vec::new()),
            existing_filtered_indices: Vec::new(),
            existing_selected_info_hashes: Vec::new(),
            batch_reviews: Vec::new(),
            probe_fixture_path: None,
        }
    }
}

struct InspectionResult {
    offer: ManualTorrentOffer,
    ranked_candidates: Vec<TorrentFileCandidate>,
}

struct ReviewState {
    display_order: Vec<usize>,
    ranked_file_indices: Vec<usize>,
    selected_file_indices: Vec<usize>,
    selected_index: i32,
    selected_download_plan: Option<DownloadPlan>,
    reviewable_file_count: usize,
    selection_summary: String,
}

struct ExistingTorrentRefresh {
    torrents: Vec<crate::qbittorrent::ExistingTorrentSummary>,
    searchable_text: Arc<Vec<String>>,
}

#[derive(Clone)]
struct BatchSourceReview {
    name: String,
    info_hash: String,
    result: Result<ManualTorrentOffer, String>,
}

const MAX_BATCH_TORRENTS: usize = 64;

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::ExternalTorrentModel {
    pub fn begin_review(
        mut self: Pin<&mut Self>,
        game_uid: QString,
        launchbox_db_id: i32,
        title: QString,
        platform: QString,
    ) {
        if *self.as_ref().busy() {
            return;
        }
        let association = CanonicalGameAssociation {
            game_uid: game_uid.to_string(),
            launchbox_db_id: i64::from(launchbox_db_id),
            title: title.to_string(),
            platform: platform.to_string(),
        };
        if association.game_uid.trim().is_empty()
            || association.title.trim().is_empty()
            || association.platform.trim().is_empty()
        {
            self.as_mut().set_message(qstring(
                "Select an exact catalog game before adding an external torrent source.",
            ));
            return;
        }
        self.as_mut().schedule_current_offer_cleanup();
        self.as_mut().rust_mut().association = Some(association.clone());
        self.as_mut().rust_mut().collection_association = Some(CollectionTorrentAssociation {
            platform: association.platform.clone(),
        });
        self.as_mut().set_collection_mode(false);
        self.as_mut().set_register_for_platform(false);
        self.as_mut().set_game_title(qstring(association.title));
        self.as_mut()
            .set_game_platform(qstring(association.platform));
        self.as_mut()
            .set_existing_selected_info_hash(QString::default());
        self.as_mut().reset_existing_selection();
        self.as_mut().reset_batch_reviews();
        self.as_mut().reset_offer();
        self.as_mut().set_message(qstring(
            "This source will be associated with the exact game shown above. Torrent labels cannot change catalog identity.",
        ));
        self.as_mut().refresh_existing_torrents();
    }

    pub fn begin_collection_review(mut self: Pin<&mut Self>, platform: QString) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().schedule_current_offer_cleanup();
        let platform = normalized_collection_platform(&platform.to_string());
        self.as_mut().rust_mut().association = None;
        self.as_mut().rust_mut().collection_association = Some(CollectionTorrentAssociation {
            platform: platform.clone(),
        });
        self.as_mut().set_collection_mode(true);
        self.as_mut().set_register_for_platform(false);
        self.as_mut().set_game_title(qstring("Collection torrent"));
        self.as_mut().set_game_platform(qstring(platform));
        self.as_mut()
            .set_existing_selected_info_hash(QString::default());
        self.as_mut().reset_existing_selection();
        self.as_mut().reset_batch_reviews();
        self.as_mut().reset_offer();
        self.as_mut().set_message(qstring(
            "Select one or more loaded torrents to review together, or import a torrent file or magnet. No payload files download when sources are added.",
        ));
        self.as_mut().refresh_existing_torrents();
    }

    pub fn set_collection_platform(mut self: Pin<&mut Self>, platform: QString) {
        if *self.as_ref().busy() || !*self.as_ref().collection_mode() {
            return;
        }
        let platform = normalized_collection_platform(&platform.to_string());
        self.as_mut().rust_mut().collection_association = Some(CollectionTorrentAssociation {
            platform: platform.clone(),
        });
        self.as_mut().set_game_platform(qstring(platform));
    }

    pub fn inspect_file(mut self: Pin<&mut Self>, url: QUrl) {
        if *self.as_ref().busy() {
            return;
        }
        if !self.as_ref().rust().has_review_target() {
            self.as_mut().set_message(qstring(
                "Choose an exact game or collection platform before selecting a torrent.",
            ));
            return;
        }
        let Some(path) = url.to_local_file() else {
            self.as_mut()
                .set_message(qstring("Choose a local .torrent file."));
            return;
        };
        let path = std::path::PathBuf::from(path.to_string());
        self.as_mut().start_inspection(path);
    }

    pub fn inspect_magnet(mut self: Pin<&mut Self>, magnet_uri: QString) {
        if *self.as_ref().busy() {
            return;
        }
        if !self.as_ref().rust().has_review_target() {
            self.as_mut().set_message(qstring(
                "Choose an exact game or collection platform before reviewing a magnet link.",
            ));
            return;
        }
        self.as_mut().reset_existing_selection();
        self.as_mut().reset_batch_reviews();
        let magnet_uri = magnet_uri.to_string();
        let platform = self.as_ref().game_platform().to_string();
        let collection_mode = *self.as_ref().collection_mode();
        let game_title = self.as_ref().rust().review_game_title();
        let previous_offer = self.as_mut().rust_mut().offer.take();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().reset_offer();
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Asking qBittorrent for bounded magnet metadata; payload files remain paused…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-magnet-review".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<InspectionResult> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    if let Some(previous_offer) = previous_offer.as_ref() {
                        external_torrent::discard_unqueued_magnet_review(
                            &settings,
                            &password,
                            previous_offer,
                        )
                        .context("removing the previous unqueued magnet review")?;
                    }
                    let offer = external_torrent::inspect_magnet_source(
                        &settings,
                        &password,
                        &magnet_uri,
                        &platform,
                        collection_mode,
                    )?;
                    prepare_inspection_result(
                        offer,
                        game_title.as_deref(),
                        Some(&platform),
                        &settings,
                    )
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_inspection(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start magnet review: {error}")));
        }
    }

    pub fn refresh_existing_torrents(mut self: Pin<&mut Self>) {
        if *self.as_ref().existing_loading() {
            return;
        }
        self.as_mut().rust_mut().existing_generation =
            self.as_ref().rust().existing_generation.wrapping_add(1);
        let generation = self.as_ref().rust().existing_generation;
        self.as_mut().rust_mut().existing_filter_generation = self
            .as_ref()
            .rust()
            .existing_filter_generation
            .wrapping_add(1);
        self.as_mut().set_existing_loading(true);
        self.as_mut().set_existing_filter_busy(false);
        self.as_mut()
            .set_existing_message(qstring("Checking qBittorrent for torrents you can import…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-qbittorrent-import-list".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<ExistingTorrentRefresh> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    let torrents = crate::qbittorrent::existing_torrents(&settings, &password)?;
                    let searchable_text = Arc::new(
                        torrents
                            .iter()
                            .map(existing_torrent_search_text)
                            .collect::<Vec<_>>(),
                    );
                    Ok(ExistingTorrentRefresh {
                        torrents,
                        searchable_text,
                    })
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_existing_refresh(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_existing_loading(false);
            self.as_mut()
                .set_existing_message(qstring(format!("Could not check qBittorrent: {error}")));
        }
    }

    fn finish_existing_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<ExistingTorrentRefresh, String>,
    ) {
        if generation != self.as_ref().rust().existing_generation {
            return;
        }
        self.as_mut().set_existing_loading(false);
        match result {
            Ok(refresh) => {
                let count = refresh.torrents.len();
                let selected_info_hash = self.as_ref().existing_selected_info_hash().to_string();
                let selection_survives = !selected_info_hash.is_empty()
                    && refresh
                        .torrents
                        .iter()
                        .any(|torrent| torrent.info_hash == selected_info_hash);
                let available_hashes = refresh
                    .torrents
                    .iter()
                    .map(|torrent| torrent.info_hash.clone())
                    .collect::<std::collections::HashSet<_>>();
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.existing_selected_info_hashes
                        .retain(|info_hash| available_hashes.contains(info_hash));
                    rust.existing_torrents = refresh.torrents;
                    rust.existing_searchable_text = refresh.searchable_text;
                    rust.existing_filtered_indices = (0..count).collect();
                }
                self.as_mut()
                    .set_existing_count(i32::try_from(count).unwrap_or(i32::MAX));
                self.as_mut()
                    .set_existing_filtered_count(i32::try_from(count).unwrap_or(i32::MAX));
                let filter_revision = self.as_ref().existing_filter_revision().wrapping_add(1);
                self.as_mut().set_existing_filter_revision(filter_revision);
                let revision = self.as_ref().existing_revision().saturating_add(1);
                self.as_mut().set_existing_revision(revision);
                if !selection_survives {
                    self.as_mut()
                        .set_existing_selected_info_hash(QString::default());
                }
                self.as_mut().sync_existing_selection_properties();
                self.as_mut().set_existing_message(qstring(if count == 0 {
                    "qBittorrent has no exportable v1 torrents loaded.".to_owned()
                } else {
                    format!(
                        "Found {count} loaded {}. Type any title, category, state, path, or info hash to filter.",
                        if count == 1 { "torrent" } else { "torrents" }
                    )
                }));
            }
            Err(error) => {
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.existing_torrents.clear();
                    rust.existing_searchable_text = Arc::new(Vec::new());
                    rust.existing_filtered_indices.clear();
                    rust.existing_selected_info_hashes.clear();
                }
                self.as_mut().set_existing_count(0);
                self.as_mut().set_existing_filtered_count(0);
                self.as_mut()
                    .set_existing_selected_info_hash(QString::default());
                self.as_mut().sync_existing_selection_properties();
                let filter_revision = self.as_ref().existing_filter_revision().wrapping_add(1);
                self.as_mut().set_existing_filter_revision(filter_revision);
                self.as_mut()
                    .set_existing_message(qstring(format!("Could not check qBittorrent: {error}")));
            }
        }
    }

    pub fn filter_existing_torrents(mut self: Pin<&mut Self>, query: QString) {
        if *self.as_ref().existing_loading() {
            return;
        }
        self.as_mut().rust_mut().existing_filter_generation = self
            .as_ref()
            .rust()
            .existing_filter_generation
            .wrapping_add(1);
        let generation = self.as_ref().rust().existing_filter_generation;
        let searchable_text = Arc::clone(&self.as_ref().rust().existing_searchable_text);
        let query = query.to_string();
        self.as_mut().set_existing_filter_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-qbittorrent-list-filter".to_owned())
            .spawn(move || {
                let indices = filter_existing_torrent_indices(&searchable_text, &query);
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_existing_filter(generation, indices);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_existing_filter_busy(false);
            self.as_mut().set_existing_message(qstring(format!(
                "Could not start qBittorrent torrent search: {error}"
            )));
        }
    }

    fn finish_existing_filter(mut self: Pin<&mut Self>, generation: u64, indices: Vec<usize>) {
        if generation != self.as_ref().rust().existing_filter_generation {
            return;
        }
        let count = indices.len();
        self.as_mut().rust_mut().existing_filtered_indices = indices;
        self.as_mut()
            .set_existing_filtered_count(i32::try_from(count).unwrap_or(i32::MAX));
        self.as_mut().set_existing_filter_busy(false);
        let revision = self.as_ref().existing_filter_revision().wrapping_add(1);
        self.as_mut().set_existing_filter_revision(revision);
    }

    pub fn inspect_existing_torrent(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() || !self.as_ref().rust().has_review_target() {
            return;
        }
        let summary = usize::try_from(index).ok().and_then(|index| {
            let model = self.as_ref();
            model.rust().existing_torrents.get(index).cloned()
        });
        let Some(summary) = summary else {
            self.as_mut().set_message(qstring(
                "That qBittorrent torrent is no longer in the import list. Refresh and try again.",
            ));
            return;
        };
        self.as_mut().reset_existing_selection();
        self.as_mut().reset_batch_reviews();
        self.as_mut()
            .set_existing_selected_info_hash(qstring(&summary.info_hash));
        let previous_offer = self.as_mut().rust_mut().offer.take();
        let game_title = self.as_ref().rust().review_game_title();
        let platform = self.as_ref().game_platform().to_string();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().reset_offer();
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Exporting and validating the selected qBittorrent metadata…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-qbittorrent-import-review".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<InspectionResult> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    if let Some(previous_offer) = previous_offer.as_ref() {
                        external_torrent::discard_unqueued_magnet_review(
                            &settings,
                            &password,
                            previous_offer,
                        )?;
                    }
                    let offer = external_torrent::inspect_existing_qbittorrent_source(
                        &settings,
                        &password,
                        &summary.info_hash,
                        &summary.name,
                    )?;
                    prepare_inspection_result(
                        offer,
                        game_title.as_deref(),
                        Some(&platform),
                        &settings,
                    )
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_inspection(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start qBittorrent import: {error}"
            )));
        }
    }

    pub fn toggle_existing_torrent(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() || self.as_ref().rust().collection_association.is_none() {
            return;
        }
        let info_hash = usize::try_from(index).ok().and_then(|index| {
            self.as_ref()
                .rust()
                .existing_torrents
                .get(index)
                .map(|torrent| torrent.info_hash.clone())
        });
        let Some(info_hash) = info_hash else {
            return;
        };
        let should_select = !self
            .as_ref()
            .rust()
            .existing_selected_info_hashes
            .iter()
            .any(|candidate| candidate == &info_hash);
        self.as_mut()
            .set_existing_torrent_selected(index, should_select);
    }

    pub fn set_existing_torrent_selected(
        mut self: Pin<&mut Self>,
        index: i32,
        should_select: bool,
    ) {
        if *self.as_ref().busy() || self.as_ref().rust().collection_association.is_none() {
            return;
        }
        let info_hash = usize::try_from(index).ok().and_then(|index| {
            self.as_ref()
                .rust()
                .existing_torrents
                .get(index)
                .map(|torrent| torrent.info_hash.clone())
        });
        let Some(info_hash) = info_hash else {
            return;
        };
        let current_position = self
            .as_ref()
            .rust()
            .existing_selected_info_hashes
            .iter()
            .position(|candidate| candidate == &info_hash);
        if should_select
            && current_position.is_none()
            && self.as_ref().rust().existing_selected_info_hashes.len() >= MAX_BATCH_TORRENTS
        {
            self.as_mut().set_message(qstring(format!(
                "Select at most {MAX_BATCH_TORRENTS} torrents in one review batch."
            )));
            return;
        }
        match (should_select, current_position) {
            (true, None) => self
                .as_mut()
                .rust_mut()
                .existing_selected_info_hashes
                .push(info_hash),
            (false, Some(position)) => {
                self.as_mut()
                    .rust_mut()
                    .existing_selected_info_hashes
                    .remove(position);
            }
            _ => return,
        }
        self.as_mut()
            .set_existing_selected_info_hash(QString::default());
        self.as_mut().reset_batch_reviews();
        self.as_mut().reset_offer();
        self.as_mut().sync_existing_selection_properties();
        let count = *self.as_ref().existing_selected_count();
        self.as_mut().set_message(qstring(if count == 0 {
            "Select one or more loaded torrents to use as platform sources.".to_owned()
        } else {
            format!(
                "Selected {count} loaded {}. Review validates and indexes metadata only; no game payloads download.",
                if count == 1 { "torrent" } else { "torrents" }
            )
        }));
    }

    pub fn clear_existing_selection(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().reset_existing_selection();
        self.as_mut().reset_batch_reviews();
        self.as_mut().reset_offer();
        self.as_mut().set_message(qstring(
            "Select one or more loaded torrents to review together.",
        ));
    }

    pub fn inspect_selected_existing_torrents(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || self.as_ref().rust().collection_association.is_none() {
            return;
        }
        let selected_hashes = self
            .as_ref()
            .rust()
            .existing_selected_info_hashes
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let summaries = self
            .as_ref()
            .rust()
            .existing_torrents
            .iter()
            .filter(|torrent| selected_hashes.contains(&torrent.info_hash))
            .cloned()
            .collect::<Vec<_>>();
        if summaries.is_empty() {
            self.as_mut()
                .set_message(qstring("Select at least one loaded torrent to review."));
            return;
        }
        let selected_count = summaries.len();
        let previous_offer = self.as_mut().rust_mut().offer.take();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().reset_batch_reviews();
        self.as_mut().reset_offer();
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(format!(
            "Exporting and validating {selected_count} selected torrent{}…",
            if selected_count == 1 { "" } else { "s" }
        )));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-qbittorrent-batch-review".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<Vec<BatchSourceReview>> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    if let Some(previous_offer) = previous_offer.as_ref() {
                        external_torrent::discard_unqueued_magnet_review(
                            &settings,
                            &password,
                            previous_offer,
                        )?;
                    }
                    Ok(summaries
                        .into_iter()
                        .map(|summary| {
                            let result = external_torrent::inspect_existing_qbittorrent_source(
                                &settings,
                                &password,
                                &summary.info_hash,
                                &summary.name,
                            )
                            .map_err(|error| format!("{error:#}"));
                            BatchSourceReview {
                                name: summary.name,
                                info_hash: summary.info_hash,
                                result,
                            }
                        })
                        .collect())
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_batch_inspection(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the torrent batch review: {error}"
            )));
        }
    }

    fn finish_batch_inspection(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<Vec<BatchSourceReview>, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(reviews) => {
                let source_count = reviews.len();
                let valid_count = reviews
                    .iter()
                    .filter(|review| review.result.is_ok())
                    .count();
                let error_count = source_count.saturating_sub(valid_count);
                self.as_mut().rust_mut().batch_reviews = reviews;
                self.as_mut()
                    .set_batch_source_count(i32::try_from(source_count).unwrap_or(i32::MAX));
                self.as_mut()
                    .set_batch_valid_count(i32::try_from(valid_count).unwrap_or(i32::MAX));
                self.as_mut()
                    .set_batch_error_count(i32::try_from(error_count).unwrap_or(i32::MAX));
                self.as_mut().set_ready(valid_count > 0);
                self.as_mut().bump_batch_revision();
                self.as_mut().set_message(qstring(if error_count == 0 {
                    format!(
                        "Reviewed {valid_count} torrent{}. Add the validated sources to make their games available on demand.",
                        if valid_count == 1 { "" } else { "s" }
                    )
                } else {
                    format!(
                        "Reviewed {source_count} torrents: {valid_count} valid, {error_count} rejected. Only validated sources will be added."
                    )
                }));
            }
            Err(error) => {
                self.as_mut().reset_batch_reviews();
                self.as_mut().set_message(qstring(format!(
                    "Could not review the selected torrents: {error}"
                )));
            }
        }
    }

    pub fn inspect_probe_fixture(mut self: Pin<&mut Self>) {
        if !std::env::args().any(|argument| argument == "--manual-torrent-ui-probe") {
            self.as_mut().set_message(qstring(
                "The deterministic fixture is available only to the UI probe.",
            ));
            return;
        }
        if *self.as_ref().busy() {
            return;
        }
        let path = std::env::temp_dir()
            .join(format!(
                "lunchbox-manual-torrent-probe-{}",
                std::process::id()
            ))
            .join("reviewed-sample-source.torrent");
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            self.as_mut().set_message(qstring(format!(
                "Could not create the deterministic torrent fixture directory: {error}"
            )));
            return;
        }
        if let Err(error) = std::fs::write(&path, external_torrent::probe_fixture_torrent_bytes()) {
            self.as_mut().set_message(qstring(format!(
                "Could not create the deterministic torrent fixture: {error}"
            )));
            return;
        }
        self.as_mut().rust_mut().probe_fixture_path = Some(path.clone());
        self.as_mut().start_inspection(path);
    }

    fn start_inspection(mut self: Pin<&mut Self>, path: std::path::PathBuf) {
        if *self.as_ref().busy() {
            return;
        }
        if !self.as_ref().rust().has_review_target() {
            self.as_mut().set_message(qstring(
                "Choose an exact game or collection platform before selecting a torrent.",
            ));
            return;
        }
        self.as_mut().reset_existing_selection();
        self.as_mut().reset_batch_reviews();
        let previous_offer = self.as_mut().rust_mut().offer.take();
        let game_title = self.as_ref().rust().review_game_title();
        let platform = self.as_ref().game_platform().to_string();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().reset_offer();
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Inspecting bounded torrent metadata…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-torrent-review".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<InspectionResult> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    if let Some(previous_offer) = previous_offer.as_ref() {
                        let password = crate::settings::load_password()?.unwrap_or_default();
                        external_torrent::discard_unqueued_magnet_review(
                            &settings,
                            &password,
                            previous_offer,
                        )
                        .context("removing the previous unqueued magnet review")?;
                    }
                    let offer = external_torrent::inspect_torrent_file(&path)?;
                    prepare_inspection_result(
                        offer,
                        game_title.as_deref(),
                        Some(&platform),
                        &settings,
                    )
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_inspection(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start torrent review: {error}")));
        }
    }

    fn finish_inspection(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<InspectionResult, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(result) => {
                let offer = result.offer;
                let file_count = offer.files.len();
                let collection_mode = *self.as_ref().collection_mode();
                let platform = self.as_ref().game_platform().to_string();
                let review = build_review_state(
                    &offer,
                    result.ranked_candidates,
                    collection_mode,
                    Some(&platform),
                );
                let searchable_paths = offer
                    .files
                    .iter()
                    .map(|file| file.path.to_lowercase())
                    .collect::<Vec<_>>();
                let selected_count = review.selected_file_indices.len();
                self.as_mut()
                    .set_file_count(i32::try_from(file_count).unwrap_or(i32::MAX));
                self.as_mut().set_filtered_file_count(
                    i32::try_from(review.display_order.len()).unwrap_or(i32::MAX),
                );
                self.as_mut().set_selected_index(review.selected_index);
                self.as_mut()
                    .set_selected_file_count(i32::try_from(selected_count).unwrap_or(i32::MAX));
                self.as_mut()
                    .set_suggested_selection(!collection_mode && review.selected_index >= 0);
                self.as_mut()
                    .set_selection_summary(qstring(&review.selection_summary));
                self.as_mut()
                    .set_source_file_name(qstring(&offer.source_file_name));
                self.as_mut().set_source_kind(qstring(&offer.source_kind));
                self.as_mut().set_torrent_name(qstring(&offer.torrent_name));
                self.as_mut().set_info_hash(qstring(&offer.info_hash));
                self.as_mut()
                    .set_importing_existing(offer.externally_managed);
                self.as_mut()
                    .set_total_size(qstring(crate::game_details::format_bytes(
                        offer.total_bytes,
                    )));
                self.as_mut()
                    .set_download_scope(qstring(if collection_mode {
                        "Indexed source · downloads on demand".to_owned()
                    } else if selected_count > 1 {
                        format!("{selected_count} related files selected")
                    } else if selected_count == 1 {
                        "Selected file only".to_owned()
                    } else {
                        "Choose an exact payload".to_owned()
                    }));
                self.as_mut().set_message(qstring(if collection_mode {
                    if offer.externally_managed {
                        format!(
                            "Reviewed {file_count} safe {}. Add Source imports this torrent into Lunchbox management and indexes its games.",
                            if file_count == 1 { "file" } else { "files" }
                        )
                    } else {
                        format!(
                            "Reviewed {file_count} safe {}. Add the source now; individual games will appear in their matching Get lists.",
                            if file_count == 1 { "file" } else { "files" }
                        )
                    }
                } else {
                    if review.selected_index < 0 {
                        format!(
                            "Reviewed {file_count} safe {} and found {} playable candidate{}. No title match was strong enough to select automatically; search or choose the exact payload.",
                            if file_count == 1 { "file" } else { "files" },
                            review.reviewable_file_count,
                            if review.reviewable_file_count == 1 { "" } else { "s" },
                        )
                    } else if offer.externally_managed {
                        format!(
                            "Reviewed {file_count} safe {} and excluded non-playable sidecars. Lunchbox put the playable match first; review it before Queue imports the exact selection.",
                            if file_count == 1 { "file" } else { "files" },
                        )
                    } else {
                        format!(
                            "Reviewed {file_count} safe {} and excluded non-playable sidecars. Lunchbox put the playable match first; review the suggested exact payload before Queue.",
                            if file_count == 1 { "file" } else { "files" },
                        )
                    }
                }));
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.display_order = Arc::new(review.display_order.clone());
                    rust.searchable_paths = Arc::new(searchable_paths);
                    rust.filtered_file_indices = review.display_order;
                    rust.ranked_file_indices = review.ranked_file_indices;
                    rust.selected_file_indices = review.selected_file_indices;
                    rust.selected_download_plan = review.selected_download_plan;
                    rust.offer = Some(offer);
                }
                self.as_mut().bump_filtered_revision();
                self.as_mut().bump_revision();
                self.as_mut().set_ready(true);
            }
            Err(error) => {
                self.as_mut()
                    .set_message(qstring(format!("Could not review this torrent: {error}")));
            }
        }
    }

    pub fn select_file(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() || !*self.as_ref().ready() {
            return;
        }
        let valid = usize::try_from(index)
            .ok()
            .is_some_and(|index| self.as_ref().rust().file(index).is_some());
        if !valid {
            self.as_mut()
                .set_message(qstring("The selected torrent file is no longer available."));
            return;
        }
        self.as_mut().set_selected_index(index);
        self.as_mut().set_selected_file_count(1);
        self.as_mut().set_suggested_selection(false);
        self.as_mut().set_selection_summary(qstring(
            "Exact file selected manually. Queue will download only this reviewed payload.",
        ));
        self.as_mut()
            .set_download_scope(qstring("Selected file only"));
        self.as_mut().rust_mut().selected_file_indices = vec![usize::try_from(index).unwrap()];
        self.as_mut().rust_mut().selected_download_plan = None;
        self.as_mut().bump_revision();
    }

    pub fn filter_files(mut self: Pin<&mut Self>, query: QString) {
        if !*self.as_ref().ready() {
            return;
        }
        let query = query.to_string().chars().take(512).collect::<String>();
        self.as_mut().set_file_filter(qstring(&query));
        self.as_mut().rust_mut().filter_generation =
            self.as_ref().rust().filter_generation.wrapping_add(1);
        let generation = self.as_ref().rust().filter_generation;
        let terms = query
            .split_whitespace()
            .map(str::to_lowercase)
            .filter(|term| !term.is_empty())
            .collect::<Vec<_>>();
        if terms.is_empty() {
            let indices = self.as_ref().rust().display_order.as_ref().clone();
            let count = indices.len();
            self.as_mut().rust_mut().filtered_file_indices = indices;
            self.as_mut()
                .set_filtered_file_count(i32::try_from(count).unwrap_or(i32::MAX));
            self.as_mut().set_filter_busy(false);
            self.as_mut().bump_filtered_revision();
            return;
        }

        let display_order = Arc::clone(&self.as_ref().rust().display_order);
        let searchable_paths = Arc::clone(&self.as_ref().rust().searchable_paths);
        self.as_mut().set_filter_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-torrent-file-filter".to_owned())
            .spawn(move || {
                let indices = filter_display_order(&display_order, &searchable_paths, &terms);
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_file_filter(generation, indices);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_filter_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not search torrent contents: {error}"
            )));
        }
    }

    fn finish_file_filter(mut self: Pin<&mut Self>, generation: u64, indices: Vec<usize>) {
        if generation != self.as_ref().rust().filter_generation {
            return;
        }
        let count = indices.len();
        self.as_mut().rust_mut().filtered_file_indices = indices;
        self.as_mut()
            .set_filtered_file_count(i32::try_from(count).unwrap_or(i32::MAX));
        self.as_mut().set_filter_busy(false);
        self.as_mut().bump_filtered_revision();
    }

    pub fn set_platform_registration(mut self: Pin<&mut Self>, enabled: bool) {
        if *self.as_ref().busy() || *self.as_ref().collection_mode() {
            return;
        }
        self.as_mut().set_register_for_platform(enabled);
    }

    pub fn queue_selected(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || !*self.as_ref().ready() {
            return;
        }
        if *self.as_ref().collection_mode() {
            self.as_mut().set_message(qstring(
                "Add this torrent as a source; individual games are downloaded later from their Get lists.",
            ));
            return;
        }
        let Some(offer) = self.as_ref().rust().offer.clone() else {
            self.as_mut()
                .set_message(qstring("Choose and review a torrent first."));
            return;
        };
        let Ok(selected_index) = usize::try_from(*self.as_ref().selected_index()) else {
            self.as_mut()
                .set_message(qstring("Choose an exact file before queueing."));
            return;
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Adding the reviewed selection to Lunchbox downloads…",
        ));
        let association = self.as_ref().rust().association.clone();
        let register_for_platform = *self.as_ref().register_for_platform();
        let download_plan = self.as_ref().rust().selected_download_plan.clone();
        let generation = self.as_ref().rust().generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-torrent-enqueue".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<external_torrent::ManualTorrentQueueResult> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    let association = association
                        .context("the exact catalog association is no longer available")?;
                    external_torrent::queue_manual_torrent(
                        &settings,
                        &password,
                        &store,
                        association,
                        offer,
                        selected_index,
                        download_plan,
                        register_for_platform,
                    )
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_queue(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the download request: {error}"
            )));
        }
    }

    fn finish_queue(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<external_torrent::ManualTorrentQueueResult, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(outcome) => {
                if let Some(offer) = self.as_mut().rust_mut().offer.as_mut() {
                    offer.magnet_review_created = false;
                }
                self.as_mut().set_queued_job_id(qstring(outcome.job_id));
                if outcome.platform_registered {
                    let revision = self.as_ref().registered_revision().saturating_add(1);
                    self.as_mut().set_registered_revision(revision);
                }
                let revision = self.as_ref().queued_revision().saturating_add(1);
                self.as_mut().set_queued_revision(revision);
                self.as_mut().set_message(qstring(if outcome.platform_registered {
                    "Queued the selected game and added this torrent as a reusable source for the platform."
                        .to_owned()
                } else if let Some(error) = outcome.platform_registration_error {
                    format!(
                        "Queued the selected game, but could not add the reusable platform source: {error}"
                    )
                } else {
                    "Queued with exact game, source, info-hash, and file provenance.".to_owned()
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not queue this torrent: {error}"))),
        }
    }

    pub fn register_source(mut self: Pin<&mut Self>) {
        let has_batch = !self.as_ref().rust().batch_reviews.is_empty();
        if *self.as_ref().busy()
            || !*self.as_ref().ready()
            || (!*self.as_ref().collection_mode() && !has_batch)
        {
            return;
        }
        let Some(association) = self.as_ref().rust().collection_association.clone() else {
            self.as_mut()
                .set_message(qstring("Choose the platform represented by this torrent."));
            return;
        };
        let batch_offers = self
            .as_ref()
            .rust()
            .batch_reviews
            .iter()
            .filter_map(|review| review.result.as_ref().ok().cloned())
            .collect::<Vec<_>>();
        let offer = self.as_ref().rust().offer.clone();
        if batch_offers.is_empty() && offer.is_none() {
            self.as_mut()
                .set_message(qstring("Choose and review one or more torrents first."));
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring(if batch_offers.is_empty() {
                "Indexing exact torrent members for on-demand game downloads…".to_owned()
            } else {
                format!(
                    "Adding {} validated torrent catalogs in one transaction…",
                    batch_offers.len()
                )
            }));
        let generation = self.as_ref().rust().generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-torrent-source-register".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<usize> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    if batch_offers.is_empty() {
                        external_torrent::register_collection_torrent(
                            &settings,
                            &password,
                            &store,
                            association,
                            offer.context("the reviewed torrent is no longer available")?,
                        )
                        .map(|_| 1)
                    } else {
                        external_torrent::register_collection_torrents(
                            &settings,
                            &password,
                            &store,
                            association,
                            batch_offers,
                        )
                    }
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_registration(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start torrent source registration: {error}"
            )));
        }
    }

    fn finish_registration(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<usize, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(source_count) => {
                if let Some(offer) = self.as_mut().rust_mut().offer.as_mut() {
                    offer.magnet_review_created = false;
                }
                self.as_mut().reset_existing_selection();
                self.as_mut().reset_batch_reviews();
                self.as_mut().set_ready(false);
                let revision = self.as_ref().registered_revision().saturating_add(1);
                self.as_mut().set_registered_revision(revision);
                self.as_mut().set_message(qstring(format!(
                    "Saved and indexed {source_count} torrent source{}. Matching games can download individual files even if the original qBittorrent entries are later removed.",
                    if source_count == 1 { "" } else { "s" }
                )));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not add this torrent source: {error}"
            ))),
        }
    }

    pub fn clear(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().schedule_current_offer_cleanup();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().rust_mut().association = None;
        self.as_mut().rust_mut().collection_association = None;
        self.as_mut().set_collection_mode(false);
        self.as_mut().set_register_for_platform(false);
        self.as_mut().set_game_title(QString::default());
        self.as_mut().set_game_platform(QString::default());
        self.as_mut()
            .set_existing_selected_info_hash(QString::default());
        self.as_mut().reset_existing_selection();
        self.as_mut().reset_batch_reviews();
        self.as_mut().reset_offer();
        self.as_mut().set_message(qstring(
            "Choose a lawful .torrent file, then review its exact contents before anything is downloaded.",
        ));
    }

    pub fn file_path_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().file(index))
            .map(|file| qstring(&file.path))
            .unwrap_or_default()
    }

    pub fn file_size_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().file(index))
            .map(|file| qstring(crate::game_details::format_bytes(file.byte_size)))
            .unwrap_or_default()
    }

    pub fn filtered_file_index_at(&self, row: i32) -> i32 {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.rust().filtered_file_indices.get(row).copied())
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    pub fn filtered_row_for_file_index(&self, index: i32) -> i32 {
        let Ok(index) = usize::try_from(index) else {
            return -1;
        };
        self.rust()
            .filtered_file_indices
            .iter()
            .position(|candidate| *candidate == index)
            .and_then(|row| i32::try_from(row).ok())
            .unwrap_or(-1)
    }

    pub fn file_selected_at(&self, index: i32) -> bool {
        usize::try_from(index)
            .ok()
            .is_some_and(|index| self.rust().selected_file_indices.contains(&index))
    }

    pub fn file_match_label_at(&self, index: i32) -> QString {
        let Ok(index) = usize::try_from(index) else {
            return QString::default();
        };
        if self.rust().selected_file_indices.contains(&index) {
            if *self.selected_index() == i32::try_from(index).unwrap_or(-1) {
                return qstring(if *self.suggested_selection() {
                    "BEST MATCH"
                } else {
                    "REVIEWED SELECTION"
                });
            }
            return qstring("RELATED FILE");
        }
        if self.rust().ranked_file_indices.contains(&index) {
            return qstring("TITLE MATCH");
        }
        QString::default()
    }

    pub fn existing_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().existing_torrents.get(index))
            .map(|torrent| {
                qstring(if torrent.name.trim().is_empty() {
                    &torrent.info_hash
                } else {
                    &torrent.name
                })
            })
            .unwrap_or_default()
    }

    pub fn existing_detail_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().existing_torrents.get(index))
            .map(|torrent| qstring(existing_torrent_detail(torrent)))
            .unwrap_or_default()
    }

    pub fn existing_info_hash_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().existing_torrents.get(index))
            .map(|torrent| qstring(&torrent.info_hash))
            .unwrap_or_default()
    }

    pub fn existing_filtered_index_at(&self, row: i32) -> i32 {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.rust().existing_filtered_indices.get(row).copied())
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    pub fn existing_filtered_row_for_info_hash(&self, info_hash: QString) -> i32 {
        let info_hash = info_hash.to_string();
        if info_hash.is_empty() {
            return -1;
        }
        self.rust()
            .existing_filtered_indices
            .iter()
            .position(|index| {
                self.rust()
                    .existing_torrents
                    .get(*index)
                    .is_some_and(|torrent| torrent.info_hash == info_hash)
            })
            .and_then(|row| i32::try_from(row).ok())
            .unwrap_or(-1)
    }

    pub fn existing_selected_at(&self, index: i32) -> bool {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().existing_torrents.get(index))
            .is_some_and(|torrent| {
                self.rust()
                    .existing_selected_info_hashes
                    .contains(&torrent.info_hash)
            })
    }

    pub fn batch_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().batch_reviews.get(index))
            .map(|review| qstring(&review.name))
            .unwrap_or_default()
    }

    pub fn batch_detail_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().batch_reviews.get(index))
            .map(|review| match &review.result {
                Ok(offer) => qstring(format!(
                    "{} {} · {} · metadata retained",
                    offer.files.len(),
                    if offer.files.len() == 1 {
                        "file"
                    } else {
                        "files"
                    },
                    crate::game_details::format_bytes(offer.total_bytes)
                )),
                Err(error) => qstring(error),
            })
            .unwrap_or_default()
    }

    pub fn batch_info_hash_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().batch_reviews.get(index))
            .map(|review| qstring(&review.info_hash))
            .unwrap_or_default()
    }

    pub fn batch_valid_at(&self, index: i32) -> bool {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().batch_reviews.get(index))
            .is_some_and(|review| review.result.is_ok())
    }

    fn reset_offer(mut self: Pin<&mut Self>) {
        {
            let mut rust = self.as_mut().rust_mut();
            rust.offer = None;
            rust.display_order = Arc::new(Vec::new());
            rust.searchable_paths = Arc::new(Vec::new());
            rust.filtered_file_indices.clear();
            rust.ranked_file_indices.clear();
            rust.selected_file_indices.clear();
            rust.selected_download_plan = None;
            rust.filter_generation = rust.filter_generation.wrapping_add(1);
        }
        self.as_mut().set_ready(false);
        self.as_mut().set_importing_existing(false);
        self.as_mut().set_filter_busy(false);
        self.as_mut().set_file_count(0);
        self.as_mut().set_filtered_file_count(0);
        self.as_mut().set_selected_index(-1);
        self.as_mut().set_selected_file_count(0);
        self.as_mut().set_suggested_selection(false);
        self.as_mut().set_file_filter(QString::default());
        self.as_mut().set_selection_summary(QString::default());
        self.as_mut().set_source_file_name(QString::default());
        self.as_mut().set_source_kind(qstring("file"));
        self.as_mut().set_torrent_name(QString::default());
        self.as_mut().set_info_hash(QString::default());
        self.as_mut().set_total_size(QString::default());
        self.as_mut()
            .set_download_scope(qstring("Selected file only"));
        self.as_mut().set_queued_job_id(QString::default());
        self.as_mut().bump_filtered_revision();
        self.as_mut().bump_revision();
    }

    fn reset_existing_selection(mut self: Pin<&mut Self>) {
        self.as_mut()
            .rust_mut()
            .existing_selected_info_hashes
            .clear();
        self.as_mut().sync_existing_selection_properties();
    }

    fn sync_existing_selection_properties(mut self: Pin<&mut Self>) {
        let count = self.as_ref().rust().existing_selected_info_hashes.len();
        self.as_mut()
            .set_existing_selected_count(i32::try_from(count).unwrap_or(i32::MAX));
        let revision = self.as_ref().existing_revision().saturating_add(1);
        self.as_mut().set_existing_revision(revision);
    }

    fn reset_batch_reviews(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().batch_reviews.clear();
        self.as_mut().set_batch_source_count(0);
        self.as_mut().set_batch_valid_count(0);
        self.as_mut().set_batch_error_count(0);
        self.as_mut().bump_batch_revision();
    }

    fn bump_batch_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().batch_revision().saturating_add(1);
        self.as_mut().set_batch_revision(revision);
    }

    fn schedule_current_offer_cleanup(mut self: Pin<&mut Self>) {
        let Some(offer) = self.as_mut().rust_mut().offer.take() else {
            return;
        };
        if offer.source_kind != "magnet" || !offer.magnet_review_created {
            return;
        }
        let _ = std::thread::Builder::new()
            .name("lunchbox-magnet-review-cleanup".to_owned())
            .spawn(move || {
                let _ = (|| -> anyhow::Result<()> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    external_torrent::discard_unqueued_magnet_review(&settings, &password, &offer)
                })();
            });
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().saturating_add(1);
        self.as_mut().set_revision(revision);
    }

    fn bump_filtered_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().filtered_revision().saturating_add(1);
        self.as_mut().set_filtered_revision(revision);
    }
}

impl ExternalTorrentModelRust {
    fn file(&self, index: usize) -> Option<&ReviewedTorrentFile> {
        self.offer.as_ref()?.files.get(index)
    }

    fn has_review_target(&self) -> bool {
        self.association.is_some() || self.collection_association.is_some()
    }

    fn review_game_title(&self) -> Option<String> {
        self.association
            .as_ref()
            .map(|association| association.title.clone())
    }
}

fn prepare_inspection_result(
    offer: ManualTorrentOffer,
    game_title: Option<&str>,
    platform: Option<&str>,
    settings: &crate::settings::AppSettings,
) -> anyhow::Result<InspectionResult> {
    let ranked_candidates = if let Some(game_title) = game_title {
        let files = offer
            .files
            .iter()
            .map(|file| TorrentFileCandidate {
                index: file.index as usize,
                filename: file.path.clone(),
                byte_size: file.byte_size,
                match_score: 0.0,
                matched_title: String::new(),
                region: String::new(),
                version: String::new(),
                download_plan: None,
            })
            .collect();
        crate::game_details::rank_file_candidates_for_platform(
            files,
            game_title,
            &[],
            &ReleasePreferences {
                region_priority: settings.region_priority.clone(),
                version_preference: settings.version_preference.clone(),
            },
            platform,
        )?
    } else {
        Vec::new()
    };
    Ok(InspectionResult {
        offer,
        ranked_candidates,
    })
}

fn build_review_state(
    offer: &ManualTorrentOffer,
    ranked_candidates: Vec<TorrentFileCandidate>,
    collection_mode: bool,
    platform: Option<&str>,
) -> ReviewState {
    let positions_by_torrent_index = offer
        .files
        .iter()
        .enumerate()
        .map(|(position, file)| (file.index as usize, position))
        .collect::<std::collections::HashMap<_, _>>();
    let position_for_torrent_index =
        |torrent_index: usize| positions_by_torrent_index.get(&torrent_index).copied();
    // Ranking and review are separate trust boundaries. Revalidate the actual
    // torrent member here so a stale, malformed, or future ranking result can
    // never automatically select tiny MIG card metadata instead of a playable
    // Switch image.
    let ranked_candidates = ranked_candidates
        .into_iter()
        .filter(|candidate| {
            position_for_torrent_index(candidate.index)
                .and_then(|position| offer.files.get(position))
                .is_some_and(|file| {
                    let actual = TorrentFileCandidate {
                        index: file.index as usize,
                        filename: file.path.clone(),
                        byte_size: file.byte_size,
                        match_score: 0.0,
                        matched_title: String::new(),
                        region: String::new(),
                        version: String::new(),
                        download_plan: None,
                    };
                    crate::game_details::platform_payload_tier(platform, &actual).is_some()
                })
        })
        .collect::<Vec<_>>();
    let mut seen_file_indices = vec![false; offer.files.len()];
    let mut ranked_file_indices = Vec::with_capacity(ranked_candidates.len());
    for candidate in &ranked_candidates {
        if let Some(index) = position_for_torrent_index(candidate.index)
            && !seen_file_indices[index]
        {
            seen_file_indices[index] = true;
            ranked_file_indices.push(index);
        }
    }
    let reviewable_file = offer
        .files
        .iter()
        .map(|file| {
            if collection_mode {
                return true;
            }
            let candidate = TorrentFileCandidate {
                index: file.index as usize,
                filename: file.path.clone(),
                byte_size: file.byte_size,
                match_score: 0.0,
                matched_title: String::new(),
                region: String::new(),
                version: String::new(),
                download_plan: None,
            };
            crate::game_details::platform_payload_tier(platform, &candidate).is_some()
        })
        .collect::<Vec<_>>();
    let reviewable_file_count = reviewable_file
        .iter()
        .filter(|reviewable| **reviewable)
        .count();
    let mut display_order = ranked_file_indices.clone();
    for index in 0..offer.files.len() {
        if reviewable_file[index] && !seen_file_indices[index] {
            display_order.push(index);
        }
    }
    if collection_mode {
        return ReviewState {
            display_order,
            ranked_file_indices,
            selected_file_indices: Vec::new(),
            selected_index: -1,
            selected_download_plan: None,
            reviewable_file_count,
            selection_summary: "All safe members will be indexed; no payload downloads now."
                .to_owned(),
        };
    }
    let Some(best) = ranked_candidates.first() else {
        return ReviewState {
            display_order,
            ranked_file_indices,
            selected_file_indices: Vec::new(),
            selected_index: -1,
            selected_download_plan: None,
            reviewable_file_count,
            selection_summary:
                "No strong title match found. Search or select the exact game payload.".to_owned(),
        };
    };
    let Some(selected_position) = position_for_torrent_index(best.index) else {
        return ReviewState {
            display_order,
            ranked_file_indices,
            selected_file_indices: Vec::new(),
            selected_index: -1,
            selected_download_plan: None,
            reviewable_file_count,
            selection_summary:
                "No strong title match found. Search or select the exact game payload.".to_owned(),
        };
    };
    let mut selected_file_indices = vec![selected_position];
    if let Some(plan) = best.download_plan.as_ref() {
        for member in &plan.members {
            if let Some(index) = position_for_torrent_index(member.index)
                && !selected_file_indices.contains(&index)
            {
                selected_file_indices.push(index);
            }
        }
    }
    let mut prioritized_display_order = Vec::with_capacity(display_order.len());
    let mut prioritized = vec![false; offer.files.len()];
    for index in &selected_file_indices {
        if !prioritized[*index] {
            prioritized[*index] = true;
            prioritized_display_order.push(*index);
        }
    }
    for index in display_order {
        if !prioritized[index] {
            prioritized[index] = true;
            prioritized_display_order.push(index);
        }
    }
    let selection_summary = if selected_file_indices.len() > 1 {
        format!(
            "Best title match selected with {} required related files. Review before queueing.",
            selected_file_indices.len()
        )
    } else {
        "Best title match selected automatically. Review before queueing.".to_owned()
    };
    ReviewState {
        display_order: prioritized_display_order,
        ranked_file_indices,
        selected_file_indices,
        selected_index: i32::try_from(selected_position).unwrap_or(-1),
        selected_download_plan: best.download_plan.clone(),
        reviewable_file_count,
        selection_summary,
    }
}

fn filter_display_order(
    display_order: &[usize],
    searchable_paths: &[String],
    terms: &[String],
) -> Vec<usize> {
    display_order
        .iter()
        .copied()
        .filter(|index| {
            searchable_paths
                .get(*index)
                .is_some_and(|path| terms.iter().all(|term| path.contains(term)))
        })
        .collect()
}

fn existing_torrent_search_text(torrent: &crate::qbittorrent::ExistingTorrentSummary) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        torrent.name, torrent.info_hash, torrent.category, torrent.state, torrent.save_path,
    )
    .to_lowercase()
}

fn existing_torrent_detail(torrent: &crate::qbittorrent::ExistingTorrentSummary) -> String {
    let percent = (torrent.progress * 100.0).round();
    let selected_size = crate::game_details::format_bytes(torrent.size);
    let total_size = crate::game_details::format_bytes(torrent.total_size);
    let size_summary = if torrent.size < torrent.total_size {
        format!("{total_size} total · {selected_size} currently selected")
    } else {
        total_size
    };
    let category = if torrent.category.trim().is_empty() {
        "uncategorized"
    } else {
        torrent.category.as_str()
    };
    format!(
        "{percent:.0}% · {size_summary} · {} · {category}",
        torrent.state
    )
}

fn filter_existing_torrent_indices(searchable_text: &[String], query: &str) -> Vec<usize> {
    let terms = query
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return (0..searchable_text.len()).collect();
    }
    searchable_text
        .iter()
        .enumerate()
        .filter_map(|(index, text)| {
            terms
                .iter()
                .all(|term| text.contains(term))
                .then_some(index)
        })
        .collect()
}

fn normalized_collection_platform(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "Unassigned platform".to_owned()
    } else {
        value.chars().take(512).collect()
    }
}

impl Drop for ExternalTorrentModelRust {
    fn drop(&mut self) {
        if let Some(path) = self.probe_fixture_path.take() {
            let _ = std::fs::remove_file(&path);
            if let Some(parent) = path.parent() {
                let _ = std::fs::remove_dir(parent);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(files: &[(u32, &str, u64)]) -> ManualTorrentOffer {
        ManualTorrentOffer {
            source_file_name: "collection.torrent".to_owned(),
            source_kind: "file".to_owned(),
            source_label: "collection.torrent".to_owned(),
            torrent_name: "Collection".to_owned(),
            torrent_sha256: "0".repeat(64),
            info_hash: "1".repeat(40),
            total_bytes: files.iter().map(|(_, _, size)| size).sum(),
            files: files
                .iter()
                .map(|(index, path, byte_size)| ReviewedTorrentFile {
                    index: *index,
                    path: (*path).to_owned(),
                    byte_size: *byte_size,
                })
                .collect(),
            torrent_bytes: Vec::new(),
            magnet_review_created: false,
            externally_managed: false,
        }
    }

    #[test]
    fn minerva_ranker_selects_the_best_exact_game_payload_by_default() {
        let offer = offer(&[
            (0, "Docs/readme.txt", 10),
            (7, "NES/Other Game (USA).zip", 20),
            (9, "NES/Faxanadu (USA).zip", 30),
        ]);
        let result = prepare_inspection_result(
            offer.clone(),
            Some("Faxanadu"),
            Some("Nintendo Entertainment System"),
            &crate::settings::AppSettings::default(),
        )
        .unwrap();
        let state = build_review_state(
            &offer,
            result.ranked_candidates,
            false,
            Some("Nintendo Entertainment System"),
        );

        assert_eq!(state.selected_index, 2);
        assert_eq!(state.selected_file_indices, [2]);
        assert_eq!(state.display_order[0], 2);
        assert!(state.selection_summary.contains("selected automatically"));
    }

    #[test]
    fn switch_review_selects_the_xci_instead_of_a_tiny_exact_bin() {
        let offer = offer(&[
            (0, "Super Mario Bros. RPG.bin", 12),
            (
                4,
                "Switch/Super Mario RPG [0100BC0018138000].xci",
                8 * 1024 * 1024 * 1024,
            ),
        ]);
        let result = prepare_inspection_result(
            offer.clone(),
            Some("Super Mario Bros. RPG"),
            Some("Nintendo Switch"),
            &crate::settings::AppSettings::default(),
        )
        .unwrap();
        let state = build_review_state(
            &offer,
            result.ranked_candidates,
            false,
            Some("Nintendo Switch"),
        );

        assert_eq!(state.selected_index, 1);
        assert_eq!(state.selected_file_indices, [1]);
        assert_eq!(state.display_order, [1]);
        assert_eq!(state.reviewable_file_count, 1);
    }

    #[test]
    fn live_mig_shape_selects_the_game_image_and_not_card_metadata() {
        let root = "MIG Switch Game Collection/Super Mario RPG Legend of the Seven Stars.xci";
        let offer = offer(&[
            (
                4400,
                &format!("{root}/Super Mario RPG Legend of the Seven Stars (Card ID Set).bin"),
                12,
            ),
            (
                4401,
                &format!("{root}/Super Mario RPG Legend of the Seven Stars (Card UID).bin"),
                64,
            ),
            (
                4402,
                &format!("{root}/Super Mario RPG Legend of the Seven Stars (Certificate).bin"),
                512,
            ),
            (
                4403,
                &format!("{root}/Super Mario RPG Legend of the Seven Stars (Initial Data).bin"),
                512,
            ),
            (
                4404,
                &format!("{root}/Super Mario RPG Legend of the Seven Stars.xci"),
                7_260_093_952,
            ),
        ]);
        let result = prepare_inspection_result(
            offer.clone(),
            Some("Super Mario Bros RPG"),
            Some("Nintendo - Nintendo Switch"),
            &crate::settings::AppSettings::default(),
        )
        .unwrap();
        let state = build_review_state(
            &offer,
            result.ranked_candidates,
            false,
            Some("Nintendo - Nintendo Switch"),
        );

        assert_eq!(state.selected_index, 4);
        assert_eq!(state.selected_file_indices, [4]);
        assert_eq!(state.display_order, [4]);
        assert_eq!(state.reviewable_file_count, 1);
        assert!(offer.files[state.display_order[0]].path.ends_with(".xci"));
    }

    #[test]
    fn parsed_mig_torrent_exposes_only_the_playable_xci_for_game_review() {
        use lava_torrent::torrent::v1::{File, Torrent};

        let root = "MIG Switch Game Collection/Super Mario RPG Legend of the Seven Stars.xci";
        let torrent = Torrent {
            announce: None,
            announce_list: None,
            length: 0,
            files: Some(vec![
                File {
                    length: 12,
                    path: std::path::PathBuf::from(format!(
                        "{root}/Super Mario RPG Legend of the Seven Stars (Card ID Set).bin"
                    )),
                    extra_fields: None,
                },
                File {
                    length: 64,
                    path: std::path::PathBuf::from(format!(
                        "{root}/Super Mario RPG Legend of the Seven Stars (Card UID).bin"
                    )),
                    extra_fields: None,
                },
                File {
                    length: 512,
                    path: std::path::PathBuf::from(format!(
                        "{root}/Super Mario RPG Legend of the Seven Stars (Certificate).bin"
                    )),
                    extra_fields: None,
                },
                File {
                    length: 512,
                    path: std::path::PathBuf::from(format!(
                        "{root}/Super Mario RPG Legend of the Seven Stars (Initial Data).bin"
                    )),
                    extra_fields: None,
                },
                File {
                    length: 7_260_093_952,
                    path: std::path::PathBuf::from(format!(
                        "{root}/Super Mario RPG Legend of the Seven Stars.xci"
                    )),
                    extra_fields: None,
                },
            ]),
            name: "MIG Switch Game Collection".to_owned(),
            // One synthetic piece is sufficient for this parser/review test;
            // production torrents retain their real piece geometry.
            piece_length: 8_000_000_000,
            // lava_torrent represents valid UTF-8 byte strings as text while
            // parsing. A real SHA-1 piece hash is arbitrary bytes, so keep the
            // fixture non-UTF-8 to exercise the torrent byte-string path.
            pieces: vec![vec![0xff; 20]],
            extra_fields: None,
            extra_info_fields: None,
        };
        let mut bytes = Vec::new();
        torrent.write_into(&mut bytes).unwrap();
        let offer = external_torrent::inspect_torrent_bytes(&bytes, "mig-switch.torrent").unwrap();
        let result = prepare_inspection_result(
            offer.clone(),
            Some("Super Mario Bros RPG"),
            Some("Nintendo Switch"),
            &crate::settings::AppSettings::default(),
        )
        .unwrap();
        let state = build_review_state(
            &offer,
            result.ranked_candidates,
            false,
            Some("Nintendo Switch"),
        );

        assert_eq!(offer.files.len(), 5);
        assert_eq!(state.reviewable_file_count, 1);
        assert_eq!(state.display_order, [4]);
        assert_eq!(state.selected_file_indices, [4]);
        assert!(offer.files[4].path.ends_with(".xci"));
        assert_eq!(offer.files[4].byte_size, 7_260_093_952);
    }

    #[test]
    fn review_boundary_rejects_a_stale_sidecar_ranking_before_default_selection() {
        let offer = offer(&[
            (4400, "Super Mario RPG (Card ID Set).bin", 12),
            (4404, "Super Mario RPG.xci", 7_260_093_952),
        ]);
        let stale_rank = vec![
            TorrentFileCandidate {
                index: 4400,
                filename: offer.files[0].path.clone(),
                byte_size: offer.files[0].byte_size,
                match_score: 1.0,
                matched_title: "Super Mario RPG".to_owned(),
                region: String::new(),
                version: String::new(),
                download_plan: None,
            },
            TorrentFileCandidate {
                index: 4404,
                filename: offer.files[1].path.clone(),
                byte_size: offer.files[1].byte_size,
                match_score: 0.9,
                matched_title: "Super Mario RPG".to_owned(),
                region: String::new(),
                version: String::new(),
                download_plan: None,
            },
        ];

        let state = build_review_state(&offer, stale_rank, false, Some("Nintendo Switch"));

        assert_eq!(state.selected_index, 1);
        assert_eq!(state.selected_file_indices, [1]);
        assert_eq!(state.display_order[0], 1);
        assert_eq!(state.ranked_file_indices, [1]);
    }

    #[test]
    fn weak_title_similarity_never_guesses_a_default_payload() {
        let offer = offer(&[
            (0, "Docs/readme.txt", 10),
            (1, "Switch/Completely Different Game.nsp", 20),
        ]);
        let result = prepare_inspection_result(
            offer.clone(),
            Some("Super Mario Odyssey"),
            Some("Nintendo Switch"),
            &crate::settings::AppSettings::default(),
        )
        .unwrap();
        let state = build_review_state(
            &offer,
            result.ranked_candidates,
            false,
            Some("Nintendo Switch"),
        );

        assert_eq!(state.selected_index, -1);
        assert!(state.selected_file_indices.is_empty());
        assert!(state.selection_summary.contains("No strong title match"));
    }

    #[test]
    fn multidisc_default_keeps_every_disc_sidecar_and_m3u_plan() {
        let offer = offer(&[
            (0, "Docs/readme.txt", 10),
            (10, "Sony/Final Fantasy VII (USA) (Disc 1).cue", 100),
            (
                11,
                "Sony/Final Fantasy VII (USA) (Disc 1) (Track 01).bin",
                1_000,
            ),
            (12, "Sony/Final Fantasy VII (USA) (Disc 2).cue", 200),
            (
                13,
                "Sony/Final Fantasy VII (USA) (Disc 2) (Track 01).bin",
                2_000,
            ),
        ]);
        let result = prepare_inspection_result(
            offer.clone(),
            Some("Final Fantasy VII"),
            Some("Sony PlayStation"),
            &crate::settings::AppSettings::default(),
        )
        .unwrap();
        let state = build_review_state(
            &offer,
            result.ranked_candidates,
            false,
            Some("Sony PlayStation"),
        );
        let plan = state.selected_download_plan.as_ref().unwrap();

        assert_eq!(state.selected_index, 1);
        assert_eq!(state.selected_file_indices, [1, 2, 3, 4]);
        assert_eq!(plan.kind, "optical_multidisc");
        assert_eq!(plan.playlist_filename, "Final Fantasy VII (USA).m3u");
        assert_eq!(plan.members.len(), 4);
    }

    #[test]
    fn filename_filter_preserves_ranked_display_order_and_source_positions() {
        let order = vec![2, 0, 3, 1];
        let paths = vec![
            "switch/super mario odyssey.nsp".to_owned(),
            "docs/readme.txt".to_owned(),
            "switch/mario kart 8 deluxe.nsp".to_owned(),
            "switch/super mario 3d world.nsp".to_owned(),
        ];
        let terms = vec!["mario".to_owned(), "switch".to_owned()];

        assert_eq!(filter_display_order(&order, &paths, &terms), [2, 0, 3]);
    }

    #[test]
    fn loaded_torrent_search_scales_and_preserves_original_indices() {
        let torrents = (0..20_000)
            .map(|index| crate::qbittorrent::ExistingTorrentSummary {
                info_hash: format!("{index:040x}"),
                name: if index == 17_321 {
                    "Super Mario Odyssey Complete".to_owned()
                } else {
                    format!("Archive {index:05}")
                },
                category: if index == 17_321 || index % 2 == 0 {
                    "switch".to_owned()
                } else {
                    "library".to_owned()
                },
                state: "pausedUP".to_owned(),
                progress: 1.0,
                size: index,
                total_size: index.saturating_mul(2),
                save_path: format!("/srv/torrents/{index:05}"),
            })
            .collect::<Vec<_>>();
        let searchable = torrents
            .iter()
            .map(existing_torrent_search_text)
            .collect::<Vec<_>>();

        assert_eq!(
            filter_existing_torrent_indices(&searchable, "mario odyssey switch"),
            [17_321]
        );
        assert_eq!(
            filter_existing_torrent_indices(&searchable, "archive /srv/torrents/00042 switch",),
            [42]
        );
        assert_eq!(
            filter_existing_torrent_indices(&searchable, &format!("{:040x}", 19_999)),
            [19_999]
        );
        assert_eq!(
            filter_existing_torrent_indices(&searchable, "").len(),
            20_000
        );
    }

    #[test]
    fn loaded_torrent_detail_distinguishes_selected_sidecar_from_total_payload() {
        let torrent = crate::qbittorrent::ExistingTorrentSummary {
            info_hash: "1".repeat(40),
            name: "MIG Switch Game Collection".to_owned(),
            category: "switch".to_owned(),
            state: "pausedDL".to_owned(),
            progress: 0.0,
            size: 12,
            total_size: 7_260_093_952,
            save_path: "/downloads".to_owned(),
        };

        assert_eq!(
            existing_torrent_detail(&torrent),
            "0% · 6.8 GiB total · 12 B currently selected · pausedDL · switch"
        );
    }
}
