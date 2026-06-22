import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import io.github.benwhite1987.soundspring

Item {
    id: root
    required property SoundboardController controller

    SoundSpringTheme {
        id: appTheme
    }

    property int uiTick: 0
    property int columnSpacing: 8
    property int rowSpacing: 8
    property int pendingSlot: -1
    property int moveFromSlot: -1
    property string lastShownWarning: ""
    readonly property real cellWidth: Math.max(0, (width - columnSpacing) / 2)
    readonly property real cellHeight: Math.max(88, (height - 4 * rowSpacing) / 5)

    function maybeShowTabWarning() {
        if (controller.tabWarning.length > 0
                && controller.tabWarning !== lastShownWarning) {
            lastShownWarning = controller.tabWarning
            tabWarningDialog.open()
        }
    }

    Component.onCompleted: maybeShowTabWarning()

    Connections {
        target: controller
        function onPlayingStateChanged() {
            uiTick = uiTick + 1
        }
        function onCurrentTabChanged() {
            uiTick = uiTick + 1
        }
        function onTabsChanged() {
            uiTick = uiTick + 1
            root.maybeShowTabWarning()
        }
    }

    Repeater {
        model: 10
        delegate: SoundButton {
            x: (index % 2) * (root.cellWidth + root.columnSpacing)
            y: Math.floor(index / 2) * (root.cellHeight + root.rowSpacing)
            width: root.cellWidth
            height: root.cellHeight
            controller: root.controller
            slotNumber: index < 9 ? index + 1 : 0
            label: {
                uiTick
                controller.tabVersion
                return controller.slotLabel(slotNumber)
            }
            shortcutLabel: {
                uiTick
                controller.tabVersion
                controller.shortcutVersion
                return controller.slotShortcutLabel(slotNumber)
            }
            filePath: {
                uiTick
                controller.tabVersion
                return controller.slotPathAt(slotNumber)
            }
            empty: {
                uiTick
                controller.tabVersion
                return controller.slotEmpty(slotNumber)
            }
            playing: {
                uiTick
                controller.playingVersion
                return controller.slotPlaying(slotNumber)
            }
            progress: {
                uiTick
                controller.progressVersion
                controller.playingVersion
                return controller.slotProgress(slotNumber)
            }
            onClicked: {
                if (!empty)
                    controller.playSlot(slotNumber)
            }
            onReplaceRequested: (slot) => {
                root.pendingSlot = slot
                replaceFileDialog.open()
            }
            onRenameRequested: (slot) => {
                root.pendingSlot = slot
                renameSlotField.text = controller.slotLabel(slot)
                renameSlotDialog.open()
            }
            onMoveRequested: (slot) => {
                root.moveFromSlot = slot
                moveTargetField.text = ""
                moveSlotDialog.open()
            }
            onRemoveRequested: (slot) => {
                root.pendingSlot = slot
                removeSlotDialog.open()
            }
        }
    }

    FileDialog {
        id: replaceFileDialog
        title: "Replace sound"
        fileMode: FileDialog.OpenFile
        nameFilters: ["Audio (*.ogg *.oga *.opus *.wav *.flac *.mp3 *.m4a *.aac)"]
        onAccepted: {
            if (root.pendingSlot < 0)
                return
            var path = selectedFile.toString()
            if (path.startsWith("file://"))
                path = path.substring(7)
            controller.replaceSlot(root.pendingSlot, decodeURIComponent(path))
            root.pendingSlot = -1
        }
        onRejected: root.pendingSlot = -1
    }

    Dialog {
        id: renameSlotDialog
        title: "Rename sound"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 360)
        padding: 24
        standardButtons: Dialog.NoButton

        onAccepted: {
            if (root.pendingSlot < 0)
                return
            controller.renameSlot(root.pendingSlot, renameSlotField.text)
            root.pendingSlot = -1
        }
        onRejected: root.pendingSlot = -1
        onOpened: {
            renameSlotField.forceActiveFocus()
            renameSlotField.selectAll()
        }

        ColumnLayout {
            spacing: 12
            width: renameSlotDialog.availableWidth

            Label {
                Layout.fillWidth: true
                text: "Display filename (without extension)"
            }
            TextField {
                id: renameSlotField
                Layout.fillWidth: true
                onAccepted: renameSlotDialog.accept()
            }
        }

        footer: RowLayout {
            spacing: 8
            width: renameSlotDialog.availableWidth
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Cancel"
                onClicked: renameSlotDialog.reject()
            }
            AppButton {
                text: "OK"
                role: "primary"
                onClicked: renameSlotDialog.accept()
            }
        }
    }

    Dialog {
        id: moveSlotDialog
        title: "Move sound to slot"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 360)
        padding: 24
        standardButtons: Dialog.NoButton

        onAccepted: {
            if (root.moveFromSlot < 0)
                return
            var text = moveTargetField.text.trim()
            var target = parseInt(text, 10)
            if (isNaN(target) || target < 1 || target > 10) {
                moveSlotDialog.reject()
                return
            }
            var toSlot = target === 10 ? 0 : target
            controller.moveSlot(root.moveFromSlot, toSlot)
            root.moveFromSlot = -1
        }
        onRejected: root.moveFromSlot = -1

        ColumnLayout {
            spacing: 12
            width: moveSlotDialog.availableWidth

            Label {
                Layout.fillWidth: true
                text: root.moveFromSlot < 0 ? ""
                      : "Move from slot " + (root.moveFromSlot === 0 ? "10" : String(root.moveFromSlot))
            }
            Label {
                Layout.fillWidth: true
                text: "Target slot (1–10)"
            }
            TextField {
                id: moveTargetField
                Layout.fillWidth: true
                inputMethodHints: Qt.ImhDigitsOnly
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: appTheme.textMuted
                text: "Enter 1–10; empty slots are valid targets."
            }
        }

        footer: RowLayout {
            spacing: 8
            width: moveSlotDialog.availableWidth
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Cancel"
                onClicked: moveSlotDialog.reject()
            }
            AppButton {
                text: "OK"
                role: "primary"
                onClicked: moveSlotDialog.accept()
            }
        }
    }

    Dialog {
        id: removeSlotDialog
        title: "Remove sound"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 360)
        padding: 24
        standardButtons: Dialog.NoButton

        onAccepted: {
            if (root.pendingSlot < 0)
                return
            controller.removeSlot(root.pendingSlot)
            root.pendingSlot = -1
        }
        onRejected: root.pendingSlot = -1

        Label {
            width: removeSlotDialog.availableWidth
            wrapMode: Text.WordWrap
            text: "Are you sure? File will be permanently deleted."
        }

        footer: RowLayout {
            spacing: 8
            width: removeSlotDialog.availableWidth
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Cancel"
                onClicked: removeSlotDialog.reject()
            }
            AppButton {
                text: "Remove"
                role: "danger"
                onClicked: removeSlotDialog.accept()
            }
        }
    }

    MessageDialog {
        id: tabWarningDialog
        title: "Tab folder notice"
        text: controller.tabWarning
        buttons: MessageDialog.Ok
    }
}
