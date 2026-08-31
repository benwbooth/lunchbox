import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GameDownloadStatus"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 520
            height: 360

            QtObject {
                id: queueState
                property int revision: 1
                property string state: "COMPLETE"

                function job_index_for_game(gameId) {
                    return gameId === "faxanadu" ? 0 : -1
                }
                function job_state_at(index) { return state }
                function job_badge_at(index) {
                    return state === "COMPLETE" ? "INSTALLING"
                         : state === "IMPORTED" ? "READY" : "ATTENTION"
                }
                function job_progress_at(index) { return 1 }
                function job_detail_at(index) {
                    return state === "IMPORTED" ? "Ready in your library"
                                                : "Finishing the exact reviewed file"
                }
            }

            Lunchbox.GameDownloadStatus {
                id: status
                width: 440
                queue: queueState
                gameId: "faxanadu"
                ink: "#f4f7fb"
                muted: "#94a0b3"
                panel: "#182233"
                line: "#2b384b"
                accent: "#f1a23b"
                accentCool: "#5de2d2"
                alternativesAvailable: true
                playEnabled: true
            }

            property alias queueState: queueState
            property alias status: status
        }
    }

    function test_completed_download_becomes_ready_without_showing_sources() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        compare(host.status.jobIndex, 0)
        compare(host.status.badge, "INSTALLING")
        compare(host.status.alternativesExpanded, false)

        host.queueState.state = "IMPORTED"
        ++host.queueState.revision
        compare(host.status.badge, "READY")
        compare(host.status.jobState, "IMPORTED")
        verify(host.status.playEnabled)
    }

    function test_unmanaged_game_has_no_download_status_card() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.status.gameId = "other"
        compare(host.status.jobIndex, -1)
        compare(host.status.badge, "")
    }
}
