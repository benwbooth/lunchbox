import QtQuick

MouseArea {
    id: handler

    required property Flickable scroller
    parent: scroller
    x: 0
    y: 0
    width: scroller.width
    height: scroller.height
    visible: true
    z: 900
    acceptedButtons: Qt.NoButton
    hoverEnabled: false
    scrollGestureEnabled: true

    property real wheelPageFactor: 0.9
    property real frictionPerSecond: 5.2
    property real maximumVelocity: 18000
    property real minimumVelocity: 24
    property real pixelVelocityGain: 38
    readonly property bool momentumRunning: momentumTimer.running
    readonly property real momentumVelocity: velocityX

    property real velocityX: 0
    property double lastFrameAt: 0
    property double lastNotchAt: 0
    property int burstCount: 0
    property int lastDirection: 0
    property bool advancing: false

    function lowerBound() {
        return scroller.originX
    }

    function upperBound() {
        return lowerBound()
                + Math.max(0, scroller.contentWidth - scroller.width)
    }

    function clampContentX(value) {
        return Math.max(lowerBound(), Math.min(upperBound(), value))
    }

    function stopMomentum() {
        momentumTimer.stop()
        velocityX = 0
        lastFrameAt = 0
    }

    function addVelocity(impulse) {
        if (!isFinite(impulse) || impulse === 0)
            return
        const direction = impulse < 0 ? -1 : 1
        if (lastDirection !== 0 && direction !== lastDirection)
            velocityX = 0
        velocityX = Math.max(-maximumVelocity,
                             Math.min(maximumVelocity, velocityX + impulse))
        lastDirection = direction
        lastFrameAt = Date.now()
        if (!momentumTimer.running)
            momentumTimer.start()
    }

    function moveImmediately(distance) {
        if (!isFinite(distance) || distance === 0)
            return
        advancing = true
        scroller.contentX = clampContentX(scroller.contentX + distance)
        advancing = false
    }

    function scrollPixels(distance) {
        moveImmediately(distance)
        addVelocity(distance * pixelVelocityGain)
    }

    function scrollNotches(steps) {
        const now = Date.now()
        const direction = steps < 0 ? -1 : 1
        burstCount = now - lastNotchAt <= 230 && direction === lastDirection
                   ? Math.min(8, burstCount + 1) : 0
        lastNotchAt = now
        const acceleration = Math.min(5.5, 1 + burstCount * 0.6)
        const pageDistance = Math.max(240,
                                      Math.min(900,
                                               scroller.width * wheelPageFactor))
        moveImmediately(steps * pageDistance * 0.32
                        * Math.min(1.8, acceleration))
        addVelocity(steps * pageDistance * frictionPerSecond * acceleration)
    }

    function advanceMomentum() {
        if (velocityX === 0) {
            stopMomentum()
            return
        }
        const now = Date.now()
        const elapsed = lastFrameAt > 0 ? (now - lastFrameAt) / 1000 : 0.016
        const deltaSeconds = Math.max(0.001, Math.min(0.05, elapsed))
        lastFrameAt = now
        const decay = Math.exp(-frictionPerSecond * deltaSeconds)
        const distance = velocityX * (1 - decay) / frictionPerSecond
        const current = scroller.contentX
        const next = clampContentX(current + distance)
        advancing = true
        scroller.contentX = next
        advancing = false
        velocityX *= decay
        const atBound = Math.abs(next - current) < 0.01
                        && ((velocityX < 0 && next <= lowerBound())
                            || (velocityX > 0 && next >= upperBound()))
        if (atBound || Math.abs(velocityX) < minimumVelocity)
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
            if (scroller.dragging)
                handler.stopMomentum()
        }
        function onContentXChanged() {
            if (!handler.advancing && handler.momentumRunning)
                handler.stopMomentum()
        }
    }

    onWheel: function(event) {
        const horizontalPixels = event.pixelDelta.x !== 0
                                 ? -event.pixelDelta.x : -event.pixelDelta.y
        const horizontalAngle = event.angleDelta.x !== 0
                                ? -event.angleDelta.x : -event.angleDelta.y
        if (Math.abs(horizontalAngle) >= 120) {
            scrollNotches(horizontalAngle / 120)
        } else if (horizontalPixels !== 0) {
            scrollPixels(horizontalPixels)
            burstCount = 0
            lastNotchAt = 0
        } else if (horizontalAngle !== 0) {
            scrollNotches(horizontalAngle / 120)
        } else {
            return
        }
        event.accepted = true
    }
}
