mod arcade;
mod arcade_download;
mod catalog;
mod collection_identity;
pub mod collection_identity_model;
mod collections;
mod controllers;
mod couch_theme;
mod download_plan;
pub mod download_queue_model;
mod emulator;
mod emulator_manager;
pub mod emulator_manager_model;
pub mod emulator_update_model;
mod emumovies;
pub mod emumovies_model;
mod exo_install;
mod external_torrent;
pub mod external_torrent_model;
mod firmware;
mod firmware_audit;
pub mod firmware_audit_model;
mod game_details;
pub mod game_details_model;
pub mod gamepad_input;
mod hover_preview;
mod igdb;
pub mod igdb_model;
mod ingest;
pub mod launch_profile_manager_model;
mod library_audit;
pub mod library_audit_model;
pub mod library_model;
mod list_view;
mod local_import;
pub mod local_import_model;
mod local_provider_manifest;
pub mod local_provider_manifest_model;
mod media;
mod media_acquisition;
mod media_audit;
pub mod media_audit_model;
mod media_repair_batch;
mod platform_process;
mod profile_backup;
mod provider_image;
mod qbittorrent;
mod region_priority;
mod retroarch_shaders;
mod rom_launch_preparation;
mod screenscraper;
pub mod screenscraper_model;
mod settings;
pub mod settings_model;
mod steamgriddb;
pub mod steamgriddb_model;
mod tags;
mod watched_torrent;
pub mod watched_torrent_model;
pub mod web_artwork_model;

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

static PROCESS_STARTED: OnceLock<Instant> = OnceLock::new();
static WEB_ARTWORK_PROBE_FIXTURE: OnceLock<std::path::PathBuf> = OnceLock::new();

pub fn mark_process_started() {
    let _ = PROCESS_STARTED.set(Instant::now());
}

pub(crate) fn startup_elapsed() -> Duration {
    PROCESS_STARTED.get_or_init(Instant::now).elapsed()
}

pub(crate) fn web_artwork_probe_fixture() -> Option<&'static std::path::Path> {
    WEB_ARTWORK_PROBE_FIXTURE
        .get()
        .map(std::path::PathBuf::as_path)
}

fn seed_web_artwork_ui_probe() -> anyhow::Result<()> {
    use std::io::Write;

    let path = std::env::temp_dir().join(format!(
        "lunchbox-web-artwork-probe-{}.png",
        std::process::id()
    ));
    let mut file = std::fs::File::create(&path)?;
    file.write_all(&[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00, 0x00, 0xb5,
        0x1c, 0x0c, 0x02, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x64,
        0xf8, 0x0f, 0x00, 0x01, 0x05, 0x01, 0x01, 0x27, 0x18, 0xe3, 0x66, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ])?;
    file.sync_all()?;
    let _ = WEB_ARTWORK_PROBE_FIXTURE.set(path);
    Ok(())
}

pub fn initialize_qt() {
    cxx_qt::init_crate!(cxx_qt);
    cxx_qt::init_crate!(cxx_qt_lib);
    cxx_qt::init_crate!(lunchbox_app);
    cxx_qt::init_qml_module!("Lunchbox");
}

pub fn run() -> i32 {
    if std::env::args().any(|argument| argument == "--emumovies-soundtrack-probe") {
        return match emumovies_model::soundtrack_saved_probe() {
            Ok(evidence) => {
                println!("LUNCHBOX_EMUMOVIES_SOUNDTRACK_READY {evidence}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_EMUMOVIES_SOUNDTRACK_FAILED error={error:#}");
                1
            }
        };
    }

    if let Some(path) = std::env::args()
        .skip_while(|argument| argument != "--emumovies-list-library-path")
        .nth(1)
    {
        return match emumovies_model::list_saved_library_path(&path) {
            Ok(entries) => {
                println!(
                    "LUNCHBOX_EMUMOVIES_LIBRARY_PATH path={path:?} entries={}",
                    entries.len()
                );
                for entry in entries {
                    println!("LUNCHBOX_EMUMOVIES_LIBRARY_ENTRY path={entry:?}");
                }
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_EMUMOVIES_LIBRARY_FAILED path={path:?} error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--web-artwork-ui-probe")
        && let Err(error) = seed_web_artwork_ui_probe()
    {
        eprintln!("LUNCHBOX_WEB_ARTWORK_UI_FAILED seed error={error:#}");
        return 1;
    }
    if std::env::args().any(|argument| argument == "--firmware-audit-ui-probe") {
        match firmware_audit::seed_ui_probe() {
            Ok(path) => println!("LUNCHBOX_FIRMWARE_AUDIT_SEEDED path={path:?}"),
            Err(error) => {
                eprintln!("LUNCHBOX_FIRMWARE_AUDIT_UI_FAILED seed error={error:#}");
                return 1;
            }
        }
    }
    match settings::state_database_path()
        .and_then(|path| profile_backup::apply_pending_restore(&path))
    {
        Ok(Some(summary)) => println!(
            "LUNCHBOX_PROFILE_RESTORED collections={} installed_games={} customized_games={} import_profiles={} themes={}",
            summary.collections,
            summary.installed_games,
            summary.customized_games,
            summary.import_profiles,
            summary.themes
        ),
        Ok(None) => {}
        Err(error) => eprintln!("LUNCHBOX_PROFILE_RESTORE_FAILED error={error:#}"),
    }

    if std::env::args().any(|argument| {
        matches!(
            argument.as_str(),
            "--hover-preview-ui-probe" | "--hover-artwork-transition-ui-probe"
        )
    }) {
        match hover_preview::prepare_ui_probe() {
            Ok(Some(path)) => println!("LUNCHBOX_HOVER_PREVIEW_SEEDED path={path:?}"),
            Ok(None) => println!("LUNCHBOX_HOVER_PREVIEW_DOWNLOAD_PROBE clean_cache=true"),
            Err(error) => {
                eprintln!("LUNCHBOX_HOVER_PREVIEW_UI_FAILED seed error={error:#}");
                return 1;
            }
        }
    }

    if std::env::args().any(|argument| argument == "--install-management-ui-probe") {
        match ingest::seed_install_management_ui_probe() {
            Ok(path) => println!("LUNCHBOX_INSTALL_MANAGEMENT_SEEDED path={:?}", path),
            Err(error) => {
                eprintln!("LUNCHBOX_INSTALL_MANAGEMENT_UI_FAILED seed error={error:#}");
                return 1;
            }
        }
    }

    if std::env::args().any(|argument| argument == "--import-profile-ui-probe")
        && let Err(error) = local_import::seed_import_profile_ui_probe()
    {
        eprintln!("LUNCHBOX_IMPORT_PROFILE_UI_FAILED seed error={error:#}");
        return 1;
    }

    if std::env::args().any(|argument| argument == "--import-profile-batch-ui-probe")
        && let Err(error) = local_import::seed_import_profile_batch_ui_probe()
    {
        eprintln!("LUNCHBOX_IMPORT_PROFILE_BATCH_UI_FAILED seed error={error:#}");
        return 1;
    }

    if std::env::args().any(|argument| argument == "--hash-cache-ui-probe")
        && let Err(error) = local_import::seed_hash_cache_ui_probe()
    {
        eprintln!("LUNCHBOX_HASH_CACHE_UI_FAILED seed error={error:#}");
        return 1;
    }

    if std::env::args().any(|argument| argument == "--couch-theme-ui-probe")
        && let Err(error) = couch_theme::seed_ui_probe()
    {
        eprintln!("LUNCHBOX_COUCH_THEME_UI_FAILED seed error={error:#}");
        return 1;
    }

    if std::env::args().any(|argument| argument == "--activity-history-ui-probe")
        && let Err(error) = settings::seed_activity_history_probe()
    {
        eprintln!("LUNCHBOX_ACTIVITY_HISTORY_UI_FAILED seed error={error:#}");
        return 1;
    }

    if std::env::args().any(|argument| argument == "--emulator-lifecycle-recovery-ui-probe") {
        match settings::seed_emulator_lifecycle_recovery_probe() {
            Ok(evidence) => println!("LUNCHBOX_EMULATOR_RECOVERY_SEEDED {evidence}"),
            Err(error) => {
                eprintln!("LUNCHBOX_EMULATOR_RECOVERY_UI_FAILED seed error={error:#}");
                return 1;
            }
        }
    }

    if std::env::args().any(|argument| argument == "--media-repair-recovery-ui-probe") {
        match media_repair_batch::seed_media_repair_recovery_probe() {
            Ok(evidence) => println!("LUNCHBOX_MEDIA_REPAIR_RECOVERY_SEEDED {evidence}"),
            Err(error) => {
                eprintln!("LUNCHBOX_MEDIA_REPAIR_RECOVERY_UI_FAILED seed error={error:#}");
                return 1;
            }
        }
    }

    if std::env::args().any(|argument| argument == "--library-audit-fixture") {
        return match library_audit::library_audit_fixture_probe() {
            Ok(evidence) => {
                println!("LUNCHBOX_LIBRARY_AUDIT_READY {evidence}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_LIBRARY_AUDIT_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--archive-import-probe") {
        return match local_import::archive_import_probe() {
            Ok(evidence) => {
                println!("LUNCHBOX_ARCHIVE_IMPORT_READY {evidence}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_ARCHIVE_IMPORT_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--multi-archive-import-probe") {
        return match local_import::multi_archive_import_probe() {
            Ok(evidence) => {
                println!("LUNCHBOX_MULTI_ARCHIVE_IMPORT_READY {evidence}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_MULTI_ARCHIVE_IMPORT_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--manual-match-probe") {
        return match local_import::manual_match_probe() {
            Ok(evidence) => {
                println!("LUNCHBOX_MANUAL_MATCH_READY {evidence}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_MANUAL_MATCH_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--alternate-title-probe") {
        return match game_details::alternate_title_probe() {
            Ok(evidence) => {
                println!("LUNCHBOX_ALTERNATE_TITLE_READY {evidence}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_ALTERNATE_TITLE_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--variant-probe") {
        return match game_details::variant_probe() {
            Ok(evidence) => {
                println!("LUNCHBOX_VARIANTS_READY {evidence}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_VARIANTS_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--faxanadu-candidate-probe") {
        return match game_details::faxanadu_candidate_probe() {
            Ok(evidence) => {
                println!("LUNCHBOX_FAXANADU_CANDIDATE_READY {evidence}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_FAXANADU_CANDIDATE_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--emulator-update-probe") {
        return match emulator_manager::load_available_emulator_updates() {
            Ok(inventory) => {
                println!(
                    "LUNCHBOX_EMULATOR_UPDATES_READY updates={} warnings={}",
                    inventory.updates.len(),
                    inventory.warnings.len()
                );
                for update in inventory.updates {
                    println!(
                        "LUNCHBOX_EMULATOR_UPDATE name={:?} source={:?} package={:?} current={:?} available={:?}",
                        update.display_name,
                        update.source_label,
                        update.row.package_id,
                        update.current_version,
                        update.available_version
                    );
                }
                for warning in inventory.warnings {
                    eprintln!("LUNCHBOX_EMULATOR_UPDATE_WARNING {warning}");
                }
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_EMULATOR_UPDATES_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--controller-probe") {
        let inventory = controllers::controller_inventory();
        println!(
            "LUNCHBOX_CONTROLLER_READY provider={} available={} service={} version={:?} controllers={} managed={} targets={} warnings={}",
            inventory.provider.provider,
            inventory.provider.available,
            inventory.provider.service_accessible,
            inventory.provider.version,
            inventory.controllers.len(),
            inventory.managed_device_count,
            inventory.supported_targets.len(),
            inventory.warnings.len()
        );
        for (index, controller) in inventory.controllers.iter().enumerate() {
            println!(
                "LUNCHBOX_CONTROLLER_DEVICE player={} id={:?} name={:?} virtual={}",
                index + 1,
                controller.stable_id,
                controller.name,
                controller.is_virtual
            );
        }
        for warning in inventory.warnings {
            eprintln!("LUNCHBOX_CONTROLLER_WARNING {warning}");
        }
        return 0;
    }

    if std::env::args().any(|argument| argument == "--manual-candidate-probe") {
        let title = "Cooking Pico - Minna to Issho ni Hajimete Cooking! (Japan)";
        return match media_acquisition::manual_candidate_probe(title) {
            Ok(candidate) => {
                println!("LUNCHBOX_MANUAL_CANDIDATE title={title:?} {candidate}");
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_MANUAL_CANDIDATE_FAILED error={error:#}");
                1
            }
        };
    }

    if std::env::args().any(|argument| argument == "--initialize-state") {
        return match settings::SettingsStore::open_default() {
            Ok(store) => {
                println!("LUNCHBOX_STATE_READY path={:?}", store.path());
                0
            }
            Err(error) => {
                eprintln!("LUNCHBOX_STATE_FAILED error={error:#}");
                1
            }
        };
    }

    initialize_qt();

    let mut application = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    let engine = engine
        .as_mut()
        .expect("Qt did not construct a QQmlApplicationEngine");
    engine.load(&QUrl::from("qrc:/qt/qml/Lunchbox/qml/Main.qml"));

    application
        .as_mut()
        .expect("Qt did not construct a QGuiApplication")
        .exec()
}
