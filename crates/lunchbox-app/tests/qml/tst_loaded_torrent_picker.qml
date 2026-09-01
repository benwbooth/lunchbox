import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "LoadedTorrentPicker"
    when: windowShown

    QtObject {
        id: torrentModel
        property bool busy: false
        property bool collection_mode: false
        property bool existing_loading: false
        property bool existing_filter_busy: false
        property int existing_count: 20000
        property int existing_filtered_count: 20000
        property int existing_filter_revision: 1
        property int existing_revision: 1
        property string existing_selected_info_hash: ""
        property int existing_selected_count: 0
        property var selectedHashes: []
        property string query: ""
        property int filterCalls: 0
        property int inspectedIndex: -1
        property string inspectedHash: ""
        property bool holdInspectionBusy: false
        property bool popupOpenWhenInspected: false
        property int batchReviewCalls: 0

        function hashFor(index) {
            return index.toString(16).padStart(40, "0")
        }
        function existing_filtered_index_at(row) {
            if (query === "mario")
                return row === 0 ? 17321 : -1
            if (query === "switch complete")
                return row === 0 ? 17321 : -1
            return row >= 0 && row < existing_count ? row : -1
        }
        function existing_filtered_row_for_info_hash(infoHash) {
            if (infoHash.length === 0)
                return -1
            const sourceIndex = parseInt(infoHash, 16)
            if (query.length > 0)
                return sourceIndex === 17321 && existing_filtered_count > 0 ? 0 : -1
            return sourceIndex >= 0 && sourceIndex < existing_count ? sourceIndex : -1
        }
        function existing_name_at(index) {
            return index === 17321
                    ? "Super Mario Odyssey Complete"
                    : "Archive " + index.toString().padStart(5, "0")
        }
        function existing_detail_at(index) {
            return "100% · 8 GiB · pausedUP · "
                    + (index % 2 === 0 ? "switch" : "library")
        }
        function existing_info_hash_at(index) {
            return index >= 0 ? hashFor(index) : ""
        }
        function existing_selected_at(index) {
            return selectedHashes.indexOf(existing_info_hash_at(index)) >= 0
        }
        function toggle_existing_torrent(index) {
            set_existing_torrent_selected(index, !existing_selected_at(index))
        }
        function set_existing_torrent_selected(index, selected) {
            const hash = existing_info_hash_at(index)
            const position = selectedHashes.indexOf(hash)
            if (!selected && position >= 0)
                selectedHashes.splice(position, 1)
            else if (selected && position < 0)
                selectedHashes.push(hash)
            existing_selected_count = selectedHashes.length
            // Match the C++ model: its invokable owns non-QML state and only
            // the explicit revision tells delegates to query it again.
            existing_revision += 1
        }
        function clear_existing_selection() {
            selectedHashes.splice(0, selectedHashes.length)
            existing_selected_count = 0
            existing_revision += 1
        }
        function inspect_selected_existing_torrents() {
            batchReviewCalls += 1
            busy = holdInspectionBusy
        }
        function filter_existing_torrents(value) {
            filterCalls += 1
            query = value.trim().toLowerCase()
            existing_filtered_count = query === "mario" || query === "switch complete"
                    ? 1 : query.length === 0 ? existing_count : 0
            existing_filter_revision += 1
        }
        function inspect_existing_torrent(index) {
            const popup = findChild(picker, "loadedTorrentPopup")
            popupOpenWhenInspected = popup !== null
                    && (popup.opened || popup.visible)
            inspectedIndex = index
            inspectedHash = existing_info_hash_at(index)
            existing_selected_info_hash = inspectedHash
            busy = holdInspectionBusy
        }
    }

    Window {
        id: testWindow
        visible: true
        width: 720
        height: 600

        Lunchbox.LoadedTorrentPicker {
            id: picker
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 24
            torrent: torrentModel
        }
    }

    function init() {
        torrentModel.existing_loading = false
        torrentModel.existing_filter_busy = false
        torrentModel.existing_count = 20000
        torrentModel.existing_filtered_count = 20000
        torrentModel.existing_selected_info_hash = ""
        torrentModel.collection_mode = false
        torrentModel.selectedHashes = []
        torrentModel.existing_selected_count = 0
        torrentModel.query = ""
        torrentModel.filterCalls = 0
        torrentModel.inspectedIndex = -1
        torrentModel.inspectedHash = ""
        torrentModel.holdInspectionBusy = false
        torrentModel.popupOpenWhenInspected = false
        torrentModel.batchReviewCalls = 0
        torrentModel.busy = false
        picker.reset()
        wait(20)
        torrentModel.filterCalls = 0
    }

    function cleanup() {
        picker.reset()
    }

    function editSearch(search, value) {
        search.text = value
        search.textEdited()
    }

    function test_opens_large_virtualized_result_model() {
        const button = findChild(picker, "loadedTorrentDropdownButton")
        mouseClick(button, button.width / 2, button.height / 2)

        const popup = findChild(picker, "loadedTorrentPopup")
        const list = findChild(picker, "loadedTorrentList")
        tryVerify(() => popup.opened)
        compare(list.count, 20000)
        tryVerify(() => findChild(list, "loadedTorrentRow-0") !== null)
        verify(findChild(list, "loadedTorrentRow-17321") === null)
    }

    function test_search_and_click_keep_original_torrent_identity() {
        const search = findChild(picker, "loadedTorrentSearch")
        search.forceActiveFocus()
        editSearch(search, "mario")

        tryVerify(() => torrentModel.filterCalls > 0, 1000)
        tryCompare(torrentModel, "existing_filtered_count", 1)
        const list = findChild(picker, "loadedTorrentList")
        const row = findChild(list, "loadedTorrentRow-17321")
        tryVerify(() => row !== null)
        mouseClick(row, row.width / 2, row.height / 2)

        const popup = findChild(picker, "loadedTorrentPopup")
        tryCompare(torrentModel, "inspectedIndex", 17321)
        compare(torrentModel.inspectedHash, torrentModel.hashFor(17321))
        compare(torrentModel.existing_selected_info_hash, torrentModel.hashFor(17321))
        compare(search.text, "Super Mario Odyssey Complete")
        tryVerify(() => !popup.opened)
        wait(20)
        verify(!popup.opened)

        torrentModel.query = ""
        torrentModel.existing_filtered_count = torrentModel.existing_count
        torrentModel.existing_filter_revision += 1
        torrentModel.existing_revision += 1
        compare(picker.highlightedRow, 17321)
    }

    function test_click_closes_results_while_real_inspection_is_busy() {
        torrentModel.holdInspectionBusy = true
        const button = findChild(picker, "loadedTorrentDropdownButton")
        mouseClick(button, button.width / 2, button.height / 2)

        const popup = findChild(picker, "loadedTorrentPopup")
        const list = findChild(picker, "loadedTorrentList")
        tryVerify(() => popup.opened)
        const row = findChild(list, "loadedTorrentRow-0")
        tryVerify(() => row !== null && row.visible && row.enabled)
        wait(20)
        mouseClick(row, row.width / 2, row.height / 2)

        tryCompare(torrentModel, "inspectedIndex", 0)
        tryVerify(() => !popup.opened)
        verify(!popup.visible)
        compare(popup.opacity, 0)
        verify(!popup.enabled)
        verify(!picker.resultsRequested)
        verify(!torrentModel.popupOpenWhenInspected)
        torrentModel.busy = false
        wait(20)
        verify(!popup.opened)
        verify(!popup.visible)
        compare(popup.opacity, 0)
        verify(!popup.enabled)
        verify(!picker.resultsRequested)
    }

    function test_filtered_row_selection_keeps_original_source_index() {
        const search = findChild(picker, "loadedTorrentSearch")
        editSearch(search, "switch complete")

        tryCompare(torrentModel, "existing_filtered_count", 1, 1000)
        compare(picker.highlightedRow, 0)
        picker.chooseRow(0)

        tryCompare(torrentModel, "inspectedIndex", 17321)
        compare(torrentModel.inspectedHash, torrentModel.hashFor(17321))
    }

    function test_collection_mode_selects_multiple_sources_before_review() {
        torrentModel.collection_mode = true
        torrentModel.existing_count = 3
        torrentModel.existing_filtered_count = 3
        const button = findChild(picker, "loadedTorrentDropdownButton")
        mouseClick(button, button.width / 2, button.height / 2)

        const popup = findChild(picker, "loadedTorrentPopup")
        const list = findChild(picker, "loadedTorrentList")
        tryVerify(() => popup.opened)
        const first = findChild(list, "loadedTorrentRow-0")
        const second = findChild(list, "loadedTorrentRow-1")
        tryVerify(() => first !== null && second !== null)
        mouseClick(first, first.width / 2, first.height / 2)
        mouseClick(second, second.width / 2, second.height / 2)

        compare(torrentModel.existing_selected_count, 2)
        verify(popup.opened)
        compare(torrentModel.inspectedIndex, -1)

        const review = findChild(picker, "loadedTorrentReviewSelected")
        verify(review !== null && review.enabled)
        mouseClick(review, review.width / 2, review.height / 2)
        tryCompare(torrentModel, "batchReviewCalls", 1)
        tryVerify(() => !popup.opened)
    }

    function test_registered_platform_sources_are_checked_when_picker_opens() {
        torrentModel.collection_mode = true
        torrentModel.existing_count = 2
        torrentModel.existing_filtered_count = 2
        torrentModel.selectedHashes = [torrentModel.hashFor(0),
                                       torrentModel.hashFor(1)]
        torrentModel.existing_selected_count = 2
        torrentModel.existing_revision += 1

        const button = findChild(picker, "loadedTorrentDropdownButton")
        mouseClick(button, button.width / 2, button.height / 2)

        const popup = findChild(picker, "loadedTorrentPopup")
        const list = findChild(picker, "loadedTorrentList")
        tryVerify(() => popup.opened)
        const first = findChild(list, "loadedTorrentRow-0")
        const second = findChild(list, "loadedTorrentRow-1")
        tryVerify(() => first !== null && second !== null)
        verify(first.batchSelected)
        verify(second.batchSelected)
        compare(torrentModel.existing_selected_count, 2)
    }

    function test_game_mode_checkboxes_select_platform_sources_with_shift_range() {
        torrentModel.collection_mode = false
        torrentModel.existing_count = 4
        torrentModel.existing_filtered_count = 4
        const button = findChild(picker, "loadedTorrentDropdownButton")
        mouseClick(button, button.width / 2, button.height / 2)

        const popup = findChild(picker, "loadedTorrentPopup")
        const list = findChild(picker, "loadedTorrentList")
        tryVerify(() => popup.opened)
        const firstRow = findChild(list, "loadedTorrentRow-0")
        const secondRow = findChild(list, "loadedTorrentRow-1")
        tryVerify(() => firstRow !== null && secondRow !== null)
        const first = findChild(firstRow, "loadedTorrentCheckbox-0")
        const second = findChild(secondRow, "loadedTorrentCheckbox-1")
        verify(first !== null && second !== null)
        mouseClick(first, first.width / 2, first.height / 2)
        compare(torrentModel.existing_selected_count, 1)
        tryVerify(() => firstRow.batchSelected)
        compare(torrentModel.inspectedIndex, -1)
        verify(popup.opened)
        picker.toggleCheckbox(1, Qt.ShiftModifier)

        compare(torrentModel.existing_selected_count, 2)
        verify(torrentModel.existing_selected_at(0))
        verify(torrentModel.existing_selected_at(1))
        compare(torrentModel.inspectedIndex, -1)
        verify(popup.opened)

        const review = findChild(picker, "loadedTorrentReviewSelected")
        verify(review !== null && review.enabled)
        mouseClick(review, review.width / 2, review.height / 2)
        tryCompare(torrentModel, "batchReviewCalls", 1)
        tryVerify(() => !popup.opened)
    }
}
