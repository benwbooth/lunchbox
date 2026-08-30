import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "CouchLaunchScreen"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 1920
            height: 1200

            QtObject {
                id: detailsState
                property bool launch_busy: true
                property bool game_running: false
                property bool activity_busy: false
                property string launch_status: "Preparing RetroArch…"
                property string emulator_name: "RetroArch · Mesen"
                property int session_history_revision: 0
                property int session_count: 0
                property int play_count: 2
                property string play_time: "36 min"

                function session_duration_at(index) { return "18 min" }
                function session_outcome_label_at(index) { return "Completed" }
                function session_emulator_at(index) { return "RetroArch · Mesen" }
            }

            Lunchbox.CouchLaunchScreen {
                id: launchScreen
                anchors.fill: parent
                details: detailsState
                gameTitle: "Faxanadu"
                platformName: "Nintendo Entertainment System"
                coverUrl: ""
                heroUrl: ""
                themeBackgroundUrl: ""
                backgroundColor: "#091019"
                panelColor: "#111927"
                panelRaisedColor: "#182233"
                inkColor: "#f4f7fb"
                mutedColor: "#94a0b3"
                accentColor: "#f1a23b"
                accentCoolColor: "#5de2d2"
                dangerColor: "#ff6b65"
                inputHint: "A SELECT  ·  B BACK"
                active: true
            }

            property alias detailsState: detailsState
            property alias launchScreen: launchScreen
        }
    }

    function test_launch_lifecycle_is_explicit() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        compare(host.launchScreen.phase, "starting")
        compare(host.launchScreen.phaseEyebrow, "NOW LAUNCHING")

        host.detailsState.launch_busy = false
        host.detailsState.game_running = true
        tryCompare(host.launchScreen, "phase", "running")
        compare(host.launchScreen.phaseHeadline, "The emulator is running")

        host.detailsState.game_running = false
        host.detailsState.activity_busy = true
        host.detailsState.launch_status = "Saving play activity…"
        tryCompare(host.launchScreen, "phase", "complete")
        compare(host.launchScreen.phaseHeadline, "Saving your play session")

        host.detailsState.activity_busy = false
        host.detailsState.session_count = 1
        ++host.detailsState.session_history_revision
        compare(host.launchScreen.latestSessionDuration, "18 min")
        compare(host.launchScreen.latestSessionOutcome, "Completed")
        compare(host.launchScreen.phaseHeadline, "Session complete")

        host.detailsState.launch_status = "Could not launch: firmware missing"
        tryCompare(host.launchScreen, "phase", "failed")
        compare(host.launchScreen.phaseEyebrow, "LAUNCH FAILED")
        compare(host.launchScreen.phaseHeadline, "The game could not be started")
    }

    function test_elapsed_time_stays_controller_readable() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        compare(host.launchScreen.formatElapsed(0), "00:00")
        compare(host.launchScreen.formatElapsed(65), "01:05")
        compare(host.launchScreen.formatElapsed(3661), "1:01:01")
    }
}
