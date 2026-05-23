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

    property int activeCaptureIndex: -1

    function handleKey(key, modifiers, nativeScanCode) {
        if (activeCaptureIndex < 0)
            return
        if (key === Qt.Key_Escape) {
            activeCaptureIndex = -1
            return
        }
        var trigger = settings.triggerFromKeyEvent(key, modifiers, nativeScanCode)
        if (trigger.length === 0)
            return
        settings.setShortcutTriggerAt(activeCaptureIndex, trigger)
        activeCaptureIndex = -1
    }

    onClosed: activeCaptureIndex = -1

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
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    ComboBox {
                        id: micCombo
                        Layout.fillWidth: true
                        model: controller.micSourceCount
                        delegate: ItemDelegate {
                            required property int index
                            text: controller.micSourceDescriptionAt(index)
                        }
                        contentItem: Text {
                            text: micCombo.displayText.length > 0
                                  ? micCombo.displayText
                                  : micCombo.selectedDescription()
                            elide: Text.ElideRight
                            verticalAlignment: Text.AlignVCenter
                            leftPadding: 8
                        }
                        onActivated: if (settings) {
                            settings.micSource = controller.micSourceIdAt(currentIndex)
                        }
                        function selectedDescription() {
                            if (!settings) return ""
                            var currentId = settings.micSource
                            for (var i = 0; i < controller.micSourceCount; ++i) {
                                if (controller.micSourceIdAt(i) === currentId) {
                                    return controller.micSourceDescriptionAt(i)
                                }
                            }
                            return currentId
                        }
                        function syncSelection() {
                            if (!settings) return
                            var currentId = settings.micSource
                            for (var i = 0; i < controller.micSourceCount; ++i) {
                                if (controller.micSourceIdAt(i) === currentId) {
                                    currentIndex = i
                                    return
                                }
                            }
                            currentIndex = -1
                        }
                        Component.onCompleted: syncSelection()
                        Connections {
                            target: controller
                            function onPlayingStateChanged() {
                                micCombo.syncSelection()
                            }
                        }
                    }
                    Button {
                        text: "Refresh"
                        onClicked: controller.refreshMicSources()
                    }
                }
                Label {
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    color: "#aaa"
                    text: "The list updates automatically when devices are plugged in or removed."
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
                    model: ["portal", "local"]
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
                    text: "portal registers global shortcuts; local uses in-window keys only while focused."
                }
                Label { text: "Click a slot, then press the key to assign"; font.bold: true }
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: settings ? settings.shortcutCount : 0
                    delegate: RowLayout {
                        width: ListView.view.width
                        spacing: 8
                        Label {
                            Layout.preferredWidth: 160
                            text: settings.shortcutDescriptionAt(index)
                        }
                        ShortcutCapture {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 36
                            shortcutIndex: index
                            settings: root.settings
                            captureHost: root
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
