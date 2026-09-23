import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "StatusPill"
    when: windowShown
    visible: true
    width: 320
    height: 80

    Lunchbox.StatusPill {
        id: pill
        x: 20
        y: 20
        label: "download listings"
        value: "1052"
        explanation: "Catalog entries, not queued downloads."
    }

    function test_count_and_explanation_are_accessible() {
        compare(pill.Accessible.name, "1052 download listings")
        compare(pill.Accessible.description, "Catalog entries, not queued downloads.")
        pill.value = "1053"
        compare(pill.Accessible.name, "1053 download listings")
    }

    function test_hover_reaches_pill() {
        const hover = findChild(pill, "pillHover")
        verify(hover !== null)
        mouseMove(pill, pill.width / 2, pill.height / 2)
        tryCompare(hover, "hovered", true, 1500)
    }
}
