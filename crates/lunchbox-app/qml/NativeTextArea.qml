import QtQuick
import QtQuick.Controls

FocusScope {
    id: control

    property alias text: editor.text
    property alias font: editor.font
    property alias readOnly: editor.readOnly
    property alias selectByMouse: editor.selectByMouse
    property string placeholderText: ""
    property color color: "#f4f7fb"
    property color placeholderTextColor: "#637085"
    property color selectionColor: "#ffb454"
    property color selectedTextColor: "#101318"
    property color backgroundColor: "#101721"
    property color line: "#283244"
    property color accent: "#ffb454"
    property real radius: 8
    property int wrapMode: TextEdit.Wrap

    implicitWidth: 240
    implicitHeight: 96
    activeFocusOnTab: true
    Accessible.role: Accessible.EditableText
    Accessible.name: placeholderText

    onActiveFocusChanged: {
        if (activeFocus && !editor.activeFocus)
            editor.forceActiveFocus()
    }

    Rectangle {
        anchors.fill: parent
        radius: control.radius
        color: control.backgroundColor
        border.color: control.activeFocus ? control.accent : control.line
    }

    Flickable {
        id: editorFlick
        anchors.fill: parent
        anchors.margins: 9
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        contentWidth: width
        contentHeight: Math.max(height, editor.contentHeight)
        flickableDirection: Flickable.VerticalFlick
        ScrollBar.vertical: ScrollBar {
            policy: editorFlick.contentHeight > editorFlick.height
                    ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
        }

        TextEdit {
            id: editor
            width: editorFlick.width
            height: Math.max(editorFlick.height, contentHeight)
            color: control.color
            selectionColor: control.selectionColor
            selectedTextColor: control.selectedTextColor
            wrapMode: control.wrapMode
            selectByMouse: true
            persistentSelection: true
            onCursorRectangleChanged: editorFlick.ensureVisible(cursorRectangle)
        }

        Text {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            visible: editor.text.length === 0
            text: control.placeholderText
            color: control.placeholderTextColor
            font: editor.font
            wrapMode: Text.Wrap
        }

        function ensureVisible(rectangle) {
            const top = rectangle.y
            const bottom = rectangle.y + rectangle.height
            if (top < contentY)
                contentY = Math.max(0, top)
            else if (bottom > contentY + height)
                contentY = Math.min(contentHeight - height, bottom - height)
        }
    }
}
