import QtQuick

WheelHandler {
    required property Flickable scroller
    target: null
    property double lastNotchAt: 0
    property double lastPixelAt: 0
    property int burstCount: 0
    property int pixelBurstCount: 0
    property int lastDirection: 0
    property real projectedY: 0

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

    function startMomentum(distance, duration) {
        if (distance === 0)
            return
        const direction = distance < 0 ? -1 : 1
        const base = wheelMomentum.running && direction === lastDirection
                   ? projectedY : scroller.contentY
        projectedY = clampContentY(base + distance)
        wheelMomentum.stop()
        wheelMomentum.from = scroller.contentY
        wheelMomentum.to = projectedY
        wheelMomentum.duration = duration
        lastDirection = direction
        wheelMomentum.start()
    }

    function scrollPixels(distance) {
        const now = Date.now()
        const direction = distance < 0 ? -1 : 1
        pixelBurstCount = now - lastPixelAt <= 90 && direction === lastDirection
                        ? Math.min(12, pixelBurstCount + 1) : 0
        lastPixelAt = now
        const acceleration = Math.min(4.2, 1 + pixelBurstCount * 0.28)
        startMomentum(distance * 9.5 * acceleration,
                      Math.min(420, 240 + pixelBurstCount * 15))
    }

    function scrollNotches(steps) {
        const now = Date.now()
        const direction = steps < 0 ? -1 : 1
        burstCount = now - lastNotchAt <= 210 && direction === lastDirection
                   ? Math.min(8, burstCount + 1) : 0
        lastNotchAt = now

        const acceleration = Math.min(8.2, 1 + burstCount * 0.9)
        const baseDistance = Math.max(900,
                                      Math.min(1800, scroller.height * 1.25))
        startMomentum(steps * baseDistance * acceleration,
                      Math.min(460, 260 + burstCount * 22))
    }

    property NumberAnimation wheelMomentum: NumberAnimation {
        id: wheelMomentum
        target: scroller
        property: "contentY"
        easing.type: Easing.OutQuart
    }

    property Connections dragConnections: Connections {
        target: scroller
        function onDraggingChanged() {
            if (scroller.dragging) {
                wheelMomentum.stop()
                projectedY = scroller.contentY
                lastDirection = 0
                burstCount = 0
                pixelBurstCount = 0
            }
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
