import QtQuick
import QtQuick.Controls

// Keep pane actions on the same themed surface and show long labels on hover.
LbButton {
    clip: true
    ToolTip.visible: hovered
    ToolTip.delay: 450
    ToolTip.text: text
}
