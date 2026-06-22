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

    readonly property bool speakerMatched: voiceController.isEnrolled
                                           && voiceController.verificationEnabled
                                           && voiceController.speakerMatchScore >= voiceController.matchThreshold

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

    EnrollmentDialog {
        id: enrollmentDialog
        controller: voiceController
        theme: voicePanel.theme
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 14

        Spectrum {
            id: spectrumView
            Layout.fillWidth: true
            Layout.preferredHeight: 180
            controller: voiceController
            theme: voicePanel.theme
            active: voiceController.isPassing
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
                Layout.preferredWidth: 100
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
                Layout.preferredWidth: 70
                text: voiceController.isSpeaking ? "Speaking" : "Silent"
                color: voiceController.isSpeaking
                       ? (voicePanel.theme ? voicePanel.theme.accent : "#6abf69")
                       : (voicePanel.theme ? voicePanel.theme.textMuted : "#888892")
            }
        }

        // Speaker-match meter (cosine similarity) with the match-threshold marker.
        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Label {
                text: "Speaker match"
                Layout.preferredWidth: 100
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
                    width: Math.max(0, (parent.width - 4) * Math.max(0, voiceController.speakerMatchScore))
                    radius: 3
                    color: voicePanel.speakerMatched
                           ? (voicePanel.theme ? voicePanel.theme.accent : "#6abf69")
                           : (voicePanel.theme ? voicePanel.theme.textMuted : "#888892")
                }
                // Match-threshold marker (follows the slider).
                Rectangle {
                    width: 2
                    height: parent.height
                    x: parent.width * voiceController.matchThreshold
                    color: voicePanel.theme ? voicePanel.theme.warningAccent : "#ffb74d"
                    opacity: 0.8
                }
            }

            Label {
                Layout.preferredWidth: 70
                text: (!voiceController.isEnrolled || !voiceController.verificationEnabled)
                      ? "Unknown"
                      : (voicePanel.speakerMatched ? "You" : "Not you")
                color: voicePanel.speakerMatched
                       ? (voicePanel.theme ? voicePanel.theme.accent : "#6abf69")
                       : (voicePanel.theme ? voicePanel.theme.textMuted : "#888892")
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.topMargin: 2
            height: 1
            color: voicePanel.theme ? voicePanel.theme.border : "#5a5a62"
            opacity: 0.5
        }

        // Enrollment status + actions.
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Label {
                text: voiceController.isEnrolled ? "Enrolled voiceprint: yes" : "Enrolled voiceprint: none"
                color: voicePanel.theme ? voicePanel.theme.textPrimary : "#ececec"
            }
            Item { Layout.fillWidth: true }
            AppButton {
                text: voiceController.isEnrolled ? "Re-enroll" : "Enroll"
                role: "primary"
                onClicked: enrollmentDialog.open()
            }
            AppButton {
                text: "Clear"
                role: "danger"
                enabled: voiceController.isEnrolled
                onClicked: voiceController.clearEnrollment()
            }
        }

        // Verification toggle + match threshold.
        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Switch {
                id: verificationSwitch
                text: "Speaker verification"
                enabled: voiceController.isEnrolled
                checked: voiceController.verificationEnabled
                palette.windowText: voicePanel.theme ? voicePanel.theme.textPrimary : "#ececec"
                onToggled: voiceController.setVerification(checked)
            }
            Item { Layout.fillWidth: true }
            Label {
                text: "Match threshold"
                color: voicePanel.theme ? voicePanel.theme.textSecondary : "#b3b3bc"
            }
            Slider {
                id: thresholdSlider
                Layout.preferredWidth: 160
                from: 0.0
                to: 1.0
                value: voiceController.matchThreshold
                onMoved: voiceController.setThreshold(value)
            }
            Label {
                Layout.preferredWidth: 36
                text: voiceController.matchThreshold.toFixed(2)
                color: voicePanel.theme ? voicePanel.theme.textPrimary : "#ececec"
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
            text: "The spectrum turns green while your voice passes the gate (speech " +
                  "detected and, when verification is on, matched to your enrolled " +
                  "voiceprint). With verification on, only matched speech is sent to " +
                  "the virtual mic and the gate keeps running in the background. " +
                  "Noise suppression arrives in a later milestone."
        }

        Item { Layout.fillHeight: true }
    }
}
