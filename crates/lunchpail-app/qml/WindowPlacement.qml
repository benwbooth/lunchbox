import QtQuick
import QtCore

Item {
    id: placement
    required property var window
    property string settingsCategory: "WindowPlacement"
    property url settingsLocation: ""
    property bool restored: false
    // Startup restores geometry without mapping the window. The owner decides
    // when the final content is ready and calls showRestored() exactly once.
    property bool deferPresentation: false
    property bool presentationReady: !deferPresentation
    property bool restoreMaximized: false
    property rect normalGeometry: Qt.rect(0, 0, 1440, 900)
    property bool normalPositioned: false
    property bool restoreNormalAfterMaximize: false
    property int previousVisibility: Window.Hidden

    Settings {
        id: saved
        category: placement.settingsCategory
        location: placement.settingsLocation
    }

    function savedFlag(key) {
        // INI-backed QSettings can return strings after a process restart.
        // In particular, Boolean("false") is true in JavaScript.
        const value = saved.value(key, false)
        return value === true || value === 1 || value === "true" || value === "1"
    }

    function save() {
        if (!enabled || !restored || !presentationReady)
            return
        if (window.visibility === Window.Windowed) {
            saved.setValue("normalWidth", window.width)
            saved.setValue("normalHeight", window.height)
            saved.setValue("normalX", window.x)
            saved.setValue("normalY", window.y)
            saved.setValue("positioned", true)
            saved.setValue("maximized", false)
        } else if (window.visibility === Window.Maximized) {
            saved.setValue("maximized", true)
            // Preserve the actual client rectangle, not a normal-window size
            // or an estimate that includes compositor decorations/panels.
            saved.setValue("maximizedWidth", window.width)
            saved.setValue("maximizedHeight", window.height)
            saved.setValue("maximizedScreen", window.screen.name)
            saved.setValue("maximizedScreenWidth", window.screen.width)
            saved.setValue("maximizedScreenHeight", window.screen.height)
            saved.setValue("maximizedAvailableWidth", window.screen.desktopAvailableWidth)
            saved.setValue("maximizedAvailableHeight", window.screen.desktopAvailableHeight)
        }
        // Minimized/fullscreen windows retain their last desktop geometry.
        saved.sync()
    }

    function restore() {
        if (!enabled)
            return
        const normalWidth = saved.value("normalWidth", 1440)
        const normalHeight = saved.value("normalHeight", 900)
        const normalX = saved.value("normalX", 0)
        const normalY = saved.value("normalY", 0)
        const positioned = savedFlag("positioned")
        let screen = window.screen
        for (const candidate of Qt.application.screens) {
            if (positioned && normalX >= candidate.virtualX
                    && normalX < candidate.virtualX + candidate.width
                    && normalY >= candidate.virtualY
                    && normalY < candidate.virtualY + candidate.height) {
                screen = candidate
                break
            }
        }
        window.width = Math.max(window.minimumWidth, Math.min(normalWidth, screen.width))
        window.height = Math.max(window.minimumHeight, Math.min(normalHeight, screen.height))
        window.screen = screen
        if (positioned) {
            window.x = Math.max(screen.virtualX, Math.min(normalX,
                                    screen.virtualX + screen.width - window.width))
            window.y = Math.max(screen.virtualY, Math.min(normalY,
                                    screen.virtualY + screen.height - window.height))
        }
        restoreMaximized = savedFlag("maximized")
        normalGeometry = Qt.rect(window.x, window.y, window.width, window.height)
        normalPositioned = positioned
        if (restoreMaximized && deferPresentation) {
            const matchingScreen = saved.value("maximizedScreen", "") === screen.name
                    && Number(saved.value("maximizedScreenWidth", 0)) === screen.width
                    && Number(saved.value("maximizedScreenHeight", 0)) === screen.height
                    && Number(saved.value("maximizedAvailableWidth", 0)) === screen.desktopAvailableWidth
                    && Number(saved.value("maximizedAvailableHeight", 0)) === screen.desktopAvailableHeight
            // On a changed/new screen, prewarm the available area. The owner
            // must reanchor its view if the compositor adjusts the client size.
            const width = matchingScreen ? Number(saved.value("maximizedWidth", 0)) : 0
            const height = matchingScreen ? Number(saved.value("maximizedHeight", 0)) : 0
            window.width = Number.isFinite(width) && width > 0 && width <= screen.width
                    ? width : Math.min(screen.width, screen.desktopAvailableWidth)
            window.height = Number.isFinite(height) && height > 0 && height <= screen.height
                    ? height : Math.min(screen.height, screen.desktopAvailableHeight)
            restoreNormalAfterMaximize = true
        }
        if (restoreMaximized && !deferPresentation)
            window.showMaximized()
        restored = true
    }

    function showRestored() {
        if (!restored)
            return
        if (restoreMaximized)
            window.showMaximized()
        else
            window.showNormal()
        presentationReady = true
    }

    Timer {
        id: saveDelay
        interval: 400
        onTriggered: placement.save()
    }
    Connections {
        target: placement.window
        function onWidthChanged() { saveDelay.restart() }
        function onHeightChanged() { saveDelay.restart() }
        function onXChanged() { saveDelay.restart() }
        function onYChanged() { saveDelay.restart() }
        function onVisibilityChanged() {
            if (placement.presentationReady && placement.restoreNormalAfterMaximize
                    && placement.previousVisibility === Window.Maximized
                    && placement.window.visibility === Window.Windowed) {
                placement.restoreNormalAfterMaximize = false
                placement.window.width = placement.normalGeometry.width
                placement.window.height = placement.normalGeometry.height
                if (placement.normalPositioned) {
                    placement.window.x = placement.normalGeometry.x
                    placement.window.y = placement.normalGeometry.y
                }
            }
            placement.previousVisibility = placement.window.visibility
            saveDelay.restart()
        }
        function onClosing(close) { placement.save() }
    }
    Component.onCompleted: Qt.callLater(restore)
}
