import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "WatchedTorrentInbox"
    when: windowShown

    Component {
        id: hostComponent

        Window {
            visible: true
            width: 900
            height: 520

            QtObject {
                id: settingsState
                property string watched_torrent_directory: "/tmp/torrent inbox"
                property int chooseCount: 0
                function choose_native_directory(field) {
                    if (field === "torrent-watch")
                        ++chooseCount
                }
            }

            QtObject {
                id: inboxState
                property bool busy: false
                property int entry_count: 1
                property int pending_count: 1
                property int registered_count: 0
                property int invalid_count: 0
                property int revision: 1
                property string directory: "/tmp/torrent inbox"
                property string message: "Watching /tmp/torrent inbox · 1 ready for review"
                property string entryStatus: "pending"
                property int refreshCount: 0
                function refresh() { ++refreshCount }
                function file_name_at(index) { return "switch-collection.torrent" }
                function file_path_at(index) { return "/tmp/torrent inbox/switch-collection.torrent" }
                function file_url_at(index) { return "file:///tmp/torrent%20inbox/switch-collection.torrent" }
                function torrent_name_at(index) { return "Switch Collection" }
                function info_hash_at(index) { return "0123456789abcdef0123456789abcdef01234567" }
                function detail_at(index) {
                    return entryStatus === "registered"
                            ? "Registered for Nintendo Switch"
                            : "Ready for platform review · 500 files · 2 TiB"
                }
                function status_at(index) { return entryStatus }
                function can_review_at(index) { return entryStatus === "pending" }
            }

            Lunchbox.WatchedTorrentInbox {
                id: inbox
                anchors.fill: parent
                settingsModel: settingsState
                inboxModel: inboxState
            }

            property alias inbox: inbox
            property alias inboxState: inboxState
            property alias settingsState: settingsState
        }
    }

    function test_pending_source_routes_exact_local_url_to_review() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const spy = signalSpy.createObject(testCase, {
            target: host.inbox,
            signalName: "reviewRequested"
        })
        verify(spy)

        const list = findChild(host.inbox, "watchedTorrentList")
        tryVerify(() => list.itemAtIndex(0) !== null)
        const review = findChild(host.inbox, "watchedTorrentReview-0")
        verify(review)
        verify(review.visible)
        mouseClick(review, review.width / 2, review.height / 2)

        compare(spy.count, 1)
        compare(spy.signalArguments[0][0].toString(),
                "file:///tmp/torrent inbox/switch-collection.torrent")
    }

    function test_registered_source_is_visible_but_not_actionable() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.inboxState.entryStatus = "registered"
        host.inboxState.pending_count = 0
        host.inboxState.registered_count = 1
        ++host.inboxState.revision

        const list = findChild(host.inbox, "watchedTorrentList")
        tryVerify(() => list.itemAtIndex(0) !== null)
        const review = findChild(host.inbox, "watchedTorrentReview-0")
        verify(review)
        compare(review.visible, false)
    }

    Component {
        id: signalSpy
        SignalSpy { }
    }
}
