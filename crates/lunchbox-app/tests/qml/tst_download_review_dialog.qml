import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "DownloadReviewDialog"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 1000
            height: 800

            QtObject {
                id: detailsState
                property string title: "Faxanadu"
                property int detail_revision: 0
                property int file_count: 1
                property bool registeredSource: false
                property bool download_busy: false
                property bool download_preflight_busy: false
                property bool download_preflight_ready: false
                property bool download_preflight_terminal: false
                property string download_preflight_action: "configure_qbittorrent"
                property string download_preflight_status: "One-time download-folder setup"
                property string download_preflight_mode: ""
                property string download_preflight_storage: ""
                property string download_preflight_destination: ""
                property int inspectCount: 0

                function file_has_download_plan(index) { return false }
                function file_plan_kind_at(index) { return "" }
                function file_name_at(index) { return "FinalBurn Neo/nes/faxanadu.zip" }
                function file_detail_at(index) { return "BEST MATCH · 100% title match" }
                function file_plan_members_at(index) { return "" }
                function inspect_download(index) { ++inspectCount }
                function selected_source_is_registered() { return registeredSource }
            }

            QtObject {
                id: settingsState
                property bool download_entire_torrent: false
            }

            Lunchbox.DownloadReviewDialog {
                id: review
                anchors.centerIn: parent
                detailsModel: detailsState
                settingsModel: settingsState
                ink: "#f4f7fb"
                muted: "#94a0b3"
                panel: "#111927"
                panelRaised: "#182233"
                line: "#2b384b"
                accent: "#f1a23b"
                accentCool: "#5de2d2"
            }

            property alias review: review
            property alias detailsState: detailsState
        }
    }

    function test_plain_language_setup_and_ready_states() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)

        host.review.openFor(0)
        compare(host.detailsState.inspectCount, 1)
        compare(host.review.reviewTitle, "DOWNLOAD FAXANADU")
        compare(host.review.readinessLabel, "SETUP NEEDED")

        host.detailsState.download_preflight_action = ""
        host.detailsState.download_preflight_ready = true
        compare(host.review.readinessLabel, "READY")
        host.detailsState.registeredSource = true
        ++host.detailsState.detail_revision
        compare(host.review.selectiveOnly, true)
        host.review.close()
    }

    function test_terminal_block_hides_retry_and_download_actions() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)

        host.review.openFor(0)
        const check = findChild(host.review, "downloadPreflightCheckButton")
        const confirm = findChild(host.review, "downloadConfirmButton")
        verify(check.visible)
        verify(confirm.visible)

        // Already queued/downloaded/installed (or the exact destination
        // exists) can never clear: both actions hide instead of looping.
        host.detailsState.download_preflight_ready = false
        host.detailsState.download_preflight_terminal = true
        host.detailsState.download_preflight_status = "This exact game download is already queued, downloaded, or installed."
        verify(!check.visible)
        verify(!confirm.visible)

        // Transient blocks keep the retry action available.
        host.detailsState.download_preflight_terminal = false
        verify(check.visible)
        verify(confirm.visible)
        verify(!confirm.enabled)
        host.review.close()
    }
}
