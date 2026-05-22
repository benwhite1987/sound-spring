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

    Timer {
        interval: 100
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
                    text: (controller.tabVersion, controller.tabNameAt(index))
                    checkable: true
                    checked: (controller.tabVersion, controller.currentTabIndex === index)
                    onClicked: controller.selectTab(index)
                }
            }

            Item { Layout.fillWidth: true }

            ToolButton {
                text: "◀"
                onClicked: controller.prevTab()
            }
            ToolButton {
                text: "▶"
                onClicked: controller.nextTab()
            }
            ToolButton {
                text: "⚙"
                onClicked: {
                    settings.loadFromConfig()
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
            Label {
                text: {
                    controller.tabVersion
                    return controller.currentTabName.length > 0
                           ? "Tab: " + controller.currentTabName
                           : "No tabs configured"
                }
            }
            Item { Layout.fillWidth: true }
            Button {
                text: "Stop All"
                onClicked: controller.stopAll()
            }
        }
    }

    SettingsDialog {
        id: settingsDialog
        controller: controller
        settings: settings
    }
}
