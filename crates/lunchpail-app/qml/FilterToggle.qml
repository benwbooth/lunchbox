import QtQuick

Rectangle {
    id: toggle
    required property string label
    required property string description
    property bool checked: false
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color accentCool: "#62d6c6"
    signal toggled()
    activeFocusOnTab: true
    Accessible.role: Accessible.CheckBox
    Accessible.name: label
    Accessible.description: description
    Accessible.checkable: true
    Accessible.checked: checked
    Accessible.onPressAction: toggle.toggled()
    Keys.onSpacePressed: toggle.toggled()
    Keys.onReturnPressed: toggle.toggled()
    Keys.onEnterPressed: toggle.toggled()
    width: parent ? parent.width : 320
    height: Math.max(58, labels.implicitHeight + 18)
    radius: 8
    color: filterHover.hovered ? "#1b2432" : "transparent"
    border.width: activeFocus ? 1 : 0
    border.color: "#ffb454"

    HoverHandler { id: filterHover }
    TapHandler { onTapped: toggle.toggled() }

    Rectangle {
        anchors.left: parent.left
        anchors.leftMargin: 11
        anchors.verticalCenter: parent.verticalCenter
        width: 18
        height: 18
        radius: 5
        color: toggle.checked ? toggle.accentCool : "#101721"
        border.color: toggle.checked ? toggle.accentCool : "#455166"
        Text {
            anchors.centerIn: parent
            text: toggle.checked ? "✓" : ""
            color: "#0c1716"
            font.pixelSize: 13
            font.weight: Font.Black
        }
    }
    Column {
        id: labels
        anchors.left: parent.left
        anchors.leftMargin: 42
        anchors.right: parent.right
        anchors.rightMargin: 10
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2
        Text {
            width: parent.width
            text: toggle.label
            color: toggle.ink
            font.pixelSize: 13
            font.weight: Font.DemiBold
            wrapMode: Text.Wrap
        }
        Text {
            width: parent.width
            text: toggle.description
            color: toggle.muted
            font.pixelSize: 11
            wrapMode: Text.Wrap
        }
    }
}
