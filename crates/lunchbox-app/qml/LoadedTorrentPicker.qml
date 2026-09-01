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
    property int highlightedRow: -1

    function reset(restoreResults) {
        searchDelay.stop()
        torrentPopup.close()
        torrentSearch.text = ""
        highlightedRow = -1
        if (restoreResults === undefined || restoreResults)
            torrent.filter_existing_torrents("")
    }

    function openResults() {
        if (!enabled || torrent.existing_loading || torrent.existing_count === 0)
            return
        torrentPopup.open()
        if (highlightedRow < 0 && torrent.existing_filtered_count > 0)
            highlightedRow = 0
        Qt.callLater(function() {
            if (torrentPopup.opened)
                torrentSearch.forceActiveFocus()
            revealHighlight()
        })
    }

    function revealHighlight() {
        if (highlightedRow >= 0)
            torrentList.positionViewAtIndex(highlightedRow, ListView.Contain)
    }

    function syncHighlight() {
        const selectedRow = torrent.existing_filtered_row_for_info_hash(
                                  torrent.existing_selected_info_hash)
        highlightedRow = selectedRow >= 0
                ? selectedRow
                : torrent.existing_filtered_count > 0 ? 0 : -1
        Qt.callLater(revealHighlight)
    }

    function chooseRow(row) {
        const sourceIndex = torrent.existing_filtered_index_at(row)
        if (sourceIndex < 0)
            return
        highlightedRow = row
        torrentSearch.text = torrent.existing_name_at(sourceIndex)
        torrentPopup.close()
        torrent.inspect_existing_torrent(sourceIndex)
    }

    implicitHeight: 42
    radius: 8
    color: "#0f1722"
    border.color: torrentSearch.activeFocus || torrentPopup.opened
                  ? root.accent : root.line
    enabled: !torrent.busy && !torrent.existing_loading
    onVisibleChanged: {
        if (!visible)
            torrentPopup.close()
    }
    onEnabledChanged: {
        if (!enabled)
            torrentPopup.close()
    }

    Timer {
        id: searchDelay
        interval: 120
        repeat: false
        onTriggered: root.torrent.filter_existing_torrents(torrentSearch.text)
    }

    Connections {
        target: root.torrent

        function onExisting_filter_revisionChanged() {
            root.syncHighlight()
        }

        function onExisting_revisionChanged() {
            if (root.torrent.existing_selected_info_hash.length === 0)
                torrentSearch.text = ""
            root.syncHighlight()
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        ClearableSearchField {
            id: torrentSearch
            objectName: "loadedTorrentSearch"
            Layout.fillWidth: true
            Layout.fillHeight: true
            leftPadding: 40
            searchIconVisible: true
            enabled: root.enabled
            color: root.ink
            placeholderTextColor: "#718097"
            placeholderText: root.torrent.existing_loading
                             ? "Checking qBittorrent…"
                             : root.torrent.existing_count > 0
                               ? "Search " + root.torrent.existing_count + " loaded torrents"
                               : "No loaded torrents available"
            font.pixelSize: 10
            background: Item {}
            onActiveFocusChanged: {
                if (activeFocus)
                    root.openResults()
            }
            onTextEdited: {
                searchDelay.restart()
                root.openResults()
            }
            onClearRequested: {
                searchDelay.stop()
                root.torrent.filter_existing_torrents("")
                root.openResults()
            }

            Keys.priority: Keys.BeforeItem
            Keys.onPressed: event => {
                if (event.key === Qt.Key_Down) {
                    root.openResults()
                    if (root.torrent.existing_filtered_count > 0)
                        root.highlightedRow = Math.min(
                                    root.torrent.existing_filtered_count - 1,
                                    Math.max(0, root.highlightedRow + 1))
                    root.revealHighlight()
                    event.accepted = true
                } else if (event.key === Qt.Key_Up) {
                    root.openResults()
                    if (root.torrent.existing_filtered_count > 0)
                        root.highlightedRow = Math.max(0, root.highlightedRow - 1)
                    root.revealHighlight()
                    event.accepted = true
                } else if (event.key === Qt.Key_Return
                           || event.key === Qt.Key_Enter) {
                    root.openResults()
                    if (root.highlightedRow >= 0)
                        root.chooseRow(root.highlightedRow)
                    event.accepted = true
                } else if (event.key === Qt.Key_Escape && torrentPopup.opened) {
                    torrentPopup.close()
                    event.accepted = true
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: root.line
        }

        ToolButton {
            id: dropdownButton
            objectName: "loadedTorrentDropdownButton"
            Layout.preferredWidth: 42
            Layout.fillHeight: true
            enabled: root.enabled && root.torrent.existing_count > 0
            text: torrentPopup.opened ? "▴" : "▾"
            Accessible.name: torrentPopup.opened
                             ? "Close loaded torrent results"
                             : "Show loaded torrents"
            onClicked: torrentPopup.opened ? torrentPopup.close() : root.openResults()
            background: Rectangle {
                color: dropdownButton.down ? "#344156"
                       : dropdownButton.hovered ? "#273346" : "transparent"
                radius: 7
            }
            contentItem: Text {
                text: dropdownButton.text
                color: dropdownButton.enabled ? root.ink : "#657287"
                font.pixelSize: 14
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        BusyIndicator {
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            Layout.rightMargin: 6
            visible: root.torrent.existing_filter_busy
            running: visible
        }
    }

    Popup {
        id: torrentPopup
        objectName: "loadedTorrentPopup"
        x: 0
        y: root.height + 5
        width: root.width
        height: Math.min(430, Math.max(76,
                         root.torrent.existing_filtered_count * 62 + 38))
        padding: 1
        focus: false
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: "#111923"
            radius: 10
            border.color: root.line
        }

        contentItem: ColumnLayout {
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 36
                color: "#151f2c"

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 8

                    Text {
                        Layout.fillWidth: true
                        text: root.torrent.existing_filter_busy
                              ? "SEARCHING…"
                              : root.torrent.existing_filtered_count + " OF "
                                + root.torrent.existing_count + " TORRENTS"
                        color: root.torrent.existing_filter_busy
                               ? root.accent : root.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }

                    Text {
                        text: "↑↓  SELECT   ENTER  OPEN"
                        color: "#657287"
                        font.pixelSize: 8
                    }
                }
            }

            ListView {
                id: torrentList
                objectName: "loadedTorrentList"
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.torrent.existing_filtered_count
                currentIndex: root.highlightedRow
                boundsBehavior: Flickable.StopAtBounds
                flickDeceleration: 4500
                maximumFlickVelocity: 8500
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                delegate: ItemDelegate {
                    id: torrentRow
                    required property int index
                    property int filterRevision: root.torrent.existing_filter_revision
                    property int listRevision: root.torrent.existing_revision
                    property int sourceIndex: {
                        filterRevision
                        return root.torrent.existing_filtered_index_at(index)
                    }
                    property string sourceHash: {
                        listRevision
                        return root.torrent.existing_info_hash_at(sourceIndex)
                    }
                    property bool selected: sourceHash.length > 0
                                            && sourceHash === root.torrent.existing_selected_info_hash

                    objectName: "loadedTorrentRow-" + sourceIndex
                    width: ListView.view.width
                    height: 62
                    highlighted: index === root.highlightedRow || selected
                    enabled: sourceIndex >= 0 && !root.torrent.busy
                    Accessible.name: root.torrent.existing_name_at(sourceIndex)
                    onHoveredChanged: {
                        if (hovered)
                            root.highlightedRow = index
                    }
                    onClicked: root.chooseRow(index)

                    background: Rectangle {
                        color: torrentRow.selected ? "#213a39"
                               : torrentRow.highlighted ? "#263342"
                               : torrentRow.hovered ? "#18212d" : "transparent"
                        border.color: torrentRow.selected
                                      ? root.accentCool
                                      : torrentRow.highlighted ? root.accent : "transparent"
                        border.width: torrentRow.highlighted ? 1 : 0
                    }

                    contentItem: RowLayout {
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 10

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Text {
                                Layout.fillWidth: true
                                text: root.torrent.existing_name_at(torrentRow.sourceIndex)
                                color: root.ink
                                font.pixelSize: 10
                                font.weight: Font.DemiBold
                                elide: Text.ElideMiddle
                            }

                            Text {
                                Layout.fillWidth: true
                                text: root.torrent.existing_detail_at(torrentRow.sourceIndex)
                                color: root.muted
                                font.pixelSize: 8
                                elide: Text.ElideRight
                            }
                        }

                        Text {
                            Layout.preferredWidth: 92
                            text: torrentRow.sourceHash.length > 12
                                  ? torrentRow.sourceHash.slice(0, 6) + "…"
                                    + torrentRow.sourceHash.slice(-6)
                                  : torrentRow.sourceHash
                            color: torrentRow.selected ? root.accentCool : "#718097"
                            font.family: "monospace"
                            font.pixelSize: 8
                            horizontalAlignment: Text.AlignRight
                        }
                    }
                }

                Text {
                    anchors.centerIn: parent
                    width: parent.width - 40
                    visible: root.torrent.existing_filtered_count === 0
                    text: root.torrent.existing_filter_busy
                          ? "Searching loaded torrents…"
                          : "No loaded torrent matches this search."
                    color: root.muted
                    font.pixelSize: 10
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                }
            }
        }
    }
}
