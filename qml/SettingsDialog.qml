import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.benwhite1987.soundspring

Window {
    id: root
    title: "Sound Spring — Settings"
    width: 680
    height: 760
    minimumWidth: 520
    minimumHeight: 560
    flags: Qt.Window | Qt.WindowTitleHint | Qt.WindowCloseButtonHint | Qt.WindowMinMaxButtonsHint
    modality: Qt.ApplicationModal
    color: appTheme.windowBg

    SoundSpringTheme {
        id: appTheme
    }

    palette: Palette {
        alternateBase: appTheme.surface
        base: appTheme.surface
        button: appTheme.surface
        buttonText: appTheme.textPrimary
        highlight: appTheme.accent
        highlightedText: appTheme.textPrimary
        text: appTheme.textPrimary
        window: appTheme.windowBg
        windowText: appTheme.textPrimary
        toolTipBase: appTheme.chromeBg
        toolTipText: appTheme.textPrimary
    }

    component SettingsTab: TabButton {
        implicitHeight: 36
        leftPadding: 12
        rightPadding: 12
        background: Rectangle {
            radius: 5
            color: parent.checked ? appTheme.surfaceActive
                  : (parent.hovered ? appTheme.surfaceHover : "transparent")
            border.color: parent.checked ? appTheme.borderAccent : appTheme.border
            border.width: parent.checked ? 1 : (parent.hovered ? 1 : 0)
        }
    }

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
        controller.refreshAudioDevices()
        controller.syncGlobalShortcutsStatus()
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
            SettingsTab { text: "Application" }
            SettingsTab { text: "Audio" }
            SettingsTab { text: "Shortcuts" }
            SettingsTab { text: "Folders" }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            // Application tab
            ScrollView {
                id: applicationScroll
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                contentWidth: availableWidth
                ColumnLayout {
                    width: applicationScroll.availableWidth
                    spacing: 4

                    SettingsSection {
                        title: "Window behavior"
                        description: "Control what happens when you close the main window and whether Sound Spring starts with your session."

                        CheckBox {
                            text: "Minimize to tray"
                            checked: settings ? settings.minimizeToTray : true
                            onCheckedChanged: if (settings) settings.minimizeToTray = checked
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Closing the window hides Sound Spring. Use the tray icon to reopen it."
                        }

                        CheckBox {
                            text: "Launch at login"
                            checked: settings ? settings.launchAtLogin : false
                            onCheckedChanged: if (settings) settings.launchAtLogin = checked
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Adds an autostart entry under ~/.config/autostart."
                        }
                    }
                }
            }

            // Audio tab
            ScrollView {
                id: audioScroll
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                contentWidth: availableWidth
                ColumnLayout {
                    width: audioScroll.availableWidth
                    spacing: 4

                    SettingsSection {
                        title: "Devices"
                        description: "PipeWire audio routing for the virtual microphone and local monitor output. Lists update automatically when devices are plugged in or removed."

                        Label { text: "Microphone source (PipeWire)" }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            ComboBox {
                                id: micCombo
                                Layout.fillWidth: true
                                model: controller.micSourceCount
                                delegate: ItemDelegate {
                                    required property int index
                                    text: {
                                        controller.micSourcesVersion
                                        return controller.micSourceDescriptionAt(index)
                                    }
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
                            AppButton {
                                text: "Refresh"
                                onClicked: controller.refreshAudioDevices()
                            }
                        }

                        Label { text: "Monitor output device" }
                        ComboBox {
                            id: monitorCombo
                            Layout.fillWidth: true
                            model: controller.audioSinkCount + 1
                            delegate: ItemDelegate {
                                required property int index
                                text: index === 0
                                      ? "Default output device"
                                      : controller.audioSinkDescriptionAt(index - 1)
                            }
                            contentItem: Text {
                                text: {
                                    controller.audioSinksVersion
                                    return monitorCombo.selectedDescription()
                                }
                                elide: Text.ElideRight
                                verticalAlignment: Text.AlignVCenter
                                leftPadding: 8
                            }
                            onActivated: if (settings) {
                                settings.monitorSink = currentIndex <= 0
                                    ? ""
                                    : controller.audioSinkIdAt(currentIndex - 1)
                            }
                            function selectedDescription() {
                                if (!settings) return "Default output device"
                                var currentId = settings.monitorSink
                                if (currentId.length === 0) return "Default output device"
                                for (var i = 0; i < controller.audioSinkCount; ++i) {
                                    if (controller.audioSinkIdAt(i) === currentId) {
                                        return controller.audioSinkDescriptionAt(i)
                                    }
                                }
                                return currentId
                            }
                            function syncSelection() {
                                if (!settings) return
                                var currentId = settings.monitorSink
                                if (currentId.length === 0) {
                                    currentIndex = 0
                                    return
                                }
                                for (var i = 0; i < controller.audioSinkCount; ++i) {
                                    if (controller.audioSinkIdAt(i) === currentId) {
                                        currentIndex = i + 1
                                        return
                                    }
                                }
                                currentIndex = 0
                            }
                            Component.onCompleted: syncSelection()
                            Connections {
                                target: controller
                                function onAudioSinksVersionChanged() {
                                    monitorCombo.syncSelection()
                                }
                            }
                        }

                        Label { text: "Latency (ms)" }
                        SpinBox {
                            from: 10
                            to: 100
                            value: settings ? settings.latencyMs : 20
                            onValueChanged: if (settings) settings.latencyMs = value
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Lower values reduce delay. Very low values may cause audio glitches."
                        }

                        CheckBox {
                            text: "Unload PipeWire modules on quit"
                            checked: settings ? settings.autoTeardown : true
                            onCheckedChanged: if (settings) settings.autoTeardown = checked
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Removes virtual audio devices when Sound Spring exits. Disable if other apps still use them."
                        }
                    }

                    SettingsSection {
                        title: "Playback"
                        description: "How sounds behave when you trigger a new one while others are playing."

                        Label { text: "Interruption mode" }
                        ComboBox {
                            Layout.fillWidth: true
                            model: ["overlap", "interrupt"]
                            currentIndex: {
                                if (!settings) return 0
                                var idx = model.indexOf(settings.interruptionMode)
                                return idx >= 0 ? idx : 0
                            }
                            onActivated: if (settings) settings.interruptionMode = model[currentIndex]
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Overlap allows multiple sounds at once; Interrupt stops any playing sound before starting a new one."
                        }

                        CheckBox {
                            text: "Mute real microphone during playback"
                            checked: settings ? settings.muteMicDuringPlayback : false
                            onCheckedChanged: if (settings) settings.muteMicDuringPlayback = checked
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Temporarily mutes your physical microphone while a sound is playing."
                        }
                    }

                    SettingsSection {
                        title: "Gate timing"
                        description: "Fine-tune voice gating when speaker verification or noise suppression is routing your mic. Leave defaults unless speech feels clipped."

                        Label { text: "Tail hold (ms)" }
                        SpinBox {
                            from: 0
                            to: 400
                            stepSize: 10
                            value: settings ? settings.gateHangoverMs : 200
                            onValueChanged: if (settings) settings.gateHangoverMs = value
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Keeps the gate open briefly after VAD drops, preserving word endings."
                        }

                        Label { text: "Fade-out (ms)" }
                        SpinBox {
                            from: 20
                            to: 200
                            stepSize: 5
                            value: settings ? settings.gateReleaseMs : 100
                            onValueChanged: if (settings) settings.gateReleaseMs = value
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "How long the output gate takes to close after speech ends."
                        }

                        CheckBox {
                            text: "Verification warm-up (pass audio until first failed check)"
                            checked: settings ? settings.verificationWarmup : true
                            onCheckedChanged: if (settings) settings.verificationWarmup = checked
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Avoids silencing the first syllable while the speaker model warms up. Disable for stricter gating from the first sample."
                        }
                    }
                }
            }

            // Shortcuts tab
            ScrollView {
                id: shortcutsScroll
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                contentWidth: availableWidth
                ColumnLayout {
                    width: shortcutsScroll.availableWidth
                    spacing: 4

                    SettingsSection {
                        title: "Global shortcut backend"
                        description: "Portal registers shortcuts with KDE System Settings (requires Apply). Local handles shortcuts only while Sound Spring is focused."

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
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: settings && settings.shortcutMode === "portal"
                                  ? "Click Apply to register global shortcuts with KDE. Accept the permission dialog when it appears."
                                  : "Local mode does not register global shortcuts. Key bindings below work only while this window is focused."
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            visible: settings && settings.shortcutMode === "portal"
                            text: "Edit bindings below, then Apply to sync with System Settings. Use Open in System Settings for advanced changes in KDE."
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            text: controller.globalShortcutsStatus
                            color: {
                                var status = controller.globalShortcutsStatus
                                if (status.indexOf("Global shortcuts active:") === 0)
                                    return appTheme.accent
                                return appTheme.warningAccent
                            }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            AppButton {
                                text: "Open in System Settings"
                                enabled: settings && settings.shortcutMode === "portal"
                                onClicked: controller.configureGlobalShortcuts()
                            }
                        }
                    }

                    SettingsSection {
                        title: "Numpad + NumLock"
                        description: ""

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: numlockColumn.implicitHeight + 16
                            color: appTheme.warningBg
                            border.color: appTheme.warningBorder
                            border.width: 1
                            radius: 4
                            ColumnLayout {
                                id: numlockColumn
                                anchors.fill: parent
                                anchors.margins: 8
                                spacing: 6
                                Label {
                                    Layout.fillWidth: true
                                    wrapMode: Text.WordWrap
                                    font.bold: true
                                    text: "NumLock affects numpad digits"
                                    color: appTheme.warningText
                                }
                                Label {
                                    Layout.fillWidth: true
                                    wrapMode: Text.WordWrap
                                    color: appTheme.warningTextMuted
                                    text: "Numpad number keys produce different X11 keysyms with " +
                                          "NumLock off (KP_End instead of KP_1, etc.), so global " +
                                          "shortcuts bound to KP_1–KP_0 only fire when NumLock is ON. " +
                                          "Numpad operator keys (+, -, *, /, Enter) are not affected."
                                }
                                CheckBox {
                                    Layout.fillWidth: true
                                    text: "Ignore NumLock state (also register navigation-cluster keysyms)"
                                    checked: settings ? settings.ignoreNumlock : false
                                    onToggled: if (settings) settings.ignoreNumlock = checked
                                }
                                Label {
                                    Layout.fillWidth: true
                                    wrapMode: Text.WordWrap
                                    color: appTheme.warningDetail
                                    font.italic: true
                                    text: "When enabled, each numpad shortcut is bound twice " +
                                          "(e.g. Num 1 AND Num End). Click Apply after changing."
                                }
                            }
                        }
                    }

                    SettingsSection {
                        title: "Key bindings"
                        description: "These bindings work immediately while Sound Spring is focused. When using portal mode, click Apply to register the same keys as global shortcuts."

                        Repeater {
                            model: settings ? settings.shortcutCount : 0
                            delegate: RowLayout {
                                Layout.fillWidth: true
                                spacing: 8
                                Label {
                                    Layout.preferredWidth: 148
                                    Layout.maximumWidth: 148
                                    elide: Text.ElideRight
                                    text: settings.shortcutDescriptionAt(index)
                                }
                                ShortcutCapture {
                                    Layout.fillWidth: true
                                    Layout.minimumWidth: 0
                                    Layout.preferredHeight: 36
                                    shortcutIndex: index
                                    settings: root.settings
                                    captureHost: root
                                }
                            }
                        }
                    }
                }
            }

            // Folders tab
            ScrollView {
                id: foldersScroll
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                contentWidth: availableWidth
                ColumnLayout {
                    width: foldersScroll.availableWidth
                    spacing: 4

                    SettingsSection {
                        title: "Paths"
                        description: "Where tab folders and session state are stored on disk."

                        Label { text: "Tabs root" }
                        TextField {
                            Layout.fillWidth: true
                            text: settings ? settings.tabsRoot : ""
                            onTextChanged: if (settings) settings.tabsRoot = text
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Default parent folder for tab subdirectories and sound files."
                        }

                        Label { text: "State directory" }
                        TextField {
                            Layout.fillWidth: true
                            text: settings ? settings.stateDir : ""
                            onTextChanged: if (settings) settings.stateDir = text
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Stores state.json (last tab, volumes, window geometry)."
                        }
                    }

                    SettingsSection {
                        title: "Custom tab folders"
                        description: "Additional tab folders listed in the [[tabs]] section of config.toml."

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
                                AppButton {
                                    text: "Remove"
                                    padding: 6
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
                            AppButton {
                                text: "Add"
                                onClicked: {
                                    if (!settings || newTabPath.text.length === 0) return
                                    settings.addCustomTab(newTabPath.text, newTabName.text)
                                    newTabPath.text = ""
                                    newTabName.text = ""
                                }
                            }
                        }
                    }
                }
            }
        }

        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: settings && settings.statusMessage.length > 0 ? appTheme.accent : "transparent"
            text: settings ? settings.statusMessage : ""
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Close"
                onClicked: root.close()
            }
            AppButton {
                text: "Apply"
                role: "primary"
                onClicked: {
                    if (settings) {
                        controller.refreshPortalParentWindow()
                        settings.apply()
                        if (ownerWindow && ownerWindow.syncTray)
                            ownerWindow.syncTray()
                    }
                }
            }
        }
    }
}
