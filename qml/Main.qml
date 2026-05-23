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

            Repeater {
                model: controller.tabCount
                delegate: ToolButton {
                    focusPolicy: Qt.NoFocus
                    text: (controller.tabVersion, controller.tabNameAt(index))
                    checkable: true
                    checked: (controller.tabVersion, controller.currentTabIndex === index)
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
                onClicked: {
                    settings.loadFromConfig()
                    controller.refreshMicSources()
                    settingsDialog.open()
                }
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
                focusPolicy: Qt.NoFocus
                display: AbstractButton.IconOnly
                text: controller.outputMuted ? "⊘" : "🔊"
                font.pointSize: controller.outputMuted ? 16 : 12
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
                onMoved: controller.updateOutputVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    controller.updateOutputVolume(Math.round(value))
            }
            Label {
                Layout.preferredWidth: 36
                horizontalAlignment: Text.AlignRight
                text: Math.round(outVolumeSlider.value) + "%"
                color: "#ccc"
            }

            Label { text: "Mon"; color: "#aaa" }
            ToolButton {
                focusPolicy: Qt.NoFocus
                display: AbstractButton.IconOnly
                text: controller.monitorMuted ? "⊘" : "🎧"
                font.pointSize: controller.monitorMuted ? 16 : 12
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
                onMoved: controller.updateMonitorVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    controller.updateMonitorVolume(Math.round(value))
            }
            Label {
                Layout.preferredWidth: 36
                horizontalAlignment: Text.AlignRight
                text: Math.round(monVolumeSlider.value) + "%"
                color: "#ccc"
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
        onVisibleChanged: controller.setPlaybackKeysEnabled(!visible)
    }
}
