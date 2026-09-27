import QtQuick

QtObject {
    id: controller

    property url desiredSource: ""
    property url activeSource: desiredSource
    property int maximumRetries: 2
    property int retryDelay: 180
    property int retryCount: 0
    property string terminalError: ""
    readonly property bool retryPending: retryTimer.running

    signal retryScheduled(int retryNumber, url source)
    signal exhausted(string errorMessage)

    onDesiredSourceChanged: reset(desiredSource)

    function reset(source) {
        retryTimer.stop()
        retryCount = 0
        terminalError = ""
        activeSource = source
    }

    function markReady() {
        retryCount = 0
        terminalError = ""
    }

    function handleFailure(errorMessage) {
        if (desiredSource.toString().length === 0)
            return false

        if (retryCount >= maximumRetries) {
            terminalError = errorMessage
            exhausted(errorMessage)
            return false
        }

        retryCount += 1
        terminalError = ""
        activeSource = ""
        retryTimer.restart()
        return true
    }

    property Timer retryTimer: Timer {
        interval: Math.max(0, controller.retryDelay)
        repeat: false
        onTriggered: {
            if (controller.desiredSource.toString().length === 0)
                return
            controller.activeSource = controller.desiredSource
            controller.retryScheduled(controller.retryCount,
                                      controller.desiredSource)
        }
    }
}
