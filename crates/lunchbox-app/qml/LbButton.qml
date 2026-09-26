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
    font.family: Qt.application.font.family
    font.pixelSize: 13
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
        anchors.leftMargin: Math.max(control.leftPadding, control.rightPadding)
        anchors.rightMargin: anchors.leftMargin
        anchors.topMargin: Math.max(control.topPadding, control.bottomPadding)
        anchors.bottomMargin: anchors.topMargin
        text: control.text
        font.family: control.font.family
        // Text actions share one label style, even when an older caller sets
        // a tiny control font. HorizontalFit still shrinks long labels to fit.
        font.weight: Font.Medium
        font.italic: control.font.italic
        font.letterSpacing: 0
        font.pixelSize: 13
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
