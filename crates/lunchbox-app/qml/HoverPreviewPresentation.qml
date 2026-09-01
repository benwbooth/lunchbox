import QtQuick

QtObject {
    id: presentation

    property bool previewRequested: false
    property bool playbackPlaying: false
    property url fetchedUrl: ""
    property string fetchedSource: ""
    property bool detailFallbackEligible: false
    property url detailUrl: ""
    property string detailSource: ""
    property bool frameReady: false

    readonly property url resolvedUrl: fetchedUrl.toString().length > 0
                                       ? fetchedUrl
                                       : detailFallbackEligible ? detailUrl : ""
    readonly property string resolvedSource: fetchedUrl.toString().length > 0
                                             ? fetchedSource
                                             : detailFallbackEligible
                                               ? detailSource : ""
    readonly property bool videoVisible: previewRequested
                                          && playbackPlaying
                                          && frameReady
                                          && resolvedUrl.toString().length > 0
    // The video output must remain rendered while it warms up or some Qt
    // multimedia backends never deliver a first frame. Keep the opaque cover
    // above that live sink until a frame arrives, and reserve a higher layer
    // for badges and actions so artwork can never cover the card controls.
    readonly property int videoLayer: 0
    readonly property int artworkLayer: videoVisible ? videoLayer : videoLayer + 1
    readonly property int overlayLayer: videoLayer + 2

    onPreviewRequestedChanged: {
        if (!previewRequested)
            frameReady = false
    }
    onPlaybackPlayingChanged: {
        if (!playbackPlaying)
            frameReady = false
    }
    onResolvedUrlChanged: frameReady = false

    function acceptDecodedFrame() {
        if (previewRequested && playbackPlaying
                && resolvedUrl.toString().length > 0)
            frameReady = true
    }
}
