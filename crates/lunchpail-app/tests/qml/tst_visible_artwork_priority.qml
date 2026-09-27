import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "VisibleArtworkPriority"
    when: windowShown
    visible: true
    width: 600; height: 500

    Component {
        id: fixture
        GridView {
            id: grid
            width: 400; height: 200
            cellWidth: 100; cellHeight: 100
            cacheBuffer: 600
            reuseItems: true
            clip: true
            model: 200
            property alias priority: reporter
            property alias backend: backend
            QtObject {
                id: backend
                property var ids: []
                property string kind: ""
                property int reports: 0
                property bool media_loading: false
                function set_visible_artwork_games(view, json, artwork) {
                    ids = JSON.parse(json)
                    kind = artwork
                    ++reports
                }
            }
            delegate: Rectangle {
                required property int index
                property string gameId: "game-" + index
                width: 100; height: 100
            }
            Lunchpail.VisibleArtworkPriority {
                id: reporter
                libraryModel: backend
                view: grid
                viewId: "test-grid"
            }
        }
    }

    function test_only_visible_delegates_not_cache_buffer() {
        const grid = createTemporaryObject(fixture, this)
        tryCompare(grid.backend, "reports", 1)
        compare(grid.backend.ids.length, 8)
        verify(grid.backend.ids.every(id => Number(id.slice(5)) < 8))
        verify(grid.contentItem.children.length > 8, "Fixture needs offscreen delegates")
        compare(grid.backend.ids[0], "game-1", "Start near viewport center")
        grid.priority.publish()
        compare(grid.backend.reports, 1, "Unchanged viewport should not hammer the backend")
    }

    function test_new_rows_prioritized_during_continuous_scroll() {
        const grid = createTemporaryObject(fixture, this)
        tryCompare(grid.backend, "reports", 1)
        for (let i = 1; i <= 12; ++i) {
            grid.contentY = i * 40
            wait(16)
        }
        verify(grid.backend.reports >= 3, "Must update during motion, not debounce until release")
        tryVerify(() => grid.backend.ids.includes("game-20"))
        verify(!grid.backend.ids.includes("game-0"))
    }

    function test_hide_resize_and_artwork_type() {
        const grid = createTemporaryObject(fixture, this)
        tryCompare(grid.backend, "reports", 1)
        grid.height = 100
        tryCompare(grid.backend, "ids", ["game-1", "game-2", "game-0", "game-3"])
        grid.priority.artworkType = "clear-logo"
        tryCompare(grid.backend, "kind", "clear-logo")
        grid.visible = false
        compare(grid.backend.ids.length, 0)
        grid.visible = true
        tryCompare(grid.backend, "ids", ["game-1", "game-2", "game-0", "game-3"])
        grid.priority.enabled = false // Desktop library under the Couch Mode overlay.
        compare(grid.backend.ids.length, 0)
        grid.priority.enabled = true
        tryCompare(grid.backend, "ids", ["game-1", "game-2", "game-0", "game-3"])
    }

    function test_loading_completion_retries_same_snapshot() {
        const grid = createTemporaryObject(fixture, this)
        tryCompare(grid.backend, "reports", 1)
        grid.backend.media_loading = true
        wait(80)
        const reports = grid.backend.reports
        grid.backend.media_loading = false
        tryVerify(() => grid.backend.reports > reports)
    }

    function test_reused_delegates_follow_model_reset() {
        const grid = createTemporaryObject(fixture, this)
        tryCompare(grid.backend, "reports", 1)
        grid.positionViewAtIndex(120, GridView.Beginning)
        tryVerify(() => grid.backend.ids.includes("game-120"))
        verify(!grid.backend.ids.includes("game-0"))
        grid.model = 3
        grid.positionViewAtBeginning()
        tryCompare(grid.backend, "ids", ["game-1", "game-2", "game-0"])
    }
}
