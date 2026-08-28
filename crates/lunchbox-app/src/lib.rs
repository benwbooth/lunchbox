mod catalog;
mod download_plan;
pub mod download_queue_model;
mod emulator;
mod exo_install;
mod game_details;
pub mod game_details_model;
mod ingest;
pub mod library_model;
mod local_import;
pub mod local_import_model;
mod media;
mod qbittorrent;
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
