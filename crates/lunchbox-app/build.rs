use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("Lunchbox")
            .depend("QtQuick.Layouts")
            .qml_files(["qml/Main.qml"]),
    )
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .file("src/game_details_model.rs")
    .file("src/library_model.rs")
    .build();
}
