use cxx_qt_build::{CxxQtBuilder, QObjectHeaderOpts, QmlModule};

fn main() {
    CxxQtBuilder::new()
        .include_prefix("src/cpp")
        .qobject_header(QObjectHeaderOpts::from("src/cpp/key_forwarder.h"))
        .cc_builder(|builder| {
            builder
                .include("src/cpp")
                .file("src/cpp/key_forwarder.cpp")
                .file("src/cpp/app_identity.cpp");
        })
        .qt_module("Network")
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qt_module("QuickDialogs2")
        .qml_module(QmlModule {
            uri: "com.benkahn.soundboard",
            rust_files: &["src/qobjects/controller.rs", "src/qobjects/settings.rs"],
            qml_files: &[
                "qml/Main.qml",
                "qml/TabPage.qml",
                "qml/SoundButton.qml",
                "qml/ShortcutCapture.qml",
                "qml/SettingsDialog.qml",
            ],
            ..Default::default()
        })
        .build();
}
