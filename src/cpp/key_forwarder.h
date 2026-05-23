#ifndef SOUND_SPRING_KEY_FORWARDER_H
#define SOUND_SPRING_KEY_FORWARDER_H

#include <QObject>

class KeyForwarder : public QObject {
    Q_OBJECT

public:
    static KeyForwarder* instance();
    explicit KeyForwarder(QObject* parent = nullptr);
    bool eventFilter(QObject* watched, QEvent* event) override;

signals:
    void keyPressed(int key, unsigned int modifiers, unsigned int nativeScanCode, bool isAutoRepeat);
};

extern "C" void sound_spring_register_key_forwarder();

#endif
