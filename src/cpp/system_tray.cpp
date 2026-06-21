#include "system_tray.h"

#include <QAction>
#include <QCoreApplication>
#include <QIcon>
#include <QMenu>
#include <QtWidgets/QSystemTrayIcon>
#include <QtQml/qqml.h>

SystemTray* SystemTray::instance()
{
    static SystemTray* s_instance = nullptr;
    if (!s_instance && QCoreApplication::instance()) {
        s_instance = new SystemTray(QCoreApplication::instance());
    }
    return s_instance;
}

SystemTray::SystemTray(QObject* parent)
    : QObject(parent)
{
}

void SystemTray::initialize()
{
    if (m_tray != nullptr || !QSystemTrayIcon::isSystemTrayAvailable()) {
        return;
    }

    m_tray = new QSystemTrayIcon(this);
    m_menu = new QMenu();

    m_showAction = m_menu->addAction(QStringLiteral("Show"));
    m_hideAction = m_menu->addAction(QStringLiteral("Hide"));
    m_menu->addSeparator();
    connect(m_menu->addAction(QStringLiteral("Stop All")), &QAction::triggered, this, [this]() {
        emit stopAllRequested();
    });
    connect(m_menu->addAction(QStringLiteral("Quit")), &QAction::triggered, this, [this]() {
        emit quitRequested();
    });

    connect(m_showAction, &QAction::triggered, this, [this]() {
        emit showWindowRequested();
    });
    connect(m_hideAction, &QAction::triggered, this, [this]() {
        emit hideWindowRequested();
    });

    connect(m_tray, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::Trigger || reason == QSystemTrayIcon::DoubleClick) {
            emit showWindowRequested();
        }
    });

    m_tray->setContextMenu(m_menu);
    updateMenuState();
}

bool SystemTray::available() const
{
    return QSystemTrayIcon::isSystemTrayAvailable();
}

bool SystemTray::visible() const
{
    return m_tray && m_tray->isVisible();
}

void SystemTray::setVisible(bool visible)
{
    if (!m_tray) {
        return;
    }
    if (m_tray->isVisible() == visible) {
        return;
    }
    m_tray->setVisible(visible);
    emit visibleChanged();
}

void SystemTray::setIconThemeName(const QString& name)
{
    if (!m_tray) {
        return;
    }
    const QIcon icon = QIcon::fromTheme(name);
    if (!icon.isNull()) {
        m_tray->setIcon(icon);
    }
}

void SystemTray::setToolTip(const QString& tip)
{
    if (m_tray) {
        m_tray->setToolTip(tip);
    }
}

void SystemTray::setWindowVisible(bool windowVisible)
{
    if (m_windowVisible == windowVisible) {
        return;
    }
    m_windowVisible = windowVisible;
    updateMenuState();
}

void SystemTray::updateMenuState()
{
    if (m_showAction) {
        m_showAction->setEnabled(!m_windowVisible);
    }
    if (m_hideAction) {
        m_hideAction->setEnabled(m_windowVisible);
    }
}

static QObject* system_tray_provider(QQmlEngine*, QJSEngine*)
{
    return SystemTray::instance();
}

extern "C" void sound_spring_register_system_tray()
{
    qmlRegisterSingletonType<SystemTray>(
        "com.benkahn.soundboard",
        1,
        0,
        "SystemTray",
        system_tray_provider);
}
