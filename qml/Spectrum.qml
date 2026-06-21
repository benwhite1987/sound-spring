import QtQuick

Item {
    id: spectrum

    // The VoiceController instance providing spectrum data.
    required property var controller
    property var theme

    // Log-frequency span must match services/voice/spectrum.rs (FREQ_MIN..Nyquist).
    readonly property real freqMin: 20.0
    readonly property real freqMax: 24000.0

    function freqFraction(freq) {
        var f = Math.max(freqMin, Math.min(freqMax, freq))
        return Math.log(f / freqMin) / Math.log(freqMax / freqMin)
    }

    // Repaint whenever a new frame lands.
    readonly property int version: controller.spectrumVersion
    onVersionChanged: curve.requestPaint()
    onWidthChanged: curve.requestPaint()
    onHeightChanged: curve.requestPaint()

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: "#101013"
        border.color: spectrum.theme ? spectrum.theme.border : "#5a5a62"
        border.width: 1
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

    Canvas {
        id: curve
        anchors.fill: parent

        onPaint: {
            var ctx = getContext("2d")
            ctx.clearRect(0, 0, width, height)

            var bins = spectrum.controller.spectrumBinCount()
            if (bins <= 1)
                return

            var accent = spectrum.theme ? spectrum.theme.accent : "#6abf69"
            var w = width
            var h = height

            ctx.beginPath()
            ctx.moveTo(0, h)
            for (var i = 0; i < bins; i++) {
                var value = spectrum.controller.spectrumValueAt(i)
                var x = (i / (bins - 1)) * w
                var y = h - value * h
                ctx.lineTo(x, y)
            }
            ctx.lineTo(w, h)
            ctx.closePath()

            ctx.fillStyle = Qt.rgba(0.42, 0.75, 0.41, 0.28)
            ctx.fill()

            ctx.beginPath()
            for (var j = 0; j < bins; j++) {
                var v = spectrum.controller.spectrumValueAt(j)
                var px = (j / (bins - 1)) * w
                var py = h - v * h
                if (j === 0)
                    ctx.moveTo(px, py)
                else
                    ctx.lineTo(px, py)
            }
            ctx.lineWidth = 1.5
            ctx.strokeStyle = accent
            ctx.stroke()
        }
    }
}
