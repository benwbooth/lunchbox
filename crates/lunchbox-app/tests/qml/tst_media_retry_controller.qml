import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "MediaRetryController"

    Lunchbox.MediaRetryController {
        id: controller
        retryDelay: 20
    }

    SignalSpy {
        id: retrySpy
        target: controller
        signalName: "retryScheduled"
    }

    SignalSpy {
        id: exhaustedSpy
        target: controller
        signalName: "exhausted"
    }

    function init() {
        controller.desiredSource = ""
        controller.maximumRetries = 2
        controller.retryDelay = 20
        retrySpy.clear()
        exhaustedSpy.clear()
    }

    function test_transient_failure_reloads_the_same_source() {
        controller.desiredSource = "file:///tmp/lunchbox-video.mp4"
        compare(controller.activeSource.toString(),
                "file:///tmp/lunchbox-video.mp4")

        verify(controller.handleFailure("temporary decoder failure"))
        compare(controller.activeSource.toString(), "")
        compare(controller.retryCount, 1)
        verify(controller.retryPending)

        tryCompare(retrySpy, "count", 1, 250)
        compare(controller.activeSource.toString(),
                "file:///tmp/lunchbox-video.mp4")
        compare(retrySpy.signalArguments[0][0], 1)
    }

    function test_failure_is_reported_only_after_bounded_retries() {
        controller.desiredSource = "file:///tmp/lunchbox-video.mp4"

        verify(controller.handleFailure("first"))
        tryCompare(retrySpy, "count", 1, 250)
        verify(controller.handleFailure("second"))
        tryCompare(retrySpy, "count", 2, 250)

        verify(!controller.handleFailure("terminal decoder failure"))
        compare(exhaustedSpy.count, 1)
        compare(controller.terminalError, "terminal decoder failure")
        compare(controller.retryCount, 2)
    }

    function test_ready_and_source_change_reset_the_retry_budget() {
        controller.desiredSource = "file:///tmp/first.mp4"
        verify(controller.handleFailure("temporary"))
        tryCompare(retrySpy, "count", 1, 250)

        controller.markReady()
        compare(controller.retryCount, 0)

        verify(controller.handleFailure("temporary again"))
        controller.desiredSource = "file:///tmp/second.mp4"
        compare(controller.retryCount, 0)
        verify(!controller.retryPending)
        compare(controller.activeSource.toString(), "file:///tmp/second.mp4")
    }

    function test_empty_source_never_schedules_a_retry() {
        verify(!controller.handleFailure("nothing to reload"))
        compare(retrySpy.count, 0)
        compare(exhaustedSpy.count, 0)
    }
}
