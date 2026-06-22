import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.benwhite1987.soundspring

ColumnLayout {
    id: root

    property string title: ""
    property string description: ""
    default property alias content: contentLayout.data

    Layout.fillWidth: true
    spacing: 8

    SoundSpringTheme {
        id: appTheme
    }

    Label {
        visible: root.title.length > 0
        Layout.fillWidth: true
        text: root.title
        font.bold: true
        color: appTheme.textPrimary
    }

    Label {
        visible: root.description.length > 0
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        text: root.description
        color: appTheme.textMuted
    }

    ColumnLayout {
        id: contentLayout
        Layout.fillWidth: true
        spacing: 8
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.topMargin: 4
        Layout.preferredHeight: 1
        color: appTheme.border
        opacity: 0.5
    }
}
