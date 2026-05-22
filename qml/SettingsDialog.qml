import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.benkahn.soundboard

Dialog {
    id: root
    title: "Settings"
    modal: true
    standardButtons: Dialog.Apply | Dialog.Close
    width: 560
    height: 480

    required property SoundboardController controller
    required property Settings settings

    onApplied: if (settings) settings.apply()

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        TabBar {
            id: tabBar
            Layout.fillWidth: true
            TabButton { text: "Audio" }
            TabButton { text: "Shortcuts" }
            TabButton { text: "General" }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            // Audio
            ColumnLayout {
                spacing: 8
                Label { text: "Microphone source (PipeWire)"; font.bold: true }
                ComboBox {
                    id: micCombo
                    Layout.fillWidth: true
                    model: controller.micSourceCount
                    textRole: "text"
                    delegate: ItemDelegate {
                        required property int index
                        text: controller.micSourceNameAt(index)
                    }
                    contentItem: Text {
                        text: micCombo.displayText.length > 0
                              ? micCombo.displayText
                              : (settings ? settings.micSource : "")
                        elide: Text.ElideRight
                        verticalAlignment: Text.AlignVCenter
                        leftPadding: 8
                    }
                    onActivated: if (settings) settings.micSource = controller.micSourceNameAt(index)
                    Component.onCompleted: {
                        if (!settings) return
                        var current = settings.micSource
                        for (var i = 0; i < controller.micSourceCount; ++i) {
                            if (controller.micSourceNameAt(i) === current) {
                                currentIndex = i
                                return
                            }
                        }
                    }
                }
                TextField {
                    Layout.fillWidth: true
                    placeholderText: "Or type a PipeWire source name"
                    text: settings ? settings.micSource : ""
                    onTextChanged: if (settings) settings.micSource = text
                }
                Label { text: "Latency (ms)" }
                SpinBox {
                    from: 10
                    to: 100
                    value: settings ? settings.latencyMs : 20
                    onValueChanged: if (settings) settings.latencyMs = value
                }
                CheckBox {
                    text: "Unload PipeWire modules on quit"
                    checked: settings ? settings.autoTeardown : true
                    onCheckedChanged: if (settings) settings.autoTeardown = checked
                }
            }

            // Shortcuts
            ColumnLayout {
                spacing: 8
                Label { text: "Global shortcut backend"; font.bold: true }
                ComboBox {
                    Layout.fillWidth: true
                    model: ["auto", "portal", "kglobalaccel"]
                    currentIndex: {
                        if (!settings) return 0
                        var idx = model.indexOf(settings.shortcutMode)
                        return idx >= 0 ? idx : 0
                    }
                    onActivated: if (settings) settings.shortcutMode = model[currentIndex]
                }
                Label {
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    text: "auto tries xdg-desktop-portal first (cross-desktop), then falls back to KGlobalAccel on KDE. Portal may show a permission dialog on first bind."
                }
                Label { text: "Default bindings"; font.bold: true }
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: settings ? settings.shortcutCount : 0
                    delegate: RowLayout {
                        width: ListView.view.width
                        Label {
                            Layout.preferredWidth: 180
                            text: settings.shortcutDescriptionAt(index)
                        }
                        Label {
                            Layout.fillWidth: true
                            text: settings.shortcutTriggerAt(index)
                            color: "#aaa"
                        }
                    }
                }
            }

            // General
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                ColumnLayout {
                    width: parent.width
                    spacing: 8
                    Label { text: "Paths"; font.bold: true }
                    Label { text: "Tabs root" }
                    TextField {
                        Layout.fillWidth: true
                        text: settings ? settings.tabsRoot : ""
                        onTextChanged: if (settings) settings.tabsRoot = text
                    }
                    Label { text: "State directory" }
                    TextField {
                        Layout.fillWidth: true
                        text: settings ? settings.stateDir : ""
                        onTextChanged: if (settings) settings.stateDir = text
                    }
                    Label { text: "Custom tab folders ([[tabs]] in config)"; font.bold: true }
                    ListView {
                        id: customTabs
                        Layout.fillWidth: true
                        Layout.preferredHeight: 120
                        clip: true
                        model: settings ? settings.customTabCount : 0
                        delegate: RowLayout {
                            width: customTabs.width
                            Label {
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                text: settings.customTabPathAt(index)
                                    + (settings.customTabNameAt(index).length > 0
                                       ? " (" + settings.customTabNameAt(index) + ")"
                                       : "")
                            }
                            ToolButton {
                                text: "✕"
                                onClicked: settings.removeCustomTab(index)
                            }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        TextField {
                            id: newTabPath
                            Layout.fillWidth: true
                            placeholderText: "/path/to/tab/folder"
                        }
                        TextField {
                            id: newTabName
                            Layout.preferredWidth: 120
                            placeholderText: "Display name"
                        }
                        Button {
                            text: "Add"
                            onClicked: {
                                if (!settings || newTabPath.text.length === 0) return
                                settings.addCustomTab(newTabPath.text, newTabName.text)
                                newTabPath.text = ""
                                newTabName.text = ""
                            }
                        }
                    }
                    CheckBox {
                        text: "Minimize to tray (not yet implemented)"
                        enabled: false
                        checked: settings ? settings.minimizeToTray : true
                    }
                    CheckBox {
                        text: "Launch at login (not yet implemented)"
                        enabled: false
                        checked: settings ? settings.launchAtLogin : false
                    }
                }
            }
        }

        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: settings && settings.statusMessage.length > 0 ? "#8bc34a" : "transparent"
            text: settings ? settings.statusMessage : ""
        }
    }
}
