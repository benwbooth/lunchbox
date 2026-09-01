import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "HoverPreviewPresentation"

    Lunchbox.HoverPreviewPresentation {
        id: presentation
    }

    function init() {
        presentation.previewRequested = false
        presentation.playbackPlaying = false
        presentation.fetchedUrl = ""
        presentation.fetchedSource = ""
        presentation.detailFallbackEligible = false
        presentation.detailUrl = ""
        presentation.detailSource = ""
    }

    function test_artwork_stays_visible_until_a_decoded_frame_arrives() {
        presentation.previewRequested = true
        presentation.fetchedUrl = "file:///tmp/hover.mp4"
        presentation.playbackPlaying = true

        verify(!presentation.frameReady)
        verify(!presentation.videoVisible)
        verify(presentation.artworkLayer > presentation.videoLayer)
        verify(presentation.overlayLayer > presentation.artworkLayer)

        presentation.acceptDecodedFrame()
        verify(presentation.frameReady)
        verify(presentation.videoVisible)
        compare(presentation.artworkLayer, presentation.videoLayer)
        verify(presentation.overlayLayer > presentation.videoLayer)

        presentation.previewRequested = false
        verify(!presentation.frameReady)
        verify(!presentation.videoVisible)
        verify(presentation.overlayLayer > presentation.artworkLayer)
    }

    function test_selected_detail_video_is_an_immediate_cache_fallback() {
        presentation.previewRequested = true
        presentation.detailFallbackEligible = true
        presentation.detailUrl = "file:///tmp/detail-video.mp4"
        presentation.detailSource = "emumovies"

        compare(presentation.resolvedUrl.toString(),
                "file:///tmp/detail-video.mp4")
        compare(presentation.resolvedSource, "emumovies")

        presentation.fetchedUrl = "file:///tmp/fetched-video.mp4"
        presentation.fetchedSource = "local"
        compare(presentation.resolvedUrl.toString(),
                "file:///tmp/fetched-video.mp4")
        compare(presentation.resolvedSource, "local")
    }

    function test_source_change_requires_a_new_decoded_frame() {
        presentation.previewRequested = true
        presentation.playbackPlaying = true
        presentation.fetchedUrl = "file:///tmp/first.mp4"
        presentation.acceptDecodedFrame()
        verify(presentation.videoVisible)

        presentation.fetchedUrl = "file:///tmp/second.mp4"
        verify(!presentation.frameReady)
        verify(!presentation.videoVisible)
    }
}
