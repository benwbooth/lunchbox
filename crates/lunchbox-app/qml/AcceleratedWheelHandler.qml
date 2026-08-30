import QtQuick

WheelHandler {
    required property Flickable scroller
    target: null
    property double lastNotchAt: 0
    property int burstCount: 0
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

    function scrollPixels(distance) {
        // Precision touchpads already emit a smooth stream with their own
        // momentum. Applying a fresh easing animation to every event makes
        // that stream feel sticky, so move it immediately instead.
        wheelMomentum.stop()
        projectedY = clampContentY(scroller.contentY + distance)
        scroller.contentY = projectedY
    }

    function scrollNotches(steps) {
        const now = Date.now()
        burstCount = now - lastNotchAt <= 190
                   ? Math.min(10, burstCount + 1) : 0
        lastNotchAt = now

        const acceleration = Math.min(12.0, 1 + burstCount * 1.05)
        const baseDistance = Math.max(620,
                                      Math.min(1050, scroller.height * 0.78))
        const base = wheelMomentum.running ? projectedY : scroller.contentY
        projectedY = clampContentY(base + steps * baseDistance * acceleration)
        wheelMomentum.stop()
        wheelMomentum.from = scroller.contentY
        wheelMomentum.to = projectedY
        wheelMomentum.duration = Math.max(90, 150 - burstCount * 6)
        wheelMomentum.start()
    }

    property NumberAnimation wheelMomentum: NumberAnimation {
        id: wheelMomentum
        target: scroller
        property: "contentY"
        easing.type: Easing.OutCubic
    }

    property Connections dragConnections: Connections {
        target: scroller
        function onDraggingChanged() {
            if (scroller.dragging) {
                wheelMomentum.stop()
                projectedY = scroller.contentY
            }
        }
    }

    onWheel: function(event) {
        // X11/Wayland mouse wheels commonly provide both deltas. The angle
        // delta represents real wheel notches; preferring the tiny synthetic
        // pixel delta was the reason scrolling barely moved on Linux.
        const notchSteps = event.angleDelta.y / 120
        if (notchSteps !== 0) {
            scrollNotches(-notchSteps)
        } else if (event.pixelDelta.y !== 0) {
            scrollPixels(-event.pixelDelta.y * 5.5)
            burstCount = 0
            lastNotchAt = 0
        } else {
            return
        }
        event.accepted = true
    }
}
