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
    property bool autoPlay: false

    signal errorOccurred(int error, string errorString)
    signal autoPlayRequested(url source)

    onAutoPlayChanged: Qt.callLater(ensureAutoPlay)
    onVideoOutputChanged: Qt.callLater(ensureAutoPlay)

    function ensureAutoPlay() {
        const activeSource = retryControllerObject.activeSource
        if (!autoPlay || activeSource.toString().length === 0
                || !mediaPlayer.videoOutput
                || mediaPlayer.playbackState === MediaPlayer.PlayingState)
            return
        autoPlayRequested(activeSource)
        // Calling play while the source is still loading records persistent
        // playback intent in QMediaPlayer. Waiting only for LoadedMedia loses
        // that intent when a cached source or VideoOutput is rebound without a
        // fresh media-status transition.
        mediaPlayer.play()
    }

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

    property Connections retryConnections: Connections {
        target: retryControllerObject
        function onActiveSourceChanged() {
            Qt.callLater(root.ensureAutoPlay)
        }
        function onRetryScheduled() {
            Qt.callLater(root.ensureAutoPlay)
        }
    }

    property MediaPlayer player: MediaPlayer {
        id: mediaPlayer
        source: retryControllerObject.activeSource

        onSourceChanged: {
            if (source.toString().length === 0)
                stop()
            else
                Qt.callLater(root.ensureAutoPlay)
        }
        onMediaStatusChanged: {
            if (mediaStatus === MediaPlayer.LoadedMedia
                    || mediaStatus === MediaPlayer.BufferedMedia) {
                retryControllerObject.markReady()
                Qt.callLater(root.ensureAutoPlay)
            }
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
