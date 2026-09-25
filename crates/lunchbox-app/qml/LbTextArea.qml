import QtQuick
import QtQuick.Templates as T

T.TextArea {
    id: control

    leftPadding: 11
    rightPadding: 11
    topPadding: 9
    bottomPadding: 9
    implicitHeight: 74
    font.pixelSize: 14
    color: "#f4f7fb"
    placeholderTextColor: "#8d99aa"
    selectionColor: "#ffb454"
    selectedTextColor: "#101318"
    selectByMouse: true

    background: LbControlBackground {
        hovered: control.hovered
        focused: control.activeFocus
        enabled: control.enabled
    }
}
