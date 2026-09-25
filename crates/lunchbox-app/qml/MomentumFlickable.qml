import QtQuick
import QtQuick.Controls

Flickable {
    id: view

    property bool defaultWheelMomentum: false
    property bool blockNativeWheel: false
    readonly property real verticalScrollBarGutter:
        ScrollBar.vertical && ScrollBar.vertical.policy !== ScrollBar.AlwaysOff
        ? ScrollBar.vertical.width + 6 : 0
    readonly property real verticalContentWidth: Math.max(0, width - verticalScrollBarGutter)

    MomentumWheelHandler {
        scroller: view
        blocking: view.blockNativeWheel
        enabled: view.defaultWheelMomentum && view.interactive
                 && view.contentHeight > view.height
    }
}
