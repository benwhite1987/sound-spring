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

            Repeater {
                model: controller.tabCount
                delegate: ToolButton {
                    id: tabButton
                    focusPolicy: Qt.NoFocus
                    ButtonGroup.group: tabButtonGroup
                    checkable: true
                    text: {
                        controller.tabVersion
                        return controller.tabNameAt(index)
                    }
                    checked: {
                        controller.uiVersion
                        return controller.currentTabIndex === index
                    }
                    onClicked: controller.selectTab(index)
                    background: Rectangle {
                        implicitWidth: tabButton.implicitWidth + 12
                        implicitHeight: tabButton.implicitHeight + 6
                        radius: 4
                        color: tabButton.checked ? palette.highlight : "transparent"
                        opacity: tabButton.checked ? 0.35 : 0
                    }
                }
            }

            Item { Layout.fillWidth: true }

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
