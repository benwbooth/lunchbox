import QtQuick
import QtQuick.Templates as T

T.RoundButton {
    id: control
    property bool positive: false

    implicitWidth: Math.max(32, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(32, implicitContentHeight + topPadding + bottomPadding)
    font.family: Qt.application.font.family
    font.pixelSize: 13
    clip: true

    background: LbControlBackground {
        radius: control.radius
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.highlighted || control.checked
        positive: control.positive
        flat: control.flat
        enabled: control.enabled
    }

    contentItem: LbButtonLabel {
        control: parent
        pixelSize: Math.max(13, control.font.pixelSize)
        fontWeight: control.font.weight
        color: !control.enabled ? "#8d99aa"
               : control.positive && (control.highlighted || control.checked) ? "#f4fff7"
               : control.highlighted || control.checked ? "#ffcb84" : "#f4f7fb"
    }
}
