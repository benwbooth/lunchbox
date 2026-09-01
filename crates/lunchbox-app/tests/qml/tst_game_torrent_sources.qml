import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GameTorrentSources"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 720
            height: 700
            property int reviewedCandidate: -1

            QtObject {
                id: detailsState
                property int detail_revision: 1
                property int registered_torrent_source_count: 2
                property bool torrent_loading: false
                property bool download_busy: false
                property string platform: "Nintendo Switch"
                function download_source_count() { return 1 }
                function download_source_bundle_at(index) { return 0 }
                function bundle_file_count_at(index) { return 1 }
                function bundle_title_at(index) { return "MIG Switch Game Collection" }
                function bundle_detail_at(index) { return "5,475 indexed members" }
                function bundle_file_name_at(bundle, file) {
                    return "Super Mario Odyssey.xci/Super Mario Odyssey.xci"
                }
                function bundle_file_detail_at(bundle, file) {
                    return "BEST MATCH · 7.4 GiB"
                }
                function select_bundle_file(bundle, file) { return 17 }
            }

            Lunchbox.GameTorrentSources {
                id: sources
                width: 640
                detailsModel: detailsState
                showAddSource: true
                onReviewCandidateRequested: function(candidateIndex) {
                    parent.reviewedCandidate = candidateIndex
                }
            }

            SignalSpy {
                id: reviewSpy
                target: sources
                signalName: "reviewCandidateRequested"
            }

            property alias sources: sources
            property alias reviewSpy: reviewSpy
        }
    }

    function test_platform_count_candidate_and_control_order() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const count = findChild(host.sources, "registeredTorrentSourceCount")
        const source = findChild(host.sources, "torrentSource-0")
        const add = findChild(host.sources, "addTorrentSourceButton")
        verify(count)
        verify(source)
        verify(add)
        compare(count.text, "2 REGISTERED")
        verify(add.mapToItem(host.sources, 0, 0).y
               >= source.mapToItem(host.sources, 0, source.height).y)

        const get = findChild(host.sources, "getTorrentCandidate-0")
        verify(get)
        get.click()
        compare(host.reviewSpy.count, 1)
        compare(host.reviewSpy.signalArguments[0][0], 17)
    }

    function test_installed_game_hides_candidates_until_alternatives_are_expanded() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.sources.installed = true
        compare(host.sources.shouldShowSources, false)
        compare(host.sources.height, 0)
        host.sources.alternativesExpanded = true
        compare(host.sources.alternativesExpanded, true)
        compare(host.sources.showAddSource, true)
        compare(host.sources.matchingSourceCount, 1)
        compare(host.sources.shouldShowSources, true)
        verify(findChild(host.sources, "torrentCandidate-0"))
    }
}
