import QtQuick
import QtQuick.Controls

Button {
    id: control
    property bool active: false
    property color accent: "#ffb454"
    property color line: "#283244"
    property color ink: "#f4f7fb"
    implicitHeight: 38
    implicitWidth: Math.max(92, contentItem.implicitWidth + leftPadding + rightPadding)
    clip: true
    leftPadding: 16
    rightPadding: 16
    font.pixelSize: 13
    font.weight: Font.DemiBold

    background: Rectangle {
        radius: 9
        color: control.down ? "#303b4d"
                            : control.active ? "#34303a" : "#1b2330"
        border.color: control.active ? control.accent : control.line
    }
    contentItem: Text {
        width: control.availableWidth
        text: control.text
        color: control.active ? control.accent : control.ink
        font: control.font
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        maximumLineCount: 1
        clip: true
    }
}
