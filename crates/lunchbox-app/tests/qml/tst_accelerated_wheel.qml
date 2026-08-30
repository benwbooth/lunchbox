import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "AcceleratedWheel"
    when: windowShown
    Window {
        id: testWindow
        visible: true
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
        verify(scroller.contentY > 550,
               "one notch should move immediately, got " + scroller.contentY)
        wait(96)
        const firstPosition = scroller.contentY
        verify(firstPosition > 1100,
               "one notch should move more than 1100px promptly, got "
               + firstPosition)
        wait(160)
        verify(scroller.contentY > firstPosition + 650,
               "viewport should retain momentum after input ends")
    }

    function test_real_wheel_event_reaches_the_handler() {
        compare(handler.parent, scroller,
                "the wheel interceptor must be parented to the viewport")
        verify(handler.visible && handler.enabled
               && handler.width === scroller.width
               && handler.height === scroller.height,
               "the wheel interceptor must cover the viewport: visible="
               + handler.visible + " enabled=" + handler.enabled + " size="
               + handler.width + "x" + handler.height + " viewport="
               + scroller.width + "x" + scroller.height)
        mouseWheel(scroller, scroller.width / 2, scroller.height / 2,
                   0, -120, Qt.LeftButton, Qt.NoModifier)
        verify(scroller.contentY > 550,
               "a delivered wheel event should use the accelerated path, got "
               + scroller.contentY)
        verify(handler.momentumRunning,
               "a delivered wheel event should retain kinetic momentum")
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
