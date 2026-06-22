import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.benkahn.soundboard

Dialog {
    id: dlg

    // The shared VoiceController instance from the Voice panel.
    required property var controller
    property var theme

    title: "Enroll your voice"
    modal: true
    anchors.centerIn: Overlay.overlay
    width: 560
    closePolicy: Popup.NoAutoClose
    standardButtons: Dialog.NoButton

    readonly property bool recording: controller.enrollActive
    readonly property int secondsLeft: Math.max(0, Math.ceil(30 * (1 - controller.enrollProgress)))

    background: Rectangle {
        radius: 8
        color: dlg.theme ? dlg.theme.windowBg : "#1b1b1f"
        border.color: dlg.theme ? dlg.theme.border : "#5a5a62"
        border.width: 1
    }

    header: Label {
        text: dlg.title
        padding: 14
        font.pixelSize: 16
        font.weight: Font.DemiBold
        color: dlg.theme ? dlg.theme.textPrimary : "#ececec"
    }

    Connections {
        target: dlg.controller
        function onEnrollmentComplete() {
            dlg.close()
        }
    }

    onClosed: if (controller.enrollActive) controller.cancelEnrollment()

    contentItem: ColumnLayout {
        spacing: 14

        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: "Read the passage below for 30 seconds. Read naturally, at normal " +
                  "volume, sitting in your usual gaming position."
            color: dlg.theme ? dlg.theme.textSecondary : "#b3b3bc"
        }

        Rectangle {
            Layout.fillWidth: true
            radius: 6
            color: dlg.theme ? dlg.theme.surface : "#333338"
            border.color: dlg.theme ? dlg.theme.border : "#5a5a62"
            border.width: 1
            implicitHeight: passage.implicitHeight + 20

            Label {
                id: passage
                anchors.fill: parent
                anchors.margins: 10
                wrapMode: Text.WordWrap
                lineHeight: 1.15
                color: dlg.theme ? dlg.theme.textPrimary : "#ececec"
                text: "When the sunlight strikes raindrops in the air, they act as a " +
                      "prism and form a rainbow. The rainbow is a division of white " +
                      "light into many beautiful colors. These take the shape of a long " +
                      "round arch, with its path high above, and its two ends apparently " +
                      "beyond the horizon. There is, according to legend, a boiling pot " +
                      "of gold at one end."
            }
        }

        Spectrum {
            Layout.fillWidth: true
            Layout.preferredHeight: 120
            controller: dlg.controller
            theme: dlg.theme
            active: dlg.controller.isSpeaking
        }

        // Progress + countdown while recording.
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 6
            visible: dlg.recording

            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: "Recording…"
                    color: dlg.theme ? dlg.theme.accent : "#6abf69"
                }
                Item { Layout.fillWidth: true }
                Label {
                    text: dlg.secondsLeft + "s left"
                    color: dlg.theme ? dlg.theme.textSecondary : "#b3b3bc"
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 10
                Rectangle {
                    anchors.fill: parent
                    radius: 5
                    color: dlg.theme ? dlg.theme.surface : "#333338"
                    border.color: dlg.theme ? dlg.theme.border : "#5a5a62"
                    border.width: 1
                }
                Rectangle {
                    height: parent.height - 4
                    y: 2
                    x: 2
                    width: Math.max(0, (parent.width - 4) * dlg.controller.enrollProgress)
                    radius: 4
                    color: dlg.theme ? dlg.theme.progressFill : "#4caf50"
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 4
            spacing: 8

            Item { Layout.fillWidth: true }

            AppButton {
                text: "Cancel"
                onClicked: {
                    dlg.controller.cancelEnrollment()
                    dlg.close()
                }
            }

            AppButton {
                text: dlg.recording ? "Recording…" : "Start recording"
                role: "primary"
                enabled: !dlg.recording
                onClicked: dlg.controller.startEnrollment()
            }
        }
    }
}
