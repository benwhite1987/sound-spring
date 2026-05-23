import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.benkahn.soundboard

Window {
    id: root
    title: "Sound Spring — Settings"
    width: 640
    height: 760
    minimumWidth: 520
    minimumHeight: 560
    flags: Qt.Window | Qt.WindowTitleHint | Qt.WindowCloseButtonHint | Qt.WindowMinMaxButtonsHint
    modality: Qt.ApplicationModal
    color: palette.window

    required property SoundboardController controller
    required property var settings
    property var ownerWindow: null
    property int activeCaptureIndex: -1

    onOwnerWindowChanged: if (ownerWindow)
        transientParent = ownerWindow

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
        controller.refreshShortcutBindings()
    }

    function openSettings() {
        if (settings)
            settings.loadFromConfig()
        controller.refreshMicSources()
        show()
        raise()
        requestActivate()
    }

    onClosing: activeCaptureIndex = -1

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        TabBar {
            id: tabBar
            Layout.fillWidth: true
            TabButton { text: "Audio" }
            TabButton { text: "Shortcuts" }
            TabButton { text: "General" }
        }

        ScrollView {
            id: tabScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: tabScroll.availableWidth
                spacing: 10

                // Audio tab
                ColumnLayout {
                    width: parent.width
                    spacing: 10
                    visible: tabBar.currentIndex === 0

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
                                text: {
                                    controller.micSourcesVersion
                                    if (micCombo.currentIndex >= 0
                                            && micCombo.currentIndex < controller.micSourceCount) {
                                        return controller.micSourceDescriptionAt(micCombo.currentIndex)
                                    }
                                    return micCombo.selectedDescription()
                                }
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
                                function onMicSourcesVersionChanged() {
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

                // Shortcuts tab
                ColumnLayout {
                    width: parent.width
                    spacing: 10
                    visible: tabBar.currentIndex === 1

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
                    text: "Global shortcuts activate when you click Apply (not on launch). " +
                          "In-window keys work immediately without Apply."
                }
                Label {
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    color: "#aaa"
                    text: "Edit bindings below, then Apply to sync with System Settings. " +
                          "Use Open in System Settings for advanced changes in KDE."
                }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        Button {
                            text: "Open in System Settings"
                            enabled: settings && settings.shortcutMode === "portal"
                            onClicked: controller.configureGlobalShortcuts()
                        }
                        Button {
                            text: "Reset global shortcuts"
                            enabled: settings && settings.shortcutMode === "portal"
                            onClicked: controller.resetGlobalShortcuts()
                        }
                    }
                    Label { text: "In-window bindings (while focused)"; font.bold: true }
                    Repeater {
                        model: settings ? settings.shortcutCount : 0
                        delegate: RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Label {
                                Layout.preferredWidth: 168
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

                // General tab
                ColumnLayout {
                    width: parent.width
                    spacing: 10
                    visible: tabBar.currentIndex === 2

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
                    Repeater {
                        model: settings ? settings.customTabCount : 0
                        delegate: RowLayout {
                            Layout.fillWidth: true
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

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Item { Layout.fillWidth: true }
            Button {
                text: "Close"
                onClicked: root.close()
            }
            Button {
                text: "Apply"
                highlighted: true
                onClicked: if (settings) settings.apply()
            }
        }
    }
}
