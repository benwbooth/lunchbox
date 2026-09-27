import QtQuick
import QtQuick.Templates as T

T.ToolButton {
    id: control
    readonly property bool iconOnly: text.length > 0 && !/[\p{L}\p{N}]/u.test(text)

    implicitWidth: Math.max(30, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(30, implicitContentHeight + topPadding + bottomPadding)
    font.family: Qt.application.font.family
    font.pixelSize: 13
    clip: true

    background: LbControlBackground {
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.checked
        flat: true
        enabled: control.enabled
    }

    contentItem: LbButtonLabel {
        control: parent
        pixelSize: control.iconOnly ? Math.max(13, control.font.pixelSize) : 13
        color: control.enabled ? "#f4f7fb" : "#8d99aa"
    }
}
