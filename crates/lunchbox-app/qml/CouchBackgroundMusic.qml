pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtMultimedia

Item {
    id: deck

    required property var library
    required property var details
    required property string selectedGameId
    required property bool active
    required property color panelColor
    required property color inkColor
    required property color mutedColor
    required property color accentColor
    property bool blocked: false
    property bool userPaused: false
    property url committedSource: ""
    readonly property bool detailsCurrent: selectedGameId.length > 0
                                           && details.game_id === selectedGameId
    readonly property bool soundtrackReady: detailsCurrent
                                              && details.soundtrack_available
                                              && details.soundtrack_url.toString().length > 0
    readonly property bool playbackAllowed: active
                                             && library.couch_music_enabled
                                             && !blocked
                                             && soundtrackReady
                                             && !details.game_running
    readonly property url requestedSource: playbackAllowed
                                           ? details.soundtrack_url : ""
    readonly property bool playing: player.playbackState === MediaPlayer.PlayingState
    readonly property bool presentationAvailable: active
                                                  && library.couch_music_enabled
                                                  && soundtrackReady
    readonly property string trackTitle: detailsCurrent
                                         ? details.soundtrack_title : ""
    readonly property string statusText: player.error !== MediaPlayer.NoError
                                         ? "PLAYBACK UNAVAILABLE"
                                         : userPaused ? "PAUSED"
                                         : playing ? "NOW PLAYING" : "PREPARING"

    implicitWidth: 380
    implicitHeight: 68
    visible: presentationAvailable
    opacity: visible ? 1 : 0

    function reconcileSource() {
        settleTimer.stop()
        if (requestedSource.toString().length === 0) {
            player.stop()
            committedSource = ""
            return
        }
        player.stop()
        committedSource = ""
        settleTimer.restart()
    }

    function togglePlayback() {
        if (!soundtrackReady)
            return
        if (playing) {
            userPaused = true
            player.pause()
        } else {
            userPaused = false
            if (committedSource.toString().length === 0)
                reconcileSource()
            else
                player.play()
        }
    }

    onRequestedSourceChanged: reconcileSource()
    onSelectedGameIdChanged: userPaused = false

    Timer {
        id: settleTimer
        interval: 650
        repeat: false
        onTriggered: {
            if (!deck.playbackAllowed
                    || deck.requestedSource.toString().length === 0)
                return
            deck.committedSource = deck.requestedSource
        }
    }

    AudioOutput {
        id: audioOutput
        muted: false
        volume: Math.max(0, Math.min(1, deck.library.couch_music_volume / 100.0))
    }

    MediaPlayer {
        id: player
        source: deck.committedSource
        audioOutput: audioOutput
        loops: MediaPlayer.Infinite
        onSourceChanged: {
            if (source.toString().length === 0)
                stop()
        }
        onMediaStatusChanged: {
            if ((mediaStatus === MediaPlayer.LoadedMedia
                    || mediaStatus === MediaPlayer.BufferedMedia)
                    && deck.playbackAllowed && !deck.userPaused)
                play()
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: 14
        color: Qt.rgba(deck.panelColor.r, deck.panelColor.g,
                       deck.panelColor.b, 0.92)
        border.color: Qt.rgba(deck.mutedColor.r, deck.mutedColor.g,
                              deck.mutedColor.b, 0.46)
        border.width: 1

        Rectangle {
            id: equalizer
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            width: 42
            height: 42
            radius: 11
            color: Qt.rgba(deck.accentColor.r, deck.accentColor.g,
                           deck.accentColor.b, 0.18)
            border.color: deck.accentColor

            Row {
                anchors.centerIn: parent
                spacing: 3
                Repeater {
                    model: [12, 21, 16]
                    delegate: Rectangle {
                        required property int index
                        required property int modelData
                        anchors.verticalCenter: parent.verticalCenter
                        width: 3
                        height: deck.playing ? modelData : 8
                        radius: 2
                        color: deck.accentColor
                        Behavior on height {
                            NumberAnimation {
                                duration: 180 + index * 70
                                easing.type: Easing.InOutSine
                            }
                        }
                    }
                }
            }
        }

        Column {
            anchors.left: equalizer.right
            anchors.leftMargin: 12
            anchors.right: toggleButton.left
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Text {
                width: parent.width
                text: deck.statusText
                color: deck.playing ? deck.accentColor : deck.mutedColor
                font.pixelSize: 8
                font.weight: Font.Bold
                font.letterSpacing: 1.1
            }
            Text {
                width: parent.width
                text: deck.trackTitle.length > 0 ? deck.trackTitle : "Game music"
                color: deck.inkColor
                font.pixelSize: 13
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: deck.details.title
                color: deck.mutedColor
                font.pixelSize: 9
                elide: Text.ElideRight
            }
        }

        Rectangle {
            id: toggleButton
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            width: 40
            height: 40
            radius: 11
            color: toggleHover.hovered
                   ? Qt.rgba(deck.accentColor.r, deck.accentColor.g,
                             deck.accentColor.b, 0.24)
                   : Qt.rgba(deck.mutedColor.r, deck.mutedColor.g,
                             deck.mutedColor.b, 0.12)
            border.color: toggleHover.hovered ? deck.accentColor
                                              : Qt.rgba(deck.mutedColor.r,
                                                        deck.mutedColor.g,
                                                        deck.mutedColor.b, 0.42)

            Text {
                anchors.centerIn: parent
                text: deck.playing ? "Ⅱ" : "▶"
                color: deck.inkColor
                font.pixelSize: 15
                font.weight: Font.Bold
            }
            HoverHandler { id: toggleHover }
            TapHandler { onTapped: deck.togglePlayback() }
            Accessible.role: Accessible.Button
            Accessible.name: deck.playing ? "Pause Couch Mode game music"
                                          : "Play Couch Mode game music"
        }
    }
}
