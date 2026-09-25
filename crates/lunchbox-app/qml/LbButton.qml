import QtQuick
import QtQuick.Templates as T

T.Button {
    id: control
    property bool positive: false
    property int maximumImplicitWidth: 220

    leftPadding: 12
    rightPadding: 12
    topPadding: 6
    bottomPadding: 6
    implicitWidth: Math.max(52, Math.min(maximumImplicitWidth,
                                         implicitContentWidth + leftPadding + rightPadding))
    implicitHeight: Math.max(32, implicitContentHeight + topPadding + bottomPadding)
    font.pixelSize: 12
    clip: true

    background: LbControlBackground {
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.highlighted || control.checked
        positive: control.positive
        flat: control.flat
        enabled: control.enabled
    }

    contentItem: Text {
        anchors.fill: parent
        anchors.leftMargin: control.leftPadding
        anchors.rightMargin: control.rightPadding
        anchors.topMargin: control.topPadding
        anchors.bottomMargin: control.bottomPadding
        text: control.text
        font.family: control.font.family
        font.weight: control.font.weight
        font.italic: control.font.italic
        font.letterSpacing: control.font.letterSpacing
        font.pixelSize: control.font.pixelSize
        color: !control.enabled ? "#8d99aa"
               : control.positive && (control.highlighted || control.checked) ? "#f4fff7"
               : control.highlighted || control.checked ? "#ffcb84" : "#f4f7fb"
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        fontSizeMode: Text.HorizontalFit
        minimumPixelSize: Math.min(8, control.font.pixelSize)
        elide: Text.ElideNone
    }
}
