import QtQuick
import QtQuick.Templates as T

T.TextField {
    id: control

    leftPadding: 11
    rightPadding: 11
    topPadding: 7
    bottomPadding: 7
    implicitWidth: 240
    implicitHeight: 36
    font.family: Qt.application.font.family
    font.pixelSize: 13
    color: "#f4f7fb"
    placeholderTextColor: "#8d99aa"
    selectionColor: "#ffb454"
    selectedTextColor: "#101318"
    selectByMouse: true

    // Templates provide editing behavior, but not the style's placeholder.
    Text {
        x: control.leftPadding
        y: control.topPadding
        width: Math.max(0, control.width - control.leftPadding - control.rightPadding)
        height: Math.max(0, control.height - control.topPadding - control.bottomPadding)
        text: control.placeholderText
        textFormat: Text.PlainText
        font: control.font
        color: control.placeholderTextColor
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        visible: !control.length && !control.preeditText
        Accessible.ignored: true
    }

    background: LbControlBackground {
        hovered: control.hovered
        focused: control.activeFocus
        enabled: control.enabled
    }
}
