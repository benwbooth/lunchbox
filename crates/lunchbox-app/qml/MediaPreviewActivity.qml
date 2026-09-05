import QtQuick
import QtQuick.Controls

// Artwork carries activity only. Provider messages and overall progress belong
// to MediaDownloadStatus in the sidebar, including setup and terminal failures.
BusyIndicator {
    property bool requested: false
    property bool resolving: false
    property bool hasVideo: false
    property bool playbackFailed: false
    property string queueState: "idle"
    width: 22
    height: 22
    running: requested && !hasVideo && !playbackFailed
             && (resolving || ["checking-setup", "queued", "downloading"].indexOf(queueState) >= 0)
    visible: running
}
