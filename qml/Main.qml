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

    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            anchors.margins: 6
            spacing: 8

            Repeater {
                model: controller.tabCount
                delegate: ToolButton {
                    text: controller.tabNameAt(index)
                    checkable: true
                    checked: controller.currentTabIndex === index
                    onClicked: controller.currentTabIndex = index
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
                text: controller.currentTabName.length > 0
                      ? "Tab: " + controller.currentTabName
                      : "No tabs configured"
            }
            Item { Layout.fillWidth: true }
            Button {
                text: "Stop All"
                onClicked: controller.stopAll()
            }
        }
    }
}
