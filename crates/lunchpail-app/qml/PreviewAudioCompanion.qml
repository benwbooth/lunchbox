import QtQuick
import QtMultimedia

// The hover video stays on its silent player. Opening a separate audio-only
// decoder on demand avoids resetting the video sink when the user unmutes.
QtObject {
    id: companion

    property url videoSource: ""
    property int videoPosition: 0
    property bool previewPlaying: false
    property bool unmuted: false
    readonly property alias audioSource: audioPlayer.source
    readonly property alias audioPlaybackState: audioPlayer.playbackState
    readonly property alias audioMuted: soundOutput.muted
    signal playbackError(string message)

    property AudioOutput output: AudioOutput {
        id: soundOutput
        muted: !companion.unmuted
        volume: 0.34
    }

    property MediaPlayer player: MediaPlayer {
        id: audioPlayer
        property bool positionedForSource: false
        source: companion.unmuted && companion.previewPlaying
                ? companion.videoSource : ""
        audioOutput: soundOutput
        activeVideoTrack: -1
        loops: MediaPlayer.Infinite

        function startIfReady() {
            if (!companion.unmuted || !companion.previewPlaying
                    || source.toString().length === 0
                    || (mediaStatus !== MediaPlayer.LoadedMedia
                        && mediaStatus !== MediaPlayer.BufferedMedia))
                return
            if (!positionedForSource) {
                position = duration > 0
                           ? Math.min(Math.max(0, companion.videoPosition), duration)
                           : Math.max(0, companion.videoPosition)
                positionedForSource = true
            }
            if (playbackState !== MediaPlayer.PlayingState)
                play()
        }

        onSourceChanged: {
            positionedForSource = false
            if (source.toString().length === 0)
                stop()
            else
                Qt.callLater(startIfReady)
        }
        onMediaStatusChanged: startIfReady()
        onErrorOccurred: function(error, errorString) {
            companion.playbackError(errorString)
        }
    }
}
