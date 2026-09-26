import QtQuick
import QtQuick.Templates as T

T.CheckBox {
    id: control
    spacing: 9
    padding: 4
    font.pixelSize: 14
    implicitWidth: indicator.width + spacing + contentItem.implicitWidth + leftPadding + rightPadding
    implicitHeight: Math.max(indicator.height, contentItem.implicitHeight) + topPadding + bottomPadding
    indicator: Rectangle {
        implicitWidth: 20
        implicitHeight: 20
        x: control.leftPadding
        y: (control.height - height) / 2
        radius: 5
        color: control.checked ? "#237c53" : "#202c3b"
        border.color: control.activeFocus ? "#ffb454" : control.checked ? "#43c981" : "#40526a"
        opacity: control.enabled ? 1 : 0.5
        Text { anchors.centerIn: parent; text: "✓"; color: "white"; visible: control.checked; font.pixelSize: 15 }
    }
    contentItem: Text {
        leftPadding: control.indicator.width + control.spacing
        text: control.text
        font: control.font
        color: control.enabled ? "#f4f7fb" : "#8793a2"
        verticalAlignment: Text.AlignVCenter
        wrapMode: Text.WordWrap
    }
}
