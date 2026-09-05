import QtQuick
import QtQuick.Controls
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
        id: gridHostComponent
        ApplicationWindow {
            id: gridHost
            visible: false
            width: 500
            height: 400
            property alias placement: gridPlacement
            property alias view: grid
            property bool prepared: false
            property bool firstFrameReported: false
            property real firstFrameContentY: -1
            property int firstFrameIndex: -1
            property int firstFrameColumns: -1

            function prepare() {
                contentItem.width = width
                contentItem.height = height
                contentItem.ensurePolished()
                grid.forceLayout()
                grid.currentIndex = 65
                grid.contentY = 500
                prepared = true
            }
            onWidthChanged: { if (prepared && !firstFrameReported) prepare() }
            onHeightChanged: { if (prepared && !firstFrameReported) prepare() }
            onFrameSwapped: {
                if (!firstFrameReported) {
                    firstFrameContentY = grid.contentY
                    firstFrameIndex = grid.currentIndex
                    firstFrameColumns = grid.columns
                    firstFrameReported = true
                }
            }
            GridView {
                id: grid
                anchors.fill: parent
                readonly property int columns: Math.max(1, Math.floor(width / 100))
                cellWidth: width / columns
                cellHeight: 100
                model: 500
                delegate: Rectangle { width: grid.cellWidth; height: 100; color: "red" }
            }
            Lunchbox.WindowPlacement {
                id: gridPlacement
                window: gridHost
                deferPresentation: true
                settingsCategory: "WindowPlacementTests"
                settingsLocation: testCase.settingsLocation
            }
        }
    }

    function test_maximized_grid_first_frame_data() {
        return [{ tag: "cached rectangle", cached: true },
                { tag: "new screen rectangle", cached: false }]
    }

    function test_maximized_grid_first_frame(data) {
        seed.setValue("normalWidth", 500)
        seed.setValue("normalHeight", 400)
        seed.setValue("positioned", false)
        seed.setValue("maximized", false)
        seed.sync()
        const previous = createTemporaryObject(hostComponent, testCase)
        tryCompare(previous.placement, "restored", true)
        previous.showMaximized()
        tryCompare(previous, "visibility", Window.Maximized)
        wait(40)
        previous.placement.save()
        const maximizedWidth = previous.width
        const maximizedHeight = previous.height
        previous.placement.enabled = false
        previous.close()
        if (!data.cached) {
            seed.setValue("maximizedScreen", "a removed display")
            seed.sync()
        }

        const host = createTemporaryObject(gridHostComponent, testCase)
        tryCompare(host.placement, "restored", true)
        compare(host.visible, false)
        host.prepare()
        const preparedColumns = host.view.columns
        if (data.cached) {
            compare(host.width, maximizedWidth)
            compare(host.height, maximizedHeight)
        }
        host.placement.showRestored()
        tryCompare(host, "firstFrameReported", true)
        compare(host.firstFrameContentY, 500)
        compare(host.firstFrameIndex, 65)
        compare(host.firstFrameColumns, Math.max(1, Math.floor(host.width / 100)))
        if (data.cached)
            compare(host.firstFrameColumns, preparedColumns)

        host.showNormal()
        tryCompare(host, "visibility", Window.Windowed)
        compare(host.width, 500)
        compare(host.height, 400)
        host.placement.enabled = false
        host.close()
    }

    Component {
        id: hostComponent
        Window {
            id: host
            visible: true
            minimumWidth: 200
            minimumHeight: 150
            property alias placement: placement
            property alias deferPresentation: placement.deferPresentation
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

    function test_saved_window_state_data() {
        return [
            { tag: "string false", flag: "false", maximized: false },
            { tag: "boolean false", flag: false, maximized: false },
            { tag: "string true", flag: "true", maximized: true },
            { tag: "boolean true", flag: true, maximized: true }
        ]
    }

    function test_deferred_maximize_does_not_show_or_save_intermediate_geometry() {
        seed.setValue("normalWidth", 500)
        seed.setValue("normalHeight", 400)
        seed.setValue("positioned", false)
        seed.setValue("maximized", true)
        seed.sync()
        const host = createTemporaryObject(hostComponent, testCase,
                                           { visible: false, deferPresentation: true })
        verify(host)
        tryCompare(host.placement, "restored", true)
        compare(host.visible, false)
        compare(host.placement.restoreMaximized, true)
        host.width = 600
        host.placement.save()
        compare(seed.value("normalWidth"), 500)
        host.placement.showRestored()
        tryCompare(host, "visibility", Window.Maximized)
        compare(host.placement.presentationReady, true)
        host.placement.enabled = false
        host.close()
    }

    function test_saved_window_state(data) {
        // QSettings INI values can be strings in a fresh process, unlike the
        // typed values retained by an in-process save/restore test.
        seed.setValue("normalWidth", 500)
        seed.setValue("normalHeight", 400)
        seed.setValue("positioned", "false")
        seed.setValue("maximized", data.flag)
        seed.sync()
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        tryCompare(host.placement, "restored", true)
        compare(host.visibility, data.maximized ? Window.Maximized : Window.Windowed)
        if (!data.maximized) {
            compare(host.width, 500)
            compare(host.height, 400)
        }
        compare(host.placement.savedFlag("positioned"), false)
        host.placement.enabled = false
        host.close()
    }
}
