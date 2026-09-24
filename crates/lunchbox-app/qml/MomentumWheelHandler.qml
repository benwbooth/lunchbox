import QtQuick

AcceleratedWheelHandler {
    // A modest, viewport-relative glide for forms and compact lists. The
    // library grid keeps its deliberately faster tuned handler.
    wheelPageFactor: 0.8
    minimumPageDistance: 120
    maximumPageDistance: 1000
    frictionPerSecond: 5.2
    maximumVelocity: 12000
    pixelVelocityGain: 18
}
