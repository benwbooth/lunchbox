import QtQuick
import QtQuick.Controls

TextField {
    id: control

    signal clearRequested()

    rightPadding: 40
    selectByMouse: true
    Accessible.name: placeholderText

    ToolButton {
        objectName: "clearSearchButton"
        anchors.right: parent.right
        anchors.rightMargin: 4
        anchors.verticalCenter: parent.verticalCenter
        width: 32
        height: 32
        visible: control.text.length > 0
        z: 20
        text: "×"
        flat: true
        focusPolicy: Qt.NoFocus
        Accessible.name: "Clear search"
        onClicked: {
            control.text = ""
            control.clearRequested()
            control.forceActiveFocus()
        }
        ToolTip.visible: hovered
        ToolTip.text: "Clear search"
    }

    Keys.priority: Keys.BeforeItem
    Keys.onEscapePressed: event => {
        if (control.text.length > 0) {
            control.text = ""
            control.clearRequested()
            event.accepted = true
        }
    }
}
