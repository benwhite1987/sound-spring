use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use cxx_qt_build::{CxxQtBuilder, QObjectHeaderOpts, QmlModule};
use sha2::{Digest, Sha256};

const ECAPA_MODEL_PATH: &str = "assets/models/ecapa-speaker-v1.onnx";
const ECAPA_MODEL_URL: &str = "https://huggingface.co/vedk00/ecapa-voxceleb-speaker-embedding-onnx/resolve/main/model/ecapa-speaker-v1.onnx";
const ECAPA_SHA256: &str = "f46380bbaeddb929fb3a10ab63a4b1877a50e3d1e5fdd55a1b618d5651d3f64e";

fn main() {
    ensure_ecapa_model();
    println!("cargo:rerun-if-changed={ECAPA_MODEL_PATH}");
    println!("cargo:rerun-if-env-changed=SOUND_SPRING_SKIP_MODEL_DOWNLOAD");

    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let icons_cpp = compile_icons_qrc(&manifest_dir);
    println!("cargo:rerun-if-changed=resources/icons/icons.qrc");
    println!("cargo:rerun-if-changed=resources/icons/source.png");
    println!("cargo:rerun-if-changed=resources/icons/hicolor");

    CxxQtBuilder::new()
        .include_prefix("src/cpp")
        .qobject_header(QObjectHeaderOpts::from("src/cpp/key_forwarder.h"))
        .qobject_header(QObjectHeaderOpts::from("src/cpp/system_tray.h"))
        .cc_builder(move |builder| {
            builder
                .include("src/cpp")
                .file("src/cpp/key_forwarder.cpp")
                .file("src/cpp/app_identity.cpp")
                .file("src/cpp/system_tray.cpp")
                .file("src/cpp/app_bootstrap.cpp")
                .file(&icons_cpp);
        })
        .qt_module("Network")
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qt_module("QuickDialogs2")
        .qt_module("Widgets")
        .qml_module(QmlModule {
            uri: "io.github.benwhite1987.soundspring",
            rust_files: &[
                "src/qobjects/controller.rs",
                "src/qobjects/settings.rs",
                "src/qobjects/voice_controller.rs",
            ],
            qml_files: &[
                "qml/Main.qml",
                "qml/TabPage.qml",
                "qml/SoundButton.qml",
                "qml/ShortcutCapture.qml",
                "qml/SettingsDialog.qml",
                "qml/SoundSpringTheme.qml",
                "qml/AppButton.qml",
                "qml/SettingsSection.qml",
                "qml/VolumeBar.qml",
                "qml/VoicePanel.qml",
                "qml/Spectrum.qml",
                "qml/EnrollmentDialog.qml",
            ],
            ..Default::default()
        })
        .build();
}

fn compile_icons_qrc(manifest_dir: &Path) -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let qrc = manifest_dir.join("resources/icons/icons.qrc");
    let cpp = out_dir.join("qrc_icons.cpp");
    let rcc = find_qt_rcc();

    let status = std::process::Command::new(&rcc)
        .arg("-o")
        .arg(&cpp)
        .arg(&qrc)
        .status()
        .unwrap_or_else(|err| panic!("failed to run Qt rcc ({}): {err}", rcc.display()));

    if !status.success() {
        panic!("Qt rcc failed compiling {}", qrc.display());
    }

    cpp
}

fn find_qt_rcc() -> PathBuf {
    let mut qmake_candidates = Vec::new();
    if let Ok(qmake) = std::env::var("QMAKE") {
        qmake_candidates.push(qmake);
    }
    qmake_candidates.push("qmake6".into());

    for qmake in qmake_candidates {
        if let Ok(output) = std::process::Command::new(&qmake)
            .args(["-query", "QT_INSTALL_LIBS"])
            .output()
        {
            if output.status.success() {
                let libs = String::from_utf8_lossy(&output.stdout);
                let rcc = PathBuf::from(libs.trim()).join("rcc");
                if rcc.is_file() {
                    return rcc;
                }
            }
        }
    }

    for candidate in ["/usr/lib/qt6/rcc", "/usr/lib/qt6/bin/rcc"] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return path;
        }
    }

    panic!(
        "could not find Qt 6 rcc; install qt6-tools or set QMAKE to your Qt 6 qmake"
    );
}

fn ensure_ecapa_model() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let dest = manifest_dir.join(ECAPA_MODEL_PATH);

    if dest.is_file() {
        if sha256_file(&dest).is_ok_and(|hash| hash == ECAPA_SHA256) {
            return;
        }
        eprintln!(
            "cargo:warning=existing {ECAPA_MODEL_PATH} failed SHA-256 check; re-downloading"
        );
        let _ = fs::remove_file(&dest);
    }

    if std::env::var("SOUND_SPRING_SKIP_MODEL_DOWNLOAD").ok().as_deref() == Some("1") {
        panic!(
            "{ECAPA_MODEL_PATH} is missing and SOUND_SPRING_SKIP_MODEL_DOWNLOAD=1 is set.\n\
             Place the model at {dest:?} or unset the variable to download during build."
        );
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).expect("create assets/models");
    }

    eprintln!("cargo:warning=downloading ECAPA model (~80 MB) from HuggingFace");
    let mut response = ureq::get(ECAPA_MODEL_URL)
        .call()
        .unwrap_or_else(|err| panic!("failed to download ECAPA model from {ECAPA_MODEL_URL}: {err}"));
    if response.status() != 200 {
        panic!(
            "failed to download ECAPA model: HTTP {}",
            response.status()
        );
    }

    let mut file = File::create(&dest).expect("create ECAPA model file");
    std::io::copy(&mut response.body_mut().as_reader(), &mut file)
        .expect("write ECAPA model file");
    file.flush().expect("flush ECAPA model file");

    let hash = sha256_file(&dest).expect("hash downloaded ECAPA model");
    if hash != ECAPA_SHA256 {
        let _ = fs::remove_file(&dest);
        panic!(
            "downloaded ECAPA model SHA-256 mismatch (got {hash}, expected {ECAPA_SHA256})"
        );
    }
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
