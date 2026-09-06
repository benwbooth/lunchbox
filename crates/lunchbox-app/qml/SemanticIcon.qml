import QtQuick
import QtQuick.Shapes

// Named, consistent vector symbols; no platform-dependent font glyphs.
Item {
    id: root
    required property string name
    property color color: "#8d99aa"
    implicitWidth: 24
    implicitHeight: 24
    readonly property var paths: ({
        menu: "M4 6 H20 M4 12 H20 M4 18 H20",
        games: "M3 3 H9 V9 H3 Z M15 3 H21 V9 H15 Z M3 15 H9 V21 H3 Z M15 15 H21 V21 H15 Z",
        collection: "M3 5 H8 L10 8 H21 V20 H3 Z M7 12 H17 M7 16 H14",
        favorite: "M12 3 L15 9 L22 10 L17 15 L18 22 L12 18 L6 22 L7 15 L2 10 L9 9 Z",
        recent: "M12 3 A9 9 0 1 1 11.99 3 M12 7 V12 L16 14",
        download: "M12 3 V15 M7 10 L12 15 L17 10 M4 16 V21 H20 V16",
        import: "M3 9 V20 H21 V9 M7 5 H17 M12 2 V14 M8 10 L12 14 L16 10",
        torrent: "M9 3 H15 V9 H9 Z M2 16 H8 V22 H2 Z M16 16 H22 V22 H16 Z M12 9 V12 M5 16 V12 H19 V16",
        audit: "M8 4 H5 V22 H19 V4 H16 M8 2 H16 V6 H8 Z M8 14 L11 17 L16 11",
        edit: "M4 4 H11 M4 4 V20 H20 V13 M10 14 L11 10 L19 2 L22 5 L14 13 Z M17 4 L20 7",
        firmware: "M6 6 H18 V18 H6 Z M9 9 H15 V15 H9 Z M9 2 V6 M15 2 V6 M9 18 V22 M15 18 V22 M2 9 H6 M2 15 H6 M18 9 H22 M18 15 H22",
        media: "M3 3 H21 V21 H3 Z M3 17 L9 11 L14 16 L17 13 L21 17 M16 7 H17 V8 H16 Z"
    })
    Shape {
        anchors.fill: parent
        ShapePath {
            strokeColor: root.color
            strokeWidth: 1.7
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.paths[root.name] || "" }
        }
    }
}
