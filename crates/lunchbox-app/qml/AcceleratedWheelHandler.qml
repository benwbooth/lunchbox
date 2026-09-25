import QtQuick

WheelHandler {
    id: handler

    required property Flickable scroller
    // A pointer handler receives wheels over delegates without a visual layer
    // that can steal events from nested scrollable panes or their scrollbars.
    parent: scroller
    target: null
    blocking: false
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad

    // A mouse-wheel notch should cover meaningful ground in a large library.
    // These bounds shape each surface; the actual travel also follows the
    // amount of content available to scroll.
    property real wheelPageFactor: 3.2
    // Page travel bounds. Large grids want a big page; a compact sidebar row
    // list wants a short one, so the bounds are configurable per scroller.
    property real minimumPageDistance: 2200
    property real maximumPageDistance: 5200
    property real frictionPerSecond: 3.8
    property real maximumVelocity: 120000
    property real minimumVelocity: 70
    readonly property bool momentumRunning: momentumTimer.running
    readonly property real momentumVelocity: velocityY

    property real velocityY: 0
    property double lastFrameAt: 0
    property double lastNotchAt: 0
    property int burstCount: 0
    property int lastDirection: 0
    property bool advancing: false

    onEnabledChanged: {
        if (!enabled)
            stopMomentum()
    }

    function lowerBound() {
        return scroller.originY
    }

    function upperBound() {
        return lowerBound()
                + Math.max(0, scroller.contentHeight - scroller.height)
    }

    function scrollableLength() {
        return upperBound() - lowerBound()
    }

    function wheelTravelDistance() {
        const length = scrollableLength()
        if (length <= 0)
            return 0
        const viewport = Math.max(1, scroller.height)
        const preferred = Math.max(minimumPageDistance,
                                   Math.min(maximumPageDistance,
                                            viewport * wheelPageFactor))
        // Small overflow gets a short glide; large libraries approach the
        // surface's configured page distance without making one notch jump
        // across the entire catalog.
        return Math.min(length, preferred * Math.sqrt(
                            length / (length + 2 * viewport)))
    }

    function momentumShare() {
        const length = scrollableLength()
        return Math.min(0.7, length / (length + Math.max(1, scroller.height)))
    }

    function clampContentY(value) {
        return Math.max(lowerBound(), Math.min(upperBound(), value))
    }

    function stopMomentum() {
        momentumTimer.stop()
        velocityY = 0
        lastFrameAt = 0
    }

    function addVelocity(impulse) {
        if (!isFinite(impulse) || impulse === 0)
            return

        const direction = impulse < 0 ? -1 : 1
        if (lastDirection !== 0 && direction !== lastDirection)
            velocityY = 0

        velocityY = Math.max(-maximumVelocity,
                             Math.min(maximumVelocity, velocityY + impulse))
        lastDirection = direction
        lastFrameAt = Date.now()
        if (!momentumTimer.running)
            momentumTimer.start()
    }

    function scrollPixels(distance) {
        // Touchpads already send a stream of pixel deltas, including their
        // native kinetic tail. Apply each packet now instead of waiting for a
        // timer and multiplying it into a delayed, oversized jump.
        stopMomentum()
        moveImmediately(distance)
    }

    function moveImmediately(distance) {
        if (!isFinite(distance) || distance === 0)
            return
        advancing = true
        scroller.contentY = clampContentY(scroller.contentY + distance)
        advancing = false
    }

    function scrollNotches(steps) {
        const now = Date.now()
        const direction = steps < 0 ? -1 : 1
        burstCount = now - lastNotchAt <= 230 && direction === lastDirection
                   ? Math.min(10, burstCount + 1) : 0
        lastNotchAt = now

        const acceleration = Math.min(9.0, 1 + burstCount * 0.7)
        const pageDistance = wheelTravelDistance()
        const kineticShare = momentumShare()
        // Give each physical notch an immediate response before the kinetic
        // tail takes over. Short scroll regions get mostly direct movement;
        // longer ones retain more glide.
        moveImmediately(steps * pageDistance * (1 - kineticShare)
                        * Math.min(2.0, acceleration))
        // Under exponential friction the remaining distance is velocity / k.
        addVelocity(steps * pageDistance * kineticShare
                    * frictionPerSecond * acceleration)
    }

    function advanceMomentum() {
        if (velocityY === 0) {
            stopMomentum()
            return
        }

        const now = Date.now()
        const elapsed = lastFrameAt > 0 ? (now - lastFrameAt) / 1000 : 0.016
        const deltaSeconds = Math.max(0.001, Math.min(0.05, elapsed))
        lastFrameAt = now

        const decay = Math.exp(-frictionPerSecond * deltaSeconds)
        const distance = velocityY * (1 - decay) / frictionPerSecond
        const current = scroller.contentY
        const next = clampContentY(current + distance)
        advancing = true
        scroller.contentY = next
        advancing = false
        velocityY *= decay

        const atBound = Math.abs(next - current) < 0.01
                        && ((velocityY < 0 && next <= lowerBound())
                            || (velocityY > 0 && next >= upperBound()))
        if (atBound || Math.abs(velocityY) < minimumVelocity)
            stopMomentum()
    }

    property Timer momentumTimer: Timer {
        id: momentumTimer
        interval: 16
        repeat: true
        onTriggered: handler.advanceMomentum()
    }

    property Connections scrollerConnections: Connections {
        target: scroller

        function onDraggingChanged() {
            if (scroller.dragging) {
                handler.stopMomentum()
                handler.lastDirection = 0
                handler.burstCount = 0
            }
        }

        function onContentYChanged() {
            // Scrollbar drags and programmatic navigation take ownership from
            // wheel momentum immediately. Changes made by our frame timer are
            // marked so they do not cancel themselves.
            if (!handler.advancing && handler.momentumRunning)
                handler.stopMomentum()
        }
    }

    onWheel: function(event) {
        const distance = Math.abs(event.angleDelta.y) >= 120
                         ? -event.angleDelta.y
                         : event.pixelDelta.y !== 0
                           ? -event.pixelDelta.y : -event.angleDelta.y
        // Let an enclosing pane take over when this one has reached its edge.
        if ((distance < 0 && scroller.contentY <= lowerBound() + 0.5)
                || (distance > 0 && scroller.contentY >= upperBound() - 0.5)) {
            stopMomentum()
            event.accepted = false
            return
        }
        // X11/Wayland mouse wheels commonly provide both deltas. The angle
        // delta represents real wheel notches; preferring the tiny synthetic
        // pixel delta was the reason scrolling barely moved on Linux.
        const notchSteps = event.angleDelta.y / 120
        if (event.device && event.device.type === PointerDevice.TouchPad
                && event.pixelDelta.y !== 0) {
            scrollPixels(-event.pixelDelta.y)
            burstCount = 0
            lastNotchAt = 0
        } else if (Math.abs(event.angleDelta.y) >= 120) {
            scrollNotches(-notchSteps)
        } else if (event.pixelDelta.y !== 0) {
            scrollPixels(-event.pixelDelta.y)
            burstCount = 0
            lastNotchAt = 0
        } else if (event.angleDelta.y !== 0) {
            scrollNotches(-notchSteps)
        } else {
            return
        }
        event.accepted = true
    }
}
