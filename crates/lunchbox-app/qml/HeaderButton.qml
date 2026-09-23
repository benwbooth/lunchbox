import QtQuick
import QtQuick.Controls

Button {
    id: control
    property bool active: false
    highlighted: active
    implicitHeight: Math.max(38, implicitBackgroundHeight + topInset + bottomInset,
                             implicitContentHeight + topPadding + bottomPadding)
    implicitWidth: Math.max(92, implicitBackgroundWidth + leftInset + rightInset,
                            implicitContentWidth + leftPadding + rightPadding)
    leftPadding: 16
    rightPadding: 16
    font.pixelSize: 13
    font.weight: Font.DemiBold
}
