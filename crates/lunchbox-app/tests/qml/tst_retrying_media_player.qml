import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "RetryingMediaPlayer"

    Lunchbox.RetryingMediaPlayer {
        id: player
        retryDelay: 20
    }

    function test_empty_player_is_idle() {
        compare(player.source.toString(), "")
        compare(player.playbackState, 0)
        compare(player.retryCount, 0)
        verify(!player.retryPending)
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
}
