import QtQuick
import QtQuick.Templates as T

T.TextArea {
    id: control

    leftPadding: 11
    rightPadding: 11
    topPadding: 9
    bottomPadding: 9
    implicitWidth: 240
    implicitHeight: 74
    font.family: Qt.application.font.family
    font.pixelSize: 13
    color: "#f4f7fb"
    placeholderTextColor: "#8d99aa"
    selectionColor: "#ffb454"
    selectedTextColor: "#101318"
    selectByMouse: true

    Text {
        x: control.leftPadding
        y: control.topPadding
        width: Math.max(0, control.width - control.leftPadding - control.rightPadding)
        text: control.placeholderText
        textFormat: Text.PlainText
        font: control.font
        color: control.placeholderTextColor
        wrapMode: Text.Wrap
        visible: !control.length && !control.preeditText
        Accessible.ignored: true
    }

    background: LbControlBackground {
        hovered: control.hovered
        focused: control.activeFocus
        enabled: control.enabled
    }
}
