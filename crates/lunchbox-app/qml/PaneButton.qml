import QtQuick
import QtQuick.Controls

// A Button that keeps the platform style (background, padding, focus,
// fonts) and only guarantees the label stays centered and inside the
// button: the stock contentItem does not elide or clip, which let long
// or changing labels overflow fixed-width buttons. Colors come from the
// control palette so the system theme keeps rendering them.
Button {
    id: control
    contentItem: Text {
        text: control.text
        font: control.font
        color: control.enabled ? control.palette.buttonText
                               : control.palette.placeholderText
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        maximumLineCount: 1
        clip: true
    }
}
