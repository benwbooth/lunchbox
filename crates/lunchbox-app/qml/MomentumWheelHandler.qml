import QtQuick

AcceleratedWheelHandler {
    // Opt-in glide for a compact surface. Most forms and short lists now use
    // Qt's direct wheel scrolling instead.
    wheelPageFactor: 0.8
    minimumPageDistance: 120
    maximumPageDistance: 1000
    frictionPerSecond: 5.2
    maximumVelocity: 12000
}
