import QtQuick
import QtQuick.Controls

Button {
    id: root
    property int slotNumber: 1
    property string label: ""
    property bool empty: true
    property bool playing: false

    enabled: !empty
    text: empty ? ("Empty (slot " + (slotNumber === 0 ? 10 : slotNumber) + ")") : label
    font.pointSize: 12

    background: Rectangle {
        radius: 6
        color: root.playing ? "#4caf50" : (root.down ? "#444" : "#333")
        border.color: root.enabled ? "#666" : "#444"
    }

    contentItem: Text {
        text: root.text
        color: root.enabled ? "white" : "#888"
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        wrapMode: Text.WordWrap
    }
}
