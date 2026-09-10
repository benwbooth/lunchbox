import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "Brawler64Diagram"
    when: windowShown
    width: 900; height: 500
    Lunchbox.Brawler64Geometry { id: geometry }
    Lunchbox.Brawler64Diagram { id: diagram; anchors.fill: parent }
    function test_front_controls_stay_inside_body() {
        const body = findChild(diagram, "brawlerBody")
        verify(body !== null)
        for (const control of geometry.controls) {
            if (control.kind === "rear") continue
            const rx = control.kind === "shoulder" ? 40 : control.kind === "menu" ? 22 : control.kind === "direction" ? 10 : 20
            const ry = control.kind === "shoulder" ? 13 : control.kind === "menu" ? 14 : control.kind === "direction" ? 10 : 20
            for (const dx of [-rx, rx])
                for (const dy of [-ry, ry])
                    verify(body.contains(Qt.point(control.x + dx, control.y + dy)), control.id + " extends outside the body")
        }
    }
    function test_unique_control_geometry_and_highlight() {
        compare(new Set(geometry.controls.map(control => control.id)).size, 21)
        diagram.activeControl = "b"
        compare(geometry.point("b").label, "B")
        compare(geometry.point("missing"), null)
    }
}
