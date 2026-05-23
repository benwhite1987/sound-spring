import QtQuick
import QtQuick.Controls
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
            }

            Repeater {
                model: controller.tabCount
                delegate: ToolButton {
                    focusPolicy: Qt.NoFocus
                    ButtonGroup.group: tabButtonGroup
                    text: {
                        controller.tabVersion
                        return controller.tabNameAt(index)
                    }
                    checkable: true
                    checked: controller.currentTabIndex === index
                    onClicked: controller.selectTab(index)
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
                icon.width: 20
                icon.height: 20
                icon.name: controller.outputMuted ? "audio-volume-muted" : "audio-volume-high"
                opacity: controller.outputMuted ? 0.45 : 1.0
                ToolTip.visible: hovered
                ToolTip.text: controller.outputMuted ? "Output muted — click to unmute" : "Output unmuted — click to mute"
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
                enabled: !controller.outputMuted
                opacity: controller.outputMuted ? 0.4 : 1.0
                onMoved: controller.updateOutputVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    controller.updateOutputVolume(Math.round(value))
            }
            Label {
                Layout.preferredWidth: 36
                horizontalAlignment: Text.AlignRight
                text: Math.round(outVolumeSlider.value) + "%"
                color: controller.outputMuted ? "#666" : "#ccc"
                opacity: controller.outputMuted ? 0.4 : 1.0
            }

            Label { text: "Mon"; color: "#aaa" }
            ToolButton {
                id: monMuteButton
                focusPolicy: Qt.NoFocus
                icon.width: 20
                icon.height: 20
                icon.name: controller.monitorMuted ? "audio-volume-muted" : "audio-headphones"
                opacity: controller.monitorMuted ? 0.45 : 1.0
                ToolTip.visible: hovered
                ToolTip.text: controller.monitorMuted ? "Monitor muted — click to unmute" : "Monitor unmuted — click to mute"
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
                enabled: !controller.monitorMuted
                opacity: controller.monitorMuted ? 0.4 : 1.0
                onMoved: controller.updateMonitorVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    controller.updateMonitorVolume(Math.round(value))
            }
            Label {
                Layout.preferredWidth: 36
                horizontalAlignment: Text.AlignRight
                text: Math.round(monVolumeSlider.value) + "%"
                color: controller.monitorMuted ? "#666" : "#ccc"
                opacity: controller.monitorMuted ? 0.4 : 1.0
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
}
