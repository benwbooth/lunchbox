import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "LibraryAuditHistory"
    when: windowShown

    Component {
        id: hostComponent
        ApplicationWindow {
            width: 900
            height: 700
            visible: true

            QtObject {
                id: historyState
                property int history_count: 3
                function history_finished_at(index) { return 1788249600 - index * 86400 }
                function history_total_count_at(index) { return 100 - index }
                function history_issue_count_at(index) { return 5 + index }
                function history_summary_at(index) {
                    return (95 - index) + " healthy · 1 missing · 1 changed · 2 duplicate · 1 new · 0 offline"
                }
                function history_comparison_at(index) {
                    return index < 2 ? "1 fewer maintenance issue than the previous complete audit"
                                     : "Oldest retained baseline"
                }
            }

            Lunchbox.LibraryAuditHistory {
                id: history
                anchors.centerIn: parent
                auditModel: historyState
            }

            property alias history: history
            property alias historyState: historyState
        }
    }

    function test_history_is_virtualized_newest_first_and_keyboard_scrollable() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.history.open()
        tryVerify(() => host.history.visible)

        const list = findChild(host.history, "libraryAuditHistoryList")
        verify(list)
        compare(list.count, 3)
        compare(list.currentIndex, 0)
        tryVerify(() => list.itemAtIndex(0) !== null)
        const latest = findChild(list.itemAtIndex(0), "libraryAuditLatestLabel")
        verify(latest)
        verify(latest.text.length > 0)

        verify(list.handleNavigation(Qt.Key_End))
        tryCompare(list, "currentIndex", 2)
        verify(list.handleNavigation(Qt.Key_Home))
        tryCompare(list, "currentIndex", 0)
        compare(list.handleNavigation(Qt.Key_A), false)
    }

    function test_empty_history_has_no_rows() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.historyState.history_count = 0
        host.history.open()
        tryVerify(() => host.history.visible)
        const list = findChild(host.history, "libraryAuditHistoryList")
        verify(list)
        compare(list.count, 0)
        compare(list.currentIndex, -1)
    }
}
