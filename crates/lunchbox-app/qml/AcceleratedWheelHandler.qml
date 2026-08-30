import QtQuick

WheelHandler {
    id: handler

    required property Flickable scroller
    target: null

    // A mouse-wheel notch should cover meaningful ground in a large library.
    // The velocity model lets quick successive notches accumulate naturally,
    // while the friction value keeps a single notch predictable.
    property real wheelPageFactor: 1.85
    property real frictionPerSecond: 4.6
    property real maximumVelocity: 72000
    property real minimumVelocity: 70
    property real pixelVelocityGain: 72
    readonly property bool momentumRunning: momentumTimer.running
    readonly property real momentumVelocity: velocityY

    property real velocityY: 0
    property double lastFrameAt: 0
    property double lastNotchAt: 0
    property int burstCount: 0
    property int lastDirection: 0
    property bool advancing: false

    function lowerBound() {
        return scroller.originY
    }

    function upperBound() {
        return lowerBound()
                + Math.max(0, scroller.contentHeight - scroller.height)
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
        // Pixel deltas come from touchpads and high-resolution wheels. Treat
        // them as velocity samples so a gesture keeps its momentum after the
        // last event instead of stopping at the final packet.
        addVelocity(distance * pixelVelocityGain)
    }

    function scrollNotches(steps) {
        const now = Date.now()
        const direction = steps < 0 ? -1 : 1
        burstCount = now - lastNotchAt <= 230 && direction === lastDirection
                   ? Math.min(10, burstCount + 1) : 0
        lastNotchAt = now

        const acceleration = Math.min(6.5, 1 + burstCount * 0.55)
        const pageDistance = Math.max(1400,
                                      Math.min(2800,
                                               scroller.height * wheelPageFactor))
        // Under exponential friction the remaining distance is velocity / k.
        // Multiplying by k therefore makes pageDistance the travel of one
        // isolated notch, independent of monitor refresh rate.
        addVelocity(steps * pageDistance * frictionPerSecond * acceleration)
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
        // X11/Wayland mouse wheels commonly provide both deltas. The angle
        // delta represents real wheel notches; preferring the tiny synthetic
        // pixel delta was the reason scrolling barely moved on Linux.
        const notchSteps = event.angleDelta.y / 120
        if (Math.abs(event.angleDelta.y) >= 120) {
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
