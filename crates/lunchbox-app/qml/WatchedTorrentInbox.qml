import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: root

    required property var settingsModel
    required property var inboxModel
    property color ink: "#edf3f8"
    property color muted: "#91a0b4"
    property color line: "#273244"
    property color panel: "#111924"
    property color accent: "#f4a442"
    property color accentCool: "#62d7c6"

    signal reviewRequested(url source)

    Layout.fillWidth: true
    spacing: 9

    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                text: "WATCHED TORRENT INBOX"
                color: root.accent
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
            Text {
                Layout.fillWidth: true
                text: "Drop lawful .torrent files here. Lunchbox indexes metadata only, keeps pending sources visible, and never queues payload until you review an exact platform."
                color: root.muted
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }
        }

        Button {
            objectName: "watchedTorrentRefresh"
            text: "Refresh"
            enabled: !root.inboxModel.busy
                     && root.inboxModel.directory.length > 0
            onClicked: root.inboxModel.refresh()
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        TextField {
            Layout.fillWidth: true
            readOnly: true
            placeholderText: "No watched inbox configured"
            text: root.settingsModel.watched_torrent_directory
            ToolTip.visible: hovered && text.length > 0
            ToolTip.text: text
        }
        Button {
            text: "Choose…"
            onClicked: root.settingsModel.choose_native_directory("torrent-watch")
        }
        Button {
            text: "Clear"
            visible: root.settingsModel.watched_torrent_directory.length > 0
            onClicked: root.settingsModel.watched_torrent_directory = ""
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 42
        radius: 8
        color: Qt.rgba(1, 1, 1, 0.025)
        border.color: root.line
        border.width: 1

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            spacing: 12
            BusyIndicator {
                Layout.preferredWidth: 22
                Layout.preferredHeight: 22
                running: root.inboxModel.busy
                visible: running
            }
            Text {
                Layout.fillWidth: true
                text: root.settingsModel.watched_torrent_directory
                      !== root.inboxModel.directory
                      ? "Save settings to begin watching this folder."
                      : root.inboxModel.message
                color: root.inboxModel.invalid_count > 0
                       ? root.accent : root.muted
                font.pixelSize: 10
                elide: Text.ElideMiddle
            }
            Text {
                visible: root.inboxModel.entry_count > 0
                text: root.inboxModel.pending_count + " REVIEW  ·  "
                      + root.inboxModel.registered_count + " ADDED"
                color: root.accentCool
                font.pixelSize: 9
                font.weight: Font.Bold
            }
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: root.inboxModel.entry_count > 0 ? 250 : 92
        radius: 9
        color: root.panel
        border.color: root.line
        border.width: 1

        ListView {
            id: inboxList
            objectName: "watchedTorrentList"
            anchors.fill: parent
            anchors.margins: 7
            clip: true
            spacing: 5
            boundsBehavior: Flickable.StopAtBounds
            model: {
                root.inboxModel.revision
                return root.inboxModel.entry_count
            }
            ScrollBar.vertical: ScrollBar { }

            delegate: Rectangle {
                id: inboxRow
                required property int index
                readonly property string status: root.inboxModel.status_at(index)
                width: inboxList.width - 10
                height: 62
                radius: 7
                color: status === "pending" ? "#1d2934" : "#151e29"
                border.color: status === "pending" ? root.accent
                              : status === "invalid" ? "#9b583f" : root.line
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 11
                    anchors.rightMargin: 8
                    spacing: 10

                    Rectangle {
                        Layout.preferredWidth: 74
                        Layout.preferredHeight: 24
                        radius: 6
                        color: inboxRow.status === "pending" ? "#3b2b18"
                               : inboxRow.status === "registered" ? "#17342d"
                               : "#3a211e"
                        border.color: inboxRow.status === "pending" ? root.accent
                                      : inboxRow.status === "registered" ? root.accentCool
                                      : "#c36b54"
                        Text {
                            anchors.centerIn: parent
                            text: inboxRow.status === "pending" ? "REVIEW"
                                  : inboxRow.status === "registered" ? "ADDED"
                                  : "INVALID"
                            color: inboxRow.status === "pending" ? root.accent
                                   : inboxRow.status === "registered" ? root.accentCool
                                   : "#ff9b91"
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Text {
                            Layout.fillWidth: true
                            text: root.inboxModel.torrent_name_at(inboxRow.index).length > 0
                                  ? root.inboxModel.torrent_name_at(inboxRow.index)
                                  : root.inboxModel.file_name_at(inboxRow.index)
                            color: root.ink
                            font.pixelSize: 11
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: root.inboxModel.detail_at(inboxRow.index)
                            color: inboxRow.status === "invalid" ? "#ff9b91" : root.muted
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: root.inboxModel.file_path_at(inboxRow.index)
                            color: "#6f7e91"
                            font.pixelSize: 8
                            elide: Text.ElideMiddle
                        }
                    }

                    Button {
                        objectName: "watchedTorrentReview-" + inboxRow.index
                        Layout.preferredWidth: 92
                        Layout.preferredHeight: 32
                        text: "Review…"
                        visible: root.inboxModel.can_review_at(inboxRow.index)
                        onClicked: root.reviewRequested(
                                       root.inboxModel.file_url_at(inboxRow.index))
                    }
                }
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(parent.width - 36, 540)
            spacing: 5
            visible: root.inboxModel.entry_count === 0
                     && !root.inboxModel.busy
            Text {
                width: parent.width
                text: root.settingsModel.watched_torrent_directory.length > 0
                      ? "INBOX IS CLEAR" : "NO WATCHED INBOX"
                color: root.ink
                font.pixelSize: 11
                font.weight: Font.Bold
                horizontalAlignment: Text.AlignHCenter
            }
            Text {
                width: parent.width
                text: root.settingsModel.watched_torrent_directory.length > 0
                      ? "New .torrent files appear here for platform review."
                      : "Choose a folder above, then save settings."
                color: root.muted
                font.pixelSize: 10
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }
    }
}
