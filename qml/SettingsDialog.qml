import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.benkahn.soundboard

Dialog {
    id: root
    title: "Settings"
    modal: true
    standardButtons: Dialog.Apply | Dialog.Close

    property Settings settings

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        Label { text: "Audio settings (stub)" }
        TextField {
            Layout.fillWidth: true
            placeholderText: "Microphone source"
            text: settings ? settings.micSource : ""
            onTextChanged: if (settings) settings.micSource = text
        }
        SpinBox {
            from: 10
            to: 100
            value: settings ? settings.latencyMs : 20
            onValueChanged: if (settings) settings.latencyMs = value
        }
    }

    onApplied: if (settings) settings.apply()
}
