import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "AcceleratedWheel"
    when: windowShown
    width: 1000
    height: 800

    Flickable {
        id: scroller
        anchors.fill: parent
        contentWidth: width
        contentHeight: 100000

        Lunchbox.AcceleratedWheelHandler {
            id: handler
            scroller: scroller
        }
    }

    function init() {
        handler.stopMomentum()
        handler.lastDirection = 0
        handler.lastNotchAt = 0
        handler.burstCount = 0
        scroller.contentY = 0
    }

    function cleanup() {
        handler.stopMomentum()
    }

    function test_single_notch_is_fast_and_keeps_moving() {
        handler.scrollNotches(1)
        verify(handler.momentumRunning)
        wait(96)
        const firstPosition = scroller.contentY
        verify(firstPosition > 400,
               "one notch should move more than 400px promptly, got "
               + firstPosition)
        wait(160)
        verify(scroller.contentY > firstPosition + 250,
               "viewport should retain momentum after input ends")
    }

    function test_repeated_notches_accumulate_velocity() {
        handler.scrollNotches(1)
        const firstVelocity = handler.momentumVelocity
        handler.scrollNotches(1)
        verify(handler.momentumVelocity > firstVelocity * 2,
               "second notch should accelerate the existing motion")
    }

    function test_reverse_input_takes_control_immediately() {
        scroller.contentY = 5000
        handler.scrollNotches(1)
        handler.scrollNotches(-1)
        verify(handler.momentumVelocity < 0,
               "opposite input should reverse instead of fighting old momentum")
    }

    function test_momentum_stops_at_bounds() {
        scroller.contentY = scroller.contentHeight - scroller.height
        handler.scrollNotches(1)
        wait(40)
        compare(scroller.contentY, scroller.contentHeight - scroller.height)
        verify(!handler.momentumRunning)
    }
}
