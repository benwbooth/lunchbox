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

    function test_diagram_selects_connections_without_any_list() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        compare(host.mapping.rows.length, 2)
        compare(host.mapping.hoveredIndex, -1)
        compare(host.mapping.selected.target_id, "x")

        // Clicking a diagram control pins its connection.
        host.mapping.chooseControl(1, "y")
        compare(host.mapping.selectedIndex, 1)
        compare(host.mapping.selected.target_id, "y")
        compare(host.mapping.selected.output, "A..South")

        // Hover state isolates a wire while the stored pin stays put.
        host.mapping.hoveredIndex = 0
        compare(host.mapping.hoveredIndex, 0)
        compare(host.mapping.selected.target_id, "y")
        host.mapping.hoveredIndex = -1
    }

    function test_unmapped_control_clears_the_pin() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.mapping.chooseControl(0, "missing")
        compare(host.mapping.selectedIndex, -1)
        verify(!host.mapping.selected)
    }
}
