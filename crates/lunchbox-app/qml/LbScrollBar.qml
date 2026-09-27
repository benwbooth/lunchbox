import QtQuick
import QtQuick.Controls as Controls

Controls.ScrollBar {
    id: control

    implicitWidth: 8
    implicitHeight: 8
    padding: 2
    minimumSize: 0.08

    background: Rectangle {
        implicitWidth: 8
        implicitHeight: 8
        radius: Math.min(width, height) / 2
        color: control.hovered ? "#263648" : "transparent"
    }

    contentItem: Rectangle {
        implicitWidth: 4
        implicitHeight: 4
        radius: Math.min(width, height) / 2
        color: !control.enabled ? "#516174"
               : control.pressed ? "#b4d2e4"
               : control.hovered ? "#a4b4c6" : "#71849a"
    }
}
