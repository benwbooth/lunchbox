import QtQuick
import QtQuick.Controls

Button {
    id: catalogLink
    required property url destination
    property color accentCool: "#62d6c6"
    property color ink: "#f4f7fb"
    signal openRequested(string label, url destination)
    implicitHeight: 32
    implicitWidth: Math.max(92, contentItem.implicitWidth + 28)
    leftPadding: 14
    rightPadding: 14
    visible: destination.toString().length > 0
    enabled: visible
    font.pixelSize: 9
    font.weight: Font.Bold
    onClicked: openRequested(text, destination)
    Accessible.name: "Open " + text + " in the system browser"

    background: Rectangle {
        radius: 8
        color: catalogLink.down ? "#29384a"
                                : catalogLink.hovered ? "#202e3d" : "#172331"
        border.color: catalogLink.hovered ? catalogLink.accentCool : "#30445a"
    }
    contentItem: Text {
        text: catalogLink.text
        color: catalogLink.hovered ? catalogLink.accentCool : catalogLink.ink
        font: catalogLink.font
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
