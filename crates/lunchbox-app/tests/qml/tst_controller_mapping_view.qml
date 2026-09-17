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
        // Nothing is pinned on load: no row looks pre-selected.
        compare(host.mapping.selectedIndex, -1)
        verify(!host.mapping.selected)
        compare(host.mapping.highlightedSourceId(), "")
        compare(host.mapping.highlightedDestId(), "")

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

    function test_artwork_highlight_follows_the_pointer_not_the_pin() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        // Pin the second row, then hover the first: artwork follows hover.
        host.mapping.chooseControl(1, "y")
        compare(host.mapping.selectedIndex, 1)
        host.mapping.hoveredIndex = 0
        compare(host.mapping.highlightedSourceId(), "a")
        compare(host.mapping.highlightedDestId(), "x")
        // Clearing the hover falls back to the pin, never to a third row.
        host.mapping.hoveredIndex = -1
        compare(host.mapping.highlightedSourceId(), "b")
        compare(host.mapping.highlightedDestId(), "y")
    }

    function test_unmapped_control_clears_the_pin() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.mapping.chooseControl(0, "missing")
        compare(host.mapping.selectedIndex, -1)
        verify(!host.mapping.selected)
    }

    function test_tooltip_names_both_ends_with_emulator_output() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const tip = host.mapping.controlTooltip(0, {id: "a", label: "A"})
        verify(tip.rich.indexOf("drives") >= 0)
        verify(tip.rich.indexOf("#ffb454") >= 0)
        verify(tip.rich.indexOf("#62dac8") >= 0)
        verify(tip.rich.indexOf("X..West") >= 0)
        verify(tip.plain.indexOf("<") < 0)
        verify(tip.plain.indexOf("X..West") >= 0)
        const flipped = host.mapping.controlTooltip(1, {id: "y", label: "Y"})
        verify(flipped.rich.indexOf("driven by") >= 0)
    }

    function test_twin_inputs_render_as_shared_wires() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        compare(host.mapping.secondaryRows.length, 0)
        host.mapping.twinRoutes = [{target_id: "x", physical_id: "b2", output: "X..West"}]
        compare(host.mapping.secondaryRows.length, 1)
        compare(host.mapping.secondaryRows[0].physical, "b2")
        compare(host.mapping.secondaryRows[0].target, "X")
        const tip = host.mapping.controlTooltip(0, {id: "b2", label: "B2"})
        verify(tip.rich.indexOf("also drives") >= 0)
        verify(tip.rich.indexOf("shares one input") >= 0)
        verify(tip.rich.indexOf("X..West") >= 0)
    }

    function test_lane_helper_spreads_wires_across_the_channel() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const first = host.mapping.laneXFor(0, 3, 100, 400)
        const middle = host.mapping.laneXFor(1, 3, 100, 400)
        const last = host.mapping.laneXFor(2, 3, 100, 400)
        verify(first < middle && middle < last)
        verify(first >= 100 && last <= 400)
        compare(host.mapping.laneXFor(0, 1, 100, 400), 250)
    }
}
