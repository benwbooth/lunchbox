import QtQuick

Rectangle {
    id: metric
    required property string label
    required property string value
    property color tone: "#f4f7fb"
    property color muted: "#8d99aa"
    width: 112
    height: 56
    radius: 9
    color: "#151d29"
    border.color: Qt.rgba(tone.r, tone.g, tone.b, 0.28)
    Accessible.name: label + ": " + value

    Column {
        anchors.left: parent.left
        anchors.leftMargin: 13
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        spacing: 1
        Text {
            width: parent.width
            text: metric.value
            color: metric.tone
            font.pixelSize: 17
            font.weight: Font.Bold
            font.features: { "tnum": 1 }
            elide: Text.ElideRight
        }
        Text {
            width: parent.width
            text: metric.label.toUpperCase()
            color: metric.muted
            font.pixelSize: 8
            font.weight: Font.Bold
            font.letterSpacing: 0.75
            elide: Text.ElideRight
        }
    }
}
