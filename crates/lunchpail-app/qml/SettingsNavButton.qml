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
    font.pixelSize: 13
    font.weight: Font.Medium

    contentItem: Text {
        anchors.fill: parent
        anchors.leftMargin: control.leftPadding
        anchors.rightMargin: control.rightPadding
        anchors.topMargin: control.topPadding
        anchors.bottomMargin: control.bottomPadding
        text: control.text
        color: control.active ? control.accent : control.muted
        font.family: control.font.family
        font.weight: control.font.weight
        font.italic: control.font.italic
        font.letterSpacing: 0
        font.pixelSize: 13
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignLeft
        fontSizeMode: Text.HorizontalFit
        minimumPixelSize: 8
        elide: Text.ElideNone
        maximumLineCount: 1
        clip: true
    }
}
