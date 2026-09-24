import QtQuick
import QtQuick.Controls

ListView {
    id: view

    property bool defaultWheelMomentum: true
    readonly property real verticalScrollBarGutter:
        ScrollBar.vertical && ScrollBar.vertical.policy !== ScrollBar.AlwaysOff
        ? ScrollBar.vertical.width + 6 : 0
    readonly property real verticalContentWidth: Math.max(0, width - verticalScrollBarGutter)

    MomentumWheelHandler {
        scroller: view
        enabled: view.defaultWheelMomentum && view.interactive
                 && view.orientation === ListView.Vertical
                 && view.contentHeight > view.height
    }
}
