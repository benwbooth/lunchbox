import QtQuick
import QtQuick.Templates as T

T.ToolButton {
    id: control

    implicitWidth: Math.max(30, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(30, implicitContentHeight + topPadding + bottomPadding)
    font.pixelSize: 12
    clip: true

    background: LbControlBackground {
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.checked
        flat: true
        enabled: control.enabled
    }

    contentItem: Text {
        text: control.text
        font: control.font
        color: control.enabled ? "#f4f7fb" : "#8d99aa"
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
}
