import QtQuick
import QtQuick.Controls

Rectangle {
    id: status

    required property var libraryModel
    required property color ink
    required property color muted
    required property color accent
    required property color accentCool
    required property color line
    signal configureRequested()

    height: 92
    radius: 10
    color: "#0d141e"
    border.color: libraryModel.media_setup_required ? accent : line

    Column {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 5

        Row {
            width: parent.width
            height: 15
            Text {
                width: parent.width - queueCount.width
                text: "MEDIA DOWNLOADS"
                color: status.ink
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.9
            }
            Text {
                id: queueCount
                text: libraryModel.media_pending_count > 0
                      ? libraryModel.media_pending_count + " queued" : "idle"
                color: libraryModel.media_pending_count > 0
                       ? status.accentCool : status.muted
                font.pixelSize: 9
                font.weight: Font.DemiBold
            }
        }

        Text {
            width: parent.width
            text: libraryModel.media_setup_required
                  ? "Set up EmuMovies for automatic videos"
                  : libraryModel.media_active_title.length > 0
                    ? libraryModel.media_active_kind + " · "
                      + libraryModel.media_active_title
                    : libraryModel.media_fetch_message
            color: libraryModel.media_setup_required ? status.accent : status.muted
            font.pixelSize: 10
            font.weight: libraryModel.media_setup_required
                         ? Font.DemiBold : Font.Normal
            elide: Text.ElideRight
        }

        ProgressBar {
            width: parent.width
            height: 7
            from: 0
            to: 100
            indeterminate: libraryModel.media_active_title.length > 0
                           && libraryModel.media_active_progress < 0
            value: Math.max(0, libraryModel.media_active_progress)
            visible: libraryModel.media_active_title.length > 0
        }

        Text {
            width: parent.width
            visible: libraryModel.media_active_title.length === 0
                     && !libraryModel.media_setup_required
            text: libraryModel.media_downloaded_count + " downloaded · "
                  + libraryModel.media_missing_count + " unavailable · "
                  + libraryModel.media_error_count + " errors"
            color: status.muted
            font.pixelSize: 8
            elide: Text.ElideRight
        }
    }

    TapHandler {
        enabled: libraryModel.media_setup_required
        onTapped: status.configureRequested()
    }
    HoverHandler { id: hover }
    ToolTip.visible: hover.hovered
    ToolTip.text: libraryModel.media_setup_required
                  ? "Open EmuMovies Settings"
                  : libraryModel.media_fetch_message
}
