import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.benkahn.soundboard

Item {
    id: root
    required property SoundboardController controller

    property int uiTick: 0

    Connections {
        target: controller
        function onPlayingStateChanged() {
            uiTick = uiTick + 1
        }
    }

    GridLayout {
        anchors.fill: parent
        columns: 2
        rowSpacing: 8
        columnSpacing: 8

        Repeater {
            model: 10
            delegate: SoundButton {
                Layout.fillWidth: true
                Layout.minimumHeight: 80
                slotNumber: index < 9 ? index + 1 : 0
                label: (uiTick, controller.tabVersion, controller.slotLabel(slotNumber))
                empty: (uiTick, controller.tabVersion, controller.slotEmpty(slotNumber))
                playing: (uiTick, controller.playingVersion, controller.slotPlaying(slotNumber))
                onClicked: controller.playSlot(slotNumber)
            }
        }
    }
}
