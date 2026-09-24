import QtQuick
import QtQuick.Controls

ScrollView {
    id: view

    // ScrollView's effective width follows the platform scrollbar style.
    // Reserve that width in the layout instead of painting over text/controls.
    rightPadding: effectiveScrollBarWidth + 6

    MomentumWheelHandler {
        scroller: view.contentItem
        enabled: view.contentItem && view.contentItem.contentHeight > view.contentItem.height
                 && typeof view.contentItem.positionViewAtIndex !== "function"
    }
}
