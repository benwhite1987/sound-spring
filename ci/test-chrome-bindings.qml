import QtQuick

// Verifies the uiVersion binding pattern used in Main.qml refreshes derived state.
Item {
    property int uiVersion: 0
    property int currentTabIndex: 0
    property int tabIndex: 1
    property bool tabChecked: {
        uiVersion
        return currentTabIndex === tabIndex
    }
    property bool muteLabel: {
        uiVersion
        return outputMuted
    }
    property bool outputMuted: false

    function fail(message) {
        console.error("FAIL:", message)
        Qt.exit(1)
    }

    Component.onCompleted: {
        if (tabChecked)
            fail("tab should start unchecked for tabIndex 1")
        if (muteLabel)
            fail("mute label should start false")

        currentTabIndex = 1
        outputMuted = true
        if (tabChecked)
            fail("tab binding should not refresh without uiVersion bump")
        if (!muteLabel)
            ; // expected: still false without bump
        else
            fail("mute binding should not refresh without uiVersion bump")

        uiVersion++
        if (!tabChecked)
            fail("tab binding should be true after uiVersion bump")
        if (!muteLabel)
            fail("mute binding should be true after uiVersion bump")

        uiVersion++
        currentTabIndex = 0
        outputMuted = false
        if (tabChecked)
            fail("tab binding should be false after second bump")
        if (muteLabel)
            fail("mute binding should be false after second bump")

        console.log("PASS: chrome binding pattern")
        Qt.exit(0)
    }
}
