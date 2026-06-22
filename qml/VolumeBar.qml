import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.benwhite1987.soundspring

ToolBar {
    id: volumeBar

    required property SoundboardController controller
    property var theme

    readonly property bool compactText: width < 720
    readonly property int layoutMinimumWidth:
        padding * 2
        + 3 * 32
        + 3 * 56
        + spacing * 6
        + stopAllButton.implicitWidth

    padding: 8
    spacing: 8
    background: Rectangle {
        color: volumeBar.theme ? volumeBar.theme.chromeBg : "#252528"
        Rectangle {
            anchors.top: parent.top
            width: parent.width
            height: 1
            color: volumeBar.theme ? volumeBar.theme.border : "#5a5a62"
            opacity: 0.45
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: volumeBar.spacing

        RowLayout {
            Layout.fillWidth: true
            Layout.minimumWidth: 3 * 32 + 3 * 56 + volumeBar.spacing * 4
            spacing: volumeBar.spacing

            Label {
                text: "Remote Output"
                visible: !volumeBar.compactText
                color: volumeBar.theme ? volumeBar.theme.textSecondary : "#b3b3bc"
                Layout.rightMargin: 2
            }
            ToolButton {
                focusPolicy: Qt.NoFocus
                display: volumeBar.compactText ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                padding: 6
                palette.buttonText: volumeBar.theme ? volumeBar.theme.textPrimary : "#ececec"
                icon.width: 20
                icon.height: 20
                icon.name: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.outputMuted ? "audio-volume-muted" : "audio-volume-high"
                }
                text: {
                    volumeBar.controller.uiVersion
                    if (volumeBar.compactText || !volumeBar.controller.outputMuted)
                        return ""
                    return "Muted"
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.outputMuted ? 0.45 : 1.0
                }
                background: Rectangle {
                    radius: 4
                    color: parent.hovered
                           ? (volumeBar.theme ? volumeBar.theme.surfaceHover : "#3d3d44")
                           : "transparent"
                }
                ToolTip.visible: hovered
                ToolTip.text: volumeBar.controller.outputMuted
                              ? "Output muted — click to unmute"
                              : "Output unmuted — click to mute"
                onClicked: volumeBar.controller.toggleOutputMute()
            }
            Slider {
                id: outVolumeSlider
                focusPolicy: Qt.NoFocus
                Layout.fillWidth: true
                Layout.minimumWidth: 56
                Layout.leftMargin: 4
                from: 0
                to: 100
                value: volumeBar.controller.outputVolume
                live: true
                enabled: {
                    volumeBar.controller.uiVersion
                    return !volumeBar.controller.outputMuted
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.outputMuted ? 0.4 : 1.0
                }
                onMoved: volumeBar.controller.updateOutputVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    volumeBar.controller.updateOutputVolume(Math.round(value))
            }
            Label {
                visible: !volumeBar.compactText
                Layout.preferredWidth: 40
                horizontalAlignment: Text.AlignRight
                text: Math.round(outVolumeSlider.value) + "%"
                color: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.outputMuted
                           ? (volumeBar.theme ? volumeBar.theme.textMuted : "#888892")
                           : (volumeBar.theme ? volumeBar.theme.textSecondary : "#b3b3bc")
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.outputMuted ? 0.4 : 1.0
                }
            }

            Rectangle {
                visible: !volumeBar.compactText
                Layout.preferredWidth: 1
                Layout.preferredHeight: 28
                Layout.leftMargin: 8
                Layout.rightMargin: 8
                color: volumeBar.theme ? volumeBar.theme.border : "#5a5a62"
                opacity: 0.5
            }

            Label {
                text: "Local Monitor"
                visible: !volumeBar.compactText
                color: volumeBar.theme ? volumeBar.theme.textSecondary : "#b3b3bc"
                Layout.rightMargin: 2
            }
            ToolButton {
                focusPolicy: Qt.NoFocus
                display: volumeBar.compactText ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                padding: 6
                palette.buttonText: volumeBar.theme ? volumeBar.theme.textPrimary : "#ececec"
                icon.width: 20
                icon.height: 20
                icon.name: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.monitorMuted ? "audio-volume-muted" : "audio-headphones"
                }
                text: {
                    volumeBar.controller.uiVersion
                    if (volumeBar.compactText || !volumeBar.controller.monitorMuted)
                        return ""
                    return "Muted"
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.monitorMuted ? 0.45 : 1.0
                }
                background: Rectangle {
                    radius: 4
                    color: parent.hovered
                           ? (volumeBar.theme ? volumeBar.theme.surfaceHover : "#3d3d44")
                           : "transparent"
                }
                ToolTip.visible: hovered
                ToolTip.text: volumeBar.controller.monitorMuted
                              ? "Monitor muted — click to unmute"
                              : "Monitor unmuted — click to mute"
                onClicked: volumeBar.controller.toggleMonitorMute()
            }
            Slider {
                id: monVolumeSlider
                focusPolicy: Qt.NoFocus
                Layout.fillWidth: true
                Layout.minimumWidth: 56
                Layout.leftMargin: 4
                from: 0
                to: 100
                value: volumeBar.controller.monitorVolume
                live: true
                enabled: {
                    volumeBar.controller.uiVersion
                    return !volumeBar.controller.monitorMuted
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.monitorMuted ? 0.4 : 1.0
                }
                onMoved: volumeBar.controller.updateMonitorVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    volumeBar.controller.updateMonitorVolume(Math.round(value))
            }
            Label {
                visible: !volumeBar.compactText
                Layout.preferredWidth: 40
                horizontalAlignment: Text.AlignRight
                text: Math.round(monVolumeSlider.value) + "%"
                color: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.monitorMuted
                           ? (volumeBar.theme ? volumeBar.theme.textMuted : "#888892")
                           : (volumeBar.theme ? volumeBar.theme.textSecondary : "#b3b3bc")
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.monitorMuted ? 0.4 : 1.0
                }
            }

            Rectangle {
                visible: !volumeBar.compactText
                Layout.preferredWidth: 1
                Layout.preferredHeight: 28
                Layout.leftMargin: 8
                Layout.rightMargin: 8
                color: volumeBar.theme ? volumeBar.theme.border : "#5a5a62"
                opacity: 0.5
            }

            Label {
                text: "Mic Output"
                visible: !volumeBar.compactText
                color: volumeBar.theme ? volumeBar.theme.textSecondary : "#b3b3bc"
                Layout.rightMargin: 2
            }
            ToolButton {
                focusPolicy: Qt.NoFocus
                display: volumeBar.compactText ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                padding: 6
                palette.buttonText: volumeBar.theme ? volumeBar.theme.textPrimary : "#ececec"
                icon.width: 20
                icon.height: 20
                icon.name: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.micMuted ? "audio-volume-muted" : "audio-input-microphone"
                }
                text: {
                    volumeBar.controller.uiVersion
                    if (volumeBar.compactText || !volumeBar.controller.micMuted)
                        return ""
                    return "Muted"
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.micMuted ? 0.45 : 1.0
                }
                background: Rectangle {
                    radius: 4
                    color: parent.hovered
                           ? (volumeBar.theme ? volumeBar.theme.surfaceHover : "#3d3d44")
                           : "transparent"
                }
                ToolTip.visible: hovered
                ToolTip.text: volumeBar.controller.micMuted
                              ? "Mic muted — click to unmute"
                              : "Mic unmuted — click to mute"
                onClicked: volumeBar.controller.toggleMicMute()
            }
            Slider {
                id: micVolumeSlider
                focusPolicy: Qt.NoFocus
                Layout.fillWidth: true
                Layout.minimumWidth: 56
                Layout.leftMargin: 4
                from: 0
                to: 100
                value: volumeBar.controller.micVolume
                live: true
                enabled: {
                    volumeBar.controller.uiVersion
                    return !volumeBar.controller.micMuted
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.micMuted ? 0.4 : 1.0
                }
                onMoved: volumeBar.controller.updateMicVolume(Math.round(value))
                onPressedChanged: if (!pressed)
                    volumeBar.controller.updateMicVolume(Math.round(value))
            }
            Label {
                visible: !volumeBar.compactText
                Layout.preferredWidth: 40
                horizontalAlignment: Text.AlignRight
                text: Math.round(micVolumeSlider.value) + "%"
                color: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.micMuted
                           ? (volumeBar.theme ? volumeBar.theme.textMuted : "#888892")
                           : (volumeBar.theme ? volumeBar.theme.textSecondary : "#b3b3bc")
                }
                opacity: {
                    volumeBar.controller.uiVersion
                    return volumeBar.controller.micMuted ? 0.4 : 1.0
                }
            }
        }

        AppButton {
            id: stopAllButton
            focusPolicy: Qt.NoFocus
            Layout.minimumWidth: implicitWidth
            text: {
                volumeBar.controller.shortcutVersion
                if (volumeBar.compactText)
                    return "Stop All"
                var seq = volumeBar.controller.shortcutSequence("stop_all")
                return seq.length > 0 ? ("Stop All (" + seq + ")") : "Stop All"
            }
            role: "danger"
            onClicked: volumeBar.controller.stopAll()
        }
    }
}
