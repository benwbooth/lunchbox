import QtQuick
import "../../qml" as Lunchpail

Window {
    width: 900; height: 500; visible: true
    title: "Brawler64 layout review"
    Lunchpail.Brawler64Diagram { id: diagram; anchors.fill: parent; activeControl: "b" }
    Timer {
        interval: 1500; running: true
        onTriggered: diagram.grabToImage(result => {
            result.saveToFile("/tmp/lunchpail-brawler64-review.png")
            Qt.quit()
        })
    }
}
