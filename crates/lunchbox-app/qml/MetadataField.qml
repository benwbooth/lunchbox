import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: field
    property string label: ""
    property string value: ""
    property string placeholder: ""
    property var fieldValidator: null
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color line: "#283244"
    property color accent: "#ffb454"
    signal edited(string value)
    spacing: 5

    Text {
        Layout.fillWidth: true
        text: field.label.toUpperCase()
        color: field.muted
        font.pixelSize: 9
        font.weight: Font.Bold
        font.letterSpacing: 0.8
    }
    TextField {
        id: editor
        Layout.fillWidth: true
        text: field.value
        placeholderText: field.placeholder
        color: field.ink
        placeholderTextColor: "#637085"
        selectByMouse: true
        validator: field.fieldValidator
        onTextEdited: field.edited(text)
        background: Rectangle {
            implicitHeight: 40
            radius: 8
            color: "#101721"
            border.color: editor.activeFocus ? field.accent : field.line
        }
    }
}
