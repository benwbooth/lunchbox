mod catalog;
pub mod library_model;

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
