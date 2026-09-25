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
        handler.minimumPageDistance = 2200
        handler.maximumPageDistance = 5200
        scroller.contentHeight = 100000
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
        verify(scroller.contentY > firstPosition + 450,
               "viewport should retain momentum after input ends")
    }

    function test_travel_follows_scrollable_length_without_exceeding_it() {
        scroller.contentHeight = scroller.height + 120
        const shortTravel = handler.wheelTravelDistance()
        verify(shortTravel > 0 && shortTravel <= 120)

        scroller.contentHeight = scroller.height + 2400
        const mediumTravel = handler.wheelTravelDistance()
        verify(mediumTravel > shortTravel)

        scroller.contentHeight = 100000
        const longTravel = handler.wheelTravelDistance()
        verify(longTravel > mediumTravel)
        verify(longTravel < scroller.contentHeight - scroller.height,
               "one notch must not jump across a large library")
        verify(handler.momentumShare() <= 0.7)
    }

    function test_short_scroll_is_immediate_with_only_a_tiny_tail() {
        scroller.contentHeight = scroller.height + 80
        handler.scrollNotches(1)
        const immediate = scroller.contentY
        verify(immediate >= 70,
               "short content should respond immediately, got " + immediate)
        wait(100)
        verify(scroller.contentY - immediate < 8,
               "short content must not snap after a delayed kinetic step")
    }

    function test_page_distance_bounds_limit_travel() {
        handler.minimumPageDistance = 100
        handler.maximumPageDistance = 150
        handler.scrollNotches(1)
        wait(400)
        verify(scroller.contentY < 400,
               "a bounded page should stay short, got " + scroller.contentY)
    }

    function test_real_wheel_event_reaches_the_handler() {
        verify(handler.parent === scroller || handler.parent === scroller.contentItem,
               "the wheel handler must be scoped to the scroller")
        verify(handler.enabled)
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

    function test_pixel_delta_moves_immediately_without_delayed_jump() {
        scroller.contentY = 5000
        handler.scrollNotches(1)
        const beforePixelPacket = scroller.contentY
        handler.scrollPixels(-42)
        compare(scroller.contentY, beforePixelPacket - 42)
        verify(!handler.momentumRunning)
        const settled = scroller.contentY
        wait(80)
        compare(scroller.contentY, settled)
    }

    function test_momentum_stops_at_bounds() {
        scroller.contentY = scroller.contentHeight - scroller.height
        handler.scrollNotches(1)
        wait(40)
        compare(scroller.contentY, scroller.contentHeight - scroller.height)
        verify(!handler.momentumRunning)
    }
}
