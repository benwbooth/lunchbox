import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "ControllerMappingView"
    when: windowShown

    Component {
        id: hostComponent

        ApplicationWindow {
            width: 1000
            height: 800
            visible: true

            QtObject {
                id: settingsState
                function controller_diagram(layoutId, highlight) { return "" }
            }

            Lunchbox.ControllerMappingView {
                id: mapping
                anchors.fill: parent
                settingsModel: settingsState
                sourceLayout: ({
                    id: "source-pad", name: "Source pad",
                    controls: [
                        {id: "a", label: "A", x: 10, y: 20},
                        {id: "b", label: "B", x: 30, y: 20}
                    ]
                })
                destinationLayout: ({
                    id: "target-pad", name: "Target pad",
                    controls: [
                        {id: "x", label: "X", x: 10, y: 20},
                        {id: "y", label: "Y", x: 30, y: 20}
                    ]
                })
                rows: [
                    {physical_id: "a", physical: "A", target_id: "x", target: "X",
                     output: "X..West", reason: "Same semantic control"},
                    {physical_id: "b", physical: "B", target_id: "y", target: "Y",
                     output: "A..South", reason: "Same semantic control"}
                ]
            }

            property alias mapping: mapping
        }
    }

    function test_wire_list_shows_every_connection_at_once() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const wires = findChild(host.mapping, "mappingWireList")
        verify(wires)
        compare(wires.count, 2)
        compare(host.mapping.hoveredIndex, -1)
        compare(host.mapping.selected.target_id, "x")
    }

    function test_clicking_a_wire_pins_its_connection() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const wires = findChild(host.mapping, "mappingWireList")
        verify(wires)
        let second = null
        for (let attempt = 0; attempt < 200 && !(second && second.visible); ++attempt) {
            wait(10)
            second = wires.itemAtIndex(1)
        }
        verify(second)
        mouseClick(second, second.width / 2, second.height / 2)
        compare(host.mapping.selectedIndex, 1)
        compare(host.mapping.selected.target_id, "y")
        compare(host.mapping.selected.output, "A..South")
    }

    function test_hover_state_is_available_for_wire_isolation() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.mapping.hoveredIndex = 1
        compare(host.mapping.hoveredIndex, 1)
        // Pinning still wins for the stored selection.
        compare(host.mapping.selected.target_id, "x")
        host.mapping.hoveredIndex = -1
    }
}
