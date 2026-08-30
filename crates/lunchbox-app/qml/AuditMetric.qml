import QtQuick

Rectangle {
    id: metric
    required property string label
    required property int value
    required property color tone
    property color muted: "#8d99aa"
    implicitHeight: 68
    radius: 10
    color: "#151d29"
    border.color: Qt.rgba(metric.tone.r, metric.tone.g, metric.tone.b, 0.38)

    Column {
        anchors.left: parent.left
        anchors.leftMargin: 15
        anchors.verticalCenter: parent.verticalCenter
        spacing: 1
        Text {
            text: metric.value.toLocaleString(Qt.locale(), "f", 0)
            color: metric.tone
            font.pixelSize: 20
            font.weight: Font.Bold
        }
        Text {
            text: metric.label.toUpperCase()
            color: metric.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
    }
}
