import QtQuick
import QtQuick.Controls

Button {
    id: root
    property int slotNumber: 1
    property string label: ""
    property string shortcutLabel: ""
    property bool empty: true
    property bool playing: false
    property real progress: 0

    focusPolicy: Qt.NoFocus
    enabled: !empty
    font.pointSize: 12

    background: Rectangle {
        radius: 6
        color: root.down ? "#444" : "#333"
        border.color: root.enabled ? "#666" : "#444"

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: parent.width * Math.min(1, Math.max(0, root.progress))
            radius: parent.radius
            color: "#4caf50"
            clip: true
        }
    }

    contentItem: Column {
        spacing: 4
        anchors.centerIn: parent
        width: parent.width - 16

        Text {
            width: parent.width
            text: root.empty
                  ? ("Empty (slot " + (root.slotNumber === 0 ? 10 : root.slotNumber) + ")")
                  : root.label
            color: root.enabled ? "white" : "#888"
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            font.pointSize: root.font.pointSize
        }

        Text {
            width: parent.width
            visible: !root.empty && root.shortcutLabel.length > 0
            text: root.shortcutLabel
            color: root.playing ? "#e8f5e9" : "#aaa"
            horizontalAlignment: Text.AlignHCenter
            font.pointSize: root.font.pointSize - 2
        }
    }
}
