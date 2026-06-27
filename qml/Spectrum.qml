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
    readonly property real slotWidth: Math.max(
        1,
        (chartArea.width - (barCount + 1) * barGap) / barCount)

    function segmentDbAt(index) {
        return controller.spectrumSegmentDbAt(index)
    }

    function segmentYFracAt(index) {
        return controller.spectrumSegmentYFracAt(index)
    }

    function segmentYOffset(index, chartH) {
        var usable = chartH - (segmentCount - 1) * segmentGap
        var above = 0
        for (var j = 0; j < index; j++)
            above += segmentYFracAt(j) * usable + segmentGap
        return chartH - above - segmentYFracAt(index) * usable
    }

    function segmentPixelHeight(index, chartH) {
        var usable = chartH - (segmentCount - 1) * segmentGap
        return Math.max(1, segmentYFracAt(index) * usable)
    }

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

    function barLevelAt(index) {
        var _ = version
        return controller.barLevelAt(index)
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
            delegate: Item {
                required property int index
                readonly property real level: spectrum.barLevelAt(index)
                readonly property int litCount: controller.litSegmentCountAt(level)

                x: spectrum.barGap + index * (spectrum.slotWidth + spectrum.barGap)
                width: spectrum.slotWidth
                height: chartArea.height

                Repeater {
                    model: spectrum.segmentCount
                    delegate: Item {
                        required property int index
                        readonly property real dbTick: spectrum.segmentDbAt(index)
                        readonly property bool isLit: index < litCount

                        y: spectrum.segmentYOffset(index, chartArea.height)
                        width: parent.width
                        height: spectrum.segmentPixelHeight(index, chartArea.height)

                        Rectangle {
                            anchors.fill: parent
                            radius: Math.min(2, width / 3)
                            color: spectrum.ghostColor(dbTick)
                            opacity: 0.14
                        }

                        Rectangle {
                            anchors.fill: parent
                            radius: Math.min(2, width / 3)
                            color: spectrum.segmentColor(dbTick)
                            opacity: isLit ? (spectrum.active ? 1.0 : 0.45) : 0
                        }
                    }
                }
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
                required property int index
                required property var modelData
                readonly property int barStart: {
                    var n = 0
                    for (var b = 0; b < index; b++)
                        n += spectrum.bands[b].subdivisions
                    return n
                }

                x: spectrum.barGap + barStart * (spectrum.slotWidth + spectrum.barGap)
                width: modelData.subdivisions * spectrum.slotWidth
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
