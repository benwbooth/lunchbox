import QtQuick
import QtQuick.Templates as T

T.ItemDelegate {
    id: control

    leftPadding: 12
    rightPadding: 12
    topPadding: 7
    bottomPadding: 7
    implicitHeight: Math.max(32, implicitContentHeight + topPadding + bottomPadding)
    font.family: Qt.application.font.family
    font.pixelSize: 13

    background: LbControlBackground {
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.highlighted
        flat: true
        enabled: control.enabled
    }

    contentItem: Text {
        anchors.fill: parent
        anchors.leftMargin: control.leftPadding
        anchors.rightMargin: control.rightPadding
        anchors.topMargin: control.topPadding
        anchors.bottomMargin: control.bottomPadding
        text: control.text
        font.family: control.font.family
        font.weight: Font.Medium
        font.italic: control.font.italic
        font.letterSpacing: 0
        font.pixelSize: 13
        color: control.enabled ? "#f4f7fb" : "#8d99aa"
        verticalAlignment: Text.AlignVCenter
        fontSizeMode: Text.HorizontalFit
        minimumPixelSize: 8
        elide: Text.ElideNone
    }
}
