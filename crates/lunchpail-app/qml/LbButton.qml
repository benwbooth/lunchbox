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

    contentItem: LbButtonLabel {
        control: parent
        color: !control.enabled ? "#8d99aa"
               : control.positive && (control.highlighted || control.checked) ? "#f4fff7"
               : control.highlighted || control.checked ? "#ffcb84" : "#f4f7fb"
    }
}
