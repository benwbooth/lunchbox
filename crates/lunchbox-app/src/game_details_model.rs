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
        #[qproperty(bool, download_preflight_busy)]
        #[qproperty(bool, download_preflight_ready)]
        #[qproperty(i32, download_review_index)]
        #[qproperty(QString, download_preflight_status)]
        #[qproperty(QString, download_preflight_storage)]
        #[qproperty(QString, download_preflight_destination)]
        #[qproperty(QString, download_preflight_mode)]
        #[qproperty(QString, download_preflight_action)]
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
        #[qproperty(i32, rating_count)]
        #[qproperty(QString, esrb)]
        #[qproperty(QString, release_type)]
        #[qproperty(QString, sort_title)]
        #[qproperty(QString, series)]
        #[qproperty(QString, region)]
        #[qproperty(QString, play_mode)]
        #[qproperty(QString, version)]
        #[qproperty(QString, release_status)]
        #[qproperty(QString, cooperative)]
        #[qproperty(QUrl, catalog_video_url)]
        #[qproperty(QUrl, wikipedia_url)]
        #[qproperty(QUrl, steam_store_url)]
        #[qproperty(QString, metadata_source)]
        #[qproperty(QString, notes)]
        #[qproperty(bool, metadata_open)]
        #[qproperty(bool, metadata_busy)]
        #[qproperty(bool, metadata_has_override)]
        #[qproperty(QString, metadata_message)]
        #[qproperty(QString, metadata_title)]
        #[qproperty(QString, metadata_description)]
        #[qproperty(QString, metadata_release_date)]
        #[qproperty(QString, metadata_developer)]
        #[qproperty(QString, metadata_publisher)]
        #[qproperty(QString, metadata_genre)]
        #[qproperty(QString, metadata_players)]
        #[qproperty(QString, metadata_rating)]
        #[qproperty(QString, metadata_esrb)]
        #[qproperty(QString, metadata_release_type)]
        #[qproperty(QString, metadata_sort_title)]
        #[qproperty(QString, metadata_series)]
        #[qproperty(QString, metadata_region)]
        #[qproperty(QString, metadata_play_mode)]
        #[qproperty(QString, metadata_version)]
        #[qproperty(QString, metadata_release_status)]
        #[qproperty(QString, metadata_cooperative)]
        #[qproperty(QString, metadata_notes)]
        #[qproperty(QString, metadata_tags)]
        #[qproperty(i32, metadata_revision)]
        #[qproperty(i32, tag_count)]
        #[qproperty(i32, tag_revision)]
        #[qproperty(i32, custom_field_count)]
        #[qproperty(i32, custom_field_revision)]
        #[qproperty(i32, metadata_custom_field_count)]
        #[qproperty(i32, metadata_custom_field_revision)]
        #[qproperty(i32, variant_count)]
        #[qproperty(i32, alternate_title_count)]
        #[qproperty(i32, related_game_count)]
        #[qproperty(i32, related_game_revision)]
        #[qproperty(QString, related_game_message)]
        #[qproperty(QString, message)]
        #[qproperty(bool, media_visible)]
        #[qproperty(bool, video_available)]
        #[qproperty(bool, manual_available)]
        #[qproperty(bool, soundtrack_available)]
        #[qproperty(i32, soundtrack_count)]
        #[qproperty(i32, soundtrack_index)]
        #[qproperty(QUrl, soundtrack_url)]
        #[qproperty(QString, soundtrack_title)]
        #[qproperty(QString, soundtrack_source)]
        #[qproperty(bool, manual_transfer_active)]
        #[qproperty(bool, manual_action_busy)]
        #[qproperty(QString, manual_download_state)]
        #[qproperty(i32, manual_download_progress)]
        #[qproperty(QString, manual_download_detail)]
        #[qproperty(QString, manual_download_message)]
        #[qproperty(bool, video_progress_busy)]
        #[qproperty(QUrl, video_url)]
        #[qproperty(QUrl, manual_url)]
        #[qproperty(QString, video_source)]
        #[qproperty(QString, manual_source)]
        #[qproperty(QString, media_message)]
        #[qproperty(i32, video_resume_position)]
        #[qproperty(i32, media_revision)]
        #[qproperty(bool, activity_busy)]
        #[qproperty(bool, activity_visible)]
        #[qproperty(i32, play_count)]
        #[qproperty(QString, play_time)]
        #[qproperty(QString, last_played)]
        #[qproperty(QString, completion_state)]
        #[qproperty(i32, activity_revision)]
        #[qproperty(i32, session_count)]
        #[qproperty(i32, session_history_revision)]
        #[qproperty(bool, local)]
        #[qproperty(bool, downloadable)]
        #[qproperty(bool, preparable)]
        #[qproperty(bool, prepared)]
        #[qproperty(bool, exo_media_imported)]
        #[qproperty(QString, preparation_phase)]
        #[qproperty(QString, preparation_file)]
        #[qproperty(QString, prepared_summary)]
        #[qproperty(QString, emulator_name)]
        #[qproperty(QString, emulator_summary)]
        #[qproperty(QString, launch_status)]
        #[qproperty(QString, emulator_preference_scope)]
        #[qproperty(bool, launch_profile_open)]
        #[qproperty(QString, launch_profile_scope)]
        #[qproperty(QString, launch_profile_default_template)]
        #[qproperty(QString, launch_profile_effective_template)]
        #[qproperty(QString, launch_profile_extra_arguments)]
        #[qproperty(QString, launch_profile_command_template)]
        #[qproperty(QString, launch_profile_inheritance)]
        #[qproperty(QString, launch_profile_status)]
        #[qproperty(i32, launch_profile_revision)]
        #[qproperty(bool, launch_profile_preview_valid)]
        #[qproperty(QString, launch_profile_preview_runtime)]
        #[qproperty(QString, launch_profile_preview_message)]
        #[qproperty(i32, launch_profile_preview_argument_count)]
        #[qproperty(i32, launch_profile_preview_revision)]
        #[qproperty(bool, firmware_busy)]
        #[qproperty(i32, firmware_progress)]
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
        #[qproperty(QString, firmware_next_package)]
        #[qproperty(QString, firmware_setup_action)]
        #[qproperty(QString, firmware_setup_label)]
        #[qproperty(bool, switch_prod_keys_imported)]
        #[qproperty(bool, switch_prod_keys_ready)]
        #[qproperty(bool, switch_firmware_imported)]
        #[qproperty(bool, switch_firmware_ready)]
        #[qproperty(i32, registered_torrent_source_count)]
        #[qproperty(i32, bundle_count)]
        #[qproperty(i32, file_count)]
        #[qproperty(i32, selected_bundle)]
        #[qproperty(i32, local_file_count)]
        #[qproperty(i32, selected_local_file)]
        #[qproperty(QUrl, selected_local_directory_url)]
        #[qproperty(bool, local_file_preference_configured)]
        #[qproperty(bool, selected_local_file_is_preferred)]
        #[qproperty(bool, local_file_preference_stale)]
        #[qproperty(QString, local_file_preference_message)]
        #[qproperty(i32, local_file_revision)]
        #[qproperty(bool, install_management_busy)]
        #[qproperty(bool, managed_install_present)]
        #[qproperty(bool, managed_install_can_delete)]
        #[qproperty(i32, managed_install_owned_count)]
        #[qproperty(i32, managed_install_shared_count)]
        #[qproperty(QString, install_management_message)]
        #[qproperty(i32, installation_revision)]
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
        fn open_metadata_editor(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn close_metadata_editor(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn save_metadata(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn reset_metadata(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn tag_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn custom_field_name_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn custom_field_value_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn metadata_custom_field_name_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn metadata_custom_field_value_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn add_metadata_custom_field(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn update_metadata_custom_field(
            self: Pin<&mut GameDetailsModel>,
            index: i32,
            name: QString,
            value: QString,
        );

        #[qinvokable]
        fn remove_metadata_custom_field(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn move_metadata_custom_field(self: Pin<&mut GameDetailsModel>, index: i32, direction: i32);

        #[qinvokable]
        fn session_started_epoch_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn session_emulator_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn session_duration_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn session_outcome_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn session_outcome_label_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn report_session_history_probe(self: &GameDetailsModel);

        #[qinvokable]
        fn load_bundle_files(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn bundle_file_count_at(self: &GameDetailsModel, bundle_index: i32) -> i32;

        #[qinvokable]
        fn bundle_file_name_at(
            self: &GameDetailsModel,
            bundle_index: i32,
            file_index: i32,
        ) -> QString;

        #[qinvokable]
        fn bundle_file_detail_at(
            self: &GameDetailsModel,
            bundle_index: i32,
            file_index: i32,
        ) -> QString;

        #[qinvokable]
        fn bundle_load_message_at(self: &GameDetailsModel, bundle_index: i32) -> QString;

        #[qinvokable]
        fn download_source_count(self: &GameDetailsModel) -> i32;

        #[qinvokable]
        fn download_source_bundle_at(self: &GameDetailsModel, index: i32) -> i32;

        #[qinvokable]
        fn download_candidate_count(self: &GameDetailsModel) -> i32;

        #[qinvokable]
        fn download_candidate_source_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn download_candidate_name_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn download_candidate_detail_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn selected_source_is_registered(self: &GameDetailsModel) -> bool;

        #[qinvokable]
        fn select_download_candidate(self: Pin<&mut GameDetailsModel>, index: i32) -> i32;

        #[qinvokable]
        fn select_bundle_file(
            self: Pin<&mut GameDetailsModel>,
            bundle_index: i32,
            file_index: i32,
        ) -> i32;

        #[qinvokable]
        fn queue_file(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn inspect_download(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn prepare_game(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn cancel_preparation(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn refresh_emulators(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn launch_game(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn cancel_launch(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn select_local_file(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn set_selected_local_file_preferred(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn clear_local_file_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn uninstall_managed_installation(
            self: Pin<&mut GameDetailsModel>,
            delete_owned_files: bool,
        );

        #[qinvokable]
        fn select_emulator_option(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn manager_emulator_target_available(
            self: &GameDetailsModel,
            emulator_id: QString,
            manager: QString,
            package_id: QString,
        ) -> bool;

        #[qinvokable]
        fn manager_emulator_target_is_default(
            self: &GameDetailsModel,
            emulator_id: QString,
            manager: QString,
            package_id: QString,
            scope: QString,
        ) -> bool;

        #[qinvokable]
        fn save_manager_emulator_preference(
            self: Pin<&mut GameDetailsModel>,
            emulator_id: QString,
            manager: QString,
            package_id: QString,
            scope: QString,
        );

        #[qinvokable]
        fn save_game_emulator_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn save_platform_emulator_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn clear_game_emulator_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn clear_platform_emulator_preference(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn open_launch_profile_editor(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn close_launch_profile_editor(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn select_launch_profile_scope(self: Pin<&mut GameDetailsModel>, scope: QString);

        #[qinvokable]
        fn save_launch_profile(
            self: Pin<&mut GameDetailsModel>,
            extra_arguments: QString,
            command_template: QString,
        );

        #[qinvokable]
        fn clear_launch_profile(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn update_launch_profile_preview(
            self: Pin<&mut GameDetailsModel>,
            extra_arguments: QString,
            command_template: QString,
        );

        #[qinvokable]
        fn launch_profile_preview_argument_at(self: &GameDetailsModel, index: i32) -> QString;

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
        fn save_video_progress(
            self: Pin<&mut GameDetailsModel>,
            position_ms: i32,
            duration_ms: i32,
        );

        #[qinvokable]
        fn reset_video_progress(self: Pin<&mut GameDetailsModel>, duration_ms: i32);

        #[qinvokable]
        fn select_soundtrack(self: Pin<&mut GameDetailsModel>, index: i32);

        #[qinvokable]
        fn report_soundtrack_ui_probe(self: &GameDetailsModel, screenshot: QString);

        #[qinvokable]
        fn refresh_media(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn download_manual(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn refresh_manual_download(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn cancel_manual_download(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn bundle_title_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn bundle_file_size_at(
            self: &GameDetailsModel,
            bundle_index: i32,
            file_index: i32,
        ) -> QString;

        #[qinvokable]
        fn bundle_detail_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn alternate_title_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn alternate_title_region_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn variant_title_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn variant_label_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn variant_game_id_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn variant_database_id_at(self: &GameDetailsModel, index: i32) -> i32;

        #[qinvokable]
        fn variant_status_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn variant_is_current_at(self: &GameDetailsModel, index: i32) -> bool;

        #[qinvokable]
        fn variant_is_local_at(self: &GameDetailsModel, index: i32) -> bool;

        #[qinvokable]
        fn variant_is_downloadable_at(self: &GameDetailsModel, index: i32) -> bool;

        #[qinvokable]
        fn related_game_id_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn related_game_database_id_at(self: &GameDetailsModel, index: i32) -> i32;

        #[qinvokable]
        fn related_game_title_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn related_game_platform_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn related_game_reason_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn related_game_is_local_at(self: &GameDetailsModel, index: i32) -> bool;

        #[qinvokable]
        fn related_game_is_downloadable_at(self: &GameDetailsModel, index: i32) -> bool;

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
        fn local_file_name_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn local_file_path_at(self: &GameDetailsModel, index: i32) -> QString;

        #[qinvokable]
        fn local_file_is_preferred_at(self: &GameDetailsModel, index: i32) -> bool;

        #[qinvokable]
        fn emulator_option_label_at(self: &GameDetailsModel, index: i32) -> QString;
    }

    impl cxx_qt::Threading for GameDetailsModel {}
}

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

use anyhow::Context;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::game_details::{
    self, AlternateTitle, GameDetails, GameVariant, MinervaBundle, RelatedGame, ReleasePreferences,
    TorrentFileCandidate,
};
use crate::settings::{
    GameCustomField, GameMetadata, GameMetadataOverride, PlaySession, SettingsStore,
};

#[derive(Clone, Debug, Default)]
struct BundleCandidateGroup {
    loaded: bool,
    files: Vec<TorrentFileCandidate>,
    error: String,
}

pub struct GameDetailsModelRust {
    panel_open: bool,
    loading: bool,
    torrent_loading: bool,
    download_busy: bool,
    download_preflight_busy: bool,
    download_preflight_ready: bool,
    download_review_index: i32,
    download_preflight_status: QString,
    download_preflight_storage: QString,
    download_preflight_destination: QString,
    download_preflight_mode: QString,
    download_preflight_action: QString,
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
    rating_count: i32,
    esrb: QString,
    release_type: QString,
    sort_title: QString,
    series: QString,
    region: QString,
    play_mode: QString,
    version: QString,
    release_status: QString,
    cooperative: QString,
    catalog_video_url: QUrl,
    wikipedia_url: QUrl,
    steam_store_url: QUrl,
    metadata_source: QString,
    notes: QString,
    metadata_open: bool,
    metadata_busy: bool,
    metadata_has_override: bool,
    metadata_message: QString,
    metadata_title: QString,
    metadata_description: QString,
    metadata_release_date: QString,
    metadata_developer: QString,
    metadata_publisher: QString,
    metadata_genre: QString,
    metadata_players: QString,
    metadata_rating: QString,
    metadata_esrb: QString,
    metadata_release_type: QString,
    metadata_sort_title: QString,
    metadata_series: QString,
    metadata_region: QString,
    metadata_play_mode: QString,
    metadata_version: QString,
    metadata_release_status: QString,
    metadata_cooperative: QString,
    metadata_notes: QString,
    metadata_tags: QString,
    metadata_revision: i32,
    tag_count: i32,
    tag_revision: i32,
    custom_field_count: i32,
    custom_field_revision: i32,
    metadata_custom_field_count: i32,
    metadata_custom_field_revision: i32,
    variant_count: i32,
    alternate_title_count: i32,
    related_game_count: i32,
    related_game_revision: i32,
    related_game_message: QString,
    message: QString,
    media_visible: bool,
    video_available: bool,
    manual_available: bool,
    soundtrack_available: bool,
    soundtrack_count: i32,
    soundtrack_index: i32,
    soundtrack_url: QUrl,
    soundtrack_title: QString,
    soundtrack_source: QString,
    manual_transfer_active: bool,
    manual_action_busy: bool,
    manual_download_state: QString,
    manual_download_progress: i32,
    manual_download_detail: QString,
    manual_download_message: QString,
    video_progress_busy: bool,
    video_url: QUrl,
    manual_url: QUrl,
    video_source: QString,
    manual_source: QString,
    media_message: QString,
    video_resume_position: i32,
    media_revision: i32,
    activity_busy: bool,
    activity_visible: bool,
    play_count: i32,
    play_time: QString,
    last_played: QString,
    completion_state: QString,
    activity_revision: i32,
    session_count: i32,
    session_history_revision: i32,
    local: bool,
    downloadable: bool,
    preparable: bool,
    prepared: bool,
    exo_media_imported: bool,
    preparation_phase: QString,
    preparation_file: QString,
    prepared_summary: QString,
    emulator_name: QString,
    emulator_summary: QString,
    launch_status: QString,
    emulator_preference_scope: QString,
    launch_profile_open: bool,
    launch_profile_scope: QString,
    launch_profile_default_template: QString,
    launch_profile_effective_template: QString,
    launch_profile_extra_arguments: QString,
    launch_profile_command_template: QString,
    launch_profile_inheritance: QString,
    launch_profile_status: QString,
    launch_profile_revision: i32,
    launch_profile_preview_valid: bool,
    launch_profile_preview_runtime: QString,
    launch_profile_preview_message: QString,
    launch_profile_preview_argument_count: i32,
    launch_profile_preview_revision: i32,
    firmware_busy: bool,
    firmware_progress: i32,
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
    firmware_next_package: QString,
    firmware_setup_action: QString,
    firmware_setup_label: QString,
    switch_prod_keys_imported: bool,
    switch_prod_keys_ready: bool,
    switch_firmware_imported: bool,
    switch_firmware_ready: bool,
    registered_torrent_source_count: i32,
    bundle_count: i32,
    file_count: i32,
    selected_bundle: i32,
    local_file_count: i32,
    selected_local_file: i32,
    selected_local_directory_url: QUrl,
    local_file_preference_configured: bool,
    selected_local_file_is_preferred: bool,
    local_file_preference_stale: bool,
    local_file_preference_message: QString,
    local_file_revision: i32,
    install_management_busy: bool,
    managed_install_present: bool,
    managed_install_can_delete: bool,
    managed_install_owned_count: i32,
    managed_install_shared_count: i32,
    install_management_message: QString,
    installation_revision: i32,
    emulator_option_count: i32,
    selected_emulator_option: i32,
    detail_revision: i32,
    canonical_title: String,
    canonical_metadata: GameMetadata,
    effective_metadata: GameMetadata,
    metadata_generation: u64,
    current_tags: Vec<String>,
    current_custom_fields: Vec<GameCustomField>,
    metadata_custom_fields: Vec<GameCustomField>,
    sessions: Vec<PlaySession>,
    soundtrack_tracks: Vec<crate::media::SoundtrackAsset>,
    bundles: Vec<MinervaBundle>,
    bundle_candidates: Vec<BundleCandidateGroup>,
    variants: Vec<GameVariant>,
    alternate_titles: Vec<AlternateTitle>,
    related_games: Vec<RelatedGame>,
    files: Vec<TorrentFileCandidate>,
    details_generation: u64,
    media_refresh_generation: u64,
    torrent_generation: u64,
    download_preflight_generation: u64,
    preparation_generation: u64,
    launch_generation: u64,
    activity_load_generation: u64,
    preparation_cancel: Option<Arc<AtomicBool>>,
    launch_cancel: Option<Arc<AtomicBool>>,
    database_id: i64,
    local_file_path: PathBuf,
    local_file_paths: Vec<PathBuf>,
    preferred_local_file_path: Option<PathBuf>,
    rom_emulator_options: Vec<crate::emulator::RomEmulatorOption>,
    rom_firmware_statuses: Vec<Vec<crate::firmware::FirmwareStatus>>,
    game_emulator_preference: Option<crate::settings::EmulatorPreference>,
    platform_emulator_preference: Option<crate::settings::EmulatorPreference>,
    launch_profile_preview_arguments: Vec<String>,
    launch_profile_preview_fallback_extra_arguments: String,
    launch_profile_preview_fallback_command_template: String,
    prepared_emulator: Option<crate::emulator::EmulatorChoice>,
    pending_firmware_message: Option<String>,
    prepared_install: Option<crate::exo_install::PreparedInstall>,
    video_media_key: String,
}

impl Default for GameDetailsModelRust {
    fn default() -> Self {
        Self {
            panel_open: false,
            loading: false,
            torrent_loading: false,
            download_busy: false,
            download_preflight_busy: false,
            download_preflight_ready: false,
            download_review_index: -1,
            download_preflight_status: QString::default(),
            download_preflight_storage: QString::default(),
            download_preflight_destination: QString::default(),
            download_preflight_mode: QString::default(),
            download_preflight_action: QString::default(),
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
            rating_count: 0,
            esrb: QString::default(),
            release_type: QString::default(),
            sort_title: QString::default(),
            series: QString::default(),
            region: QString::default(),
            play_mode: QString::default(),
            version: QString::default(),
            release_status: QString::default(),
            cooperative: QString::from("unknown"),
            catalog_video_url: QUrl::default(),
            wikipedia_url: QUrl::default(),
            steam_store_url: QUrl::default(),
            metadata_source: QString::default(),
            notes: QString::default(),
            metadata_open: false,
            metadata_busy: false,
            metadata_has_override: false,
            metadata_message: QString::from(
                "Edits are stored in your profile; catalog identity stays unchanged.",
            ),
            metadata_title: QString::default(),
            metadata_description: QString::default(),
            metadata_release_date: QString::default(),
            metadata_developer: QString::default(),
            metadata_publisher: QString::default(),
            metadata_genre: QString::default(),
            metadata_players: QString::default(),
            metadata_rating: QString::default(),
            metadata_esrb: QString::default(),
            metadata_release_type: QString::default(),
            metadata_sort_title: QString::default(),
            metadata_series: QString::default(),
            metadata_region: QString::default(),
            metadata_play_mode: QString::default(),
            metadata_version: QString::default(),
            metadata_release_status: QString::default(),
            metadata_cooperative: QString::from("unknown"),
            metadata_notes: QString::default(),
            metadata_tags: QString::default(),
            metadata_revision: 0,
            tag_count: 0,
            tag_revision: 0,
            custom_field_count: 0,
            custom_field_revision: 0,
            metadata_custom_field_count: 0,
            metadata_custom_field_revision: 0,
            variant_count: 0,
            alternate_title_count: 0,
            related_game_count: 0,
            related_game_revision: 0,
            related_game_message: QString::default(),
            message: QString::from("Select a game to inspect it."),
            media_visible: false,
            video_available: false,
            manual_available: false,
            soundtrack_available: false,
            soundtrack_count: 0,
            soundtrack_index: -1,
            soundtrack_url: QUrl::default(),
            soundtrack_title: QString::default(),
            soundtrack_source: QString::default(),
            manual_transfer_active: false,
            manual_action_busy: false,
            manual_download_state: QString::default(),
            manual_download_progress: 0,
            manual_download_detail: QString::default(),
            manual_download_message: QString::default(),
            video_progress_busy: false,
            video_url: QUrl::default(),
            manual_url: QUrl::default(),
            video_source: QString::default(),
            manual_source: QString::default(),
            media_message: QString::default(),
            video_resume_position: 0,
            media_revision: 0,
            activity_busy: false,
            activity_visible: false,
            play_count: 0,
            play_time: QString::from("Never played"),
            last_played: QString::from("Never"),
            completion_state: QString::from("not_started"),
            activity_revision: 0,
            session_count: 0,
            session_history_revision: 0,
            local: false,
            downloadable: false,
            preparable: false,
            prepared: false,
            exo_media_imported: false,
            preparation_phase: QString::default(),
            preparation_file: QString::default(),
            prepared_summary: QString::default(),
            emulator_name: QString::default(),
            emulator_summary: QString::default(),
            launch_status: QString::default(),
            emulator_preference_scope: QString::default(),
            launch_profile_open: false,
            launch_profile_scope: QString::from("game"),
            launch_profile_default_template: QString::default(),
            launch_profile_effective_template: QString::default(),
            launch_profile_extra_arguments: QString::default(),
            launch_profile_command_template: QString::default(),
            launch_profile_inheritance: QString::default(),
            launch_profile_status: QString::default(),
            launch_profile_revision: 0,
            launch_profile_preview_valid: false,
            launch_profile_preview_runtime: QString::default(),
            launch_profile_preview_message: QString::default(),
            launch_profile_preview_argument_count: 0,
            launch_profile_preview_revision: 0,
            firmware_busy: false,
            firmware_progress: -1,
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
            firmware_next_package: QString::default(),
            firmware_setup_action: QString::from("review"),
            firmware_setup_label: QString::from("SET UP EMULATOR"),
            switch_prod_keys_imported: false,
            switch_prod_keys_ready: false,
            switch_firmware_imported: false,
            switch_firmware_ready: false,
            registered_torrent_source_count: 0,
            bundle_count: 0,
            file_count: 0,
            selected_bundle: -1,
            local_file_count: 0,
            selected_local_file: -1,
            selected_local_directory_url: QUrl::default(),
            local_file_preference_configured: false,
            selected_local_file_is_preferred: false,
            local_file_preference_stale: false,
            local_file_preference_message: QString::default(),
            local_file_revision: 0,
            install_management_busy: false,
            managed_install_present: false,
            managed_install_can_delete: false,
            managed_install_owned_count: 0,
            managed_install_shared_count: 0,
            install_management_message: QString::default(),
            installation_revision: 0,
            emulator_option_count: 0,
            selected_emulator_option: -1,
            detail_revision: 0,
            canonical_title: String::new(),
            canonical_metadata: GameMetadata::default(),
            effective_metadata: GameMetadata::default(),
            metadata_generation: 0,
            current_tags: Vec::new(),
            current_custom_fields: Vec::new(),
            metadata_custom_fields: Vec::new(),
            sessions: Vec::new(),
            soundtrack_tracks: Vec::new(),
            bundles: Vec::new(),
            bundle_candidates: Vec::new(),
            variants: Vec::new(),
            alternate_titles: Vec::new(),
            related_games: Vec::new(),
            files: Vec::new(),
            details_generation: 0,
            media_refresh_generation: 0,
            torrent_generation: 0,
            download_preflight_generation: 0,
            preparation_generation: 0,
            launch_generation: 0,
            activity_load_generation: 0,
            preparation_cancel: None,
            launch_cancel: None,
            database_id: 0,
            local_file_path: PathBuf::new(),
            local_file_paths: Vec::new(),
            preferred_local_file_path: None,
            rom_emulator_options: Vec::new(),
            rom_firmware_statuses: Vec::new(),
            game_emulator_preference: None,
            platform_emulator_preference: None,
            launch_profile_preview_arguments: Vec::new(),
            launch_profile_preview_fallback_extra_arguments: String::new(),
            launch_profile_preview_fallback_command_template: String::new(),
            prepared_emulator: None,
            pending_firmware_message: None,
            prepared_install: None,
            video_media_key: String::new(),
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn custom_field_at(fields: &[GameCustomField], index: i32) -> Option<&GameCustomField> {
    usize::try_from(index)
        .ok()
        .and_then(|index| fields.get(index))
}

fn launch_scope_label(scope: &str) -> &'static str {
    match scope {
        "game" => "this game",
        "platform" => "platform",
        "global" => "all-platform",
        _ => "inherited",
    }
}

fn validate_launch_profile_template(
    target: &LaunchProfileTarget,
    template: &str,
) -> anyhow::Result<()> {
    for placeholder in crate::emulator::launch_template_placeholders(template)? {
        anyhow::ensure!(
            target
                .available_placeholders
                .iter()
                .any(|available| available == &placeholder),
            "placeholder %{{{placeholder}}} is unavailable for {}; use only placeholders shown by its built-in template",
            target.emulator_label
        );
    }
    Ok(())
}

fn preview_for_launch_profile_target(
    target: &LaunchProfileTarget,
    extra_arguments: &str,
    command_template: &str,
) -> anyhow::Result<crate::emulator::LaunchCommandPreview> {
    validate_launch_profile_template(target, command_template)?;
    crate::emulator::preview_launch_command(
        &target.emulator_label,
        &target.default_template,
        extra_arguments,
        command_template,
        &target.available_placeholders,
        target.preview_extra_insert_index,
    )
}

fn launch_profile_preview_fallback(
    store: &SettingsStore,
    scope: &str,
    platform: &str,
    target: &LaunchProfileTarget,
) -> anyhow::Result<crate::settings::ResolvedLaunchCustomization> {
    match scope {
        "game" => store.resolve_launch_customization(
            "",
            platform,
            &target.emulator_id,
            target.runtime_kind,
            &target.core_name,
        ),
        "platform" => store.resolve_launch_customization(
            "",
            "",
            &target.emulator_id,
            target.runtime_kind,
            &target.core_name,
        ),
        "global" => Ok(crate::settings::ResolvedLaunchCustomization::default()),
        _ => anyhow::bail!("unsupported launch profile preview scope {scope}"),
    }
}

fn local_file_url(path: &std::path::Path) -> QUrl {
    QUrl::from_local_file(&qstring(path.to_string_lossy()))
}

fn local_directory_url(path: &std::path::Path) -> QUrl {
    path.parent().map(local_file_url).unwrap_or_default()
}

fn local_file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.as_os_str().to_string_lossy().into_owned())
}

fn local_file_preference_message(
    preferred: Option<&std::path::Path>,
    stale: bool,
    selected: Option<&std::path::Path>,
    file_count: usize,
) -> String {
    if file_count == 0 {
        return String::new();
    }
    if stale {
        let unavailable = preferred.map(local_file_name).unwrap_or_default();
        let fallback = selected.map(local_file_name).unwrap_or_default();
        return format!(
            "Saved default {unavailable} is unavailable. Using {fallback} for this session."
        );
    }
    if let Some(preferred) = preferred {
        if selected == Some(preferred) {
            return "This exact version opens by default.".to_owned();
        }
        return format!(
            "One-off version selected. {} remains the default.",
            local_file_name(preferred)
        );
    }
    if file_count == 1 {
        "This is the only available game file.".to_owned()
    } else {
        "Automatic selection is active. Choose a version or make one the default.".to_owned()
    }
}

fn catalog_url(value: &str) -> QUrl {
    if value.is_empty() {
        QUrl::default()
    } else {
        QUrl::from(value)
    }
}

fn steam_store_url(app_id: i64) -> QUrl {
    let value = steam_store_url_string(app_id);
    if value.is_empty() {
        QUrl::default()
    } else {
        QUrl::from(value.as_str())
    }
}

fn steam_store_url_string(app_id: i64) -> String {
    (app_id > 0)
        .then(|| format!("https://store.steampowered.com/app/{app_id}"))
        .unwrap_or_default()
}

fn metadata_source_label(source: &str) -> &str {
    match source.trim() {
        "launchbox" => "LaunchBox",
        "libretro" => "Libretro",
        source => source,
    }
}

fn count_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn metadata_save_messages(
    reset_requested: bool,
    metadata_is_canonical: bool,
    tag_count: usize,
    custom_field_count: usize,
) -> (String, String) {
    let tag_label = if tag_count == 1 { "tag" } else { "tags" };
    let custom_field_label = if custom_field_count == 1 {
        "custom field"
    } else {
        "custom fields"
    };
    let local_profile_summary = match (tag_count, custom_field_count) {
        (0, 0) => None,
        (tags, 0) => Some(format!("{tags} local {tag_label}")),
        (0, fields) => Some(format!("{fields} {custom_field_label}")),
        (tags, fields) => Some(format!(
            "{tags} local {tag_label} and {fields} {custom_field_label}"
        )),
    };
    let local_profile_label = match (tag_count > 0, custom_field_count > 0) {
        (false, false) => None,
        (true, false) => Some("Local tags"),
        (false, true) => Some("Local custom fields"),
        (true, true) => Some("Local tags and custom fields"),
    };
    if reset_requested {
        let detail = match &local_profile_summary {
            None => "Restored canonical catalog metadata.".to_owned(),
            Some(summary) => {
                format!("Restored canonical catalog metadata and kept {summary}.")
            }
        };
        return (
            detail,
            match local_profile_label {
                None => "Canonical catalog metadata restored.".to_owned(),
                Some(label) => {
                    format!("Canonical catalog metadata restored. {label} were kept.")
                }
            },
        );
    }
    if metadata_is_canonical {
        if local_profile_summary.is_none() {
            return (
                "No local presentation changes are stored.".to_owned(),
                "Canonical catalog metadata and identity are unchanged.".to_owned(),
            );
        }
        let summary = local_profile_summary.expect("profile summary exists");
        let label = local_profile_label.expect("profile label exists");
        return (
            format!("Saved {summary} without changing canonical metadata."),
            format!("{label} saved. Downloads and matching still use canonical identity."),
        );
    }
    let Some(summary) = local_profile_summary else {
        return (
            "Saved local metadata without changing canonical identity.".to_owned(),
            "Local metadata saved. Downloads and matching still use canonical identity.".to_owned(),
        );
    };
    let label = local_profile_label.expect("profile label exists");
    (
        format!("Saved local metadata, {summary} without changing canonical identity."),
        format!(
            "Local metadata, {} saved. Downloads and matching still use canonical identity.",
            label.trim_start_matches("Local ").to_lowercase()
        ),
    )
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

fn format_session_duration(seconds: i64, outcome: &str) -> String {
    if outcome == "running" {
        return "In progress".to_owned();
    }
    let seconds = u64::try_from(seconds).unwrap_or_default();
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

fn session_outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "completed" => "Completed",
        "failed" => "Failed",
        "terminated" => "Interrupted",
        "running" => "In progress",
        _ => "Unknown",
    }
}

fn format_release_date(value: &str) -> String {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() < 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return value.to_owned();
    }
    let Ok(month) = value[5..7].parse::<usize>() else {
        return value.to_owned();
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return value.to_owned();
    };
    let Some(month) = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .get(month.saturating_sub(1)) else {
        return value.to_owned();
    };
    if day == 0 || day > 31 {
        return value.to_owned();
    }
    format!("{month} {day}, {}", &value[..4])
}

fn has_cli_flag(flag: &str) -> bool {
    std::env::args_os().any(|argument| argument == flag)
}

fn is_local_launch_probe() -> bool {
    has_cli_flag("--rom-launch-probe")
        || has_cli_flag("--arcade-launch-probe")
        || has_cli_flag("--couch-launch-ui-probe")
}

fn is_emulator_launch_probe() -> bool {
    has_cli_flag("--exo-launch-probe") || is_local_launch_probe()
}

enum EmulatorDiscoveryResult {
    Prepared {
        availability: crate::emulator::LaunchAvailability,
        game_preference: Option<crate::settings::EmulatorPreference>,
        platform_preference: Option<crate::settings::EmulatorPreference>,
    },
    Rom {
        availability: crate::emulator::RomLaunchAvailability,
        firmware_statuses: Vec<Vec<crate::firmware::FirmwareStatus>>,
        game_preference: Option<crate::settings::EmulatorPreference>,
        platform_preference: Option<crate::settings::EmulatorPreference>,
    },
}

fn emulator_preference_matches_option(
    preference: &crate::settings::EmulatorPreference,
    option: &crate::emulator::RomEmulatorOption,
) -> bool {
    preference.emulator_id == option.emulator_id
        && preference.runtime_kind == option.runtime_kind.key()
        && preference.core_name == option.core_name
}

fn emulator_preference_for_option(
    option: &crate::emulator::RomEmulatorOption,
    scope: &str,
) -> crate::settings::EmulatorPreference {
    crate::settings::EmulatorPreference {
        emulator_id: option.emulator_id.clone(),
        runtime_kind: option.runtime_kind.key().to_owned(),
        core_name: option.core_name.clone(),
        scope: scope.to_owned(),
    }
}

enum LaunchInput {
    Prepared {
        install: crate::exo_install::PreparedInstall,
        catalog_database: PathBuf,
        emulator_id: String,
    },
    Rom {
        path: PathBuf,
        platform: String,
        option: crate::emulator::RomEmulatorOption,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LaunchProfileTarget {
    emulator_id: String,
    emulator_label: String,
    runtime_kind: &'static str,
    core_name: String,
    default_template: String,
    available_placeholders: Vec<String>,
    preview_extra_insert_index: usize,
}

struct LaunchStarted {
    game_id: String,
    emulator_name: String,
    process_id: u32,
    command_summary: String,
    tracking_warning: Option<String>,
    activity_recorded: bool,
}

struct LaunchCleanupGuard(Vec<PathBuf>);

impl Drop for LaunchCleanupGuard {
    fn drop(&mut self) {
        crate::emulator::cleanup_after_launch(&self.0);
    }
}

struct SupplementalMediaRefresh {
    game_id: String,
    media: crate::media::SupplementalMedia,
    video_media_key: String,
    video_progress: Option<crate::settings::MediaPlaybackProgress>,
    manual_transfer: Option<crate::settings::MediaTransfer>,
}

fn load_supplemental_media_refresh(
    game_id: &str,
    database_id: i64,
) -> anyhow::Result<SupplementalMediaRefresh> {
    let media = crate::media::supplemental_media(game_id, database_id)?;
    let manual_transfer = crate::media_acquisition::load_manual_transfer(game_id)?;
    let (video_media_key, video_progress) = match media.video.as_ref() {
        Some(video) => {
            let media_key = crate::media::media_identity(&video.path)?;
            let progress =
                SettingsStore::open_default()?.media_playback_progress(game_id, &media_key)?;
            (media_key, progress)
        }
        None => (String::new(), None),
    };
    Ok(SupplementalMediaRefresh {
        game_id: game_id.to_owned(),
        media,
        video_media_key,
        video_progress,
        manual_transfer,
    })
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
        let download_review_probe = has_cli_flag("--multidisc-ui-probe")
            || has_cli_flag("--exo-archive-ui-probe")
            || has_cli_flag("--laserdisc-ui-probe");
        if has_cli_flag("--metadata-ui-probe") {
            println!("LUNCHBOX_METADATA_MODEL_SELECT id={}", game_id.to_string());
        }
        let game_id_string = game_id.to_string();
        let title_string = title.to_string();
        let platform_string = platform.to_string();
        if download_review_probe {
            println!(
                "LUNCHBOX_DOWNLOAD_REVIEW_MODEL_SELECT id={game_id_string:?} title={title_string:?} platform={platform_string:?}"
            );
        }
        self.as_mut().invalidate_preparation();
        self.as_mut().invalidate_launch_state();
        self.as_mut().rust_mut().details_generation =
            self.as_ref().rust().details_generation.wrapping_add(1);
        self.as_mut().rust_mut().media_refresh_generation = self
            .as_ref()
            .rust()
            .media_refresh_generation
            .wrapping_add(1);
        self.as_mut().rust_mut().torrent_generation =
            self.as_ref().rust().torrent_generation.wrapping_add(1);
        self.as_mut().rust_mut().metadata_generation =
            self.as_ref().rust().metadata_generation.wrapping_add(1);
        self.as_mut().rust_mut().canonical_title = title_string.clone();
        let generation = self.as_ref().rust().details_generation;

        self.as_mut().set_panel_open(true);
        self.as_mut().set_loading(true);
        self.as_mut().set_torrent_loading(false);
        self.as_mut().set_game_id(game_id);
        self.as_mut().set_title(title);
        self.as_mut().set_platform(platform);
        self.as_mut().set_local(local);
        self.as_mut().set_downloadable(downloadable);
        self.as_mut().set_metadata_open(false);
        self.as_mut().set_metadata_busy(false);
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
                if download_review_probe {
                    match &loaded {
                        Ok(details) => println!(
                            "LUNCHBOX_DOWNLOAD_REVIEW_MODEL_LOADED id={game_id_string:?} bundles={}",
                            details.bundles.len()
                        ),
                        Err(error) => eprintln!(
                            "LUNCHBOX_DOWNLOAD_REVIEW_MODEL_FAILED id={game_id_string:?} error={error}"
                        ),
                    }
                }
                let queued = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_game_details(generation, loaded);
                });
                if download_review_probe {
                    match queued {
                        Ok(()) => println!(
                            "LUNCHBOX_DOWNLOAD_REVIEW_MODEL_QUEUED id={game_id_string:?}"
                        ),
                        Err(error) => eprintln!(
                            "LUNCHBOX_DOWNLOAD_REVIEW_MODEL_QUEUE_FAILED id={game_id_string:?} error={error}"
                        ),
                    }
                }
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
        self.as_mut().rust_mut().media_refresh_generation = self
            .as_ref()
            .rust()
            .media_refresh_generation
            .wrapping_add(1);
        self.as_mut().rust_mut().torrent_generation =
            self.as_ref().rust().torrent_generation.wrapping_add(1);
        self.as_mut().rust_mut().metadata_generation =
            self.as_ref().rust().metadata_generation.wrapping_add(1);
        self.as_mut().set_loading(false);
        self.as_mut().set_torrent_loading(false);
        self.as_mut().set_prepare_busy(false);
        self.as_mut().set_launch_discovery_busy(false);
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_firmware_busy(false);
        self.as_mut().set_metadata_open(false);
        self.as_mut().set_metadata_busy(false);
        self.as_mut().rust_mut().pending_firmware_message = None;
        self.as_mut().set_panel_open(false);
    }

    pub fn open_metadata_editor(mut self: Pin<&mut Self>) {
        if *self.as_ref().loading()
            || *self.as_ref().metadata_busy()
            || self.as_ref().game_id().is_empty()
        {
            return;
        }
        let metadata = self.as_ref().rust().effective_metadata.clone();
        self.as_mut().set_metadata_title(qstring(&metadata.title));
        self.as_mut()
            .set_metadata_description(qstring(&metadata.description));
        self.as_mut()
            .set_metadata_release_date(qstring(&metadata.release_date));
        self.as_mut()
            .set_metadata_developer(qstring(&metadata.developer));
        self.as_mut()
            .set_metadata_publisher(qstring(&metadata.publisher));
        self.as_mut().set_metadata_genre(qstring(&metadata.genre));
        self.as_mut()
            .set_metadata_players(qstring(&metadata.players));
        self.as_mut().set_metadata_rating(qstring(&metadata.rating));
        self.as_mut().set_metadata_esrb(qstring(&metadata.esrb));
        self.as_mut()
            .set_metadata_release_type(qstring(&metadata.release_type));
        self.as_mut()
            .set_metadata_sort_title(qstring(&metadata.sort_title));
        self.as_mut().set_metadata_series(qstring(&metadata.series));
        self.as_mut().set_metadata_region(qstring(&metadata.region));
        self.as_mut()
            .set_metadata_play_mode(qstring(&metadata.play_mode));
        self.as_mut()
            .set_metadata_version(qstring(&metadata.version));
        self.as_mut()
            .set_metadata_release_status(qstring(&metadata.release_status));
        self.as_mut()
            .set_metadata_cooperative(qstring(&metadata.cooperative));
        self.as_mut().set_metadata_notes(qstring(&metadata.notes));
        let tags = self.as_ref().rust().current_tags.join(", ");
        self.as_mut().set_metadata_tags(qstring(tags));
        let custom_fields = self.as_ref().rust().current_custom_fields.clone();
        let custom_field_count = custom_fields.len();
        self.as_mut().rust_mut().metadata_custom_fields = custom_fields;
        self.as_mut()
            .set_metadata_custom_field_count(count_i32(custom_field_count));
        let custom_field_revision = self
            .as_ref()
            .metadata_custom_field_revision()
            .wrapping_add(1);
        self.as_mut()
            .set_metadata_custom_field_revision(custom_field_revision);
        self.as_mut().set_metadata_message(qstring(
            "Presentation metadata and searchable custom fields stay in your profile. Stable game identity, platform, provider matches, and files are preserved.",
        ));
        self.as_mut().set_metadata_open(true);
        if has_cli_flag("--metadata-ui-probe") {
            println!("LUNCHBOX_METADATA_MODEL_OPEN");
        }
    }

    pub fn close_metadata_editor(mut self: Pin<&mut Self>) {
        if has_cli_flag("--metadata-ui-probe") {
            println!(
                "LUNCHBOX_METADATA_MODEL_CLOSE busy={}",
                *self.as_ref().metadata_busy()
            );
        }
        if !*self.as_ref().metadata_busy() {
            self.as_mut().set_metadata_open(false);
        }
    }

    pub fn save_metadata(mut self: Pin<&mut Self>) {
        if !*self.as_ref().metadata_open() || *self.as_ref().metadata_busy() {
            return;
        }
        let effective = GameMetadata {
            title: self.as_ref().metadata_title().to_string().trim().to_owned(),
            description: self
                .as_ref()
                .metadata_description()
                .to_string()
                .trim()
                .to_owned(),
            release_date: self
                .as_ref()
                .metadata_release_date()
                .to_string()
                .trim()
                .to_owned(),
            developer: self
                .as_ref()
                .metadata_developer()
                .to_string()
                .trim()
                .to_owned(),
            publisher: self
                .as_ref()
                .metadata_publisher()
                .to_string()
                .trim()
                .to_owned(),
            genre: self.as_ref().metadata_genre().to_string().trim().to_owned(),
            players: self
                .as_ref()
                .metadata_players()
                .to_string()
                .trim()
                .to_owned(),
            rating: self
                .as_ref()
                .metadata_rating()
                .to_string()
                .trim()
                .to_owned(),
            esrb: self.as_ref().metadata_esrb().to_string().trim().to_owned(),
            release_type: self
                .as_ref()
                .metadata_release_type()
                .to_string()
                .trim()
                .to_owned(),
            sort_title: self
                .as_ref()
                .metadata_sort_title()
                .to_string()
                .trim()
                .to_owned(),
            series: self
                .as_ref()
                .metadata_series()
                .to_string()
                .trim()
                .to_owned(),
            region: self
                .as_ref()
                .metadata_region()
                .to_string()
                .trim()
                .to_owned(),
            play_mode: self
                .as_ref()
                .metadata_play_mode()
                .to_string()
                .trim()
                .to_owned(),
            version: self
                .as_ref()
                .metadata_version()
                .to_string()
                .trim()
                .to_owned(),
            release_status: self
                .as_ref()
                .metadata_release_status()
                .to_string()
                .trim()
                .to_owned(),
            cooperative: self
                .as_ref()
                .metadata_cooperative()
                .to_string()
                .trim()
                .to_owned(),
            notes: self.as_ref().metadata_notes().to_string().trim().to_owned(),
        };
        let tags = self.as_ref().metadata_tags().to_string();
        let custom_fields = self.as_ref().rust().metadata_custom_fields.clone();
        self.as_mut()
            .start_metadata_save(effective, tags, custom_fields, false);
    }

    pub fn reset_metadata(mut self: Pin<&mut Self>) {
        if !*self.as_ref().metadata_open() || *self.as_ref().metadata_busy() {
            return;
        }
        let canonical = self.as_ref().rust().canonical_metadata.clone();
        let tags = self.as_ref().rust().current_tags.join(", ");
        let custom_fields = self.as_ref().rust().metadata_custom_fields.clone();
        self.as_mut()
            .start_metadata_save(canonical, tags, custom_fields, true);
    }

    fn start_metadata_save(
        mut self: Pin<&mut Self>,
        effective: GameMetadata,
        tag_input: String,
        custom_fields: Vec<GameCustomField>,
        reset_requested: bool,
    ) {
        let canonical = self.as_ref().rust().canonical_metadata.clone();
        let metadata_override = GameMetadataOverride::from_effective(&canonical, &effective);
        if let Err(error) = metadata_override.validate() {
            self.as_mut()
                .set_metadata_message(qstring(format!("Could not save metadata: {error}")));
            return;
        }
        if let Err(error) = crate::settings::parse_game_tags(&tag_input) {
            self.as_mut()
                .set_metadata_message(qstring(format!("Could not save tags: {error}")));
            return;
        }
        let custom_fields = match crate::settings::validate_game_custom_fields(&custom_fields) {
            Ok(custom_fields) => custom_fields,
            Err(error) => {
                self.as_mut().set_metadata_message(qstring(format!(
                    "Could not save custom fields: {error}"
                )));
                return;
            }
        };
        self.as_mut().rust_mut().metadata_generation =
            self.as_ref().rust().metadata_generation.wrapping_add(1);
        let generation = self.as_ref().rust().metadata_generation;
        let game_uid = self.as_ref().game_id().to_string();
        let launchbox_db_id = self.as_ref().rust().database_id;
        let canonical_title = self.as_ref().rust().canonical_title.clone();
        let platform = self.as_ref().platform().to_string();
        self.as_mut().set_metadata_busy(true);
        self.as_mut()
            .set_metadata_message(qstring("Saving local metadata…"));
        let qt_thread = self.as_ref().qt_thread();
        let saved_override = metadata_override.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-metadata-save".into())
            .spawn(move || {
                let result = SettingsStore::open_default()
                    .and_then(|store| {
                        store.save_game_metadata_tags_and_custom_fields(
                            &game_uid,
                            launchbox_db_id,
                            &canonical_title,
                            &platform,
                            &metadata_override,
                            &tag_input,
                            &custom_fields,
                        )
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_metadata_save(
                        generation,
                        effective,
                        saved_override,
                        reset_requested,
                        result,
                    );
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_metadata_busy(false);
            self.as_mut()
                .set_metadata_message(qstring(format!("Could not start metadata save: {error}")));
        }
    }

    fn finish_metadata_save(
        mut self: Pin<&mut Self>,
        generation: u64,
        effective: GameMetadata,
        metadata_override: GameMetadataOverride,
        reset_requested: bool,
        result: Result<(Vec<String>, Vec<GameCustomField>), String>,
    ) {
        if generation != self.as_ref().rust().metadata_generation {
            return;
        }
        self.as_mut().set_metadata_busy(false);
        match result {
            Ok((tags, custom_fields)) => {
                let metadata_is_canonical = metadata_override.is_empty();
                let tag_count = tags.len();
                let custom_field_count = custom_fields.len();
                self.as_mut().rust_mut().current_tags = tags;
                self.as_mut().set_tag_count(count_i32(tag_count));
                let tag_revision = self.as_ref().tag_revision().wrapping_add(1);
                self.as_mut().set_tag_revision(tag_revision);
                self.as_mut().rust_mut().current_custom_fields = custom_fields.clone();
                self.as_mut().rust_mut().metadata_custom_fields = custom_fields;
                self.as_mut()
                    .set_custom_field_count(count_i32(custom_field_count));
                self.as_mut()
                    .set_metadata_custom_field_count(count_i32(custom_field_count));
                let custom_field_revision = self.as_ref().custom_field_revision().wrapping_add(1);
                self.as_mut()
                    .set_custom_field_revision(custom_field_revision);
                let draft_revision = self
                    .as_ref()
                    .metadata_custom_field_revision()
                    .wrapping_add(1);
                self.as_mut()
                    .set_metadata_custom_field_revision(draft_revision);
                self.as_mut().rust_mut().effective_metadata = effective.clone();
                self.as_mut().set_title(qstring(&effective.title));
                self.as_mut()
                    .set_description(qstring(&effective.description));
                self.as_mut()
                    .set_release_date(qstring(format_release_date(&effective.release_date)));
                self.as_mut().set_developer(qstring(&effective.developer));
                self.as_mut().set_publisher(qstring(&effective.publisher));
                self.as_mut().set_genre(qstring(&effective.genre));
                self.as_mut().set_players(qstring(&effective.players));
                self.as_mut().set_rating(qstring(&effective.rating));
                self.as_mut().set_esrb(qstring(&effective.esrb));
                self.as_mut()
                    .set_release_type(qstring(&effective.release_type));
                self.as_mut().set_sort_title(qstring(&effective.sort_title));
                self.as_mut().set_series(qstring(&effective.series));
                self.as_mut().set_region(qstring(&effective.region));
                self.as_mut().set_play_mode(qstring(&effective.play_mode));
                self.as_mut().set_version(qstring(&effective.version));
                self.as_mut()
                    .set_release_status(qstring(&effective.release_status));
                self.as_mut()
                    .set_cooperative(qstring(&effective.cooperative));
                self.as_mut().set_notes(qstring(&effective.notes));
                self.as_mut()
                    .set_metadata_has_override(!metadata_is_canonical);
                let (metadata_message, message) = metadata_save_messages(
                    reset_requested,
                    metadata_is_canonical,
                    tag_count,
                    custom_field_count,
                );
                self.as_mut()
                    .set_metadata_message(qstring(metadata_message));
                self.as_mut().set_message(qstring(message));
                self.as_mut().set_metadata_open(false);
                let revision = self.as_ref().metadata_revision().wrapping_add(1);
                self.as_mut().set_metadata_revision(revision);
                self.as_mut().bump_revision();
            }
            Err(error) => self
                .as_mut()
                .set_metadata_message(qstring(format!("Could not save metadata: {error}"))),
        }
    }

    pub fn tag_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().current_tags.get(index))
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn custom_field_name_at(&self, index: i32) -> QString {
        custom_field_at(&self.rust().current_custom_fields, index)
            .map(|field| qstring(&field.name))
            .unwrap_or_default()
    }

    pub fn custom_field_value_at(&self, index: i32) -> QString {
        custom_field_at(&self.rust().current_custom_fields, index)
            .map(|field| qstring(&field.value))
            .unwrap_or_default()
    }

    pub fn metadata_custom_field_name_at(&self, index: i32) -> QString {
        custom_field_at(&self.rust().metadata_custom_fields, index)
            .map(|field| qstring(&field.name))
            .unwrap_or_default()
    }

    pub fn metadata_custom_field_value_at(&self, index: i32) -> QString {
        custom_field_at(&self.rust().metadata_custom_fields, index)
            .map(|field| qstring(&field.value))
            .unwrap_or_default()
    }

    pub fn add_metadata_custom_field(mut self: Pin<&mut Self>) {
        if !*self.as_ref().metadata_open() || *self.as_ref().metadata_busy() {
            return;
        }
        if self.as_ref().rust().metadata_custom_fields.len() >= 32 {
            self.as_mut()
                .set_metadata_message(qstring("A game can have at most 32 custom fields."));
            return;
        }
        self.as_mut()
            .rust_mut()
            .metadata_custom_fields
            .push(GameCustomField::default());
        let count = self.as_ref().rust().metadata_custom_fields.len();
        self.as_mut()
            .set_metadata_custom_field_count(count_i32(count));
        self.as_mut().bump_metadata_custom_field_revision();
        self.as_mut().set_metadata_message(qstring(
            "Name the new field and enter a value before saving.",
        ));
    }

    pub fn update_metadata_custom_field(
        mut self: Pin<&mut Self>,
        index: i32,
        name: QString,
        value: QString,
    ) {
        if !*self.as_ref().metadata_open() || *self.as_ref().metadata_busy() {
            return;
        }
        let Some(index) = usize::try_from(index).ok() else {
            return;
        };
        if let Some(field) = self
            .as_mut()
            .rust_mut()
            .metadata_custom_fields
            .get_mut(index)
        {
            field.name = name.to_string();
            field.value = value.to_string();
        }
        if self
            .as_ref()
            .rust()
            .metadata_custom_fields
            .iter()
            .all(|field| !field.name.trim().is_empty() && !field.value.trim().is_empty())
        {
            self.as_mut().set_metadata_message(qstring(
                "Custom fields are ready. Save changes to publish them atomically with metadata and tags.",
            ));
        }
    }

    pub fn remove_metadata_custom_field(mut self: Pin<&mut Self>, index: i32) {
        if !*self.as_ref().metadata_open() || *self.as_ref().metadata_busy() {
            return;
        }
        let Some(index) = usize::try_from(index).ok() else {
            return;
        };
        if index >= self.as_ref().rust().metadata_custom_fields.len() {
            return;
        }
        self.as_mut()
            .rust_mut()
            .metadata_custom_fields
            .remove(index);
        let count = self.as_ref().rust().metadata_custom_fields.len();
        self.as_mut()
            .set_metadata_custom_field_count(count_i32(count));
        self.as_mut().bump_metadata_custom_field_revision();
    }

    pub fn move_metadata_custom_field(mut self: Pin<&mut Self>, index: i32, direction: i32) {
        if !*self.as_ref().metadata_open() || *self.as_ref().metadata_busy() {
            return;
        }
        let Some(index) = usize::try_from(index).ok() else {
            return;
        };
        let Some(target) = i32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(direction))
            .and_then(|index| usize::try_from(index).ok())
        else {
            return;
        };
        let fields = &mut self.as_mut().rust_mut().metadata_custom_fields;
        if index >= fields.len() || target >= fields.len() || index == target {
            return;
        }
        fields.swap(index, target);
        self.as_mut().bump_metadata_custom_field_revision();
    }

    pub fn session_started_epoch_at(&self, index: i32) -> QString {
        self.session(index)
            .map(|session| qstring(session.started_at.to_string()))
            .unwrap_or_default()
    }

    pub fn session_emulator_at(&self, index: i32) -> QString {
        self.session(index)
            .map(|session| qstring(&session.emulator))
            .unwrap_or_default()
    }

    pub fn session_duration_at(&self, index: i32) -> QString {
        self.session(index)
            .map(|session| {
                qstring(format_session_duration(
                    session.duration_seconds,
                    &session.outcome,
                ))
            })
            .unwrap_or_default()
    }

    pub fn session_outcome_at(&self, index: i32) -> QString {
        self.session(index)
            .map(|session| qstring(&session.outcome))
            .unwrap_or_default()
    }

    pub fn session_outcome_label_at(&self, index: i32) -> QString {
        self.session(index)
            .map(|session| qstring(session_outcome_label(&session.outcome)))
            .unwrap_or_default()
    }

    pub fn report_session_history_probe(&self) {
        if has_cli_flag("--activity-history-ui-probe") {
            println!(
                "LUNCHBOX_ACTIVITY_HISTORY_UI_READY sessions={} outcomes={}",
                self.rust().sessions.len(),
                self.rust()
                    .sessions
                    .iter()
                    .map(|session| session.outcome.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
    }

    fn session(&self, index: i32) -> Option<&PlaySession> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().sessions.get(index))
    }

    fn clear_details(mut self: Pin<&mut Self>) {
        self.as_mut().set_description(QString::default());
        self.as_mut().set_release_date(QString::default());
        self.as_mut().set_developer(QString::default());
        self.as_mut().set_publisher(QString::default());
        self.as_mut().set_genre(QString::default());
        self.as_mut().set_players(QString::default());
        self.as_mut().set_rating(QString::default());
        self.as_mut().set_rating_count(0);
        self.as_mut().set_esrb(QString::default());
        self.as_mut().set_release_type(QString::default());
        self.as_mut().set_sort_title(QString::default());
        self.as_mut().set_series(QString::default());
        self.as_mut().set_region(QString::default());
        self.as_mut().set_play_mode(QString::default());
        self.as_mut().set_version(QString::default());
        self.as_mut().set_release_status(QString::default());
        self.as_mut().set_cooperative(qstring("unknown"));
        self.as_mut().set_catalog_video_url(QUrl::default());
        self.as_mut().set_wikipedia_url(QUrl::default());
        self.as_mut().set_steam_store_url(QUrl::default());
        self.as_mut().set_metadata_source(QString::default());
        self.as_mut().set_notes(QString::default());
        self.as_mut().set_metadata_tags(QString::default());
        self.as_mut().rust_mut().current_tags.clear();
        self.as_mut().set_tag_count(0);
        let tag_revision = self.as_ref().tag_revision().wrapping_add(1);
        self.as_mut().set_tag_revision(tag_revision);
        self.as_mut().rust_mut().current_custom_fields.clear();
        self.as_mut().rust_mut().metadata_custom_fields.clear();
        self.as_mut().set_custom_field_count(0);
        self.as_mut().set_metadata_custom_field_count(0);
        let custom_field_revision = self.as_ref().custom_field_revision().wrapping_add(1);
        self.as_mut()
            .set_custom_field_revision(custom_field_revision);
        self.as_mut().bump_metadata_custom_field_revision();
        self.as_mut().set_variant_count(0);
        self.as_mut().set_alternate_title_count(0);
        self.as_mut().set_related_game_count(0);
        self.as_mut().set_related_game_message(QString::default());
        let related_revision = self.as_ref().related_game_revision().wrapping_add(1);
        self.as_mut().set_related_game_revision(related_revision);
        self.as_mut().set_media_visible(false);
        self.as_mut().set_video_available(false);
        self.as_mut().set_manual_available(false);
        self.as_mut().set_soundtrack_available(false);
        self.as_mut().set_soundtrack_count(0);
        self.as_mut().set_soundtrack_index(-1);
        self.as_mut().set_soundtrack_url(QUrl::default());
        self.as_mut().set_soundtrack_title(QString::default());
        self.as_mut().set_soundtrack_source(QString::default());
        self.as_mut().rust_mut().soundtrack_tracks.clear();
        self.as_mut().set_manual_transfer_active(false);
        self.as_mut().set_manual_action_busy(false);
        self.as_mut().set_manual_download_state(QString::default());
        self.as_mut().set_manual_download_progress(0);
        self.as_mut().set_manual_download_detail(QString::default());
        self.as_mut()
            .set_manual_download_message(QString::default());
        self.as_mut().set_video_progress_busy(false);
        self.as_mut().set_video_url(QUrl::default());
        self.as_mut().set_manual_url(QUrl::default());
        self.as_mut().set_video_source(QString::default());
        self.as_mut().set_manual_source(QString::default());
        self.as_mut().set_media_message(QString::default());
        self.as_mut().set_video_resume_position(0);
        self.as_mut().rust_mut().video_media_key.clear();
        let media_revision = self.as_ref().media_revision().wrapping_add(1);
        self.as_mut().set_media_revision(media_revision);
        self.as_mut().set_activity_busy(false);
        self.as_mut().set_activity_visible(false);
        self.as_mut().set_play_count(0);
        self.as_mut().set_play_time(qstring("Never played"));
        self.as_mut().set_last_played(qstring("Never"));
        self.as_mut().set_completion_state(qstring("not_started"));
        self.as_mut().rust_mut().sessions.clear();
        self.as_mut().set_session_count(0);
        let session_revision = self.as_ref().session_history_revision().wrapping_add(1);
        self.as_mut().set_session_history_revision(session_revision);
        self.as_mut().set_preparable(false);
        self.as_mut().set_prepared(false);
        self.as_mut().set_exo_media_imported(false);
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
        self.as_mut().rust_mut().preferred_local_file_path = None;
        self.as_mut().rust_mut().rom_emulator_options.clear();
        self.as_mut().rust_mut().rom_firmware_statuses.clear();
        self.as_mut().rust_mut().game_emulator_preference = None;
        self.as_mut().rust_mut().platform_emulator_preference = None;
        self.as_mut().rust_mut().prepared_emulator = None;
        self.as_mut().rust_mut().prepared_install = None;
        self.as_mut().rust_mut().bundles.clear();
        self.as_mut().rust_mut().bundle_candidates.clear();
        self.as_mut().rust_mut().variants.clear();
        self.as_mut().rust_mut().alternate_titles.clear();
        self.as_mut().rust_mut().related_games.clear();
        self.as_mut().rust_mut().files.clear();
        self.as_mut().set_bundle_count(0);
        self.as_mut().set_registered_torrent_source_count(0);
        self.as_mut().set_file_count(0);
        self.as_mut().set_selected_bundle(-1);
        self.as_mut().rust_mut().download_preflight_generation = self
            .as_ref()
            .rust()
            .download_preflight_generation
            .wrapping_add(1);
        self.as_mut().set_download_preflight_busy(false);
        self.as_mut().set_download_preflight_ready(false);
        self.as_mut().set_download_review_index(-1);
        self.as_mut()
            .set_download_preflight_status(QString::default());
        self.as_mut()
            .set_download_preflight_storage(QString::default());
        self.as_mut()
            .set_download_preflight_destination(QString::default());
        self.as_mut()
            .set_download_preflight_mode(QString::default());
        self.as_mut()
            .set_download_preflight_action(QString::default());
        self.as_mut().set_local_file_count(0);
        self.as_mut().set_selected_local_file(-1);
        self.as_mut()
            .set_selected_local_directory_url(QUrl::default());
        self.as_mut().set_local_file_preference_configured(false);
        self.as_mut().set_selected_local_file_is_preferred(false);
        self.as_mut().set_local_file_preference_stale(false);
        self.as_mut()
            .set_local_file_preference_message(QString::default());
        let local_file_revision = self.as_ref().local_file_revision().wrapping_add(1);
        self.as_mut().set_local_file_revision(local_file_revision);
        self.as_mut().set_install_management_busy(false);
        self.as_mut().set_managed_install_present(false);
        self.as_mut().set_managed_install_can_delete(false);
        self.as_mut().set_managed_install_owned_count(0);
        self.as_mut().set_managed_install_shared_count(0);
        self.as_mut()
            .set_install_management_message(QString::default());
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
        match loaded {
            Ok(details) => {
                let activity = details.activity.clone();
                let sessions = details.sessions.clone();
                let supplemental_media = details.supplemental_media.clone();
                let video_media_key = details.video_media_key.clone();
                let video_progress = details.video_progress.clone();
                let manual_transfer = details.manual_transfer.clone();
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
                let selected_local_file = local_file_paths
                    .iter()
                    .position(|path| path == &details.local_file_path);
                let selected_local_path =
                    selected_local_file.and_then(|index| local_file_paths.get(index));
                let preferred_local_file_path = details.preferred_local_file_path.clone();
                let local_file_preference_stale = details.local_file_preference_stale;
                let local_file_preference_message = local_file_preference_message(
                    preferred_local_file_path.as_deref(),
                    local_file_preference_stale,
                    selected_local_path.map(PathBuf::as_path),
                    local_file_count,
                );
                let selected_local_file_is_preferred = selected_local_path.is_some()
                    && selected_local_path.map(PathBuf::as_path)
                        == preferred_local_file_path.as_deref();
                let managed_installation = details.managed_installation.clone();
                self.as_mut().set_title(qstring(&details.title));
                self.as_mut().set_description(qstring(&details.description));
                self.as_mut()
                    .set_release_date(qstring(format_release_date(&details.release_date)));
                self.as_mut().set_developer(qstring(&details.developer));
                self.as_mut().set_publisher(qstring(&details.publisher));
                self.as_mut().set_genre(qstring(&details.genre));
                self.as_mut().set_players(qstring(&details.players));
                self.as_mut().set_rating(qstring(&details.rating));
                self.as_mut()
                    .set_rating_count(i32::try_from(details.rating_count).unwrap_or(i32::MAX));
                self.as_mut().set_esrb(qstring(&details.esrb));
                self.as_mut()
                    .set_release_type(qstring(&details.release_type));
                self.as_mut().set_sort_title(qstring(&details.sort_title));
                self.as_mut().set_series(qstring(&details.series));
                self.as_mut().set_region(qstring(&details.region));
                self.as_mut().set_play_mode(qstring(&details.play_mode));
                self.as_mut().set_version(qstring(&details.version));
                self.as_mut()
                    .set_release_status(qstring(&details.release_status));
                self.as_mut().set_cooperative(qstring(&details.cooperative));
                self.as_mut()
                    .set_catalog_video_url(catalog_url(&details.catalog_video_url));
                self.as_mut()
                    .set_wikipedia_url(catalog_url(&details.wikipedia_url));
                self.as_mut()
                    .set_steam_store_url(steam_store_url(details.steam_app_id));
                self.as_mut()
                    .set_metadata_source(qstring(metadata_source_label(&details.metadata_source)));
                self.as_mut().set_notes(qstring(&details.notes));
                let tag_count = details.tags.len();
                self.as_mut().rust_mut().current_tags = details.tags.clone();
                self.as_mut().set_tag_count(count_i32(tag_count));
                let tag_revision = self.as_ref().tag_revision().wrapping_add(1);
                self.as_mut().set_tag_revision(tag_revision);
                let custom_field_count = details.custom_fields.len();
                self.as_mut().rust_mut().current_custom_fields = details.custom_fields;
                self.as_mut()
                    .set_custom_field_count(count_i32(custom_field_count));
                let custom_field_revision = self.as_ref().custom_field_revision().wrapping_add(1);
                self.as_mut()
                    .set_custom_field_revision(custom_field_revision);
                let has_override = !GameMetadataOverride::from_effective(
                    &details.canonical_metadata,
                    &details.effective_metadata,
                )
                .is_empty();
                self.as_mut().set_metadata_has_override(has_override);
                self.as_mut().rust_mut().canonical_title = details.canonical_metadata.title.clone();
                self.as_mut().rust_mut().canonical_metadata = details.canonical_metadata.clone();
                self.as_mut().rust_mut().effective_metadata = details.effective_metadata.clone();
                let variant_count = details.variants.len();
                self.as_mut().rust_mut().variants = details.variants.clone();
                self.as_mut().set_variant_count(count_i32(variant_count));
                let alternate_title_count = details.alternate_titles.len();
                self.as_mut().rust_mut().alternate_titles = details.alternate_titles.clone();
                self.as_mut()
                    .set_alternate_title_count(count_i32(alternate_title_count));
                let related_game_count = details.related_games.len();
                self.as_mut().rust_mut().related_games = details.related_games.clone();
                self.as_mut()
                    .set_related_game_count(count_i32(related_game_count));
                self.as_mut()
                    .set_related_game_message(qstring(&details.related_message));
                let related_revision = self.as_ref().related_game_revision().wrapping_add(1);
                self.as_mut().set_related_game_revision(related_revision);
                self.as_mut().apply_supplemental_media(
                    supplemental_media,
                    video_media_key,
                    video_progress,
                    manual_transfer,
                );
                self.as_mut().rust_mut().database_id = details.database_id;
                self.as_mut().set_local(details.local);
                self.as_mut().set_downloadable(details.downloadable);
                self.as_mut().apply_play_activity(activity, details.local);
                self.as_mut().apply_play_sessions(sessions);
                self.as_mut().rust_mut().local_file_path = details.local_file_path;
                let selected_local_directory_url = selected_local_file
                    .and_then(|index| local_file_paths.get(index))
                    .map(|path| local_directory_url(path))
                    .unwrap_or_default();
                self.as_mut().rust_mut().local_file_paths = local_file_paths;
                self.as_mut().rust_mut().preferred_local_file_path =
                    preferred_local_file_path.clone();
                self.as_mut()
                    .set_local_file_count(count_i32(local_file_count));
                self.as_mut().set_selected_local_file(
                    selected_local_file
                        .and_then(|index| i32::try_from(index).ok())
                        .unwrap_or(-1),
                );
                self.as_mut()
                    .set_selected_local_directory_url(selected_local_directory_url);
                self.as_mut()
                    .set_local_file_preference_configured(preferred_local_file_path.is_some());
                self.as_mut()
                    .set_selected_local_file_is_preferred(selected_local_file_is_preferred);
                self.as_mut()
                    .set_local_file_preference_stale(local_file_preference_stale);
                self.as_mut()
                    .set_local_file_preference_message(qstring(local_file_preference_message));
                let local_file_revision = self.as_ref().local_file_revision().wrapping_add(1);
                self.as_mut().set_local_file_revision(local_file_revision);
                if let Some(managed) = managed_installation {
                    self.as_mut().set_managed_install_present(true);
                    self.as_mut()
                        .set_managed_install_can_delete(managed.owned_file_count > 0);
                    self.as_mut()
                        .set_managed_install_owned_count(count_i32(managed.owned_file_count));
                    self.as_mut()
                        .set_managed_install_shared_count(count_i32(managed.shared_file_count));
                    let message = if managed.receipt_count == 0 {
                        "This predates ownership receipts. Removing it will keep every file."
                            .to_owned()
                    } else if managed.owned_file_count == 0 {
                        "Lunchbox does not own these files. Removing it will keep every file."
                            .to_owned()
                    } else if managed.shared_file_count > 0 {
                        format!(
                            "{} verified file{} can be removed; {} shared file{} will stay.",
                            managed.deletable_file_count,
                            if managed.deletable_file_count == 1 {
                                ""
                            } else {
                                "s"
                            },
                            managed.shared_file_count,
                            if managed.shared_file_count == 1 {
                                ""
                            } else {
                                "s"
                            }
                        )
                    } else {
                        format!(
                            "{} Lunchbox-owned file{} will be verified before removal.",
                            managed.owned_file_count,
                            if managed.owned_file_count == 1 {
                                ""
                            } else {
                                "s"
                            }
                        )
                    };
                    self.as_mut()
                        .set_install_management_message(qstring(message));
                } else {
                    self.as_mut().set_managed_install_present(false);
                    self.as_mut().set_managed_install_can_delete(false);
                    self.as_mut().set_managed_install_owned_count(0);
                    self.as_mut().set_managed_install_shared_count(0);
                    self.as_mut()
                        .set_install_management_message(QString::default());
                }
                self.as_mut().rust_mut().prepared_install = prepared_install;
                self.as_mut().set_preparable(preparable);
                self.as_mut().set_prepared(prepared);
                self.as_mut()
                    .set_exo_media_imported(details.exo_media_imported);
                self.as_mut()
                    .set_prepared_summary(qstring(prepared_summary));
                self.as_mut().set_registered_torrent_source_count(count_i32(
                    details.registered_torrent_source_count,
                ));
                let bundle_count = details.bundles.len();
                self.as_mut().rust_mut().bundles = details.bundles;
                self.as_mut().rust_mut().bundle_candidates =
                    vec![BundleCandidateGroup::default(); bundle_count];
                self.as_mut().set_bundle_count(count_i32(bundle_count));
                self.as_mut().bump_revision();
                let message = if details.local {
                    "Installed in your collection".to_owned()
                } else if bundle_count == 0 {
                    "No matching download source is registered for this game.".to_owned()
                } else {
                    format!("{bundle_count} torrent source bundles available")
                };
                self.as_mut().set_message(qstring(message));
                self.as_mut().set_loading(false);
                if bundle_count > 0 {
                    self.as_mut().load_all_bundle_files();
                }
                if prepared || (local_file_count > 0 && !preparable) {
                    self.as_mut().refresh_emulators();
                }
            }
            Err(error) => {
                if has_cli_flag("--activity-history-ui-probe") {
                    eprintln!("LUNCHBOX_ACTIVITY_HISTORY_MODEL_FAILED error={error}");
                }
                self.as_mut()
                    .set_message(qstring(format!("Could not load game details: {error}")));
                self.as_mut().set_loading(false);
            }
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

    fn apply_play_sessions(mut self: Pin<&mut Self>, sessions: Vec<PlaySession>) {
        let count = sessions.len();
        if has_cli_flag("--activity-history-ui-probe") {
            println!(
                "LUNCHBOX_ACTIVITY_HISTORY_MODEL_READY sessions={} outcomes={}",
                count,
                sessions
                    .iter()
                    .map(|session| session.outcome.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        self.as_mut().rust_mut().sessions = sessions;
        self.as_mut().set_session_count(count_i32(count));
        let revision = self.as_ref().session_history_revision().wrapping_add(1);
        self.as_mut().set_session_history_revision(revision);
    }

    fn apply_supplemental_media(
        mut self: Pin<&mut Self>,
        media: crate::media::SupplementalMedia,
        video_media_key: String,
        video_progress: Option<crate::settings::MediaPlaybackProgress>,
        manual_transfer: Option<crate::settings::MediaTransfer>,
    ) {
        let video_available = media.video.is_some();
        let manual_available = media.manual.is_some();
        let soundtrack_available = !media.soundtrack.is_empty();
        self.as_mut().set_media_visible(true);
        self.as_mut().set_video_available(video_available);
        self.as_mut().set_manual_available(manual_available);
        self.as_mut().set_soundtrack_available(soundtrack_available);
        self.as_mut().set_video_url(
            media
                .video
                .as_ref()
                .map(|asset| local_file_url(&asset.path))
                .unwrap_or_default(),
        );
        self.as_mut().set_manual_url(
            media
                .manual
                .as_ref()
                .map(|asset| local_file_url(&asset.path))
                .unwrap_or_default(),
        );
        self.as_mut().set_video_source(qstring(
            media
                .video
                .as_ref()
                .map(|asset| asset.source.as_str())
                .unwrap_or_default(),
        ));
        self.as_mut().set_manual_source(qstring(
            media
                .manual
                .as_ref()
                .map(|asset| asset.source.as_str())
                .unwrap_or_default(),
        ));
        let soundtrack_count = count_i32(media.soundtrack.len());
        self.as_mut().rust_mut().soundtrack_tracks = media.soundtrack;
        self.as_mut().set_soundtrack_count(soundtrack_count);
        if soundtrack_count > 0 {
            self.as_mut().select_soundtrack(0);
        } else {
            self.as_mut().set_soundtrack_index(-1);
            self.as_mut().set_soundtrack_url(QUrl::default());
            self.as_mut().set_soundtrack_title(QString::default());
            self.as_mut().set_soundtrack_source(QString::default());
        }
        self.as_mut().rust_mut().video_media_key = video_media_key;
        let resume_position = video_progress
            .filter(|progress| {
                progress.position_ms >= 5_000
                    && (progress.duration_ms == 0
                        || progress.duration_ms.saturating_sub(progress.position_ms) > 10_000)
            })
            .map(|progress| i32::try_from(progress.position_ms).unwrap_or(i32::MAX))
            .unwrap_or_default();
        self.as_mut().set_video_resume_position(resume_position);
        self.as_mut().apply_manual_transfer(manual_transfer);
        let mut ready = Vec::with_capacity(3);
        if video_available {
            ready.push("gameplay video");
        }
        if manual_available {
            ready.push("manual");
        }
        if soundtrack_available {
            ready.push("game music");
        }
        self.as_mut()
            .set_media_message(qstring(if ready.is_empty() {
                "Cached media is not available yet.".to_owned()
            } else {
                format!("{} ready.", ready.join(", "))
            }));
        let media_revision = self.as_ref().media_revision().wrapping_add(1);
        self.as_mut().set_media_revision(media_revision);
    }

    pub fn select_soundtrack(mut self: Pin<&mut Self>, index: i32) {
        let track = {
            let model = self.as_ref();
            let rust = model.rust();
            usize::try_from(index)
                .ok()
                .and_then(|index| rust.soundtrack_tracks.get(index))
                .cloned()
        };
        let Some(track) = track else {
            self.as_mut()
                .set_media_message(qstring("That soundtrack track is no longer cached."));
            return;
        };
        self.as_mut().set_soundtrack_index(index);
        self.as_mut()
            .set_soundtrack_url(local_file_url(&track.path));
        self.as_mut().set_soundtrack_title(qstring(track.title));
        self.as_mut().set_soundtrack_source(qstring(track.source));
    }

    pub fn report_soundtrack_ui_probe(&self, screenshot: QString) {
        if std::env::args().any(|argument| argument == "--soundtrack-ui-probe")
            && *self.soundtrack_available()
            && *self.soundtrack_count() > 0
            && !self.soundtrack_url().is_empty()
        {
            println!(
                "LUNCHBOX_SOUNDTRACK_UI_READY title={:?} source={:?} count={} url={:?} screenshot={:?}",
                self.soundtrack_title().to_string(),
                self.soundtrack_source().to_string(),
                self.soundtrack_count(),
                self.soundtrack_url().to_string(),
                screenshot.to_string()
            );
        }
    }

    fn apply_manual_transfer(
        mut self: Pin<&mut Self>,
        transfer: Option<crate::settings::MediaTransfer>,
    ) {
        let Some(transfer) = transfer else {
            self.as_mut().set_manual_transfer_active(false);
            self.as_mut()
                .set_manual_download_state(qstring("not_started"));
            self.as_mut().set_manual_download_progress(0);
            self.as_mut().set_manual_download_detail(QString::default());
            self.as_mut().set_manual_download_message(qstring(
                "Search Minerva's manual-scan archive for an exact title match.",
            ));
            return;
        };

        self.as_mut()
            .set_manual_transfer_active(crate::media_acquisition::transfer_needs_refresh(
                &transfer.state,
            ));
        self.as_mut()
            .set_manual_download_state(qstring(&transfer.state));
        self.as_mut().set_manual_download_progress(
            (transfer.progress.clamp(0.0, 1.0) * 100.0).round() as i32,
        );
        let mut detail = if transfer.total_bytes > 0 {
            format!(
                "{} / {}",
                game_details::format_bytes(transfer.downloaded_bytes),
                game_details::format_bytes(transfer.total_bytes)
            )
        } else {
            String::new()
        };
        if transfer.download_speed > 0 {
            if !detail.is_empty() {
                detail.push_str(" · ");
            }
            detail.push_str(&format!(
                "{}/s",
                game_details::format_bytes(transfer.download_speed)
            ));
        }
        self.as_mut().set_manual_download_detail(qstring(detail));
        self.as_mut()
            .set_manual_download_message(qstring(transfer.message));
    }

    pub fn refresh_media(mut self: Pin<&mut Self>) {
        if !*self.as_ref().panel_open()
            || *self.as_ref().loading()
            || self.as_ref().game_id().is_empty()
        {
            return;
        }
        self.as_mut().rust_mut().media_refresh_generation = self
            .as_ref()
            .rust()
            .media_refresh_generation
            .wrapping_add(1);
        let refresh_generation = self.as_ref().rust().media_refresh_generation;
        let details_generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        let database_id = self.as_ref().rust().database_id;
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-game-media-refresh".into())
            .spawn(move || {
                let result = load_supplemental_media_refresh(&game_id, database_id)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_media_refresh(
                        details_generation,
                        refresh_generation,
                        result,
                    );
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_media_message(qstring(format!(
                "Could not start the media refresh: {error}"
            )));
        }
    }

    fn finish_media_refresh(
        mut self: Pin<&mut Self>,
        details_generation: u64,
        refresh_generation: u64,
        result: Result<SupplementalMediaRefresh, String>,
    ) {
        if details_generation != self.as_ref().rust().details_generation
            || refresh_generation != self.as_ref().rust().media_refresh_generation
        {
            return;
        }
        match result {
            Ok(refresh) if refresh.game_id == self.as_ref().game_id().to_string() => {
                self.as_mut().apply_supplemental_media(
                    refresh.media,
                    refresh.video_media_key,
                    refresh.video_progress,
                    refresh.manual_transfer,
                );
            }
            Ok(_) => {}
            Err(error) => self
                .as_mut()
                .set_media_message(qstring(format!("Could not refresh game media: {error}"))),
        }
    }

    pub fn download_manual(mut self: Pin<&mut Self>) {
        if *self.as_ref().manual_action_busy()
            || *self.as_ref().manual_available()
            || self.as_ref().game_id().is_empty()
        {
            return;
        }
        let generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        let completed_game_id = game_id.clone();
        let database_id = self.as_ref().rust().database_id;
        let title = self.as_ref().rust().canonical_title.clone();
        let platform = self.as_ref().platform().to_string();
        self.as_mut().set_manual_action_busy(true);
        self.as_mut()
            .set_manual_download_state(qstring("searching"));
        self.as_mut()
            .set_manual_download_message(qstring("Searching Minerva for an exact manual…"));

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-manual-enqueue".into())
            .spawn(move || {
                let result = crate::media_acquisition::enqueue_manual(
                    &game_id,
                    database_id,
                    &title,
                    &platform,
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_manual_action(generation, completed_game_id, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_manual_action_busy(false);
            self.as_mut().set_manual_download_state(qstring("error"));
            self.as_mut().set_manual_download_message(qstring(format!(
                "Could not start the manual search: {error}"
            )));
        }
    }

    fn finish_manual_action(
        mut self: Pin<&mut Self>,
        generation: u64,
        game_id: String,
        result: Result<crate::settings::MediaTransfer, String>,
    ) {
        if generation != self.as_ref().rust().details_generation
            || game_id != self.as_ref().game_id().to_string()
        {
            return;
        }
        self.as_mut().set_manual_action_busy(false);
        match result {
            Ok(transfer) => self.as_mut().apply_manual_transfer(Some(transfer)),
            Err(error) => {
                self.as_mut().set_manual_transfer_active(false);
                self.as_mut().set_manual_download_state(qstring("error"));
                self.as_mut().set_manual_download_progress(0);
                self.as_mut().set_manual_download_detail(QString::default());
                self.as_mut().set_manual_download_message(qstring(error));
            }
        }
    }

    pub fn refresh_manual_download(mut self: Pin<&mut Self>) {
        if *self.as_ref().manual_action_busy()
            || !*self.as_ref().manual_transfer_active()
            || self.as_ref().game_id().is_empty()
        {
            return;
        }
        let generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        let completed_game_id = game_id.clone();
        self.as_mut().set_manual_action_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-manual-refresh".into())
            .spawn(move || {
                let result = crate::media_acquisition::refresh_manual(&game_id)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_manual_refresh(generation, completed_game_id, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_manual_action_busy(false);
            self.as_mut().set_manual_download_message(qstring(format!(
                "Could not refresh the manual download: {error}"
            )));
        }
    }

    fn finish_manual_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        game_id: String,
        result: Result<crate::media_acquisition::MediaRefresh, String>,
    ) {
        if generation != self.as_ref().rust().details_generation
            || game_id != self.as_ref().game_id().to_string()
        {
            return;
        }
        self.as_mut().set_manual_action_busy(false);
        match result {
            Ok(refresh) => {
                let published = refresh.published;
                self.as_mut().apply_manual_transfer(Some(refresh.transfer));
                if published {
                    let game_id = self.as_ref().game_id().clone();
                    let title = qstring(&self.as_ref().rust().canonical_title);
                    let platform = self.as_ref().platform().clone();
                    let local = *self.as_ref().local();
                    let downloadable = *self.as_ref().downloadable();
                    self.as_mut()
                        .select_game(game_id, title, platform, local, downloadable);
                }
            }
            Err(error) => self
                .as_mut()
                .set_manual_download_message(qstring(format!("Download check failed: {error}"))),
        }
    }

    pub fn cancel_manual_download(mut self: Pin<&mut Self>) {
        if *self.as_ref().manual_action_busy()
            || !*self.as_ref().manual_transfer_active()
            || self.as_ref().game_id().is_empty()
        {
            return;
        }
        let generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        let completed_game_id = game_id.clone();
        self.as_mut().set_manual_action_busy(true);
        self.as_mut()
            .set_manual_download_message(qstring("Cancelling this manual download…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-manual-cancel".into())
            .spawn(move || {
                let result = crate::media_acquisition::cancel_manual(&game_id)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_manual_action(generation, completed_game_id, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_manual_action_busy(false);
            self.as_mut().set_manual_download_message(qstring(format!(
                "Could not start manual cancellation: {error}"
            )));
        }
    }

    pub fn save_video_progress(mut self: Pin<&mut Self>, position_ms: i32, duration_ms: i32) {
        self.as_mut()
            .persist_video_progress(position_ms, duration_ms, false);
    }

    pub fn reset_video_progress(mut self: Pin<&mut Self>, duration_ms: i32) {
        self.as_mut()
            .persist_video_progress(0, duration_ms.max(0), true);
    }

    fn persist_video_progress(
        mut self: Pin<&mut Self>,
        position_ms: i32,
        duration_ms: i32,
        reset: bool,
    ) {
        if *self.as_ref().video_progress_busy() || !*self.as_ref().video_available() {
            return;
        }
        let game_id = self.as_ref().game_id().to_string();
        let media_key = self.as_ref().rust().video_media_key.clone();
        if game_id.trim().is_empty() || media_key.is_empty() {
            return;
        }
        let duration_ms = duration_ms.max(0);
        let mut position_ms = position_ms.max(0);
        if duration_ms > 0 {
            position_ms = position_ms.min(duration_ms);
            if duration_ms.saturating_sub(position_ms) <= 10_000 {
                position_ms = 0;
            }
        }
        let generation = self.as_ref().rust().details_generation;
        self.as_mut().set_video_progress_busy(true);
        if reset {
            self.as_mut().set_video_resume_position(0);
        }

        let qt_thread = self.as_ref().qt_thread();
        let worker_media_key = media_key.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-video-progress".into())
            .spawn(move || {
                let saved = crate::settings::SettingsStore::open_default()
                    .and_then(|store| {
                        store.save_media_playback_progress(
                            &game_id,
                            &worker_media_key,
                            i64::from(position_ms),
                            i64::from(duration_ms),
                        )
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_video_progress_save(generation, media_key, saved);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_video_progress_busy(false);
            self.as_mut().set_media_message(qstring(format!(
                "Could not start video progress worker: {error}"
            )));
        }
    }

    fn finish_video_progress_save(
        mut self: Pin<&mut Self>,
        generation: u64,
        media_key: String,
        saved: Result<crate::settings::MediaPlaybackProgress, String>,
    ) {
        if generation != self.as_ref().rust().details_generation
            || media_key != self.as_ref().rust().video_media_key
        {
            return;
        }
        self.as_mut().set_video_progress_busy(false);
        match saved {
            Ok(_) => {}
            Err(error) => self
                .as_mut()
                .set_media_message(qstring(format!("Could not save video position: {error}"))),
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
        let title = self.as_ref().rust().canonical_title.clone();
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
                    .and_then(|store| {
                        Ok((
                            store.play_activity(&game_id)?,
                            store.play_sessions(&game_id, 200)?,
                        ))
                    })
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
        result: Result<(Option<crate::settings::PlayActivity>, Vec<PlaySession>), String>,
    ) {
        if generation != self.as_ref().rust().activity_load_generation
            || details_generation != self.as_ref().rust().details_generation
            || completed_game_id != self.as_ref().game_id().to_string()
        {
            return;
        }
        match result {
            Ok((activity, sessions)) => {
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
                self.as_mut().apply_play_sessions(sessions);
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
                "The selected torrent source is no longer available.",
            ));
            return;
        };
        let bundle = self.as_ref().rust().bundles.get(bundle_index).cloned();
        let Some(bundle) = bundle else {
            self.as_mut().set_message(qstring(
                "The selected torrent source is no longer available.",
            ));
            return;
        };
        if let Some(group) = self
            .as_ref()
            .rust()
            .bundle_candidates
            .get(bundle_index)
            .filter(|group| group.loaded)
            .cloned()
        {
            let count = group.files.len();
            let message = bundle_group_message(&group, "this source");
            self.as_mut().rust_mut().files = group.files;
            self.as_mut().set_file_count(count_i32(count));
            self.as_mut().set_selected_bundle(index);
            self.as_mut().bump_revision();
            self.as_mut().set_message(qstring(message));
            return;
        }
        if *self.as_ref().torrent_loading() {
            return;
        }
        self.as_mut().rust_mut().torrent_generation =
            self.as_ref().rust().torrent_generation.wrapping_add(1);
        let generation = self.as_ref().rust().torrent_generation;
        let title = self.as_ref().rust().canonical_title.clone();
        let alternate_titles = self.as_ref().rust().alternate_titles.clone();
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
                        &alternate_titles,
                        &ReleasePreferences {
                            region_priority: settings.region_priority,
                            version_preference: settings.version_preference,
                        },
                    )
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_torrent_files(generation, bundle_index, loaded);
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
        bundle_index: usize,
        loaded: Result<Vec<TorrentFileCandidate>, String>,
    ) {
        if generation != self.as_ref().rust().torrent_generation {
            return;
        }
        self.as_mut().set_torrent_loading(false);
        match loaded {
            Ok(files) => {
                let count = files.len();
                if let Some(group) = self
                    .as_mut()
                    .rust_mut()
                    .bundle_candidates
                    .get_mut(bundle_index)
                {
                    group.loaded = true;
                    group.files = files.clone();
                    group.error.clear();
                }
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
            Err(error) => {
                if let Some(group) = self
                    .as_mut()
                    .rust_mut()
                    .bundle_candidates
                    .get_mut(bundle_index)
                {
                    group.loaded = true;
                    group.files.clear();
                    group.error.clone_from(&error);
                }
                self.as_mut().bump_revision();
                self.as_mut().set_message(qstring(format!(
                    "Could not inspect torrent contents: {error}"
                )));
            }
        }
    }

    fn load_all_bundle_files(mut self: Pin<&mut Self>) {
        if self.as_ref().rust().bundles.is_empty() || *self.as_ref().torrent_loading() {
            return;
        }
        self.as_mut().rust_mut().torrent_generation =
            self.as_ref().rust().torrent_generation.wrapping_add(1);
        let generation = self.as_ref().rust().torrent_generation;
        let bundles = self.as_ref().rust().bundles.clone();
        let title = self.as_ref().rust().canonical_title.clone();
        let alternate_titles = self.as_ref().rust().alternate_titles.clone();
        self.as_mut().set_torrent_loading(true);
        self.as_mut().set_message(qstring(format!(
            "Finding the best download across {} torrent source{}…",
            bundles.len(),
            if bundles.len() == 1 { "" } else { "s" }
        )));
        self.as_mut().bump_revision();

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-torrent-catalog".into())
            .spawn(move || {
                let preferences = crate::settings::SettingsStore::open_default()
                    .and_then(|store| store.load())
                    .map(|settings| ReleasePreferences {
                        region_priority: settings.region_priority,
                        version_preference: settings.version_preference,
                    })
                    .map_err(|error| error.to_string());
                let bundles = Arc::new(bundles);
                let title = Arc::new(title);
                let alternate_titles = Arc::new(alternate_titles);
                let preferences = Arc::new(preferences);
                let next_bundle = AtomicUsize::new(0);
                let bundle_count = bundles.len();
                let worker_count = bundle_count.min(4);
                let (sender, receiver) = std::sync::mpsc::channel();
                std::thread::scope(|scope| {
                    for _ in 0..worker_count {
                        let sender = sender.clone();
                        let bundles = Arc::clone(&bundles);
                        let title = Arc::clone(&title);
                        let alternate_titles = Arc::clone(&alternate_titles);
                        let preferences = Arc::clone(&preferences);
                        let next_bundle = &next_bundle;
                        scope.spawn(move || {
                            loop {
                                let bundle_index =
                                    next_bundle.fetch_add(1, AtomicOrdering::Relaxed);
                                let Some(bundle) = bundles.get(bundle_index) else {
                                    break;
                                };
                                let loaded = match preferences.as_ref() {
                                    Ok(preferences) => game_details::load_torrent_files(
                                        bundle,
                                        &title,
                                        &alternate_titles,
                                        preferences,
                                    )
                                    .map_err(|error| error.to_string()),
                                    Err(error) => Err(error.clone()),
                                };
                                if sender.send((bundle_index, loaded)).is_err() {
                                    break;
                                }
                            }
                        });
                    }
                    drop(sender);
                    let mut completed = 0;
                    for (bundle_index, loaded) in receiver {
                        completed += 1;
                        let _ = qt_thread.queue(move |mut model| {
                            model.as_mut().finish_bundle_files(
                                generation,
                                bundle_index,
                                loaded,
                                completed,
                                bundle_count,
                            );
                        });
                    }
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_torrent_loading(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start torrent catalog worker: {error}"
            )));
        }
    }

    fn finish_bundle_files(
        mut self: Pin<&mut Self>,
        generation: u64,
        bundle_index: usize,
        loaded: Result<Vec<TorrentFileCandidate>, String>,
        completed: usize,
        bundle_count: usize,
    ) {
        if generation != self.as_ref().rust().torrent_generation {
            return;
        }
        let group = match loaded {
            Ok(files) => BundleCandidateGroup {
                loaded: true,
                files,
                error: String::new(),
            },
            Err(error) => BundleCandidateGroup {
                loaded: true,
                files: Vec::new(),
                error,
            },
        };
        if let Some(target) = self
            .as_mut()
            .rust_mut()
            .bundle_candidates
            .get_mut(bundle_index)
        {
            *target = group;
        }
        let preferred_group = (*self.as_ref().selected_bundle() < 0)
            .then(|| {
                preferred_loaded_group_index(&self.as_ref().rust().bundle_candidates).map(|index| {
                    (
                        index,
                        self.as_ref().rust().bundle_candidates[index].files.clone(),
                    )
                })
            })
            .flatten();
        if let Some((preferred_index, files)) = preferred_group {
            let file_count = files.len();
            self.as_mut().rust_mut().files = files;
            self.as_mut().set_file_count(count_i32(file_count));
            self.as_mut()
                .set_selected_bundle(i32::try_from(preferred_index).unwrap_or(-1));
        }

        let candidate_count = self
            .as_ref()
            .rust()
            .bundle_candidates
            .iter()
            .map(|group| group.files.len())
            .sum::<usize>();
        let error_count = self
            .as_ref()
            .rust()
            .bundle_candidates
            .iter()
            .filter(|group| !group.error.is_empty())
            .count();
        let pending = bundle_count.saturating_sub(completed);
        let message = if pending > 0 && candidate_count > 0 {
            format!(
                "{candidate_count} download candidate{} ready now · checking {pending} more source{} in the background…",
                if candidate_count == 1 { "" } else { "s" },
                if pending == 1 { "" } else { "s" },
            )
        } else if pending > 0 {
            format!(
                "Checking {pending} more torrent source{} for an exact match…",
                if pending == 1 { "" } else { "s" },
            )
        } else if candidate_count == 0 {
            if error_count > 0 {
                format!(
                    "No torrent files could be ranked; {error_count} source{} could not be inspected.",
                    if error_count == 1 { "" } else { "s" }
                )
            } else {
                "No matching files were found in the available torrent sources.".to_owned()
            }
        } else {
            format!(
                "{candidate_count} download candidate{} ranked across {bundle_count} torrent source{}{}",
                if candidate_count == 1 { "" } else { "s" },
                if bundle_count == 1 { "" } else { "s" },
                if error_count == 0 {
                    ".".to_owned()
                } else {
                    format!(
                        "; {error_count} source{} unavailable.",
                        if error_count == 1 { "" } else { "s" }
                    )
                }
            )
        };
        self.as_mut().set_torrent_loading(pending > 0);
        self.as_mut().bump_revision();
        self.as_mut().set_message(qstring(message));
    }

    pub fn inspect_download(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().download_busy() || *self.as_ref().download_preflight_busy() {
            return;
        }
        let Some(file) = self.as_ref().file(index).cloned() else {
            self.as_mut().set_download_preflight_ready(false);
            self.as_mut().set_download_preflight_status(qstring(
                "The selected torrent file is no longer available. Inspect the source again.",
            ));
            return;
        };
        let selected_bundle = *self.as_ref().selected_bundle();
        let Some(bundle) = self.as_ref().bundle(selected_bundle).cloned() else {
            self.as_mut().set_download_preflight_ready(false);
            self.as_mut().set_download_preflight_status(qstring(
                "The selected torrent source is no longer available.",
            ));
            return;
        };
        let game_id = self.as_ref().game_id().to_string();
        let launchbox_db_id = self.as_ref().rust().database_id;
        let title = self.as_ref().rust().canonical_title.clone();
        let platform = self.as_ref().platform().to_string();
        self.as_mut().rust_mut().download_preflight_generation = self
            .as_ref()
            .rust()
            .download_preflight_generation
            .wrapping_add(1);
        let generation = self.as_ref().rust().download_preflight_generation;
        self.as_mut().set_download_review_index(index);
        self.as_mut().set_download_preflight_busy(true);
        self.as_mut().set_download_preflight_ready(false);
        self.as_mut()
            .set_download_preflight_status(qstring("Checking storage and existing downloads…"));
        self.as_mut()
            .set_download_preflight_storage(QString::default());
        self.as_mut()
            .set_download_preflight_destination(QString::default());
        self.as_mut()
            .set_download_preflight_mode(QString::default());
        self.as_mut()
            .set_download_preflight_action(QString::default());

        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-download-preflight".into())
            .spawn(move || {
                let inspected = (|| {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = effective_download_settings(store.load()?, &bundle);
                    let torrent_bytes = game_details::torrent_bytes(&bundle)?;
                    let request = torrent_enqueue_request(
                        game_id.clone(),
                        launchbox_db_id,
                        title,
                        platform,
                        bundle,
                        file,
                        torrent_bytes,
                    )?;
                    crate::qbittorrent::inspect_enqueue(&settings, &store, &request)
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_download_preflight(generation, game_id, index, inspected);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_download_preflight_busy(false);
            self.as_mut().set_download_preflight_status(qstring(format!(
                "Could not start the download preflight worker: {error}"
            )));
        }
    }

    fn finish_download_preflight(
        mut self: Pin<&mut Self>,
        generation: u64,
        game_id: String,
        index: i32,
        inspected: Result<crate::qbittorrent::DownloadPreflight, String>,
    ) {
        if generation != self.as_ref().rust().download_preflight_generation
            || self.as_ref().game_id().to_string() != game_id
            || *self.as_ref().download_review_index() != index
        {
            return;
        }
        self.as_mut().set_download_preflight_busy(false);
        self.as_mut()
            .set_download_preflight_action(QString::default());
        match inspected {
            Ok(preflight) => {
                self.as_mut()
                    .set_download_preflight_ready(preflight.can_queue);
                self.as_mut()
                    .set_download_preflight_mode(qstring(preflight_mode_label(&preflight)));
                self.as_mut()
                    .set_download_preflight_storage(qstring(preflight_storage_summary(&preflight)));
                let destination = if preflight.uses_install_destination {
                    format!(
                        "{} · {}",
                        preflight.file_link_mode.replace('_', " "),
                        preflight.install_path.display()
                    )
                } else {
                    format!(
                        "leave in torrent library · {}",
                        preflight.download_path.display()
                    )
                };
                self.as_mut()
                    .set_download_preflight_destination(qstring(destination));
                self.as_mut()
                    .set_download_preflight_status(qstring(if preflight.can_queue {
                        "Ready to add this reviewed selection to Downloads.".to_owned()
                    } else {
                        preflight.blocked_reason
                    }));
            }
            Err(error) => {
                self.as_mut().set_download_preflight_ready(false);
                let needs_setup = error.contains("native torrent library directory")
                    || error.contains("qBittorrent torrent-library path")
                    || error.contains("native ROM directory");
                if needs_setup {
                    self.as_mut()
                        .set_download_preflight_action(qstring("configure_qbittorrent"));
                    self.as_mut().set_download_preflight_status(qstring(
                        "One-time setup: choose the download folder shared by Lunchbox and qBittorrent. Your game and exact file choice are already ready.",
                    ));
                } else {
                    self.as_mut().set_download_preflight_status(qstring(format!(
                        "Lunchbox could not check this download yet: {error}"
                    )));
                }
            }
        }
    }

    pub fn queue_file(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().download_busy() {
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
                "The selected torrent source is no longer available.",
            ));
            return;
        };
        let game_id = self.as_ref().game_id().to_string();
        let completed_game_id = game_id.clone();
        let launchbox_db_id = self.as_ref().rust().database_id;
        let title = self.as_ref().rust().canonical_title.clone();
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
            .name("lunchbox-torrent-enqueue".into())
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
        self.as_mut().rust_mut().game_emulator_preference = None;
        self.as_mut().rust_mut().platform_emulator_preference = None;
        self.as_mut().rust_mut().prepared_emulator = None;
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
                        let store = crate::settings::SettingsStore::open_default()?;
                        let game_preference = store.game_emulator_preference(&game_id)?;
                        let platform_preference = store.platform_emulator_preference(&platform)?;
                        let preference = game_preference.as_ref().or(platform_preference.as_ref());
                        let availability = crate::emulator::inspect_launch_availability(
                            &prepared,
                            &catalog_database,
                            preference,
                        )?;
                        return Ok(EmulatorDiscoveryResult::Prepared {
                            availability,
                            game_preference,
                            platform_preference,
                        });
                    }
                    let store = crate::settings::SettingsStore::open_default()?;
                    let game_preference = store.game_emulator_preference(&game_id)?;
                    let platform_preference = store.platform_emulator_preference(&platform)?;
                    let preference = game_preference.as_ref().or(platform_preference.as_ref());
                    let rom_path = selected_local_file
                        .as_deref()
                        .context("selected local game file disappeared")?;
                    let availability = crate::emulator::inspect_rom_launch_availability(
                        &platform,
                        rom_path,
                        &catalog_database,
                        preference,
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
                        game_preference,
                        platform_preference,
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
            Ok(EmulatorDiscoveryResult::Prepared {
                availability,
                game_preference,
                platform_preference,
            }) => {
                self.as_mut().rust_mut().game_emulator_preference = game_preference;
                self.as_mut().rust_mut().platform_emulator_preference = platform_preference;
                self.as_mut()
                    .set_launch_status(qstring(&availability.detail));
                if let Some(emulator) = availability.emulator {
                    self.as_mut().rust_mut().prepared_emulator = Some(emulator.clone());
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
                game_preference,
                platform_preference,
            }) => {
                let selected_index = availability.selected_index;
                let selected =
                    selected_index.and_then(|index| availability.options.get(index).cloned());
                let option_count = availability.options.len();
                self.as_mut().rust_mut().rom_emulator_options = availability.options;
                self.as_mut().rust_mut().rom_firmware_statuses = firmware_statuses;
                self.as_mut().rust_mut().game_emulator_preference = game_preference;
                self.as_mut().rust_mut().platform_emulator_preference = platform_preference;
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
        self.as_mut()
            .set_selected_local_directory_url(local_directory_url(&path));
        self.as_mut().rust_mut().local_file_path = path.clone();
        let preferred = self.as_ref().rust().preferred_local_file_path.clone();
        let preference_stale = *self.as_ref().local_file_preference_stale();
        let local_file_count = self.as_ref().rust().local_file_paths.len();
        let preference_message = local_file_preference_message(
            preferred.as_deref(),
            preference_stale,
            Some(path.as_path()),
            local_file_count,
        );
        self.as_mut()
            .set_selected_local_file_is_preferred(preferred.as_ref() == Some(&path));
        self.as_mut()
            .set_local_file_preference_message(qstring(preference_message));
        let local_file_revision = self.as_ref().local_file_revision().wrapping_add(1);
        self.as_mut().set_local_file_revision(local_file_revision);
        self.as_mut().invalidate_launch_state();
        self.as_mut().refresh_emulators();
    }

    pub fn set_selected_local_file_preferred(mut self: Pin<&mut Self>) {
        if *self.as_ref().launch_busy()
            || *self.as_ref().game_running()
            || self.as_ref().game_id().is_empty()
        {
            return;
        }
        let Some(path) = usize::try_from(*self.as_ref().selected_local_file())
            .ok()
            .and_then(|index| self.as_ref().rust().local_file_paths.get(index).cloned())
        else {
            return;
        };
        let game_id = self.as_ref().game_id().to_string();
        match SettingsStore::open_default()
            .and_then(|store| store.set_preferred_game_rom(&game_id, &path))
        {
            Ok(()) => {
                self.as_mut().rust_mut().preferred_local_file_path = Some(path.clone());
                self.as_mut().set_local_file_preference_configured(true);
                self.as_mut().set_selected_local_file_is_preferred(true);
                self.as_mut().set_local_file_preference_stale(false);
                self.as_mut().set_local_file_preference_message(qstring(
                    "This exact version opens by default.",
                ));
                let revision = self.as_ref().local_file_revision().wrapping_add(1);
                self.as_mut().set_local_file_revision(revision);
            }
            Err(error) => self
                .as_mut()
                .set_local_file_preference_message(qstring(format!(
                    "Could not save the default version: {error}"
                ))),
        }
    }

    pub fn clear_local_file_preference(mut self: Pin<&mut Self>) {
        if *self.as_ref().launch_busy()
            || *self.as_ref().game_running()
            || self.as_ref().game_id().is_empty()
        {
            return;
        }
        let game_id = self.as_ref().game_id().to_string();
        match SettingsStore::open_default()
            .and_then(|store| store.clear_preferred_game_rom(&game_id))
        {
            Ok(()) => {
                self.as_mut().rust_mut().preferred_local_file_path = None;
                self.as_mut().set_local_file_preference_configured(false);
                self.as_mut().set_selected_local_file_is_preferred(false);
                self.as_mut().set_local_file_preference_stale(false);
                let selected = usize::try_from(*self.as_ref().selected_local_file())
                    .ok()
                    .and_then(|index| self.as_ref().rust().local_file_paths.get(index).cloned());
                let local_file_count = self.as_ref().rust().local_file_paths.len();
                let message = local_file_preference_message(
                    None,
                    false,
                    selected.as_deref(),
                    local_file_count,
                );
                self.as_mut()
                    .set_local_file_preference_message(qstring(message));
                let revision = self.as_ref().local_file_revision().wrapping_add(1);
                self.as_mut().set_local_file_revision(revision);
            }
            Err(error) => self
                .as_mut()
                .set_local_file_preference_message(qstring(format!(
                    "Could not clear the default version: {error}"
                ))),
        }
    }

    pub fn uninstall_managed_installation(mut self: Pin<&mut Self>, delete_owned_files: bool) {
        if *self.as_ref().install_management_busy()
            || !*self.as_ref().managed_install_present()
            || self.as_ref().game_id().is_empty()
        {
            return;
        }
        let game_id = self.as_ref().game_id().to_string();
        let generation = self.as_ref().rust().details_generation;
        let delete_owned_files = delete_owned_files && *self.as_ref().managed_install_can_delete();
        self.as_mut().set_install_management_busy(true);
        self.as_mut()
            .set_install_management_message(qstring(if delete_owned_files {
                "Verifying every Lunchbox-owned file before removal…"
            } else {
                "Removing the library association while preserving files…"
            }));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-install-removal".into())
            .spawn(move || {
                let result = (|| {
                    let store = SettingsStore::open_default()?;
                    crate::ingest::uninstall_managed_installation(
                        &store,
                        &game_id,
                        delete_owned_files,
                    )
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_install_removal(generation, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_install_management_busy(false);
            self.as_mut()
                .set_install_management_message(qstring(format!(
                    "Could not start install-removal worker: {error}"
                )));
        }
    }

    fn finish_install_removal(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<crate::ingest::ManagedInstallationResult, String>,
    ) {
        if generation != self.as_ref().rust().details_generation {
            return;
        }
        self.as_mut().set_install_management_busy(false);
        match result {
            Ok(result) => {
                if result.prepared_cache_removed {
                    self.as_mut().rust_mut().prepared_install = None;
                    self.as_mut().set_prepared(false);
                    self.as_mut().set_prepared_summary(QString::default());
                }
                let remaining = result.remaining_local_paths;
                let remaining_count = remaining.len();
                let preferred = self.as_ref().rust().preferred_local_file_path.clone();
                let selected_index = preferred
                    .as_ref()
                    .and_then(|preferred| remaining.iter().position(|path| path == preferred))
                    .or((remaining_count > 0).then_some(0));
                let preference_stale = preferred
                    .as_ref()
                    .is_some_and(|preferred| !remaining.contains(preferred));
                let selected_path = selected_index
                    .and_then(|index| remaining.get(index))
                    .cloned();
                let selected_directory = selected_path
                    .as_ref()
                    .map(|path| local_directory_url(path))
                    .unwrap_or_default();
                self.as_mut().rust_mut().local_file_path =
                    selected_path.clone().unwrap_or_default();
                self.as_mut().rust_mut().local_file_paths = remaining;
                self.as_mut()
                    .set_local_file_count(count_i32(remaining_count));
                self.as_mut().set_selected_local_file(
                    selected_index
                        .and_then(|index| i32::try_from(index).ok())
                        .unwrap_or(-1),
                );
                self.as_mut()
                    .set_selected_local_directory_url(selected_directory);
                self.as_mut()
                    .set_local_file_preference_stale(preference_stale);
                self.as_mut().set_selected_local_file_is_preferred(
                    selected_path
                        .as_ref()
                        .is_some_and(|path| preferred.as_ref() == Some(path)),
                );
                self.as_mut().set_local_file_preference_message(qstring(
                    local_file_preference_message(
                        preferred.as_deref(),
                        preference_stale,
                        selected_path.as_deref(),
                        remaining_count,
                    ),
                ));
                let local_file_revision = self.as_ref().local_file_revision().wrapping_add(1);
                self.as_mut().set_local_file_revision(local_file_revision);
                self.as_mut().set_local(remaining_count > 0);
                self.as_mut().set_managed_install_present(false);
                self.as_mut().set_managed_install_can_delete(false);
                self.as_mut().set_managed_install_owned_count(0);
                self.as_mut().set_managed_install_shared_count(0);
                self.as_mut().set_can_launch(false);
                let message = if result.removed_file_count == 0 {
                    format!(
                        "Removed from the library. {} file{} retained.",
                        result.retained_file_count,
                        if result.retained_file_count == 1 {
                            ""
                        } else {
                            "s"
                        }
                    )
                } else if result.retained_file_count == 0 {
                    format!(
                        "Uninstalled {} verified file{}.",
                        result.removed_file_count,
                        if result.removed_file_count == 1 {
                            ""
                        } else {
                            "s"
                        }
                    )
                } else {
                    format!(
                        "Uninstalled {} verified file{}; retained {} shared or user-owned file{}.",
                        result.removed_file_count,
                        if result.removed_file_count == 1 {
                            ""
                        } else {
                            "s"
                        },
                        result.retained_file_count,
                        if result.retained_file_count == 1 {
                            ""
                        } else {
                            "s"
                        }
                    )
                };
                self.as_mut()
                    .set_install_management_message(qstring(message));
                self.as_mut()
                    .set_message(qstring("Library installation removed."));
                let revision = self.as_ref().installation_revision().wrapping_add(1);
                self.as_mut().set_installation_revision(revision);
                self.as_mut().bump_revision();
                if remaining_count > 0 {
                    self.as_mut().refresh_emulators();
                }
            }
            Err(error) => {
                self.as_mut()
                    .set_install_management_message(qstring(format!(
                        "Could not remove this installation: {error}"
                    )));
            }
        }
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
        self.as_mut().set_launch_profile_open(false);
        self.as_mut().set_launch_profile_status(QString::default());
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

    pub fn manager_emulator_target_available(
        &self,
        emulator_id: QString,
        manager: QString,
        package_id: QString,
    ) -> bool {
        self.manager_emulator_option_index(
            &emulator_id.to_string(),
            &manager.to_string(),
            &package_id.to_string(),
        )
        .is_some()
    }

    pub fn manager_emulator_target_is_default(
        &self,
        emulator_id: QString,
        manager: QString,
        package_id: QString,
        scope: QString,
    ) -> bool {
        let Some(option) = self
            .manager_emulator_option_index(
                &emulator_id.to_string(),
                &manager.to_string(),
                &package_id.to_string(),
            )
            .and_then(|index| self.rust().rom_emulator_options.get(index))
        else {
            return false;
        };
        let preference = match scope.to_string().as_str() {
            "game" => self.rust().game_emulator_preference.as_ref(),
            "platform" => self.rust().platform_emulator_preference.as_ref(),
            _ => return false,
        };
        preference.is_some_and(|preference| emulator_preference_matches_option(preference, option))
    }

    pub fn save_manager_emulator_preference(
        mut self: Pin<&mut Self>,
        emulator_id: QString,
        manager: QString,
        package_id: QString,
        scope: QString,
    ) {
        let scope = scope.to_string();
        let Some(index) = self.as_ref().manager_emulator_option_index(
            &emulator_id.to_string(),
            &manager.to_string(),
            &package_id.to_string(),
        ) else {
            self.as_mut().set_launch_status(qstring(
                "Install or refresh this emulator before making it a default.",
            ));
            return;
        };
        let Ok(index) = i32::try_from(index) else {
            self.as_mut()
                .set_launch_status(qstring("The selected emulator index is out of range."));
            return;
        };
        self.as_mut().select_emulator_option(index);
        match scope.as_str() {
            "game" => self.as_mut().save_game_emulator_preference(),
            "platform" => self.as_mut().save_platform_emulator_preference(),
            _ => self
                .as_mut()
                .set_launch_status(qstring("Choose a game or platform default scope.")),
        }
    }

    fn manager_emulator_option_index(
        &self,
        emulator_id: &str,
        manager: &str,
        package_id: &str,
    ) -> Option<usize> {
        let emulator_id = emulator_id.trim();
        let package_id = package_id.trim();
        self.rust().rom_emulator_options.iter().position(|option| {
            if manager.trim() == "libretro" {
                option.runtime_kind == crate::emulator::EmulatorRuntimeKind::RetroArch
                    && option.core_name == package_id
            } else {
                option.runtime_kind == crate::emulator::EmulatorRuntimeKind::Standalone
                    && option.emulator_id == emulator_id
            }
        })
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
                self.as_mut().rust_mut().game_emulator_preference =
                    Some(emulator_preference_for_option(&option, "game"));
                self.as_mut().set_emulator_preference_scope(qstring("game"));
                self.as_mut().set_launch_status(qstring(format!(
                    "{} is now the default for this game.",
                    option.label()
                )));
                self.as_mut().bump_revision();
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
                self.as_mut().rust_mut().platform_emulator_preference =
                    Some(emulator_preference_for_option(&option, "platform"));
                self.as_mut()
                    .set_emulator_preference_scope(qstring("platform"));
                self.as_mut().set_launch_status(qstring(format!(
                    "{} is now the default for {platform}.",
                    option.label()
                )));
                self.as_mut().bump_revision();
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
            Ok(()) => {
                self.as_mut().bump_revision();
                self.as_mut().refresh_emulators();
            }
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
            Ok(()) => {
                self.as_mut().bump_revision();
                self.as_mut().refresh_emulators();
            }
            Err(error) => self.as_mut().set_launch_status(qstring(format!(
                "Could not clear the platform emulator default: {error}"
            ))),
        }
    }

    pub fn open_launch_profile_editor(mut self: Pin<&mut Self>) {
        if let Err(error) = self.launch_profile_target() {
            self.as_mut()
                .set_launch_profile_status(qstring(error.to_string()));
            return;
        }
        self.as_mut().set_launch_profile_open(true);
        self.as_mut().load_launch_profile_scope("game");
    }

    pub fn close_launch_profile_editor(mut self: Pin<&mut Self>) {
        self.as_mut().set_launch_profile_open(false);
        self.as_mut().set_launch_profile_status(QString::default());
        self.as_mut().clear_launch_profile_preview();
    }

    pub fn select_launch_profile_scope(mut self: Pin<&mut Self>, scope: QString) {
        self.as_mut().load_launch_profile_scope(&scope.to_string());
    }

    pub fn save_launch_profile(
        mut self: Pin<&mut Self>,
        extra_arguments: QString,
        command_template: QString,
    ) {
        let target = match self.launch_profile_target() {
            Ok(target) => target,
            Err(error) => {
                self.as_mut()
                    .set_launch_profile_status(qstring(error.to_string()));
                return;
            }
        };
        if let Err(error) = validate_launch_profile_template(&target, &command_template.to_string())
        {
            self.as_mut().set_launch_profile_status(qstring(format!(
                "Could not save the launch profile: {error}"
            )));
            return;
        }
        let scope = self.as_ref().launch_profile_scope().to_string();
        let scope_key = self.launch_profile_scope_key(&scope);
        let result = scope_key.and_then(|scope_key| {
            crate::settings::SettingsStore::open_default()?.set_emulator_launch_profile(
                &crate::settings::EmulatorLaunchProfile {
                    scope_kind: scope.clone(),
                    scope_key,
                    emulator_id: target.emulator_id.clone(),
                    runtime_kind: target.runtime_kind.to_owned(),
                    core_name: target.core_name.clone(),
                    extra_arguments: extra_arguments.to_string(),
                    command_template: command_template.to_string(),
                    updated_at: 0,
                },
            )
        });
        match result {
            Ok(()) => {
                self.as_mut().load_launch_profile_scope(&scope);
                self.as_mut().set_launch_profile_status(qstring(format!(
                    "Saved the exact {} launch profile for {}.",
                    launch_scope_label(&scope),
                    target.emulator_label
                )));
                println!(
                    "LUNCHBOX_LAUNCH_PROFILE_SAVED scope={} runtime={} template={} extra={}",
                    scope,
                    target.runtime_kind,
                    !command_template.to_string().trim().is_empty(),
                    !extra_arguments.to_string().trim().is_empty()
                );
            }
            Err(error) => self.as_mut().set_launch_profile_status(qstring(format!(
                "Could not save the launch profile: {error}"
            ))),
        }
    }

    pub fn clear_launch_profile(mut self: Pin<&mut Self>) {
        let target = match self.launch_profile_target() {
            Ok(target) => target,
            Err(error) => {
                self.as_mut()
                    .set_launch_profile_status(qstring(error.to_string()));
                return;
            }
        };
        let scope = self.as_ref().launch_profile_scope().to_string();
        let scope_key = self.launch_profile_scope_key(&scope);
        let result = scope_key.and_then(|scope_key| {
            crate::settings::SettingsStore::open_default()?.clear_emulator_launch_profile(
                &scope,
                &scope_key,
                &target.emulator_id,
                target.runtime_kind,
                &target.core_name,
            )
        });
        match result {
            Ok(()) => {
                self.as_mut().load_launch_profile_scope(&scope);
                self.as_mut().set_launch_profile_status(qstring(format!(
                    "Cleared the {} launch profile; inherited behavior is active.",
                    launch_scope_label(&scope)
                )));
            }
            Err(error) => self.as_mut().set_launch_profile_status(qstring(format!(
                "Could not clear the launch profile: {error}"
            ))),
        }
    }

    pub fn update_launch_profile_preview(
        mut self: Pin<&mut Self>,
        extra_arguments: QString,
        command_template: QString,
    ) {
        let fallback_extra_arguments = self
            .as_ref()
            .rust()
            .launch_profile_preview_fallback_extra_arguments
            .clone();
        let fallback_command_template = self
            .as_ref()
            .rust()
            .launch_profile_preview_fallback_command_template
            .clone();
        let (effective_extra_arguments, effective_command_template) =
            crate::emulator::effective_launch_preview_values(
                &extra_arguments.to_string(),
                &command_template.to_string(),
                &fallback_extra_arguments,
                &fallback_command_template,
            );
        let result = self.launch_profile_target().and_then(|target| {
            preview_for_launch_profile_target(
                &target,
                &effective_extra_arguments,
                &effective_command_template,
            )
        });
        self.as_mut().publish_launch_profile_preview(result);
    }

    pub fn launch_profile_preview_argument_at(&self, index: i32) -> QString {
        qstring(
            usize::try_from(index)
                .ok()
                .and_then(|index| self.rust().launch_profile_preview_arguments.get(index))
                .map(String::as_str)
                .unwrap_or(""),
        )
    }

    fn publish_launch_profile_preview(
        mut self: Pin<&mut Self>,
        result: anyhow::Result<crate::emulator::LaunchCommandPreview>,
    ) {
        match result {
            Ok(preview) => {
                let argument_count = preview.arguments.len();
                let message = preview.summary();
                self.as_mut().set_launch_profile_preview_valid(true);
                self.as_mut()
                    .set_launch_profile_preview_runtime(qstring(preview.runtime));
                self.as_mut()
                    .set_launch_profile_preview_message(qstring(message));
                self.as_mut().rust_mut().launch_profile_preview_arguments = preview.arguments;
                self.as_mut().set_launch_profile_preview_argument_count(
                    i32::try_from(argument_count).unwrap_or(i32::MAX),
                );
            }
            Err(error) => {
                self.as_mut().set_launch_profile_preview_valid(false);
                self.as_mut()
                    .set_launch_profile_preview_runtime(QString::default());
                self.as_mut()
                    .set_launch_profile_preview_message(qstring(format!(
                        "Check this command: {error}"
                    )));
                self.as_mut()
                    .rust_mut()
                    .launch_profile_preview_arguments
                    .clear();
                self.as_mut().set_launch_profile_preview_argument_count(0);
            }
        }
        let revision = self
            .as_ref()
            .launch_profile_preview_revision()
            .wrapping_add(1);
        self.as_mut().set_launch_profile_preview_revision(revision);
    }

    fn clear_launch_profile_preview(mut self: Pin<&mut Self>) {
        self.as_mut().set_launch_profile_preview_valid(false);
        self.as_mut()
            .set_launch_profile_preview_runtime(QString::default());
        self.as_mut()
            .set_launch_profile_preview_message(QString::default());
        self.as_mut()
            .rust_mut()
            .launch_profile_preview_arguments
            .clear();
        self.as_mut()
            .rust_mut()
            .launch_profile_preview_fallback_extra_arguments
            .clear();
        self.as_mut()
            .rust_mut()
            .launch_profile_preview_fallback_command_template
            .clear();
        self.as_mut().set_launch_profile_preview_argument_count(0);
        let revision = self
            .as_ref()
            .launch_profile_preview_revision()
            .wrapping_add(1);
        self.as_mut().set_launch_profile_preview_revision(revision);
    }

    fn load_launch_profile_scope(mut self: Pin<&mut Self>, scope: &str) {
        if !matches!(scope, "game" | "platform" | "global") {
            self.as_mut()
                .set_launch_profile_status(qstring("Choose a valid launch profile scope."));
            return;
        }
        let target = match self.launch_profile_target() {
            Ok(target) => target,
            Err(error) => {
                self.as_mut()
                    .set_launch_profile_status(qstring(error.to_string()));
                return;
            }
        };
        let Some(scope_key) = self.launch_profile_scope_key(scope).ok() else {
            return;
        };
        let game_uid = self.as_ref().game_id().to_string();
        let platform = self.as_ref().platform().to_string();
        let loaded = (|| -> anyhow::Result<_> {
            let store = crate::settings::SettingsStore::open_default()?;
            let exact = store.emulator_launch_profile(
                scope,
                &scope_key,
                &target.emulator_id,
                target.runtime_kind,
                &target.core_name,
            )?;
            let resolved = store.resolve_launch_customization(
                &game_uid,
                &platform,
                &target.emulator_id,
                target.runtime_kind,
                &target.core_name,
            )?;
            let fallback = launch_profile_preview_fallback(&store, scope, &platform, &target)?;
            Ok((exact, resolved, fallback))
        })();
        match loaded {
            Ok((exact, resolved, fallback)) => {
                let default_template = target.default_template.clone();
                let effective_template = if resolved.command_template.is_empty() {
                    default_template.clone()
                } else {
                    resolved.command_template.clone()
                };
                let exact = exact.unwrap_or_default();
                let argument_source = if resolved.argument_scope.is_empty() {
                    "built-in"
                } else {
                    launch_scope_label(&resolved.argument_scope)
                };
                let template_source = if resolved.template_scope.is_empty() {
                    "built-in"
                } else {
                    launch_scope_label(&resolved.template_scope)
                };
                let (preview_extra_arguments, preview_command_template) =
                    crate::emulator::effective_launch_preview_values(
                        &exact.extra_arguments,
                        &exact.command_template,
                        &fallback.extra_arguments,
                        &fallback.command_template,
                    );
                self.as_mut()
                    .rust_mut()
                    .launch_profile_preview_fallback_extra_arguments = fallback.extra_arguments;
                self.as_mut()
                    .rust_mut()
                    .launch_profile_preview_fallback_command_template = fallback.command_template;
                self.as_mut().set_launch_profile_scope(qstring(scope));
                self.as_mut()
                    .set_launch_profile_default_template(qstring(default_template));
                self.as_mut()
                    .set_launch_profile_effective_template(qstring(effective_template));
                self.as_mut()
                    .set_launch_profile_extra_arguments(qstring(exact.extra_arguments));
                self.as_mut()
                    .set_launch_profile_command_template(qstring(exact.command_template));
                self.as_mut().set_launch_profile_inheritance(qstring(format!(
                    "Effective extra arguments: {argument_source} · command template: {template_source}"
                )));
                self.as_mut().set_launch_profile_status(QString::default());
                let preview = preview_for_launch_profile_target(
                    &target,
                    &preview_extra_arguments,
                    &preview_command_template,
                );
                self.as_mut().publish_launch_profile_preview(preview);
                let revision = self.as_ref().launch_profile_revision().wrapping_add(1);
                self.as_mut().set_launch_profile_revision(revision);
            }
            Err(error) => self.as_mut().set_launch_profile_status(qstring(format!(
                "Could not load the launch profile: {error}"
            ))),
        }
    }

    fn launch_profile_scope_key(&self, scope: &str) -> anyhow::Result<String> {
        match scope {
            "game" => {
                let game_uid = self.game_id().to_string();
                anyhow::ensure!(
                    !game_uid.trim().is_empty(),
                    "this game does not have a stable identity"
                );
                Ok(game_uid)
            }
            "platform" => {
                let platform = self.platform().to_string();
                anyhow::ensure!(!platform.trim().is_empty(), "this game has no platform");
                Ok(platform)
            }
            "global" => Ok(String::new()),
            _ => anyhow::bail!("unsupported launch profile scope {scope}"),
        }
    }

    fn launch_profile_target(&self) -> anyhow::Result<LaunchProfileTarget> {
        if let Some(option) = self.selected_rom_emulator_option() {
            let default_template =
                crate::emulator::default_rom_launch_template(&option, &self.platform().to_string());
            let preview_extra_insert_index =
                crate::emulator::default_rom_extra_argument_insert_index(
                    &option.emulator_name,
                    option.runtime_kind,
                    &self.platform().to_string(),
                );
            let mut available_placeholders =
                crate::emulator::launch_template_placeholders(&default_template)?;
            if !available_placeholders.iter().any(|name| name == "file") {
                available_placeholders.push("file".to_owned());
            }
            return Ok(LaunchProfileTarget {
                emulator_id: option.emulator_id.clone(),
                emulator_label: option.label(),
                runtime_kind: option.runtime_kind.key(),
                core_name: option.core_name.clone(),
                default_template,
                available_placeholders,
                preview_extra_insert_index,
            });
        }
        let prepared = self
            .rust()
            .prepared_install
            .as_ref()
            .context("Prepare this game before editing its launch command.")?;
        let emulator = self.rust().prepared_emulator.as_ref().context(
            "Select or install a compatible emulator before editing its launch command.",
        )?;
        let default_template =
            crate::emulator::default_prepared_launch_template(prepared, &emulator.name)?;
        let preview_extra_insert_index =
            crate::emulator::default_prepared_extra_argument_insert_index(&default_template)?;
        let available_placeholders =
            crate::emulator::launch_template_placeholders(&default_template)?;
        Ok(LaunchProfileTarget {
            emulator_id: emulator.id.clone(),
            emulator_label: emulator.name.clone(),
            runtime_kind: crate::emulator::EmulatorRuntimeKind::Standalone.key(),
            core_name: String::new(),
            default_template,
            available_placeholders,
            preview_extra_insert_index,
        })
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
        self.as_mut().set_firmware_progress(0);
        self.as_mut()
            .set_launch_status(qstring("Importing and verifying firmware package…"));
        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress_game_id = game_id.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-firmware-import".into())
            .spawn(move || {
                let result = crate::firmware::import_and_sync_with_progress(
                    &statuses,
                    &path,
                    |message, percent| {
                        let progress_game_id = progress_game_id.clone();
                        let _ = progress_thread.queue(move |mut model| {
                            model.as_mut().update_firmware_progress(
                                generation,
                                progress_game_id,
                                message,
                                i32::from(percent),
                            );
                        });
                    },
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_firmware_action(generation, game_id, result);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_firmware_busy(false);
            self.as_mut().set_firmware_progress(-1);
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
        self.as_mut().set_firmware_progress(-1);
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
                            -1,
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
            self.as_mut().set_firmware_progress(-1);
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
        if !statuses
            .iter()
            .any(crate::firmware::FirmwareStatus::can_sync)
        {
            self.as_mut().set_launch_status(qstring(
                "No saved package needs to be applied to this emulator.",
            ));
            return;
        }
        let generation = self.as_ref().rust().details_generation;
        let game_id = self.as_ref().game_id().to_string();
        self.as_mut().set_firmware_busy(true);
        self.as_mut().set_firmware_progress(0);
        self.as_mut()
            .set_launch_status(qstring("Verifying and applying firmware to this emulator…"));
        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress_game_id = game_id.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-firmware-sync".into())
            .spawn(move || {
                let result =
                    crate::firmware::sync_imported_with_progress(&statuses, |message, percent| {
                        let progress_game_id = progress_game_id.clone();
                        let _ = progress_thread.queue(move |mut model| {
                            model.as_mut().update_firmware_progress(
                                generation,
                                progress_game_id,
                                message,
                                i32::from(percent),
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
            self.as_mut().set_firmware_progress(-1);
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
        self.as_mut().set_firmware_progress(-1);
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
        percent: i32,
    ) {
        if generation == self.as_ref().rust().details_generation
            && game_id == self.as_ref().game_id().to_string()
            && *self.as_ref().firmware_busy()
        {
            self.as_mut().set_launch_status(qstring(message));
            self.as_mut().set_firmware_progress(percent.clamp(-1, 100));
        }
    }

    fn clear_firmware_status(mut self: Pin<&mut Self>) {
        if !*self.as_ref().firmware_busy() {
            self.as_mut().set_firmware_progress(-1);
        }
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
        self.as_mut().set_firmware_next_package(QString::default());
        self.as_mut()
            .set_firmware_setup_action(QString::from("review"));
        self.as_mut()
            .set_firmware_setup_label(QString::from("SET UP EMULATOR"));
        self.as_mut().set_switch_prod_keys_imported(false);
        self.as_mut().set_switch_prod_keys_ready(false);
        self.as_mut().set_switch_firmware_imported(false);
        self.as_mut().set_switch_firmware_ready(false);
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
        let can_sync = statuses
            .iter()
            .any(crate::firmware::FirmwareStatus::can_sync);
        let package_ready = |package: &str| {
            statuses.iter().any(|status| {
                status.package_name.eq_ignore_ascii_case(package)
                    && status.imported
                    && (status.synced
                        || matches!(
                            status.target_strategy.as_str(),
                            "launch_scoped" | "manual_import"
                        ))
            })
        };
        let package_imported = |package: &str| {
            statuses
                .iter()
                .any(|status| status.package_name.eq_ignore_ascii_case(package) && status.imported)
        };
        let next_action = statuses
            .iter()
            .find(|status| status.needs_action() && status.can_sync())
            .map(|status| ("SYNC", status.package_name.as_str()))
            .or_else(|| {
                statuses
                    .iter()
                    .find(|status| {
                        status.needs_action()
                            && status.target_strategy != "manual_import"
                            && !status.imported
                            && status.source_transport != "manual"
                    })
                    .map(|status| ("DOWNLOAD", status.package_name.as_str()))
            })
            .or_else(|| {
                statuses
                    .iter()
                    .find(|status| {
                        status.needs_action()
                            && status.target_strategy != "manual_import"
                            && !status.imported
                    })
                    .map(|status| ("CHOOSE", status.package_name.as_str()))
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
        let summary = crate::firmware::summarize(&statuses);
        if missing > 0 {
            let next_step = next_action.map_or_else(
                || "Review firmware requirements".to_owned(),
                |(action, package)| {
                    let action = if action == "SYNC" { "APPLY" } else { action };
                    format!("{action} {package}")
                },
            );
            self.as_mut().set_can_launch(false);
            self.as_mut()
                .set_launch_status(qstring(format!("{summary} Next: {next_step}.")));
        }
        self.as_mut().set_firmware_needs_import(needs_import);
        self.as_mut().set_firmware_can_download(can_download);
        self.as_mut().set_firmware_can_sync(can_sync);
        self.as_mut().set_firmware_summary(qstring(summary));
        self.as_mut()
            .set_firmware_source_summary(qstring(sources.join(" · ")));
        self.as_mut()
            .set_firmware_package_summary(qstring(packages.join(" · ")));
        self.as_mut()
            .set_firmware_runtime_path(qstring(runtime_path));
        self.as_mut().set_firmware_next_package(qstring(
            next_action.map(|(_, package)| package).unwrap_or_default(),
        ));
        self.as_mut().set_firmware_setup_action(qstring(
            next_action
                .map(|(action, _)| action.to_ascii_lowercase())
                .unwrap_or_else(|| "review".to_owned()),
        ));
        self.as_mut()
            .set_firmware_setup_label(QString::from("SET UP EMULATOR"));
        self.as_mut()
            .set_switch_prod_keys_imported(package_imported("switch-keys.zip"));
        self.as_mut()
            .set_switch_prod_keys_ready(package_ready("switch-keys.zip"));
        self.as_mut()
            .set_switch_firmware_imported(package_imported("switch-firmware.zip"));
        self.as_mut()
            .set_switch_firmware_ready(package_ready("switch-firmware.zip"));
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
            let Some(emulator) = self.as_ref().rust().prepared_emulator.clone() else {
                self.as_mut().set_launch_status(qstring(
                    "Refresh emulator detection before launching this prepared game.",
                ));
                return;
            };
            LaunchInput::Prepared {
                install,
                catalog_database,
                emulator_id: emulator.id,
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
        let activity_title = self.as_ref().rust().canonical_title.clone();
        let activity_platform = self.as_ref().platform().to_string();
        let activity_database_id = self.as_ref().rust().database_id;
        let rom_probe = is_local_launch_probe();
        let preparing_archived_playlist = matches!(
            &launch_input,
            LaunchInput::Rom { path, .. }
                if path.extension().and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("m3u"))
        );
        let launch_cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().launch_cancel = Some(Arc::clone(&launch_cancel));
        self.as_mut().set_launch_busy(true);
        self.as_mut()
            .set_launch_status(qstring(if preparing_archived_playlist {
                "Preparing the multi-disc playlist and any compressed disc images…"
            } else {
                "Building the exact launch plan and preparing writable runtime files…"
            }));

        let qt_thread = self.as_ref().qt_thread();
        let started_thread = qt_thread.clone();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-emulator-launch".into())
            .spawn(move || {
                let launch = (|| -> anyhow::Result<(Result<(), String>, Option<String>, bool)> {
                    if launch_cancel.load(AtomicOrdering::Relaxed) {
                        anyhow::bail!(crate::rom_launch_preparation::LAUNCH_CANCELLED_ERROR);
                    }
                    let plan = match &launch_input {
                        LaunchInput::Prepared {
                            install,
                            catalog_database,
                            emulator_id,
                        } => {
                            let customization = crate::settings::SettingsStore::open_default()?
                                .resolve_launch_customization(
                                    &game_id,
                                    &activity_platform,
                                    emulator_id,
                                    crate::emulator::EmulatorRuntimeKind::Standalone.key(),
                                    "",
                                )?;
                            crate::emulator::build_prepared_launch_plan_with_customization(
                                install,
                                catalog_database,
                                emulator_id,
                                &customization,
                            )?
                        }
                        LaunchInput::Rom {
                            path,
                            platform,
                            option,
                        } => {
                            let customization = crate::settings::SettingsStore::open_default()?
                                .resolve_launch_customization(
                                    &game_id,
                                    platform,
                                    &option.emulator_id,
                                    option.runtime_kind.key(),
                                    &option.core_name,
                                )?;
                            crate::emulator::build_rom_launch_plan_with_customization_and_cancellation(
                                path,
                                platform,
                                option,
                                &customization,
                                &launch_cancel,
                            )?
                        }
                    };
                    let emulator_name = plan.emulator_name.clone();
                    let command_summary = plan.command_summary();
                    let cleanup_paths = plan.cleanup_paths.clone();
                    let _cleanup_guard = LaunchCleanupGuard(cleanup_paths);
                    if launch_cancel.load(AtomicOrdering::Relaxed) {
                        anyhow::bail!(crate::rom_launch_preparation::LAUNCH_CANCELLED_ERROR);
                    }
                    let controller_settings = crate::settings::SettingsStore::open_default()
                        .and_then(|store| store.load())
                        .context("loading controller mapping settings")?;
                    let controller_activation = crate::controllers::activate_for_launch(
                        &controller_settings,
                        Some(&activity_platform),
                        Some(activity_database_id),
                    )
                    .map_err(anyhow::Error::msg)
                    .context("preparing controller mapping")?;
                    let crate::controllers::ControllerActivation {
                        session: controller_session,
                        warning: controller_warning,
                    } = controller_activation;
                    if launch_cancel.load(AtomicOrdering::Relaxed) {
                        anyhow::bail!(crate::rom_launch_preparation::LAUNCH_CANCELLED_ERROR);
                    }
                    let mut child = crate::emulator::spawn_launch_plan(&plan)?;
                    let process_id = child.id();
                    let startup_deadline = Instant::now() + Duration::from_millis(700);
                    loop {
                        if launch_cancel.load(AtomicOrdering::Relaxed) {
                            let _ = child.kill();
                            let _ = child.wait();
                            anyhow::bail!(crate::rom_launch_preparation::LAUNCH_CANCELLED_ERROR);
                        }
                        if let Some(status) = child
                            .try_wait()
                            .context("checking whether the emulator survived startup")?
                        {
                            anyhow::bail!(
                                "{} exited during startup with {status}; no game window was opened",
                                plan.emulator_name
                            );
                        }
                        if Instant::now() >= startup_deadline {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(35));
                    }
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
                    let activity_warning = play_session
                        .as_ref()
                        .err()
                        .map(|error| format!("Play activity could not be recorded: {error}"));
                    let tracking_warning = [controller_warning, activity_warning]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    let tracking_warning =
                        (!tracking_warning.is_empty()).then(|| tracking_warning.join(" · "));
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
                    drop(controller_session);
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
            self.as_mut().rust_mut().launch_cancel = None;
            self.as_mut().set_launch_busy(false);
            self.as_mut().set_launch_status(qstring(format!(
                "Could not start emulator launch worker: {error}"
            )));
        }
    }

    pub fn cancel_launch(mut self: Pin<&mut Self>) {
        if !*self.as_ref().launch_busy() {
            return;
        }
        if let Some(cancel) = self.as_ref().rust().launch_cancel.as_ref() {
            cancel.store(true, AtomicOrdering::Relaxed);
            self.as_mut().set_launch_status(qstring(
                "Cancelling launch preparation and removing temporary files…",
            ));
        }
    }

    fn finish_launch_started(mut self: Pin<&mut Self>, generation: u64, started: LaunchStarted) {
        if generation != self.as_ref().rust().launch_generation
            || self.as_ref().game_id().to_string() != started.game_id
        {
            return;
        }
        self.as_mut().rust_mut().launch_cancel = None;
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
        self.as_mut().rust_mut().launch_cancel = None;
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
        self.as_mut().rust_mut().launch_cancel = None;
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_game_running(false);
        if error.contains(crate::rom_launch_preparation::LAUNCH_CANCELLED_ERROR) {
            self.as_mut().set_launch_status(qstring(
                "Launch cancelled. Temporary preparation files were removed.",
            ));
        } else {
            self.as_mut()
                .set_launch_status(qstring(format!("Could not launch this game: {error}")));
        }
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
        if let Some(cancel) = self.as_ref().rust().launch_cancel.as_ref() {
            cancel.store(true, AtomicOrdering::Relaxed);
        }
        self.as_mut().rust_mut().launch_cancel = None;
        self.as_mut().rust_mut().launch_generation =
            self.as_ref().rust().launch_generation.wrapping_add(1);
        self.as_mut().set_launch_discovery_busy(false);
        self.as_mut().set_launch_busy(false);
        self.as_mut().set_can_launch(false);
        self.as_mut().set_game_running(false);
        self.as_mut().set_launch_profile_open(false);
        self.as_mut()
            .set_launch_profile_default_template(QString::default());
        self.as_mut()
            .set_launch_profile_effective_template(QString::default());
        self.as_mut()
            .set_launch_profile_extra_arguments(QString::default());
        self.as_mut()
            .set_launch_profile_command_template(QString::default());
        self.as_mut()
            .set_launch_profile_inheritance(QString::default());
        self.as_mut().set_launch_profile_status(QString::default());
        self.as_mut().clear_launch_profile_preview();
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

    fn bump_metadata_custom_field_revision(mut self: Pin<&mut Self>) {
        let revision = self
            .as_ref()
            .metadata_custom_field_revision()
            .wrapping_add(1);
        self.as_mut().set_metadata_custom_field_revision(revision);
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
                let source = if bundle.source_kind == "manual_torrent" {
                    "registered local source"
                } else {
                    bundle.match_kind.label()
                };
                qstring(format!(
                    "{} files · {} · {}",
                    bundle.rom_count,
                    game_details::format_bytes(bundle.total_size),
                    source
                ))
            })
            .unwrap_or_default()
    }

    pub fn bundle_file_count_at(&self, bundle_index: i32) -> i32 {
        self.bundle_group(bundle_index)
            .map(|group| count_i32(group.files.len()))
            .unwrap_or_default()
    }

    pub fn bundle_file_size_at(&self, bundle_index: i32, file_index: i32) -> QString {
        self.bundle_file(bundle_index, file_index)
            .map(|file| qstring(game_details::format_bytes(file.byte_size)))
            .unwrap_or_default()
    }

    pub fn bundle_file_name_at(&self, bundle_index: i32, file_index: i32) -> QString {
        self.bundle_file(bundle_index, file_index)
            .map(file_name_label)
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn bundle_file_detail_at(&self, bundle_index: i32, file_index: i32) -> QString {
        self.bundle_file(bundle_index, file_index)
            .map(|file| {
                let first_source = download_source_location(&self.rust().bundle_candidates, 0)
                    .and_then(|index| i32::try_from(index).ok());
                let badge = if first_source == Some(bundle_index) && file_index == 0 {
                    Some("BEST MATCH")
                } else if file_index == 0 {
                    Some("BEST IN SOURCE")
                } else {
                    None
                };
                qstring(file_detail_label(file, badge, &self.rust().canonical_title))
            })
            .unwrap_or_default()
    }

    pub fn bundle_load_message_at(&self, bundle_index: i32) -> QString {
        self.bundle_group(bundle_index)
            .map(|group| {
                let source = self
                    .bundle(bundle_index)
                    .map(|bundle| bundle.collection.as_str())
                    .filter(|source| !source.trim().is_empty())
                    .unwrap_or("this source");
                qstring(bundle_group_message(group, source))
            })
            .unwrap_or_else(|| qstring("This torrent source is no longer available."))
    }

    pub fn download_source_count(&self) -> i32 {
        count_i32(
            self.rust()
                .bundle_candidates
                .iter()
                .filter(|group| !group.files.is_empty())
                .count(),
        )
    }

    pub fn download_source_bundle_at(&self, index: i32) -> i32 {
        download_source_location(&self.rust().bundle_candidates, index)
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    pub fn download_candidate_count(&self) -> i32 {
        count_i32(
            self.rust()
                .bundle_candidates
                .iter()
                .map(|group| group.files.len())
                .sum(),
        )
    }

    pub fn download_candidate_source_at(&self, index: i32) -> QString {
        download_candidate_location(&self.rust().bundle_candidates, index)
            .and_then(|(bundle_index, _)| {
                i32::try_from(bundle_index)
                    .ok()
                    .map(|bundle_index| self.bundle_title_at(bundle_index))
            })
            .unwrap_or_default()
    }

    pub fn download_candidate_name_at(&self, index: i32) -> QString {
        download_candidate_location(&self.rust().bundle_candidates, index)
            .and_then(|(bundle_index, file_index)| {
                self.rust()
                    .bundle_candidates
                    .get(bundle_index)
                    .and_then(|group| group.files.get(file_index))
            })
            .map(file_name_label)
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn download_candidate_detail_at(&self, index: i32) -> QString {
        download_candidate_location(&self.rust().bundle_candidates, index)
            .and_then(|(bundle_index, file_index)| {
                self.rust()
                    .bundle_candidates
                    .get(bundle_index)
                    .and_then(|group| group.files.get(file_index))
                    .map(|file| {
                        let badge = if index == 0 {
                            Some("BEST MATCH")
                        } else if file_index == 0 {
                            Some("BEST IN SOURCE")
                        } else {
                            None
                        };
                        file_detail_label(file, badge, &self.rust().canonical_title)
                    })
            })
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn selected_source_is_registered(&self) -> bool {
        self.bundle(*self.selected_bundle())
            .is_some_and(|bundle| bundle.source_kind == "manual_torrent")
    }

    pub fn select_download_candidate(mut self: Pin<&mut Self>, index: i32) -> i32 {
        let Some((bundle_index, file_index)) =
            download_candidate_location(&self.as_ref().rust().bundle_candidates, index)
        else {
            self.as_mut().set_message(qstring(
                "That download candidate is no longer available. Wait for the list to refresh and try again.",
            ));
            return -1;
        };
        let (Ok(bundle_index), Ok(file_index)) =
            (i32::try_from(bundle_index), i32::try_from(file_index))
        else {
            return -1;
        };
        self.as_mut().select_bundle_file(bundle_index, file_index)
    }

    pub fn select_bundle_file(mut self: Pin<&mut Self>, bundle_index: i32, file_index: i32) -> i32 {
        let Some(bundle_index_usize) = usize::try_from(bundle_index).ok() else {
            return -1;
        };
        let Some(file_index_usize) = usize::try_from(file_index).ok() else {
            return -1;
        };
        let Some(files) = self
            .as_ref()
            .rust()
            .bundle_candidates
            .get(bundle_index_usize)
            .filter(|group| group.loaded && file_index_usize < group.files.len())
            .map(|group| group.files.clone())
        else {
            self.as_mut().set_message(qstring(
                "That download candidate is no longer available. Refresh the game details and try again.",
            ));
            return -1;
        };
        let file_count = files.len();
        self.as_mut().rust_mut().files = files;
        self.as_mut().set_file_count(count_i32(file_count));
        self.as_mut().set_selected_bundle(bundle_index);
        self.as_mut().bump_revision();
        file_index
    }

    pub fn alternate_title_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().alternate_titles.get(index))
            .map(|title| qstring(&title.name))
            .unwrap_or_default()
    }

    pub fn alternate_title_region_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().alternate_titles.get(index))
            .map(|title| qstring(&title.region))
            .unwrap_or_default()
    }

    pub fn variant_title_at(&self, index: i32) -> QString {
        self.variant(index)
            .map(|variant| qstring(&variant.title))
            .unwrap_or_default()
    }

    pub fn variant_label_at(&self, index: i32) -> QString {
        self.variant(index)
            .map(|variant| qstring(&variant.release_label))
            .unwrap_or_default()
    }

    pub fn variant_game_id_at(&self, index: i32) -> QString {
        self.variant(index)
            .map(|variant| qstring(&variant.id))
            .unwrap_or_default()
    }

    pub fn variant_database_id_at(&self, index: i32) -> i32 {
        self.variant(index)
            .map(|variant| i32::try_from(variant.launchbox_db_id).unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn variant_status_at(&self, index: i32) -> QString {
        self.variant(index)
            .map(|variant| {
                qstring(if variant.current {
                    "CURRENT"
                } else if variant.local {
                    "INSTALLED"
                } else if variant.downloadable {
                    "MINERVA"
                } else {
                    "CATALOG"
                })
            })
            .unwrap_or_default()
    }

    pub fn variant_is_current_at(&self, index: i32) -> bool {
        self.variant(index).is_some_and(|variant| variant.current)
    }

    pub fn variant_is_local_at(&self, index: i32) -> bool {
        self.variant(index).is_some_and(|variant| variant.local)
    }

    pub fn variant_is_downloadable_at(&self, index: i32) -> bool {
        self.variant(index)
            .is_some_and(|variant| variant.downloadable)
    }

    pub fn related_game_id_at(&self, index: i32) -> QString {
        self.related_game(index)
            .map(|game| qstring(&game.id))
            .unwrap_or_default()
    }

    pub fn related_game_database_id_at(&self, index: i32) -> i32 {
        self.related_game(index)
            .map(|game| i32::try_from(game.launchbox_db_id).unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn related_game_title_at(&self, index: i32) -> QString {
        self.related_game(index)
            .map(|game| qstring(&game.title))
            .unwrap_or_default()
    }

    pub fn related_game_platform_at(&self, index: i32) -> QString {
        self.related_game(index)
            .map(|game| qstring(&game.platform))
            .unwrap_or_default()
    }

    pub fn related_game_reason_at(&self, index: i32) -> QString {
        self.related_game(index)
            .map(|game| qstring(&game.reason))
            .unwrap_or_default()
    }

    pub fn related_game_is_local_at(&self, index: i32) -> bool {
        self.related_game(index).is_some_and(|game| game.local)
    }

    pub fn related_game_is_downloadable_at(&self, index: i32) -> bool {
        self.related_game(index)
            .is_some_and(|game| game.downloadable)
    }

    pub fn file_name_at(&self, index: i32) -> QString {
        self.file(index)
            .map(file_name_label)
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn file_detail_at(&self, index: i32) -> QString {
        self.file(index)
            .map(|file| {
                qstring(file_detail_label(
                    file,
                    (index == 0).then_some("BEST MATCH"),
                    &self.rust().canonical_title,
                ))
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
                let name = local_file_name(path);
                if self.rust().local_file_paths.len() == 1 {
                    qstring(name)
                } else {
                    qstring(format!("{} · {}", name, path.display()))
                }
            })
            .unwrap_or_default()
    }

    pub fn local_file_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().local_file_paths.get(index))
            .map(|path| qstring(local_file_name(path)))
            .unwrap_or_default()
    }

    pub fn local_file_path_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().local_file_paths.get(index))
            .map(|path| qstring(path.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn local_file_is_preferred_at(&self, index: i32) -> bool {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().local_file_paths.get(index))
            .is_some_and(|path| self.rust().preferred_local_file_path.as_ref() == Some(path))
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

    fn bundle_group(&self, index: i32) -> Option<&BundleCandidateGroup> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().bundle_candidates.get(index))
    }

    fn bundle_file(&self, bundle_index: i32, file_index: i32) -> Option<&TorrentFileCandidate> {
        usize::try_from(file_index)
            .ok()
            .and_then(|file_index| self.bundle_group(bundle_index)?.files.get(file_index))
    }

    fn variant(&self, index: i32) -> Option<&GameVariant> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().variants.get(index))
    }

    fn related_game(&self, index: i32) -> Option<&RelatedGame> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().related_games.get(index))
    }

    fn file(&self, index: i32) -> Option<&TorrentFileCandidate> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().files.get(index))
    }
}

fn bundle_group_message(group: &BundleCandidateGroup, source: &str) -> String {
    if !group.loaded {
        return "Inspecting torrent contents…".to_owned();
    }
    if !group.error.is_empty() {
        return format!("Could not inspect {source}: {}", group.error);
    }
    if group.files.is_empty() {
        return format!("No matching game files were found in {source}.");
    }
    format!(
        "{} ranked download candidate{}",
        group.files.len(),
        if group.files.len() == 1 { "" } else { "s" }
    )
}

fn download_candidate_location(
    groups: &[BundleCandidateGroup],
    index: i32,
) -> Option<(usize, usize)> {
    let mut remaining = usize::try_from(index).ok()?;
    for (bundle_index, group) in groups.iter().enumerate() {
        if remaining < group.files.len() {
            return Some((bundle_index, remaining));
        }
        remaining = remaining.checked_sub(group.files.len())?;
    }
    None
}

fn download_source_location(groups: &[BundleCandidateGroup], index: i32) -> Option<usize> {
    let mut remaining = usize::try_from(index).ok()?;
    groups.iter().enumerate().find_map(|(bundle_index, group)| {
        if group.files.is_empty() {
            return None;
        }
        if remaining == 0 {
            Some(bundle_index)
        } else {
            remaining -= 1;
            None
        }
    })
}

fn preferred_loaded_group_index(groups: &[BundleCandidateGroup]) -> Option<usize> {
    groups.iter().enumerate().find_map(|(index, group)| {
        if group.files.is_empty() {
            return None;
        }
        groups[..index]
            .iter()
            .all(|earlier| earlier.loaded && earlier.files.is_empty())
            .then_some(index)
    })
}

fn file_name_label(file: &TorrentFileCandidate) -> String {
    if let Some(plan) = &file.download_plan {
        if plan.is_optical_multidisc() {
            format!("{} — {}-disc set", plan.display_name, plan.disc_count())
        } else if plan.is_exo_archive_set() {
            format!("{} — eXo archive set", plan.display_name)
        } else {
            format!("{} — machine layout", plan.display_name)
        }
    } else {
        file.filename.clone()
    }
}

fn file_detail_label(
    file: &TorrentFileCandidate,
    badge: Option<&str>,
    canonical_title: &str,
) -> String {
    let mut details = Vec::new();
    if let Some(badge) = badge {
        details.push(badge.to_owned());
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
    if !file.matched_title.is_empty() && file.matched_title != canonical_title {
        details.push(format!("matched as {}", file.matched_title));
    }
    details.push(format!("{:.0}% title match", file.match_score * 100.0));
    details.push(game_details::format_bytes(file.byte_size));
    details.push(format!("torrent file #{}", file.index));
    details.join(" · ")
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
    let settings = effective_download_settings(store.load()?, &bundle);
    let registered_source = bundle.source_kind == "manual_torrent";
    let password = crate::settings::load_password()?.unwrap_or_default();
    let torrent_bytes = game_details::torrent_bytes(&bundle)?;
    let request = torrent_enqueue_request(
        game_id.clone(),
        launchbox_db_id,
        title.clone(),
        platform.clone(),
        bundle,
        file.clone(),
        torrent_bytes,
    )?;
    let job = crate::qbittorrent::enqueue(&settings, &password, &store, request)?;
    if registered_source {
        let source = store
            .registered_torrent_source(&job.info_hash)?
            .context("registered torrent source disappeared before its download was recorded")?;
        store.record_manual_torrent_enqueue(
            &crate::settings::ManualTorrentSourceReceipt {
                game_uid: game_id,
                launchbox_db_id,
                canonical_title: title.clone(),
                platform,
                source_file_name: source.source_file_name,
                torrent_name: source.torrent_name,
                torrent_sha256: source.torrent_sha256,
                info_hash: source.info_hash,
                selected_file_index: u32::try_from(file.index)
                    .context("torrent file index is too large")?,
                selected_file_path: job.torrent_file_path.clone(),
                queued_job_id: job.id.clone(),
                reviewed_at: crate::settings::unix_timestamp(),
            },
            &job,
        )?;
    } else {
        store.upsert_job(&job)?;
    }
    Ok(title)
}

fn effective_download_settings(
    mut settings: crate::settings::AppSettings,
    bundle: &MinervaBundle,
) -> crate::settings::AppSettings {
    if bundle.source_kind == "manual_torrent" {
        settings.download_entire_torrent = false;
    }
    settings
}

fn torrent_enqueue_request(
    game_id: String,
    launchbox_db_id: i64,
    title: String,
    platform: String,
    bundle: MinervaBundle,
    file: TorrentFileCandidate,
    torrent_bytes: Vec<u8>,
) -> anyhow::Result<crate::qbittorrent::EnqueueRequest> {
    let file_index = u32::try_from(file.index)
        .map_err(|_| anyhow::anyhow!("torrent file index is too large"))?;
    let torrent_url = if bundle.source_kind == "manual_torrent" {
        format!(
            "manual-torrent:sha256:{}",
            bundle.torrent_sha256.to_ascii_lowercase()
        )
    } else {
        bundle.torrent_url
    };
    Ok(crate::qbittorrent::EnqueueRequest {
        game_id,
        launchbox_db_id,
        title,
        platform,
        source_kind: bundle.source_kind,
        torrent_url,
        torrent_bytes,
        selected_file_index: file_index,
        selected_file_path: file.filename,
        download_plan: file.download_plan,
        adopt_existing_torrent: false,
    })
}

fn preflight_mode_label(preflight: &crate::qbittorrent::DownloadPreflight) -> String {
    let selected = game_details::format_bytes(preflight.selected_bytes);
    let additional = game_details::format_bytes(preflight.required_download_bytes);
    if preflight.required_download_bytes == preflight.selected_bytes {
        format!("{} · {selected}", preflight.selection_kind)
    } else if preflight.required_download_bytes == 0 {
        format!(
            "{} · {selected} · already present in the shared transfer",
            preflight.selection_kind
        )
    } else {
        format!(
            "{} · {selected} selected · {additional} additional transfer",
            preflight.selection_kind
        )
    }
}

fn preflight_storage_summary(preflight: &crate::qbittorrent::DownloadPreflight) -> String {
    let download = game_details::format_bytes(preflight.required_download_bytes);
    let available = game_details::format_bytes(preflight.download_available_bytes);
    if preflight.required_install_bytes == 0 {
        return format!(
            "{download} additional download · {available} free · {}",
            preflight.download_path.display()
        );
    }
    let install = game_details::format_bytes(preflight.required_install_bytes);
    if preflight.shared_filesystem {
        format!(
            "{download} download + {install} copy · {available} shared free space · {}",
            preflight.download_path.display()
        )
    } else {
        format!(
            "{download} download · {available} free at {}\n{install} copy · {} free at {}",
            preflight.download_path.display(),
            game_details::format_bytes(preflight.install_available_bytes),
            preflight.install_path.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BundleCandidateGroup, LaunchProfileTarget, download_candidate_location,
        download_source_location, effective_download_settings, format_last_played,
        format_play_time, format_release_date, format_session_duration, metadata_save_messages,
        preferred_loaded_group_index, preview_for_launch_profile_target, session_outcome_label,
        steam_store_url_string, validate_launch_profile_template,
    };
    use crate::emulator::effective_launch_preview_values;
    use crate::game_details::{BundleMatchKind, MinervaBundle, TorrentFileCandidate};

    fn candidate_group(loaded: bool, has_candidate: bool) -> BundleCandidateGroup {
        BundleCandidateGroup {
            loaded,
            files: has_candidate
                .then(|| TorrentFileCandidate {
                    index: 0,
                    filename: "Faxanadu (USA).zip".to_owned(),
                    byte_size: 1,
                    match_score: 1.0,
                    matched_title: "Faxanadu".to_owned(),
                    region: "USA".to_owned(),
                    version: String::new(),
                    download_plan: None,
                })
                .into_iter()
                .collect(),
            error: String::new(),
        }
    }

    fn bundle(source_kind: &str) -> MinervaBundle {
        MinervaBundle {
            torrent_id: 1,
            torrent_url: "registered-torrent:0123456789abcdef0123456789abcdef01234567".to_owned(),
            source_kind: source_kind.to_owned(),
            torrent_sha256: "0".repeat(64),
            collection: "Reviewed source".to_owned(),
            provider_platform: "Nintendo Switch".to_owned(),
            rom_count: 2,
            total_size: 2,
            match_kind: BundleMatchKind::MappedName,
        }
    }

    #[test]
    fn registered_sources_always_download_only_the_selected_game() {
        let mut settings = crate::settings::AppSettings::default();
        settings.download_entire_torrent = true;

        assert!(
            effective_download_settings(settings.clone(), &bundle("minerva"))
                .download_entire_torrent
        );
        assert!(
            !effective_download_settings(settings, &bundle("manual_torrent"))
                .download_entire_torrent
        );
    }

    #[test]
    fn progressive_candidates_wait_for_higher_priority_sources_before_defaulting() {
        let groups = vec![candidate_group(false, false), candidate_group(true, true)];
        assert_eq!(preferred_loaded_group_index(&groups), None);

        let groups = vec![candidate_group(true, false), candidate_group(true, true)];
        assert_eq!(preferred_loaded_group_index(&groups), Some(1));

        let groups = vec![candidate_group(true, true), candidate_group(true, true)];
        assert_eq!(preferred_loaded_group_index(&groups), Some(0));
    }

    #[test]
    fn couch_download_candidates_flatten_sources_without_losing_exact_indices() {
        let mut first = candidate_group(true, true);
        first.files.push(TorrentFileCandidate {
            index: 1,
            filename: "Faxanadu (Europe).zip".to_owned(),
            byte_size: 2,
            match_score: 0.9,
            matched_title: "Faxanadu".to_owned(),
            region: "Europe".to_owned(),
            version: String::new(),
            download_plan: None,
        });
        let second = candidate_group(true, true);
        let groups = vec![first, BundleCandidateGroup::default(), second];

        assert_eq!(download_candidate_location(&groups, 0), Some((0, 0)));
        assert_eq!(download_candidate_location(&groups, 1), Some((0, 1)));
        assert_eq!(download_candidate_location(&groups, 2), Some((2, 0)));
        assert_eq!(download_candidate_location(&groups, 3), None);
        assert_eq!(download_candidate_location(&groups, -1), None);
        assert_eq!(download_source_location(&groups, 0), Some(0));
        assert_eq!(download_source_location(&groups, 1), Some(2));
        assert_eq!(download_source_location(&groups, 2), None);
        assert_eq!(download_source_location(&groups, -1), None);
    }

    #[test]
    fn activity_labels_stay_compact_in_the_details_card() {
        assert_eq!(format_play_time(0, 0), "Never played");
        assert_eq!(format_play_time(0, 1), "Under 1 min");
        assert_eq!(format_play_time(59, 1), "Under 1 min");
        assert_eq!(format_play_time(60, 1), "1 min");
        assert_eq!(format_play_time(3_600, 1), "1 hr");
        assert_eq!(format_play_time(3_900, 1), "1 hr 5 min");
        assert_eq!(format_last_played(0), "Never");
        assert_eq!(format_session_duration(0, "running"), "In progress");
        assert_eq!(format_session_duration(30, "failed"), "Under 1 min");
        assert_eq!(format_session_duration(845, "terminated"), "14 min");
        assert_eq!(format_session_duration(3_723, "completed"), "1 hr 2 min");
        assert_eq!(session_outcome_label("terminated"), "Interrupted");
        assert_eq!(session_outcome_label("failed"), "Failed");
        assert_eq!(
            format_release_date("1985-09-13T00:00:00+00:00"),
            "Sep 13, 1985"
        );
        assert_eq!(format_release_date("1994"), "1994");
    }

    #[test]
    fn launch_profile_editor_rejects_placeholders_outside_the_exact_context() {
        let target = LaunchProfileTarget {
            emulator_id: "retroarch".to_owned(),
            emulator_label: "RetroArch · FCEUmm".to_owned(),
            runtime_kind: "retroarch",
            core_name: "fceumm".to_owned(),
            default_template: "--verbose -L %{core} %f".to_owned(),
            available_placeholders: vec!["core".to_owned(), "file".to_owned()],
            preview_extra_insert_index: 3,
        };
        validate_launch_profile_template(&target, "-L %{core} %f").unwrap();
        let error = validate_launch_profile_template(&target, "%{config}").unwrap_err();
        assert!(error.to_string().contains("unavailable"));

        let preview = preview_for_launch_profile_target(&target, "--latency 1", "").unwrap();
        assert_eq!(
            preview.arguments,
            [
                "--verbose",
                "-L",
                "<retroarch-core>",
                "--latency",
                "1",
                "<selected-file>",
            ]
        );
        let effective = effective_launch_preview_values("", "-L %{core} %f", "--inherited", "%f");
        assert_eq!(effective.0, "--inherited");
        assert_eq!(effective.1, "-L %{core} %f");
    }

    #[test]
    fn metadata_save_copy_distinguishes_tags_from_metadata_resets() {
        assert_eq!(
            metadata_save_messages(false, true, 2, 0).0,
            "Saved 2 local tags without changing canonical metadata."
        );
        assert_eq!(
            metadata_save_messages(true, true, 2, 0).0,
            "Restored canonical catalog metadata and kept 2 local tags."
        );
        assert_eq!(
            metadata_save_messages(false, false, 1, 0).0,
            "Saved local metadata, 1 local tag without changing canonical identity."
        );
        assert_eq!(
            metadata_save_messages(false, true, 2, 2).0,
            "Saved 2 local tags and 2 custom fields without changing canonical metadata."
        );
        assert_eq!(
            metadata_save_messages(true, true, 0, 1).1,
            "Canonical catalog metadata restored. Local custom fields were kept."
        );
    }

    #[test]
    fn steam_store_links_require_an_exact_positive_app_id() {
        assert_eq!(
            steam_store_url_string(620),
            "https://store.steampowered.com/app/620"
        );
        assert!(steam_store_url_string(0).is_empty());
        assert!(steam_store_url_string(-1).is_empty());
    }
}
