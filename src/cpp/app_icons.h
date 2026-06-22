#ifndef SOUND_SPRING_APP_ICONS_H
#define SOUND_SPRING_APP_ICONS_H

#include <QIcon>
#include <QString>

inline QString sound_spring_embedded_icon_path()
{
    return QStringLiteral(":/icons/hicolor/256x256/apps/io.github.benwhite1987.SoundSpring.png");
}

inline QIcon sound_spring_application_icon()
{
    const QIcon embedded(sound_spring_embedded_icon_path());
    if (!embedded.isNull()) {
        return embedded;
    }
    const QIcon themed = QIcon::fromTheme(QStringLiteral("io.github.benwhite1987.SoundSpring"));
    if (!themed.isNull()) {
        return themed;
    }
    return QIcon::fromTheme(QStringLiteral("audio-volume-high"));
}

#endif
