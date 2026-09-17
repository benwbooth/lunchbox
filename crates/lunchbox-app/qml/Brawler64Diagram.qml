import QtQuick
import QtQuick.Shapes

Item {
    id: diagram
    Brawler64Geometry { id: geometry }
    property string activeControl: ""
    readonly property real sx: width / 900
    readonly property real sy: height / 500
    implicitWidth: 900
    implicitHeight: 500
    Rectangle { anchors.fill: parent; radius: 16; color: "#0e141b"; border.color: "#27333f" }

    // Only vector paths are transformed. Text is laid out at its final UI size.
    Shape {
        objectName: "brawlerBody"
        containsMode: Shape.FillContains
        width: 900; height: 500
        preferredRendererType: Shape.CurveRenderer
        transform: Scale { xScale: diagram.sx; yScale: diagram.sy }
        ShapePath {
            strokeWidth: 3; strokeColor: "#71808d"
            fillGradient: LinearGradient {
                x1: 0; y1: 0; x2: 0; y2: 500
                GradientStop { position: 0; color: "#3d4d5e" }
                GradientStop { position: 0.55; color: "#2c3947" }
                GradientStop { position: 1; color: "#202b37" }
            }
            PathSvg { path: "M140 114 Q174 87 237 111 L465 111 Q531 89 562 114 Q615 141 626 203 L638 364 Q648 412 614 422 Q590 427 564 394 L496 342 Q468 328 443 338 L264 338 Q236 329 217 350 L146 400 Q121 428 98 417 Q70 403 82 362 L114 206 Q123 141 140 114Z" }
        }
        ShapePath {
            strokeWidth: 1.5; strokeColor: "#525f6b"; fillColor: "transparent"
            PathSvg { path: "M103 345 Q142 281 207 292 M496 296 Q558 284 616 351" }
        }
        ShapePath {
            strokeWidth: 2; strokeColor: "#4a5a67"; fillColor: "#1c2732"
            PathSvg { path: "M263 256 H287 V278 H309 V302 H287 V324 H263 V302 H241 V278 H263Z" }
        }
    }
    // Analog stick: recessed well, steel ring, two-tone cap, center pivot.
    Rectangle {
        x: 143 * diagram.sx; y: 153 * diagram.sy
        width: 94 * diagram.sx; height: 94 * diagram.sy; radius: width / 2
        color: "#10161d"; border.color: "#05090d"; border.width: 2
        Rectangle {
            anchors.centerIn: parent
            width: parent.width * 0.86; height: width; radius: width / 2
            color: "transparent"; border.color: "#3d4c5a"; border.width: Math.max(1, 2 * diagram.sx)
        }
        Rectangle {
            anchors.centerIn: parent
            width: parent.width * 0.69; height: width; radius: width / 2
            color: "#33404d"; border.color: "#6b7d8c"; border.width: Math.max(1, 2 * diagram.sx)
            Rectangle {
                x: parent.width * 0.18; y: parent.height * 0.14
                width: parent.width * 0.42; height: width; radius: width / 2
                color: "#5b6e81"; opacity: 0.85
            }
            Rectangle {
                anchors.centerIn: parent
                width: parent.width * 0.3; height: width; radius: width / 2
                color: "#1c2732"; border.color: "#596975"
            }
        }
    }
    Rectangle {
        x: 677 * diagram.sx; y: 111 * diagram.sy
        width: 197 * diagram.sx; height: 213 * diagram.sy
        radius: 12; color: "#121a23"; border.color: "#344450"
    }
    Repeater {
        model: [
            {text:"FRONT", x:40, y:30, w:560},
            {text:"REAR CONTROLS", x:687, y:130, w:180},
            {text:"ANALOG STICK", x:127, y:365, w:145},
            {text:"D-PAD", x:271, y:365, w:100},
            {text:"C BUTTONS", x:508, y:83, w:135},
            {text:"Z triggers", x:713, y:284, w:150},
            {text:"Brawler64 · N64 layout", x:40, y:450, w:470},
            {text:"Extra buttons vary by edition", x:500, y:450, w:365}
        ]
        delegate: PixelAlignedText {
            required property var modelData
            x: modelData.x * diagram.sx; y: modelData.y * diagram.sy
            width: modelData.w * diagram.sx; height: 24 * diagram.sy
            text: modelData.text; color: "#a4b4c1"
            font.pixelSize: Math.max(9, Math.round(13 * diagram.sx))
        }
    }
    Repeater {
        model: geometry.controls
        delegate: Item {
            id: controlBody
            required property var modelData
            readonly property bool lit: diagram.activeControl === modelData.id
            readonly property bool direction: modelData.kind === "direction"
            readonly property bool round: modelData.kind === "face" || modelData.kind === "c" || direction
            // Steel cap with a top-light sheen; amber C cluster; lit wins.
            readonly property color ring: lit ? "#fff1d5" : "#05090d"
            readonly property color edge: lit ? "#ffb454" : modelData.kind === "c" ? "#8a6d2f" : "#3a4a59"
            readonly property color cap: lit ? "#ffb454" : modelData.kind === "c" ? "#8a6d2f" : modelData.id === "b" ? "#3d5a52" : modelData.id === "a" ? "#3a4f74" : "#33404d"
            readonly property color sheen: lit ? "#ffd9a0" : modelData.kind === "c" ? "#b28e42" : "#5b6e81"
            readonly property color glyph: lit || modelData.kind === "c" ? "#17232b" : "#f1f5f8"
            width: (modelData.kind === "rear" ? 68 : modelData.kind === "shoulder" ? 80 : modelData.kind === "menu" ? 43 : direction ? 20 : 39) * diagram.sx
            height: (modelData.kind === "rear" ? 73 : modelData.kind === "shoulder" ? 25 : modelData.kind === "menu" ? 27 : direction ? 20 : 39) * diagram.sy
            x: modelData.x * diagram.sx - width / 2
            y: modelData.y * diagram.sy - height / 2
            Rectangle {
                anchors.fill: parent
                radius: controlBody.round ? width / 2 : 6
                color: controlBody.ring
            }
            Rectangle {
                anchors.fill: parent
                anchors.margins: Math.max(1, 2 * diagram.sx)
                radius: controlBody.round ? width / 2 : 5
                color: controlBody.edge
            }
            Rectangle {
                anchors.fill: parent
                anchors.margins: Math.max(2, 4 * diagram.sx)
                radius: controlBody.round ? width / 2 : 4
                color: controlBody.cap
                Rectangle {
                    x: parent.width * 0.16; y: parent.height * 0.1
                    width: parent.width * 0.44; height: width
                    radius: width / 2
                    color: controlBody.sheen
                    opacity: 0.8
                    visible: controlBody.round
                }
            }
            PixelAlignedText {
                anchors.fill: parent
                // Short labels stay centered without scaling their glyphs.
                horizontalAlignment: Text.AlignHCenter
                text: controlBody.modelData.label
                color: controlBody.glyph
                font.pixelSize: Math.max(9, Math.round((controlBody.modelData.kind === "face" ? 17 : controlBody.modelData.kind === "menu" ? 10 : 12) * diagram.sx))
            }
        }
    }
}
