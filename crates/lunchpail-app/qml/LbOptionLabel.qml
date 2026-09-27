import QtQuick

Text {
    required property var control
    leftPadding: control.mirrored ? 0 : control.indicator.width + control.spacing
    rightPadding: control.mirrored ? control.indicator.width + control.spacing : 0
    text: control.text
    textFormat: Text.PlainText
    font: control.font
    color: control.enabled ? "#f4f7fb" : "#8d99aa"
    verticalAlignment: Text.AlignVCenter
    wrapMode: Text.Wrap
}
