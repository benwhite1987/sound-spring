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
    readonly property real cellWidth: Math.max(0, (width - columnSpacing) / 2)
    readonly property real cellHeight: Math.max(88, (height - 4 * rowSpacing) / 5)

    function maybeShowTabWarning() {
        if (controller.tabWarning.length > 0
                && controller.tabWarning !== lastShownWarning) {
            lastShownWarning = controller.tabWarning
            tabWarningDialog.open()
        }
    }

    function showSlotErrorIfNeeded(ok) {
        if (ok)
            return
        var msg = controller.lastError
        slotErrorDialog.text = msg.length > 0 ? msg : "Slot operation failed."
        slotErrorDialog.open()
    }

    property string lastShownWarning: ""

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
            progress: {
                // progressVersion notify drives the live fill; uiTick covers start/stop edges.
                uiTick
                controller.progressVersion
                return controller.slotProgress(slotNumber)
            }
            playing: {
                uiTick
                controller.playingVersion
                return controller.slotPlaying(slotNumber)
            }
            onClicked: {
                if (empty) {
                    root.pendingSlot = slotNumber
                    replaceFileDialog.title = "Add sound"
                    replaceFileDialog.open()
                } else {
                    controller.playSlot(slotNumber)
                }
            }
            onReplaceRequested: (slot) => {
                root.pendingSlot = slot
                replaceFileDialog.title = "Replace sound"
                replaceFileDialog.open()
            }
            onRenameRequested: (slot) => {
                root.pendingSlot = slot
                renameSlotField.text = controller.slotLabel(slot)
                renameSlotDialog.open()
            }
            onMoveRequested: (slot) => {
                root.moveFromSlot = slot
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
            root.showSlotErrorIfNeeded(
                        controller.replaceSlot(root.pendingSlot, decodeURIComponent(path)))
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
            root.showSlotErrorIfNeeded(
                        controller.renameSlot(root.pendingSlot, renameSlotField.text))
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
        width: Math.min(root.width - 80, 420)
        padding: 24
        standardButtons: Dialog.NoButton

        onRejected: root.moveFromSlot = -1

        ColumnLayout {
            spacing: 12
            width: moveSlotDialog.availableWidth

            Label {
                Layout.fillWidth: true
                text: root.moveFromSlot < 0 ? ""
                      : "Move from slot " + (root.moveFromSlot === 0 ? "10" : String(root.moveFromSlot))
                      + ". Tap a target slot (empty slots are fine)."
                wrapMode: Text.WordWrap
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                rowSpacing: 8
                columnSpacing: 8

                Repeater {
                    model: 10
                    delegate: AppButton {
                        Layout.fillWidth: true
                        readonly property int slotNum: index < 9 ? index + 1 : 0
                        text: slotNum === 0 ? "10" : String(slotNum)
                        enabled: root.moveFromSlot !== slotNum
                        role: "secondary"
                        onClicked: {
                            if (root.moveFromSlot < 0)
                                return
                            root.showSlotErrorIfNeeded(
                                        controller.moveSlot(root.moveFromSlot, slotNum))
                            root.moveFromSlot = -1
                            moveSlotDialog.close()
                        }
                    }
                }
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
            root.showSlotErrorIfNeeded(controller.removeSlot(root.pendingSlot))
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

    MessageDialog {
        id: slotErrorDialog
        title: "Slot operation failed"
        text: ""
        buttons: MessageDialog.Ok
    }
}
