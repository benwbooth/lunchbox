import QtQuick
import QtQuick.Controls as Controls

Controls.Dialog {
    id: control

    padding: 20
    font.family: Qt.application.font.family
    font.pixelSize: 13
    palette.windowText: "#f4f7fb"
    palette.text: "#f4f7fb"
    palette.buttonText: "#f4f7fb"
    palette.placeholderText: "#8d99aa"
    palette.highlight: "#ffb454"
    palette.highlightedText: "#101318"
    palette.window: "#1a2230"
    palette.base: "#202a39"
    palette.button: "#202a39"

    background: Rectangle {
        color: "#1a2230"
        border.color: "#3a495f"
        radius: 12
    }
    header: Item {
        implicitHeight: control.title.length ? Math.max(56, titleLabel.implicitHeight + 28) : 0
        visible: control.title.length > 0
        Text {
            id: titleLabel
            anchors.left: parent.left
            anchors.right: closeButton.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 20
            anchors.rightMargin: 12
            text: control.title
            textFormat: Text.PlainText
            color: "#f4f7fb"
            font.family: control.font.family
            font.pixelSize: 18
            font.weight: Font.DemiBold
            wrapMode: Text.Wrap
        }
        LbToolButton {
            id: closeButton
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            width: 32; height: 32
            text: "×"
            font.pixelSize: 20
            Accessible.name: "Close " + control.title
            onClicked: control.reject()
        }
    }

    footer: Controls.DialogButtonBox {
        visible: count > 0
        standardButtons: control.standardButtons
        spacing: 8
        padding: 16
        delegate: LbButton {}
        background: Rectangle { color: "transparent" }
    }
}
