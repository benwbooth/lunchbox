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
    font.pixelSize: 12
    clip: true

    background: LbControlBackground {
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.checked
        enabled: control.enabled
    }

    contentItem: Text {
        text: control.text
        font: control.font
        color: !control.enabled ? "#8d99aa"
               : control.checked ? "#ffcb84" : "#f4f7fb"
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
}
