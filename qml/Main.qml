import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import com.benkahn.soundboard

ApplicationWindow {
    id: root
    width: 800
    height: 600
    visible: true
    title: "Sound Spring"

    onActiveChanged: controller.setWindowActive(active && visible)
    onVisibilityChanged: controller.setWindowActive(active && visible)

    SoundboardController {
        id: controller
    }

    Settings {
        id: settings
    }

    Connections {
        target: KeyForwarder
        function onKeyPressed(key, modifiers, nativeScanCode, isAutoRepeat) {
            if (isAutoRepeat)
                return
            if (settingsDialog.visible) {
                settingsDialog.handleKey(key, modifiers, nativeScanCode)
            } else {
                controller.handleKeyEvent(key, modifiers, nativeScanCode)
            }
        }
    }

    Timer {
        interval: 50
        running: true
        repeat: true
        onTriggered: controller.processEvents()
    }

    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            anchors.margins: 6
            spacing: 8

            ButtonGroup {
                id: tabButtonGroup
                exclusive: true
            }

            ListView {
                id: tabList
                Layout.fillWidth: true
                Layout.preferredHeight: 40
                orientation: ListView.Horizontal
                spacing: 4
                clip: true
                property int tabStripTick: 0
                model: {
                    tabStripTick
                    controller.uiVersion
                    return controller.tabCount
                }

                Connections {
                    target: controller
                    function onTabsChanged() {
                        tabList.tabStripTick++
                        tabList.forceLayout()
                    }
                    function onCurrentTabChanged() {
                        tabList.tabStripTick++
                    }
                }

                property int dragIndex: -1

                delegate: Item {
                    id: tabDelegate
                    height: tabList.height
                    width: {
                        tabList.tabStripTick
                        controller.uiVersion
                        return Math.max(tabButton.implicitWidth + 12, 48)
                    }

                    ToolButton {
                        id: tabButton
                        anchors.fill: parent
                        focusPolicy: Qt.NoFocus
                        ButtonGroup.group: tabButtonGroup
                        checkable: true
                        text: (tabList.tabStripTick, controller.uiVersion, controller.tabVersion,
                               controller.tabNameAt(index))
                        checked: {
                            controller.uiVersion
                            return controller.currentTabIndex === index
                        }
                        background: Rectangle {
                            implicitWidth: tabButton.implicitWidth + 12
                            implicitHeight: tabButton.implicitHeight + 6
                            radius: 4
                            color: tabButton.checked ? palette.highlight : "transparent"
                            opacity: tabButton.checked ? 0.35 : 0
                        }
                    }

                    MouseArea {
                        id: tabMouse
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        propagateComposedEvents: true

                        property real pressX: 0
                        property bool dragging: false

                        onPressed: (mouse) => {
                            if (mouse.button === Qt.RightButton) {
                                tabContextMenu.tabIndex = index
                                tabContextMenu.popup()
                                mouse.accepted = true
                                return
                            }
                            pressX = mouse.x
                            dragging = false
                            tabList.dragIndex = index
                            mouse.accepted = true
                        }

                        onPositionChanged: (mouse) => {
                            if (!pressed || mouse.buttons !== Qt.LeftButton)
                                return
                            if (!dragging && Math.abs(mouse.x - pressX) > 10) {
                                dragging = true
                                dragHandle.visible = true
                            }
                            if (dragging)
                                dragHandle.x = Math.max(0, Math.min(
                                    tabDelegate.width - dragHandle.width,
                                    mouse.x - dragHandle.width / 2))
                        }

                        onReleased: (mouse) => {
                            if (dragging) {
                                var dropX = tabList.contentX + tabDelegate.x + dragHandle.x + dragHandle.width / 2
                                var dropIdx = tabList.indexAt(dropX, tabDelegate.height / 2)
                                if (dropIdx < 0)
                                    dropIdx = tabList.dragIndex
                                if (dropIdx !== tabList.dragIndex)
                                    controller.moveTab(tabList.dragIndex, dropIdx)
                            } else if (mouse.button === Qt.LeftButton) {
                                controller.selectTab(index)
                            }
                            dragging = false
                            dragHandle.visible = false
                            dragHandle.x = 0
                            tabList.dragIndex = -1
                        }

                        Rectangle {
                            id: dragHandle
                            visible: false
                            width: parent.width
                            height: parent.height - 4
                            anchors.verticalCenter: parent.verticalCenter
                            radius: 4
                            color: palette.highlight
                            opacity: 0.45
                        }
                    }
                }
            }

            ToolButton {
                focusPolicy: Qt.NoFocus
                text: "+"
                ToolTip.visible: hovered
                ToolTip.text: "Add tab"
                onClicked: {
                    addTabNameField.text = ""
                    addTabDialog.existingPath = ""
                    addTabDialog.open()
                }
            }

            ToolButton {
                focusPolicy: Qt.NoFocus
                text: "◀"
                onClicked: controller.prevTab()
            }
            ToolButton {
                focusPolicy: Qt.NoFocus
                text: "▶"
                onClicked: controller.nextTab()
            }
            ToolButton {
                focusPolicy: Qt.NoFocus
                text: "⚙"
                onClicked: settingsDialog.openSettings()
            }
        }
    }

    TabPage {
        anchors.fill: parent
        anchors.margins: 12
        controller: controller
    }

    footer: ToolBar {
        RowLayout {
            anchors.fill: parent
            anchors.margins: 6
            spacing: 10

            Label { text: "Out"; color: "#aaa" }
            ToolButton {
                id: outMuteButton
                focusPolicy: Qt.NoFocus
                display: AbstractButton.TextBesideIcon
                icon.width: 20
                icon.height: 20
                icon.name: {
                    controller.uiVersion
                    return controller.outputMuted ? "audio-volume-muted" : "audio-volume-high"
                }
                text: {
                    controller.uiVersion
                    return controller.outputMuted ? "Muted" : ""
                }
                opacity: {
                    controller.uiVersion
                    return controller.outputMuted ? 0.45 : 1.0
                }
                ToolTip.visible: hovered
                ToolTip.text: {
                    controller.uiVersion
                    return controller.outputMuted
                           ? "Output muted — click to unmute"
                           : "Output unmuted — click to mute"
                }
                onClicked: controller.toggleOutputMute()
            }
            Slider {
                id: outVolumeSlider
                focusPolicy: Qt.NoFocus
                Layout.preferredWidth: 100
                from: 0
                to: 100
                value: controller.outputVolume
                live: true
                enabled: {
                    controller.uiVersion
                    return !controller.outputMuted
                }
                opacity: {
                    controller.uiVersion
                    return controller.outputMuted ? 0.4 : 1.0
                }
                onMoved: controller.updateOutputVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    controller.updateOutputVolume(Math.round(value))
            }
            Label {
                Layout.preferredWidth: 36
                horizontalAlignment: Text.AlignRight
                text: Math.round(outVolumeSlider.value) + "%"
                color: {
                    controller.uiVersion
                    return controller.outputMuted ? "#666" : "#ccc"
                }
                opacity: {
                    controller.uiVersion
                    return controller.outputMuted ? 0.4 : 1.0
                }
            }

            Label { text: "Mon"; color: "#aaa" }
            ToolButton {
                id: monMuteButton
                focusPolicy: Qt.NoFocus
                display: AbstractButton.TextBesideIcon
                icon.width: 20
                icon.height: 20
                icon.name: {
                    controller.uiVersion
                    return controller.monitorMuted ? "audio-volume-muted" : "audio-headphones"
                }
                text: {
                    controller.uiVersion
                    return controller.monitorMuted ? "Muted" : ""
                }
                opacity: {
                    controller.uiVersion
                    return controller.monitorMuted ? 0.45 : 1.0
                }
                ToolTip.visible: hovered
                ToolTip.text: {
                    controller.uiVersion
                    return controller.monitorMuted
                           ? "Monitor muted — click to unmute"
                           : "Monitor unmuted — click to mute"
                }
                onClicked: controller.toggleMonitorMute()
            }
            Slider {
                id: monVolumeSlider
                focusPolicy: Qt.NoFocus
                Layout.preferredWidth: 100
                from: 0
                to: 100
                value: controller.monitorVolume
                live: true
                enabled: {
                    controller.uiVersion
                    return !controller.monitorMuted
                }
                opacity: {
                    controller.uiVersion
                    return controller.monitorMuted ? 0.4 : 1.0
                }
                onMoved: controller.updateMonitorVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    controller.updateMonitorVolume(Math.round(value))
            }
            Label {
                Layout.preferredWidth: 36
                horizontalAlignment: Text.AlignRight
                text: Math.round(monVolumeSlider.value) + "%"
                color: {
                    controller.uiVersion
                    return controller.monitorMuted ? "#666" : "#ccc"
                }
                opacity: {
                    controller.uiVersion
                    return controller.monitorMuted ? 0.4 : 1.0
                }
            }

            Item { Layout.fillWidth: true }

            Button {
                focusPolicy: Qt.NoFocus
                text: "Stop All"
                onClicked: controller.stopAll()
            }
        }
    }

    SettingsDialog {
        id: settingsDialog
        controller: controller
        settings: settings
        ownerWindow: root
    }

    Menu {
        id: tabContextMenu
        property int tabIndex: -1

        MenuItem {
            text: "Rename…"
            onTriggered: {
                renameTabDialog.tabIndex = tabContextMenu.tabIndex
                renameTabNameField.text = controller.tabNameAt(tabContextMenu.tabIndex)
                renameTabDialog.open()
            }
        }
        MenuItem {
            text: "Remove"
            enabled: controller.tabUsesCustomList
            onTriggered: controller.removeTab(tabContextMenu.tabIndex)
        }
        MenuSeparator {}
        MenuItem {
            text: "Move Left"
            enabled: tabContextMenu.tabIndex > 0
            onTriggered: controller.moveTab(tabContextMenu.tabIndex, tabContextMenu.tabIndex - 1)
        }
        MenuItem {
            text: "Move Right"
            enabled: tabContextMenu.tabIndex >= 0
                    && tabContextMenu.tabIndex < controller.tabCount - 1
            onTriggered: controller.moveTab(tabContextMenu.tabIndex, tabContextMenu.tabIndex + 1)
        }
    }

    Dialog {
        id: addTabDialog
        title: "Add Tab"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 420)
        standardButtons: Dialog.Ok | Dialog.Cancel

        property string existingPath: ""

        onAboutToShow: {
            addTabNameField.text = ""
            existingPath = ""
        }

        onAccepted: {
            controller.addTab(addTabDialog.existingPath, addTabNameField.text)
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            Label { text: "Tab name" }
            TextField {
                id: addTabNameField
                Layout.fillWidth: true
                placeholderText: "New Tab"
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: "#aaa"
                text: addTabDialog.existingPath.length > 0
                      ? addTabDialog.existingPath
                      : "Creates a new folder under the tabs root."
            }
            Button {
                text: "Choose existing folder…"
                onClicked: addTabFolderDialog.open()
            }
        }
    }

    FolderDialog {
        id: addTabFolderDialog
        title: "Choose tab folder"
        onAccepted: {
            var path = selectedFolder.toString()
            if (path.startsWith("file://"))
                path = path.substring(7)
            addTabDialog.existingPath = decodeURIComponent(path)
        }
    }

    Dialog {
        id: renameTabDialog
        title: "Rename Tab"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 360)
        standardButtons: Dialog.Ok | Dialog.Cancel

        property int tabIndex: -1

        onAccepted: controller.renameTab(renameTabDialog.tabIndex, renameTabNameField.text)

        ColumnLayout {
            anchors.fill: parent
            spacing: 10
            Label { text: "Display name" }
            TextField {
                id: renameTabNameField
                Layout.fillWidth: true
            }
        }
    }

    QtObject {
        id: shortcutPromptGuard
        property bool shown: false
    }

    Connections {
        target: controller
        function onGlobalShortcutsStatusChanged() {
            if (shortcutPromptGuard.shown)
                return
            if (controller.needsGlobalShortcutApply()) {
                shortcutPromptGuard.shown = true
                globalShortcutPrompt.open()
            }
        }
    }

    MessageDialog {
        id: globalShortcutPrompt
        title: "Register global shortcuts"
        text: "KDE could not register global shortcuts for Sound Spring. " +
              "Open Settings → Shortcuts and click Apply, or relaunch Sound Spring " +
              "from the application launcher (not from inside an IDE terminal)."
        buttons: MessageDialog.Ok | MessageDialog.Cancel
        onAccepted: settingsDialog.openSettings()
        onRejected: controller.dismissGlobalShortcutsPrompt()
    }
}
