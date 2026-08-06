import QtQuick

Item {
    id: spectrum

    required property var controller
    property var theme
    property bool active: true

    readonly property color colorGreen: theme ? theme.accent : "#6abf69"
    readonly property color colorYellow: theme ? theme.warningAccent : "#ffb74d"
    readonly property color colorRed: theme ? theme.danger : "#c62828"
    readonly property color ghostGreen: Qt.darker(colorGreen, 2.8)
    readonly property color ghostRed: Qt.darker(colorRed, 2.8)

    readonly property int chartMargin: 4
    readonly property int labelHeight: 20
    readonly property real barGap: 3
    readonly property real segmentGap: 2
    readonly property int barCount: controller.spectrumBarCount
    readonly property int segmentCount: controller.spectrumSegmentCount

    readonly property var bands: [
        { label: "Sub-bass", subdivisions: 3 },
        { label: "Bass", subdivisions: 3 },
        { label: "Low-mid", subdivisions: 3 },
        { label: "Mid", subdivisions: 3 },
        { label: "High-mid", subdivisions: 3 },
        { label: "Presence", subdivisions: 3 },
        { label: "Brilliance", subdivisions: 3 }
    ]

    readonly property int version: controller.spectrumVersion
    readonly property real chartHeight: Math.max(1, height - labelHeight - chartMargin * 2)

    onVersionChanged: canvas.requestPaint()
    onWidthChanged: canvas.requestPaint()
    onHeightChanged: canvas.requestPaint()
    onActiveChanged: canvas.requestPaint()

    function segmentColor(dbTick) {
        if (dbTick <= -2)
            return colorGreen
        if (dbTick === 0)
            return colorYellow
        return colorRed
    }

    function ghostColor(dbTick) {
        return dbTick <= -2 ? ghostGreen : ghostRed
    }

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: "#101013"
        border.color: spectrum.theme ? spectrum.theme.border : "#5a5a62"
        border.width: 1
    }

    Canvas {
        id: canvas
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: chartMargin
        height: chartHeight

        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()
            ctx.clearRect(0, 0, width, height)

            var bars = spectrum.barCount
            var segs = spectrum.segmentCount
            if (bars <= 0 || segs <= 0 || width <= 0 || height <= 0)
                return

            var slot = Math.max(1, (width - (bars + 1) * spectrum.barGap) / bars)
            var usable = height - (segs - 1) * spectrum.segmentGap

            for (var b = 0; b < bars; b++) {
                var level = spectrum.controller.barLevelAt(b)
                var lit = spectrum.controller.litSegmentCountAt(level)
                var x = spectrum.barGap + b * (slot + spectrum.barGap)
                var above = 0
                for (var s = 0; s < segs; s++) {
                    var yFrac = spectrum.controller.spectrumSegmentYFracAt(s)
                    var segH = Math.max(1, yFrac * usable)
                    var y = height - above - segH
                    above += segH + spectrum.segmentGap
                    var dbTick = spectrum.controller.spectrumSegmentDbAt(s)

                    ctx.globalAlpha = 0.14
                    ctx.fillStyle = spectrum.ghostColor(dbTick)
                    ctx.fillRect(x, y, slot, segH)

                    if (s < lit) {
                        ctx.globalAlpha = spectrum.active ? 1.0 : 0.45
                        ctx.fillStyle = spectrum.segmentColor(dbTick)
                        ctx.fillRect(x, y, slot, segH)
                    }
                }
            }
            ctx.globalAlpha = 1.0
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
                required property int index
                required property var modelData
                readonly property int barStart: {
                    var n = 0
                    for (var b = 0; b < index; b++)
                        n += spectrum.bands[b].subdivisions
                    return n
                }
                readonly property real slotWidth: Math.max(
                    1,
                    (canvas.width - (spectrum.barCount + 1) * spectrum.barGap) / spectrum.barCount)

                x: spectrum.barGap + barStart * (slotWidth + spectrum.barGap)
                width: modelData.subdivisions * slotWidth
                    + (modelData.subdivisions - 1) * spectrum.barGap
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
