#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("QtCore/QAbstractListModel");
        type QAbstractListModel;

        include!("cxx-qt-lib/qbytearray.h");
        type QByteArray = cxx_qt_lib::QByteArray;
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(QString, database_path)]
        #[qproperty(QString, status_message)]
        #[qproperty(QString, current_platform)]
        #[qproperty(QString, availability_filter)]
        #[qproperty(bool, loading)]
        #[qproperty(bool, filtering)]
        #[qproperty(bool, ready)]
        #[qproperty(bool, hide_non_retail)]
        #[qproperty(bool, hide_adult)]
        #[qproperty(QString, artwork_type)]
        #[qproperty(i32, grid_zoom)]
        #[qproperty(QString, media_directory)]
        #[qproperty(bool, media_loading)]
        #[qproperty(bool, media_retrieval_enabled)]
        #[qproperty(QString, media_fetch_message)]
        #[qproperty(QString, favorite_message)]
        #[qproperty(QString, collection_message)]
        #[qproperty(bool, startup_probe)]
        #[qproperty(bool, catalog_probe)]
        #[qproperty(bool, filter_probe)]
        #[qproperty(bool, media_probe)]
        #[qproperty(bool, media_fetch_probe)]
        #[qproperty(bool, favorite_probe)]
        #[qproperty(bool, collection_probe)]
        #[qproperty(bool, collection_busy)]
        #[qproperty(i32, startup_ms)]
        #[qproperty(i32, catalog_ms)]
        #[qproperty(i32, game_count)]
        #[qproperty(i32, filtered_count)]
        #[qproperty(i32, platform_count)]
        #[qproperty(i32, local_file_count)]
        #[qproperty(i32, local_game_count)]
        #[qproperty(i32, offer_count)]
        #[qproperty(i32, downloadable_game_count)]
        #[qproperty(i32, emulator_count)]
        #[qproperty(i32, media_game_count)]
        #[qproperty(i32, media_asset_count)]
        #[qproperty(i32, media_pending_count)]
        #[qproperty(i32, media_downloaded_count)]
        #[qproperty(i32, media_missing_count)]
        #[qproperty(i32, media_error_count)]
        #[qproperty(i32, favorite_count)]
        #[qproperty(i32, recent_count)]
        #[qproperty(i32, favorite_pending_count)]
        #[qproperty(i32, favorite_revision)]
        #[qproperty(i32, collection_count)]
        #[qproperty(i32, collection_revision)]
        #[qproperty(i32, activity_revision)]
        #[qproperty(i32, media_revision)]
        #[qproperty(i32, platform_revision)]
        type LibraryModel = super::LibraryModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn reload(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn refresh_media(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn refresh_activity(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn apply_filter(
            self: Pin<&mut LibraryModel>,
            search: QString,
            platform: QString,
            availability: QString,
        );

        #[qinvokable]
        fn set_content_filters(
            self: Pin<&mut LibraryModel>,
            hide_non_retail: bool,
            hide_adult: bool,
        );

        #[qinvokable]
        fn set_presentation_preferences(
            self: Pin<&mut LibraryModel>,
            artwork_type: QString,
            grid_zoom: i32,
        );

        #[qinvokable]
        fn artwork_url(self: &LibraryModel, launchbox_db_id: i32, artwork_type: QString) -> QUrl;

        #[qinvokable]
        fn exact_artwork_url(
            self: &LibraryModel,
            launchbox_db_id: i32,
            artwork_type: QString,
        ) -> QUrl;

        #[qinvokable]
        fn artwork_source(
            self: &LibraryModel,
            launchbox_db_id: i32,
            artwork_type: QString,
        ) -> QString;

        #[qinvokable]
        fn artwork_candidate_count(
            self: &LibraryModel,
            launchbox_db_id: i32,
            artwork_type: QString,
        ) -> i32;

        #[qinvokable]
        fn artwork_candidate_url(
            self: &LibraryModel,
            launchbox_db_id: i32,
            artwork_type: QString,
            index: i32,
        ) -> QUrl;

        #[qinvokable]
        fn artwork_candidate_source(
            self: &LibraryModel,
            launchbox_db_id: i32,
            artwork_type: QString,
            index: i32,
        ) -> QString;

        #[qinvokable]
        fn request_artwork(
            self: Pin<&mut LibraryModel>,
            launchbox_db_id: i32,
            title: QString,
            platform: QString,
            artwork_type: QString,
        );

        #[qinvokable]
        fn redownload_artwork(
            self: Pin<&mut LibraryModel>,
            launchbox_db_id: i32,
            title: QString,
            platform: QString,
            artwork_type: QString,
        );

        #[qinvokable]
        fn is_favorite(self: &LibraryModel, game_uid: QString) -> bool;

        #[qinvokable]
        fn favorite_pending(self: &LibraryModel, game_uid: QString) -> bool;

        #[qinvokable]
        fn set_favorite(self: Pin<&mut LibraryModel>, game_uid: QString, favorite: bool);

        #[qinvokable]
        fn collection_id_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn collection_name_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn collection_description_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn collection_game_count_at(self: &LibraryModel, index: i32) -> i32;

        #[qinvokable]
        fn collection_exists(self: &LibraryModel, collection_id: QString) -> bool;

        #[qinvokable]
        fn collection_contains(
            self: &LibraryModel,
            collection_id: QString,
            game_uid: QString,
        ) -> bool;

        #[qinvokable]
        fn create_collection(self: Pin<&mut LibraryModel>, name: QString, description: QString);

        #[qinvokable]
        fn update_collection(
            self: Pin<&mut LibraryModel>,
            collection_id: QString,
            name: QString,
            description: QString,
        );

        #[qinvokable]
        fn delete_collection(self: Pin<&mut LibraryModel>, collection_id: QString);

        #[qinvokable]
        fn set_collection_membership(
            self: Pin<&mut LibraryModel>,
            collection_id: QString,
            game_uid: QString,
            member: bool,
        );

        #[qinvokable]
        fn platform_name_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn platform_game_count_at(self: &LibraryModel, index: i32) -> i32;

        #[qinvokable]
        fn shell_ready(self: Pin<&mut LibraryModel>);
    }

    unsafe extern "RustQt" {
        #[qinvokable]
        #[cxx_name = "rowCount"]
        #[cxx_override]
        fn row_count(self: &LibraryModel, parent: &QModelIndex) -> i32;

        #[qinvokable]
        #[cxx_override]
        fn data(self: &LibraryModel, index: &QModelIndex, role: i32) -> QVariant;

        #[qinvokable]
        #[cxx_name = "roleNames"]
        #[cxx_override]
        fn role_names(self: &LibraryModel) -> QHash_i32_QByteArray;

        #[cxx_name = "beginResetModel"]
        #[inherit]
        fn begin_reset_model(self: Pin<&mut LibraryModel>);

        #[cxx_name = "endResetModel"]
        #[inherit]
        fn end_reset_model(self: Pin<&mut LibraryModel>);
    }

    impl cxx_qt::Threading for LibraryModel {}
}

use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QModelIndex, QString, QUrl, QVariant};

use crate::catalog::{self, Catalog, Filter};
use crate::media::{
    ArtworkKind, MediaAsset, MediaFetchOutcome, MediaFetchQueue, MediaFetchRequest, MediaIndex,
};
use crate::settings::{
    LibraryPreferences, PlayActivity, SettingsStore, UserCollection, UserCollections,
};

type RoleNames = QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
type CatalogLoadResult = Result<
    (
        Catalog,
        Result<LibraryPreferences, String>,
        Result<HashSet<String>, String>,
        Result<UserCollections, String>,
        Result<Vec<PlayActivity>, String>,
    ),
    String,
>;

enum CollectionMutation {
    Created(UserCollection),
    Updated {
        collection_id: String,
        name: String,
        description: String,
    },
    Deleted {
        collection_id: String,
    },
    Membership(CollectionMembership),
}

#[derive(Clone)]
struct CollectionMembership {
    collection_id: String,
    game_uid: String,
    member: bool,
}

const DISPLAY_ROLE: i32 = 0;
const USER_ROLE: i32 = 0x0100;
const GAME_ID_ROLE: i32 = USER_ROLE + 1;
const GAME_TITLE_ROLE: i32 = USER_ROLE + 2;
const GAME_PLATFORM_ROLE: i32 = USER_ROLE + 3;
const GAME_STATUS_ROLE: i32 = USER_ROLE + 4;
const GAME_LOCAL_ROLE: i32 = USER_ROLE + 5;
const GAME_DOWNLOADABLE_ROLE: i32 = USER_ROLE + 6;
const GAME_DATABASE_ID_ROLE: i32 = USER_ROLE + 7;

const ROLES: [(i32, &str); 7] = [
    (GAME_ID_ROLE, "gameId"),
    (GAME_TITLE_ROLE, "gameTitle"),
    (GAME_PLATFORM_ROLE, "gamePlatform"),
    (GAME_STATUS_ROLE, "gameStatus"),
    (GAME_LOCAL_ROLE, "gameLocal"),
    (GAME_DOWNLOADABLE_ROLE, "gameDownloadable"),
    (GAME_DATABASE_ID_ROLE, "gameDatabaseId"),
];

pub struct LibraryModelRust {
    database_path: QString,
    status_message: QString,
    current_platform: QString,
    availability_filter: QString,
    loading: bool,
    filtering: bool,
    ready: bool,
    hide_non_retail: bool,
    hide_adult: bool,
    artwork_type: QString,
    grid_zoom: i32,
    media_directory: QString,
    media_loading: bool,
    media_retrieval_enabled: bool,
    media_fetch_message: QString,
    favorite_message: QString,
    collection_message: QString,
    startup_probe: bool,
    catalog_probe: bool,
    filter_probe: bool,
    media_probe: bool,
    media_fetch_probe: bool,
    favorite_probe: bool,
    collection_probe: bool,
    collection_busy: bool,
    startup_ms: i32,
    catalog_ms: i32,
    game_count: i32,
    filtered_count: i32,
    platform_count: i32,
    local_file_count: i32,
    local_game_count: i32,
    offer_count: i32,
    downloadable_game_count: i32,
    emulator_count: i32,
    media_game_count: i32,
    media_asset_count: i32,
    media_pending_count: i32,
    media_downloaded_count: i32,
    media_missing_count: i32,
    media_error_count: i32,
    favorite_count: i32,
    recent_count: i32,
    favorite_pending_count: i32,
    favorite_revision: i32,
    collection_count: i32,
    collection_revision: i32,
    activity_revision: i32,
    media_revision: i32,
    platform_revision: i32,
    catalog: Arc<Catalog>,
    media: Arc<MediaIndex>,
    artwork_kind: ArtworkKind,
    filtered_indices: Vec<usize>,
    load_generation: u64,
    filter_generation: u64,
    reload_pending: bool,
    load_started: Option<std::time::Instant>,
    filter_started: Option<std::time::Instant>,
    media_generation: u64,
    media_started: Option<std::time::Instant>,
    media_fetch_started: Option<std::time::Instant>,
    media_fetch_queue: Option<MediaFetchQueue>,
    media_requests: HashSet<(i64, ArtworkKind)>,
    favorite_game_ids: Arc<HashSet<String>>,
    favorite_requests: HashSet<String>,
    favorite_started: Option<std::time::Instant>,
    recent_game_order: Arc<HashMap<String, i64>>,
    activity_generation: u64,
    collections: Arc<Vec<UserCollection>>,
    collection_members: Arc<HashMap<String, Arc<HashSet<String>>>>,
    collection_started: Option<std::time::Instant>,
}

impl Default for LibraryModelRust {
    fn default() -> Self {
        let preferences = LibraryPreferences::default();
        Self {
            database_path: QString::default(),
            status_message: QString::from("Starting instantly; catalog loading is deferred."),
            current_platform: QString::default(),
            availability_filter: QString::default(),
            loading: false,
            filtering: false,
            ready: false,
            hide_non_retail: preferences.hide_non_retail,
            hide_adult: preferences.hide_adult,
            artwork_type: qstring(&preferences.artwork_type),
            grid_zoom: preferences.grid_zoom,
            media_directory: QString::default(),
            media_loading: false,
            media_retrieval_enabled: crate::media::media_retrieval_enabled(),
            media_fetch_message: QString::from(
                "Artwork is cached on demand as games enter the visible library.",
            ),
            favorite_message: QString::from("Favorites are stored in your local profile."),
            collection_message: QString::from(
                "Create playlists and collections without changing game identity.",
            ),
            startup_probe: std::env::args().any(|argument| argument == "--startup-probe"),
            catalog_probe: std::env::args().any(|argument| argument == "--catalog-probe"),
            filter_probe: std::env::args().any(|argument| argument == "--filter-probe"),
            media_probe: std::env::args().any(|argument| argument == "--media-probe"),
            media_fetch_probe: std::env::args().any(|argument| argument == "--media-fetch-probe"),
            favorite_probe: std::env::args().any(|argument| argument == "--favorite-probe"),
            collection_probe: std::env::args().any(|argument| argument == "--collection-probe"),
            collection_busy: false,
            startup_ms: 0,
            catalog_ms: 0,
            game_count: 0,
            filtered_count: 0,
            platform_count: 0,
            local_file_count: 0,
            local_game_count: 0,
            offer_count: 0,
            downloadable_game_count: 0,
            emulator_count: 0,
            media_game_count: 0,
            media_asset_count: 0,
            media_pending_count: 0,
            media_downloaded_count: 0,
            media_missing_count: 0,
            media_error_count: 0,
            favorite_count: 0,
            recent_count: 0,
            favorite_pending_count: 0,
            favorite_revision: 0,
            collection_count: 0,
            collection_revision: 0,
            activity_revision: 0,
            media_revision: 0,
            platform_revision: 0,
            catalog: Arc::new(Catalog::default()),
            media: Arc::new(MediaIndex::default()),
            artwork_kind: ArtworkKind::default(),
            filtered_indices: Vec::new(),
            load_generation: 0,
            filter_generation: 0,
            reload_pending: false,
            load_started: None,
            filter_started: None,
            media_generation: 0,
            media_started: None,
            media_fetch_started: None,
            media_fetch_queue: None,
            media_requests: HashSet::new(),
            favorite_game_ids: Arc::new(HashSet::new()),
            favorite_requests: HashSet::new(),
            favorite_started: None,
            recent_game_order: Arc::new(HashMap::new()),
            activity_generation: 0,
            collections: Arc::new(Vec::new()),
            collection_members: Arc::new(HashMap::new()),
            collection_started: None,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn collection_at(model: &qobject::LibraryModel, index: i32) -> Option<&UserCollection> {
    usize::try_from(index)
        .ok()
        .and_then(|index| model.rust().collections.get(index))
}

fn sort_collections(collections: &mut [UserCollection]) {
    collections.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

impl qobject::LibraryModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().loading() || *self.as_ref().ready() {
            return;
        }
        self.as_mut().start_load();
    }

    pub fn reload(mut self: Pin<&mut Self>) {
        if *self.as_ref().loading()
            || !self.as_ref().rust().favorite_requests.is_empty()
            || *self.as_ref().collection_busy()
        {
            self.as_mut().rust_mut().reload_pending = true;
            if !*self.as_ref().loading() {
                self.as_mut().set_status_message(qstring(
                    "Finishing profile changes before refreshing the catalog…",
                ));
            }
            return;
        }
        self.as_mut().start_load();
    }

    pub fn refresh_media(mut self: Pin<&mut Self>) {
        if !*self.as_ref().ready() {
            return;
        }
        self.as_mut().start_media_load();
    }

    pub fn refresh_activity(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().activity_generation =
            self.as_ref().rust().activity_generation.wrapping_add(1);
        let generation = self.as_ref().rust().activity_generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-activity-load".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| store.recent_play_activity(500))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_activity_refresh(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_status_message(qstring(format!(
                "Could not start play-activity refresh: {error}"
            )));
        }
    }

    fn finish_activity_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<Vec<PlayActivity>, String>,
    ) {
        if generation != self.as_ref().rust().activity_generation {
            return;
        }
        match result {
            Ok(activity) => {
                let recent_game_order = activity
                    .into_iter()
                    .map(|activity| (activity.game_uid, activity.last_played_at))
                    .collect::<HashMap<_, _>>();
                let recent_count = self
                    .as_ref()
                    .rust()
                    .catalog
                    .games
                    .iter()
                    .filter(|game| recent_game_order.contains_key(&game.id))
                    .count();
                self.as_mut().rust_mut().recent_game_order = Arc::new(recent_game_order);
                self.as_mut().set_recent_count(saturating_i32(recent_count));
                let revision = self.as_ref().activity_revision().wrapping_add(1);
                self.as_mut().set_activity_revision(revision);
            }
            Err(error) => self
                .as_mut()
                .set_status_message(qstring(format!("Could not refresh play activity: {error}"))),
        }
    }

    fn start_load(mut self: Pin<&mut Self>) {
        let Some(path) = catalog::requested_database_path() else {
            self.as_mut().set_ready(false);
            self.as_mut().set_status_message(qstring(
                "No database found. Pass --database PATH or set LUNCHBOX_DATABASE.",
            ));
            return;
        };

        self.as_mut().rust_mut().load_generation =
            self.as_ref().rust().load_generation.wrapping_add(1);
        self.as_mut().rust_mut().filter_generation =
            self.as_ref().rust().filter_generation.wrapping_add(1);
        self.as_mut().rust_mut().load_started = Some(std::time::Instant::now());
        let generation = self.as_ref().rust().load_generation;
        self.as_mut()
            .set_database_path(qstring(path.to_string_lossy()));
        self.as_mut().set_ready(false);
        self.as_mut().set_filtering(false);
        self.as_mut().set_loading(true);
        self.as_mut()
            .set_status_message(qstring("Loading the catalog…"));

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-catalog-load".into())
            .spawn(move || {
                let loaded = catalog::load(&path)
                    .map(|catalog| {
                        let (preferences, favorites, collections, activity) =
                            match SettingsStore::open_default() {
                                Ok(store) => (
                                    store
                                        .load_library_preferences()
                                        .map_err(|error| error.to_string()),
                                    store.favorite_game_ids().map_err(|error| error.to_string()),
                                    store.user_collections().map_err(|error| error.to_string()),
                                    store
                                        .recent_play_activity(500)
                                        .map_err(|error| error.to_string()),
                                ),
                                Err(error) => {
                                    let error = error.to_string();
                                    (
                                        Err(error.clone()),
                                        Err(error.clone()),
                                        Err(error.clone()),
                                        Err(error),
                                    )
                                }
                            };
                        (catalog, preferences, favorites, collections, activity)
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_load(generation, loaded);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_loading(false);
            self.as_mut()
                .set_status_message(qstring(format!("Could not start catalog loader: {error}")));
        }
    }

    fn finish_load(mut self: Pin<&mut Self>, generation: u64, loaded: CatalogLoadResult) {
        if generation != self.as_ref().rust().load_generation {
            return;
        }
        self.as_mut().set_loading(false);
        match loaded {
            Ok((catalog, preferences, favorites, collections, activity)) => {
                let preference_warning = match preferences {
                    Ok(preferences) => {
                        let artwork_kind =
                            ArtworkKind::parse(&preferences.artwork_type).unwrap_or_default();
                        self.as_mut()
                            .set_hide_non_retail(preferences.hide_non_retail);
                        self.as_mut().set_hide_adult(preferences.hide_adult);
                        self.as_mut()
                            .set_artwork_type(qstring(&preferences.artwork_type));
                        self.as_mut().set_grid_zoom(preferences.grid_zoom);
                        self.as_mut().rust_mut().artwork_kind = artwork_kind;
                        None
                    }
                    Err(error) => Some(error),
                };
                let (favorite_game_ids, favorite_warning) = match favorites {
                    Ok(favorites) => (favorites, None),
                    Err(error) => (HashSet::new(), Some(error)),
                };
                let favorite_count = catalog
                    .games
                    .iter()
                    .filter(|game| favorite_game_ids.contains(&game.id))
                    .count();
                let (collections, collection_members, collection_warning) = match collections {
                    Ok(collections) => {
                        let members = collections
                            .members
                            .into_iter()
                            .map(|(id, games)| (id, Arc::new(games)))
                            .collect::<HashMap<_, _>>();
                        (collections.collections, members, None)
                    }
                    Err(error) => (Vec::new(), HashMap::new(), Some(error)),
                };
                let collection_count = collections.len();
                let (recent_game_order, activity_warning) = match activity {
                    Ok(activity) => (
                        activity
                            .into_iter()
                            .map(|activity| (activity.game_uid, activity.last_played_at))
                            .collect::<HashMap<_, _>>(),
                        None,
                    ),
                    Err(error) => (HashMap::new(), Some(error)),
                };
                let recent_count = catalog
                    .games
                    .iter()
                    .filter(|game| recent_game_order.contains_key(&game.id))
                    .count();
                let indices = (0..catalog.games.len()).collect::<Vec<_>>();
                self.as_mut().begin_reset_model();
                {
                    let mut this = self.as_mut();
                    let mut rust = this.as_mut().rust_mut();
                    rust.catalog = Arc::new(catalog);
                    rust.filtered_indices = indices;
                    rust.favorite_game_ids = Arc::new(favorite_game_ids);
                    rust.recent_game_order = Arc::new(recent_game_order);
                    rust.collections = Arc::new(collections);
                    rust.collection_members = Arc::new(collection_members);
                }
                self.as_mut().end_reset_model();

                let game_count = self.as_ref().rust().catalog.games.len();
                let platform_count = self.as_ref().rust().catalog.platforms.len();
                let local_file_count = self.as_ref().rust().catalog.local_file_count;
                let local_game_count = self
                    .as_ref()
                    .rust()
                    .catalog
                    .games
                    .iter()
                    .filter(|game| game.local)
                    .count();
                let offer_count = self.as_ref().rust().catalog.offer_count;
                let downloadable_game_count = self
                    .as_ref()
                    .rust()
                    .catalog
                    .games
                    .iter()
                    .filter(|game| game.downloadable && !game.local)
                    .count();
                let emulator_count = self.as_ref().rust().catalog.emulator_count;
                let source_label = self.as_ref().rust().catalog.source_label.clone();
                let catalog_ms = self
                    .as_ref()
                    .rust()
                    .load_started
                    .map(|started| i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX))
                    .unwrap_or_default();
                self.as_mut().set_game_count(saturating_i32(game_count));
                self.as_mut().set_filtered_count(saturating_i32(game_count));
                self.as_mut()
                    .set_platform_count(saturating_i32(platform_count));
                self.as_mut()
                    .set_local_file_count(saturating_i32(local_file_count));
                self.as_mut()
                    .set_local_game_count(saturating_i32(local_game_count));
                self.as_mut().set_offer_count(saturating_i32(offer_count));
                self.as_mut()
                    .set_downloadable_game_count(saturating_i32(downloadable_game_count));
                self.as_mut()
                    .set_emulator_count(saturating_i32(emulator_count));
                self.as_mut()
                    .set_favorite_count(saturating_i32(favorite_count));
                self.as_mut().set_recent_count(saturating_i32(recent_count));
                let favorite_revision = self.as_ref().favorite_revision().wrapping_add(1);
                self.as_mut().set_favorite_revision(favorite_revision);
                if let Some(warning) = &favorite_warning {
                    self.as_mut().set_favorite_message(qstring(format!(
                        "Favorites could not be loaded: {warning}"
                    )));
                } else {
                    self.as_mut().set_favorite_message(qstring(format!(
                        "{favorite_count} favorite games in this catalog."
                    )));
                }
                self.as_mut()
                    .set_collection_count(saturating_i32(collection_count));
                let collection_revision = self.as_ref().collection_revision().wrapping_add(1);
                self.as_mut().set_collection_revision(collection_revision);
                if let Some(warning) = &collection_warning {
                    self.as_mut().set_collection_message(qstring(format!(
                        "Collections could not be loaded: {warning}"
                    )));
                } else {
                    self.as_mut().set_collection_message(qstring(format!(
                        "{collection_count} named collections in this profile."
                    )));
                }
                if *self.as_ref().collection_probe() {
                    let membership_count = self
                        .as_ref()
                        .rust()
                        .collection_members
                        .values()
                        .map(|members| members.len())
                        .sum::<usize>();
                    println!(
                        "LUNCHBOX_COLLECTIONS_LOADED collections={collection_count} memberships={membership_count}"
                    );
                }
                self.as_mut().set_catalog_ms(catalog_ms);
                println!(
                    "LUNCHBOX_CATALOG_READY_MS={catalog_ms} games={game_count} platforms={platform_count} local_files={local_file_count} downloadable_games={downloadable_game_count} offers={offer_count} emulators={emulator_count} source={source_label:?}"
                );
                self.as_mut().set_ready(true);
                let revision = self.as_ref().platform_revision().wrapping_add(1);
                self.as_mut().set_platform_revision(revision);
                let mut status = format!(
                    "Ready — {game_count} games across {platform_count} platforms — {source_label}"
                );
                if let Some(warning) = preference_warning {
                    status.push_str(&format!(" — filter preferences unavailable: {warning}"));
                }
                if let Some(warning) = favorite_warning {
                    status.push_str(&format!(" — favorites unavailable: {warning}"));
                }
                if let Some(warning) = collection_warning {
                    status.push_str(&format!(" — collections unavailable: {warning}"));
                }
                if let Some(warning) = activity_warning {
                    status.push_str(&format!(" — play activity unavailable: {warning}"));
                }
                self.as_mut().set_status_message(qstring(status));
                self.as_mut().start_media_load();
                if self.as_ref().rust().reload_pending {
                    self.as_mut().rust_mut().reload_pending = false;
                    self.as_mut().start_load();
                }
            }
            Err(error) => {
                self.as_mut().set_ready(false);
                self.as_mut()
                    .set_status_message(qstring(format!("Could not load the catalog: {error}")));
            }
        }
    }

    pub fn apply_filter(
        mut self: Pin<&mut Self>,
        search: QString,
        platform: QString,
        availability: QString,
    ) {
        if !*self.as_ref().ready() || *self.as_ref().loading() {
            return;
        }
        let availability_text = availability.to_string();
        let collection_game_ids = availability_text
            .strip_prefix("collection:")
            .and_then(|collection_id| {
                self.as_ref()
                    .rust()
                    .collection_members
                    .get(collection_id)
                    .cloned()
            })
            .unwrap_or_else(|| Arc::new(HashSet::new()));
        let filter = Filter {
            search: search.to_string(),
            platform: platform.to_string(),
            availability: availability_text,
            hide_non_retail: *self.as_ref().hide_non_retail(),
            hide_adult: *self.as_ref().hide_adult(),
            favorite_game_ids: Arc::clone(&self.as_ref().rust().favorite_game_ids),
            collection_game_ids,
            recent_game_order: Arc::clone(&self.as_ref().rust().recent_game_order),
        };
        self.as_mut().set_current_platform(platform);
        self.as_mut().set_availability_filter(availability);
        self.as_mut().rust_mut().filter_generation =
            self.as_ref().rust().filter_generation.wrapping_add(1);
        self.as_mut().rust_mut().filter_started = Some(std::time::Instant::now());
        let generation = self.as_ref().rust().filter_generation;
        let catalog = Arc::clone(&self.as_ref().rust().catalog);
        self.as_mut().set_filtering(true);

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-catalog-filter".into())
            .spawn(move || {
                let indices = catalog::filter_indices(&catalog, &filter);
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_filter(generation, indices);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_filtering(false);
            self.as_mut()
                .set_status_message(qstring(format!("Could not start catalog filter: {error}")));
        }
    }

    pub fn set_content_filters(mut self: Pin<&mut Self>, hide_non_retail: bool, hide_adult: bool) {
        if hide_non_retail == *self.as_ref().hide_non_retail()
            && hide_adult == *self.as_ref().hide_adult()
        {
            return;
        }
        self.as_mut().set_hide_non_retail(hide_non_retail);
        self.as_mut().set_hide_adult(hide_adult);
        let preferences = LibraryPreferences {
            hide_non_retail,
            hide_adult,
            artwork_type: self.as_ref().artwork_type().to_string(),
            grid_zoom: *self.as_ref().grid_zoom(),
        };
        if let Err(error) = SettingsStore::open_default()
            .and_then(|store| store.save_library_preferences(&preferences))
        {
            self.as_mut().set_status_message(qstring(format!(
                "Filters applied, but the preference could not be saved: {error}"
            )));
        }
    }

    pub fn set_presentation_preferences(
        mut self: Pin<&mut Self>,
        artwork_type: QString,
        grid_zoom: i32,
    ) {
        let artwork_type = artwork_type.to_string();
        let Some(artwork_kind) = ArtworkKind::parse(&artwork_type) else {
            self.as_mut().set_status_message(qstring(format!(
                "Could not apply view preferences: unsupported artwork type {artwork_type}"
            )));
            return;
        };
        if !(50..=200).contains(&grid_zoom) {
            self.as_mut().set_status_message(qstring(
                "Could not apply view preferences: grid zoom must be between 50 and 200 percent",
            ));
            return;
        }

        self.as_mut().rust_mut().artwork_kind = artwork_kind;
        self.as_mut().set_artwork_type(qstring(&artwork_type));
        self.as_mut().set_grid_zoom(grid_zoom);
        let preferences = LibraryPreferences {
            hide_non_retail: *self.as_ref().hide_non_retail(),
            hide_adult: *self.as_ref().hide_adult(),
            artwork_type,
            grid_zoom,
        };
        if let Err(error) = SettingsStore::open_default()
            .and_then(|store| store.save_library_preferences(&preferences))
        {
            self.as_mut().set_status_message(qstring(format!(
                "View updated, but the preference could not be saved: {error}"
            )));
        }
    }

    pub fn artwork_url(&self, launchbox_db_id: i32, artwork_type: QString) -> QUrl {
        self.media_asset(launchbox_db_id, &artwork_type.to_string(), false)
            .map(media_asset_url)
            .unwrap_or_default()
    }

    pub fn exact_artwork_url(&self, launchbox_db_id: i32, artwork_type: QString) -> QUrl {
        self.media_asset(launchbox_db_id, &artwork_type.to_string(), true)
            .map(media_asset_url)
            .unwrap_or_default()
    }

    pub fn artwork_source(&self, launchbox_db_id: i32, artwork_type: QString) -> QString {
        self.media_asset(launchbox_db_id, &artwork_type.to_string(), false)
            .map(|asset| qstring(&asset.source))
            .unwrap_or_default()
    }

    pub fn artwork_candidate_count(&self, launchbox_db_id: i32, artwork_type: QString) -> i32 {
        let kind =
            ArtworkKind::parse(&artwork_type.to_string()).unwrap_or(self.rust().artwork_kind);
        saturating_i32(
            self.rust()
                .media
                .candidate_count(i64::from(launchbox_db_id), kind),
        )
    }

    pub fn artwork_candidate_url(
        &self,
        launchbox_db_id: i32,
        artwork_type: QString,
        index: i32,
    ) -> QUrl {
        self.media_candidate(launchbox_db_id, &artwork_type.to_string(), index)
            .map(media_asset_url)
            .unwrap_or_default()
    }

    pub fn artwork_candidate_source(
        &self,
        launchbox_db_id: i32,
        artwork_type: QString,
        index: i32,
    ) -> QString {
        self.media_candidate(launchbox_db_id, &artwork_type.to_string(), index)
            .map(|asset| qstring(&asset.source))
            .unwrap_or_default()
    }

    pub fn request_artwork(
        mut self: Pin<&mut Self>,
        launchbox_db_id: i32,
        title: QString,
        platform: QString,
        artwork_type: QString,
    ) {
        self.as_mut()
            .queue_artwork_request(launchbox_db_id, title, platform, artwork_type, false);
    }

    pub fn redownload_artwork(
        mut self: Pin<&mut Self>,
        launchbox_db_id: i32,
        title: QString,
        platform: QString,
        artwork_type: QString,
    ) {
        self.as_mut()
            .queue_artwork_request(launchbox_db_id, title, platform, artwork_type, true);
    }

    fn queue_artwork_request(
        mut self: Pin<&mut Self>,
        launchbox_db_id: i32,
        title: QString,
        platform: QString,
        artwork_type: QString,
        force: bool,
    ) {
        if !*self.as_ref().media_retrieval_enabled()
            || *self.as_ref().media_loading()
            || launchbox_db_id <= 0
        {
            return;
        }
        let artwork_type = artwork_type.to_string();
        let Some(kind) = ArtworkKind::parse(&artwork_type) else {
            return;
        };
        let title = title.to_string();
        let platform = platform.to_string();
        if title.trim().is_empty() || platform.trim().is_empty() {
            return;
        }
        let database_id = i64::from(launchbox_db_id);
        if !force
            && self
                .as_ref()
                .rust()
                .media
                .selected(database_id, kind)
                .is_some()
        {
            return;
        }
        let key = (database_id, kind);
        if self.as_ref().rust().media_requests.contains(&key) {
            return;
        }
        let request = MediaFetchRequest {
            database_id,
            title,
            platform,
            requested_kind: kind,
            force,
        };
        let queued = self
            .as_ref()
            .rust()
            .media_fetch_queue
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("artwork retrieval is not initialized"))
            .and_then(|queue| queue.try_request(request));
        match queued {
            Ok(()) => {
                if self.as_ref().rust().media_fetch_started.is_none() {
                    self.as_mut().rust_mut().media_fetch_started = Some(std::time::Instant::now());
                }
                self.as_mut().rust_mut().media_requests.insert(key);
                let pending = self.as_ref().rust().media_requests.len();
                self.as_mut()
                    .set_media_pending_count(saturating_i32(pending));
                if force {
                    self.as_mut().set_media_fetch_message(qstring(format!(
                        "Refreshing {} artwork from LibRetro…",
                        kind.key().replace('-', " ")
                    )));
                }
            }
            Err(error) => {
                self.as_mut().set_media_fetch_message(qstring(format!(
                    "Artwork queue unavailable: {error}"
                )));
            }
        }
    }

    pub fn is_favorite(&self, game_uid: QString) -> bool {
        self.rust()
            .favorite_game_ids
            .contains(&game_uid.to_string())
    }

    pub fn favorite_pending(&self, game_uid: QString) -> bool {
        self.rust()
            .favorite_requests
            .contains(&game_uid.to_string())
    }

    pub fn set_favorite(mut self: Pin<&mut Self>, game_uid: QString, favorite: bool) {
        if !*self.as_ref().ready() || *self.as_ref().loading() {
            return;
        }
        let game_uid = game_uid.to_string();
        let Some(game) = self
            .as_ref()
            .rust()
            .catalog
            .games
            .iter()
            .find(|game| game.id == game_uid)
            .cloned()
        else {
            self.as_mut().set_favorite_message(qstring(
                "Favorite was not changed because the game identity is not in this catalog.",
            ));
            return;
        };
        if self.as_ref().rust().favorite_requests.contains(&game.id)
            || self.as_ref().rust().favorite_game_ids.contains(&game.id) == favorite
        {
            return;
        }

        self.as_mut().rust_mut().favorite_started = Some(std::time::Instant::now());
        self.as_mut()
            .rust_mut()
            .favorite_requests
            .insert(game.id.clone());
        self.as_mut().apply_favorite_state(&game.id, favorite);
        let pending = self.as_ref().rust().favorite_requests.len();
        self.as_mut()
            .set_favorite_pending_count(saturating_i32(pending));
        self.as_mut().set_favorite_message(qstring(format!(
            "Saving {} as {}…",
            game.title,
            if favorite {
                "a favorite"
            } else {
                "not a favorite"
            }
        )));

        let game_uid = game.id.clone();
        let rollback_uid = game_uid.clone();
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-favorite-save".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| {
                        store.set_favorite(
                            &game.id,
                            game.launchbox_db_id,
                            &game.title,
                            &game.platform,
                            favorite,
                        )
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_favorite_save(game_uid, favorite, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().finish_favorite_save(
                rollback_uid,
                favorite,
                Err(format!("could not start favorite save: {error}")),
            );
        }
    }

    fn apply_favorite_state(mut self: Pin<&mut Self>, game_uid: &str, favorite: bool) {
        let changed = if favorite {
            Arc::make_mut(&mut self.as_mut().rust_mut().favorite_game_ids)
                .insert(game_uid.to_owned())
        } else {
            Arc::make_mut(&mut self.as_mut().rust_mut().favorite_game_ids).remove(game_uid)
        };
        if !changed {
            return;
        }
        let count = if favorite {
            self.as_ref().favorite_count().saturating_add(1)
        } else {
            self.as_ref().favorite_count().saturating_sub(1)
        };
        self.as_mut().set_favorite_count(count);
        let revision = self.as_ref().favorite_revision().wrapping_add(1);
        self.as_mut().set_favorite_revision(revision);
    }

    fn finish_favorite_save(
        mut self: Pin<&mut Self>,
        game_uid: String,
        favorite: bool,
        result: Result<(), String>,
    ) {
        self.as_mut().rust_mut().favorite_requests.remove(&game_uid);
        let pending = self.as_ref().rust().favorite_requests.len();
        self.as_mut()
            .set_favorite_pending_count(saturating_i32(pending));
        match result {
            Ok(()) => {
                self.as_mut().set_favorite_message(qstring(if favorite {
                    "Added to Favorites."
                } else {
                    "Removed from Favorites."
                }));
                if *self.as_ref().favorite_probe() {
                    let elapsed = self
                        .as_ref()
                        .rust()
                        .favorite_started
                        .map(|started| started.elapsed().as_millis())
                        .unwrap_or_default();
                    println!(
                        "LUNCHBOX_FAVORITE_READY_MS={elapsed} game_uid={game_uid:?} favorite={favorite}"
                    );
                }
            }
            Err(error) => {
                self.as_mut().apply_favorite_state(&game_uid, !favorite);
                self.as_mut().set_favorite_message(qstring(format!(
                    "Favorite change was rolled back: {error}"
                )));
                self.as_mut().set_status_message(qstring(format!(
                    "Could not save the favorite change: {error}"
                )));
            }
        }
        if pending == 0
            && self.as_ref().rust().reload_pending
            && !*self.as_ref().loading()
            && !*self.as_ref().collection_busy()
        {
            self.as_mut().rust_mut().reload_pending = false;
            self.as_mut().start_load();
        }
    }

    pub fn collection_id_at(&self, index: i32) -> QString {
        collection_at(self, index)
            .map(|collection| qstring(&collection.id))
            .unwrap_or_default()
    }

    pub fn collection_name_at(&self, index: i32) -> QString {
        collection_at(self, index)
            .map(|collection| qstring(&collection.name))
            .unwrap_or_default()
    }

    pub fn collection_description_at(&self, index: i32) -> QString {
        collection_at(self, index)
            .map(|collection| qstring(&collection.description))
            .unwrap_or_default()
    }

    pub fn collection_game_count_at(&self, index: i32) -> i32 {
        collection_at(self, index)
            .map(|collection| saturating_i32(collection.game_count))
            .unwrap_or_default()
    }

    pub fn collection_exists(&self, collection_id: QString) -> bool {
        let collection_id = collection_id.to_string();
        self.rust()
            .collections
            .iter()
            .any(|collection| collection.id == collection_id)
    }

    pub fn collection_contains(&self, collection_id: QString, game_uid: QString) -> bool {
        self.rust()
            .collection_members
            .get(&collection_id.to_string())
            .is_some_and(|members| members.contains(&game_uid.to_string()))
    }

    pub fn create_collection(mut self: Pin<&mut Self>, name: QString, description: QString) {
        if !self.as_ref().can_change_collections() {
            return;
        }
        let name = name.to_string();
        let description = description.to_string();
        self.as_mut()
            .start_collection_task("lunchbox-collection-create", None, move || {
                SettingsStore::open_default()
                    .and_then(|store| store.create_collection(&name, &description))
                    .map(CollectionMutation::Created)
                    .map_err(|error| error.to_string())
            });
    }

    pub fn update_collection(
        mut self: Pin<&mut Self>,
        collection_id: QString,
        name: QString,
        description: QString,
    ) {
        if !self.as_ref().can_change_collections() {
            return;
        }
        let collection_id = collection_id.to_string();
        if !self
            .as_ref()
            .rust()
            .collections
            .iter()
            .any(|collection| collection.id == collection_id)
        {
            self.as_mut()
                .set_collection_message(qstring("Collection no longer exists."));
            return;
        }
        let name = name.to_string();
        let description = description.to_string();
        let task_collection_id = collection_id.clone();
        self.as_mut()
            .start_collection_task("lunchbox-collection-update", None, move || {
                SettingsStore::open_default()
                    .and_then(|store| {
                        store.update_collection(&task_collection_id, &name, &description)
                    })
                    .map(|()| CollectionMutation::Updated {
                        collection_id: task_collection_id,
                        name: name.trim().to_owned(),
                        description: description.trim().to_owned(),
                    })
                    .map_err(|error| error.to_string())
            });
    }

    pub fn delete_collection(mut self: Pin<&mut Self>, collection_id: QString) {
        if !self.as_ref().can_change_collections() {
            return;
        }
        let collection_id = collection_id.to_string();
        if !self
            .as_ref()
            .rust()
            .collections
            .iter()
            .any(|collection| collection.id == collection_id)
        {
            self.as_mut()
                .set_collection_message(qstring("Collection no longer exists."));
            return;
        }
        let task_collection_id = collection_id.clone();
        self.as_mut()
            .start_collection_task("lunchbox-collection-delete", None, move || {
                SettingsStore::open_default()
                    .and_then(|store| store.delete_collection(&task_collection_id))
                    .map(|()| CollectionMutation::Deleted {
                        collection_id: task_collection_id,
                    })
                    .map_err(|error| error.to_string())
            });
    }

    pub fn set_collection_membership(
        mut self: Pin<&mut Self>,
        collection_id: QString,
        game_uid: QString,
        member: bool,
    ) {
        if !self.as_ref().can_change_collections() {
            return;
        }
        let membership = CollectionMembership {
            collection_id: collection_id.to_string(),
            game_uid: game_uid.to_string(),
            member,
        };
        let collection_exists = self
            .as_ref()
            .rust()
            .collections
            .iter()
            .any(|collection| collection.id == membership.collection_id);
        let game_exists = self
            .as_ref()
            .rust()
            .catalog
            .games
            .iter()
            .any(|game| game.id == membership.game_uid);
        if !collection_exists || !game_exists {
            self.as_mut().set_collection_message(qstring(
                "Membership was not changed because its collection or game identity is unavailable.",
            ));
            return;
        }
        let currently_member = self
            .as_ref()
            .rust()
            .collection_members
            .get(&membership.collection_id)
            .is_some_and(|members| members.contains(&membership.game_uid));
        if currently_member == member {
            return;
        }

        self.as_mut().set_collection_busy(true);
        self.as_mut().apply_collection_membership(&membership);
        let task_membership = membership.clone();
        self.as_mut().start_collection_task(
            "lunchbox-collection-membership",
            Some(membership),
            move || {
                SettingsStore::open_default()
                    .and_then(|store| {
                        store.set_collection_membership(
                            &task_membership.collection_id,
                            &task_membership.game_uid,
                            task_membership.member,
                        )
                    })
                    .map(|()| CollectionMutation::Membership(task_membership))
                    .map_err(|error| error.to_string())
            },
        );
    }

    fn can_change_collections(&self) -> bool {
        *self.ready() && !*self.loading() && !*self.collection_busy()
    }

    fn start_collection_task<F>(
        mut self: Pin<&mut Self>,
        thread_name: &'static str,
        rollback: Option<CollectionMembership>,
        task: F,
    ) where
        F: FnOnce() -> Result<CollectionMutation, String> + Send + 'static,
    {
        self.as_mut().set_collection_busy(true);
        self.as_mut().rust_mut().collection_started = Some(std::time::Instant::now());
        self.as_mut()
            .set_collection_message(qstring("Saving collection changes…"));
        let rollback_for_error = rollback.clone();
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name(thread_name.into())
            .spawn(move || {
                let result = task();
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_collection_task(result, rollback);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().finish_collection_task(
                Err(format!("could not start collection worker: {error}")),
                rollback_for_error,
            );
        }
    }

    fn apply_collection_membership(
        mut self: Pin<&mut Self>,
        membership: &CollectionMembership,
    ) -> bool {
        let changed = {
            let mut this = self.as_mut();
            let mut rust = this.as_mut().rust_mut();
            let members = Arc::make_mut(&mut rust.collection_members)
                .entry(membership.collection_id.clone())
                .or_insert_with(|| Arc::new(HashSet::new()));
            if membership.member {
                Arc::make_mut(members).insert(membership.game_uid.clone())
            } else {
                Arc::make_mut(members).remove(&membership.game_uid)
            }
        };
        if !changed {
            return false;
        }
        if let Some(collection) = Arc::make_mut(&mut self.as_mut().rust_mut().collections)
            .iter_mut()
            .find(|collection| collection.id == membership.collection_id)
        {
            collection.game_count = if membership.member {
                collection.game_count.saturating_add(1)
            } else {
                collection.game_count.saturating_sub(1)
            };
        }
        let revision = self.as_ref().collection_revision().wrapping_add(1);
        self.as_mut().set_collection_revision(revision);
        true
    }

    fn finish_collection_task(
        mut self: Pin<&mut Self>,
        result: Result<CollectionMutation, String>,
        rollback: Option<CollectionMembership>,
    ) {
        self.as_mut().set_collection_busy(false);
        match result {
            Ok(CollectionMutation::Created(collection)) => {
                let name = collection.name.clone();
                let collection_id = collection.id.clone();
                let collection_count = {
                    let mut this = self.as_mut();
                    let mut rust = this.as_mut().rust_mut();
                    {
                        let collections: &mut Vec<UserCollection> =
                            Arc::make_mut(&mut rust.collections);
                        collections.push(collection);
                        sort_collections(collections);
                    }
                    Arc::make_mut(&mut rust.collection_members)
                        .entry(collection_id)
                        .or_insert_with(|| Arc::new(HashSet::new()));
                    rust.collections.len()
                };
                self.as_mut()
                    .set_collection_count(saturating_i32(collection_count));
                self.as_mut()
                    .set_collection_message(qstring(format!("Created {name}.")));
                self.as_mut().bump_collection_revision();
            }
            Ok(CollectionMutation::Updated {
                collection_id,
                name,
                description,
            }) => {
                if let Some(collection) = Arc::make_mut(&mut self.as_mut().rust_mut().collections)
                    .iter_mut()
                    .find(|collection| collection.id == collection_id)
                {
                    collection.name = name.clone();
                    collection.description = description;
                }
                {
                    let mut this = self.as_mut();
                    let mut rust = this.as_mut().rust_mut();
                    let collections: &mut Vec<UserCollection> =
                        Arc::make_mut(&mut rust.collections);
                    sort_collections(collections);
                }
                self.as_mut()
                    .set_collection_message(qstring(format!("Updated {name}.")));
                self.as_mut().bump_collection_revision();
            }
            Ok(CollectionMutation::Deleted { collection_id }) => {
                let collection_count = {
                    let mut this = self.as_mut();
                    let mut rust = this.as_mut().rust_mut();
                    Arc::make_mut(&mut rust.collections)
                        .retain(|collection| collection.id != collection_id);
                    Arc::make_mut(&mut rust.collection_members).remove(&collection_id);
                    rust.collections.len()
                };
                self.as_mut()
                    .set_collection_count(saturating_i32(collection_count));
                self.as_mut()
                    .set_collection_message(qstring("Collection deleted."));
                self.as_mut().bump_collection_revision();
            }
            Ok(CollectionMutation::Membership(membership)) => {
                self.as_mut()
                    .set_collection_message(qstring(if membership.member {
                        "Added game to collection."
                    } else {
                        "Removed game from collection."
                    }));
            }
            Err(error) => {
                let rolled_back = rollback.is_some();
                if let Some(mut rollback) = rollback {
                    rollback.member = !rollback.member;
                    self.as_mut().apply_collection_membership(&rollback);
                }
                self.as_mut()
                    .set_collection_message(qstring(if rolled_back {
                        format!("Collection change was rolled back: {error}")
                    } else {
                        format!("Collection change failed: {error}")
                    }));
                self.as_mut().set_status_message(qstring(format!(
                    "Could not save the collection change: {error}"
                )));
            }
        }
        if *self.as_ref().collection_probe() {
            let elapsed = self
                .as_ref()
                .rust()
                .collection_started
                .map(|started| started.elapsed().as_millis())
                .unwrap_or_default();
            println!(
                "LUNCHBOX_COLLECTION_READY_MS={elapsed} collections={} message={:?}",
                self.as_ref().collection_count(),
                self.as_ref().collection_message().to_string()
            );
        }
        if self.as_ref().rust().reload_pending
            && !*self.as_ref().loading()
            && self.as_ref().rust().favorite_requests.is_empty()
        {
            self.as_mut().rust_mut().reload_pending = false;
            self.as_mut().start_load();
        }
    }

    fn bump_collection_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().collection_revision().wrapping_add(1);
        self.as_mut().set_collection_revision(revision);
    }

    fn media_asset(
        &self,
        launchbox_db_id: i32,
        artwork_type: &str,
        exact: bool,
    ) -> Option<&MediaAsset> {
        let kind = ArtworkKind::parse(artwork_type).unwrap_or(self.rust().artwork_kind);
        let database_id = i64::from(launchbox_db_id);
        if exact {
            self.rust().media.exact(database_id, kind)
        } else {
            self.rust().media.selected(database_id, kind)
        }
    }

    fn media_candidate(
        &self,
        launchbox_db_id: i32,
        artwork_type: &str,
        index: i32,
    ) -> Option<&MediaAsset> {
        let index = usize::try_from(index).ok()?;
        let kind = ArtworkKind::parse(artwork_type).unwrap_or(self.rust().artwork_kind);
        self.rust()
            .media
            .candidate(i64::from(launchbox_db_id), kind, index)
    }

    fn start_media_load(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().media_generation =
            self.as_ref().rust().media_generation.wrapping_add(1);
        self.as_mut().rust_mut().media_started = Some(std::time::Instant::now());
        let generation = self.as_ref().rust().media_generation;
        let directory = crate::media::requested_media_directory();
        self.as_mut()
            .set_media_directory(qstring(directory.to_string_lossy()));
        self.as_mut().set_media_loading(true);

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-media-index".into())
            .spawn(move || {
                let provider_priority = crate::settings::SettingsStore::open_default()
                    .and_then(|store| store.load())
                    .map(|settings| {
                        crate::media::effective_provider_priority(&settings.media_provider_priority)
                    })
                    .unwrap_or_else(|_| crate::media::default_provider_priority());
                let index = MediaIndex::scan_with_provider_priority(directory, provider_priority);
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_media_load(generation, index);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_media_loading(false);
            self.as_mut().set_status_message(qstring(format!(
                "Catalog ready, but the media index could not start: {error}"
            )));
        }
    }

    fn finish_media_load(mut self: Pin<&mut Self>, generation: u64, index: MediaIndex) {
        if generation != self.as_ref().rust().media_generation {
            return;
        }
        let game_count = index.games.len();
        let asset_count = index.asset_count;
        let skipped_entries = index.skipped_entries;
        let warning = index.warning.clone();
        let root = index.root.clone();
        let elapsed_ms = self
            .as_ref()
            .rust()
            .media_started
            .map(|started| started.elapsed().as_millis())
            .unwrap_or_default();
        self.as_mut().rust_mut().media = Arc::new(index);
        self.as_mut()
            .set_media_directory(qstring(root.to_string_lossy()));
        self.as_mut()
            .set_media_game_count(saturating_i32(game_count));
        self.as_mut()
            .set_media_asset_count(saturating_i32(asset_count));
        self.as_mut().set_media_loading(false);
        self.as_mut().start_media_fetch_queue(root);
        let revision = self.as_ref().media_revision().wrapping_add(1);
        self.as_mut().set_media_revision(revision);
        println!(
            "LUNCHBOX_MEDIA_READY_MS={elapsed_ms} games={game_count} assets={asset_count} skipped={skipped_entries}"
        );
        if let Some(warning) = warning {
            self.as_mut().set_status_message(qstring(format!(
                "Catalog ready — artwork cache unavailable: {warning}"
            )));
        }
    }

    fn start_media_fetch_queue(mut self: Pin<&mut Self>, root: std::path::PathBuf) {
        if !*self.as_ref().media_retrieval_enabled()
            || self.as_ref().rust().media_fetch_queue.is_some()
        {
            if !*self.as_ref().media_retrieval_enabled() {
                self.as_mut()
                    .set_media_fetch_message(qstring("Offline mode — cached artwork only."));
            }
            return;
        }
        let qt_thread = self.as_ref().qt_thread();
        match MediaFetchQueue::start(root, move |outcome| {
            let _ = qt_thread.queue(move |mut model| {
                model.as_mut().finish_media_fetch(outcome);
            });
        }) {
            Ok(queue) => {
                self.as_mut().rust_mut().media_fetch_queue = Some(queue);
                self.as_mut().set_media_fetch_message(qstring(
                    "On-demand LibRetro artwork is ready; visible games load first.",
                ));
            }
            Err(error) => {
                self.as_mut().set_media_retrieval_enabled(false);
                self.as_mut().set_media_fetch_message(qstring(format!(
                    "Artwork retrieval could not start: {error}"
                )));
            }
        }
    }

    fn finish_media_fetch(mut self: Pin<&mut Self>, outcome: MediaFetchOutcome) {
        let (database_id, requested_kind) = match &outcome {
            MediaFetchOutcome::Found {
                database_id,
                requested_kind,
                ..
            }
            | MediaFetchOutcome::Missing {
                database_id,
                requested_kind,
                ..
            }
            | MediaFetchOutcome::Failed {
                database_id,
                requested_kind,
                ..
            } => (*database_id, *requested_kind),
        };
        self.as_mut()
            .rust_mut()
            .media_requests
            .remove(&(database_id, requested_kind));
        let pending = self.as_ref().rust().media_requests.len();
        self.as_mut()
            .set_media_pending_count(saturating_i32(pending));

        match outcome {
            MediaFetchOutcome::Found {
                fetched_kind,
                path,
                force,
                ..
            } => {
                let inserted = Arc::make_mut(&mut self.as_mut().rust_mut().media).insert_asset(
                    database_id,
                    fetched_kind,
                    path,
                    "libretro",
                );
                if inserted || force {
                    let games = self.as_ref().rust().media.games.len();
                    let assets = self.as_ref().rust().media.asset_count;
                    self.as_mut().set_media_game_count(saturating_i32(games));
                    self.as_mut().set_media_asset_count(saturating_i32(assets));
                    let downloaded = self.as_ref().media_downloaded_count().saturating_add(1);
                    self.as_mut().set_media_downloaded_count(downloaded);
                    let revision = self.as_ref().media_revision().wrapping_add(1);
                    self.as_mut().set_media_revision(revision);
                    self.as_mut().set_media_fetch_message(qstring(format!(
                        "{} {} artwork from LibRetro.",
                        if force { "Refreshed" } else { "Cached" },
                        fetched_kind.key().replace('-', " ")
                    )));
                    if *self.as_ref().media_fetch_probe() {
                        let elapsed = self
                            .as_ref()
                            .rust()
                            .media_fetch_started
                            .map(|started| started.elapsed().as_millis())
                            .unwrap_or_default();
                        println!(
                            "LUNCHBOX_MEDIA_FETCH_READY_MS={elapsed} database_id={database_id} kind={} assets={assets} refreshed={force}",
                            fetched_kind.key()
                        );
                    }
                    if force {
                        println!(
                            "LUNCHBOX_MEDIA_REFRESH_READY database_id={database_id} kind={} assets={assets}",
                            fetched_kind.key()
                        );
                    }
                }
            }
            MediaFetchOutcome::Missing { force, .. } => {
                let missing = self.as_ref().media_missing_count().saturating_add(1);
                self.as_mut().set_media_missing_count(missing);
                if force {
                    let revision = self.as_ref().media_revision().wrapping_add(1);
                    self.as_mut().set_media_revision(revision);
                    self.as_mut().set_media_fetch_message(qstring(
                        "LibRetro has no replacement for this artwork; the cached copy was kept.",
                    ));
                }
            }
            MediaFetchOutcome::Failed { error, force, .. } => {
                let errors = self.as_ref().media_error_count().saturating_add(1);
                self.as_mut().set_media_error_count(errors);
                self.as_mut()
                    .set_media_fetch_message(qstring(format!("Artwork retrieval paused: {error}")));
                if force {
                    let revision = self.as_ref().media_revision().wrapping_add(1);
                    self.as_mut().set_media_revision(revision);
                }
            }
        }
    }

    fn finish_filter(mut self: Pin<&mut Self>, generation: u64, indices: Vec<usize>) {
        if generation != self.as_ref().rust().filter_generation {
            return;
        }
        let count = indices.len();
        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().filtered_indices = indices;
        self.as_mut().end_reset_model();
        self.as_mut().set_filtered_count(saturating_i32(count));
        self.as_mut().set_filtering(false);
        let filter_ms = self
            .as_ref()
            .rust()
            .filter_started
            .map(|started| started.elapsed().as_millis())
            .unwrap_or_default();
        println!("LUNCHBOX_FILTER_READY_MS={filter_ms} results={count}");
    }

    pub fn platform_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().catalog.platforms.get(index))
            .map(|platform| qstring(&platform.name))
            .unwrap_or_default()
    }

    pub fn platform_game_count_at(&self, index: i32) -> i32 {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().catalog.platforms.get(index))
            .map(|platform| saturating_i32(platform.game_count))
            .unwrap_or_default()
    }

    pub fn shell_ready(mut self: Pin<&mut Self>) {
        if *self.as_ref().startup_ms() != 0 {
            return;
        }
        let milliseconds = crate::startup_elapsed().as_millis();
        let milliseconds = i32::try_from(milliseconds).unwrap_or(i32::MAX).max(1);
        self.as_mut().set_startup_ms(milliseconds);
        println!("LUNCHBOX_SHELL_READY_MS={milliseconds}");
    }

    pub fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() {
            0
        } else {
            saturating_i32(self.rust().filtered_indices.len())
        }
    }

    pub fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        if !index.is_valid() || index.column() != 0 {
            return QVariant::default();
        }
        let Some(game) = usize::try_from(index.row())
            .ok()
            .and_then(|row| self.rust().filtered_indices.get(row))
            .and_then(|index| self.rust().catalog.games.get(*index))
        else {
            return QVariant::default();
        };

        match role {
            DISPLAY_ROLE | GAME_TITLE_ROLE => QVariant::from(&qstring(&game.title)),
            GAME_ID_ROLE => QVariant::from(&qstring(&game.id)),
            GAME_PLATFORM_ROLE => QVariant::from(&qstring(&game.platform)),
            GAME_STATUS_ROLE => QVariant::from(&qstring(&game.status)),
            GAME_LOCAL_ROLE => QVariant::from(&game.local),
            GAME_DOWNLOADABLE_ROLE => QVariant::from(&game.downloadable),
            GAME_DATABASE_ID_ROLE => {
                QVariant::from(&i32::try_from(game.launchbox_db_id).unwrap_or_default())
            }
            _ => QVariant::default(),
        }
    }

    pub fn role_names(&self) -> RoleNames {
        let mut roles = RoleNames::default();
        for (role, name) in ROLES {
            roles.insert(role, QByteArray::from(name));
        }
        roles
    }
}

fn media_asset_url(asset: &MediaAsset) -> QUrl {
    QUrl::from_local_file(&qstring(asset.path.to_string_lossy()))
}
