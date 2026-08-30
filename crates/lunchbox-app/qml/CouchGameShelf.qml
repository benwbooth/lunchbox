pragma ComponentBehavior: Bound

import QtQuick

ListView {
    id: shelf

    required property var library
    required property color background
    required property color panel
    required property color panelRaised
    required property color ink
    required property color muted
    required property color accent
    required property color accentCool
    required property int cardRadius
    property bool cinematic: false
    property bool navigationActive: false

    signal currentGameChanged()
    signal cardActivated(int index)

    orientation: cinematic ? ListView.Vertical : ListView.Horizontal
    spacing: cinematic ? 5 : 13
    clip: true
    reuseItems: true
    cacheBuffer: cinematic ? height * 2 : width
    model: library
    boundsBehavior: Flickable.StopAtBounds
    highlightMoveDuration: 130
    preferredHighlightBegin: cinematic ? height * 0.38 : width * 0.12
    preferredHighlightEnd: cinematic ? height * 0.62 : width * 0.64
    highlightRangeMode: ListView.ApplyRange
    maximumFlickVelocity: 18000
    flickDeceleration: 2600

    function withAlpha(value, opacity) {
        return Qt.rgba(value.r, value.g, value.b, opacity)
    }

    function accentFor(value) {
        const colors = [shelf.accent, shelf.accentCool,
                        Qt.lighter(shelf.accent, 1.22),
                        Qt.darker(shelf.accent, 1.28),
                        Qt.lighter(shelf.accentCool, 1.16),
                        Qt.darker(shelf.accentCool, 1.22)]
        if (!value || value.length === 0)
            return colors[0]
        return colors[value.charCodeAt(0) % colors.length]
    }

    onCurrentIndexChanged: currentGameChanged()
    onCurrentItemChanged: currentGameChanged()

    delegate: Item {
        id: gameTile

        required property int index
        required property string gameId
        required property string gameTitle
        required property string gameCanonicalTitle
        required property string gamePlatform
        required property string gameStatus
        required property bool gameLocal
        required property bool gameDownloadable
        required property int gameDatabaseId
        property int artworkRevision: shelf.library.media_revision
        property url artworkUrl: {
            artworkRevision
            return shelf.library.artwork_url(gameDatabaseId, "box-front")
        }
        readonly property bool current: ListView.isCurrentItem
        readonly property int selectionDistance: Math.abs(index - shelf.currentIndex)

        width: shelf.cinematic ? shelf.width : (current ? 162 : 132)
        height: shelf.cinematic ? 98 : shelf.height - 8

        function requestArtwork() {
            shelf.library.request_artwork(gameDatabaseId, gameCanonicalTitle,
                                          gamePlatform, "box-front")
        }

        Component.onCompleted: requestArtwork()
        onGameDatabaseIdChanged: requestArtwork()
        onGameCanonicalTitleChanged: requestArtwork()
        onGamePlatformChanged: requestArtwork()
        onArtworkRevisionChanged: requestArtwork()

        ArtworkMat {
            id: coverCard
            visible: !shelf.cinematic
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: gameTile.current ? 10 : 18
            width: gameTile.current ? 158 : 128
            height: gameTile.current ? 214 : 174
            radius: Math.max(8, shelf.cardRadius - 4)
            artworkPresent: tileCover.source.toString().length > 0
                            && tileCover.status !== Image.Error
            fallbackColor: shelf.accentFor(gameTile.gameTitle)
            border.color: gameTile.current
                          ? (shelf.navigationActive ? shelf.ink : shelf.accent)
                          : shelf.withAlpha(shelf.muted, 0.4)
            border.width: gameTile.current ? 3 : 1
            clip: true
            scale: gameTile.current ? 1 : coverHover.hovered ? 1.03 : 1
            Behavior on width { NumberAnimation { duration: 120 } }
            Behavior on height { NumberAnimation { duration: 120 } }
            Behavior on scale { NumberAnimation { duration: 90 } }

            Image {
                id: tileCover
                anchors.fill: parent
                anchors.margins: 2
                source: gameTile.artworkUrl
                asynchronous: true
                cache: true
                mipmap: true
                autoTransform: true
                fillMode: Image.PreserveAspectFit
                sourceSize.width: 300
                sourceSize.height: 420
                opacity: status === Image.Ready ? 1 : 0
            }
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 54
                color: shelf.withAlpha(shelf.background, 0.84)
            }
            Text {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.margins: 9
                text: gameTile.gameTitle
                color: shelf.ink
                font.pixelSize: gameTile.current ? 11 : 9
                font.weight: Font.Bold
                maximumLineCount: 2
                wrapMode: Text.WordWrap
                elide: Text.ElideRight
            }
            Rectangle {
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                width: 10
                height: 10
                radius: 5
                visible: gameTile.gameLocal || gameTile.gameDownloadable
                color: gameTile.gameLocal ? shelf.accentCool : shelf.accent
            }
            Text {
                anchors.centerIn: parent
                text: gameTile.gameTitle.length > 0
                      ? gameTile.gameTitle.charAt(0).toUpperCase() : "?"
                color: shelf.withAlpha(shelf.ink, 0.85)
                font.pixelSize: 48
                font.weight: Font.Black
                visible: tileCover.status !== Image.Ready
            }
            HoverHandler { id: coverHover }
            TapHandler { onTapped: shelf.cardActivated(gameTile.index) }
        }

        Rectangle {
            id: wheelCard
            visible: shelf.cinematic
            anchors.verticalCenter: parent.verticalCenter
            x: gameTile.current ? 0 : 22
            width: parent.width - (gameTile.current ? 8 : 38)
            height: gameTile.current ? 92 : 78
            radius: Math.max(10, shelf.cardRadius - 2)
            color: gameTile.current
                   ? shelf.withAlpha(shelf.panelRaised, 0.95)
                   : wheelHover.hovered ? shelf.withAlpha(shelf.panelRaised, 0.8)
                                        : shelf.withAlpha(shelf.panel, 0.67)
            border.color: gameTile.current
                          ? (shelf.navigationActive ? shelf.accent : shelf.ink)
                          : shelf.withAlpha(shelf.muted, 0.28)
            border.width: gameTile.current ? 2 : 1
            opacity: gameTile.current ? 1
                     : Math.max(0.48, 0.88 - gameTile.selectionDistance * 0.055)
            clip: true

            Behavior on x { NumberAnimation { duration: 130; easing.type: Easing.OutCubic } }
            Behavior on width { NumberAnimation { duration: 130; easing.type: Easing.OutCubic } }
            Behavior on height { NumberAnimation { duration: 130; easing.type: Easing.OutCubic } }
            Behavior on opacity { NumberAnimation { duration: 100 } }

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: gameTile.current ? 6 : 3
                color: gameTile.current ? shelf.accent
                                        : shelf.withAlpha(shelf.muted, 0.32)
            }

            ArtworkMat {
                id: wheelCoverFrame
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                width: gameTile.current ? 58 : 48
                height: gameTile.current ? 78 : 64
                radius: 7
                artworkPresent: wheelCover.source.toString().length > 0
                                && wheelCover.status !== Image.Error
                fallbackColor: shelf.accentFor(gameTile.gameTitle)
                clip: true

                Image {
                    id: wheelCover
                    anchors.fill: parent
                    anchors.margins: 1
                    source: gameTile.artworkUrl
                    asynchronous: true
                    cache: true
                    mipmap: true
                    autoTransform: true
                    fillMode: Image.PreserveAspectFit
                    sourceSize.width: 180
                    sourceSize.height: 250
                    opacity: status === Image.Ready ? 1 : 0
                }
                Text {
                    anchors.centerIn: parent
                    text: gameTile.gameTitle.length > 0
                          ? gameTile.gameTitle.charAt(0).toUpperCase() : "?"
                    color: shelf.withAlpha(shelf.ink, 0.86)
                    font.pixelSize: gameTile.current ? 25 : 20
                    font.weight: Font.Black
                    visible: wheelCover.status !== Image.Ready
                }
            }

            Column {
                anchors.left: wheelCoverFrame.right
                anchors.leftMargin: 14
                anchors.right: statePill.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 5

                Text {
                    width: parent.width
                    text: gameTile.gameTitle
                    color: shelf.ink
                    font.pixelSize: gameTile.current ? 15 : 12
                    font.weight: gameTile.current ? Font.Bold : Font.DemiBold
                    maximumLineCount: 2
                    wrapMode: Text.WordWrap
                    elide: Text.ElideRight
                }
                Text {
                    width: parent.width
                    text: gameTile.gamePlatform.toUpperCase()
                    color: gameTile.current ? shelf.accentCool : shelf.muted
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    font.letterSpacing: 0.65
                    elide: Text.ElideRight
                }
            }

            Rectangle {
                id: statePill
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                width: stateLabel.implicitWidth + 16
                height: 24
                radius: 7
                color: gameTile.gameLocal
                       ? shelf.withAlpha(shelf.accentCool, 0.22)
                       : gameTile.gameDownloadable
                         ? shelf.withAlpha(shelf.accent, 0.2)
                         : shelf.withAlpha(shelf.background, 0.46)
                border.color: gameTile.gameLocal ? shelf.accentCool
                              : gameTile.gameDownloadable ? shelf.accent
                                                          : shelf.withAlpha(shelf.muted, 0.35)
                Text {
                    id: stateLabel
                    anchors.centerIn: parent
                    text: gameTile.gameLocal ? "READY"
                          : gameTile.gameDownloadable ? "GET" : "CATALOG"
                    color: gameTile.gameLocal ? shelf.accentCool
                           : gameTile.gameDownloadable ? shelf.accent : shelf.muted
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    font.letterSpacing: 0.7
                }
            }

            HoverHandler { id: wheelHover }
            TapHandler { onTapped: shelf.cardActivated(gameTile.index) }
        }
    }
}
