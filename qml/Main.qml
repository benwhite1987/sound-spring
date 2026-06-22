import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import io.github.benwhite1987.soundspring

ApplicationWindow {
    id: root
    width: controller.hasSavedWindowGeometry() ? controller.savedWindowWidth() : 800
    height: controller.hasSavedWindowGeometry() ? controller.savedWindowHeight() : 600
    minimumWidth: Math.max(520, volumeFooter.layoutMinimumWidth)
    minimumHeight: 400
    visible: true
    title: "Sound Spring"
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

    component ChromeButton: ToolButton {
        focusPolicy: Qt.NoFocus
        padding: 8
        palette.buttonText: appTheme.textPrimary
        background: Rectangle {
            implicitWidth: 36
            implicitHeight: 32
            radius: 4
            color: parent.down ? appTheme.surfaceHover
                               : (parent.hovered ? appTheme.surface : "transparent")
            border.color: parent.hovered ? appTheme.border : "transparent"
            border.width: 1
        }
    }

    component PanelButton: ToolButton {
        property int panelIndex: 0
        readonly property bool isActive: root.activePanel === panelIndex
        focusPolicy: Qt.NoFocus
        padding: 8
        display: AbstractButton.TextOnly
        palette.buttonText: appTheme.textPrimary
        contentItem: Text {
            text: parent.text
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            color: parent.palette.buttonText
            font.pixelSize: 13
            font.weight: parent.isActive ? Font.DemiBold : Font.Normal
        }
        background: Rectangle {
            anchors.fill: parent
            implicitHeight: 36
            radius: 4
            color: parent.isActive ? appTheme.surfaceActive
                                    : (parent.hovered ? appTheme.surface : "transparent")
            border.color: parent.isActive ? appTheme.borderAccent : "transparent"
            border.width: 1
        }
        onClicked: root.activePanel = panelIndex
    }

    // 0 = Soundboard (Phase 1), 1 = Voice (Phase 2).
    property int activePanel: 0
    property bool windowGeometryReady: false

    function applyCloseChoice(minimizeToTray, remember) {
        controller.applyCloseActionChoice(minimizeToTray, remember)
        settings.minimizeToTray = minimizeToTray
        syncTray()
        if (minimizeToTray && SystemTray.available) {
            root.hide()
        } else {
            if (SystemTray.available)
                SystemTray.visible = false
            quitApplication()
        }
    }

    function quitApplication() {
        controller.shutdownBackend()
        controller.saveSessionOnQuit(root.x, root.y, root.width, root.height)
        if (SystemTray.available)
            SystemTray.visible = false
        Qt.quit()
    }

    onClosing: function(close) {
        controller.saveWindowGeometry(root.x, root.y, root.width, root.height)

        if (controller.needsCloseActionPrompt() && SystemTray.available) {
            close.accepted = false
            closeActionDialog.open()
            return
        }

        if (settings.minimizeToTray && SystemTray.available) {
            close.accepted = false
            root.hide()
            return
        }

        close.accepted = false
        quitApplication()
    }

    function syncTray() {
        if (!SystemTray.available)
            return
        SystemTray.initialize()
        if (settings.minimizeToTray) {
            SystemTray.setToolTip("Sound Spring")
            SystemTray.visible = true
        } else {
            SystemTray.visible = false
        }
    }

    Component.onCompleted: {
        controller.noteFirstPaint()
        if (controller.hasSavedWindowGeometry()) {
            root.x = controller.savedWindowX()
            root.y = controller.savedWindowY()
        }
        windowGeometryReady = true
        Qt.callLater(function() {
            syncTray()
            if (SystemTray.available)
                SystemTray.setWindowVisible(root.visible)
        })
    }

    Timer {
        id: geometrySaveTimer
        interval: 500
        onTriggered: controller.saveWindowGeometry(root.x, root.y, root.width, root.height)
    }

    function scheduleWindowGeometrySave() {
        if (!windowGeometryReady)
            return
        geometrySaveTimer.restart()
    }

    onXChanged: scheduleWindowGeometrySave()
    onYChanged: scheduleWindowGeometrySave()
    onWidthChanged: scheduleWindowGeometrySave()
    onHeightChanged: scheduleWindowGeometrySave()

    onActiveChanged: controller.setWindowActive(active && visible)

    onVisibilityChanged: {
        controller.setWindowActive(active && visible)
        if (SystemTray.available)
            SystemTray.setWindowVisible(visible)
    }

    Connections {
        target: SystemTray
        function onShowWindowRequested() {
            root.show()
            root.raise()
            root.requestActivate()
        }
        function onHideWindowRequested() {
            root.hide()
        }
        function onStopAllRequested() {
            controller.stopAll()
        }
        function onQuitRequested() {
            quitApplication()
        }
    }

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

    header: ColumnLayout {
        spacing: 0

        ToolBar {
            id: appBar
            Layout.fillWidth: true
            padding: 8
            spacing: 8
            background: Rectangle {
                color: appTheme.chromeBg
                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: 1
                    color: appTheme.border
                    opacity: 0.45
                }
            }

            RowLayout {
                anchors.fill: parent
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    Item {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 0
                        Layout.preferredHeight: 36
                        PanelButton {
                            anchors.fill: parent
                            text: "Soundboard"
                            panelIndex: 0
                        }
                    }
                    Item {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 0
                        Layout.preferredHeight: 36
                        PanelButton {
                            anchors.fill: parent
                            text: "Voice"
                            panelIndex: 1
                        }
                    }
                }

                ChromeButton {
                    text: "⚙"
                    ToolTip.visible: hovered
                    ToolTip.text: "Settings"
                    onClicked: settingsDialog.openSettings()
                }
            }
        }

        ToolBar {
            id: tabBarRow
            visible: root.activePanel === 0
            Layout.fillWidth: true
            padding: 8
            spacing: 8
            background: Rectangle {
                color: appTheme.chromeBg
                Rectangle {
                    anchors.top: parent.top
                    width: parent.width
                    height: 1
                    color: appTheme.border
                    opacity: 0.25
                }
                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: 1
                    color: appTheme.border
                    opacity: 0.45
                }
            }

            RowLayout {
                anchors.fill: parent
                spacing: 8

                ListView {
                    id: tabList
                    Layout.fillWidth: true
                    Layout.preferredHeight: 36
                    orientation: ListView.Horizontal
                    spacing: 6
                    clip: true
                    property int tabStripTick: 0
                    model: {
                        tabStripTick
                        controller.uiVersion
                        return controller.tabCount
                    }

                    Connections {
                        target: controller
                        function onTabsChanged() {
                            tabList.tabStripTick++
                            tabList.forceLayout()
                        }
                        function onCurrentTabChanged() {
                            tabList.tabStripTick++
                        }
                    }

                    property int dragIndex: -1

                    delegate: Item {
                        id: tabDelegate
                        height: tabList.height
                        readonly property bool isActive: {
                            controller.uiVersion
                            return controller.currentTabIndex === index
                        }
                        readonly property string tabLabel: {
                            tabList.tabStripTick
                            controller.uiVersion
                            controller.tabVersion
                            return controller.tabNameAt(index)
                        }
                        width: Math.max(tabLabelText.implicitWidth + 24, 52)

                        Rectangle {
                            anchors.fill: parent
                            radius: 5
                            color: tabDelegate.isActive ? appTheme.surfaceActive
                                  : (tabMouse.containsMouse ? appTheme.surfaceHover : "transparent")
                            border.color: tabDelegate.isActive ? appTheme.borderAccent : appTheme.border
                            border.width: tabDelegate.isActive ? 1 : 0
                            opacity: tabDelegate.isActive ? 1.0 : (tabMouse.containsMouse ? 0.85 : 0.0)
                        }

                        Text {
                            id: tabLabelText
                            anchors.centerIn: parent
                            text: tabDelegate.tabLabel
                            color: tabDelegate.isActive ? appTheme.textPrimary : appTheme.textSecondary
                            font.pixelSize: 13
                            font.weight: tabDelegate.isActive ? Font.DemiBold : Font.Normal
                        }

                        MouseArea {
                            id: tabMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton

                            property real pressX: 0
                            property bool dragging: false

                            onPressed: (mouse) => {
                                if (mouse.button === Qt.RightButton) {
                                    tabContextMenu.tabIndex = index
                                    tabContextMenu.popup()
                                    mouse.accepted = true
                                    return
                                }
                                pressX = mouse.x
                                dragging = false
                                tabList.dragIndex = index
                                mouse.accepted = true
                            }

                            onPositionChanged: (mouse) => {
                                if (!pressed || mouse.buttons !== Qt.LeftButton)
                                    return
                                if (!dragging && Math.abs(mouse.x - pressX) > 10) {
                                    dragging = true
                                    dragHandle.visible = true
                                }
                                if (dragging)
                                    dragHandle.x = Math.max(0, Math.min(
                                        tabDelegate.width - dragHandle.width,
                                        mouse.x - dragHandle.width / 2))
                            }

                            onReleased: (mouse) => {
                                if (dragging) {
                                    var dropX = tabList.contentX + tabDelegate.x + dragHandle.x + dragHandle.width / 2
                                    var dropIdx = tabList.indexAt(dropX, tabDelegate.height / 2)
                                    if (dropIdx < 0)
                                        dropIdx = tabList.dragIndex
                                    if (dropIdx !== tabList.dragIndex)
                                        controller.moveTab(tabList.dragIndex, dropIdx)
                                } else if (mouse.button === Qt.LeftButton) {
                                    controller.selectTab(index)
                                }
                                dragging = false
                                dragHandle.visible = false
                                dragHandle.x = 0
                                tabList.dragIndex = -1
                            }

                            Rectangle {
                                id: dragHandle
                                visible: false
                                width: parent.width
                                height: parent.height
                                radius: 5
                                color: appTheme.accent
                                opacity: 0.35
                                z: -1
                            }
                        }
                    }
                }

                ChromeButton {
                    text: "+"
                    ToolTip.visible: hovered
                    ToolTip.text: "Add tab"
                    onClicked: {
                        addTabNameField.text = ""
                        addTabDialog.existingPath = ""
                        addTabDialog.open()
                    }
                }

                ChromeButton {
                    text: "◀"
                    ToolTip.visible: hovered
                    ToolTip.text: "Previous tab"
                    onClicked: controller.prevTab()
                }
                ChromeButton {
                    text: "▶"
                    ToolTip.visible: hovered
                    ToolTip.text: "Next tab"
                    onClicked: controller.nextTab()
                }
            }
        }
    }

    StackLayout {
        anchors.fill: parent
        currentIndex: root.activePanel

        Item {
            TabPage {
                anchors.fill: parent
                anchors.margins: 12
                controller: controller
            }
        }

        VoicePanel {
            controller: controller
            theme: appTheme
        }
    }

    footer: VolumeBar {
        id: volumeFooter
        controller: controller
        theme: appTheme
    }

    SettingsDialog {
        id: settingsDialog
        controller: controller
        settings: settings
        ownerWindow: root
    }

    Menu {
        id: tabContextMenu
        property int tabIndex: -1

        MenuItem {
            text: "Rename…"
            onTriggered: {
                renameTabDialog.tabIndex = tabContextMenu.tabIndex
                renameTabNameField.text = controller.tabNameAt(tabContextMenu.tabIndex)
                renameTabDialog.open()
            }
        }
        MenuItem {
            text: "Remove"
            enabled: controller.tabUsesCustomList
            onTriggered: controller.removeTab(tabContextMenu.tabIndex)
        }
        MenuSeparator {}
        MenuItem {
            text: "Move Left"
            enabled: tabContextMenu.tabIndex > 0
            onTriggered: controller.moveTab(tabContextMenu.tabIndex, tabContextMenu.tabIndex - 1)
        }
        MenuItem {
            text: "Move Right"
            enabled: tabContextMenu.tabIndex >= 0
                    && tabContextMenu.tabIndex < controller.tabCount - 1
            onTriggered: controller.moveTab(tabContextMenu.tabIndex, tabContextMenu.tabIndex + 1)
        }
    }

    Dialog {
        id: addTabDialog
        title: "Add Tab"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 420)
        padding: 24
        standardButtons: Dialog.NoButton

        property string existingPath: ""

        onAboutToShow: {
            addTabNameField.text = ""
            existingPath = ""
        }

        onAccepted: {
            controller.addTab(addTabDialog.existingPath, addTabNameField.text)
        }

        ColumnLayout {
            spacing: 12
            width: addTabDialog.availableWidth

            Label {
                Layout.fillWidth: true
                text: "Tab name"
            }
            TextField {
                id: addTabNameField
                Layout.fillWidth: true
                placeholderText: "New Tab"
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: appTheme.textMuted
                text: addTabDialog.existingPath.length > 0
                      ? addTabDialog.existingPath
                      : "Creates a new folder under the tabs root."
            }
            AppButton {
                text: "Choose existing folder…"
                onClicked: addTabFolderDialog.open()
            }
        }

        footer: RowLayout {
            spacing: 8
            width: addTabDialog.availableWidth
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Cancel"
                onClicked: addTabDialog.reject()
            }
            AppButton {
                text: "OK"
                role: "primary"
                onClicked: addTabDialog.accept()
            }
        }
    }

    FolderDialog {
        id: addTabFolderDialog
        title: "Choose tab folder"
        onAccepted: {
            var path = selectedFolder.toString()
            if (path.startsWith("file://"))
                path = path.substring(7)
            addTabDialog.existingPath = decodeURIComponent(path)
        }
    }

    Dialog {
        id: renameTabDialog
        title: "Rename Tab"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 360)
        padding: 24
        standardButtons: Dialog.NoButton

        property int tabIndex: -1

        onAccepted: controller.renameTab(renameTabDialog.tabIndex, renameTabNameField.text)

        ColumnLayout {
            spacing: 12
            width: renameTabDialog.availableWidth

            Label {
                Layout.fillWidth: true
                text: "Display name"
            }
            TextField {
                id: renameTabNameField
                Layout.fillWidth: true
            }
        }

        footer: RowLayout {
            spacing: 8
            width: renameTabDialog.availableWidth
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Cancel"
                onClicked: renameTabDialog.reject()
            }
            AppButton {
                text: "OK"
                role: "primary"
                onClicked: renameTabDialog.accept()
            }
        }
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

    Dialog {
        id: closeActionDialog
        title: "Close Sound Spring"
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.width - 80, 500)
        padding: 24
        standardButtons: Dialog.NoButton

        ColumnLayout {
            spacing: 16
            width: closeActionDialog.availableWidth

            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "When you close the window, should Sound Spring keep running " +
                      "in the system tray, or exit completely?"
                color: appTheme.textPrimary
            }

            CheckBox {
                id: rememberCloseChoice
                Layout.fillWidth: true
                text: "Remember my choice"
                checked: true
                palette.text: appTheme.textPrimary
            }
        }

        footer: RowLayout {
            spacing: 10
            width: closeActionDialog.availableWidth
            Item { Layout.fillWidth: true }
            AppButton {
                text: "Cancel"
                onClicked: closeActionDialog.close()
            }
            AppButton {
                text: "Exit"
                onClicked: {
                    closeActionDialog.close()
                    applyCloseChoice(false, rememberCloseChoice.checked)
                }
            }
            AppButton {
                text: "Minimize to Tray"
                role: "primary"
                onClicked: {
                    closeActionDialog.close()
                    applyCloseChoice(true, rememberCloseChoice.checked)
                }
            }
        }
    }
}
