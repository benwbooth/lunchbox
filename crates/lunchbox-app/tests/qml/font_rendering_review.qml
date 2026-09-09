import QtQuick
import QtQuick.Window
import QtQuick.Controls

Window {
    id: window
    visible: true
    width: 1000
    height: 850
    color: "#141719"
    title: "Lunchbox font rendering comparison"
    Rectangle { anchors.fill: parent; color: "#141719" }
    Column {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 14
        Repeater {
            model: [
                {family: "Noto Sans", renderer: Text.NativeRendering, name: "Noto / native"},
                {family: "Noto Sans", renderer: Text.QtRendering, name: "Noto / distance field"},
                {family: "Noto Sans", renderer: Text.CurveRendering, name: "Noto / curve"},
                {family: "DejaVu Sans", renderer: Text.NativeRendering, name: "DejaVu / native"},
                {family: "DejaVu Sans", renderer: Text.QtRendering, name: "DejaVu / distance field"},
                {family: "DejaVu Sans", renderer: Text.CurveRendering, name: "DejaVu / curve"}
            ]
            delegate: Column {
                required property var modelData
                spacing: 6
                Text { text: modelData.name; color: "#8ad4b7"; font.pixelSize: 14 }
                Text {
                    text: "Choose the controller you are holding"
                    color: "white"; font.family: modelData.family; font.pixelSize: 22
                    font.bold: true; renderType: modelData.renderer
                }
                Text {
                    text: "Xbox-style — dual sticks and diamond   ·   xbox"
                    color: "white"; font.family: modelData.family; font.pixelSize: 14
                    renderType: modelData.renderer
                }
                Text {
                    text: "Recorded controls stay unchanged. Press a button to identify your controller."
                    color: "white"; font.family: modelData.family; font.pixelSize: 12
                    renderType: modelData.renderer
                }
            }
        }
    }
    Timer {
        interval: 1500; running: true
        onTriggered: window.contentItem.grabToImage(function(result) {
            result.saveToFile("/tmp/lunchbox-font-comparison.png")
            console.log("FONT_REVIEW_COMPLETE screenScale=" + window.Screen.devicePixelRatio)
            if (!Qt.application.arguments.includes("--keep-open"))
                Qt.quit()
        })
    }
}
