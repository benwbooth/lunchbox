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

    function revealSelection() {
        if (torrent.selected_index < 0)
            return
        const row = torrent.filtered_row_for_file_index(torrent.selected_index)
        if (row >= 0)
            payloadFiles.positionViewAtIndex(row, ListView.Contain)
    }

    radius: 10
    color: "#0e141d"
    border.color: root.line

    Timer {
        id: searchDelay
        interval: 120
        repeat: false
        onTriggered: root.torrent.filter_files(payloadSearch.text)
    }

    Connections {
        target: root.torrent
        function onReadyChanged() {
            if (!root.torrent.ready) {
                payloadSearch.text = ""
                searchDelay.stop()
            } else {
                Qt.callLater(root.revealSelection)
            }
        }
        function onFiltered_revisionChanged() {
            Qt.callLater(root.revealSelection)
        }
        function onSelected_indexChanged() {
            Qt.callLater(root.revealSelection)
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 1
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: root.torrent.ready ? 80 : 42
            color: "#151d29"

            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                anchors.topMargin: 7
                anchors.bottomMargin: 7
                spacing: 6

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    Text {
                        Layout.fillWidth: true
                        text: root.torrent.ready
                              ? (root.torrent.collection_mode
                                 ? "FILES AVAILABLE ON DEMAND"
                                 : "SELECT THE EXACT GAME PAYLOAD")
                              : "TORRENT CONTENTS"
                        color: root.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.9
                    }

                    Text {
                        visible: root.torrent.ready
                        text: root.torrent.filter_busy
                              ? "SEARCHING…"
                              : root.torrent.filtered_file_count + " / "
                                + root.torrent.file_count + " FILES"
                        color: root.torrent.filter_busy ? root.accent : "#6f7c8f"
                        font.pixelSize: 8
                        font.weight: Font.DemiBold
                    }
                }

                ClearableSearchField {
                    id: payloadSearch
                    objectName: "torrentPayloadSearch"
                    Layout.fillWidth: true
                    Layout.preferredHeight: 34
                    visible: root.torrent.ready
                    enabled: visible && !root.torrent.busy
                    searchIconVisible: true
                    leftPadding: 40
                    placeholderText: root.torrent.collection_mode
                                     ? "Search torrent filenames"
                                     : "Search for " + root.torrent.game_title
                    font.pixelSize: 10
                    color: root.ink
                    placeholderTextColor: "#718097"
                    onTextChanged: searchDelay.restart()
                    onClearRequested: {
                        searchDelay.stop()
                        root.torrent.filter_files("")
                    }
                    background: Rectangle {
                        radius: 8
                        color: "#0f1722"
                        border.color: payloadSearch.activeFocus
                                      ? root.accent : root.line
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: !root.torrent.collection_mode
                                    && root.torrent.ready ? 34 : 0
            visible: !root.torrent.collection_mode && root.torrent.ready
            color: "#111923"

            Text {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                verticalAlignment: Text.AlignVCenter
                text: root.torrent.selection_summary
                color: root.torrent.suggested_selection
                       ? root.accentCool : root.muted
                font.pixelSize: 8
                font.weight: root.torrent.suggested_selection
                             ? Font.DemiBold : Font.Normal
                elide: Text.ElideRight
                ToolTip.visible: summaryHover.hovered && truncated
                ToolTip.text: text

                HoverHandler { id: summaryHover }
            }
        }

        ListView {
            id: payloadFiles
            objectName: "torrentPayloadList"
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 2
            model: root.torrent.filtered_file_count
            boundsBehavior: Flickable.StopAtBounds
            flickDeceleration: 4500
            maximumFlickVelocity: 8500
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            delegate: ItemDelegate {
                id: payloadRow
                required property int index
                property int filterRevision: root.torrent.filtered_revision
                property int offerRevision: root.torrent.revision
                property int sourceIndex: {
                    filterRevision
                    return root.torrent.filtered_file_index_at(index)
                }
                property bool chosen: {
                    offerRevision
                    return root.torrent.file_selected_at(sourceIndex)
                }
                property string matchLabel: {
                    offerRevision
                    return root.torrent.file_match_label_at(sourceIndex)
                }

                objectName: "torrentPayloadRow-" + sourceIndex
                width: ListView.view.width
                height: 58
                highlighted: !root.torrent.collection_mode && chosen
                enabled: !root.torrent.busy && sourceIndex >= 0
                Accessible.name: root.torrent.file_path_at(sourceIndex)
                onClicked: {
                    if (!root.torrent.collection_mode)
                        root.torrent.select_file(sourceIndex)
                }

                background: Rectangle {
                    color: payloadRow.highlighted ? "#263342"
                                                   : payloadRow.hovered
                                                     ? "#18212d"
                                                     : "transparent"
                    border.color: payloadRow.highlighted
                                  ? root.accent : "transparent"
                    border.width: payloadRow.highlighted ? 1 : 0
                }

                contentItem: RowLayout {
                    spacing: 10

                    Rectangle {
                        visible: !root.torrent.collection_mode
                        Layout.preferredWidth: 18
                        Layout.preferredHeight: 18
                        radius: 5
                        color: payloadRow.chosen ? root.accent : "transparent"
                        border.color: payloadRow.chosen ? root.accent : "#5a687d"

                        Text {
                            anchors.centerIn: parent
                            text: "✓"
                            visible: payloadRow.chosen
                            color: "#1b140c"
                            font.pixelSize: 12
                            font.weight: Font.Bold
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Text {
                            Layout.fillWidth: true
                            text: {
                                payloadRow.offerRevision
                                return root.torrent.file_path_at(payloadRow.sourceIndex)
                            }
                            color: root.ink
                            font.pixelSize: 10
                            font.weight: Font.Medium
                            elide: Text.ElideMiddle
                        }

                        Text {
                            text: {
                                payloadRow.offerRevision
                                return root.torrent.file_size_at(payloadRow.sourceIndex)
                            }
                            color: root.muted
                            font.pixelSize: 9
                        }
                    }

                    Text {
                        visible: root.torrent.collection_mode
                                 || payloadRow.matchLabel.length > 0
                        text: root.torrent.collection_mode
                              ? "INDEXED" : payloadRow.matchLabel
                        color: root.torrent.collection_mode
                               ? root.accentCool
                               : payloadRow.matchLabel === "BEST MATCH"
                                 ? root.accentCool : root.accent
                        font.pixelSize: 8
                        font.weight: Font.Bold
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                width: parent.width - 50
                visible: root.torrent.filtered_file_count === 0
                text: root.torrent.busy
                      ? "Reading torrent metadata…"
                      : root.torrent.ready && payloadSearch.text.length > 0
                        ? "No torrent filenames match this search."
                        : "Import a .torrent file, choose one already loaded in qBittorrent, or paste a magnet link to inspect exact paths and sizes."
                color: root.muted
                font.pixelSize: 11
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }
    }
}
