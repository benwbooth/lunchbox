import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    required property var torrent
    property color ink: "#f4f7fb"
    property color muted: "#95a2b6"
    property color accent: "#f2a13b"
    property color accentCool: "#62dac8"
    property color line: "#2b3647"

    radius: 9
    color: "#111923"
    border.color: line

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 42
            color: "#151f2c"
            radius: 9

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 12

                Text {
                    Layout.fillWidth: true
                    text: "REVIEWED PLATFORM SOURCES"
                    color: root.ink
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 0.6
                }
                Text {
                    text: root.torrent.batch_valid_count + " VALID"
                    color: root.accentCool
                    font.pixelSize: 9
                    font.weight: Font.Bold
                }
                Text {
                    visible: root.torrent.batch_error_count > 0
                    text: root.torrent.batch_error_count + " REJECTED"
                    color: "#ff9b91"
                    font.pixelSize: 9
                    font.weight: Font.Bold
                }
            }
        }

        ListView {
            id: batchList
            objectName: "torrentSourceBatchList"
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.torrent.batch_source_count
            boundsBehavior: Flickable.StopAtBounds
            flickDeceleration: 4500
            maximumFlickVelocity: 8500
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            delegate: Rectangle {
                id: sourceRow
                required property int index
                property int batchRevision: root.torrent.batch_revision
                property bool valid: {
                    batchRevision
                    return root.torrent.batch_valid_at(index)
                }
                width: ListView.view.width
                height: 72
                color: index % 2 === 0 ? "#111923" : "#131c28"
                border.color: "transparent"

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 10

                    Rectangle {
                        Layout.preferredWidth: 24
                        Layout.preferredHeight: 24
                        radius: 7
                        color: sourceRow.valid ? "#1d3934" : "#40232a"
                        border.color: sourceRow.valid ? root.accentCool : "#98505d"
                        Text {
                            anchors.centerIn: parent
                            text: sourceRow.valid ? "✓" : "!"
                            color: sourceRow.valid ? root.accentCool : "#ff9b91"
                            font.pixelSize: 13
                            font.weight: Font.Bold
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text {
                            Layout.fillWidth: true
                            text: root.torrent.batch_name_at(sourceRow.index)
                            color: root.ink
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            elide: Text.ElideMiddle
                        }
                        Text {
                            Layout.fillWidth: true
                            text: root.torrent.batch_detail_at(sourceRow.index)
                            color: sourceRow.valid ? root.muted : "#ff9b91"
                            font.pixelSize: 8
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        Layout.preferredWidth: 98
                        property string hash: root.torrent.batch_info_hash_at(sourceRow.index)
                        text: hash.length > 12
                              ? hash.slice(0, 6) + "…" + hash.slice(-6)
                              : hash
                        color: sourceRow.valid ? root.accentCool : "#718097"
                        font.family: "monospace"
                        font.pixelSize: 8
                        horizontalAlignment: Text.AlignRight
                    }
                }
            }
        }
    }
}
