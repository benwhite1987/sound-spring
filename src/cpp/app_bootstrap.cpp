#include "app_bootstrap.h"

#include "app_icons.h"

#include <QApplication>
#include <QCoreApplication>
#include <cstdlib>
#include <string>
#include <vector>

static QApplication* s_app = nullptr;
static std::vector<std::string> s_arg_strings;
static std::vector<char*> s_argv_storage;
static int s_argc = 0;

extern "C" void sound_spring_init_qt_application(int argc, char** argv)
{
    if (QCoreApplication::instance() != nullptr) {
        return;
    }
    // Fusion is themeable; Basic is flat grey and clashes with our custom surfaces.
    qputenv("QT_QUICK_CONTROLS_STYLE", "Fusion");

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
    QApplication::setWindowIcon(sound_spring_application_icon());
}

extern "C" int sound_spring_exec_qt_application()
{
    if (s_app == nullptr) {
        return 1;
    }
    return s_app->exec();
}
