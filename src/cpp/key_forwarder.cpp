#include "key_forwarder.h"

#include <QCoreApplication>
#include <QEvent>
#include <QGuiApplication>
#include <QKeyEvent>
#include <QtQml/qqml.h>

KeyForwarder* KeyForwarder::instance()
{
    static KeyForwarder* s_instance = nullptr;
    if (!s_instance && QCoreApplication::instance()) {
        s_instance = new KeyForwarder(QCoreApplication::instance());
    }
    return s_instance;
}

KeyForwarder::KeyForwarder(QObject* parent)
    : QObject(parent)
{
    if (auto* app = qobject_cast<QGuiApplication*>(QCoreApplication::instance())) {
        app->installEventFilter(this);
    }
}

bool KeyForwarder::eventFilter(QObject* watched, QEvent* event)
{
    Q_UNUSED(watched);
    if (event->type() != QEvent::KeyPress) {
        return false;
    }

    auto* keyEvent = static_cast<QKeyEvent*>(event);
    const int key = keyEvent->key();
    const unsigned int modifiers = static_cast<unsigned int>(keyEvent->modifiers());
    const unsigned int nativeScanCode = static_cast<unsigned int>(keyEvent->nativeScanCode());
    const bool isAutoRepeat = keyEvent->isAutoRepeat();

    emit keyPressed(key, modifiers, nativeScanCode, isAutoRepeat);
    return false;
}

static QObject* key_forwarder_provider(QQmlEngine*, QJSEngine*)
{
    return KeyForwarder::instance();
}

extern "C" void sound_spring_register_key_forwarder()
{
    qmlRegisterSingletonType<KeyForwarder>(
        "com.benkahn.soundboard",
        1,
        0,
        "KeyForwarder",
        key_forwarder_provider);
}
