import QtQuick
import QtQuick.Templates as T

T.CheckBox {
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
        radius: 5
        color: control.checkState !== Qt.Unchecked ? "#237a4d" : "#202a39"
        border.color: control.visualFocus ? "#ffb454" : control.checkState !== Qt.Unchecked ? "#5ee391" : control.hovered ? "#53647c" : "#3a495f"
        opacity: control.enabled ? 1 : 0.5
        Text { anchors.centerIn: parent; text: control.checkState === Qt.PartiallyChecked ? "−" : "✓"; color: "#f4fff7"; visible: control.checkState !== Qt.Unchecked; font.pixelSize: 15 }
    }
    contentItem: LbOptionLabel { control: parent }
}
