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
        #[qproperty(QString, metadata_notes)]
        #[qproperty(QString, metadata_tags)]
        #[qproperty(i32, metadata_revision)]
        #[qproperty(i32, tag_count)]
        #[qproperty(i32, tag_revision)]
        #[qproperty(i32, variant_count)]
        #[qproperty(i32, alternate_title_count)]
        #[qproperty(QString, message)]
        #[qproperty(bool, media_visible)]
        #[qproperty(bool, video_available)]
        #[qproperty(bool, manual_available)]
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
        #[qproperty(bool, launch_profile_open)]
        #[qproperty(QString, launch_profile_scope)]
        #[qproperty(QString, launch_profile_default_template)]
        #[qproperty(QString, launch_profile_effective_template)]
        #[qproperty(QString, launch_profile_extra_arguments)]
        #[qproperty(QString, launch_profile_command_template)]
        #[qproperty(QString, launch_profile_inheritance)]
        #[qproperty(QString, launch_profile_status)]
        #[qproperty(i32, launch_profile_revision)]
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
        fn download_manual(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn refresh_manual_download(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn cancel_manual_download(self: Pin<&mut GameDetailsModel>);

        #[qinvokable]
        fn bundle_title_at(self: &GameDetailsModel, index: i32) -> QString;

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
    self, AlternateTitle, GameDetails, GameVariant, MinervaBundle, ReleasePreferences,
    TorrentFileCandidate,
};
use crate::settings::{GameMetadata, GameMetadataOverride, SettingsStore};

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
    metadata_notes: QString,
    metadata_tags: QString,
    metadata_revision: i32,
    tag_count: i32,
    tag_revision: i32,
    variant_count: i32,
    alternate_title_count: i32,
    message: QString,
    media_visible: bool,
    video_available: bool,
    manual_available: bool,
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
    launch_profile_open: bool,
    launch_profile_scope: QString,
    launch_profile_default_template: QString,
    launch_profile_effective_template: QString,
    launch_profile_extra_arguments: QString,
    launch_profile_command_template: QString,
    launch_profile_inheritance: QString,
    launch_profile_status: QString,
    launch_profile_revision: i32,
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
    canonical_title: String,
    canonical_metadata: GameMetadata,
    effective_metadata: GameMetadata,
    metadata_generation: u64,
    current_tags: Vec<String>,
    bundles: Vec<MinervaBundle>,
    variants: Vec<GameVariant>,
    alternate_titles: Vec<AlternateTitle>,
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
            metadata_notes: QString::default(),
            metadata_tags: QString::default(),
            metadata_revision: 0,
            tag_count: 0,
            tag_revision: 0,
            variant_count: 0,
            alternate_title_count: 0,
            message: QString::from("Select a game to inspect it."),
            media_visible: false,
            video_available: false,
            manual_available: false,
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
            launch_profile_open: false,
            launch_profile_scope: QString::from("game"),
            launch_profile_default_template: QString::default(),
            launch_profile_effective_template: QString::default(),
            launch_profile_extra_arguments: QString::default(),
            launch_profile_command_template: QString::default(),
            launch_profile_inheritance: QString::default(),
            launch_profile_status: QString::default(),
            launch_profile_revision: 0,
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
            canonical_title: String::new(),
            canonical_metadata: GameMetadata::default(),
            effective_metadata: GameMetadata::default(),
            metadata_generation: 0,
            current_tags: Vec::new(),
            bundles: Vec::new(),
            variants: Vec::new(),
            alternate_titles: Vec::new(),
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

fn local_file_url(path: &std::path::Path) -> QUrl {
    QUrl::from_local_file(&qstring(path.to_string_lossy()))
}

fn count_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn metadata_save_messages(
    reset_requested: bool,
    metadata_is_canonical: bool,
    tag_count: usize,
) -> (String, String) {
    let tag_label = if tag_count == 1 { "tag" } else { "tags" };
    if reset_requested {
        let detail = if tag_count == 0 {
            "Restored canonical catalog metadata.".to_owned()
        } else {
            format!("Restored canonical catalog metadata and kept {tag_count} local {tag_label}.")
        };
        return (
            detail,
            if tag_count == 0 {
                "Canonical catalog metadata restored.".to_owned()
            } else {
                "Canonical catalog metadata restored. Local tags were kept.".to_owned()
            },
        );
    }
    if metadata_is_canonical {
        if tag_count == 0 {
            return (
                "No local presentation changes are stored.".to_owned(),
                "Canonical catalog metadata and identity are unchanged.".to_owned(),
            );
        }
        return (
            format!("Saved {tag_count} local {tag_label} without changing canonical metadata."),
            "Local tags saved. Downloads and matching still use canonical identity.".to_owned(),
        );
    }
    if tag_count == 0 {
        return (
            "Saved local metadata without changing canonical identity.".to_owned(),
            "Local metadata saved. Downloads and matching still use canonical identity.".to_owned(),
        );
    }
    (
        format!(
            "Saved local metadata and {tag_count} {tag_label} without changing canonical identity."
        ),
        "Local metadata and tags saved. Downloads and matching still use canonical identity."
            .to_owned(),
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
        if has_cli_flag("--metadata-ui-probe") {
            println!("LUNCHBOX_METADATA_MODEL_SELECT id={}", game_id.to_string());
        }
        let game_id_string = game_id.to_string();
        let title_string = title.to_string();
        let platform_string = platform.to_string();
        self.as_mut().invalidate_preparation();
        self.as_mut().invalidate_launch_state();
        self.as_mut().rust_mut().details_generation =
            self.as_ref().rust().details_generation.wrapping_add(1);
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
        self.as_mut().set_metadata_notes(qstring(&metadata.notes));
        let tags = self.as_ref().rust().current_tags.join(", ");
        self.as_mut().set_metadata_tags(qstring(tags));
        self.as_mut().set_metadata_message(qstring(
            "Only presentation metadata is edited. Stable game identity, platform, provider matches, and files are preserved.",
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
            notes: self.as_ref().metadata_notes().to_string().trim().to_owned(),
        };
        let tags = self.as_ref().metadata_tags().to_string();
        self.as_mut().start_metadata_save(effective, tags, false);
    }

    pub fn reset_metadata(mut self: Pin<&mut Self>) {
        if !*self.as_ref().metadata_open() || *self.as_ref().metadata_busy() {
            return;
        }
        let canonical = self.as_ref().rust().canonical_metadata.clone();
        let tags = self.as_ref().rust().current_tags.join(", ");
        self.as_mut().start_metadata_save(canonical, tags, true);
    }

    fn start_metadata_save(
        mut self: Pin<&mut Self>,
        effective: GameMetadata,
        tag_input: String,
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
                        store.save_game_metadata_and_tags(
                            &game_uid,
                            launchbox_db_id,
                            &canonical_title,
                            &platform,
                            &metadata_override,
                            &tag_input,
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
        result: Result<Vec<String>, String>,
    ) {
        if generation != self.as_ref().rust().metadata_generation {
            return;
        }
        self.as_mut().set_metadata_busy(false);
        match result {
            Ok(tags) => {
                let metadata_is_canonical = metadata_override.is_empty();
                let tag_count = tags.len();
                self.as_mut().rust_mut().current_tags = tags;
                self.as_mut().set_tag_count(count_i32(tag_count));
                let tag_revision = self.as_ref().tag_revision().wrapping_add(1);
                self.as_mut().set_tag_revision(tag_revision);
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
                self.as_mut().set_notes(qstring(&effective.notes));
                self.as_mut()
                    .set_metadata_has_override(!metadata_is_canonical);
                let (metadata_message, message) =
                    metadata_save_messages(reset_requested, metadata_is_canonical, tag_count);
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
        self.as_mut().set_metadata_tags(QString::default());
        self.as_mut().rust_mut().current_tags.clear();
        self.as_mut().set_tag_count(0);
        let tag_revision = self.as_ref().tag_revision().wrapping_add(1);
        self.as_mut().set_tag_revision(tag_revision);
        self.as_mut().set_variant_count(0);
        self.as_mut().set_alternate_title_count(0);
        self.as_mut().set_media_visible(false);
        self.as_mut().set_video_available(false);
        self.as_mut().set_manual_available(false);
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
        self.as_mut().rust_mut().prepared_emulator = None;
        self.as_mut().rust_mut().prepared_install = None;
        self.as_mut().rust_mut().bundles.clear();
        self.as_mut().rust_mut().variants.clear();
        self.as_mut().rust_mut().alternate_titles.clear();
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
                self.as_mut().set_title(qstring(&details.title));
                self.as_mut().set_description(qstring(&details.description));
                self.as_mut()
                    .set_release_date(qstring(format_release_date(&details.release_date)));
                self.as_mut().set_developer(qstring(&details.developer));
                self.as_mut().set_publisher(qstring(&details.publisher));
                self.as_mut().set_genre(qstring(&details.genre));
                self.as_mut().set_players(qstring(&details.players));
                self.as_mut().set_rating(qstring(&details.rating));
                self.as_mut().set_esrb(qstring(&details.esrb));
                self.as_mut()
                    .set_release_type(qstring(&details.release_type));
                self.as_mut().set_notes(qstring(&details.notes));
                let tag_count = details.tags.len();
                self.as_mut().rust_mut().current_tags = details.tags.clone();
                self.as_mut().set_tag_count(count_i32(tag_count));
                let tag_revision = self.as_ref().tag_revision().wrapping_add(1);
                self.as_mut().set_tag_revision(tag_revision);
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

    fn apply_supplemental_media(
        mut self: Pin<&mut Self>,
        media: crate::media::SupplementalMedia,
        video_media_key: String,
        video_progress: Option<crate::settings::MediaPlaybackProgress>,
        manual_transfer: Option<crate::settings::MediaTransfer>,
    ) {
        let video_available = media.video.is_some();
        let manual_available = media.manual.is_some();
        self.as_mut().set_media_visible(true);
        self.as_mut().set_video_available(video_available);
        self.as_mut().set_manual_available(manual_available);
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
        let message = match (video_available, manual_available) {
            (true, true) => "Gameplay video and manual are ready.",
            (true, false) => "Gameplay video is ready.",
            (false, true) => "Game manual is ready.",
            (false, false) => "Cached media is not available yet.",
        };
        self.as_mut().set_media_message(qstring(message));
        let media_revision = self.as_ref().media_revision().wrapping_add(1);
        self.as_mut().set_media_revision(media_revision);
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
            Ok((exact, resolved))
        })();
        match loaded {
            Ok((exact, resolved)) => {
                let default_template = target.default_template;
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
        let available_placeholders =
            crate::emulator::launch_template_placeholders(&default_template)?;
        Ok(LaunchProfileTarget {
            emulator_id: emulator.id.clone(),
            emulator_label: emulator.name.clone(),
            runtime_kind: crate::emulator::EmulatorRuntimeKind::Standalone.key(),
            core_name: String::new(),
            default_template,
            available_placeholders,
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
                            crate::emulator::build_rom_launch_plan_with_customization(
                                path,
                                platform,
                                option,
                                &customization,
                            )?
                        }
                    };
                    let emulator_name = plan.emulator_name.clone();
                    let command_summary = plan.command_summary();
                    let cleanup_paths = plan.cleanup_paths.clone();
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
                    crate::emulator::cleanup_after_launch(&cleanup_paths);
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
                if !file.matched_title.is_empty()
                    && file.matched_title != self.rust().canonical_title
                {
                    details.push(format!("matched as {}", file.matched_title));
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

    fn variant(&self, index: i32) -> Option<&GameVariant> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().variants.get(index))
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
    use super::{
        LaunchProfileTarget, format_last_played, format_play_time, format_release_date,
        metadata_save_messages, validate_launch_profile_template,
    };

    #[test]
    fn activity_labels_stay_compact_in_the_details_card() {
        assert_eq!(format_play_time(0, 0), "Never played");
        assert_eq!(format_play_time(0, 1), "Under 1 min");
        assert_eq!(format_play_time(59, 1), "Under 1 min");
        assert_eq!(format_play_time(60, 1), "1 min");
        assert_eq!(format_play_time(3_600, 1), "1 hr");
        assert_eq!(format_play_time(3_900, 1), "1 hr 5 min");
        assert_eq!(format_last_played(0), "Never");
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
        };
        validate_launch_profile_template(&target, "-L %{core} %f").unwrap();
        let error = validate_launch_profile_template(&target, "%{config}").unwrap_err();
        assert!(error.to_string().contains("unavailable"));
    }

    #[test]
    fn metadata_save_copy_distinguishes_tags_from_metadata_resets() {
        assert_eq!(
            metadata_save_messages(false, true, 2).0,
            "Saved 2 local tags without changing canonical metadata."
        );
        assert_eq!(
            metadata_save_messages(true, true, 2).0,
            "Restored canonical catalog metadata and kept 2 local tags."
        );
        assert_eq!(
            metadata_save_messages(false, false, 1).0,
            "Saved local metadata and 1 tag without changing canonical identity."
        );
    }
}
