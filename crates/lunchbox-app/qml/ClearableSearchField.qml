import QtQuick
import QtQuick.Controls

TextField {
    id: control

    signal clearRequested()
    property bool searchIconVisible: false

    rightPadding: text.length > 0 ? 40 : 12
    selectByMouse: true
    Accessible.name: placeholderText

    Image {
        objectName: "searchIcon"
        anchors.left: parent.left
        anchors.leftMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        width: 18
        height: 18
        visible: control.searchIconVisible
        source: Qt.resolvedUrl("icons/search.svg")
        sourceSize.width: 18
        sourceSize.height: 18
        fillMode: Image.PreserveAspectFit
        smooth: true
        mipmap: true
        z: 20
    }

    ToolButton {
        objectName: "clearSearchButton"
        anchors.right: parent.right
        anchors.rightMargin: 4
        anchors.verticalCenter: parent.verticalCenter
        width: 32
        height: 32
        visible: control.text.length > 0
        enabled: visible
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
        background: Rectangle {
            radius: 8
            color: !parent.enabled ? "#151d29"
                   : parent.down ? "#344156"
                   : parent.hovered ? "#273346" : "#1b2534"
            border.color: parent.enabled
                          ? (parent.hovered ? "#718097" : "#46546a")
                          : "#303b4d"
        }
        contentItem: Text {
            text: parent.text
            color: parent.enabled ? "#f4f7fb" : "#718097"
            font.pixelSize: 20
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
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
