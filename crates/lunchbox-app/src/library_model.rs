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
        #[qproperty(QString, tag_filter)]
        #[qproperty(bool, loading)]
        #[qproperty(bool, filtering)]
        #[qproperty(bool, ready)]
        #[qproperty(bool, hide_non_retail)]
        #[qproperty(bool, hide_adult)]
        #[qproperty(QString, artwork_type)]
        #[qproperty(i32, grid_zoom)]
        #[qproperty(QString, view_mode)]
        #[qproperty(QString, sort_field)]
        #[qproperty(bool, sort_descending)]
        #[qproperty(QString, list_columns)]
        #[qproperty(i32, list_filter_active_count)]
        #[qproperty(QString, list_filter_column)]
        #[qproperty(bool, list_filter_loading)]
        #[qproperty(i32, list_filter_value_count)]
        #[qproperty(i32, list_filter_total_distinct)]
        #[qproperty(i32, list_filter_revision)]
        #[qproperty(QString, hover_preview_game_id)]
        #[qproperty(QUrl, hover_preview_url)]
        #[qproperty(QString, hover_preview_source)]
        #[qproperty(QString, hover_preview_message)]
        #[qproperty(bool, hover_preview_loading)]
        #[qproperty(bool, hover_preview_probe)]
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
        #[qproperty(bool, sidebar_probe)]
        #[qproperty(bool, collection_busy)]
        #[qproperty(i32, startup_ms)]
        #[qproperty(i32, catalog_ms)]
        #[qproperty(i32, game_count)]
        #[qproperty(i32, filtered_count)]
        #[qproperty(i32, platform_count)]
        #[qproperty(QString, platform_search)]
        #[qproperty(i32, filtered_platform_count)]
        #[qproperty(i32, sidebar_width)]
        #[qproperty(bool, sidebar_state_saving)]
        #[qproperty(QString, couch_shelf)]
        #[qproperty(QString, couch_platform)]
        #[qproperty(QString, couch_theme_id)]
        #[qproperty(QString, couch_theme_name)]
        #[qproperty(QString, couch_theme_author)]
        #[qproperty(QString, couch_theme_description)]
        #[qproperty(QString, couch_theme_background)]
        #[qproperty(QString, couch_theme_panel)]
        #[qproperty(QString, couch_theme_panel_raised)]
        #[qproperty(QString, couch_theme_ink)]
        #[qproperty(QString, couch_theme_muted)]
        #[qproperty(QString, couch_theme_accent)]
        #[qproperty(QString, couch_theme_accent_cool)]
        #[qproperty(QString, couch_theme_danger)]
        #[qproperty(QUrl, couch_theme_background_image)]
        #[qproperty(i32, couch_theme_hero_scrim_percent)]
        #[qproperty(i32, couch_theme_card_radius)]
        #[qproperty(i32, couch_theme_count)]
        #[qproperty(i32, couch_theme_revision)]
        #[qproperty(bool, couch_theme_busy)]
        #[qproperty(QString, couch_theme_message)]
        #[qproperty(bool, couch_attract_enabled)]
        #[qproperty(i32, couch_attract_idle_seconds)]
        #[qproperty(i32, couch_attract_cycle_seconds)]
        #[qproperty(bool, couch_state_saving)]
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
        #[qproperty(i32, metadata_revision)]
        #[qproperty(i32, tag_count)]
        #[qproperty(i32, tag_revision)]
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
        fn refresh_metadata(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn select_tag_filter(self: Pin<&mut LibraryModel>, tag: QString);

        #[qinvokable]
        fn tag_name_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn tag_game_count_at(self: &LibraryModel, index: i32) -> i32;

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
        fn choose_view_mode(self: Pin<&mut LibraryModel>, view_mode: QString);

        #[qinvokable]
        fn set_sort_preferences(
            self: Pin<&mut LibraryModel>,
            sort_field: QString,
            descending: bool,
        );

        #[qinvokable]
        fn configure_list_columns(self: Pin<&mut LibraryModel>, columns: QString);

        #[qinvokable]
        fn begin_list_column_filter(self: Pin<&mut LibraryModel>, column: QString);

        #[qinvokable]
        fn search_list_column_filter(self: Pin<&mut LibraryModel>, query: QString);

        #[qinvokable]
        fn set_list_filter_value_selected(
            self: Pin<&mut LibraryModel>,
            value: QString,
            selected: bool,
        );

        #[qinvokable]
        fn select_all_list_filter_values(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn select_no_list_filter_values(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn apply_list_column_filter(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn apply_exact_list_column_filter(
            self: Pin<&mut LibraryModel>,
            column: QString,
            value: QString,
        );

        #[qinvokable]
        fn clear_all_list_column_filters(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn list_column_filter_active(self: &LibraryModel, column: QString) -> bool;

        #[qinvokable]
        fn list_filter_value_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn list_filter_value_game_count_at(self: &LibraryModel, index: i32) -> i32;

        #[qinvokable]
        fn list_filter_value_selected_at(self: &LibraryModel, index: i32) -> bool;

        #[qinvokable]
        fn list_filter_draft_summary(self: &LibraryModel) -> QString;

        #[qinvokable]
        fn list_column_filter_summary(self: &LibraryModel) -> QString;

        #[qinvokable]
        fn request_hover_preview(self: Pin<&mut LibraryModel>, game_uid: QString);

        #[qinvokable]
        fn cancel_hover_preview(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn report_hover_preview_ui_probe(self: &LibraryModel);

        #[qinvokable]
        fn report_hover_preview_ui_failure(self: &LibraryModel, detail: QString);

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
        fn row_for_game(self: &LibraryModel, game_uid: QString) -> i32;

        #[qinvokable]
        fn canonical_title_for_game(self: &LibraryModel, game_uid: QString) -> QString;

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
        fn collection_kind_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn collection_rule_summary_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn collection_rule_at(self: &LibraryModel, index: i32, field: QString) -> QString;

        #[qinvokable]
        fn collection_member_game_uid_at(
            self: &LibraryModel,
            collection_index: i32,
            member_index: i32,
        ) -> QString;

        #[qinvokable]
        fn collection_member_name_at(
            self: &LibraryModel,
            collection_index: i32,
            member_index: i32,
        ) -> QString;

        #[qinvokable]
        fn collection_member_platform_at(
            self: &LibraryModel,
            collection_index: i32,
            member_index: i32,
        ) -> QString;

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
        fn set_smart_collection_rule_draft(
            self: Pin<&mut LibraryModel>,
            title_contains: QString,
            platform: QString,
            tag: QString,
            availability: QString,
            favorite: QString,
            completion_state: QString,
            content: QString,
            cooperative: QString,
        ) -> bool;

        #[qinvokable]
        fn create_smart_collection(
            self: Pin<&mut LibraryModel>,
            name: QString,
            description: QString,
        );

        #[qinvokable]
        fn update_collection(
            self: Pin<&mut LibraryModel>,
            collection_id: QString,
            name: QString,
            description: QString,
        );

        #[qinvokable]
        fn update_smart_collection(
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
        fn move_collection_game(
            self: Pin<&mut LibraryModel>,
            collection_id: QString,
            member_index: i32,
            new_index: i32,
        );

        #[qinvokable]
        fn bulk_set_visible_collection_membership(
            self: Pin<&mut LibraryModel>,
            collection_id: QString,
            member: bool,
        );

        #[qinvokable]
        fn export_collection(
            self: Pin<&mut LibraryModel>,
            collection_id: QString,
            destination: QUrl,
        );

        #[qinvokable]
        fn import_collection(self: Pin<&mut LibraryModel>, source: QUrl);

        #[qinvokable]
        fn platform_name_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn platform_game_count_at(self: &LibraryModel, index: i32) -> i32;

        #[qinvokable]
        fn filtered_platform_name_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn filtered_platform_game_count_at(self: &LibraryModel, index: i32) -> i32;

        #[qinvokable]
        fn filter_platforms(self: Pin<&mut LibraryModel>, query: QString);

        #[qinvokable]
        fn preview_sidebar_width(self: Pin<&mut LibraryModel>, width: i32);

        #[qinvokable]
        fn save_sidebar_state(self: Pin<&mut LibraryModel>, query: QString, width: i32);

        #[qinvokable]
        fn save_couch_state(
            self: Pin<&mut LibraryModel>,
            shelf: QString,
            platform: QString,
        ) -> bool;

        #[qinvokable]
        fn save_couch_attract_settings(
            self: Pin<&mut LibraryModel>,
            enabled: bool,
            idle_seconds: i32,
            cycle_seconds: i32,
        ) -> bool;

        #[qinvokable]
        fn couch_theme_id_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn couch_theme_name_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn couch_theme_author_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn couch_theme_description_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn couch_theme_installed_at(self: &LibraryModel, index: i32) -> bool;

        #[qinvokable]
        fn couch_theme_background_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn couch_theme_accent_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn couch_theme_ink_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn couch_theme_muted_at(self: &LibraryModel, index: i32) -> QString;

        #[qinvokable]
        fn select_couch_theme(self: Pin<&mut LibraryModel>, index: i32) -> bool;

        #[qinvokable]
        fn install_couch_theme(self: Pin<&mut LibraryModel>, package: QUrl);

        #[qinvokable]
        fn remove_couch_theme(self: Pin<&mut LibraryModel>, index: i32);

        #[qinvokable]
        fn refresh_couch_themes(self: Pin<&mut LibraryModel>);

        #[qinvokable]
        fn report_couch_theme_ui_probe(self: &LibraryModel);

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
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QModelIndex, QString, QUrl, QVariant};

use crate::catalog::{self, Catalog, Filter};
use crate::collections::{
    PortableCollection, PortableGameReference, SmartCollectionRules, load_portable_collection,
    resolve_portable_game_references, save_portable_collection,
};
use crate::couch_theme::{CouchTheme, ThemeCatalog};
use crate::media::{
    ArtworkKind, MediaAsset, MediaFetchOutcome, MediaFetchQueue, MediaFetchRequest, MediaIndex,
};
use crate::settings::{
    CouchModePreferences, GameCustomField, GameMetadataOverride, LibraryPreferences, PlayActivity,
    SettingsStore, SidebarPreferences, UserCollection, UserCollections, UserTag, UserTags,
};

type RoleNames = QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
type CatalogLoadResult = Result<
    (
        Catalog,
        Result<LibraryPreferences, String>,
        Result<SidebarPreferences, String>,
        Result<CouchModePreferences, String>,
        ThemeCatalog,
        Result<HashSet<String>, String>,
        Result<UserCollections, String>,
        Result<Vec<PlayActivity>, String>,
        Result<HashMap<String, GameMetadataOverride>, String>,
        Result<UserTags, String>,
        Result<HashMap<String, Vec<GameCustomField>>, String>,
    ),
    String,
>;

enum CollectionMutation {
    Created(UserCollection),
    Updated {
        collection_id: String,
        name: String,
        description: String,
        rules: Option<SmartCollectionRules>,
    },
    Deleted {
        collection_id: String,
    },
    Membership(CollectionMembership),
    Reloaded {
        collections: UserCollections,
        message: String,
    },
    Message(String),
}

struct CouchThemeUpdate {
    catalog: ThemeCatalog,
    selected_id: String,
    persist_selection: bool,
    message: String,
}

#[derive(Clone)]
struct CollectionMembership {
    collection_id: String,
    game_uid: String,
    member: bool,
    position: Option<usize>,
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
const GAME_CANONICAL_TITLE_ROLE: i32 = USER_ROLE + 8;
const GAME_DEVELOPER_ROLE: i32 = USER_ROLE + 9;
const GAME_PUBLISHER_ROLE: i32 = USER_ROLE + 10;
const GAME_YEAR_ROLE: i32 = USER_ROLE + 11;
const GAME_RELEASE_DATE_ROLE: i32 = USER_ROLE + 12;
const GAME_GENRE_ROLE: i32 = USER_ROLE + 13;
const GAME_PLAYERS_ROLE: i32 = USER_ROLE + 14;
const GAME_RATING_ROLE: i32 = USER_ROLE + 15;
const GAME_ESRB_ROLE: i32 = USER_ROLE + 16;
const GAME_COOPERATIVE_ROLE: i32 = USER_ROLE + 17;
const GAME_VARIANTS_ROLE: i32 = USER_ROLE + 18;
const GAME_RELEASE_TYPE_ROLE: i32 = USER_ROLE + 19;
const GAME_SERIES_ROLE: i32 = USER_ROLE + 20;
const GAME_REGION_ROLE: i32 = USER_ROLE + 21;
const GAME_NOTES_ROLE: i32 = USER_ROLE + 22;
const GAME_PLAY_MODE_ROLE: i32 = USER_ROLE + 23;
const GAME_VERSION_ROLE: i32 = USER_ROLE + 24;
const GAME_RELEASE_STATUS_ROLE: i32 = USER_ROLE + 25;

const ROLES: [(i32, &str); 25] = [
    (GAME_ID_ROLE, "gameId"),
    (GAME_TITLE_ROLE, "gameTitle"),
    (GAME_PLATFORM_ROLE, "gamePlatform"),
    (GAME_STATUS_ROLE, "gameStatus"),
    (GAME_LOCAL_ROLE, "gameLocal"),
    (GAME_DOWNLOADABLE_ROLE, "gameDownloadable"),
    (GAME_DATABASE_ID_ROLE, "gameDatabaseId"),
    (GAME_CANONICAL_TITLE_ROLE, "gameCanonicalTitle"),
    (GAME_DEVELOPER_ROLE, "gameDeveloper"),
    (GAME_PUBLISHER_ROLE, "gamePublisher"),
    (GAME_YEAR_ROLE, "gameYear"),
    (GAME_RELEASE_DATE_ROLE, "gameReleaseDate"),
    (GAME_GENRE_ROLE, "gameGenre"),
    (GAME_PLAYERS_ROLE, "gamePlayers"),
    (GAME_RATING_ROLE, "gameRating"),
    (GAME_ESRB_ROLE, "gameEsrb"),
    (GAME_COOPERATIVE_ROLE, "gameCooperative"),
    (GAME_VARIANTS_ROLE, "gameVariants"),
    (GAME_RELEASE_TYPE_ROLE, "gameReleaseType"),
    (GAME_SERIES_ROLE, "gameSeries"),
    (GAME_REGION_ROLE, "gameRegion"),
    (GAME_NOTES_ROLE, "gameNotes"),
    (GAME_PLAY_MODE_ROLE, "gamePlayMode"),
    (GAME_VERSION_ROLE, "gameVersion"),
    (GAME_RELEASE_STATUS_ROLE, "gameReleaseStatus"),
];

pub struct LibraryModelRust {
    database_path: QString,
    status_message: QString,
    current_platform: QString,
    availability_filter: QString,
    tag_filter: QString,
    loading: bool,
    filtering: bool,
    ready: bool,
    hide_non_retail: bool,
    hide_adult: bool,
    artwork_type: QString,
    grid_zoom: i32,
    view_mode: QString,
    sort_field: QString,
    sort_descending: bool,
    list_columns: QString,
    list_filter_active_count: i32,
    list_filter_column: QString,
    list_filter_loading: bool,
    list_filter_value_count: i32,
    list_filter_total_distinct: i32,
    list_filter_revision: i32,
    hover_preview_game_id: QString,
    hover_preview_url: QUrl,
    hover_preview_source: QString,
    hover_preview_message: QString,
    hover_preview_loading: bool,
    hover_preview_probe: bool,
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
    sidebar_probe: bool,
    collection_busy: bool,
    startup_ms: i32,
    catalog_ms: i32,
    game_count: i32,
    filtered_count: i32,
    platform_count: i32,
    platform_search: QString,
    filtered_platform_count: i32,
    sidebar_width: i32,
    sidebar_state_saving: bool,
    couch_shelf: QString,
    couch_platform: QString,
    couch_theme_id: QString,
    couch_theme_name: QString,
    couch_theme_author: QString,
    couch_theme_description: QString,
    couch_theme_background: QString,
    couch_theme_panel: QString,
    couch_theme_panel_raised: QString,
    couch_theme_ink: QString,
    couch_theme_muted: QString,
    couch_theme_accent: QString,
    couch_theme_accent_cool: QString,
    couch_theme_danger: QString,
    couch_theme_background_image: QUrl,
    couch_theme_hero_scrim_percent: i32,
    couch_theme_card_radius: i32,
    couch_theme_count: i32,
    couch_theme_revision: i32,
    couch_theme_busy: bool,
    couch_theme_message: QString,
    couch_attract_enabled: bool,
    couch_attract_idle_seconds: i32,
    couch_attract_cycle_seconds: i32,
    couch_state_saving: bool,
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
    metadata_revision: i32,
    tag_count: i32,
    tag_revision: i32,
    platform_revision: i32,
    catalog: Arc<Catalog>,
    media: Arc<MediaIndex>,
    artwork_kind: ArtworkKind,
    filtered_indices: Vec<usize>,
    filtered_platform_indices: Vec<usize>,
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
    collection_order: Arc<HashMap<String, Arc<Vec<String>>>>,
    completion_states: Arc<HashMap<String, String>>,
    metadata_titles: Arc<HashMap<String, String>>,
    metadata_cooperative: Arc<HashMap<String, String>>,
    metadata_overrides: Arc<HashMap<String, GameMetadataOverride>>,
    list_column_filters:
        Arc<HashMap<crate::list_view::ListColumn, crate::list_view::ListColumnFilter>>,
    list_filter_values: Vec<catalog::ListFacetValue>,
    list_filter_draft: crate::list_view::ListColumnFilter,
    list_filter_generation: u64,
    hover_preview_generation: u64,
    tags: Arc<Vec<UserTag>>,
    game_tags: Arc<HashMap<String, Vec<String>>>,
    game_custom_fields: Arc<HashMap<String, Vec<String>>>,
    metadata_generation: u64,
    game_index_by_id: Arc<HashMap<String, usize>>,
    current_search: String,
    smart_collection_rule_draft: Option<SmartCollectionRules>,
    collection_started: Option<std::time::Instant>,
    sidebar_save_generation: u64,
    sidebar_save_pending: bool,
    couch_save_generation: u64,
    couch_save_pending: bool,
    couch_themes: Vec<CouchTheme>,
    couch_theme_generation: u64,
    couch_theme_ui_probe: bool,
}

impl Default for LibraryModelRust {
    fn default() -> Self {
        let preferences = LibraryPreferences::default();
        let couch_preferences = CouchModePreferences::default();
        let theme_catalog = crate::couch_theme::built_in_catalog();
        let couch_theme = crate::couch_theme::default_theme();
        Self {
            database_path: QString::default(),
            status_message: QString::from("Starting instantly; catalog loading is deferred."),
            current_platform: QString::default(),
            availability_filter: QString::default(),
            tag_filter: QString::default(),
            loading: false,
            filtering: false,
            ready: false,
            hide_non_retail: preferences.hide_non_retail,
            hide_adult: preferences.hide_adult,
            artwork_type: qstring(&preferences.artwork_type),
            grid_zoom: preferences.grid_zoom,
            view_mode: qstring(&preferences.view_mode),
            sort_field: qstring(&preferences.sort_field),
            sort_descending: preferences.sort_descending,
            list_columns: qstring(&preferences.list_columns),
            list_filter_active_count: 0,
            list_filter_column: QString::default(),
            list_filter_loading: false,
            list_filter_value_count: 0,
            list_filter_total_distinct: 0,
            list_filter_revision: 0,
            hover_preview_game_id: QString::default(),
            hover_preview_url: QUrl::default(),
            hover_preview_source: QString::default(),
            hover_preview_message: QString::from(
                "Pause over a game to preview a cached gameplay video.",
            ),
            hover_preview_loading: false,
            hover_preview_probe: std::env::args()
                .any(|argument| argument == "--hover-preview-ui-probe"),
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
            sidebar_probe: std::env::args().any(|argument| {
                matches!(
                    argument.as_str(),
                    "--sidebar-ui-probe" | "--sidebar-restored-ui-probe"
                )
            }),
            collection_busy: false,
            startup_ms: 0,
            catalog_ms: 0,
            game_count: 0,
            filtered_count: 0,
            platform_count: 0,
            platform_search: QString::default(),
            filtered_platform_count: 0,
            sidebar_width: SidebarPreferences::default().width,
            sidebar_state_saving: false,
            couch_shelf: qstring(&couch_preferences.shelf),
            couch_platform: QString::default(),
            couch_theme_id: qstring(&couch_theme.id),
            couch_theme_name: qstring(&couch_theme.name),
            couch_theme_author: qstring(&couch_theme.author),
            couch_theme_description: qstring(&couch_theme.description),
            couch_theme_background: qstring(&couch_theme.background),
            couch_theme_panel: qstring(&couch_theme.panel),
            couch_theme_panel_raised: qstring(&couch_theme.panel_raised),
            couch_theme_ink: qstring(&couch_theme.ink),
            couch_theme_muted: qstring(&couch_theme.muted),
            couch_theme_accent: qstring(&couch_theme.accent),
            couch_theme_accent_cool: qstring(&couch_theme.accent_cool),
            couch_theme_danger: qstring(&couch_theme.danger),
            couch_theme_background_image: QUrl::default(),
            couch_theme_hero_scrim_percent: i32::from(couch_theme.hero_scrim_percent),
            couch_theme_card_radius: i32::from(couch_theme.card_radius),
            couch_theme_count: saturating_i32(theme_catalog.themes.len()),
            couch_theme_revision: 0,
            couch_theme_busy: false,
            couch_theme_message: qstring(
                "Choose a built-in theme or install a declarative .lunchbox-theme package.",
            ),
            couch_attract_enabled: couch_preferences.attract_enabled,
            couch_attract_idle_seconds: couch_preferences.attract_idle_seconds,
            couch_attract_cycle_seconds: couch_preferences.attract_cycle_seconds,
            couch_state_saving: false,
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
            metadata_revision: 0,
            tag_count: 0,
            tag_revision: 0,
            platform_revision: 0,
            catalog: Arc::new(Catalog::default()),
            media: Arc::new(MediaIndex::default()),
            artwork_kind: ArtworkKind::default(),
            filtered_indices: Vec::new(),
            filtered_platform_indices: Vec::new(),
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
            collection_order: Arc::new(HashMap::new()),
            completion_states: Arc::new(HashMap::new()),
            metadata_titles: Arc::new(HashMap::new()),
            metadata_cooperative: Arc::new(HashMap::new()),
            metadata_overrides: Arc::new(HashMap::new()),
            list_column_filters: Arc::new(HashMap::new()),
            list_filter_values: Vec::new(),
            list_filter_draft: crate::list_view::ListColumnFilter::all(),
            list_filter_generation: 0,
            hover_preview_generation: 0,
            tags: Arc::new(Vec::new()),
            game_tags: Arc::new(HashMap::new()),
            game_custom_fields: Arc::new(HashMap::new()),
            metadata_generation: 0,
            game_index_by_id: Arc::new(HashMap::new()),
            current_search: String::new(),
            smart_collection_rule_draft: None,
            collection_started: None,
            sidebar_save_generation: 0,
            sidebar_save_pending: false,
            couch_save_generation: 0,
            couch_save_pending: false,
            couch_themes: theme_catalog.themes,
            couch_theme_generation: 0,
            couch_theme_ui_probe: std::env::args()
                .any(|argument| argument == "--couch-theme-ui-probe"),
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn metadata_display_title<'a>(
    game: &'a catalog::Game,
    metadata_titles: &'a HashMap<String, String>,
) -> &'a str {
    metadata_titles
        .get(&game.id)
        .map(String::as_str)
        .unwrap_or(&game.title)
}

fn effective_cooperative<'a>(
    game: &'a catalog::Game,
    metadata_cooperative: &'a HashMap<String, String>,
) -> &'a str {
    metadata_cooperative
        .get(&game.id)
        .map(String::as_str)
        .unwrap_or(&game.cooperative)
}

fn custom_field_search_values(
    fields: HashMap<String, Vec<GameCustomField>>,
) -> HashMap<String, Vec<String>> {
    fields
        .into_iter()
        .map(|(game_uid, fields)| {
            let values = fields
                .into_iter()
                .flat_map(|field| [field.name, field.value])
                .collect();
            (game_uid, values)
        })
        .collect()
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn collection_at(model: &qobject::LibraryModel, index: i32) -> Option<&UserCollection> {
    usize::try_from(index)
        .ok()
        .and_then(|index| model.rust().collections.get(index))
}

fn collection_member_game(
    model: &qobject::LibraryModel,
    collection_index: i32,
    member_index: i32,
) -> Option<&catalog::Game> {
    let collection = collection_at(model, collection_index)?;
    let member_index = usize::try_from(member_index).ok()?;
    let game_uid = model
        .rust()
        .collection_order
        .get(&collection.id)?
        .get(member_index)?;
    let game_index = *model.rust().game_index_by_id.get(game_uid)?;
    model.rust().catalog.games.get(game_index)
}

fn smart_rules_from_fields(
    title_contains: QString,
    platform: QString,
    tag: QString,
    availability: QString,
    favorite: QString,
    completion_state: QString,
    content: QString,
    cooperative: QString,
) -> anyhow::Result<SmartCollectionRules> {
    SmartCollectionRules {
        title_contains: title_contains.to_string(),
        platform: platform.to_string(),
        tag: tag.to_string(),
        availability: availability.to_string(),
        favorite: favorite.to_string(),
        completion_state: completion_state.to_string(),
        content: content.to_string(),
        cooperative: cooperative.to_string(),
    }
    .normalized()
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
                    .and_then(|store| store.all_play_activity())
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

    pub fn refresh_metadata(mut self: Pin<&mut Self>) {
        if !*self.as_ref().ready() {
            return;
        }
        self.as_mut().rust_mut().metadata_generation =
            self.as_ref().rust().metadata_generation.wrapping_add(1);
        let generation = self.as_ref().rust().metadata_generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-metadata-load".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| {
                        let metadata = store.all_game_metadata_overrides()?;
                        let tags = store.all_user_tags()?;
                        let custom_fields = store.all_game_custom_fields()?;
                        let mut titles = HashMap::new();
                        let mut cooperative = HashMap::new();
                        for (game_uid, values) in &metadata {
                            if let Some(title) = &values.title {
                                titles.insert(game_uid.clone(), title.clone());
                            }
                            if let Some(value) = &values.cooperative {
                                cooperative.insert(game_uid.clone(), value.clone());
                            }
                        }
                        Ok((
                            metadata,
                            titles,
                            cooperative,
                            tags,
                            custom_field_search_values(custom_fields),
                        ))
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_metadata_refresh(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_status_message(qstring(format!(
                "Could not start metadata refresh: {error}"
            )));
        }
    }

    fn finish_metadata_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<
            (
                HashMap<String, GameMetadataOverride>,
                HashMap<String, String>,
                HashMap<String, String>,
                UserTags,
                HashMap<String, Vec<String>>,
            ),
            String,
        >,
    ) {
        if generation != self.as_ref().rust().metadata_generation {
            return;
        }
        match result {
            Ok((metadata, metadata_titles, metadata_cooperative, tags, custom_fields)) => {
                self.as_mut().begin_reset_model();
                self.as_mut().rust_mut().metadata_overrides = Arc::new(metadata);
                self.as_mut().rust_mut().metadata_titles = Arc::new(metadata_titles);
                self.as_mut().rust_mut().metadata_cooperative = Arc::new(metadata_cooperative);
                self.as_mut().rust_mut().game_tags = Arc::new(tags.game_tags);
                self.as_mut().rust_mut().tags = Arc::new(tags.tags);
                self.as_mut().rust_mut().game_custom_fields = Arc::new(custom_fields);
                self.as_mut().rebuild_smart_collections();
                self.as_mut().end_reset_model();
                let tag_count = self.as_ref().rust().tags.len();
                self.as_mut().set_tag_count(saturating_i32(tag_count));
                let active_tag_filter = self.as_ref().tag_filter().to_string().to_lowercase();
                if !active_tag_filter.is_empty()
                    && !self
                        .as_ref()
                        .rust()
                        .tags
                        .iter()
                        .any(|tag| tag.name.to_lowercase() == active_tag_filter)
                {
                    self.as_mut().set_tag_filter(QString::default());
                }
                let tag_revision = self.as_ref().tag_revision().wrapping_add(1);
                self.as_mut().set_tag_revision(tag_revision);
                let collection_revision = self.as_ref().collection_revision().wrapping_add(1);
                self.as_mut().set_collection_revision(collection_revision);
                let metadata_revision = self.as_ref().metadata_revision().wrapping_add(1);
                self.as_mut().set_metadata_revision(metadata_revision);
                let search = qstring(&self.as_ref().rust().current_search);
                let platform = self.as_ref().current_platform().clone();
                let availability = self.as_ref().availability_filter().clone();
                self.as_mut().apply_filter(search, platform, availability);
            }
            Err(error) => self.as_mut().set_status_message(qstring(format!(
                "Could not refresh metadata overrides: {error}"
            ))),
        }
    }

    pub fn select_tag_filter(mut self: Pin<&mut Self>, tag: QString) {
        let requested = tag.to_string();
        let normalized_request = requested.trim().to_lowercase();
        let canonical = if requested.trim().is_empty() {
            String::new()
        } else {
            self.as_ref()
                .rust()
                .tags
                .iter()
                .find(|tag| tag.name.to_lowercase() == normalized_request)
                .map(|tag| tag.name.clone())
                .unwrap_or_default()
        };
        if self.as_ref().tag_filter().to_string() == canonical {
            return;
        }
        self.as_mut().set_tag_filter(qstring(&canonical));
        if *self.as_ref().ready() {
            let search = qstring(&self.as_ref().rust().current_search);
            let platform = self.as_ref().current_platform().clone();
            let availability = self.as_ref().availability_filter().clone();
            self.as_mut().apply_filter(search, platform, availability);
        }
    }

    pub fn tag_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().tags.get(index))
            .map(|tag| qstring(&tag.name))
            .unwrap_or_default()
    }

    pub fn tag_game_count_at(&self, index: i32) -> i32 {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().tags.get(index))
            .map(|tag| saturating_i32(tag.game_count))
            .unwrap_or_default()
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
                    .iter()
                    .filter(|activity| activity.play_count > 0)
                    .map(|activity| (activity.game_uid.clone(), activity.last_played_at))
                    .collect::<HashMap<_, _>>();
                let completion_states = activity
                    .into_iter()
                    .map(|activity| (activity.game_uid, activity.completion_state))
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
                self.as_mut().rust_mut().completion_states = Arc::new(completion_states);
                self.as_mut().set_recent_count(saturating_i32(recent_count));
                let has_smart_collections = self
                    .as_ref()
                    .rust()
                    .collections
                    .iter()
                    .any(|collection| collection.kind == "smart");
                self.as_mut().rebuild_smart_collections();
                if has_smart_collections {
                    self.as_mut().bump_collection_revision();
                }
                let revision = self.as_ref().activity_revision().wrapping_add(1);
                self.as_mut().set_activity_revision(revision);
                self.as_mut().refresh_active_collection_filter();
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
                        let (
                            preferences,
                            sidebar_preferences,
                            couch_preferences,
                            favorites,
                            collections,
                            activity,
                            metadata,
                            tags,
                            custom_fields,
                        ) = match SettingsStore::open_default() {
                            Ok(store) => (
                                store
                                    .load_library_preferences()
                                    .map_err(|error| error.to_string()),
                                store
                                    .load_sidebar_preferences()
                                    .map_err(|error| error.to_string()),
                                store
                                    .load_couch_mode_preferences()
                                    .map_err(|error| error.to_string()),
                                store.favorite_game_ids().map_err(|error| error.to_string()),
                                store.user_collections().map_err(|error| error.to_string()),
                                store.all_play_activity().map_err(|error| error.to_string()),
                                store
                                    .all_game_metadata_overrides()
                                    .map_err(|error| error.to_string()),
                                store.all_user_tags().map_err(|error| error.to_string()),
                                store
                                    .all_game_custom_fields()
                                    .map_err(|error| error.to_string()),
                            ),
                            Err(error) => {
                                let error = error.to_string();
                                (
                                    Err(error.clone()),
                                    Err(error.clone()),
                                    Err(error.clone()),
                                    Err(error.clone()),
                                    Err(error.clone()),
                                    Err(error.clone()),
                                    Err(error.clone()),
                                    Err(error.clone()),
                                    Err(error),
                                )
                            }
                        };
                        let theme_catalog = match crate::settings::state_database_path() {
                            Ok(state_database) => {
                                crate::couch_theme::catalog_for_state(&state_database)
                            }
                            Err(error) => {
                                let mut catalog = crate::couch_theme::built_in_catalog();
                                catalog.warnings.push(error.to_string());
                                catalog
                            }
                        };
                        (
                            catalog,
                            preferences,
                            sidebar_preferences,
                            couch_preferences,
                            theme_catalog,
                            favorites,
                            collections,
                            activity,
                            metadata,
                            tags,
                            custom_fields,
                        )
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
            Ok((
                catalog,
                preferences,
                sidebar_preferences,
                couch_preferences,
                theme_catalog,
                favorites,
                collections,
                activity,
                metadata,
                tags,
                custom_fields,
            )) => {
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
                        self.as_mut().set_view_mode(qstring(&preferences.view_mode));
                        self.as_mut()
                            .set_sort_field(qstring(&preferences.sort_field));
                        self.as_mut()
                            .set_sort_descending(preferences.sort_descending);
                        self.as_mut()
                            .set_list_columns(qstring(&preferences.list_columns));
                        self.as_mut().rust_mut().artwork_kind = artwork_kind;
                        None
                    }
                    Err(error) => Some(error),
                };
                let sidebar_warning = match sidebar_preferences {
                    Ok(preferences) => {
                        self.as_mut()
                            .set_platform_search(qstring(&preferences.platform_search));
                        self.as_mut().set_sidebar_width(preferences.width);
                        None
                    }
                    Err(error) => Some(error),
                };
                let ThemeCatalog {
                    themes,
                    warnings: mut couch_warnings,
                } = theme_catalog;
                self.as_mut().rust_mut().couch_themes = themes;
                let theme_count = self.as_ref().rust().couch_themes.len();
                self.as_mut()
                    .set_couch_theme_count(saturating_i32(theme_count));
                match couch_preferences {
                    Ok(preferences) => {
                        self.as_mut()
                            .set_couch_attract_enabled(preferences.attract_enabled);
                        self.as_mut()
                            .set_couch_attract_idle_seconds(preferences.attract_idle_seconds);
                        self.as_mut()
                            .set_couch_attract_cycle_seconds(preferences.attract_cycle_seconds);
                        if !self.as_mut().apply_couch_theme_id(&preferences.theme_id) {
                            couch_warnings.push(format!(
                                "saved theme {} is not installed; using {}",
                                preferences.theme_id,
                                crate::couch_theme::DEFAULT_THEME_ID
                            ));
                            let _ = self
                                .as_mut()
                                .apply_couch_theme_id(crate::couch_theme::DEFAULT_THEME_ID);
                        }
                        if preferences.shelf == "platform"
                            && !catalog
                                .platforms
                                .iter()
                                .any(|platform| platform.name == preferences.platform)
                        {
                            self.as_mut().set_couch_shelf(qstring("all"));
                            self.as_mut().set_couch_platform(QString::default());
                            couch_warnings.push(format!(
                                "saved platform {} is no longer present in the catalog",
                                preferences.platform
                            ));
                        } else {
                            self.as_mut().set_couch_shelf(qstring(&preferences.shelf));
                            self.as_mut()
                                .set_couch_platform(qstring(&preferences.platform));
                        }
                    }
                    Err(error) => {
                        couch_warnings.push(error);
                        let _ = self
                            .as_mut()
                            .apply_couch_theme_id(crate::couch_theme::DEFAULT_THEME_ID);
                    }
                }
                let couch_warning = (!couch_warnings.is_empty()).then(|| couch_warnings.join("; "));
                self.as_mut().set_couch_theme_message(qstring(
                    couch_warning.clone().unwrap_or_else(|| {
                        format!(
                            "{theme_count} verified Couch Mode theme{} available.",
                            if theme_count == 1 { "" } else { "s" }
                        )
                    }),
                ));
                let (favorite_game_ids, favorite_warning) = match favorites {
                    Ok(favorites) => (favorites, None),
                    Err(error) => (HashSet::new(), Some(error)),
                };
                let favorite_count = catalog
                    .games
                    .iter()
                    .filter(|game| favorite_game_ids.contains(&game.id))
                    .count();
                let (collections, collection_order, collection_warning) = match collections {
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
                let (recent_game_order, completion_states, activity_warning) = match activity {
                    Ok(activity) => {
                        let recent = activity
                            .iter()
                            .filter(|activity| activity.play_count > 0)
                            .map(|activity| (activity.game_uid.clone(), activity.last_played_at))
                            .collect::<HashMap<_, _>>();
                        let completion = activity
                            .into_iter()
                            .map(|activity| (activity.game_uid, activity.completion_state))
                            .collect::<HashMap<_, _>>();
                        (recent, completion, None)
                    }
                    Err(error) => (HashMap::new(), HashMap::new(), Some(error)),
                };
                let (metadata_overrides, metadata_titles, metadata_cooperative, metadata_warning) =
                    match metadata {
                        Ok(metadata) => {
                            let mut titles = HashMap::new();
                            let mut cooperative = HashMap::new();
                            for (game_uid, values) in &metadata {
                                if let Some(title) = &values.title {
                                    titles.insert(game_uid.clone(), title.clone());
                                }
                                if let Some(value) = &values.cooperative {
                                    cooperative.insert(game_uid.clone(), value.clone());
                                }
                            }
                            (metadata, titles, cooperative, None)
                        }
                        Err(error) => (HashMap::new(), HashMap::new(), HashMap::new(), Some(error)),
                    };
                let (tags, game_tags, tag_warning) = match tags {
                    Ok(tags) => (tags.tags, tags.game_tags, None),
                    Err(error) => (Vec::new(), HashMap::new(), Some(error)),
                };
                let (game_custom_fields, custom_field_warning) = match custom_fields {
                    Ok(fields) => (custom_field_search_values(fields), None),
                    Err(error) => (HashMap::new(), Some(error)),
                };
                let recent_count = catalog
                    .games
                    .iter()
                    .filter(|game| recent_game_order.contains_key(&game.id))
                    .count();
                let metadata_titles = Arc::new(metadata_titles);
                let metadata_cooperative = Arc::new(metadata_cooperative);
                let metadata_overrides = Arc::new(metadata_overrides);
                let game_tags = Arc::new(game_tags);
                let game_custom_fields = Arc::new(game_custom_fields);
                let indices = catalog::filter_indices(
                    &catalog,
                    &Filter {
                        display_titles: Arc::clone(&metadata_titles),
                        game_tags: Arc::clone(&game_tags),
                        game_custom_fields: Arc::clone(&game_custom_fields),
                        metadata_overrides: Arc::clone(&metadata_overrides),
                        list_column_filters: Arc::clone(&self.as_ref().rust().list_column_filters),
                        sort_field: self.as_ref().sort_field().to_string(),
                        sort_descending: *self.as_ref().sort_descending(),
                        ..Filter::default()
                    },
                );
                let platform_query = self.as_ref().platform_search().to_string().to_lowercase();
                self.as_mut().begin_reset_model();
                {
                    let mut this = self.as_mut();
                    let mut rust = this.as_mut().rust_mut();
                    let game_index_by_id = catalog
                        .games
                        .iter()
                        .enumerate()
                        .map(|(index, game)| (game.id.clone(), index))
                        .collect::<HashMap<_, _>>();
                    rust.catalog = Arc::new(catalog);
                    rust.filtered_indices = indices;
                    rust.filtered_platform_indices = rust
                        .catalog
                        .platforms
                        .iter()
                        .enumerate()
                        .filter_map(|(index, platform)| {
                            (platform_query.is_empty()
                                || platform.search_key.contains(&platform_query))
                            .then_some(index)
                        })
                        .collect();
                    rust.favorite_game_ids = Arc::new(favorite_game_ids);
                    rust.recent_game_order = Arc::new(recent_game_order);
                    rust.completion_states = Arc::new(completion_states);
                    rust.metadata_titles = metadata_titles;
                    rust.metadata_cooperative = metadata_cooperative;
                    rust.metadata_overrides = metadata_overrides;
                    rust.tags = Arc::new(tags);
                    rust.game_tags = game_tags;
                    rust.game_custom_fields = game_custom_fields;
                    rust.game_index_by_id = Arc::new(game_index_by_id);
                    rust.collections = Arc::new(collections);
                    rust.collection_order = Arc::new(collection_order);
                }
                self.as_mut().rebuild_smart_collections();
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
                let filtered_platform_count = self.as_ref().rust().filtered_platform_indices.len();
                self.as_mut()
                    .set_filtered_platform_count(saturating_i32(filtered_platform_count));
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
                let tag_count = self.as_ref().rust().tags.len();
                self.as_mut().set_tag_count(saturating_i32(tag_count));
                let tag_revision = self.as_ref().tag_revision().wrapping_add(1);
                self.as_mut().set_tag_revision(tag_revision);
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
                let metadata_revision = self.as_ref().metadata_revision().wrapping_add(1);
                self.as_mut().set_metadata_revision(metadata_revision);
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
                if *self.as_ref().sidebar_probe() {
                    println!("LUNCHBOX_SIDEBAR_PROBE_ARMED");
                }
                self.as_mut().set_ready(true);
                let revision = self.as_ref().platform_revision().wrapping_add(1);
                self.as_mut().set_platform_revision(revision);
                let mut status = format!(
                    "Ready — {game_count} games across {platform_count} platforms — {source_label}"
                );
                if let Some(warning) = preference_warning {
                    status.push_str(&format!(" — filter preferences unavailable: {warning}"));
                }
                if let Some(warning) = sidebar_warning {
                    status.push_str(&format!(" — sidebar preferences unavailable: {warning}"));
                }
                if let Some(warning) = couch_warning {
                    status.push_str(&format!(" — Couch Mode preferences unavailable: {warning}"));
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
                if let Some(warning) = metadata_warning {
                    status.push_str(&format!(" — metadata overrides unavailable: {warning}"));
                }
                if let Some(warning) = tag_warning {
                    status.push_str(&format!(" — tags unavailable: {warning}"));
                }
                if let Some(warning) = custom_field_warning {
                    status.push_str(&format!(" — custom fields unavailable: {warning}"));
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

    fn filter_snapshot(&self, search: String, platform: String, availability: String) -> Filter {
        let collection_order = availability
            .strip_prefix("collection:")
            .and_then(|collection_id| self.rust().collection_order.get(collection_id).cloned())
            .unwrap_or_else(|| Arc::new(Vec::new()));
        let collection_game_ids = Arc::new(collection_order.iter().cloned().collect());
        let collection_game_order = Arc::new(
            collection_order
                .iter()
                .enumerate()
                .map(|(index, game_uid)| (game_uid.clone(), index))
                .collect(),
        );
        Filter {
            search,
            platform,
            availability,
            tag: self.tag_filter().to_string(),
            hide_non_retail: *self.hide_non_retail(),
            hide_adult: *self.hide_adult(),
            favorite_game_ids: Arc::clone(&self.rust().favorite_game_ids),
            collection_game_ids,
            collection_game_order,
            recent_game_order: Arc::clone(&self.rust().recent_game_order),
            display_titles: Arc::clone(&self.rust().metadata_titles),
            game_tags: Arc::clone(&self.rust().game_tags),
            game_custom_fields: Arc::clone(&self.rust().game_custom_fields),
            metadata_overrides: Arc::clone(&self.rust().metadata_overrides),
            list_column_filters: Arc::clone(&self.rust().list_column_filters),
            sort_field: self.sort_field().to_string(),
            sort_descending: *self.sort_descending(),
        }
    }

    fn refilter_current_library(mut self: Pin<&mut Self>) {
        if !*self.as_ref().ready() || *self.as_ref().loading() {
            return;
        }
        let search = qstring(&self.as_ref().rust().current_search);
        let platform = self.as_ref().current_platform().clone();
        let availability = self.as_ref().availability_filter().clone();
        self.as_mut().apply_filter(search, platform, availability);
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
        let filter = self.as_ref().filter_snapshot(
            search.to_string(),
            platform.to_string(),
            availability.to_string(),
        );
        self.as_mut().rust_mut().current_search = search.to_string();
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
                if let Err(error) = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_filter(generation, indices);
                }) {
                    eprintln!("Could not publish the filtered catalog to Qt: {error:?}");
                }
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
            view_mode: self.as_ref().view_mode().to_string(),
            sort_field: self.as_ref().sort_field().to_string(),
            sort_descending: *self.as_ref().sort_descending(),
            list_columns: self.as_ref().list_columns().to_string(),
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
            view_mode: self.as_ref().view_mode().to_string(),
            sort_field: self.as_ref().sort_field().to_string(),
            sort_descending: *self.as_ref().sort_descending(),
            list_columns: self.as_ref().list_columns().to_string(),
        };
        if let Err(error) = SettingsStore::open_default()
            .and_then(|store| store.save_library_preferences(&preferences))
        {
            self.as_mut().set_status_message(qstring(format!(
                "View updated, but the preference could not be saved: {error}"
            )));
        }
    }

    pub fn choose_view_mode(mut self: Pin<&mut Self>, view_mode: QString) {
        let view_mode = view_mode.to_string();
        if !matches!(view_mode.as_str(), "grid" | "list") {
            self.as_mut().set_status_message(qstring(format!(
                "Could not apply view mode: unsupported mode {view_mode}"
            )));
            return;
        }
        if self.as_ref().view_mode().to_string() == view_mode {
            return;
        }
        self.as_mut().set_view_mode(qstring(&view_mode));
        let preferences = LibraryPreferences {
            hide_non_retail: *self.as_ref().hide_non_retail(),
            hide_adult: *self.as_ref().hide_adult(),
            artwork_type: self.as_ref().artwork_type().to_string(),
            grid_zoom: *self.as_ref().grid_zoom(),
            view_mode,
            sort_field: self.as_ref().sort_field().to_string(),
            sort_descending: *self.as_ref().sort_descending(),
            list_columns: self.as_ref().list_columns().to_string(),
        };
        if let Err(error) = SettingsStore::open_default()
            .and_then(|store| store.save_library_preferences(&preferences))
        {
            self.as_mut().set_status_message(qstring(format!(
                "View mode changed, but the preference could not be saved: {error}"
            )));
        }
    }

    pub fn set_sort_preferences(mut self: Pin<&mut Self>, sort_field: QString, descending: bool) {
        let sort_field = sort_field.to_string();
        if sort_field != "default" && crate::list_view::ListColumn::parse(&sort_field).is_none() {
            self.as_mut().set_status_message(qstring(format!(
                "Could not sort the library: unsupported field {sort_field}"
            )));
            return;
        }
        if self.as_ref().sort_field().to_string() == sort_field
            && *self.as_ref().sort_descending() == descending
        {
            return;
        }
        self.as_mut().set_sort_field(qstring(&sort_field));
        self.as_mut().set_sort_descending(descending);
        let preferences = LibraryPreferences {
            hide_non_retail: *self.as_ref().hide_non_retail(),
            hide_adult: *self.as_ref().hide_adult(),
            artwork_type: self.as_ref().artwork_type().to_string(),
            grid_zoom: *self.as_ref().grid_zoom(),
            view_mode: self.as_ref().view_mode().to_string(),
            sort_field,
            sort_descending: descending,
            list_columns: self.as_ref().list_columns().to_string(),
        };
        if let Err(error) = SettingsStore::open_default()
            .and_then(|store| store.save_library_preferences(&preferences))
        {
            self.as_mut().set_status_message(qstring(format!(
                "Sort changed, but the preference could not be saved: {error}"
            )));
        }
        if *self.as_ref().ready() && !*self.as_ref().loading() {
            let search = qstring(&self.as_ref().rust().current_search);
            let platform = self.as_ref().current_platform().clone();
            let availability = self.as_ref().availability_filter().clone();
            self.as_mut().apply_filter(search, platform, availability);
        }
    }

    pub fn configure_list_columns(mut self: Pin<&mut Self>, columns: QString) {
        let columns = columns.to_string();
        let parsed = match crate::list_view::parse_list_columns(&columns) {
            Ok(columns) => columns,
            Err(error) => {
                self.as_mut().set_status_message(qstring(format!(
                    "Could not configure list columns: {error}"
                )));
                return;
            }
        };
        let columns = parsed
            .into_iter()
            .map(crate::list_view::ListColumn::key)
            .collect::<Vec<_>>()
            .join(",");
        if self.as_ref().list_columns().to_string() == columns {
            return;
        }
        self.as_mut().set_list_columns(qstring(&columns));
        let preferences = LibraryPreferences {
            hide_non_retail: *self.as_ref().hide_non_retail(),
            hide_adult: *self.as_ref().hide_adult(),
            artwork_type: self.as_ref().artwork_type().to_string(),
            grid_zoom: *self.as_ref().grid_zoom(),
            view_mode: self.as_ref().view_mode().to_string(),
            sort_field: self.as_ref().sort_field().to_string(),
            sort_descending: *self.as_ref().sort_descending(),
            list_columns: columns,
        };
        if let Err(error) = SettingsStore::open_default()
            .and_then(|store| store.save_library_preferences(&preferences))
        {
            self.as_mut().set_status_message(qstring(format!(
                "List columns changed, but the preference could not be saved: {error}"
            )));
        }
    }

    pub fn begin_list_column_filter(mut self: Pin<&mut Self>, column: QString) {
        let column = column.to_string();
        let Some(parsed) = crate::list_view::ListColumn::parse(&column) else {
            self.as_mut().set_status_message(qstring(format!(
                "Could not filter the list: unsupported column {column}"
            )));
            return;
        };
        let draft = self
            .as_ref()
            .rust()
            .list_column_filters
            .get(&parsed)
            .cloned()
            .unwrap_or_else(crate::list_view::ListColumnFilter::all);
        {
            let mut this = self.as_mut();
            let mut rust = this.as_mut().rust_mut();
            rust.list_filter_draft = draft;
            rust.list_filter_values.clear();
        }
        self.as_mut().set_list_filter_column(qstring(parsed.key()));
        self.as_mut().set_list_filter_value_count(0);
        self.as_mut().set_list_filter_total_distinct(0);
        let revision = self.as_ref().list_filter_revision().wrapping_add(1);
        self.as_mut().set_list_filter_revision(revision);
        self.as_mut().search_list_column_filter(QString::default());
    }

    pub fn search_list_column_filter(mut self: Pin<&mut Self>, query: QString) {
        if !*self.as_ref().ready() || *self.as_ref().loading() {
            return;
        }
        let Some(column) =
            crate::list_view::ListColumn::parse(&self.as_ref().list_filter_column().to_string())
        else {
            return;
        };
        self.as_mut().rust_mut().list_filter_generation =
            self.as_ref().rust().list_filter_generation.wrapping_add(1);
        let generation = self.as_ref().rust().list_filter_generation;
        self.as_mut().set_list_filter_loading(true);
        let catalog = Arc::clone(&self.as_ref().rust().catalog);
        let filter = self.as_ref().filter_snapshot(
            self.as_ref().rust().current_search.clone(),
            self.as_ref().current_platform().to_string(),
            self.as_ref().availability_filter().to_string(),
        );
        let query = query.to_string();
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-list-facet".into())
            .spawn(move || {
                let result = catalog::list_facet_values(
                    &catalog,
                    &filter,
                    column,
                    &query,
                    crate::list_view::LIST_FACET_LIMIT,
                );
                if let Err(error) = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_list_facet(generation, result);
                }) {
                    eprintln!("Could not publish list filter values to Qt: {error:?}");
                }
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_list_filter_loading(false);
            self.as_mut().set_status_message(qstring(format!(
                "Could not start list filter values: {error}"
            )));
        }
    }

    pub fn set_list_filter_value_selected(
        mut self: Pin<&mut Self>,
        value: QString,
        selected: bool,
    ) {
        if self.as_ref().list_filter_column().is_empty() {
            return;
        }
        self.as_mut()
            .rust_mut()
            .list_filter_draft
            .set_selected(value.to_string(), selected);
        let revision = self.as_ref().list_filter_revision().wrapping_add(1);
        self.as_mut().set_list_filter_revision(revision);
    }

    pub fn select_all_list_filter_values(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().list_filter_draft = crate::list_view::ListColumnFilter::all();
        let revision = self.as_ref().list_filter_revision().wrapping_add(1);
        self.as_mut().set_list_filter_revision(revision);
    }

    pub fn select_no_list_filter_values(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().list_filter_draft = crate::list_view::ListColumnFilter::none();
        let revision = self.as_ref().list_filter_revision().wrapping_add(1);
        self.as_mut().set_list_filter_revision(revision);
    }

    pub fn apply_list_column_filter(mut self: Pin<&mut Self>) {
        let Some(column) =
            crate::list_view::ListColumn::parse(&self.as_ref().list_filter_column().to_string())
        else {
            return;
        };
        let draft = self.as_ref().rust().list_filter_draft.clone();
        let mut filters = (*self.as_ref().rust().list_column_filters).clone();
        if draft.is_effective() {
            filters.insert(column, draft);
        } else {
            filters.remove(&column);
        }
        self.as_mut().rust_mut().list_column_filters = Arc::new(filters);
        let count = self.as_ref().rust().list_column_filters.len();
        self.as_mut()
            .set_list_filter_active_count(saturating_i32(count));
        let revision = self.as_ref().list_filter_revision().wrapping_add(1);
        self.as_mut().set_list_filter_revision(revision);
        self.as_mut().refilter_current_library();
    }

    pub fn apply_exact_list_column_filter(
        mut self: Pin<&mut Self>,
        column: QString,
        value: QString,
    ) {
        let column = column.to_string();
        let value = value.to_string().trim().to_owned();
        let Some(column) = crate::list_view::ListColumn::parse(&column) else {
            self.as_mut().set_status_message(qstring(format!(
                "Could not apply exact filter: unsupported column {column}"
            )));
            return;
        };
        if value.is_empty() || value.chars().count() > 1_000 || value.contains('\0') {
            self.as_mut()
                .set_status_message(qstring("Could not apply exact filter: invalid value"));
            return;
        }
        let mut selection = crate::list_view::ListColumnFilter::none();
        selection.set_selected(value, true);
        let mut filters = (*self.as_ref().rust().list_column_filters).clone();
        filters.insert(column, selection);
        self.as_mut().rust_mut().list_column_filters = Arc::new(filters);
        let count = self.as_ref().rust().list_column_filters.len();
        self.as_mut()
            .set_list_filter_active_count(saturating_i32(count));
        let revision = self.as_ref().list_filter_revision().wrapping_add(1);
        self.as_mut().set_list_filter_revision(revision);
        self.as_mut().refilter_current_library();
    }

    pub fn clear_all_list_column_filters(mut self: Pin<&mut Self>) {
        if self.as_ref().rust().list_column_filters.is_empty() {
            return;
        }
        self.as_mut().rust_mut().list_column_filters = Arc::new(HashMap::new());
        self.as_mut().set_list_filter_active_count(0);
        let revision = self.as_ref().list_filter_revision().wrapping_add(1);
        self.as_mut().set_list_filter_revision(revision);
        self.as_mut().refilter_current_library();
    }

    pub fn list_column_filter_active(&self, column: QString) -> bool {
        crate::list_view::ListColumn::parse(&column.to_string())
            .is_some_and(|column| self.rust().list_column_filters.contains_key(&column))
    }

    pub fn list_filter_value_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().list_filter_values.get(index))
            .map(|value| qstring(&value.value))
            .unwrap_or_default()
    }

    pub fn list_filter_value_game_count_at(&self, index: i32) -> i32 {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().list_filter_values.get(index))
            .map(|value| saturating_i32(value.game_count))
            .unwrap_or_default()
    }

    pub fn list_filter_value_selected_at(&self, index: i32) -> bool {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().list_filter_values.get(index))
            .is_some_and(|value| self.rust().list_filter_draft.selected(&value.value))
    }

    pub fn list_filter_draft_summary(&self) -> QString {
        qstring(self.rust().list_filter_draft.summary())
    }

    pub fn list_column_filter_summary(&self) -> QString {
        let summaries = crate::list_view::LIST_COLUMN_KEYS
            .into_iter()
            .filter_map(crate::list_view::ListColumn::parse)
            .filter_map(|column| {
                self.rust()
                    .list_column_filters
                    .get(&column)
                    .map(|selection| format!("{}: {}", column.label(), selection.summary()))
            })
            .collect::<Vec<_>>();
        qstring(summaries.join("  ·  "))
    }

    pub fn request_hover_preview(mut self: Pin<&mut Self>, game_uid: QString) {
        let game_uid = game_uid.to_string();
        let Some(game_index) = self
            .as_ref()
            .rust()
            .game_index_by_id
            .get(&game_uid)
            .copied()
        else {
            self.as_mut().cancel_hover_preview();
            self.as_mut().set_hover_preview_message(qstring(
                "Preview unavailable because the game is no longer in this catalog.",
            ));
            return;
        };
        let launchbox_db_id = {
            let this = self.as_ref();
            let Some(game) = this.rust().catalog.games.get(game_index) else {
                self.as_mut().cancel_hover_preview();
                return;
            };
            game.launchbox_db_id
        };
        if *self.as_ref().hover_preview_probe() {
            println!(
                "LUNCHBOX_HOVER_PREVIEW_UI_LOOKUP game_uid={game_uid:?} database_id={launchbox_db_id}"
            );
        }
        if self.as_ref().hover_preview_game_id().to_string() == game_uid
            && (*self.as_ref().hover_preview_loading()
                || !self.as_ref().hover_preview_url().is_empty())
        {
            return;
        }

        self.as_mut().rust_mut().hover_preview_generation = self
            .as_ref()
            .rust()
            .hover_preview_generation
            .wrapping_add(1);
        let generation = self.as_ref().rust().hover_preview_generation;
        self.as_mut().set_hover_preview_game_id(qstring(&game_uid));
        self.as_mut().set_hover_preview_url(QUrl::default());
        self.as_mut().set_hover_preview_source(QString::default());
        self.as_mut()
            .set_hover_preview_message(qstring("Looking for a cached gameplay preview…"));
        self.as_mut().set_hover_preview_loading(true);

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-hover-preview".into())
            .spawn(move || {
                let result =
                    crate::hover_preview::requested_cached_video(&game_uid, launchbox_db_id)
                        .map_err(|error| error.to_string());
                if let Err(error) = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_hover_preview(generation, game_uid, result);
                }) {
                    eprintln!("Could not publish the hover preview to Qt: {error:?}");
                }
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_hover_preview_loading(false);
            self.as_mut().set_hover_preview_message(qstring(format!(
                "Could not inspect cached gameplay media: {error}"
            )));
        }
    }

    fn finish_hover_preview(
        mut self: Pin<&mut Self>,
        generation: u64,
        game_uid: String,
        result: Result<Option<crate::hover_preview::CachedVideoPreview>, String>,
    ) {
        if generation != self.as_ref().rust().hover_preview_generation
            || self.as_ref().hover_preview_game_id().to_string() != game_uid
        {
            if *self.as_ref().hover_preview_probe() {
                println!(
                    "LUNCHBOX_HOVER_PREVIEW_UI_STALE game_uid={game_uid:?} generation={generation}"
                );
            }
            return;
        }
        self.as_mut().set_hover_preview_loading(false);
        match result {
            Ok(Some(preview)) => {
                if *self.as_ref().hover_preview_probe() {
                    println!(
                        "LUNCHBOX_HOVER_PREVIEW_UI_FOUND game_uid={game_uid:?} source={:?} path={:?}",
                        preview.source, preview.path
                    );
                }
                self.as_mut()
                    .set_hover_preview_url(QUrl::from_local_file(&qstring(
                        preview.path.to_string_lossy(),
                    )));
                self.as_mut()
                    .set_hover_preview_source(qstring(&preview.source));
                self.as_mut().set_hover_preview_message(qstring(format!(
                    "Cached gameplay preview from {}.",
                    preview.source
                )));
            }
            Ok(None) => {
                self.as_mut().set_hover_preview_url(QUrl::default());
                self.as_mut().set_hover_preview_source(QString::default());
                self.as_mut().set_hover_preview_message(qstring(
                    "No cached gameplay video is available for this game.",
                ));
            }
            Err(error) => {
                self.as_mut().set_hover_preview_url(QUrl::default());
                self.as_mut().set_hover_preview_source(QString::default());
                self.as_mut().set_hover_preview_message(qstring(format!(
                    "Could not inspect cached gameplay media: {error}"
                )));
            }
        }
    }

    pub fn cancel_hover_preview(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().hover_preview_generation = self
            .as_ref()
            .rust()
            .hover_preview_generation
            .wrapping_add(1);
        self.as_mut().set_hover_preview_game_id(QString::default());
        self.as_mut().set_hover_preview_url(QUrl::default());
        self.as_mut().set_hover_preview_source(QString::default());
        self.as_mut().set_hover_preview_loading(false);
        self.as_mut().set_hover_preview_message(qstring(
            "Pause over a game to preview a cached gameplay video.",
        ));
    }

    pub fn report_hover_preview_ui_probe(&self) {
        if *self.hover_preview_probe() && !self.hover_preview_url().is_empty() {
            println!(
                "LUNCHBOX_HOVER_PREVIEW_UI_READY game_uid={:?} source={:?} url={:?}",
                self.hover_preview_game_id().to_string(),
                self.hover_preview_source().to_string(),
                self.hover_preview_url().to_string()
            );
        }
    }

    pub fn report_hover_preview_ui_failure(&self, detail: QString) {
        if *self.hover_preview_probe() {
            eprintln!("LUNCHBOX_HOVER_PREVIEW_UI_FAILED {}", detail.to_string());
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
            request_id: String::new(),
            database_id,
            title,
            platform,
            requested_kind: kind,
            force,
            exact_only: false,
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

    pub fn row_for_game(&self, game_uid: QString) -> i32 {
        let game_uid = game_uid.to_string();
        let Some(catalog_index) = self.rust().game_index_by_id.get(&game_uid) else {
            return -1;
        };
        self.rust()
            .filtered_indices
            .iter()
            .position(|index| index == catalog_index)
            .map(saturating_i32)
            .unwrap_or(-1)
    }

    pub fn canonical_title_for_game(&self, game_uid: QString) -> QString {
        let game_uid = game_uid.to_string();
        self.rust()
            .game_index_by_id
            .get(&game_uid)
            .and_then(|index| self.rust().catalog.games.get(*index))
            .map(|game| qstring(&game.title))
            .unwrap_or_default()
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
        let updates_smart_collection = self.as_ref().rust().collections.iter().any(|collection| {
            collection
                .rules
                .as_ref()
                .is_some_and(|rules| rules.favorite != "any")
        });
        if updates_smart_collection {
            self.as_mut().rebuild_smart_collections();
            self.as_mut().bump_collection_revision();
            self.as_mut().refresh_active_collection_filter();
        }
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

    pub fn collection_kind_at(&self, index: i32) -> QString {
        collection_at(self, index)
            .map(|collection| qstring(&collection.kind))
            .unwrap_or_default()
    }

    pub fn collection_rule_summary_at(&self, index: i32) -> QString {
        collection_at(self, index)
            .and_then(|collection| collection.rules.as_ref())
            .map(|rules| qstring(rules.summary()))
            .unwrap_or_default()
    }

    pub fn collection_rule_at(&self, index: i32, field: QString) -> QString {
        let Some(rules) =
            collection_at(self, index).and_then(|collection| collection.rules.as_ref())
        else {
            return QString::default();
        };
        qstring(match field.to_string().as_str() {
            "title" => &rules.title_contains,
            "platform" => &rules.platform,
            "tag" => &rules.tag,
            "availability" => &rules.availability,
            "favorite" => &rules.favorite,
            "completion" => &rules.completion_state,
            "content" => &rules.content,
            "cooperative" => &rules.cooperative,
            _ => "",
        })
    }

    pub fn collection_member_game_uid_at(
        &self,
        collection_index: i32,
        member_index: i32,
    ) -> QString {
        collection_member_game(self, collection_index, member_index)
            .map(|game| qstring(&game.id))
            .unwrap_or_default()
    }

    pub fn collection_member_name_at(&self, collection_index: i32, member_index: i32) -> QString {
        collection_member_game(self, collection_index, member_index)
            .map(|game| {
                qstring(
                    self.rust()
                        .metadata_titles
                        .get(&game.id)
                        .unwrap_or(&game.title),
                )
            })
            .unwrap_or_default()
    }

    pub fn collection_member_platform_at(
        &self,
        collection_index: i32,
        member_index: i32,
    ) -> QString {
        collection_member_game(self, collection_index, member_index)
            .map(|game| qstring(&game.platform))
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

    pub fn set_smart_collection_rule_draft(
        mut self: Pin<&mut Self>,
        title_contains: QString,
        platform: QString,
        tag: QString,
        availability: QString,
        favorite: QString,
        completion_state: QString,
        content: QString,
        cooperative: QString,
    ) -> bool {
        match smart_rules_from_fields(
            title_contains,
            platform,
            tag,
            availability,
            favorite,
            completion_state,
            content,
            cooperative,
        ) {
            Ok(rules) => {
                self.as_mut().rust_mut().smart_collection_rule_draft = Some(rules);
                true
            }
            Err(error) => {
                self.as_mut().rust_mut().smart_collection_rule_draft = None;
                self.as_mut()
                    .set_collection_message(qstring(format!("Smart rules are invalid: {error}")));
                false
            }
        }
    }

    pub fn create_smart_collection(mut self: Pin<&mut Self>, name: QString, description: QString) {
        if !self.as_ref().can_change_collections() {
            return;
        }
        let Some(rules) = self.as_ref().rust().smart_collection_rule_draft.clone() else {
            self.as_mut()
                .set_collection_message(qstring("Configure at least one smart rule first."));
            return;
        };
        let name = name.to_string();
        let description = description.to_string();
        self.as_mut()
            .start_collection_task("lunchbox-smart-collection-create", None, move || {
                SettingsStore::open_default()
                    .and_then(|store| store.create_smart_collection(&name, &description, rules))
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
                        rules: None,
                    })
                    .map_err(|error| error.to_string())
            });
    }

    pub fn update_smart_collection(
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
            .any(|collection| collection.id == collection_id && collection.kind == "smart")
        {
            self.as_mut()
                .set_collection_message(qstring("Smart collection no longer exists."));
            return;
        }
        let Some(rules) = self.as_ref().rust().smart_collection_rule_draft.clone() else {
            self.as_mut()
                .set_collection_message(qstring("Configure at least one smart rule first."));
            return;
        };
        let name = name.to_string();
        let description = description.to_string();
        let task_collection_id = collection_id.clone();
        let task_rules = rules.clone();
        self.as_mut()
            .start_collection_task("lunchbox-smart-collection-update", None, move || {
                SettingsStore::open_default()
                    .and_then(|store| {
                        store.update_smart_collection(
                            &task_collection_id,
                            &name,
                            &description,
                            task_rules,
                        )
                    })
                    .map(|()| CollectionMutation::Updated {
                        collection_id: task_collection_id,
                        name: name.trim().to_owned(),
                        description: description.trim().to_owned(),
                        rules: Some(rules),
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
            position: None,
        };
        let collection_exists = self.as_ref().rust().collections.iter().any(|collection| {
            collection.id == membership.collection_id && collection.kind == "manual"
        });
        let game_exists = self
            .as_ref()
            .rust()
            .catalog
            .games
            .iter()
            .any(|game| game.id == membership.game_uid);
        if !collection_exists || !game_exists {
            self.as_mut().set_collection_message(qstring(
                "Membership was not changed because the manual collection or game identity is unavailable.",
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

        let mut membership = membership;
        if !member {
            membership.position = self
                .as_ref()
                .rust()
                .collection_order
                .get(&membership.collection_id)
                .and_then(|order| {
                    order
                        .iter()
                        .position(|game_uid| game_uid == &membership.game_uid)
                });
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

    pub fn move_collection_game(
        mut self: Pin<&mut Self>,
        collection_id: QString,
        member_index: i32,
        new_index: i32,
    ) {
        if !self.as_ref().can_change_collections() {
            return;
        }
        let collection_id = collection_id.to_string();
        let order = self
            .as_ref()
            .rust()
            .collection_order
            .get(&collection_id)
            .cloned();
        let Some(order) = order else {
            return;
        };
        let Some(game_uid) = usize::try_from(member_index)
            .ok()
            .and_then(|index| order.get(index))
            .cloned()
        else {
            return;
        };
        let Ok(new_index) = usize::try_from(new_index) else {
            return;
        };
        if new_index >= order.len() {
            return;
        }
        self.as_mut()
            .start_collection_task("lunchbox-collection-reorder", None, move || {
                let store = SettingsStore::open_default().map_err(|error| error.to_string())?;
                store
                    .move_collection_game(&collection_id, &game_uid, new_index)
                    .map_err(|error| error.to_string())?;
                let collections = store
                    .user_collections()
                    .map_err(|error| error.to_string())?;
                Ok(CollectionMutation::Reloaded {
                    collections,
                    message: "Collection order saved.".to_owned(),
                })
            });
    }

    pub fn bulk_set_visible_collection_membership(
        mut self: Pin<&mut Self>,
        collection_id: QString,
        member: bool,
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
            .any(|collection| collection.id == collection_id && collection.kind == "manual")
        {
            self.as_mut().set_collection_message(qstring(
                "Bulk membership applies only to manual collections.",
            ));
            return;
        }
        let catalog = Arc::clone(&self.as_ref().rust().catalog);
        let filtered_indices = self.as_ref().rust().filtered_indices.clone();
        let game_uids = filtered_indices
            .iter()
            .filter_map(|index| catalog.games.get(*index))
            .map(|game| game.id.clone())
            .collect::<Vec<_>>();
        if game_uids.is_empty() {
            self.as_mut()
                .set_collection_message(qstring("There are no visible games to change."));
            return;
        }
        self.as_mut()
            .start_collection_task("lunchbox-collection-bulk", None, move || {
                let store = SettingsStore::open_default().map_err(|error| error.to_string())?;
                let changed = store
                    .set_collection_memberships(&collection_id, &game_uids, member)
                    .map_err(|error| error.to_string())?;
                let collections = store
                    .user_collections()
                    .map_err(|error| error.to_string())?;
                Ok(CollectionMutation::Reloaded {
                    collections,
                    message: format!(
                        "{} {changed} visible games {} the collection.",
                        if member { "Added" } else { "Removed" },
                        if member { "to" } else { "from" }
                    ),
                })
            });
    }

    pub fn export_collection(mut self: Pin<&mut Self>, collection_id: QString, destination: QUrl) {
        if !self.as_ref().can_change_collections() {
            return;
        }
        let Some(path) = destination
            .to_local_file()
            .map(|path| std::path::PathBuf::from(path.to_string()))
        else {
            self.as_mut()
                .set_collection_message(qstring("Choose a local collection export file."));
            return;
        };
        let collection_id = collection_id.to_string();
        let Some(collection) = self
            .as_ref()
            .rust()
            .collections
            .iter()
            .find(|collection| collection.id == collection_id)
            .cloned()
        else {
            self.as_mut()
                .set_collection_message(qstring("Collection no longer exists."));
            return;
        };
        let catalog = Arc::clone(&self.as_ref().rust().catalog);
        let collection_order = Arc::clone(&self.as_ref().rust().collection_order);
        let game_index_by_id = Arc::clone(&self.as_ref().rust().game_index_by_id);
        let games = if collection.kind == "smart" {
            Vec::new()
        } else {
            collection_order
                .get(&collection_id)
                .into_iter()
                .flat_map(|order| order.iter())
                .filter_map(|game_uid| {
                    let index = *game_index_by_id.get(game_uid)?;
                    catalog.games.get(index)
                })
                .map(|game| PortableGameReference {
                    game_uid: game.id.clone(),
                    launchbox_db_id: game.launchbox_db_id,
                    title: game.title.clone(),
                    platform: game.platform.clone(),
                })
                .collect::<Vec<_>>()
        };
        let portable = match PortableCollection::new(
            collection.name.clone(),
            collection.description,
            collection.kind,
            collection.rules,
            games,
        ) {
            Ok(portable) => portable,
            Err(error) => {
                self.as_mut().set_collection_message(qstring(format!(
                    "Could not prepare collection export: {error}"
                )));
                return;
            }
        };
        let name = collection.name;
        self.as_mut()
            .start_collection_task("lunchbox-collection-export", None, move || {
                let path = save_portable_collection(&path, &portable)
                    .map_err(|error| error.to_string())?;
                Ok(CollectionMutation::Message(format!(
                    "Exported {name} to {}.",
                    path.display()
                )))
            });
    }

    pub fn import_collection(mut self: Pin<&mut Self>, source: QUrl) {
        if !self.as_ref().can_change_collections() {
            return;
        }
        let Some(path) = source
            .to_local_file()
            .map(|path| std::path::PathBuf::from(path.to_string()))
        else {
            self.as_mut()
                .set_collection_message(qstring("Choose a local Lunchbox collection file."));
            return;
        };
        let catalog = Arc::clone(&self.as_ref().rust().catalog);
        self.as_mut()
            .start_collection_task("lunchbox-collection-import", None, move || {
                let portable =
                    load_portable_collection(&path).map_err(|error| error.to_string())?;
                let store = SettingsStore::open_default().map_err(|error| error.to_string())?;
                let (collection, message) = if portable.kind == "smart" {
                    let collection = store
                        .create_smart_collection(
                            &portable.name,
                            &portable.description,
                            portable
                                .rules
                                .ok_or_else(|| "smart collection rules are missing".to_owned())?,
                        )
                        .map_err(|error| error.to_string())?;
                    let name = collection.name.clone();
                    (collection, format!("Imported smart collection {name}."))
                } else {
                    let (members, unavailable) =
                        resolve_portable_game_references(&portable.games, &catalog.games);
                    let collection = store
                        .create_collection(&portable.name, &portable.description)
                        .map_err(|error| error.to_string())?;
                    if let Err(error) = store.replace_collection_members(&collection.id, &members) {
                        let _ = store.delete_collection(&collection.id);
                        return Err(error.to_string());
                    }
                    let name = collection.name.clone();
                    (
                        collection,
                        format!(
                            "Imported {name}: {} exact games; {unavailable} unavailable.",
                            members.len()
                        ),
                    )
                };
                let collections = store
                    .user_collections()
                    .map_err(|error| error.to_string())?;
                if !collections
                    .collections
                    .iter()
                    .any(|candidate| candidate.id == collection.id)
                {
                    return Err("imported collection could not be reloaded".to_owned());
                }
                Ok(CollectionMutation::Reloaded {
                    collections,
                    message,
                })
            });
    }

    fn rebuild_smart_collections(mut self: Pin<&mut Self>) {
        let catalog = Arc::clone(&self.as_ref().rust().catalog);
        let favorites = Arc::clone(&self.as_ref().rust().favorite_game_ids);
        let completion_states = Arc::clone(&self.as_ref().rust().completion_states);
        let metadata_titles = Arc::clone(&self.as_ref().rust().metadata_titles);
        let metadata_cooperative = Arc::clone(&self.as_ref().rust().metadata_cooperative);
        let game_tags = Arc::clone(&self.as_ref().rust().game_tags);
        let game_index_by_id = Arc::clone(&self.as_ref().rust().game_index_by_id);
        let mut collections = self.as_ref().rust().collections.as_ref().clone();
        let mut order = self.as_ref().rust().collection_order.as_ref().clone();

        for collection in &mut collections {
            let members = if collection.kind == "smart" {
                collection
                    .rules
                    .as_ref()
                    .map(|rules| {
                        let title_needle = rules.title_contains.to_lowercase();
                        catalog
                            .games
                            .iter()
                            .filter(|game| {
                                rules.matches_with_display_title(
                                    game,
                                    &favorites,
                                    &completion_states,
                                    &title_needle,
                                    metadata_display_title(game, &metadata_titles),
                                    game_tags.get(&game.id).map(Vec::as_slice).unwrap_or(&[]),
                                    effective_cooperative(game, &metadata_cooperative),
                                )
                            })
                            .map(|game| game.id.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                order
                    .get(&collection.id)
                    .map(|members| {
                        members
                            .iter()
                            .filter(|game_uid| game_index_by_id.contains_key(*game_uid))
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            };
            collection.game_count = members.len();
            order.insert(collection.id.clone(), Arc::new(members));
        }

        let members = order
            .iter()
            .map(|(collection_id, order)| {
                (
                    collection_id.clone(),
                    Arc::new(order.iter().cloned().collect::<HashSet<_>>()),
                )
            })
            .collect::<HashMap<_, _>>();
        let mut this = self.as_mut();
        let mut rust = this.as_mut().rust_mut();
        rust.collections = Arc::new(collections);
        rust.collection_order = Arc::new(order);
        rust.collection_members = Arc::new(members);
    }

    fn replace_collection_state(
        mut self: Pin<&mut Self>,
        collections: UserCollections,
        message: String,
    ) {
        let order = collections
            .members
            .into_iter()
            .map(|(collection_id, game_uids)| (collection_id, Arc::new(game_uids)))
            .collect::<HashMap<_, _>>();
        {
            let mut this = self.as_mut();
            let mut rust = this.as_mut().rust_mut();
            rust.collections = Arc::new(collections.collections);
            rust.collection_order = Arc::new(order);
        }
        self.as_mut().rebuild_smart_collections();
        let count = self.as_ref().rust().collections.len();
        self.as_mut().set_collection_count(saturating_i32(count));
        self.as_mut().set_collection_message(qstring(message));
        self.as_mut().bump_collection_revision();
        self.as_mut().refresh_active_collection_filter();
    }

    fn refresh_active_collection_filter(mut self: Pin<&mut Self>) {
        if !*self.as_ref().ready() || *self.as_ref().loading() {
            return;
        }
        let availability = self.as_ref().availability_filter().to_string();
        let Some(collection_id) = availability.strip_prefix("collection:") else {
            return;
        };
        let availability = if self
            .as_ref()
            .rust()
            .collections
            .iter()
            .any(|collection| collection.id == collection_id)
        {
            availability
        } else {
            String::new()
        };
        let search = self.as_ref().rust().current_search.clone();
        let platform = self.as_ref().current_platform().to_string();
        self.as_mut()
            .apply_filter(qstring(search), qstring(platform), qstring(availability));
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
            let changed = if membership.member {
                Arc::make_mut(members).insert(membership.game_uid.clone())
            } else {
                Arc::make_mut(members).remove(&membership.game_uid)
            };
            if changed {
                let order = Arc::make_mut(&mut rust.collection_order)
                    .entry(membership.collection_id.clone())
                    .or_insert_with(|| Arc::new(Vec::new()));
                let order = Arc::make_mut(order);
                if membership.member {
                    let position = membership.position.unwrap_or(order.len()).min(order.len());
                    order.insert(position, membership.game_uid.clone());
                } else {
                    order.retain(|game_uid| game_uid != &membership.game_uid);
                }
            }
            changed
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
        self.as_mut().refresh_active_collection_filter();
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
                        .entry(collection_id.clone())
                        .or_insert_with(|| Arc::new(HashSet::new()));
                    Arc::make_mut(&mut rust.collection_order)
                        .entry(collection_id)
                        .or_insert_with(|| Arc::new(Vec::new()));
                    rust.collections.len()
                };
                self.as_mut().rebuild_smart_collections();
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
                rules,
            }) => {
                if let Some(collection) = Arc::make_mut(&mut self.as_mut().rust_mut().collections)
                    .iter_mut()
                    .find(|collection| collection.id == collection_id)
                {
                    collection.name = name.clone();
                    collection.description = description;
                    if rules.is_some() {
                        collection.rules = rules;
                    }
                }
                {
                    let mut this = self.as_mut();
                    let mut rust = this.as_mut().rust_mut();
                    let collections: &mut Vec<UserCollection> =
                        Arc::make_mut(&mut rust.collections);
                    sort_collections(collections);
                }
                self.as_mut().rebuild_smart_collections();
                self.as_mut()
                    .set_collection_message(qstring(format!("Updated {name}.")));
                self.as_mut().bump_collection_revision();
                self.as_mut().refresh_active_collection_filter();
            }
            Ok(CollectionMutation::Deleted { collection_id }) => {
                let collection_count = {
                    let mut this = self.as_mut();
                    let mut rust = this.as_mut().rust_mut();
                    Arc::make_mut(&mut rust.collections)
                        .retain(|collection| collection.id != collection_id);
                    Arc::make_mut(&mut rust.collection_members).remove(&collection_id);
                    Arc::make_mut(&mut rust.collection_order).remove(&collection_id);
                    rust.collections.len()
                };
                self.as_mut()
                    .set_collection_count(saturating_i32(collection_count));
                self.as_mut()
                    .set_collection_message(qstring("Collection deleted."));
                self.as_mut().bump_collection_revision();
                self.as_mut().refresh_active_collection_filter();
            }
            Ok(CollectionMutation::Membership(membership)) => {
                self.as_mut()
                    .set_collection_message(qstring(if membership.member {
                        "Added game to collection."
                    } else {
                        "Removed game from collection."
                    }));
            }
            Ok(CollectionMutation::Reloaded {
                collections,
                message,
            }) => self.as_mut().replace_collection_state(collections, message),
            Ok(CollectionMutation::Message(message)) => {
                self.as_mut().set_collection_message(qstring(message));
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

    fn finish_list_facet(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: catalog::ListFacetResult,
    ) {
        if generation != self.as_ref().rust().list_filter_generation {
            return;
        }
        let count = result.values.len();
        self.as_mut().rust_mut().list_filter_values = result.values;
        self.as_mut()
            .set_list_filter_value_count(saturating_i32(count));
        self.as_mut()
            .set_list_filter_total_distinct(saturating_i32(result.total_distinct));
        self.as_mut().set_list_filter_loading(false);
        let revision = self.as_ref().list_filter_revision().wrapping_add(1);
        self.as_mut().set_list_filter_revision(revision);
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

    pub fn filtered_platform_name_at(&self, index: i32) -> QString {
        filtered_platform(self, index)
            .map(|platform| qstring(&platform.name))
            .unwrap_or_default()
    }

    pub fn filtered_platform_game_count_at(&self, index: i32) -> i32 {
        filtered_platform(self, index)
            .map(|platform| saturating_i32(platform.game_count))
            .unwrap_or_default()
    }

    pub fn filter_platforms(mut self: Pin<&mut Self>, query: QString) {
        let query = query.to_string().chars().take(80).collect::<String>();
        if self.as_ref().platform_search().to_string() == query {
            return;
        }
        self.as_mut().set_platform_search(qstring(&query));
        self.as_mut().rebuild_filtered_platforms();
    }

    pub fn preview_sidebar_width(mut self: Pin<&mut Self>, width: i32) {
        let width = width.clamp(180, 400);
        if width != *self.as_ref().sidebar_width() {
            self.as_mut().set_sidebar_width(width);
        }
    }

    pub fn save_sidebar_state(mut self: Pin<&mut Self>, query: QString, width: i32) {
        self.as_mut().filter_platforms(query);
        self.as_mut().preview_sidebar_width(width);
        self.as_mut().rust_mut().sidebar_save_generation =
            self.as_ref().rust().sidebar_save_generation.wrapping_add(1);
        if !self.as_ref().rust().sidebar_save_pending {
            self.as_mut().start_sidebar_save();
        }
    }

    fn rebuild_filtered_platforms(mut self: Pin<&mut Self>) {
        let query = self
            .as_ref()
            .platform_search()
            .to_string()
            .trim()
            .to_lowercase();
        let indices = self
            .as_ref()
            .rust()
            .catalog
            .platforms
            .iter()
            .enumerate()
            .filter_map(|(index, platform)| {
                (query.is_empty() || platform.search_key.contains(&query)).then_some(index)
            })
            .collect::<Vec<_>>();
        let count = indices.len();
        self.as_mut().rust_mut().filtered_platform_indices = indices;
        self.as_mut()
            .set_filtered_platform_count(saturating_i32(count));
        let revision = self.as_ref().platform_revision().wrapping_add(1);
        self.as_mut().set_platform_revision(revision);
    }

    fn start_sidebar_save(mut self: Pin<&mut Self>) {
        let generation = self.as_ref().rust().sidebar_save_generation;
        let preferences = SidebarPreferences {
            platform_search: self.as_ref().platform_search().to_string(),
            width: *self.as_ref().sidebar_width(),
        };
        self.as_mut().rust_mut().sidebar_save_pending = true;
        self.as_mut().set_sidebar_state_saving(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-sidebar-save".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| store.save_sidebar_preferences(&preferences))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_sidebar_save(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().rust_mut().sidebar_save_pending = false;
            self.as_mut().set_sidebar_state_saving(false);
            self.as_mut().set_status_message(qstring(format!(
                "Sidebar updated, but its state could not be saved: {error}"
            )));
        }
    }

    fn finish_sidebar_save(mut self: Pin<&mut Self>, generation: u64, result: Result<(), String>) {
        self.as_mut().rust_mut().sidebar_save_pending = false;
        if let Err(error) = result {
            self.as_mut().set_status_message(qstring(format!(
                "Sidebar updated, but its state could not be saved: {error}"
            )));
        }
        if generation != self.as_ref().rust().sidebar_save_generation {
            self.as_mut().start_sidebar_save();
        } else {
            self.as_mut().set_sidebar_state_saving(false);
        }
    }

    pub fn save_couch_state(mut self: Pin<&mut Self>, shelf: QString, platform: QString) -> bool {
        let preferences = CouchModePreferences {
            shelf: shelf.to_string(),
            platform: platform.to_string(),
            theme_id: self.as_ref().couch_theme_id().to_string(),
            attract_enabled: *self.as_ref().couch_attract_enabled(),
            attract_idle_seconds: *self.as_ref().couch_attract_idle_seconds(),
            attract_cycle_seconds: *self.as_ref().couch_attract_cycle_seconds(),
        };
        if let Err(error) = preferences.validate() {
            self.as_mut().set_status_message(qstring(format!(
                "Couch Mode navigation was not saved: {error}"
            )));
            return false;
        }
        if preferences.shelf == "platform"
            && !self
                .as_ref()
                .rust()
                .catalog
                .platforms
                .iter()
                .any(|candidate| candidate.name == preferences.platform)
        {
            self.as_mut().set_status_message(qstring(format!(
                "Couch Mode navigation was not saved: platform {} is not in the catalog",
                preferences.platform
            )));
            return false;
        }

        self.as_mut().set_couch_shelf(qstring(&preferences.shelf));
        self.as_mut()
            .set_couch_platform(qstring(&preferences.platform));
        self.as_mut().rust_mut().couch_save_generation =
            self.as_ref().rust().couch_save_generation.wrapping_add(1);
        if !self.as_ref().rust().couch_save_pending {
            self.as_mut().start_couch_save();
        }
        true
    }

    pub fn save_couch_attract_settings(
        mut self: Pin<&mut Self>,
        enabled: bool,
        idle_seconds: i32,
        cycle_seconds: i32,
    ) -> bool {
        let preferences = CouchModePreferences {
            shelf: self.as_ref().couch_shelf().to_string(),
            platform: self.as_ref().couch_platform().to_string(),
            theme_id: self.as_ref().couch_theme_id().to_string(),
            attract_enabled: enabled,
            attract_idle_seconds: idle_seconds,
            attract_cycle_seconds: cycle_seconds,
        };
        if let Err(error) = preferences.validate() {
            self.as_mut().set_status_message(qstring(format!(
                "Couch Mode attract settings were not saved: {error}"
            )));
            return false;
        }

        self.as_mut().set_couch_attract_enabled(enabled);
        self.as_mut().set_couch_attract_idle_seconds(idle_seconds);
        self.as_mut().set_couch_attract_cycle_seconds(cycle_seconds);
        self.as_mut().rust_mut().couch_save_generation =
            self.as_ref().rust().couch_save_generation.wrapping_add(1);
        if !self.as_ref().rust().couch_save_pending {
            self.as_mut().start_couch_save();
        }
        true
    }

    pub fn couch_theme_id_at(&self, index: i32) -> QString {
        self.couch_theme(index)
            .map(|theme| qstring(&theme.id))
            .unwrap_or_default()
    }

    pub fn couch_theme_name_at(&self, index: i32) -> QString {
        self.couch_theme(index)
            .map(|theme| qstring(&theme.name))
            .unwrap_or_default()
    }

    pub fn couch_theme_author_at(&self, index: i32) -> QString {
        self.couch_theme(index)
            .map(|theme| qstring(&theme.author))
            .unwrap_or_default()
    }

    pub fn couch_theme_description_at(&self, index: i32) -> QString {
        self.couch_theme(index)
            .map(|theme| qstring(&theme.description))
            .unwrap_or_default()
    }

    pub fn couch_theme_installed_at(&self, index: i32) -> bool {
        self.couch_theme(index).is_some_and(|theme| theme.installed)
    }

    pub fn couch_theme_background_at(&self, index: i32) -> QString {
        self.couch_theme(index)
            .map(|theme| qstring(&theme.background))
            .unwrap_or_default()
    }

    pub fn couch_theme_accent_at(&self, index: i32) -> QString {
        self.couch_theme(index)
            .map(|theme| qstring(&theme.accent))
            .unwrap_or_default()
    }

    pub fn couch_theme_ink_at(&self, index: i32) -> QString {
        self.couch_theme(index)
            .map(|theme| qstring(&theme.ink))
            .unwrap_or_default()
    }

    pub fn couch_theme_muted_at(&self, index: i32) -> QString {
        self.couch_theme(index)
            .map(|theme| qstring(&theme.muted))
            .unwrap_or_default()
    }

    pub fn select_couch_theme(mut self: Pin<&mut Self>, index: i32) -> bool {
        if *self.as_ref().couch_theme_busy() {
            return false;
        }
        let Some(theme) = self.as_ref().couch_theme(index).cloned() else {
            self.as_mut()
                .set_couch_theme_message(qstring("The selected Couch Mode theme is unavailable."));
            return false;
        };
        self.as_mut().apply_couch_theme(theme.clone());
        self.as_mut().set_couch_theme_message(qstring(format!(
            "Using {} by {}. The selection is saved automatically.",
            theme.name, theme.author
        )));
        self.as_mut().rust_mut().couch_save_generation =
            self.as_ref().rust().couch_save_generation.wrapping_add(1);
        if !self.as_ref().rust().couch_save_pending {
            self.as_mut().start_couch_save();
        }
        true
    }

    pub fn install_couch_theme(mut self: Pin<&mut Self>, package: QUrl) {
        if *self.as_ref().couch_theme_busy() {
            return;
        }
        let Some(package) = package
            .to_local_file()
            .map(|path| PathBuf::from(path.to_string()))
        else {
            self.as_mut().set_couch_theme_message(qstring(
                "Choose a local .lunchbox-theme package to install.",
            ));
            return;
        };
        self.as_mut().rust_mut().couch_theme_generation =
            self.as_ref().rust().couch_theme_generation.wrapping_add(1);
        let generation = self.as_ref().rust().couch_theme_generation;
        self.as_mut().set_couch_theme_busy(true);
        self.as_mut().set_couch_theme_message(qstring(
            "Validating and installing the declarative theme package…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-couch-theme-install".into())
            .spawn(move || {
                let result = (|| -> anyhow::Result<CouchThemeUpdate> {
                    let state_database = crate::settings::state_database_path()?;
                    let installed = crate::couch_theme::install_package(&package, &state_database)?;
                    let selected_id = installed.theme.id.clone();
                    let theme_name = installed.theme.name.clone();
                    let catalog = crate::couch_theme::catalog_for_state(&state_database);
                    Ok(CouchThemeUpdate {
                        catalog,
                        selected_id,
                        persist_selection: true,
                        message: if installed.reused {
                            format!(
                                "{theme_name} was already verified; selected it for Couch Mode."
                            )
                        } else {
                            format!("Installed and selected {theme_name} for Couch Mode.")
                        },
                    })
                })()
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_couch_theme_update(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_couch_theme_busy(false);
            self.as_mut().set_couch_theme_message(qstring(format!(
                "Could not start theme installation: {error}"
            )));
        }
    }

    pub fn remove_couch_theme(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().couch_theme_busy() {
            return;
        }
        let Some(theme) = self.as_ref().couch_theme(index).cloned() else {
            self.as_mut()
                .set_couch_theme_message(qstring("The selected Couch Mode theme is unavailable."));
            return;
        };
        if !theme.installed {
            self.as_mut()
                .set_couch_theme_message(qstring("Built-in Couch Mode themes cannot be removed."));
            return;
        }
        let current_id = self.as_ref().couch_theme_id().to_string();
        self.as_mut().rust_mut().couch_theme_generation =
            self.as_ref().rust().couch_theme_generation.wrapping_add(1);
        let generation = self.as_ref().rust().couch_theme_generation;
        self.as_mut().set_couch_theme_busy(true);
        self.as_mut()
            .set_couch_theme_message(qstring(format!("Removing {} safely…", theme.name)));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-couch-theme-remove".into())
            .spawn(move || {
                let result = (|| -> anyhow::Result<CouchThemeUpdate> {
                    let state_database = crate::settings::state_database_path()?;
                    crate::couch_theme::remove_installed_theme(&theme.id, &state_database)?;
                    let selected_was_removed = current_id == theme.id;
                    let selected_id = if selected_was_removed {
                        crate::couch_theme::DEFAULT_THEME_ID.to_owned()
                    } else {
                        current_id
                    };
                    Ok(CouchThemeUpdate {
                        catalog: crate::couch_theme::catalog_for_state(&state_database),
                        selected_id,
                        persist_selection: selected_was_removed,
                        message: format!(
                            "Removed {}{}.",
                            theme.name,
                            if selected_was_removed {
                                " and restored the built-in Lunchbox theme"
                            } else {
                                ""
                            }
                        ),
                    })
                })()
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_couch_theme_update(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_couch_theme_busy(false);
            self.as_mut().set_couch_theme_message(qstring(format!(
                "Could not start theme removal: {error}"
            )));
        }
    }

    pub fn refresh_couch_themes(mut self: Pin<&mut Self>) {
        if *self.as_ref().couch_theme_busy() {
            return;
        }
        let selected_id = self.as_ref().couch_theme_id().to_string();
        self.as_mut().rust_mut().couch_theme_generation =
            self.as_ref().rust().couch_theme_generation.wrapping_add(1);
        let generation = self.as_ref().rust().couch_theme_generation;
        self.as_mut().set_couch_theme_busy(true);
        self.as_mut()
            .set_couch_theme_message(qstring("Refreshing verified Couch Mode themes…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-couch-theme-refresh".into())
            .spawn(move || {
                let result = (|| -> anyhow::Result<CouchThemeUpdate> {
                    let state_database = crate::settings::state_database_path()?;
                    let catalog = crate::couch_theme::catalog_for_state(&state_database);
                    let selected_exists = catalog
                        .themes
                        .iter()
                        .any(|theme| theme.id == selected_id);
                    Ok(CouchThemeUpdate {
                        catalog,
                        selected_id: if selected_exists {
                            selected_id
                        } else {
                            crate::couch_theme::DEFAULT_THEME_ID.to_owned()
                        },
                        persist_selection: !selected_exists,
                        message: if selected_exists {
                            "Refreshed verified Couch Mode themes.".to_owned()
                        } else {
                            "The selected package is no longer installed; restored the built-in Lunchbox theme.".to_owned()
                        },
                    })
                })()
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_couch_theme_update(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_couch_theme_busy(false);
            self.as_mut().set_couch_theme_message(qstring(format!(
                "Could not start theme refresh: {error}"
            )));
        }
    }

    fn finish_couch_theme_update(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<CouchThemeUpdate, String>,
    ) {
        if generation != self.as_ref().rust().couch_theme_generation {
            return;
        }
        self.as_mut().set_couch_theme_busy(false);
        match result {
            Ok(update) => {
                let warning = (!update.catalog.warnings.is_empty())
                    .then(|| update.catalog.warnings.join("; "));
                let previous_selected = self.as_ref().couch_theme_id().to_string();
                self.as_mut().rust_mut().couch_themes = update.catalog.themes;
                let count = self.as_ref().rust().couch_themes.len();
                self.as_mut().set_couch_theme_count(saturating_i32(count));
                let selected = if self.as_mut().apply_couch_theme_id(&update.selected_id) {
                    update.selected_id
                } else {
                    let fallback = crate::couch_theme::DEFAULT_THEME_ID.to_owned();
                    let _ = self.as_mut().apply_couch_theme_id(&fallback);
                    fallback
                };
                self.as_mut()
                    .set_couch_theme_message(qstring(match warning {
                        Some(warning) => {
                            format!("{} Some packages were ignored: {warning}", update.message)
                        }
                        None => update.message,
                    }));
                if update.persist_selection || selected != previous_selected {
                    self.as_mut().rust_mut().couch_save_generation =
                        self.as_ref().rust().couch_save_generation.wrapping_add(1);
                    if !self.as_ref().rust().couch_save_pending {
                        self.as_mut().start_couch_save();
                    }
                }
            }
            Err(error) => self
                .as_mut()
                .set_couch_theme_message(qstring(format!("Theme operation failed: {error}"))),
        }
    }

    fn couch_theme(&self, index: i32) -> Option<&CouchTheme> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().couch_themes.get(index))
    }

    fn apply_couch_theme_id(mut self: Pin<&mut Self>, id: &str) -> bool {
        let Some(theme) = self
            .as_ref()
            .rust()
            .couch_themes
            .iter()
            .find(|theme| theme.id == id)
            .cloned()
        else {
            return false;
        };
        self.as_mut().apply_couch_theme(theme);
        true
    }

    fn apply_couch_theme(mut self: Pin<&mut Self>, theme: CouchTheme) {
        self.as_mut().set_couch_theme_id(qstring(&theme.id));
        self.as_mut().set_couch_theme_name(qstring(&theme.name));
        self.as_mut().set_couch_theme_author(qstring(&theme.author));
        self.as_mut()
            .set_couch_theme_description(qstring(&theme.description));
        self.as_mut()
            .set_couch_theme_background(qstring(&theme.background));
        self.as_mut().set_couch_theme_panel(qstring(&theme.panel));
        self.as_mut()
            .set_couch_theme_panel_raised(qstring(&theme.panel_raised));
        self.as_mut().set_couch_theme_ink(qstring(&theme.ink));
        self.as_mut().set_couch_theme_muted(qstring(&theme.muted));
        self.as_mut().set_couch_theme_accent(qstring(&theme.accent));
        self.as_mut()
            .set_couch_theme_accent_cool(qstring(&theme.accent_cool));
        self.as_mut().set_couch_theme_danger(qstring(&theme.danger));
        self.as_mut().set_couch_theme_background_image(
            theme
                .background_path
                .as_deref()
                .map(|path| QUrl::from_local_file(&qstring(path.to_string_lossy())))
                .unwrap_or_default(),
        );
        self.as_mut()
            .set_couch_theme_hero_scrim_percent(i32::from(theme.hero_scrim_percent));
        self.as_mut()
            .set_couch_theme_card_radius(i32::from(theme.card_radius));
        let revision = self.as_ref().couch_theme_revision().wrapping_add(1);
        self.as_mut().set_couch_theme_revision(revision);
    }

    pub fn report_couch_theme_ui_probe(&self) {
        if self.rust().couch_theme_ui_probe {
            println!(
                "LUNCHBOX_COUCH_THEME_UI_READY id={} themes={} accent={} background={} radius={}",
                self.couch_theme_id().to_string(),
                self.couch_theme_count(),
                self.couch_theme_accent().to_string(),
                self.couch_theme_background().to_string(),
                self.couch_theme_card_radius()
            );
        }
    }

    fn start_couch_save(mut self: Pin<&mut Self>) {
        let generation = self.as_ref().rust().couch_save_generation;
        let preferences = CouchModePreferences {
            shelf: self.as_ref().couch_shelf().to_string(),
            platform: self.as_ref().couch_platform().to_string(),
            theme_id: self.as_ref().couch_theme_id().to_string(),
            attract_enabled: *self.as_ref().couch_attract_enabled(),
            attract_idle_seconds: *self.as_ref().couch_attract_idle_seconds(),
            attract_cycle_seconds: *self.as_ref().couch_attract_cycle_seconds(),
        };
        self.as_mut().rust_mut().couch_save_pending = true;
        self.as_mut().set_couch_state_saving(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-couch-state-save".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| store.save_couch_mode_preferences(&preferences))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_couch_save(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().rust_mut().couch_save_pending = false;
            self.as_mut().set_couch_state_saving(false);
            self.as_mut().set_status_message(qstring(format!(
                "Couch Mode navigation changed, but its state could not be saved: {error}"
            )));
        }
    }

    fn finish_couch_save(mut self: Pin<&mut Self>, generation: u64, result: Result<(), String>) {
        self.as_mut().rust_mut().couch_save_pending = false;
        if let Err(error) = result {
            self.as_mut().set_status_message(qstring(format!(
                "Couch Mode navigation changed, but its state could not be saved: {error}"
            )));
        }
        if generation != self.as_ref().rust().couch_save_generation {
            self.as_mut().start_couch_save();
        } else {
            self.as_mut().set_couch_state_saving(false);
        }
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
        let Some(source_index) = usize::try_from(index.row())
            .ok()
            .and_then(|row| self.rust().filtered_indices.get(row))
            .copied()
        else {
            return QVariant::default();
        };
        let Some(game) = self.rust().catalog.games.get(source_index) else {
            return QVariant::default();
        };
        let list_value = |column| {
            QVariant::from(&qstring(self.rust().catalog.list_metadata.display_value(
                source_index,
                game,
                column,
                &self.rust().metadata_overrides,
            )))
        };

        match role {
            DISPLAY_ROLE | GAME_TITLE_ROLE => QVariant::from(&qstring(
                self.rust()
                    .metadata_titles
                    .get(&game.id)
                    .unwrap_or(&game.title),
            )),
            GAME_ID_ROLE => QVariant::from(&qstring(&game.id)),
            GAME_PLATFORM_ROLE => QVariant::from(&qstring(&game.platform)),
            GAME_STATUS_ROLE => QVariant::from(&qstring(&game.status)),
            GAME_LOCAL_ROLE => QVariant::from(&game.local),
            GAME_DOWNLOADABLE_ROLE => QVariant::from(&game.downloadable),
            GAME_DATABASE_ID_ROLE => {
                QVariant::from(&i32::try_from(game.launchbox_db_id).unwrap_or_default())
            }
            GAME_CANONICAL_TITLE_ROLE => QVariant::from(&qstring(&game.title)),
            GAME_DEVELOPER_ROLE => list_value(crate::list_view::ListColumn::Developer),
            GAME_PUBLISHER_ROLE => list_value(crate::list_view::ListColumn::Publisher),
            GAME_YEAR_ROLE => list_value(crate::list_view::ListColumn::Year),
            GAME_RELEASE_DATE_ROLE => list_value(crate::list_view::ListColumn::ReleaseDate),
            GAME_GENRE_ROLE => list_value(crate::list_view::ListColumn::Genre),
            GAME_PLAYERS_ROLE => list_value(crate::list_view::ListColumn::Players),
            GAME_RATING_ROLE => list_value(crate::list_view::ListColumn::Rating),
            GAME_ESRB_ROLE => list_value(crate::list_view::ListColumn::Esrb),
            GAME_COOPERATIVE_ROLE => list_value(crate::list_view::ListColumn::Cooperative),
            GAME_VARIANTS_ROLE => list_value(crate::list_view::ListColumn::Variants),
            GAME_RELEASE_TYPE_ROLE => list_value(crate::list_view::ListColumn::ReleaseType),
            GAME_SERIES_ROLE => list_value(crate::list_view::ListColumn::Series),
            GAME_REGION_ROLE => list_value(crate::list_view::ListColumn::Region),
            GAME_NOTES_ROLE => list_value(crate::list_view::ListColumn::Notes),
            GAME_PLAY_MODE_ROLE => list_value(crate::list_view::ListColumn::PlayMode),
            GAME_VERSION_ROLE => list_value(crate::list_view::ListColumn::Version),
            GAME_RELEASE_STATUS_ROLE => list_value(crate::list_view::ListColumn::ReleaseStatus),
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

fn filtered_platform(model: &qobject::LibraryModel, index: i32) -> Option<&catalog::Platform> {
    let source_index = usize::try_from(index)
        .ok()
        .and_then(|index| model.rust().filtered_platform_indices.get(index))?;
    model.rust().catalog.platforms.get(*source_index)
}

fn media_asset_url(asset: &MediaAsset) -> QUrl {
    QUrl::from_local_file(&qstring(asset.path.to_string_lossy()))
}
