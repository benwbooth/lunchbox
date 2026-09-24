import QtQuick
import QtQuick.Templates as T

T.ItemDelegate {
    id: control

    leftPadding: 12
    rightPadding: 12
    topPadding: 7
    bottomPadding: 7
    implicitHeight: Math.max(32, implicitContentHeight + topPadding + bottomPadding)
    font.pixelSize: 12

    background: LbControlBackground {
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.highlighted
        flat: true
        enabled: control.enabled
    }

    contentItem: Text {
        text: control.text
        font: control.font
        color: control.enabled ? "#f4f7fb" : "#8d99aa"
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
}
