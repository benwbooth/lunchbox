import QtQuick
import QtQuick.Controls

ListView {
    id: view

    // The shared handler scales its glide with the scrollable length, so
    // compact lists stay direct while long lists retain momentum.
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

    HorizontalWheelHandler {
        scroller: view
        enabled: view.defaultWheelMomentum && view.interactive
                 && view.orientation === ListView.Horizontal
                 && view.contentWidth > view.width
    }
}
