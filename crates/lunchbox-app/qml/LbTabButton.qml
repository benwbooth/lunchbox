import QtQuick
import QtQuick.Templates as T

T.TabButton {
    id: control

    leftPadding: 12
    rightPadding: 12
    topPadding: 6
    bottomPadding: 6
    implicitWidth: Math.max(80, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(34, implicitContentHeight + topPadding + bottomPadding)
    font.family: Qt.application.font.family
    font.pixelSize: 13
    clip: true

    background: LbControlBackground {
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.checked
        enabled: control.enabled
    }

    contentItem: Text {
        anchors.fill: parent
        anchors.leftMargin: Math.max(control.leftPadding, control.rightPadding)
        anchors.rightMargin: anchors.leftMargin
        anchors.topMargin: Math.max(control.topPadding, control.bottomPadding)
        anchors.bottomMargin: anchors.topMargin
        text: control.text
        font.family: control.font.family
        font.weight: Font.Medium
        font.italic: control.font.italic
        font.letterSpacing: 0
        font.pixelSize: 13
        color: !control.enabled ? "#8d99aa"
               : control.checked ? "#ffcb84" : "#f4f7fb"
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        fontSizeMode: Text.HorizontalFit
        minimumPixelSize: 8
        elide: Text.ElideNone
    }
}
