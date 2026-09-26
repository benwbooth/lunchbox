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

    contentItem: LbButtonLabel {
        control: parent
        color: !control.enabled ? "#8d99aa"
               : control.checked ? "#ffcb84" : "#f4f7fb"
    }
}
