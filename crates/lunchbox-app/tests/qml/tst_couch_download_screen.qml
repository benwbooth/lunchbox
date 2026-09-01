import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "CouchDownloadScreen"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 1920
            height: 1200

            QtObject {
                id: detailsState
                property int detail_revision: 1
                property bool torrent_loading: false
                property string message: "3 candidates ranked"
                property bool download_busy: false
                property bool download_preflight_busy: false
                property bool download_preflight_ready: false
                property string download_preflight_action: ""
                property string download_preflight_status: "Checking storage"
                property string download_preflight_mode: "139 KiB exact file"
                property string download_preflight_storage: "Enough space"
                property string download_preflight_destination: "/roms/Faxanadu.zip"
                property int selectedIndex: -1
                property int inspectedIndex: -1
                property int queuedIndex: -1
                property int candidateCount: 3

                function download_candidate_count() { return candidateCount }
                function download_candidate_source_at(index) {
                    return index < 2 ? "No-Intro · Nintendo Entertainment System"
                                     : "FinalBurn Neo"
                }
                function download_candidate_name_at(index) {
                    return ["Faxanadu (USA) (Rev 1).zip",
                            "Faxanadu (Europe).zip",
                            "nes/faxanadu.zip"][index]
                }
                function download_candidate_detail_at(index) {
                    return index === 0 ? "BEST MATCH · USA · 139 KiB"
                                       : "Exact title match"
                }
                function select_download_candidate(index) {
                    selectedIndex = index
                    return index
                }
                function inspect_download(index) { inspectedIndex = index }
                function queue_file(index) {
                    queuedIndex = index
                    message = "Faxanadu was added to Downloads."
                    download_busy = true
                }
            }

            Lunchbox.CouchDownloadScreen {
                id: downloadScreen
                anchors.fill: parent
                details: detailsState
                active: true
                gameTitle: "Faxanadu"
                platformName: "Nintendo Entertainment System"
                coverUrl: ""
                heroUrl: ""
            }

            SignalSpy {
                id: configureSpy
                target: downloadScreen
                signalName: "configureRequested"
            }

            SignalSpy {
                id: closeSpy
                target: downloadScreen
                signalName: "closeRequested"
            }

            SignalSpy {
                id: importSpy
                target: downloadScreen
                signalName: "importTorrentRequested"
            }

            property alias detailsState: detailsState
            property alias downloadScreen: downloadScreen
            property alias configureSpy: configureSpy
            property alias closeSpy: closeSpy
            property alias importSpy: importSpy
        }
    }

    function test_controller_flow_reviews_setup_and_queues_exact_candidate() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        compare(host.downloadScreen.candidateCount, 3)
        compare(host.downloadScreen.selectedCandidate, 0)

        verify(host.downloadScreen.handleNavigation("down"))
        compare(host.downloadScreen.selectedCandidate, 1)
        verify(host.downloadScreen.handleNavigation("accept"))
        compare(host.downloadScreen.phase, "review")
        compare(host.detailsState.selectedIndex, 1)
        compare(host.detailsState.inspectedIndex, 1)

        host.detailsState.download_preflight_action = "configure_qbittorrent"
        host.downloadScreen.activateReview()
        compare(host.configureSpy.count, 1)

        host.detailsState.download_preflight_action = ""
        host.detailsState.download_preflight_ready = true
        host.downloadScreen.activateReview()
        compare(host.detailsState.queuedIndex, 1)
        compare(host.downloadScreen.phase, "queue")

        host.detailsState.download_busy = false
        tryCompare(host.downloadScreen, "phase", "result")
        verify(host.downloadScreen.handleNavigation("accept"))
        compare(host.closeSpy.count, 1)
    }

    function test_back_from_review_keeps_the_ranked_list_in_couch_mode() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.downloadScreen.beginReview()
        compare(host.downloadScreen.phase, "review")
        verify(host.downloadScreen.handleNavigation("back"))
        compare(host.downloadScreen.phase, "candidates")
        compare(host.closeSpy.count, 0)
        verify(host.downloadScreen.handleNavigation("back"))
        compare(host.closeSpy.count, 1)
    }

    function test_empty_download_state_imports_a_torrent() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.detailsState.candidateCount = 0
        host.detailsState.detail_revision += 1
        tryCompare(host.downloadScreen, "candidateCount", 0)
        compare(host.downloadScreen.selectedCandidate, -1)
        verify(host.downloadScreen.handleNavigation("accept"))
        compare(host.importSpy.count, 1)

        host.detailsState.torrent_loading = true
        verify(host.downloadScreen.handleNavigation("accept"))
        compare(host.importSpy.count, 1)
    }
}
