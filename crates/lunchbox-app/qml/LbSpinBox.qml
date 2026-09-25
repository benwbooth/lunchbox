import QtQuick
import QtQuick.Controls as Controls

Controls.SpinBox {
    id: control

    implicitHeight: 34
    leftPadding: 11
    rightPadding: 38
    font.pixelSize: 14

    background: LbControlBackground {
        hovered: control.hovered
        focused: control.activeFocus
        enabled: control.enabled
    }

    up.indicator: Item {
        x: control.width - width - 4
        y: 2
        width: 30
        height: (control.height - 4) / 2
        Text {
            anchors.centerIn: parent
            text: "⌃"
            color: control.up.enabled ? "#c0c8d4" : "#637085"
            font.pixelSize: 15
        }
    }

    down.indicator: Item {
        x: control.width - width - 4
        y: control.height / 2
        width: 30
        height: (control.height - 4) / 2
        Text {
            anchors.centerIn: parent
            text: "⌄"
            color: control.down.enabled ? "#c0c8d4" : "#637085"
            font.pixelSize: 15
        }
    }
}
