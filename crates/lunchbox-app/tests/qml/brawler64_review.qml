import QtQuick
import "../../qml" as Lunchbox

Window {
    width: 900; height: 500; visible: true
    title: "Brawler64 layout review"
    Lunchbox.Brawler64Diagram { id: diagram; anchors.fill: parent; activeControl: "b" }
    Timer {
        interval: 1500; running: true
        onTriggered: diagram.grabToImage(result => {
            result.saveToFile("/tmp/lunchbox-brawler64-review.png")
            Qt.quit()
        })
    }
}
