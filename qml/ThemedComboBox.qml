import QtQuick
import QtQuick.Controls

ComboBox {
    id: control

    property var theme

    readonly property color _text: theme ? theme.textPrimary : "#ececec"
    readonly property color _surface: theme ? theme.surface : "#333338"
    readonly property color _hover: theme ? theme.surfaceHover : "#3d3d44"
    readonly property color _window: theme ? theme.windowBg : "#1b1b1f"
    readonly property color _border: theme ? theme.border : "#5a5a62"
    readonly property color _accent: theme ? theme.accent : "#6abf69"
    readonly property color _muted: theme ? theme.textMuted : "#888892"

    palette {
        text: control._text
        buttonText: control._text
        windowText: control._text
        button: control._surface
        base: control._surface
        window: control._window
        highlight: control._accent
        highlightedText: control._text
        mid: theme ? theme.mid : "#2c2c31"
        midlight: theme ? theme.midlight : "#3a3a40"
        light: theme ? theme.light : "#4a4a52"
        dark: theme ? theme.dark : "#121215"
        placeholderText: control._muted
    }

    contentItem: Text {
        leftPadding: 8
        rightPadding: control.indicator ? control.indicator.width + 8 : 8
        text: control.displayText
        font: control.font
        color: control._text
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        implicitWidth: 120
        implicitHeight: 32
        radius: 4
        color: control.pressed || control.down ? control._hover : control._surface
        border.color: control.activeFocus ? control._accent : control._border
        border.width: 1
    }

    popup: Popup {
        y: control.height
        width: control.width
        implicitHeight: Math.min(contentItem.implicitHeight + 2, 320)
        padding: 1

        palette {
            window: control._window
            base: control._surface
            text: control._text
            highlight: control._accent
            highlightedText: control._text
            button: control._surface
            buttonText: control._text
        }

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: control.popup.visible ? control.delegateModel : null
            currentIndex: control.highlightedIndex
            ScrollIndicator.vertical: ScrollIndicator {}
        }

        background: Rectangle {
            color: control._window
            border.color: control._border
            border.width: 1
            radius: 4
        }
    }

    delegate: ItemDelegate {
        required property var modelData
        required property int index

        width: control.width
        highlighted: control.highlightedIndex === index
        text: {
            if (control.textRole && modelData !== undefined && modelData !== null
                    && typeof modelData === "object") {
                return modelData[control.textRole]
            }
            if (typeof modelData === "string" || typeof modelData === "number") {
                return String(modelData)
            }
            // Integer model (micSourceCount) — caller should override delegate.
            return control.textAt(index)
        }

        palette {
            text: control._text
            highlightedText: control._text
            highlight: control._accent
            button: control._surface
            window: control._window
        }

        background: Rectangle {
            color: parent.highlighted ? control._hover : "transparent"
        }

        contentItem: Text {
            text: parent.text
            color: control._text
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            leftPadding: 8
        }
    }
}
