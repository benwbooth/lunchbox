import QtQuick
import QtQuick.Controls

// Dark-theme replacement for default-styled buttons in the settings and
// emulator-manager panes. The platform Button style sizes and pads labels
// for its own look, which let long or changing labels overflow fixed-width
// buttons; an explicit centered, eliding contentItem keeps the text inside
// the button and optically centered at every label length.
Button {
    id: control
    implicitHeight: 32
    leftPadding: 14
    rightPadding: 14
    font.pixelSize: 12

    background: Rectangle {
        radius: 7
        color: control.down ? "#3a465c"
                            : control.enabled ? "#222b3a" : "#1a2230"
        border.color: control.enabled ? "#46536b" : "#2c3547"
    }
    contentItem: Text {
        width: control.availableWidth
        text: control.text
        color: control.enabled ? "#dfe6f1" : "#8d99aa"
        font: control.font
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        maximumLineCount: 1
        clip: true
    }
}
