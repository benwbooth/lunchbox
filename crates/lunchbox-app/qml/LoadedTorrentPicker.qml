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
    property string rangeAnchorHash: ""
    property bool selectionInProgress: false
    property bool resultsRequested: false

    function closeResults() {
        resultsRequested = false
        torrentPopup.close()
        torrentPopup.visible = false
        torrentSearch.focus = false
    }

    function reset(restoreResults) {
        searchDelay.stop()
        selectionInProgress = false
        closeResults()
        torrentSearch.text = ""
        highlightedRow = -1
        rangeAnchorHash = ""
        if (restoreResults === undefined || restoreResults)
            torrent.filter_existing_torrents("")
    }

    function openResults() {
        if (selectionInProgress || !enabled || torrent.existing_loading
                || torrent.existing_count === 0)
            return
        resultsRequested = true
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
        if (torrent.collection_mode) {
            if (highlightedRow >= torrent.existing_filtered_count)
                highlightedRow = torrent.existing_filtered_count > 0 ? 0 : -1
            Qt.callLater(revealHighlight)
            return
        }
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
        if (torrent.collection_mode) {
            highlightedRow = row
            torrent.toggle_existing_torrent(sourceIndex)
            return
        }
        selectionInProgress = true
        searchDelay.stop()
        highlightedRow = row
        torrentSearch.text = torrent.existing_name_at(sourceIndex)
        torrentSearch.focus = false

        // Hide the overlay before model inspection can disable or relayout the
        // picker. Popup.close() alone is asynchronous on some real Wayland
        // compositors even when the transition is empty.
        closeResults()
        Qt.callLater(function() {
            root.closeResults()
            root.torrent.inspect_existing_torrent(sourceIndex)
            if (!root.torrent.busy)
                root.selectionInProgress = false
        })
    }

    function toggleCheckbox(row, modifiers) {
        const sourceIndex = torrent.existing_filtered_index_at(row)
        if (sourceIndex < 0)
            return
        const sourceHash = torrent.existing_info_hash_at(sourceIndex)
        const shouldSelect = !torrent.existing_selected_at(sourceIndex)
        const anchorRow = rangeAnchorHash.length > 0
                ? torrent.existing_filtered_row_for_info_hash(rangeAnchorHash) : -1
        if ((modifiers & Qt.ShiftModifier) !== 0 && anchorRow >= 0) {
            const first = Math.min(anchorRow, row)
            const last = Math.max(anchorRow, row)
            for (let filteredRow = first; filteredRow <= last; ++filteredRow) {
                const candidate = torrent.existing_filtered_index_at(filteredRow)
                if (candidate >= 0)
                    torrent.set_existing_torrent_selected(candidate, shouldSelect)
            }
        } else {
            torrent.set_existing_torrent_selected(sourceIndex, shouldSelect)
        }
        highlightedRow = row
        rangeAnchorHash = sourceHash
    }

    implicitHeight: 42
    radius: 8
    color: "#0f1722"
    border.color: torrentSearch.activeFocus
                  || (root.resultsRequested && torrentPopup.opened)
                  ? root.accent : root.line
    enabled: !torrent.busy && !torrent.existing_loading
    onVisibleChanged: {
        if (!visible)
            closeResults()
    }
    onEnabledChanged: {
        if (!enabled)
            closeResults()
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
            if (!root.torrent.collection_mode
                    && root.torrent.existing_selected_info_hash.length === 0)
                torrentSearch.text = ""
            root.syncHighlight()
        }

        function onBusyChanged() {
            if (!root.torrent.busy && root.selectionInProgress) {
                root.closeResults()
                root.selectionInProgress = false
            }
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
                    root.closeResults()
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
            text: root.resultsRequested && torrentPopup.opened ? "▴" : "▾"
            Accessible.name: root.resultsRequested && torrentPopup.opened
                             ? "Close loaded torrent results"
                             : "Show loaded torrents"
            onClicked: root.resultsRequested && torrentPopup.opened
                       ? root.closeResults() : root.openResults()
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
                         root.torrent.existing_filtered_count * 62 + 38
                         + 52))
        enabled: root.resultsRequested && !root.selectionInProgress
        opacity: root.resultsRequested && !root.selectionInProgress ? 1 : 0
        padding: 1
        focus: false
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        enter: Transition {}
        exit: Transition {}
        onClosed: {
            root.resultsRequested = false
            torrentSearch.focus = false
        }

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
                        text: root.torrent.collection_mode
                              ? "ROW/ENTER  TOGGLE   SHIFT+CHECK  RANGE"
                              : "ROW  OPEN GAME   CHECK  ADD PLATFORM"
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

                delegate: Item {
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
                    property bool batchSelected: sourceHash.length > 0
                                                 && root.torrent.existing_selected_at(sourceIndex)
                    property bool exactSelected: sourceHash.length > 0
                                                 && sourceHash === root.torrent.existing_selected_info_hash
                    property bool selected: batchSelected || exactSelected

                    objectName: "loadedTorrentRow-" + sourceIndex
                    width: ListView.view.width
                    height: 62
                    property bool highlighted: index === root.highlightedRow || selected
                    property bool hovered: torrentRowMouse.containsMouse
                    enabled: sourceIndex >= 0 && !root.torrent.busy
                    Accessible.name: root.torrent.existing_name_at(sourceIndex)

                    Rectangle {
                        anchors.fill: parent
                        color: torrentRow.selected ? "#213a39"
                               : torrentRow.highlighted ? "#263342"
                               : torrentRow.hovered ? "#18212d" : "transparent"
                        border.color: torrentRow.selected
                                      ? root.accentCool
                                      : torrentRow.highlighted ? root.accent : "transparent"
                        border.width: torrentRow.highlighted ? 1 : 0
                    }

                    MouseArea {
                        id: torrentRowMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onEntered: root.highlightedRow = torrentRow.index
                        onClicked: root.chooseRow(torrentRow.index)
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 10

                        Rectangle {
                            id: torrentCheckbox
                            objectName: "loadedTorrentCheckbox-" + torrentRow.sourceIndex
                            Layout.preferredWidth: 20
                            Layout.preferredHeight: 20
                            radius: 5
                            color: torrentRow.batchSelected ? root.accentCool : "transparent"
                            border.color: torrentRow.batchSelected ? root.accentCool : "#657287"
                            border.width: 1
                            Text {
                                anchors.centerIn: parent
                                text: "✓"
                                visible: torrentRow.batchSelected
                                color: "#10201d"
                                font.pixelSize: 12
                                font.weight: Font.Bold
                            }
                            MouseArea {
                                anchors.fill: parent
                                anchors.margins: -7
                                cursorShape: Qt.PointingHandCursor
                                onClicked: mouse => root.toggleCheckbox(
                                               torrentRow.index,
                                               mouse.modifiers
                                               | Qt.application.keyboardModifiers)
                            }
                        }

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

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 52
                color: "#151f2c"
                border.color: root.line

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 10
                    spacing: 8

                    Text {
                        objectName: "loadedTorrentSelectedCount"
                        Layout.fillWidth: true
                        text: root.torrent.existing_selected_count + " PLATFORM "
                              + (root.torrent.existing_selected_count === 1
                                 ? "SOURCE" : "SOURCES")
                        color: root.torrent.existing_selected_count > 0
                               ? root.accentCool : root.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                    }

                    Button {
                        objectName: "loadedTorrentClearSelection"
                        Layout.preferredWidth: 72
                        Layout.preferredHeight: 32
                        text: "CLEAR"
                        enabled: root.torrent.existing_selected_count > 0
                        onClicked: root.torrent.clear_existing_selection()
                    }

                    Button {
                        id: reviewSelectedButton
                        objectName: "loadedTorrentReviewSelected"
                        Layout.preferredWidth: 146
                        Layout.preferredHeight: 32
                        text: root.torrent.existing_selected_count === 1
                              ? "REVIEW 1 SOURCE"
                              : "REVIEW " + root.torrent.existing_selected_count + " SOURCES"
                        enabled: root.torrent.existing_selected_count > 0
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        onClicked: {
                            root.selectionInProgress = true
                            root.closeResults()
                            Qt.callLater(function() {
                                root.torrent.inspect_selected_existing_torrents()
                                if (!root.torrent.busy)
                                    root.selectionInProgress = false
                            })
                        }
                        background: Rectangle {
                            radius: 7
                            color: reviewSelectedButton.enabled
                                   ? (reviewSelectedButton.down ? "#d68d36" : root.accent)
                                   : "#26313e"
                        }
                        contentItem: Text {
                            text: reviewSelectedButton.text
                            color: reviewSelectedButton.enabled ? "#1b140c" : root.muted
                            font: reviewSelectedButton.font
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
            }
        }
    }
}
