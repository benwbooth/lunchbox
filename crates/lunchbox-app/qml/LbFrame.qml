import QtQuick
import QtQuick.Templates as T

T.Frame {
    padding: 16
    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             implicitContentHeight + topPadding + bottomPadding)
    background: Rectangle {
        color: "#17212e"
        border.color: "#344358"
        radius: 10
    }
}
