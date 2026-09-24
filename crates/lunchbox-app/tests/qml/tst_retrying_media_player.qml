import QtQuick
import QtMultimedia
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "RetryingMediaPlayer"

    QtObject {
        id: selectedVideo
        property url current: ""
    }

    Lunchbox.RetryingMediaPlayer {
        id: player
        retryDelay: 20
    }

    VideoOutput {
        id: output
    }

    SignalSpy {
        id: autoPlaySpy
        target: player
        signalName: "autoPlayRequested"
    }

    function init() {
        player.autoPlay = false
        player.source = ""
        selectedVideo.current = ""
        player.videoOutput = null
        player.maximumRetries = 0
        autoPlaySpy.clear()
    }

    function test_empty_player_is_idle() {
        compare(player.source.toString(), "")
        compare(player.playbackState, 0)
        compare(player.retryCount, 0)
        verify(!player.retryPending)
        verify(!player.autoPlay)
    }

    function test_source_change_is_exposed_without_starting_a_retry() {
        player.source = "file:///tmp/lunchbox-retrying-player-probe.mp4"
        compare(player.source.toString(),
                "file:///tmp/lunchbox-retrying-player-probe.mp4")
        compare(player.retryCount, 0)

        player.source = ""
        compare(player.source.toString(), "")
        verify(!player.retryPending)
    }

    function test_autoplay_is_reasserted_when_cached_source_is_rebound() {
        player.videoOutput = output
        player.autoPlay = true
        player.source = "file:///tmp/lunchbox-retrying-player-probe.mp4"
        tryVerify(function() { return autoPlaySpy.count > 0 }, 250)
        const firstRequestCount = autoPlaySpy.count

        player.source = ""
        player.source = "file:///tmp/lunchbox-retrying-player-probe.mp4"
        tryVerify(function() {
            return autoPlaySpy.count > firstRequestCount
        }, 250)
        compare(autoPlaySpy.signalArguments[autoPlaySpy.count - 1][0].toString(),
                "file:///tmp/lunchbox-retrying-player-probe.mp4")
    }

    function test_audio_reload_preserves_bound_video_selection() {
        const first = "file:///tmp/lunchbox-retrying-player-probe.mp4"
        const second = "file:///tmp/lunchbox-next-player-probe.mp4"
        player.source = Qt.binding(function() { return selectedVideo.current })
        selectedVideo.current = first
        tryVerify(function() { return player.player.source.toString() === first }, 250)

        player.reloadPipeline()
        compare(player.source.toString(), first)
        compare(player.player.source.toString(), "")
        tryVerify(function() { return player.player.source.toString() === first }, 250)

        selectedVideo.current = second
        tryVerify(function() { return player.source.toString() === second }, 250)
        tryVerify(function() { return player.player.source.toString() === second }, 250)
    }
}
