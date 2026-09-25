import QtQuick
import QtQuick.Templates as T

T.TextField {
    id: control

    leftPadding: 11
    rightPadding: 11
    topPadding: 7
    bottomPadding: 7
    implicitHeight: 36
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
