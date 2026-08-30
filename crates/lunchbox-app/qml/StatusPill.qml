import QtQuick

Rectangle {
    id: pill
    required property string label
    required property string value
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color line: "#283244"
    implicitWidth: statusRow.implicitWidth + 24
    implicitHeight: 34
    radius: 9
    color: "#151d29"
    border.color: line

    Row {
        id: statusRow
        anchors.centerIn: parent
        spacing: 7
        Text {
            text: pill.value
            color: pill.ink
            font.pixelSize: 13
            font.weight: Font.Bold
        }
        Text {
            text: pill.label
            color: pill.muted
            font.pixelSize: 12
        }
    }
}
