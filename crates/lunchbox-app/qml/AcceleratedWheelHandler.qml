import QtQuick

WheelHandler {
    required property Flickable scroller
    target: null
    property double lastNotchAt: 0
    property int burstCount: 0
    property real projectedY: 0

    property NumberAnimation wheelMomentum: NumberAnimation {
        id: wheelMomentum
        target: scroller
        property: "contentY"
        easing.type: Easing.OutCubic
    }

    property Connections dragConnections: Connections {
        target: scroller
        function onDraggingChanged() {
            if (scroller.dragging)
                wheelMomentum.stop()
        }
    }

    onWheel: function(event) {
        let distance = 0
        let duration = 160
        const pixelDelta = event.pixelDelta.y
        if (pixelDelta !== 0) {
            distance = -pixelDelta * 3.4
            burstCount = 0
            lastNotchAt = 0
        } else {
            const steps = event.angleDelta.y / 120
            if (steps === 0)
                return
            const now = Date.now()
            burstCount = now - lastNotchAt <= 190
                       ? Math.min(10, burstCount + 1) : 0
            lastNotchAt = now
            const acceleration = Math.min(10.0, 1 + burstCount * 0.82)
            const baseDistance = Math.max(420,
                                          Math.min(760, scroller.height * 0.48))
            distance = -steps * baseDistance * acceleration
            duration = Math.min(460, 240 + burstCount * 22)
        }

        const lowerBound = scroller.originY
        const upperBound = lowerBound
                + Math.max(0, scroller.contentHeight - scroller.height)
        const base = wheelMomentum.running ? projectedY : scroller.contentY
        projectedY = Math.max(lowerBound,
                              Math.min(upperBound, base + distance))
        wheelMomentum.stop()
        wheelMomentum.from = scroller.contentY
        wheelMomentum.to = projectedY
        wheelMomentum.duration = duration
        wheelMomentum.start()
        event.accepted = true
    }
}
