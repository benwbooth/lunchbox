import QtQuick

AcceleratedWheelHandler {
    // Shared profile for ordinary surfaces; the handler scales glide from
    // the actual scrollable length.
    wheelPageFactor: 0.8
    minimumPageDistance: 120
    maximumPageDistance: 1000
    frictionPerSecond: 5.2
    maximumVelocity: 12000
}
