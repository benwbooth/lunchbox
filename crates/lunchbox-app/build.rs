use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("Lunchbox")
            .depend("QtQuick.Dialogs")
            .depend("QtQuick.Layouts")
            .depend("QtQuick3D")
            .depend("QtMultimedia")
            .qml_files([
                "qml/CouchModeView.qml",
                "qml/Box3DViewer.qml",
                "qml/Main.qml",
            ]),
    )
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .qt_module("Quick3D")
    .qt_module("Multimedia")
    .file("src/download_queue_model.rs")
    .file("src/emulator_manager_model.rs")
    .file("src/emulator_update_model.rs")
    .file("src/external_torrent_model.rs")
    .file("src/game_details_model.rs")
    .file("src/gamepad_input.rs")
    .file("src/igdb_model.rs")
    .file("src/launch_profile_manager_model.rs")
    .file("src/library_audit_model.rs")
    .file("src/library_model.rs")
    .file("src/local_import_model.rs")
    .file("src/settings_model.rs")
    .file("src/steamgriddb_model.rs")
    .build();
}
