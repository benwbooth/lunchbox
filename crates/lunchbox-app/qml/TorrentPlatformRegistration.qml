import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    required property var torrent
    property color ink: "#f4f7fb"
    property color muted: "#95a2b6"
    property color line: "#2b3647"

    objectName: "torrentPlatformRegistration"
    visible: !torrent.collection_mode
    radius: 9
    color: "#111923"
    border.color: line

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        spacing: 10

        CheckBox {
            id: platformSourceCheckBox
            objectName: "torrentPlatformRegistrationCheckBox"
            checked: root.torrent.batch_source_count > 0
                     || root.torrent.register_for_platform
            enabled: !root.torrent.busy
                     && root.torrent.batch_source_count === 0
            onToggled: {
                if (root.torrent.batch_source_count === 0)
                    root.torrent.set_platform_registration(checked)
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                objectName: "torrentPlatformRegistrationTitle"
                Layout.fillWidth: true
                text: (root.torrent.batch_source_count > 0
                       ? "USE THESE TORRENTS FOR " : "USE THIS TORRENT FOR ")
                      + root.torrent.game_platform.toUpperCase()
                color: root.ink
                font.pixelSize: 10
                font.weight: Font.Bold
                elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                text: root.torrent.batch_source_count > 0
                      ? "Multi-source review indexes every validated torrent for this platform; matching games can then download individual payloads on demand."
                      : "Save its metadata and member index in Lunchbox for future exact-title Get results. The source remains usable if its qBittorrent entry is removed."
                color: root.muted
                font.pixelSize: 9
                wrapMode: Text.WordWrap
            }
        }
    }
}
