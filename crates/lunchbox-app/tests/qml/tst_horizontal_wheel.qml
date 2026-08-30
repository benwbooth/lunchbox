import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "HorizontalWheel"
    when: windowShown

    Window {
        id: testWindow
        visible: true
        width: 900
        height: 180

        Flickable {
            id: scroller
            anchors.fill: parent
            contentWidth: 6000
            contentHeight: height

            Lunchbox.HorizontalWheelHandler {
                id: handler
                scroller: scroller
            }
        }
    }

    function init() {
        handler.stopMomentum()
        handler.lastDirection = 0
        handler.lastNotchAt = 0
        handler.burstCount = 0
        scroller.contentX = 0
    }

    function cleanup() {
        handler.stopMomentum()
    }

    function test_vertical_wheel_moves_horizontal_strip_with_momentum() {
        mouseWheel(scroller, scroller.width / 2, scroller.height / 2,
                   0, -120, Qt.LeftButton, Qt.NoModifier)
        verify(scroller.contentX > 70,
               "wheel should move the horizontal release strip immediately")
        verify(handler.momentumRunning)
        const immediate = scroller.contentX
        wait(100)
        verify(scroller.contentX > immediate,
               "release strip should retain momentum after the wheel event")
    }

    function test_reverse_input_changes_direction_immediately() {
        scroller.contentX = 2000
        handler.scrollNotches(1)
        handler.scrollNotches(-1)
        verify(handler.momentumVelocity < 0)
    }

    function test_momentum_stops_at_horizontal_bound() {
        scroller.contentX = scroller.contentWidth - scroller.width
        handler.scrollNotches(1)
        wait(40)
        compare(scroller.contentX, scroller.contentWidth - scroller.width)
        verify(!handler.momentumRunning)
    }
}
