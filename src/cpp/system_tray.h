#ifndef SOUND_SPRING_SYSTEM_TRAY_H
#define SOUND_SPRING_SYSTEM_TRAY_H

#include <QObject>

class QAction;
class QMenu;
class QSystemTrayIcon;

class SystemTray : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool available READ available CONSTANT)
    Q_PROPERTY(bool visible READ visible WRITE setVisible NOTIFY visibleChanged)

public:
    static SystemTray* instance();

    bool available() const;
    bool visible() const;
    void setVisible(bool visible);

    Q_INVOKABLE void setIconThemeName(const QString& name);
    Q_INVOKABLE void setToolTip(const QString& tip);
    Q_INVOKABLE void setWindowVisible(bool windowVisible);
    Q_INVOKABLE void initialize();

signals:
    void visibleChanged();
    void showWindowRequested();
    void hideWindowRequested();
    void stopAllRequested();
    void quitRequested();

private:
    explicit SystemTray(QObject* parent = nullptr);
    void updateMenuState();

    QSystemTrayIcon* m_tray = nullptr;
    QMenu* m_menu = nullptr;
    QAction* m_showAction = nullptr;
    QAction* m_hideAction = nullptr;
    bool m_windowVisible = true;
};

extern "C" void sound_spring_register_system_tray();

#endif
