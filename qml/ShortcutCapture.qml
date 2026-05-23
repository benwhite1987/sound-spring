import QtQuick
import QtQuick.Controls

Button {
    id: root
    required property int shortcutIndex
    required property var settings
    required property var captureHost

    property bool listening: captureHost && captureHost.activeCaptureIndex === shortcutIndex

    focusPolicy: Qt.NoFocus
    text: listening ? "Press a key…" : settings.shortcutDisplayAt(shortcutIndex)
    font.pointSize: 11

    onClicked: {
        if (captureHost)
            captureHost.activeCaptureIndex = shortcutIndex
    }

    background: Rectangle {
        radius: 4
        color: root.listening ? "#3a4a3a" : (root.down ? "#444" : "#333")
        border.color: root.listening ? "#4caf50" : "#666"
    }
}
