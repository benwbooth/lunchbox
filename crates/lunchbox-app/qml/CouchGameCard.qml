pragma ComponentBehavior: Bound

import QtQuick

// The same identity/artwork contract is used by the wall and both animated paths.
Item {
    id: card
    required property var library
    required property int index
    required property string gameId
    required property string gameTitle
    required property string gameCanonicalTitle
    required property string gamePlatform
    required property bool gameLocal
    required property bool gameDownloadable
    required property int gameDatabaseId
    required property double gameMediaId
    property bool selected: false
    property bool wheel: false
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color accent: "#ffb454"
    property color panel: "#182230"
    property int artworkRevision: library.media_revision
    readonly property url artwork: {
        artworkRevision
        return library.artwork_url(gameMediaId, wheel ? "clear-logo" : "box-front")
    }
    signal activated(int index)

    function requestArtwork() {
        library.request_artwork(gameMediaId, gameCanonicalTitle, gamePlatform,
                                wheel ? "clear-logo" : "box-front")
    }
    Component.onCompleted: requestArtwork()
    onGameMediaIdChanged: requestArtwork()
    onWheelChanged: requestArtwork()

    Rectangle {
        anchors.fill: frame
        anchors.margins: -5
        radius: 17
        color: "transparent"
        border.color: card.accent
        border.width: 2
        opacity: card.selected ? 0.45 : 0
        Behavior on opacity { NumberAnimation { duration: 220 } }
    }
    Rectangle {
        id: frame
        anchors.fill: parent
        anchors.margins: 9
        radius: 12
        color: card.wheel ? Qt.rgba(0.04, 0.06, 0.1, card.selected ? 0.72 : 0.35) : card.panel
        border.color: card.selected ? card.accent : Qt.rgba(1, 1, 1, 0.14)
        border.width: card.selected ? 2 : 1
        scale: card.selected ? 1 : hover.hovered ? 1.025 : 0.97
        Behavior on scale { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
        Image {
            id: cover
            anchors.fill: parent
            anchors.margins: card.wheel ? 14 : 5
            anchors.bottomMargin: card.wheel ? 14 : 52
            source: card.artwork
            sourceSize.width: card.wheel ? 640 : 400
            sourceSize.height: card.wheel ? 180 : 560
            asynchronous: true
            cache: true
            mipmap: true
            fillMode: Image.PreserveAspectFit
            opacity: status === Image.Ready ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: 200 } }
        }
        Text {
            anchors.centerIn: cover
            width: cover.width - 20
            visible: cover.status !== Image.Ready
            text: card.wheel ? card.gameTitle : card.gameTitle.charAt(0).toUpperCase()
            color: card.ink
            font.pixelSize: card.wheel ? 28 : 70
            font.weight: Font.Black
            minimumPixelSize: 14
            fontSizeMode: Text.Fit
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            height: cover.height
            verticalAlignment: Text.AlignVCenter
        }
        Text {
            visible: !card.wheel
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 12
            height: 34
            text: card.gameTitle
            color: card.ink
            font.pixelSize: 14
            minimumPixelSize: 10
            fontSizeMode: Text.Fit
            font.weight: Font.DemiBold
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        Rectangle {
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 9
            width: 8; height: 8; radius: 4
            visible: card.gameLocal
            color: "#5ee391"
        }
    }
    HoverHandler { id: hover }
    TapHandler { onTapped: card.activated(card.index) }
    Accessible.role: Accessible.ListItem
    Accessible.name: gameTitle + ", " + gamePlatform
    Accessible.selected: selected
    Accessible.onPressAction: activated(index)
}
