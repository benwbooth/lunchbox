import QtQuick
import QtTest
import "../../qml" as Lunchpail

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

            Lunchpail.AcceleratedWheelHandler {
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

    function test_coast_keeps_scaling_beyond_a_few_screens() {
        let previousFactor = 0
        for (const pages of [0.1, 2, 10, 40, 100]) {
            scroller.contentHeight = scroller.height * (1 + pages)
            const factor = handler.momentumLengthFactor()
            verify(factor > previousFactor, "Longer content should retain more momentum")
            previousFactor = factor
        }
        scroller.contentHeight = scroller.height * 1000000
        verify(handler.momentumLengthFactor() <= 5,
               "Even an enormous catalog must have a bounded coast")
    }

    function releaseTravel(pages) {
        handler.stopMomentum()
        scroller.contentHeight = scroller.height * (1 + pages)
        scroller.contentY = 0
        handler.scrollPixels(32, true)
        handler.lastPixelAt = Date.now() - 16
        handler.scrollPixels(32, true)
        compare(scroller.contentY, 64, "Length scaling must never amplify finger motion")
        handler.finishPixelGesture()
        const released = scroller.contentY
        // Advance the real integrator deterministically instead of sleeping
        // through several complete coasts. Each frame uses the same 16 ms.
        let frames = 0
        while (handler.momentumRunning && ++frames < 500) {
            handler.lastFrameAt = Date.now() - 16
            handler.advanceMomentum()
        }
        verify(!handler.momentumRunning, "A release glide must eventually stop")
        return scroller.contentY - released
    }

    function test_trackpad_coasts_much_farther_in_long_content() {
        const shortTravel = releaseTravel(1)
        const mediumTravel = releaseTravel(10)
        const longTravel = releaseTravel(100)
        verify(mediumTravel > shortTravel * 2,
               "Medium content must glide farther: " + mediumTravel + " vs " + shortTravel)
        verify(longTravel > mediumTravel * 1.5,
               "Large catalogs must not plateau after ten screens")
        verify(longTravel > 1500,
               "The sample flick should coast several grid rows, got " + longTravel)
    }

    function test_short_trackpad_overflow_stays_controlled() {
        const travel = releaseTravel(0.125)
        verify(travel <= 36, "A compact pane must not overshoot its remaining content")
        compare(scroller.contentY, 100)
    }

    function test_long_trackpad_coast_can_reverse_immediately() {
        pixelSwipe()
        wheelPacket(0, 0, Qt.ScrollEnd, PointerDevice.TouchPad)
        const before = scroller.contentY
        wheelPacket(24, 12, Qt.ScrollUpdate, PointerDevice.TouchPad)
        compare(scroller.contentY, before - 24)
        verify(!handler.momentumRunning, "The next finger motion must interrupt the longer coast")
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

    function test_pixel_rounding_does_not_cancel_but_navigation_does() {
        handler.scrollNotches(1)
        verify(handler.momentumRunning)
        scroller.contentY = Math.round(scroller.contentY)
        verify(handler.momentumRunning,
               "A deferred pixel alignment is not a manual scroll")
        scroller.contentY += 80
        verify(!handler.momentumRunning,
               "A scrollbar or navigation jump should take control")
    }

    function wheelPacket(pixels, angle, phase, device) {
        const event = { pixelDelta: Qt.point(0, pixels), angleDelta: Qt.point(0, angle),
                        phase: phase, device: { type: device }, accepted: false }
        handler.handleWheel(event)
        return event
    }

    function pixelSwipe() {
        wheelPacket(0, 0, Qt.ScrollBegin, PointerDevice.TouchPad)
        for (let i = 0; i < 4; ++i) {
            wheelPacket(-24, -12, Qt.ScrollUpdate, PointerDevice.TouchPad)
            wait(16)
        }
    }

    function test_touchpad_tracks_fingers_then_glides_on_release() {
        pixelSwipe()
        compare(scroller.contentY, 96, "Finger movement must be direct, not accelerated")
        verify(!handler.momentumRunning, "Do not coast while fingers are still scrolling")
        wheelPacket(0, 0, Qt.ScrollEnd, PointerDevice.TouchPad)
        verify(handler.momentumRunning, "A touchpad without native inertia needs a release glide")
        wait(100)
        verify(scroller.contentY > 150, "Lifting fingers must not stop the grid dead")
    }

    function test_native_inertia_does_not_get_a_second_tail() {
        pixelSwipe()
        wheelPacket(0, 0, Qt.ScrollEnd, PointerDevice.TouchPad)
        verify(handler.momentumRunning)
        const released = scroller.contentY
        wheelPacket(-12, -6, Qt.ScrollMomentum, PointerDevice.TouchPad)
        compare(scroller.contentY, released + 12)
        verify(!handler.momentumRunning, "Native inertia replaces synthesized inertia")
        wheelPacket(0, 0, Qt.ScrollEnd, PointerDevice.TouchPad)
        const stopped = scroller.contentY
        wait(100)
        compare(scroller.contentY, stopped, "Never coast again after native inertia ends")
    }

    function test_touchpad_pause_or_new_gesture_stops_glide() {
        pixelSwipe()
        wait(150)
        wheelPacket(0, 0, Qt.ScrollEnd, PointerDevice.TouchPad)
        verify(!handler.momentumRunning, "A finger pause before release means stop")
        pixelSwipe()
        wheelPacket(0, 0, Qt.ScrollEnd, PointerDevice.TouchPad)
        verify(handler.momentumRunning)
        wheelPacket(0, 0, Qt.ScrollBegin, PointerDevice.TouchPad)
        verify(!handler.momentumRunning, "Touching down again takes control")
    }

    function test_unphased_pixel_stream_gets_one_release_glide() {
        for (let i = 0; i < 4; ++i) {
            wheelPacket(-20, 0, Qt.NoScrollPhase, PointerDevice.Mouse)
            wait(16)
        }
        compare(scroller.contentY, 80)
        wait(140)
        verify(scroller.contentY > 120, "Unphased smooth scrolling still needs momentum")
    }

    function test_small_mouse_wheel_deltas_retain_momentum() {
        wheelPacket(-2, -15, Qt.NoScrollPhase, PointerDevice.Mouse)
        verify(handler.momentumRunning, "Fractional notches must not use the no-inertia path")
        const immediate = scroller.contentY
        wait(100)
        verify(scroller.contentY > immediate + 40)
        handler.stopMomentum()
        handler.lastNotchAt = 0
        handler.burstCount = 0
        for (let i = 0; i < 8; ++i)
            wheelPacket(-2, -15, Qt.NoScrollPhase, PointerDevice.Mouse)
        verify(handler.burstCount < 1, "Acceleration counts full notches, not tiny packets")
    }

    function test_navigation_cancels_pending_pixel_release() {
        handler.scrollPixels(24)
        wait(16)
        handler.scrollPixels(24)
        scroller.contentY = 500
        wait(150)
        compare(scroller.contentY, 500)
        verify(!handler.momentumRunning)
    }
}
