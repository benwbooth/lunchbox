import QtQuick
import QtQuick.Controls

// Always visible over artwork, including covers that are not yet favorites.
LbRoundButton {
    id: control
    property bool favorite: false
    property bool busy: false
    property string gameTitle: ""
    signal toggleRequested(bool favorite)

    objectName: "coverFavoriteButton"
    width: 32
    height: 32
    enabled: !busy
    text: favorite ? "★" : "☆"
    highlighted: favorite
    font.pixelSize: 17
    Accessible.name: (favorite ? "Remove from Favorites: " : "Add to Favorites: ") + gameTitle
    ToolTip.visible: hovered
    ToolTip.text: busy ? "Saving favorite…"
                      : favorite ? "Remove from Favorites" : "Add to Favorites"
    onClicked: toggleRequested(!favorite)
}
