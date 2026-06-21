#include "app_bootstrap.h"

#include <QApplication>
#include <QCoreApplication>
#include <cstdlib>

static QApplication* s_app = nullptr;

extern "C" void sound_spring_init_qt_application(int argc, char** argv)
{
    if (QCoreApplication::instance() != nullptr) {
        return;
    }
    // Fusion is themeable; Basic is flat grey and clashes with our custom surfaces.
    qputenv("QT_QUICK_CONTROLS_STYLE", "Fusion");
    s_app = new QApplication(argc, argv);
}

extern "C" int sound_spring_exec_qt_application()
{
    if (s_app == nullptr) {
        return 1;
    }
    return s_app->exec();
}
