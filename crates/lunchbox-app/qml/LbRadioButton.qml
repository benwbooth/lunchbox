import QtQuick
import QtQuick.Templates as T

T.RadioButton {
    id: control
    spacing: 9
    padding: 4
    hoverEnabled: true
    font.family: Qt.application.font.family
    font.pixelSize: 13
    implicitWidth: contentItem.implicitWidth + leftPadding + rightPadding
    implicitHeight: Math.max(indicator.height, contentItem.implicitHeight) + topPadding + bottomPadding
    indicator: Rectangle {
        implicitWidth: 20
        implicitHeight: 20
        x: control.mirrored ? control.width - width - control.rightPadding : control.leftPadding
        y: (control.height - height) / 2
        radius: width / 2
        color: "#202a39"
        border.color: control.visualFocus ? "#ffb454" : control.checked ? "#5ee391" : control.hovered ? "#53647c" : "#3a495f"
        opacity: control.enabled ? 1 : 0.5
        Rectangle {
            anchors.centerIn: parent
            width: 10; height: 10; radius: 5
            color: "#5ee391"
            visible: control.checked
        }
    }
    contentItem: LbOptionLabel { control: parent }
}
