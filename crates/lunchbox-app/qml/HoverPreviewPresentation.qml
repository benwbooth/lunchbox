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
