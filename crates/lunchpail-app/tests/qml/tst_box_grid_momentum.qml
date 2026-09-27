import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "BoxGridMomentum"
    when: windowShown
    visible: true
    width: 1000
    height: 800

    // Match the desktop grid: a virtualized/recycling view, attached slim
    // scrollbar and the accelerated (not ordinary Flickable) wheel profile.
    Lunchpail.MomentumGridView {
        id: grid
        anchors.fill: parent
        defaultWheelMomentum: false
        model: 10000
        reuseItems: true
        clip: true
        cacheBuffer: height
        cellWidth: 240
        cellHeight: 330
        rightMargin: 50
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: Lunchpail.LbScrollBar { policy: ScrollBar.AlwaysOn }
        Lunchpail.AcceleratedWheelHandler {
            id: wheel
            scroller: grid
            blocking: true
        }
        delegate: Rectangle {
            required property int index
            width: grid.cellWidth
            height: grid.cellHeight
            color: index % 2 ? "#223344" : "#334455"
        }
    }

    function init() {
        wheel.stopMomentum()
        wheel.lastNotchAt = 0
        wheel.lastDirection = 0
        grid.contentY = 12000
        wait(50)
    }

    function cleanup() { wheel.stopMomentum() }

    function test_notched_wheel_keeps_gliding_after_input() {
        mouseWheel(grid, 400, 350, 0, -120, Qt.NoButton, Qt.NoModifier)
        const start = grid.contentY
        verify(wheel.momentumRunning)
        wait(120)
        verify(grid.contentY > start + 300)
        const middle = grid.contentY
        wait(150)
        verify(grid.contentY > middle + 200)
    }

    function test_touchpad_release_keeps_gliding_across_rows() {
        for (let i = 0; i < 6; ++i) {
            wheel.handleWheel({pixelDelta: Qt.point(0, -50), angleDelta: Qt.point(0, -25),
                               device: {type: PointerDevice.TouchPad}, phase: Qt.ScrollUpdate})
            wait(16)
        }
        const released = grid.contentY
        wheel.handleWheel({pixelDelta: Qt.point(0, 0), angleDelta: Qt.point(0, 0),
                           device: {type: PointerDevice.TouchPad}, phase: Qt.ScrollEnd})
        verify(wheel.momentumRunning)
        wait(150)
        verify(grid.contentY > released + 240, "Release should glide through virtualized rows")
        // Previously the fixed friction exhausted this flick in about one
        // second / two rows, despite the grid containing 10,000 games.
        for (let i = 0; i < 180; ++i) {
            wheel.lastFrameAt = Date.now() - 16
            wheel.advanceMomentum()
        }
        verify(wheel.momentumRunning, "A large library should retain a long glide")
        verify(grid.contentY > released + grid.cellHeight * 7,
               "A large-grid flick should coast well beyond a couple of rows")
        wheel.handleWheel({pixelDelta: Qt.point(0, 0), angleDelta: Qt.point(0, 0),
                           device: {type: PointerDevice.TouchPad}, phase: Qt.ScrollBegin})
        verify(!wheel.momentumRunning, "Touching the trackpad must stop the coast immediately")
    }
}
