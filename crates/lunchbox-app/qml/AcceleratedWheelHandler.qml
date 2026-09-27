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
    property real coastFriction: frictionPerSecond
    property double lastFrameAt: 0
    property double lastNotchAt: 0
    property real burstCount: 0
    property int lastDirection: 0
    property bool advancing: false
    property real lastAppliedContentY: NaN
    property double lastPixelAt: 0
    property real pixelVelocity: 0
    property int pixelSamples: 0
    property bool pixelGestureActive: false

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

    function momentumLengthFactor() {
        const pages = scrollableLength() / Math.max(1, scroller.height)
        // A fixed friction stopped a trackpad flick after roughly one row,
        // whether there were ten games or ten thousand. Grow the coast with
        // the number of scrollable screens, not the input packet size. The
        // logarithm and cap keep huge catalogs from coasting indefinitely.
        return 1 + Math.min(4, 0.6 * Math.log(Math.max(1, pages)) / Math.LN2)
    }

    function effectiveFriction() {
        return frictionPerSecond / momentumLengthFactor()
    }

    function clampContentY(value) {
        return Math.max(lowerBound(), Math.min(upperBound(), value))
    }

    function stopMomentum() {
        momentumTimer.stop()
        pixelReleaseTimer.stop()
        velocityY = 0
        lastFrameAt = 0
        lastAppliedContentY = NaN
        lastPixelAt = 0
        pixelVelocity = 0
        pixelSamples = 0
        pixelGestureActive = false
    }

    function addVelocity(impulse, friction) {
        if (!isFinite(impulse) || impulse === 0)
            return

        const direction = impulse < 0 ? -1 : 1
        if (lastDirection !== 0 && direction !== lastDirection)
            velocityY = 0

        velocityY = Math.max(-maximumVelocity,
                             Math.min(maximumVelocity, velocityY + impulse))
        coastFriction = friction === undefined ? frictionPerSecond : friction
        lastDirection = direction
        lastFrameAt = Date.now()
        if (!momentumTimer.running)
            momentumTimer.start()
    }

    function scrollPixels(distance, phased) {
        // Follow the fingers exactly while they move. Not every platform
        // supplies a kinetic tail (notably libinput touchpads), so retain the
        // release velocity instead of unconditionally discarding momentum.
        const now = Date.now()
        const elapsed = now - lastPixelAt
        if (!pixelGestureActive || elapsed > 120) {
            pixelVelocity = 0
            pixelSamples = 0
        } else {
            const sample = distance / (Math.max(8, elapsed) / 1000)
            pixelVelocity = pixelVelocity * 0.35 + sample * 0.65
        }
        if (pixelVelocity * distance < 0)
            pixelVelocity = 0
        ++pixelSamples
        pixelGestureActive = true
        lastPixelAt = now
        momentumTimer.stop()
        velocityY = 0
        lastFrameAt = 0
        moveImmediately(distance)
        // Phased gestures release on ScrollEnd. Older/unphased devices need
        // a short inactivity fallback, never an extra glide between packets.
        if (phased)
            pixelReleaseTimer.stop()
        else
            pixelReleaseTimer.restart()
    }

    function finishPixelGesture() {
        pixelReleaseTimer.stop()
        if (!pixelGestureActive)
            return
        const releasedVelocity = pixelSamples >= 2 && Date.now() - lastPixelAt <= 120
                ? pixelVelocity * momentumShare() / 0.7 : 0
        pixelGestureActive = false
        pixelVelocity = 0
        pixelSamples = 0
        lastPixelAt = 0
        if (Math.abs(releasedVelocity) >= minimumVelocity)
            addVelocity(releasedVelocity, effectiveFriction())
    }

    function moveImmediately(distance) {
        if (!isFinite(distance) || distance === 0)
            return
        advancing = true
        lastAppliedContentY = clampContentY(scroller.contentY + distance)
        scroller.contentY = lastAppliedContentY
        advancing = false
    }

    function scrollNotches(steps) {
        const now = Date.now()
        const direction = steps < 0 ? -1 : 1
        burstCount = now - lastNotchAt <= 230 && direction === lastDirection
                   ? Math.min(10, burstCount + Math.abs(steps)) : 0
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
        // Mouse notches already use length-scaled page travel. Preserve their
        // response and bounds; only trackpads use the longer release coast.
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

        const friction = coastFriction
        const decay = Math.exp(-friction * deltaSeconds)
        const distance = velocityY * (1 - decay) / friction
        const current = scroller.contentY
        const next = clampContentY(current + distance)
        advancing = true
        lastAppliedContentY = next
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

    property Timer pixelReleaseTimer: Timer {
        interval: 60
        onTriggered: handler.finishPixelGesture()
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
            // marked so they do not cancel themselves. GridView can also
            // round our position to a pixel on a later layout pass; that is
            // still our movement, not a scrollbar/navigation takeover.
            if (!handler.advancing && (handler.momentumRunning || handler.pixelGestureActive)
                    && Math.abs(scroller.contentY - handler.lastAppliedContentY) > 1)
                handler.stopMomentum()
        }
    }

    function handleWheel(event) {
        const phase = event.phase === undefined ? Qt.NoScrollPhase : event.phase
        if (phase === Qt.ScrollBegin)
            stopMomentum()
        if (phase === Qt.ScrollEnd) {
            const handled = pixelGestureActive
            finishPixelGesture()
            event.accepted = handled
            return
        }
        // Native inertia (e.g. macOS) owns its tail. Never add a second one.
        if (phase === Qt.ScrollMomentum) {
            stopMomentum()
            moveImmediately(-event.pixelDelta.y)
            event.accepted = event.pixelDelta.y !== 0
            return
        }
        const touchpad = event.device && event.device.type === PointerDevice.TouchPad
        const pixelScroll = event.pixelDelta.y !== 0
                && (touchpad || phase !== Qt.NoScrollPhase || event.angleDelta.y === 0)
        const distance = pixelScroll ? -event.pixelDelta.y : -event.angleDelta.y
        if (distance === 0) {
            event.accepted = false
            return
        }
        // Let an enclosing pane take over when this one has reached its edge.
        if ((distance < 0 && scroller.contentY <= lowerBound() + 0.5)
                || (distance > 0 && scroller.contentY >= upperBound() - 0.5)) {
            stopMomentum()
            event.accepted = false
            return
        }
        if (pixelScroll) {
            scrollPixels(distance, phase !== Qt.NoScrollPhase)
            burstCount = 0
            lastNotchAt = 0
        } else {
            // High-resolution mouse wheels can emit fractions of a notch
            // alongside synthetic pixel deltas. They still need inertia.
            pixelReleaseTimer.stop()
            pixelGestureActive = false
            scrollNotches(-event.angleDelta.y / 120)
        }
        event.accepted = true
    }

    onWheel: event => handleWheel(event)
}
