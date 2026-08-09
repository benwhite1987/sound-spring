import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.benwhite1987.soundspring

ToolBar {
    id: volumeBar

    required property SoundboardController controller
    property var theme

    function uiIcon(name) {
        return "qrc:/icons/ui/" + name + ".svg"
    }

    function uiIconColor(muted) {
        if (muted || !theme)
            return undefined
        return theme.textPrimary
    }

    readonly property bool compactText: width < 720
    readonly property int layoutMinimumWidth:
        padding * 2
        + 3 * 32
        + 3 * 56
        + spacing * 6
        + stopAllButton.implicitWidth

    component MuteVolumeChannel: RowLayout {
        id: channel
        property string labelText: ""
        property string unmutedIcon: "audio-volume-high"
        property bool muted: false
        property int volume: 0
        property string muteTooltip: ""
        property string unmuteTooltip: ""

        signal toggleMuteRequested()
        signal volumeEdited(int value)

        spacing: volumeBar.spacing

        Label {
            text: channel.labelText
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
            icon.source: {
                volumeBar.controller.uiVersion
                return channel.muted
                       ? volumeBar.uiIcon("audio-volume-muted")
                       : volumeBar.uiIcon(channel.unmutedIcon)
            }
            icon.color: {
                volumeBar.controller.uiVersion
                return volumeBar.uiIconColor(channel.muted)
            }
            text: {
                volumeBar.controller.uiVersion
                if (volumeBar.compactText || !channel.muted)
                    return ""
                return "Muted"
            }
            opacity: {
                volumeBar.controller.uiVersion
                return channel.muted ? 0.45 : 1.0
            }
            background: Rectangle {
                radius: 4
                color: parent.hovered
                       ? (volumeBar.theme ? volumeBar.theme.surfaceHover : "#3d3d44")
                       : "transparent"
            }
            ToolTip.visible: hovered
            ToolTip.text: channel.muted ? channel.muteTooltip : channel.unmuteTooltip
            onClicked: channel.toggleMuteRequested()
        }
        Slider {
            id: volumeSlider
            focusPolicy: Qt.NoFocus
            Layout.fillWidth: true
            Layout.minimumWidth: 56
            Layout.leftMargin: 4
            from: 0
            to: 100
            value: channel.volume
            live: true
            palette {
                highlight: volumeBar.theme ? volumeBar.theme.accent : "#6abf69"
                window: volumeBar.theme ? volumeBar.theme.windowBg : "#1b1b1f"
                button: volumeBar.theme ? volumeBar.theme.surface : "#333338"
                mid: volumeBar.theme ? volumeBar.theme.mid : "#2c2c31"
                light: volumeBar.theme ? volumeBar.theme.light : "#4a4a52"
                dark: volumeBar.theme ? volumeBar.theme.dark : "#121215"
                text: volumeBar.theme ? volumeBar.theme.textPrimary : "#ececec"
            }
            enabled: {
                volumeBar.controller.uiVersion
                return !channel.muted
            }
            opacity: {
                volumeBar.controller.uiVersion
                return channel.muted ? 0.4 : 1.0
            }
            onMoved: channel.volumeEdited(Math.round(value))
            onPressedChanged: if (!pressed)
                channel.volumeEdited(Math.round(value))
        }
        Label {
            visible: !volumeBar.compactText
            Layout.preferredWidth: 40
            horizontalAlignment: Text.AlignRight
            text: Math.round(volumeSlider.value) + "%"
            color: {
                volumeBar.controller.uiVersion
                return channel.muted
                       ? (volumeBar.theme ? volumeBar.theme.textMuted : "#888892")
                       : (volumeBar.theme ? volumeBar.theme.textSecondary : "#b3b3bc")
            }
            opacity: {
                volumeBar.controller.uiVersion
                return channel.muted ? 0.4 : 1.0
            }
        }
    }

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

            MuteVolumeChannel {
                Layout.fillWidth: true
                labelText: "Remote Output"
                unmutedIcon: "audio-volume-high"
                muted: volumeBar.controller.outputMuted
                volume: volumeBar.controller.outputVolume
                muteTooltip: "Output muted — click to unmute"
                unmuteTooltip: "Output unmuted — click to mute"
                onToggleMuteRequested: volumeBar.controller.toggleOutputMute()
                onVolumeEdited: (v) => volumeBar.controller.updateOutputVolume(v)
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

            MuteVolumeChannel {
                Layout.fillWidth: true
                labelText: "Local Monitor"
                unmutedIcon: "audio-headphones"
                muted: volumeBar.controller.monitorMuted
                volume: volumeBar.controller.monitorVolume
                muteTooltip: "Monitor muted — click to unmute"
                unmuteTooltip: "Monitor unmuted — click to mute"
                onToggleMuteRequested: volumeBar.controller.toggleMonitorMute()
                onVolumeEdited: (v) => volumeBar.controller.updateMonitorVolume(v)
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

            MuteVolumeChannel {
                Layout.fillWidth: true
                labelText: "Mic Output"
                unmutedIcon: "audio-input-microphone"
                muted: volumeBar.controller.micMuted
                volume: volumeBar.controller.micVolume
                muteTooltip: "Mic muted — click to unmute"
                unmuteTooltip: "Mic unmuted — click to mute"
                onToggleMuteRequested: volumeBar.controller.toggleMicMute()
                onVolumeEdited: (v) => volumeBar.controller.updateMicVolume(v)
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
