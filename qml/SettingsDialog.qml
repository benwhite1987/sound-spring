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
        brightText: appTheme.brightText
        dark: appTheme.dark
        highlight: appTheme.accent
        highlightedText: appTheme.textPrimary
        light: appTheme.light
        link: appTheme.link
        mid: appTheme.mid
        midlight: appTheme.midlight
        placeholderText: appTheme.placeholderText
        shadow: appTheme.shadow
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
    property bool dirty: false
    property bool allowClose: false
    property string applyStatusKind: "" // "" | "ok" | "warn" | "error"

    onOwnerWindowChanged: if (ownerWindow)
        transientParent = ownerWindow

    function touch() {
        dirty = true
    }

    function clearApplyStatusSoon() {
        applyStatusClearTimer.restart()
    }

    function classifyApplyStatus(message) {
        if (!message || message.length === 0)
            return ""
        var lower = message.toLowerCase()
        if (lower.indexOf("failed") >= 0 || lower.indexOf("error") >= 0)
            return "error"
        if (lower.indexOf("but") >= 0 || lower.indexOf("partial") >= 0)
            return "warn"
        return "ok"
    }

    function applyStatusColor() {
        if (applyStatusKind === "error")
            return appTheme.danger
        if (applyStatusKind === "warn")
            return appTheme.warningAccent
        if (applyStatusKind === "ok")
            return appTheme.accent
        return "transparent"
    }

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
        dirty = true
        controller.refreshShortcutBindings()
    }

    function openSettings(options) {
        allowClose = false
        dirty = false
        applyStatusKind = ""
        if (settings)
            settings.loadFromConfig()
        controller.refreshAudioDevices()
        controller.syncGlobalShortcutsStatus()
        if (options && options.tab === "shortcuts")
            tabBar.currentIndex = 2
        else
            tabBar.currentIndex = 0
        show()
        raise()
        requestActivate()
        if (options && options.applyPortal && settings
                && settings.shortcutMode === "portal") {
            controller.refreshPortalParentWindow()
            settings.apply()
            applyStatusKind = classifyApplyStatus(settings.statusMessage)
            dirty = false
            clearApplyStatusSoon()
        }
    }

    function performClose() {
        allowClose = true
        activeCaptureIndex = -1
        close()
    }

    onClosing: (close) => {
        if (allowClose || !dirty) {
            activeCaptureIndex = -1
            allowClose = false
            return
        }
        close.accepted = false
        unsavedDialog.open()
    }

    Timer {
        id: applyStatusClearTimer
        interval: 4000
        onTriggered: applyStatusKind = ""
    }

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
                            onCheckedChanged: if (settings) {
                                settings.minimizeToTray = checked
                                root.touch()
                            }
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
                            onCheckedChanged: if (settings) {
                                settings.launchAtLogin = checked
                                root.touch()
                            }
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
                            ThemedComboBox {
                                id: micCombo
                                theme: appTheme
                                Layout.fillWidth: true
                                model: controller.micSourceCount
                                displayText: {
                                    controller.micSourcesVersion
                                    if (micCombo.currentIndex >= 0
                                            && micCombo.currentIndex < controller.micSourceCount) {
                                        return controller.micSourceDescriptionAt(micCombo.currentIndex)
                                    }
                                    return micCombo.selectedDescription()
                                }
                                delegate: ItemDelegate {
                                    required property int index
                                    width: micCombo.width
                                    text: {
                                        controller.micSourcesVersion
                                        return controller.micSourceDescriptionAt(index)
                                    }
                                    highlighted: micCombo.highlightedIndex === index
                                    palette {
                                        text: appTheme.textPrimary
                                        highlightedText: appTheme.textPrimary
                                        highlight: appTheme.accent
                                    }
                                    background: Rectangle {
                                        color: parent.highlighted ? appTheme.surfaceHover : "transparent"
                                    }
                                    contentItem: Text {
                                        text: parent.text
                                        color: appTheme.textPrimary
                                        elide: Text.ElideRight
                                        verticalAlignment: Text.AlignVCenter
                                        leftPadding: 8
                                    }
                                }
                                onActivated: if (settings) {
                                    settings.micSource = controller.micSourceIdAt(currentIndex)
                                    root.touch()
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

                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            visible: controller.micSourceCount === 0
                            color: appTheme.textMuted
                            text: "No PipeWire microphone sources found. Check that PipeWire is running, then click Refresh."
                        }

                        Label { text: "Monitor output device" }
                        ThemedComboBox {
                            id: monitorCombo
                            theme: appTheme
                            Layout.fillWidth: true
                            model: controller.audioSinkCount + 1
                            displayText: {
                                controller.audioSinksVersion
                                return monitorCombo.selectedDescription()
                            }
                            delegate: ItemDelegate {
                                required property int index
                                width: monitorCombo.width
                                text: index === 0
                                      ? "Default output device"
                                      : controller.audioSinkDescriptionAt(index - 1)
                                highlighted: monitorCombo.highlightedIndex === index
                                palette {
                                    text: appTheme.textPrimary
                                    highlightedText: appTheme.textPrimary
                                    highlight: appTheme.accent
                                }
                                background: Rectangle {
                                    color: parent.highlighted ? appTheme.surfaceHover : "transparent"
                                }
                                contentItem: Text {
                                    text: parent.text
                                    color: appTheme.textPrimary
                                    elide: Text.ElideRight
                                    verticalAlignment: Text.AlignVCenter
                                    leftPadding: 8
                                }
                            }
                            onActivated: if (settings) {
                                settings.monitorSink = currentIndex <= 0
                                    ? ""
                                    : controller.audioSinkIdAt(currentIndex - 1)
                                root.touch()
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

                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            visible: controller.audioSinkCount === 0
                            color: appTheme.textMuted
                            text: "No physical output devices listed. Playback still uses the system default sink. Click Refresh after plugging in hardware."
                        }

                        Label { text: "Latency (ms)" }
                        SpinBox {
                            from: 10
                            to: 100
                            value: settings ? settings.latencyMs : 20
                            onValueChanged: if (settings) {
                                settings.latencyMs = value
                                root.touch()
                            }
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
                            onCheckedChanged: if (settings) {
                                settings.autoTeardown = checked
                                root.touch()
                            }
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
                        ThemedComboBox {
                            id: interruptionCombo
                            theme: appTheme
                            Layout.fillWidth: true
                            textRole: "label"
                            valueRole: "value"
                            model: [
                                { label: "Overlap (play together)", value: "overlap" },
                                { label: "Interrupt previous", value: "interrupt" }
                            ]
                            currentIndex: {
                                if (!settings) return 0
                                for (var i = 0; i < model.length; ++i) {
                                    if (model[i].value === settings.interruptionMode)
                                        return i
                                }
                                return 0
                            }
                            onActivated: if (settings) {
                                settings.interruptionMode = model[currentIndex].value
                                root.touch()
                            }
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
                            onCheckedChanged: if (settings) {
                                settings.muteMicDuringPlayback = checked
                                root.touch()
                            }
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Temporarily mutes your physical microphone while a sound is playing."
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

                        ThemedComboBox {
                            id: shortcutModeCombo
                            theme: appTheme
                            Layout.fillWidth: true
                            textRole: "label"
                            valueRole: "value"
                            model: [
                                { label: "Portal (global / KDE)", value: "portal" },
                                { label: "Local (focused window only)", value: "local" }
                            ]
                            currentIndex: {
                                if (!settings) return 0
                                for (var i = 0; i < model.length; ++i) {
                                    if (model[i].value === settings.shortcutMode)
                                        return i
                                }
                                return 0
                            }
                            onActivated: if (settings) {
                                settings.shortcutMode = model[currentIndex].value
                                root.touch()
                            }
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
                                    onToggled: if (settings) {
                                        settings.ignoreNumlock = checked
                                        root.touch()
                                    }
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
                            onTextChanged: if (settings) {
                                settings.tabsRoot = text
                                root.touch()
                            }
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
                            onTextChanged: if (settings) {
                                settings.stateDir = text
                                root.touch()
                            }
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: appTheme.textMuted
                            text: "Stores state.json (last tab, volumes, window geometry)."
                        }
                    }

                    SettingsSection {
                        title: "Current tab folder"
                        description: "Open the active tab’s folder in your file manager. Use the + button on the main window to add tabs."

                        AppButton {
                            text: "Open current tab folder"
                            onClicked: controller.openTabFolder()
                        }
                    }
                }
            }
        }

        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            visible: applyStatusKind.length > 0
            color: root.applyStatusColor()
            text: settings ? settings.statusMessage : ""
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Close"
                onClicked: {
                    if (root.dirty)
                        unsavedDialog.open()
                    else
                        root.performClose()
                }
            }
            AppButton {
                text: "Apply"
                role: "primary"
                onClicked: {
                    if (settings) {
                        controller.refreshPortalParentWindow()
                        settings.apply()
                        root.dirty = false
                        root.applyStatusKind = root.classifyApplyStatus(settings.statusMessage)
                        root.clearApplyStatusSoon()
                        if (ownerWindow && ownerWindow.syncTray)
                            ownerWindow.syncTray()
                    }
                }
            }
        }
    }

    Dialog {
        id: unsavedDialog
        title: "Unsaved settings"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 420)
        padding: 24
        standardButtons: Dialog.NoButton

        Label {
            width: unsavedDialog.availableWidth
            wrapMode: Text.WordWrap
            text: "You have unsaved settings changes. Save before closing?"
        }

        footer: RowLayout {
            spacing: 8
            width: unsavedDialog.availableWidth
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Cancel"
                onClicked: unsavedDialog.close()
            }
            AppButton {
                text: "Discard"
                onClicked: {
                    unsavedDialog.close()
                    root.performClose()
                }
            }
            AppButton {
                text: "Save"
                role: "primary"
                onClicked: {
                    if (settings) {
                        controller.refreshPortalParentWindow()
                        settings.apply()
                        root.dirty = false
                        root.applyStatusKind = root.classifyApplyStatus(settings.statusMessage)
                        if (ownerWindow && ownerWindow.syncTray)
                            ownerWindow.syncTray()
                    }
                    unsavedDialog.close()
                    root.performClose()
                }
            }
        }
    }
}
