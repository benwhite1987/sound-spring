#include <QByteArray>
#include <QGuiApplication>
#include <QMutex>
#include <QMutexLocker>
#include <QString>
#include <QWindow>

#include <algorithm>
#include <cstring>

static QMutex s_parent_mutex;
static QString s_parent_window;

extern "C" void sound_spring_init_app_identity()
{
    QGuiApplication::setApplicationDisplayName(QStringLiteral("Sound Spring"));
    QGuiApplication::setDesktopFileName(QStringLiteral("sound-spring"));
}

extern "C" void sound_spring_refresh_portal_parent_window()
{
    QWindow* window = QGuiApplication::focusWindow();
    if (!window) {
        const auto windows = QGuiApplication::topLevelWindows();
        if (!windows.isEmpty()) {
            window = windows.first();
        }
    }

    QString handle;
    if (window && window->winId() != 0) {
        handle = QStringLiteral("x11:0x") + QString::number(window->winId(), 16);
    }

    QMutexLocker lock(&s_parent_mutex);
    s_parent_window = handle;
}

extern "C" void sound_spring_portal_parent_window(char* out, size_t out_len)
{
    if (!out || out_len == 0) {
        return;
    }
    QMutexLocker lock(&s_parent_mutex);
    const QByteArray utf8 = s_parent_window.toUtf8();
    const size_t n = std::min(static_cast<size_t>(utf8.size()), out_len - 1);
    if (n > 0) {
        std::memcpy(out, utf8.constData(), n);
    }
    out[n] = '\0';
}
