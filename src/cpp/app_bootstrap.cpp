#include "app_bootstrap.h"

#include "app_icons.h"

#include <QApplication>
#include <QColor>
#include <QCoreApplication>
#include <QPalette>
#include <QQuickStyle>

#include <cstdlib>
#include <string>
#include <vector>

static QApplication* s_app = nullptr;
static std::vector<std::string> s_arg_strings;
static std::vector<char*> s_argv_storage;
static int s_argc = 0;

static QPalette sound_spring_dark_palette()
{
    // Keep in sync with qml/SoundSpringTheme.qml.
    const QColor windowBg(0x1b, 0x1b, 0x1f);
    const QColor chromeBg(0x25, 0x25, 0x28);
    const QColor surface(0x33, 0x33, 0x38);
    const QColor textPrimary(0xec, 0xec, 0xec);
    const QColor textMuted(0x88, 0x88, 0x92);
    const QColor accent(0x6a, 0xbf, 0x69);
    const QColor mid(0x2c, 0x2c, 0x31);
    const QColor midlight(0x3a, 0x3a, 0x40);
    const QColor light(0x4a, 0x4a, 0x52);
    const QColor dark(0x12, 0x12, 0x15);
    const QColor shadow(0x00, 0x00, 0x00);
    const QColor brightText(0xff, 0xff, 0xff);

    QPalette palette;
    palette.setColor(QPalette::Window, windowBg);
    palette.setColor(QPalette::WindowText, textPrimary);
    palette.setColor(QPalette::Base, surface);
    palette.setColor(QPalette::AlternateBase, surface);
    palette.setColor(QPalette::Text, textPrimary);
    palette.setColor(QPalette::Button, surface);
    palette.setColor(QPalette::ButtonText, textPrimary);
    palette.setColor(QPalette::BrightText, brightText);
    palette.setColor(QPalette::Highlight, accent);
    palette.setColor(QPalette::HighlightedText, textPrimary);
    palette.setColor(QPalette::ToolTipBase, chromeBg);
    palette.setColor(QPalette::ToolTipText, textPrimary);
    palette.setColor(QPalette::PlaceholderText, textMuted);
    palette.setColor(QPalette::Link, accent);
    palette.setColor(QPalette::LinkVisited, accent);
    palette.setColor(QPalette::Light, light);
    palette.setColor(QPalette::Midlight, midlight);
    palette.setColor(QPalette::Mid, mid);
    palette.setColor(QPalette::Dark, dark);
    palette.setColor(QPalette::Shadow, shadow);
    return palette;
}

extern "C" void sound_spring_init_qt_application(int argc, char** argv)
{
    if (QCoreApplication::instance() != nullptr) {
        return;
    }
    // Fusion is themeable; Basic is flat grey and clashes with our custom surfaces.
    qputenv("QT_QUICK_CONTROLS_STYLE", "Fusion");
    QQuickStyle::setStyle(QStringLiteral("Fusion"));

    // Qt keeps argv for the application lifetime and may rewrite the pointer
    // table. Copy into static C++ storage instead of borrowing Rust/C strings.
    s_arg_strings.clear();
    s_argv_storage.clear();
    if (argc > 0 && argv != nullptr) {
        for (int i = 0; i < argc; ++i) {
            if (argv[i] == nullptr) {
                break;
            }
            s_arg_strings.emplace_back(argv[i]);
        }
    }
    if (s_arg_strings.empty()) {
        s_arg_strings.emplace_back("sound-spring");
    }
    s_argv_storage.reserve(s_arg_strings.size() + 1);
    for (std::string& arg : s_arg_strings) {
        s_argv_storage.push_back(arg.data());
    }
    s_argv_storage.push_back(nullptr);
    s_argc = static_cast<int>(s_argv_storage.size()) - 1;

    s_app = new QApplication(s_argc, s_argv_storage.data());
    s_app->setPalette(sound_spring_dark_palette());
    QApplication::setWindowIcon(sound_spring_application_icon());
}

extern "C" int sound_spring_exec_qt_application()
{
    if (s_app == nullptr) {
        return 1;
    }
    return s_app->exec();
}
