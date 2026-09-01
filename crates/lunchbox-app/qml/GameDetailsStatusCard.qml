pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

Rectangle {
    id: card

    required property bool busy
    required property string message
    required property bool importTorrentAvailable
    required property color ink
    required property color muted
    required property color panel
    required property color line
    required property color accent

    signal importTorrentRequested()

    implicitHeight: contents.implicitHeight + 22
    radius: 9
    color: panel
    border.color: importTorrentAvailable ? accent : line

    Column {
        id: contents
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 11
        spacing: 9

        Text {
            width: parent.width
            visible: card.importTorrentAvailable
            text: "NO MATCHING DOWNLOAD"
            color: card.accent
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.9
        }

        Row {
            width: parent.width
            spacing: 9

            BusyIndicator {
                id: statusBusy
                width: 18
                height: 18
                running: card.busy
                visible: running
            }

            Text {
                width: parent.width - (statusBusy.visible
                                       ? statusBusy.width + parent.spacing : 0)
                text: card.message
                color: card.muted
                font.pixelSize: 11
                lineHeight: 1.2
                wrapMode: Text.WordWrap
            }
        }

        Button {
            objectName: "importTorrentForGameButton"
            width: parent.width
            height: 42
            visible: card.importTorrentAvailable
            enabled: visible && !card.busy
            text: "IMPORT TORRENT FOR THIS GAME"
            font.pixelSize: 10
            font.weight: Font.Bold
            Accessible.name: "Import a torrent for this game"
            onClicked: card.importTorrentRequested()
            background: Rectangle {
                radius: 8
                color: parent.enabled
                       ? (parent.down ? "#d68d36" : card.accent)
                       : "#26313e"
                border.color: parent.enabled ? "#ffc579" : card.line
            }
            contentItem: Text {
                text: parent.text
                color: parent.enabled ? "#1b140c" : card.muted
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }
    }
}
