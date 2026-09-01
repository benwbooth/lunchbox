import QtQuick
import QtMultimedia

QtObject {
    id: root

    property alias source: retryControllerObject.desiredSource
    property alias maximumRetries: retryControllerObject.maximumRetries
    property alias retryDelay: retryControllerObject.retryDelay
    readonly property alias retryCount: retryControllerObject.retryCount
    readonly property alias retryPending: retryControllerObject.retryPending

    property alias activeAudioTrack: mediaPlayer.activeAudioTrack
    property alias audioOutput: mediaPlayer.audioOutput
    property alias videoOutput: mediaPlayer.videoOutput
    property alias loops: mediaPlayer.loops
    property alias position: mediaPlayer.position
    readonly property alias duration: mediaPlayer.duration
    readonly property alias hasVideo: mediaPlayer.hasVideo
    readonly property alias mediaStatus: mediaPlayer.mediaStatus
    readonly property alias playbackState: mediaPlayer.playbackState
    readonly property alias seekable: mediaPlayer.seekable

    signal errorOccurred(int error, string errorString)

    function play() {
        player.play()
    }

    function pause() {
        player.pause()
    }

    function stop() {
        player.stop()
    }

    property MediaRetryController retryController: MediaRetryController {
        id: retryControllerObject
    }

    property MediaPlayer player: MediaPlayer {
        id: mediaPlayer
        source: retryControllerObject.activeSource

        onSourceChanged: {
            if (source.toString().length === 0)
                stop()
        }
        onMediaStatusChanged: {
            if (mediaStatus === MediaPlayer.LoadedMedia
                    || mediaStatus === MediaPlayer.BufferedMedia)
                retryControllerObject.markReady()
        }
        onPlaybackStateChanged: {
            if (playbackState === MediaPlayer.PlayingState)
                retryControllerObject.markReady()
        }
        onErrorOccurred: function(error, errorString) {
            if (!retryControllerObject.handleFailure(errorString))
                root.errorOccurred(error, errorString)
        }
    }
}
