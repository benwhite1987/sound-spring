import QtQuick
import QtQuick.Controls
import com.benkahn.soundboard

Button {
    id: root
    property int slotNumber: 1
    property string label: ""
    property string shortcutLabel: ""
    property string filePath: ""
    property bool empty: true
    property bool playing: false
    property real progress: 0
    property SoundboardController controller

    signal replaceRequested(int slot)
    signal renameRequested(int slot)
    signal moveRequested(int slot)
    signal removeRequested(int slot)

    focusPolicy: Qt.NoFocus
    opacity: root.empty ? 0.65 : 1.0
    font.pointSize: 13

    ToolTip.visible: !empty && hovered && filePath.length > 0
    ToolTip.text: filePath
    ToolTip.delay: 500

    background: Rectangle {
        id: bg
        radius: 6
        color: root.down ? "#444" : (root.playing ? "#3a4a3a" : "#333")
        border.color: root.playing ? "#6abf69" : (root.enabled ? "#666" : "#444")
        border.width: root.playing ? 2 : 1

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: parent.width * Math.min(1, Math.max(0, root.progress))
            radius: parent.radius
            color: "#4caf50"
            opacity: 0.55
            clip: true
        }
    }

  // Slot number badge
    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.margins: 6
        width: badgeText.implicitWidth + 10
        height: badgeText.implicitHeight + 4
        radius: 3
        color: "#222"
        border.color: "#555"
        z: 2

        Text {
            id: badgeText
            anchors.centerIn: parent
            text: root.slotNumber === 0 ? "10" : String(root.slotNumber)
            color: "#ccc"
            font.pointSize: root.font.pointSize - 3
            font.bold: true
        }
    }

    contentItem: Item {
        anchors.fill: parent

        Column {
            anchors.centerIn: parent
            width: parent.width - 20
            spacing: 4

            Text {
                width: parent.width
                text: root.empty
                      ? ("Empty (slot " + (root.slotNumber === 0 ? 10 : root.slotNumber) + ")")
                      : root.label
                color: root.enabled ? "white" : "#888"
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                maximumLineCount: 3
                elide: Text.ElideRight
                font.pointSize: root.font.pointSize
            }

            Text {
                width: parent.width
                visible: !root.empty && root.shortcutLabel.length > 0
                text: root.shortcutLabel
                color: root.playing ? "#c8e6c9" : "#aaa"
                horizontalAlignment: Text.AlignHCenter
                font.pointSize: root.font.pointSize - 2
            }
        }

        Text {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 8
            visible: root.playing
            text: "▶"
            color: "#a5d6a7"
            font.pointSize: root.font.pointSize
            z: 2
        }
    }

    Menu {
        id: slotMenu

        MenuItem {
            text: "Replace…"
            onTriggered: root.replaceRequested(root.slotNumber)
        }
        MenuItem {
            text: "Rename…"
            enabled: !root.empty
            onTriggered: root.renameRequested(root.slotNumber)
        }
        MenuItem {
            text: "Move to slot…"
            enabled: !root.empty
            onTriggered: root.moveRequested(root.slotNumber)
        }
        MenuItem {
            text: "Remove"
            enabled: !root.empty
            onTriggered: root.removeRequested(root.slotNumber)
        }
        MenuSeparator {}
        MenuItem {
            text: "Open Folder"
            onTriggered: root.controller.openTabFolder()
        }
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.RightButton
        z: 10
        onClicked: (mouse) => {
            if (mouse.button === Qt.RightButton)
                slotMenu.popup()
        }
    }
}
