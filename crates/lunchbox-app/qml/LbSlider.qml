import QtQuick
import QtQuick.Templates as T

T.Slider {
    id: control
    readonly property bool providesFocusIndicator: true
    property color progressColor: "#62d6c6"
    implicitWidth: horizontal ? 160 : 32
    implicitHeight: horizontal ? 32 : 160
    padding: 6
    hoverEnabled: true
    background: Rectangle {
        x: control.horizontal ? control.leftPadding : control.leftPadding + (control.availableWidth - width) / 2
        y: control.horizontal ? control.topPadding + (control.availableHeight - height) / 2 : control.topPadding
        width: control.horizontal ? control.availableWidth : 4
        height: control.horizontal ? 4 : control.availableHeight
        radius: 2
        color: "#3a495f"
        opacity: control.enabled ? 1 : 0.45
        Rectangle {
            x: control.horizontal && control.mirrored ? parent.width - width : 0
            y: control.horizontal ? 0 : parent.height - height
            width: control.horizontal ? control.position * parent.width : parent.width
            height: control.horizontal ? parent.height : control.position * parent.height
            radius: 2
            color: control.progressColor
        }
    }
    handle: Rectangle {
        x: control.leftPadding + (control.horizontal ? control.visualPosition * (control.availableWidth - width) : (control.availableWidth - width) / 2)
        y: control.topPadding + (control.horizontal ? (control.availableHeight - height) / 2 : control.visualPosition * (control.availableHeight - height))
        implicitWidth: 14
        implicitHeight: 14
        radius: width / 2
        color: control.enabled ? "#f4f7fb" : "#71849a"
        border.width: control.visualFocus ? 2 : 0
        border.color: control.progressColor
        scale: control.pressed || control.visualFocus ? 1.15 : 1
    }
}
