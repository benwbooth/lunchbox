import QtQuick
import QtQuick.Controls

Rectangle {
    id: pill
    required property string label
    required property string value
    property string explanation: ""
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color line: "#283244"
    implicitWidth: statusRow.implicitWidth + 24
    implicitHeight: 34
    radius: 9
    color: "#151d29"
    border.color: line
    Accessible.role: Accessible.StaticText
    Accessible.name: pill.value + " " + pill.label
    Accessible.description: pill.explanation

    HoverHandler { id: pillHover; objectName: "pillHover" }
    ToolTip.visible: pillHover.hovered && pill.explanation.length > 0
    ToolTip.delay: 500
    ToolTip.text: pill.explanation

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
