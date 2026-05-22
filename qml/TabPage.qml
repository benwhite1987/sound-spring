import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.benkahn.soundboard

Item {
    id: root
    required property SoundboardController controller

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
                label: controller.slotLabel(slotNumber)
                empty: controller.slotEmpty(slotNumber)
                playing: controller.slotPlaying(slotNumber)
                onClicked: controller.playSlot(slotNumber)
            }
        }
    }
}
