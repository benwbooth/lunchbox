import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "MediaPreviewActivity"
    when: windowShown
    visible: true
    Lunchbox.MediaPreviewActivity { id: indicator }
    function init() {
        indicator.requested = true
        indicator.resolving = false
        indicator.hasVideo = false
        indicator.playbackFailed = false
        indicator.queueState = "idle"
    }
    function test_states_data() {
        return [
            { tag: "setup check", state: "checking-setup", busy: true },
            { tag: "queued", state: "queued", busy: true },
            { tag: "download", state: "downloading", busy: true },
            { tag: "no match", state: "unavailable", busy: false },
            { tag: "needs setup", state: "setup-required", busy: false },
            { tag: "ready", state: "ready", busy: false },
            { tag: "not scheduled", state: "automatic", busy: false }
        ]
    }
    function test_states(data) {
        indicator.queueState = data.state
        compare(indicator.running, data.busy)
        compare(indicator.visible, data.busy)
        compare(indicator.width, 22); compare(indicator.height, 22)
    }
    function test_only_requested_unresolved_video_shows_activity() {
        indicator.resolving = true
        verify(indicator.running)
        indicator.hasVideo = true; verify(!indicator.running)
        indicator.hasVideo = false; indicator.requested = false; verify(!indicator.running)
        indicator.requested = true; indicator.playbackFailed = true; verify(!indicator.running)
    }
}
