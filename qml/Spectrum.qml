import QtQuick

Item {
    id: spectrum

    // The VoiceController instance providing spectrum data.
    required property var controller
    property var theme
    // When false (gate not passing), bars are drawn at reduced opacity.
    property bool active: true

    // Log-frequency span must match services/voice/spectrum.rs (FREQ_MIN..Nyquist).
    readonly property real freqMin: 20.0
    readonly property real freqMax: 24000.0

    readonly property color colorGreen: theme ? theme.accent : "#6abf69"
    readonly property color colorYellow: theme ? theme.warningAccent : "#ffb74d"
    readonly property color colorRed: theme ? theme.danger : "#c62828"

    readonly property int binCount: controller.spectrumBinCount()
    readonly property real barGap: 1
    readonly property real barWidth: binCount > 0
        ? Math.max(1, (width - 8 - barGap * (binCount - 1)) / binCount) : 1

    // Repaint binding for bar level delegates.
    readonly property int version: controller.spectrumVersion

    function freqFraction(freq) {
        var f = Math.max(freqMin, Math.min(freqMax, freq))
        return Math.log(f / freqMin) / Math.log(freqMax / freqMin)
    }

    function mixColors(c1, c2, u) {
        return Qt.rgba(
            c1.r + (c2.r - c1.r) * u,
            c1.g + (c2.g - c1.g) * u,
            c1.b + (c2.b - c1.b) * u,
            1)
    }

    // Map normalized amplitude (0..1) to green → yellow → red.
    function amplitudeColor(amplitude) {
        var t = Math.max(0, Math.min(1, amplitude))
        if (t < 0.5)
            return mixColors(colorGreen, colorYellow, t * 2)
        return mixColors(colorYellow, colorRed, (t - 0.5) * 2)
    }

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: "#101013"
        border.color: spectrum.theme ? spectrum.theme.border : "#5a5a62"
        border.width: 1
    }

    Repeater {
        model: spectrum.binCount
        delegate: Rectangle {
            required property int index
            property real level: {
                var _ = spectrum.version
                return spectrum.controller.spectrumValueAt(index)
            }
            x: 4 + index * (spectrum.barWidth + spectrum.barGap)
            width: spectrum.barWidth
            height: Math.max(1, level * (spectrum.height - 8))
            y: spectrum.height - 4 - height
            radius: Math.min(2, spectrum.barWidth / 2)
            color: spectrum.amplitudeColor(level)
            opacity: spectrum.active ? 1.0 : 0.45
        }
    }

    // Band markers: sub-100 Hz, speech band, sibilance/clicks.
    Repeater {
        model: [
            { freq: 100, label: "100 Hz" },
            { freq: 4000, label: "4 kHz" }
        ]
        delegate: Item {
            x: spectrum.freqFraction(modelData.freq) * spectrum.width
            width: 1
            height: spectrum.height

            Rectangle {
                width: 1
                height: parent.height
                color: spectrum.theme ? spectrum.theme.border : "#5a5a62"
                opacity: 0.35
            }
            Text {
                x: 4
                y: 4
                text: modelData.label
                font.pixelSize: 10
                color: spectrum.theme ? spectrum.theme.textMuted : "#888892"
            }
        }
    }
}
