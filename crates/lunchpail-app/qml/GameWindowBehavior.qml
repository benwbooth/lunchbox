import QtQuick
import QtQuick.Window

QtObject {
    id: root

    required property var applicationWindow
    property bool gameRunning: false
    property bool minimizeEnabled: false
    property bool minimizedForRunningGame: false
    property int visibilityBeforeGameMinimize: Window.Windowed

    function syncWindowState() {
        if (gameRunning && minimizeEnabled) {
            if (applicationWindow.visibility === Window.Minimized
                    || minimizedForRunningGame)
                return
            visibilityBeforeGameMinimize = applicationWindow.visibility
            minimizedForRunningGame = true
            applicationWindow.showMinimized()
            return
        }
        if (gameRunning || !minimizedForRunningGame)
            return
        const previousVisibility = visibilityBeforeGameMinimize
        minimizedForRunningGame = false
        if (previousVisibility === Window.FullScreen)
            applicationWindow.showFullScreen()
        else if (previousVisibility === Window.Maximized)
            applicationWindow.showMaximized()
        else
            applicationWindow.showNormal()
        applicationWindow.requestActivate()
    }

    onGameRunningChanged: syncWindowState()
    onMinimizeEnabledChanged: syncWindowState()
}
