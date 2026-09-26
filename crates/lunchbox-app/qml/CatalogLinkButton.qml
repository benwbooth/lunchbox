import QtQuick
import QtQuick.Controls

LbButton {
    id: catalogLink
    required property url destination
    signal openRequested(string label, url destination)
    implicitHeight: 32
    implicitWidth: Math.max(92, contentItem.implicitWidth + 28)
    leftPadding: 14
    rightPadding: 14
    visible: destination.toString().length > 0
    enabled: visible
    font.pixelSize: 13
    font.weight: Font.Medium
    onClicked: openRequested(text, destination)
    Accessible.name: "Open " + text + " in the system browser"
}
