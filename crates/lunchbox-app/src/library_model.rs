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
        #[qproperty(bool, startup_probe)]
        #[qproperty(bool, catalog_probe)]
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
        #[qproperty(i32, platform_revision)]
        type LibraryModel = super::LibraryModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn reload(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn apply_filter(
            self: Pin<&mut LibraryModel>,
            search: QString,
            platform: QString,
            availability: QString,
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

use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QModelIndex, QString, QVariant};

use crate::catalog::{self, Catalog, Filter};

type RoleNames = QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;

const DISPLAY_ROLE: i32 = 0;
const USER_ROLE: i32 = 0x0100;
const GAME_ID_ROLE: i32 = USER_ROLE + 1;
const GAME_TITLE_ROLE: i32 = USER_ROLE + 2;
const GAME_PLATFORM_ROLE: i32 = USER_ROLE + 3;
const GAME_STATUS_ROLE: i32 = USER_ROLE + 4;
const GAME_LOCAL_ROLE: i32 = USER_ROLE + 5;
const GAME_DOWNLOADABLE_ROLE: i32 = USER_ROLE + 6;

const ROLES: [(i32, &str); 6] = [
    (GAME_ID_ROLE, "gameId"),
    (GAME_TITLE_ROLE, "gameTitle"),
    (GAME_PLATFORM_ROLE, "gamePlatform"),
    (GAME_STATUS_ROLE, "gameStatus"),
    (GAME_LOCAL_ROLE, "gameLocal"),
    (GAME_DOWNLOADABLE_ROLE, "gameDownloadable"),
];

pub struct LibraryModelRust {
    database_path: QString,
    status_message: QString,
    current_platform: QString,
    availability_filter: QString,
    loading: bool,
    filtering: bool,
    ready: bool,
    startup_probe: bool,
    catalog_probe: bool,
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
    platform_revision: i32,
    catalog: Arc<Catalog>,
    filtered_indices: Vec<usize>,
    load_generation: u64,
    filter_generation: u64,
    load_started: Option<std::time::Instant>,
}

impl Default for LibraryModelRust {
    fn default() -> Self {
        Self {
            database_path: QString::default(),
            status_message: QString::from("Starting instantly; catalog loading is deferred."),
            current_platform: QString::default(),
            availability_filter: QString::default(),
            loading: false,
            filtering: false,
            ready: false,
            startup_probe: std::env::args().any(|argument| argument == "--startup-probe"),
            catalog_probe: std::env::args().any(|argument| argument == "--catalog-probe"),
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
            platform_revision: 0,
            catalog: Arc::new(Catalog::default()),
            filtered_indices: Vec::new(),
            load_generation: 0,
            filter_generation: 0,
            load_started: None,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

impl qobject::LibraryModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().loading() || *self.as_ref().ready() {
            return;
        }
        self.as_mut().start_load();
    }

    pub fn reload(mut self: Pin<&mut Self>) {
        if *self.as_ref().loading() {
            return;
        }
        self.as_mut().start_load();
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
                let loaded = catalog::load(&path).map_err(|error| error.to_string());
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

    fn finish_load(mut self: Pin<&mut Self>, generation: u64, loaded: Result<Catalog, String>) {
        if generation != self.as_ref().rust().load_generation {
            return;
        }
        self.as_mut().set_loading(false);
        match loaded {
            Ok(catalog) => {
                let indices = (0..catalog.games.len()).collect::<Vec<_>>();
                self.as_mut().begin_reset_model();
                {
                    let mut this = self.as_mut();
                    let mut rust = this.as_mut().rust_mut();
                    rust.catalog = Arc::new(catalog);
                    rust.filtered_indices = indices;
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
                    .filter(|game| game.downloadable)
                    .count();
                let emulator_count = self.as_ref().rust().catalog.emulator_count;
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
                self.as_mut().set_catalog_ms(catalog_ms);
                println!(
                    "LUNCHBOX_CATALOG_READY_MS={catalog_ms} games={game_count} platforms={platform_count} local_files={local_file_count} offers={offer_count} emulators={emulator_count}"
                );
                self.as_mut().set_ready(true);
                let revision = self.as_ref().platform_revision().wrapping_add(1);
                self.as_mut().set_platform_revision(revision);
                self.as_mut().set_status_message(qstring(format!(
                    "Ready — {game_count} games across {platform_count} platforms"
                )));
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
        let filter = Filter {
            search: search.to_string(),
            platform: platform.to_string(),
            availability: availability.to_string(),
        };
        self.as_mut().set_current_platform(platform);
        self.as_mut().set_availability_filter(availability);
        self.as_mut().rust_mut().filter_generation =
            self.as_ref().rust().filter_generation.wrapping_add(1);
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
