import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia

Rectangle {
    id: card

    property var detailsModel
    property var emuMoviesModel
    property var mediaPlayer
    property string gameId: ""
    property int databaseId: 0
    property string gameTitle: ""
    property string platform: ""
    property color accent: "#f4a340"
    property color accentCool: "#62c8f2"
    property color ink: "#eef2f7"
    property color muted: "#8b96a8"
    property color line: "#354155"

    signal settingsRequested()

    readonly property int providerRevision: emuMoviesModel
                                            ? emuMoviesModel.soundtrack_revision
                                              + emuMoviesModel.published_revision : 0
    readonly property bool discoveryMatchesGame: emuMoviesModel
                                                 && emuMoviesModel.soundtrack_game_id
                                                    === gameId

    width: parent ? parent.width : implicitWidth
    implicitHeight: content.implicitHeight + 20
    radius: 8
    color: "#151f2c"
    border.color: detailsModel && detailsModel.soundtrack_available
                  ? "#38556b" : line

    function formatTime(milliseconds) {
        if (!isFinite(milliseconds) || milliseconds < 0)
            return "0:00"
        const seconds = Math.floor(milliseconds / 1000)
        const minutes = Math.floor(seconds / 60)
        const remainder = seconds % 60
        return minutes + ":" + (remainder < 10 ? "0" : "") + remainder
    }

    ColumnLayout {
        id: content
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 10
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Column {
                Layout.fillWidth: true
                spacing: 3
                Text {
                    text: "GAME MUSIC"
                    color: card.ink
                    font.pixelSize: 10
                    font.weight: Font.Bold
                }
                Text {
                    text: detailsModel && detailsModel.soundtrack_available
                          ? detailsModel.soundtrack_count + " CACHED · "
                            + detailsModel.soundtrack_source.toUpperCase()
                          : card.discoveryMatchesGame
                            && emuMoviesModel.soundtrack_count > 0
                            ? emuMoviesModel.soundtrack_count + " FOUND · EMUMOVIES"
                            : "OPTIONAL · EMUMOVIES HYPERAUDIO"
                    color: detailsModel && detailsModel.soundtrack_available
                           ? card.accentCool : card.muted
                    font.pixelSize: 7
                    font.weight: Font.Bold
                    font.letterSpacing: 0.5
                }
            }

            BusyIndicator {
                Layout.preferredWidth: 26
                Layout.preferredHeight: 26
                running: emuMoviesModel && emuMoviesModel.busy
                         && emuMoviesModel.last_media_kind === "soundtrack"
                visible: running
            }

            Button {
                id: discoverButton
                Layout.preferredWidth: 94
                Layout.preferredHeight: 32
                text: emuMoviesModel && emuMoviesModel.busy
                      && emuMoviesModel.last_media_kind === "soundtrack"
                      ? "CANCEL"
                      : emuMoviesModel && emuMoviesModel.credentials_saved
                        ? "FIND MUSIC" : "SET UP"
                enabled: emuMoviesModel
                Accessible.name: text === "SET UP"
                                 ? "Set up EmuMovies game music"
                                 : text === "CANCEL"
                                   ? "Cancel EmuMovies game music operation"
                                   : "Find EmuMovies game music"
                onClicked: {
                    if (!emuMoviesModel.credentials_saved) {
                        card.settingsRequested()
                    } else if (emuMoviesModel.busy
                               && emuMoviesModel.last_media_kind === "soundtrack") {
                        emuMoviesModel.cancel()
                    } else {
                        emuMoviesModel.discover_soundtrack(
                                    card.gameId, card.databaseId,
                                    card.gameTitle, card.platform)
                    }
                }
                background: Rectangle {
                    radius: 7
                    color: discoverButton.down ? "#344254"
                                                : discoverButton.text === "FIND MUSIC"
                                                  ? card.accent : "#263647"
                    border.color: discoverButton.text === "FIND MUSIC"
                                  ? "transparent" : card.accentCool
                }
                contentItem: Text {
                    text: discoverButton.text
                    color: discoverButton.text === "FIND MUSIC" ? "#17110a" : card.ink
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: detailsModel && detailsModel.soundtrack_available
            spacing: 6

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Button {
                    Layout.preferredWidth: 34
                    Layout.preferredHeight: 30
                    text: "‹"
                    enabled: detailsModel && detailsModel.soundtrack_index > 0
                    Accessible.name: "Previous soundtrack track"
                    onClicked: {
                        mediaPlayer.stop()
                        detailsModel.select_soundtrack(detailsModel.soundtrack_index - 1)
                    }
                }
                Button {
                    id: playButton
                    Layout.preferredWidth: 42
                    Layout.preferredHeight: 34
                    text: mediaPlayer && mediaPlayer.playbackState === MediaPlayer.PlayingState
                          ? "Ⅱ" : "▶"
                    Accessible.name: text === "Ⅱ" ? "Pause game music" : "Play game music"
                    onClicked: {
                        if (mediaPlayer.playbackState === MediaPlayer.PlayingState)
                            mediaPlayer.pause()
                        else
                            mediaPlayer.play()
                    }
                    background: Rectangle {
                        radius: 17
                        color: playButton.down ? "#d89444" : card.accent
                    }
                    contentItem: Text {
                        text: playButton.text
                        color: "#17110a"
                        font.pixelSize: 11
                        font.weight: Font.Bold
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
                Button {
                    Layout.preferredWidth: 34
                    Layout.preferredHeight: 30
                    text: "›"
                    enabled: detailsModel
                             && detailsModel.soundtrack_index + 1 < detailsModel.soundtrack_count
                    Accessible.name: "Next soundtrack track"
                    onClicked: {
                        mediaPlayer.stop()
                        detailsModel.select_soundtrack(detailsModel.soundtrack_index + 1)
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        Layout.fillWidth: true
                        text: detailsModel ? detailsModel.soundtrack_title : ""
                        color: card.ink
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        ToolTip.visible: soundtrackTitleHover.containsMouse
                        ToolTip.text: text
                        MouseArea {
                            id: soundtrackTitleHover
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.NoButton
                        }
                    }
                    Text {
                        text: detailsModel
                              ? "TRACK " + (detailsModel.soundtrack_index + 1)
                                + " OF " + detailsModel.soundtrack_count
                              : ""
                        color: card.muted
                        font.pixelSize: 7
                        font.weight: Font.Bold
                        font.letterSpacing: 0.5
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Text {
                    text: card.formatTime(mediaPlayer ? mediaPlayer.position : 0)
                    color: card.muted
                    font.pixelSize: 8
                    font.features: { "tnum": 1 }
                }
                Slider {
                    Layout.fillWidth: true
                    from: 0
                    to: mediaPlayer ? Math.max(1, mediaPlayer.duration) : 1
                    value: mediaPlayer ? mediaPlayer.position : 0
                    enabled: mediaPlayer && mediaPlayer.seekable
                    onMoved: mediaPlayer.position = value
                    Accessible.name: "Game music position"
                }
                Text {
                    text: card.formatTime(mediaPlayer ? mediaPlayer.duration : 0)
                    color: card.muted
                    font.pixelSize: 8
                    font.features: { "tnum": 1 }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: card.discoveryMatchesGame
                     && emuMoviesModel.soundtrack_count > 0
            spacing: 5

            Repeater {
                model: emuMoviesModel ? emuMoviesModel.soundtrack_count : 0
                delegate: Rectangle {
                    id: trackRow
                    required property int index
                    readonly property bool trackCached: {
                        card.providerRevision
                        return emuMoviesModel.soundtrack_cached_at(index)
                    }
                    Layout.fillWidth: true
                    Layout.preferredHeight: 38
                    radius: 6
                    color: "#101721"
                    border.color: card.line

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 9
                        anchors.rightMargin: 6
                        spacing: 8
                        Text {
                            Layout.fillWidth: true
                            text: emuMoviesModel.soundtrack_title_at(index)
                            color: card.ink
                            font.pixelSize: 9
                            elide: Text.ElideRight
                            ToolTip.visible: trackHover.containsMouse
                            ToolTip.text: text
                            MouseArea {
                                id: trackHover
                                anchors.fill: parent
                                hoverEnabled: true
                                acceptedButtons: Qt.NoButton
                            }
                        }
                        Text {
                            visible: trackRow.trackCached
                            text: "CACHED"
                            color: card.accentCool
                            font.pixelSize: 7
                            font.weight: Font.Bold
                        }
                        Button {
                            Layout.preferredWidth: 78
                            Layout.preferredHeight: 28
                            text: trackRow.trackCached
                                  ? "READY" : "DOWNLOAD"
                            enabled: !emuMoviesModel.busy
                                     && !trackRow.trackCached
                            Accessible.name: text + " "
                                             + emuMoviesModel.soundtrack_title_at(index)
                            onClicked: emuMoviesModel.download_soundtrack(index)
                        }
                    }
                }
            }
        }

        InlineProgressBar {
            Layout.fillWidth: true
            visible: emuMoviesModel && emuMoviesModel.busy
                     && emuMoviesModel.last_media_kind === "soundtrack"
            from: 0
            to: 100
            value: emuMoviesModel ? Math.max(0, emuMoviesModel.transfer_progress) : 0
            fillColor: card.accentCool
            trackColor: "#303b4d"
        }

    }
}
