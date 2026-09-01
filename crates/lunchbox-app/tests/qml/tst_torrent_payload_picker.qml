import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "TorrentPayloadPicker"
    when: windowShown

    QtObject {
        id: torrentModel
        property bool ready: true
        property bool collection_mode: false
        property bool busy: false
        property bool filter_busy: false
        property bool suggested_selection: true
        property int file_count: 3
        property int filtered_file_count: order.length
        property int filtered_revision: 1
        property int revision: 1
        property int selected_index: 1
        property string game_title: "Super Mario Odyssey"
        property string selection_summary: "Best title match selected automatically."
        property var files: ["Notes/readme.txt",
                             "Switch/Super Mario Odyssey (USA).nsp",
                             "Switch/Mario Kart 8 Deluxe.nsp"]
        property var order: [1, 2, 0]
        property var selected: [1]
        property int filterCalls: 0

        function filtered_file_index_at(row) {
            return row >= 0 && row < order.length ? order[row] : -1
        }
        function filtered_row_for_file_index(index) {
            return order.indexOf(index)
        }
        function file_path_at(index) {
            return index >= 0 && index < files.length ? files[index] : ""
        }
        function file_size_at(index) {
            return index === 0 ? "1 KiB" : "7 GiB"
        }
        function file_selected_at(index) {
            return selected.indexOf(index) >= 0
        }
        function file_match_label_at(index) {
            return index === selected_index
                   ? (suggested_selection ? "BEST MATCH" : "REVIEWED SELECTION")
                   : index === 2 ? "TITLE MATCH" : ""
        }
        function select_file(index) {
            selected = [index]
            selected_index = index
            suggested_selection = false
            revision += 1
        }
        function filter_files(query) {
            filterCalls += 1
            const needle = query.trim().toLowerCase()
            const ranked = [1, 2, 0]
            order = needle.length === 0
                    ? ranked
                    : ranked.filter(index => files[index].toLowerCase().includes(needle))
            filtered_file_count = order.length
            filtered_revision += 1
        }
    }

    Window {
        id: testWindow
        visible: true
        width: 760
        height: 520

        Lunchbox.TorrentPayloadPicker {
            id: picker
            anchors.fill: parent
            torrent: torrentModel
        }
    }

    function init() {
        torrentModel.ready = true
        torrentModel.suggested_selection = true
        torrentModel.selected_index = 1
        torrentModel.selected = [1]
        torrentModel.order = [1, 2, 0]
        torrentModel.filtered_file_count = 3
        torrentModel.filtered_revision += 1
        torrentModel.revision += 1
        torrentModel.filterCalls = 0
        const search = findChild(picker, "torrentPayloadSearch")
        search.text = ""
        wait(150)
        torrentModel.filterCalls = 0
    }

    function test_ranked_default_uses_stable_source_index() {
        tryCompare(torrentModel, "selected_index", 1)
        const row = findChild(picker, "torrentPayloadRow-1")
        verify(row !== null)
        verify(row.chosen)
        compare(row.matchLabel, "BEST MATCH")
    }

    function test_search_filters_and_clear_restores_ranked_order() {
        const search = findChild(picker, "torrentPayloadSearch")
        search.text = "kart"
        tryVerify(() => torrentModel.filterCalls > 0, 1000)
        tryCompare(torrentModel, "filtered_file_count", 1)
        compare(torrentModel.filtered_file_index_at(0), 2)

        const clearButton = findChild(search, "clearSearchButton")
        verify(clearButton.visible)
        mouseClick(clearButton, clearButton.width / 2, clearButton.height / 2)
        tryCompare(torrentModel, "filtered_file_count", 3)
        compare(torrentModel.filtered_file_index_at(0), 1)
    }

    function test_click_selects_original_torrent_position() {
        const row = findChild(picker, "torrentPayloadRow-2")
        verify(row !== null)
        mouseClick(row, row.width / 2, row.height / 2)
        compare(torrentModel.selected_index, 2)
        verify(!torrentModel.suggested_selection)
    }
}
