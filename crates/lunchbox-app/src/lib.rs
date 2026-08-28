mod arcade_download;
mod catalog;
mod controllers;
mod download_plan;
pub mod download_queue_model;
mod emulator;
mod emulator_manager;
pub mod emulator_manager_model;
pub mod emulator_update_model;
mod exo_install;
mod firmware;
mod game_details;
pub mod game_details_model;
mod ingest;
pub mod launch_profile_manager_model;
pub mod library_model;
mod local_import;
pub mod local_import_model;
mod media;
mod media_acquisition;
mod qbittorrent;
mod region_priority;
mod settings;
pub mod settings_model;

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

static PROCESS_STARTED: OnceLock<Instant> = OnceLock::new();

pub fn mark_process_started() {
    let _ = PROCESS_STARTED.set(Instant::now());
}

pub(crate) fn startup_elapsed() -> Duration {
    PROCESS_STARTED.get_or_init(Instant::now).elapsed()
}

pub fn initialize_qt() {
    cxx_qt::init_crate!(cxx_qt);
    cxx_qt::init_crate!(cxx_qt_lib);
    cxx_qt::init_crate!(lunchbox_app);
    cxx_qt::init_qml_module!("Lunchbox");
}

pub fn run() -> i32 {
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
