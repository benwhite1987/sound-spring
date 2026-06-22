import QtQuick
import QtQuick.Controls
import io.github.benwhite1987.soundspring

Button {
    id: root
    required property int shortcutIndex
    required property var settings
    required property var captureHost

    property bool listening: captureHost && captureHost.activeCaptureIndex === shortcutIndex

    SoundSpringTheme {
        id: appTheme
    }

    focusPolicy: Qt.NoFocus
    implicitWidth: 0
    text: listening ? "Press a key…" : settings.shortcutDisplayAt(shortcutIndex)
    font.pointSize: 11
    palette.buttonText: appTheme.textPrimary

    contentItem: Text {
        text: root.text
        font: root.font
        color: root.palette.buttonText
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        width: parent.width
        leftPadding: 8
        rightPadding: 8
    }

    onClicked: {
        if (captureHost)
            captureHost.activeCaptureIndex = shortcutIndex
    }

    background: Rectangle {
        radius: 4
        color: {
            if (root.listening)
                return appTheme.surfaceActive
            if (root.down || root.hovered)
                return appTheme.surfaceHover
            return appTheme.surface
        }
        border.color: root.listening ? appTheme.borderAccent : appTheme.border
        border.width: 1
    }
}
