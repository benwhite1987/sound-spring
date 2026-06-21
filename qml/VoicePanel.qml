import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.benkahn.soundboard

Item {
    id: voicePanel

    property var theme

    VoiceController {
        id: voiceController
    }

    // Capture only runs while this panel is the active page; switching away
    // tears the pw-cat session down.
    onVisibleChanged: voiceController.setVisualizationActive(visible)
    Component.onCompleted: if (visible) voiceController.setVisualizationActive(true)

    Timer {
        interval: 33
        running: voicePanel.visible
        repeat: true
        onTriggered: voiceController.processSpectrum()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 16

        Spectrum {
            id: spectrumView
            Layout.fillWidth: true
            Layout.preferredHeight: 200
            controller: voiceController
            theme: voicePanel.theme
            active: voiceController.isSpeaking
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Rectangle {
                width: 10
                height: 10
                radius: 5
                color: {
                    voiceController.spectrumVersion
                    return voiceController.isCapturing
                           ? (voicePanel.theme ? voicePanel.theme.accent : "#6abf69")
                           : (voicePanel.theme ? voicePanel.theme.textMuted : "#888892")
                }
            }
            Label {
                text: {
                    voiceController.spectrumVersion
                    return voiceController.isCapturing ? "Listening to microphone" : "Capture idle"
                }
                color: voicePanel.theme ? voicePanel.theme.textPrimary : "#ececec"
            }
            Item { Layout.fillWidth: true }
        }

        // Voice activity detection meter with the open-threshold marker.
        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Label {
                text: "Voice activity"
                Layout.preferredWidth: 90
                color: voicePanel.theme ? voicePanel.theme.textSecondary : "#b3b3bc"
            }

            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 16

                Rectangle {
                    anchors.fill: parent
                    radius: 4
                    color: voicePanel.theme ? voicePanel.theme.surface : "#333338"
                    border.color: voicePanel.theme ? voicePanel.theme.border : "#5a5a62"
                    border.width: 1
                }
                Rectangle {
                    height: parent.height - 4
                    y: 2
                    x: 2
                    width: Math.max(0, (parent.width - 4) * voiceController.vadProbability)
                    radius: 3
                    color: voiceController.isSpeaking
                           ? (voicePanel.theme ? voicePanel.theme.accent : "#6abf69")
                           : (voicePanel.theme ? voicePanel.theme.textMuted : "#888892")
                }
                // Open-threshold marker (matches [voice] vad_open_threshold default 0.7).
                Rectangle {
                    width: 2
                    height: parent.height
                    x: parent.width * 0.7
                    color: voicePanel.theme ? voicePanel.theme.warningAccent : "#ffb74d"
                    opacity: 0.8
                }
            }

            Label {
                Layout.preferredWidth: 64
                text: voiceController.isSpeaking ? "Speaking" : "Silent"
                color: voiceController.isSpeaking
                       ? (voicePanel.theme ? voicePanel.theme.accent : "#6abf69")
                       : (voicePanel.theme ? voicePanel.theme.textMuted : "#888892")
            }
        }

        Label {
            Layout.fillWidth: true
            text: "Routing to: soundboard_virtmic"
            color: voicePanel.theme ? voicePanel.theme.textSecondary : "#b3b3bc"
        }

        AppButton {
            text: "Open in pavucontrol"
            onClicked: voiceController.openPavucontrol()
        }

        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            font.pixelSize: 12
            color: voicePanel.theme ? voicePanel.theme.textMuted : "#888892"
            text: "Speaker verification and noise suppression arrive in a later " +
                  "milestone. This panel shows the live microphone spectrum and " +
                  "voice activity detection; the spectrum turns green while speech " +
                  "is detected."
        }

        Item { Layout.fillHeight: true }
    }
}
