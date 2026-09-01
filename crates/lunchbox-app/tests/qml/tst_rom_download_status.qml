import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "RomDownloadStatus"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 1000
            height: 800

            QtObject {
                id: queueState
                property bool busy: false
                property bool refreshing: false
                property int active_count: 0
                property int finished_count: 2
                property int failed_count: 0
                property int job_count: 2
                property int revision: 1
                property string aggregate_speed: "0 B/s"
                property string message: "Recent downloads are retained here."
                property int refreshCount: 0
                property string firstState: "IMPORTED"

                function refresh() { ++refreshCount }
                function job_title_at(index) { return index === 0 ? "Faxanadu" : "Contra" }
                function job_platform_at(index) { return "Nintendo Entertainment System" }
                function job_state_at(index) { return index === 0 ? firstState : "COMPLETE" }
                function job_progress_at(index) { return 1 }
                function job_detail_at(index) { return "100% · Downloaded" }
                function job_can_pause(index) { return index === 0 && firstState === "DOWNLOADING" }
                function job_can_resume(index) { return index === 0 && firstState === "PAUSED" }
                function job_can_import(index) { return index === 0 && firstState === "IMPORTED" }
                function job_import_directory_at(index) { return index === 0 ? "/tmp/lunchbox/imports" : "" }
                function pause_job(index) {}
                function resume_job(index) {}
            }

            Lunchbox.RomDownloadStatus {
                id: downloadStatus
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                queue: queueState
            }

            property alias queueState: queueState
            property alias downloadStatus: downloadStatus
        }
    }

    function test_recent_downloads_remain_visible_when_collapsed() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        compare(host.downloadStatus.expanded, false)
        compare(host.downloadStatus.collapsedSummary, "2 recent downloads")
        host.downloadStatus.toggle()
        compare(host.downloadStatus.expanded, true)
        const rows = findChild(host.downloadStatus, "downloadRows")
        verify(rows)
        compare(rows.count, 2)
    }

    function test_new_active_download_opens_the_hideable_status() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.queueState.aggregate_speed = "8.4 MiB/s"
        host.queueState.active_count = 1
        tryCompare(host.downloadStatus, "expanded", true)
        compare(host.downloadStatus.collapsedSummary,
                "1 active download  ·  8.4 MiB/s")
        host.downloadStatus.expanded = false
        compare(host.downloadStatus.expanded, false)
    }

    function test_completed_collection_exposes_import_review() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.downloadStatus.expanded = true
        const rows = findChild(host.downloadStatus, "downloadRows")
        verify(rows)
        compare(rows.count, 2)
        tryVerify(function() { return rows.height > 0 })
        tryVerify(function() { return rows.itemAtIndex(0) !== null })
        const collectionRow = rows.itemAtIndex(0)
        verify(collectionRow)
        const importButton = findChild(collectionRow, "importButton-0")
        verify(importButton)
        compare(importButton.importAvailable, true)
        compare(importButton.text, "Review import")
    }

    function test_poll_updates_action_without_blinking_the_control() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.queueState.job_count = 1
        host.queueState.active_count = 1
        host.queueState.firstState = "PAUSED"
        host.queueState.revision += 1
        host.downloadStatus.expanded = true

        const rows = findChild(host.downloadStatus, "downloadRows")
        tryVerify(function() { return rows.itemAtIndex(0) !== null })
        const row = rows.itemAtIndex(0)
        const resume = findChild(row, "resumeButton-0")
        const pause = findChild(row, "pauseButton-0")
        verify(resume && pause)
        compare(row.stateName, "PAUSED")
        verify(host.queueState.job_can_resume(0))
        tryVerify(function() { return resume.actionAvailable })
        verify(!pause.actionAvailable)

        host.queueState.refreshing = true
        verify(resume.enabled)
        host.queueState.firstState = "DOWNLOADING"
        host.queueState.revision += 1

        tryVerify(function() { return !resume.actionAvailable })
        tryVerify(function() { return pause.actionAvailable })
        verify(pause.enabled)
    }
}
