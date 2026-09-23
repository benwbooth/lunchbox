import QtQuick
import QtQuick.Controls

// Keep the entire native control, including text painting. KDE's desktop
// style draws its button label in the background QStyle item, so replacing
// contentItem would draw the label a second time.
Button {
    clip: true
    ToolTip.visible: hovered
    ToolTip.text: text
}
