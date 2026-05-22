use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new()
        .qt_module("Network")
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qml_module(QmlModule {
            uri: "com.benkahn.soundboard",
            rust_files: &[
                "src/qobjects/controller.rs",
                "src/qobjects/settings.rs",
            ],
            qml_files: &[
                "qml/Main.qml",
                "qml/TabPage.qml",
                "qml/SoundButton.qml",
                "qml/SettingsDialog.qml",
            ],
            ..Default::default()
        })
        .build();
}
