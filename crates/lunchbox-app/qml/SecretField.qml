import QtQuick
import QtQuick.Controls

Item {
    id: control

    property alias text: editor.text
    property alias placeholderText: editor.placeholderText
    property alias readOnly: editor.readOnly
    property alias validator: editor.validator
    property bool revealed: false
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color line: "#283244"
    property color accent: "#ffb454"
    readonly property int effectiveEchoMode: editor.echoMode
    readonly property string actionText: revealButton.text

    signal textEdited(string text)
    signal accepted()

    implicitWidth: 240
    implicitHeight: editor.implicitHeight
    activeFocusOnTab: true

    onActiveFocusChanged: {
        if (activeFocus)
            editor.forceActiveFocus()
    }

    TextField {
        id: editor
        anchors.fill: parent
        rightPadding: revealButton.width + 10
        color: control.ink
        placeholderTextColor: control.muted
        selectByMouse: true
        echoMode: control.revealed ? TextInput.Normal : TextInput.Password
        onTextEdited: control.textEdited(text)
        onAccepted: control.accepted()
        background: Rectangle {
            implicitHeight: 40
            radius: 8
            color: "#101721"
            border.color: editor.activeFocus ? control.accent : control.line
        }
    }

    ToolButton {
        id: revealButton
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 3
        width: 58
        text: control.revealed ? "HIDE" : "SHOW"
        enabled: control.enabled
        focusPolicy: Qt.TabFocus
        Accessible.name: control.revealed ? "Hide secret" : "Show secret"
        Accessible.description: "Changes only how the value is displayed"
        onClicked: control.revealed = !control.revealed
        background: Rectangle {
            radius: 6
            color: revealButton.down ? "#303b4d"
                                      : revealButton.hovered ? "#202b39"
                                                            : "transparent"
            border.color: revealButton.activeFocus ? control.accent : "transparent"
        }
        contentItem: Text {
            text: revealButton.text
            color: revealButton.enabled ? control.accent : control.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.7
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }
}
