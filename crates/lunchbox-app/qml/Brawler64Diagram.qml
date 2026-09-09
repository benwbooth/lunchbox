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
    Rectangle { anchors.fill: parent; radius: 16; color: "#101820"; border.color: "#27333f" }

    // Only vector paths are transformed. Text is laid out at its final UI size.
    Shape {
        objectName: "brawlerBody"
        containsMode: Shape.FillContains
        width: 900; height: 500
        preferredRendererType: Shape.CurveRenderer
        transform: Scale { xScale: diagram.sx; yScale: diagram.sy }
        ShapePath {
            strokeWidth: 2; strokeColor: "#647382"; fillColor: "#35434f"
            PathSvg { path: "M140 114 Q174 87 237 111 L465 111 Q531 89 562 114 Q615 141 626 203 L638 364 Q648 412 614 422 Q590 427 564 394 L496 342 Q468 328 443 338 L264 338 Q236 329 217 350 L146 400 Q121 428 98 417 Q70 403 82 362 L114 206 Q123 141 140 114Z" }
        }
        ShapePath {
            strokeWidth: 1.5; strokeColor: "#4b5a66"; fillColor: "transparent"
            PathSvg { path: "M103 345 Q142 281 207 292 M496 296 Q558 284 616 351" }
        }
        ShapePath {
            strokeWidth: 2; strokeColor: "#72808a"; fillColor: "#1a252e"
            PathSvg { path: "M263 256 H287 V278 H309 V302 H287 V324 H263 V302 H241 V278 H263Z" }
        }
    }
    Rectangle {
        x: 143 * diagram.sx; y: 153 * diagram.sy
        width: 94 * diagram.sx; height: 94 * diagram.sy; radius: width / 2
        color: "#17212b"; border.color: "#647482"; border.width: 2
        Rectangle {
            anchors.centerIn: parent
            width: parent.width * 0.69; height: width; radius: width / 2
            color: "#45535f"; border.color: "#84919a"
            Rectangle { anchors.centerIn: parent; width: parent.width * 0.72; height: width; radius: width / 2; color: "transparent"; border.color: "#596975" }
        }
    }
    Rectangle {
        x: 677 * diagram.sx; y: 111 * diagram.sy
        width: 197 * diagram.sx; height: 213 * diagram.sy
        radius: 12; color: "#18232c"; border.color: "#344450"
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
        delegate: Rectangle {
            required property var modelData
            readonly property bool lit: diagram.activeControl === modelData.id
            readonly property bool direction: modelData.kind === "direction"
            width: (modelData.kind === "rear" ? 68 : modelData.kind === "shoulder" ? 80 : modelData.kind === "menu" ? 43 : direction ? 20 : 39) * diagram.sx
            height: (modelData.kind === "rear" ? 73 : modelData.kind === "shoulder" ? 25 : modelData.kind === "menu" ? 27 : direction ? 20 : 39) * diagram.sy
            x: modelData.x * diagram.sx - width / 2
            y: modelData.y * diagram.sy - height / 2
            radius: modelData.kind === "face" || modelData.kind === "c" || direction ? width / 2 : 6
            color: lit ? "#ffb454" : direction ? "transparent" : modelData.kind === "c" ? "#c9b16b" : modelData.id === "b" ? "#447c70" : modelData.id === "a" ? "#526f9f" : "#202d37"
            border.color: lit ? "#fff0ce" : direction ? "transparent" : "#81909c"
            border.width: lit ? 3 : 1
            PixelAlignedText {
                anchors.fill: parent
                // Short labels stay centered without scaling their glyphs.
                horizontalAlignment: Text.AlignHCenter
                text: parent.modelData.label
                color: parent.lit || parent.modelData.kind === "c" ? "#17232b" : "#f1f5f8"
                font.pixelSize: Math.max(9, Math.round((parent.modelData.kind === "face" ? 17 : 12) * diagram.sx))
            }
        }
    }
}
