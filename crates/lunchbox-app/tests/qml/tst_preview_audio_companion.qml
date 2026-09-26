import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "PreviewAudioCompanion"

    QtObject {
        id: video
        property url source: ""
        property int position: 8200
        property bool playing: false
    }

    Lunchbox.HoverPreviewPresentation {
        id: presentation
        previewRequested: video.playing
        playbackPlaying: video.playing
        fetchedUrl: video.source
    }

    Lunchbox.PreviewAudioCompanion {
        id: sound
        videoSource: video.source
        videoPosition: video.position
        previewPlaying: video.playing
    }

    function init() {
        sound.unmuted = false
        video.playing = false
        video.source = ""
        video.position = 8200
    }

    function test_unmute_opens_only_audio_and_preserves_video_frame() {
        video.source = "file:///tmp/lunchbox-preview-audio-test.mp4"
        video.playing = true
        presentation.acceptDecodedFrame()
        verify(presentation.videoVisible)
        compare(sound.audioSource.toString(), "")

        sound.unmuted = true
        compare(sound.audioSource.toString(), video.source.toString())
        verify(!sound.audioMuted)
        compare(sound.player.activeVideoTrack, -1)
        compare(video.position, 8200)
        verify(presentation.videoVisible)
        verify(presentation.frameReady)

        sound.unmuted = false
        compare(sound.audioSource.toString(), "")
        verify(sound.audioMuted)
        verify(presentation.videoVisible)
    }

    function test_leaving_card_closes_audio_without_changing_video_source() {
        video.source = "file:///tmp/lunchbox-preview-audio-test.mp4"
        video.playing = true
        sound.unmuted = true
        compare(sound.audioSource.toString(), video.source.toString())

        video.playing = false
        compare(sound.audioSource.toString(), "")
        compare(video.source.toString(),
                "file:///tmp/lunchbox-preview-audio-test.mp4")
    }
}
