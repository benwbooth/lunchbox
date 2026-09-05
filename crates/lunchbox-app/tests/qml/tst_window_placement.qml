import QtQuick
import QtCore
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "WindowPlacement"
    when: windowShown
    readonly property url settingsLocation: StandardPaths.writableLocation(StandardPaths.TempLocation)
                                           + "/lunchbox-window-test-" + Date.now() + ".ini"

    Settings {
        id: seed
        category: "WindowPlacementTests"
        location: testCase.settingsLocation
    }

    Component {
        id: hostComponent
        Window {
            id: host
            visible: true
            minimumWidth: 200
            minimumHeight: 150
            property alias placement: placement
            Lunchbox.WindowPlacement {
                id: placement
                window: host
                settingsCategory: "WindowPlacementTests"
                settingsLocation: testCase.settingsLocation
            }
        }
    }

    function test_reopen_restores_size_and_keeps_offscreen_positions_reachable() {
        seed.setValue("normalWidth", 500)
        seed.setValue("normalHeight", 400)
        seed.setValue("normalX", -100000)
        seed.setValue("normalY", -100000)
        seed.setValue("positioned", true)
        seed.setValue("maximized", false)
        seed.sync()
        const first = createTemporaryObject(hostComponent, testCase)
        verify(first)
        tryCompare(first.placement, "restored", true)
        compare(first.width, 500)
        compare(first.height, 400)
        verify(first.x >= first.screen.virtualX)
        verify(first.y >= first.screen.virtualY)
        first.width = 550
        first.height = 450
        first.placement.save()
        first.placement.enabled = false
        first.close()
        const second = createTemporaryObject(hostComponent, testCase)
        verify(second)
        tryCompare(second.placement, "restored", true)
        compare(second.width, 550)
        compare(second.height, 450)
        second.placement.enabled = false
        second.close()
    }
}
