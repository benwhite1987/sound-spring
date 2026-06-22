import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.benkahn.soundboard

Item {
    id: voicePanel

    required property SoundboardController controller
    property var theme

    readonly property color textPrimary: theme ? theme.textPrimary : "#ececec"
    readonly property color textSecondary: theme ? theme.textSecondary : "#b3b3bc"
    readonly property color textMuted: theme ? theme.textMuted : "#888892"
    readonly property color accent: theme ? theme.accent : "#6abf69"
    readonly property color danger: theme ? theme.danger : "#c62828"
    readonly property color infoAccent: theme ? theme.infoAccent : "#5b9bd5"
    readonly property color surface: theme ? theme.surface : "#333338"
    readonly property color border: theme ? theme.border : "#5a5a62"
    readonly property color warningAccent: theme ? theme.warningAccent : "#ffb74d"
    readonly property int meterLabelWidth: 96
    readonly property int contentMaxWidth: 720

    function activeFilterNames() {
        var names = []
        if (voiceController.vadEnabled)
            names.push("VAD")
        if (voiceController.verificationEnabled && voiceController.isEnrolled)
            names.push("voiceprint")
        if (voiceController.suppressionEnabled)
            names.push("denoiser")
        return names
    }

    function captureStatusColor() {
        voiceController.spectrumVersion
        voicePanel.controller.uiVersion
        var err = voiceController.captureError
        if (err && err.length > 0)
            return voicePanel.danger
        if (voicePanel.controller.micMuted)
            return voicePanel.danger
        if (!voiceController.isCapturing)
            return voicePanel.textMuted
        if (activeFilterNames().length > 0)
            return voicePanel.infoAccent
        return voicePanel.accent
    }

    function captureStatusText() {
        voiceController.spectrumVersion
        voicePanel.controller.uiVersion
        var err = voiceController.captureError
        if (err && err.length > 0)
            return err
        if (voicePanel.controller.micMuted)
            return "Microphone muted"
        if (!voiceController.isCapturing)
            return "Capture idle"
        var filters = activeFilterNames()
        if (filters.length > 0)
            return "Listening to microphone / Filtering with " + filters.join(", ")
        return "Listening to microphone"
    }

    VoiceController {
        id: voiceController
    }

    readonly property bool speakerMatched: voiceController.isEnrolled
                                           && voiceController.verificationEnabled
                                           && voiceController.speakerMatchScore >= voiceController.matchThreshold

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
        spacing: 8

        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: pinnedColumn.implicitHeight

            ColumnLayout {
                id: pinnedColumn
                width: Math.min(parent.width, voicePanel.contentMaxWidth)
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: 6

                Flow {
                    Layout.fillWidth: true
                    spacing: 8

                    Label {
                        text: "Spectrum source"
                        color: voicePanel.textSecondary
                        height: 28
                        verticalAlignment: Text.AlignVCenter
                    }

                    ButtonGroup { id: spectrumSourceGroup }

                    RadioButton {
                        text: "Raw mic"
                        height: 28
                        ButtonGroup.group: spectrumSourceGroup
                        checked: voiceController.spectrumSource === "raw"
                        onClicked: voiceController.persistSpectrumSource("raw")
                    }
                    RadioButton {
                        text: "Filtered"
                        height: 28
                        ButtonGroup.group: spectrumSourceGroup
                        checked: voiceController.spectrumSource === "filtered"
                        onClicked: voiceController.persistSpectrumSource("filtered")
                    }
                    RadioButton {
                        text: "Mixed"
                        height: 28
                        ButtonGroup.group: spectrumSourceGroup
                        checked: voiceController.spectrumSource === "mixed"
                        onClicked: voiceController.persistSpectrumSource("mixed")
                    }
                }

                Spectrum {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 240
                    controller: voiceController
                    theme: voicePanel.theme
                    active: voiceController.isPassing
                }
            }
        }

        ScrollView {
            id: settingsScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            contentWidth: availableWidth

            Item {
                width: settingsScroll.availableWidth
                implicitHeight: settingsColumn.implicitHeight + 16

                ColumnLayout {
                    id: settingsColumn
                    width: Math.min(parent.width, voicePanel.contentMaxWidth)
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: 2

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Rectangle {
                            width: 8
                            height: 8
                            radius: 4
                            color: voicePanel.captureStatusColor()
                        }
                        Label {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            font.pixelSize: 12
                            text: voicePanel.captureStatusText()
                            color: {
                                voiceController.spectrumVersion
                                voicePanel.controller.uiVersion
                                var err = voiceController.captureError
                                if (err && err.length > 0)
                                    return voicePanel.danger
                                return voicePanel.textPrimary
                            }
                        }
                        AppButton {
                            text: voicePanel.controller.micMuted ? "Unmute mic" : "Mute mic"
                            role: voicePanel.controller.micMuted ? "danger" : "secondary"
                            onClicked: voicePanel.controller.toggleMicMute()
                        }
                    }

                    SettingsSection {
                        title: "Voice activity"

                        Switch {
                            Layout.fillWidth: true
                            text: "Voice activity detection"
                            checked: voiceController.vadEnabled
                            palette.windowText: voicePanel.textPrimary
                            onToggled: voiceController.persistVadEnabled(checked)
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            opacity: voiceController.vadEnabled ? 1.0 : 0.4
                            enabled: voiceController.vadEnabled

                            Label {
                                text: "Level"
                                Layout.preferredWidth: voicePanel.meterLabelWidth
                                color: voicePanel.textSecondary
                                font.pixelSize: 12
                            }

                            Item {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 14

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 3
                                    color: voicePanel.surface
                                    border.color: voicePanel.border
                                    border.width: 1
                                }
                                Rectangle {
                                    height: parent.height - 4
                                    y: 2
                                    x: 2
                                    width: Math.max(0, (parent.width - 4) * voiceController.vadProbability)
                                    radius: 2
                                    color: voiceController.isSpeaking ? voicePanel.accent : voicePanel.textMuted
                                }
                                Rectangle {
                                    width: 2
                                    height: parent.height
                                    x: parent.width * voiceController.vadOpenThreshold
                                    color: voicePanel.warningAccent
                                    opacity: 0.8
                                }
                            }

                            Label {
                                Layout.preferredWidth: 32
                                font.pixelSize: 12
                                text: voiceController.vadOpenThreshold.toFixed(2)
                                color: voicePanel.textPrimary
                            }

                            Slider {
                                Layout.preferredWidth: 100
                                from: 0.05
                                to: 0.95
                                value: voiceController.vadOpenThreshold
                                onMoved: voiceController.setVadThreshold(value)
                            }

                            Label {
                                Layout.preferredWidth: 56
                                font.pixelSize: 12
                                text: voiceController.isSpeaking ? "Speaking" : "Silent"
                                color: voiceController.isSpeaking ? voicePanel.accent : voicePanel.textMuted
                            }
                        }
                    }

                    SettingsSection {
                        title: "Speaker identity"

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Label {
                                text: "Match"
                                Layout.preferredWidth: voicePanel.meterLabelWidth
                                color: voicePanel.textSecondary
                                font.pixelSize: 12
                            }

                            Item {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 14

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 3
                                    color: voicePanel.surface
                                    border.color: voicePanel.border
                                    border.width: 1
                                }
                                Rectangle {
                                    height: parent.height - 4
                                    y: 2
                                    x: 2
                                    width: Math.max(0, (parent.width - 4) * Math.max(0, voiceController.speakerMatchScore))
                                    radius: 2
                                    color: voicePanel.speakerMatched ? voicePanel.accent : voicePanel.textMuted
                                }
                                Rectangle {
                                    width: 2
                                    height: parent.height
                                    x: parent.width * voiceController.matchThreshold
                                    color: voicePanel.warningAccent
                                    opacity: 0.8
                                }
                            }

                            Label {
                                Layout.preferredWidth: 56
                                font.pixelSize: 12
                                text: (!voiceController.isEnrolled || !voiceController.verificationEnabled)
                                      ? "Unknown"
                                      : (voicePanel.speakerMatched ? "You" : "Not you")
                                color: voicePanel.speakerMatched ? voicePanel.accent : voicePanel.textMuted
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Label {
                                Layout.fillWidth: true
                                font.pixelSize: 12
                                text: voiceController.isEnrolled
                                      ? "Voiceprint enrolled"
                                      : "No voiceprint"
                                color: voicePanel.textPrimary
                            }
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

                        Switch {
                            Layout.fillWidth: true
                            text: "Speaker verification"
                            enabled: voiceController.isEnrolled
                            checked: voiceController.verificationEnabled
                            palette.windowText: voicePanel.textPrimary
                            onToggled: voiceController.setVerification(checked)
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            enabled: voiceController.isEnrolled
                            opacity: voiceController.isEnrolled ? 1.0 : 0.4

                            Label {
                                text: "Threshold"
                                Layout.preferredWidth: voicePanel.meterLabelWidth
                                color: voicePanel.textSecondary
                                font.pixelSize: 12
                            }

                            Slider {
                                Layout.fillWidth: true
                                from: 0.0
                                to: 1.0
                                value: voiceController.matchThreshold
                                onMoved: voiceController.setThreshold(value)
                            }

                            Label {
                                Layout.preferredWidth: 32
                                font.pixelSize: 12
                                text: voiceController.matchThreshold.toFixed(2)
                                color: voicePanel.textPrimary
                            }
                        }
                    }

                    SettingsSection {
                        title: "Routing & output"
                        description: "Filtered mic plus soundboard on the mixed view. Verification and suppression run in the background when enabled."

                        Switch {
                            Layout.fillWidth: true
                            text: "Noise suppression (DeepFilterNet3)"
                            checked: voiceController.suppressionEnabled
                            palette.windowText: voicePanel.textPrimary
                            onToggled: voiceController.setSuppression(checked)
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Label {
                                Layout.fillWidth: true
                                font.pixelSize: 12
                                text: "Routing to: soundboard_virtmic"
                                color: voicePanel.textSecondary
                            }
                            AppButton {
                                text: "Open in pavucontrol"
                                onClicked: voiceController.openPavucontrol()
                            }
                        }
                    }
                }
            }
        }
    }
}
