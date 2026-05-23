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

static void ensure_window_handle(QWindow* window)
{
    if (window && !window->handle()) {
        window->create();
    }
}

static QWindow* main_portal_window()
{
    QWindow* fallback = nullptr;
    for (QWindow* window : QGuiApplication::topLevelWindows()) {
        if (!window) {
            continue;
        }
        ensure_window_handle(window);
        const QString title = window->title();
        if (title.contains(QStringLiteral("Settings"), Qt::CaseInsensitive)) {
            continue;
        }
        if (title.contains(QStringLiteral("Sound Spring"), Qt::CaseInsensitive)) {
            return window;
        }
        if (!fallback) {
            fallback = window;
        }
    }
    if (fallback) {
        return fallback;
    }
    QWindow* focused = QGuiApplication::focusWindow();
    if (focused) {
        ensure_window_handle(focused);
        return focused;
    }
    return nullptr;
}

static QString portal_handle_for_window(QWindow* window)
{
    if (!window) {
        return QString();
    }
    ensure_window_handle(window);

    // On Wayland we intentionally send an empty parent_window. Qt's
    // QWaylandShellSurface::externWindowHandle() returns Qt's internal surface
    // identifier, not an xdg_foreign-exported handle, so xdg-desktop-portal-kde
    // cannot resolve it and silently dismisses the BindShortcuts dialog. An
    // empty handle causes portal-kde to show the dialog unparented, which works.
    if (QGuiApplication::platformName().contains(QStringLiteral("wayland"), Qt::CaseInsensitive)) {
        return QString();
    }

    if (window->winId() != 0) {
        return QStringLiteral("x11:0x") + QString::number(window->winId(), 16);
    }
    return QString();
}

extern "C" void sound_spring_init_app_identity()
{
    QGuiApplication::setApplicationDisplayName(QStringLiteral("Sound Spring"));
    QGuiApplication::setDesktopFileName(QStringLiteral("sound-spring"));
}

extern "C" void sound_spring_refresh_portal_parent_window()
{
    const QString handle = portal_handle_for_window(main_portal_window());
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
