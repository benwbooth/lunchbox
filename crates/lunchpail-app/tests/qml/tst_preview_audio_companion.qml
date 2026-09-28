import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "PreviewAudioCompanion"

    QtObject {
        id: video
        property url source: ""
        property int position: 8200
        property bool playing: false
    }

    Lunchpail.HoverPreviewPresentation {
        id: presentation
        previewRequested: video.playing
        playbackPlaying: video.playing
        fetchedUrl: video.source
    }

    Lunchpail.PreviewAudioCompanion {
        id: sound
        videoSource: video.source
        videoPosition: video.position
        previewPlaying: video.playing
    }

    QtObject {
        id: sharedAudio
        property bool muted: true
    }

    Lunchpail.PreviewAudioCompanion {
        id: detailsSound
        unmuted: !sharedAudio.muted
        previewPlaying: true
    }

    function init() {
        sharedAudio.muted = true
        detailsSound.videoSource = ""
        sound.unmuted = false
        video.playing = false
        video.source = ""
        video.position = 8200
    }

    function test_unmute_opens_only_audio_and_preserves_video_frame() {
        video.source = "file:///tmp/lunchpail-preview-audio-test.mp4"
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
        video.source = "file:///tmp/lunchpail-preview-audio-test.mp4"
        video.playing = true
        sound.unmuted = true
        compare(sound.audioSource.toString(), video.source.toString())

        video.playing = false
        compare(sound.audioSource.toString(), "")
        compare(video.source.toString(),
                "file:///tmp/lunchpail-preview-audio-test.mp4")
    }

    function test_shared_unmute_survives_changing_games_and_reopening_media() {
        sound.unmuted = Qt.binding(function() { return !sharedAudio.muted })
        video.source = "file:///tmp/lunchpail-first-preview.mp4"
        detailsSound.videoSource = "file:///tmp/lunchpail-details-preview.mp4"
        video.playing = true

        sharedAudio.muted = false
        verify(!sound.audioMuted)
        verify(!detailsSound.audioMuted)
        compare(sound.audioSource.toString(), video.source.toString())
        compare(detailsSound.audioSource.toString(), detailsSound.videoSource.toString())

        // Leaving one grid card stops its sound, not the shared user choice.
        video.playing = false
        video.source = ""
        detailsSound.videoSource = ""
        compare(sound.audioSource.toString(), "")
        verify(!sharedAudio.muted)

        video.source = "file:///tmp/lunchpail-next-preview.mp4"
        video.playing = true
        detailsSound.videoSource = "file:///tmp/lunchpail-other-details-preview.mp4"
        compare(sound.audioSource.toString(), video.source.toString())
        compare(detailsSound.audioSource.toString(), detailsSound.videoSource.toString())
        verify(!sound.audioMuted)
        verify(!detailsSound.audioMuted)

        // A mute from either surface silences both without touching selection.
        sharedAudio.muted = true
        verify(sound.audioMuted)
        verify(detailsSound.audioMuted)
        compare(sound.audioSource.toString(), "")
        compare(detailsSound.audioSource.toString(), "")
        compare(video.source.toString(), "file:///tmp/lunchpail-next-preview.mp4")
    }
}
