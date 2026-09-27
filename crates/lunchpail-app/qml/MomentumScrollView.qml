import QtQuick
import QtQuick.Controls

ScrollView {
    id: view

    property bool defaultWheelMomentum: true

    ScrollBar.vertical: LbScrollBar { policy: ScrollBar.AsNeeded }
    ScrollBar.horizontal: LbScrollBar { policy: ScrollBar.AsNeeded }

    // ScrollView's effective width follows the platform scrollbar style.
    // Reserve that width in the layout instead of painting over text/controls.
    rightPadding: effectiveScrollBarWidth + 6

    MomentumWheelHandler {
        scroller: view.contentItem
        enabled: view.defaultWheelMomentum && view.contentItem
                 && view.contentItem.contentHeight > view.contentItem.height
                 && typeof view.contentItem.positionViewAtIndex !== "function"
    }
}
