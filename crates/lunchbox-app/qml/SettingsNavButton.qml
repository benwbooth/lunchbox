import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

LbButton {
    id: control
    property bool active: false
    property color accent: "#ffb454"
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"

    Layout.fillWidth: true
    implicitHeight: 42
    leftPadding: 14
    rightPadding: 12
    flat: true
    highlighted: active
    font.pixelSize: 12
    font.weight: active ? Font.DemiBold : Font.Normal

    contentItem: Text {
        width: control.availableWidth
        text: control.text
        color: control.active ? control.accent : control.muted
        font: control.font
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignLeft
        elide: Text.ElideRight
        maximumLineCount: 1
        clip: true
    }
}
