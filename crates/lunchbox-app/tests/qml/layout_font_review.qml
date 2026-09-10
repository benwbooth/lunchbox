import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    visible: true
    width: 900
    height: 560
    color: "#141719"
    title: "Lunchbox layout text rendering diagnostic"
    Rectangle {
        id: canvas
        anchors.fill: parent
        color: window.color
    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12
        Text {
            text: "Native / full hinting: Select a physical layout"
            color: "white"; font.family: window.font.family; font.pointSize: window.font.pointSize
            font.hintingPreference: Font.PreferFullHinting
            renderType: Text.NativeRendering
        }
        Text {
            text: "Native / physical font size: Select a physical layout"
            color: "white"; font.family: window.font.family
            font.pointSize: Math.round(window.font.pointSize * 96 / 72 * window.devicePixelRatio) * 72 / 96 / window.devicePixelRatio
            renderType: Text.NativeRendering
        }
        Repeater {
            model: [Text.NativeRendering, Text.QtRendering, Text.CurveRendering]
            delegate: Column {
                required property int modelData
                spacing: 4
                Text { text: ["Distance field", "Native", "Curve"][modelData]; color: "#80dcb8" }
                Row {
                    spacing: 30
                    Repeater {
                        model: [0, 0.25, 0.5, 0.75]
                        delegate: Text {
                            required property real modelData
                            property int renderer: parent.parent.modelData
                            y: modelData
                            text: "Select a physical layout"
                            color: "white"
                            font: window.font
                            renderType: renderer
                        }
                    }
                }
                ComboBox {
                    id: picker
                    width: 400
                    model: ["Select a physical layout", "Xbox-style — dual sticks and diamond"]
                    contentItem: Text {
                        text: picker.displayText
                        font: picker.font
                        color: "white"
                        verticalAlignment: Text.AlignVCenter
                        renderType: modelData
                    }
                    Component.onCompleted: console.log("PICKER_FONT", font.family, font.pointSize, font.pixelSize, font.hintingPreference)
                }
            }
        }
    }
    }
    Timer {
        interval: 1500; running: true
        onTriggered: canvas.grabToImage(result => {
            result.saveToFile("/tmp/lunchbox-layout-font-review.png")
            Qt.quit()
        })
    }
}
