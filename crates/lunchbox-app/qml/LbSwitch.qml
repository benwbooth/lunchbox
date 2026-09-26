import QtQuick
import QtQuick.Templates as T

T.Switch {
    id: control
    spacing: 10
    padding: 4
    hoverEnabled: true
    font.family: Qt.application.font.family
    font.pixelSize: 13
    implicitWidth: contentItem.implicitWidth + leftPadding + rightPadding
    implicitHeight: Math.max(indicator.height, contentItem.implicitHeight) + topPadding + bottomPadding
    indicator: Rectangle {
        implicitWidth: 38
        implicitHeight: 22
        x: control.mirrored ? control.width - width - control.rightPadding : control.leftPadding
        y: (control.height - height) / 2
        radius: height / 2
        color: control.checked ? "#237a4d" : "#202a39"
        border.color: control.visualFocus ? "#ffb454" : control.checked ? "#5ee391" : control.hovered ? "#53647c" : "#3a495f"
        opacity: control.enabled ? 1 : 0.5
        Rectangle {
            x: 3 + control.visualPosition * (parent.width - width - 6)
            y: 3
            width: 16
            height: 16
            radius: 8
            color: control.checked ? "#f4fff7" : "#b7c3d2"
            Behavior on x { enabled: !control.down; NumberAnimation { duration: 100 } }
        }
    }
    contentItem: LbOptionLabel { control: parent }
}
