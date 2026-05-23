import QtQuick
import QtQuick.Controls
import com.benkahn.soundboard

Item {
    id: root
    required property SoundboardController controller

    property int uiTick: 0
    property int columnSpacing: 8
    property int rowSpacing: 8
    readonly property real cellWidth: Math.max(0, (width - columnSpacing) / 2)
    readonly property real cellHeight: Math.max(88, (height - 4 * rowSpacing) / 5)

    Connections {
        target: controller
        function onPlayingStateChanged() {
            uiTick = uiTick + 1
        }
        function onCurrentTabChanged() {
            uiTick = uiTick + 1
        }
    }

    Repeater {
        model: 10
        delegate: SoundButton {
            x: (index % 2) * (root.cellWidth + root.columnSpacing)
            y: Math.floor(index / 2) * (root.cellHeight + root.rowSpacing)
            width: root.cellWidth
            height: root.cellHeight
            slotNumber: index < 9 ? index + 1 : 0
            label: (uiTick, controller.tabVersion, controller.slotLabel(slotNumber))
            shortcutLabel: (uiTick, controller.tabVersion, controller.shortcutVersion,
                            controller.slotShortcutLabel(slotNumber))
            empty: (uiTick, controller.tabVersion, controller.slotEmpty(slotNumber))
            playing: (uiTick, controller.playingVersion, controller.slotPlaying(slotNumber))
            progress: (uiTick, controller.progressVersion, controller.playingVersion,
                       controller.slotProgress(slotNumber))
            onClicked: controller.playSlot(slotNumber)
        }
    }
}
