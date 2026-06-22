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

    // Display scaling: idle room noise maps ~0.45 raw → green after floor + gamma.
    readonly property real noiseFloor: 0.38
    readonly property real displayGamma: 1.6

    readonly property int chartMargin: 4
    readonly property int labelHeight: 20
    readonly property real barGap: 4
    readonly property int barCount: 21

    readonly property var bands: [
        { label: "Sub-bass", minHz: 20, maxHz: 60, subdivisions: 3 },
        { label: "Bass", minHz: 60, maxHz: 250, subdivisions: 3 },
        { label: "Low-mid", minHz: 250, maxHz: 500, subdivisions: 3 },
        { label: "Mid", minHz: 500, maxHz: 2000, subdivisions: 3 },
        { label: "High-mid", minHz: 2000, maxHz: 4000, subdivisions: 3 },
        { label: "Presence", minHz: 4000, maxHz: 6000, subdivisions: 3 },
        { label: "Brilliance", minHz: 6000, maxHz: 24000, subdivisions: 3 }
    ]

    readonly property int version: controller.spectrumVersion
    readonly property real chartHeight: Math.max(1, height - labelHeight - chartMargin * 2)

    function freqFraction(freq) {
        var f = Math.max(freqMin, Math.min(freqMax, freq))
        return Math.log(f / freqMin) / Math.log(freqMax / freqMin)
    }

    function freqFromFraction(t) {
        var clamped = Math.max(0, Math.min(1, t))
        return freqMin * Math.pow(freqMax / freqMin, clamped)
    }

    function displayLevel(raw) {
        var t = Math.max(0, (raw - noiseFloor) / (1 - noiseFloor))
        return Math.min(1, Math.pow(t, displayGamma))
    }

    function mixColors(c1, c2, u) {
        return Qt.rgba(
            c1.r + (c2.r - c1.r) * u,
            c1.g + (c2.g - c1.g) * u,
            c1.b + (c2.b - c1.b) * u,
            1)
    }

    function amplitudeColor(amplitude) {
        var t = Math.max(0, Math.min(1, amplitude))
        if (t < 0.5)
            return mixColors(colorGreen, colorYellow, t * 2)
        return mixColors(colorYellow, colorRed, (t - 0.5) * 2)
    }

    function barDescriptor(index) {
        var n = 0
        for (var b = 0; b < bands.length; b++) {
            var sub = bands[b].subdivisions
            if (index < n + sub)
                return { bandIndex: b, subIndex: index - n }
            n += sub
        }
        return { bandIndex: 0, subIndex: 0 }
    }

    function subBarRange(bandIndex, subIndex) {
        var b = bands[bandIndex]
        var t0 = freqFraction(b.minHz)
        var t1 = freqFraction(b.maxHz)
        var w = (t1 - t0) / b.subdivisions
        return { t0: t0 + subIndex * w, t1: t0 + (subIndex + 1) * w }
    }

    function subBarLevel(bandIndex, subIndex) {
        var _ = version
        var range = subBarRange(bandIndex, subIndex)
        var hz0 = freqFromFraction(range.t0)
        var hz1 = freqFromFraction(range.t1)
        var bins = controller.spectrumBinCount()
        var peak = 0
        for (var i = 0; i < bins; i++) {
            var t = i / Math.max(1, bins - 1)
            var hz = freqFromFraction(t)
            if (hz >= hz0 && hz < hz1)
                peak = Math.max(peak, controller.spectrumValueAt(i))
        }
        return displayLevel(peak)
    }

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: "#101013"
        border.color: spectrum.theme ? spectrum.theme.border : "#5a5a62"
        border.width: 1
    }

    Item {
        id: chartArea
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: chartMargin
        height: chartHeight

        Repeater {
            model: barCount
            delegate: Rectangle {
                required property int index
                property var desc: spectrum.barDescriptor(index)
                property var range: spectrum.subBarRange(desc.bandIndex, desc.subIndex)
                property real level: spectrum.subBarLevel(desc.bandIndex, desc.subIndex)

                readonly property real slotLeft: range.t0 * chartArea.width
                readonly property real slotWidth: (range.t1 - range.t0) * chartArea.width

                x: slotLeft + spectrum.barGap / 2
                width: Math.max(1, slotWidth - spectrum.barGap)
                height: Math.max(level > 0.02 ? 1 : 0, level * chartArea.height)
                y: chartArea.height - height
                radius: Math.min(3, width / 2)
                color: spectrum.amplitudeColor(level)
                opacity: spectrum.active ? 1.0 : 0.45
                visible: level > 0.02
            }
        }
    }

    Item {
        id: labelRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: chartMargin
        height: labelHeight

        Repeater {
            model: bands
            delegate: Text {
                required property var modelData
                readonly property real x0: spectrum.freqFraction(modelData.minHz) * labelRow.width
                readonly property real x1: spectrum.freqFraction(modelData.maxHz) * labelRow.width

                x: x0
                width: Math.max(1, x1 - x0)
                anchors.bottom: parent.bottom
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignBottom
                text: modelData.label
                font.pixelSize: 9
                color: spectrum.theme ? spectrum.theme.textMuted : "#888892"
                elide: Text.ElideRight
            }
        }
    }
}
