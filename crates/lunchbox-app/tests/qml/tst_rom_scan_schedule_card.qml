import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "RomScanScheduleCard"
    when: windowShown

    QtObject {
        id: scanModel
        property bool busy: false
        property bool profile_busy: false
        property bool schedule_busy: false
        property bool batch_scanning: false
        property int profile_count: 2
        property string schedule_cadence: "manual"
        property string schedule_status: "Automatic collection checks are off."
        property string schedule_last_run: "No scheduled scan has run yet"
        property string schedule_next_run: "Manual scans only"
        property int schedule_review_profile_count: 0
        property string savedCadence: ""
        property int runCount: 0

        function set_scan_schedule(cadence) {
            savedCadence = cadence
            schedule_cadence = cadence
        }
        function run_scheduled_scan_now() {
            runCount += 1
        }
        function scheduled_review_profile_name_at(index) {
            return index === 0 ? "Nintendo Switch" : ""
        }
    }

    Window {
        visible: true
        width: 900
        height: 260

        Lunchbox.RomScanScheduleCard {
            id: card
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 20
            scanModel: scanModel
        }
    }

    SignalSpy {
        id: reviewSpy
        target: card
        signalName: "reviewRequested"
    }

    function init() {
        scanModel.busy = false
        scanModel.profile_busy = false
        scanModel.schedule_busy = false
        scanModel.batch_scanning = false
        scanModel.profile_count = 2
        scanModel.schedule_cadence = "manual"
        scanModel.schedule_status = "Automatic collection checks are off."
        scanModel.schedule_last_run = "No scheduled scan has run yet"
        scanModel.schedule_next_run = "Manual scans only"
        scanModel.schedule_review_profile_count = 0
        scanModel.savedCadence = ""
        scanModel.runCount = 0
        reviewSpy.clear()
        wait(20)
    }

    function test_manual_state_is_clear_and_run_now_is_immediate() {
        const badge = findChild(card, "romScanScheduleBadge")
        const runButton = findChild(card, "romScanRunNowButton")
        verify(badge)
        verify(runButton)
        compare(card.badgeText, "MANUAL")
        verify(runButton.enabled)
        runButton.clicked()
        compare(scanModel.runCount, 1)
    }

    function test_cadence_change_routes_the_exact_value() {
        const picker = findChild(card, "romScanCadencePicker")
        verify(picker)
        picker.currentIndex = 2
        picker.activated(2)
        compare(scanModel.savedCadence, "daily")
        compare(scanModel.schedule_cadence, "daily")
        compare(card.badgeText, "UP TO DATE")
    }

    function test_review_action_is_prominent_and_busy_safe() {
        scanModel.schedule_review_profile_count = 1
        scanModel.schedule_status = "Nintendo Switch has files that need review."
        wait(20)
        const reviewButton = findChild(card, "romScanReviewButton")
        const runButton = findChild(card, "romScanRunNowButton")
        verify(reviewButton)
        verify(reviewButton.visible)
        compare(reviewButton.text, "REVIEW CHANGES")
        compare(card.badgeText, "1 TO REVIEW")
        reviewButton.clicked()
        compare(reviewSpy.count, 1)

        scanModel.schedule_busy = true
        wait(20)
        verify(!reviewButton.enabled)
        verify(!runButton.enabled)
    }
}
