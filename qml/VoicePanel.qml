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
            text: "Voice activity detection, speaker verification, and noise " +
                  "suppression arrive in a later milestone. This panel currently " +
                  "shows the live microphone spectrum."
        }

        Item { Layout.fillHeight: true }
    }
}
