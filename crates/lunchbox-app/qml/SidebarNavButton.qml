import QtQuick
import QtQuick.Controls

Rectangle {
    id: nav

    required property string label
    required property string glyph
    property string count: ""
    property bool active: false
    property url iconSource: ""
    signal clicked()
    activeFocusOnTab: true
    Keys.onReturnPressed: clicked()
    Keys.onEnterPressed: clicked()
    Keys.onSpacePressed: clicked()

    width: ListView.view ? ListView.view.width : 228
    height: 43
    radius: 9
    color: active ? "#272c34" : hover.hovered ? "#1b2330" : "transparent"
    border.color: active ? "#443b31" : "transparent"

    HoverHandler { id: hover }
    TapHandler { onTapped: nav.clicked() }

    Rectangle {
        visible: nav.active
        width: 3
        height: 23
        radius: 2
        color: "#ffb454"
        anchors.left: parent.left
        anchors.leftMargin: 1
        anchors.verticalCenter: parent.verticalCenter
    }
    Image {
        id: navIcon
        source: nav.iconSource
        visible: nav.iconSource.toString().length > 0 && status !== Image.Error
        width: 24
        height: 24
        anchors.left: parent.left
        anchors.leftMargin: 15
        anchors.verticalCenter: parent.verticalCenter
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        mipmap: true
    }
    Text {
        text: nav.glyph
        visible: nav.iconSource.toString().length === 0
                 || navIcon.status === Image.Error
        width: 28
        anchors.left: parent.left
        anchors.leftMargin: 13
        anchors.verticalCenter: parent.verticalCenter
        color: nav.active ? "#ffb454" : "#8d99aa"
        font.pixelSize: 16
        horizontalAlignment: Text.AlignHCenter
    }
    Text {
        id: labelText
        text: nav.label
        anchors.left: parent.left
        anchors.leftMargin: 54
        anchors.right: countText.left
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        elide: Text.ElideRight
        fontSizeMode: Text.HorizontalFit
        minimumPixelSize: 10
        color: nav.active ? "#f4f7fb" : "#c0c8d4"
        font.pixelSize: 14
        font.weight: nav.active ? Font.DemiBold : Font.Medium
    }
    ToolTip.visible: hover.hovered
    ToolTip.text: nav.label
    Text {
        id: countText
        text: nav.count
        anchors.right: parent.right
        anchors.rightMargin: 13
        anchors.verticalCenter: parent.verticalCenter
        color: "#8d99aa"
        font.pixelSize: 12
        font.features: { "tnum": 1 }
    }
}
