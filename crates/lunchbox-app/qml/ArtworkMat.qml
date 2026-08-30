import QtQuick

Rectangle {
    id: surface

    property bool artworkPresent: false
    property color fallbackColor: "#40516a"
    property color neutralColor: "#080c12"
    readonly property bool fallbackVisible: !artworkPresent

    color: neutralColor

    Rectangle {
        objectName: "fallbackColorLayer"
        anchors.fill: parent
        visible: surface.fallbackVisible
        color: surface.fallbackColor
        gradient: Gradient {
            GradientStop {
                position: 0
                color: Qt.lighter(surface.fallbackColor, 1.17)
            }
            GradientStop {
                position: 1
                color: Qt.darker(surface.fallbackColor, 1.55)
            }
        }
    }
}
