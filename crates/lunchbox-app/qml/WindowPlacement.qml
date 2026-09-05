import QtQuick
import QtCore

Item {
    id: placement
    required property var window
    property string settingsCategory: "WindowPlacement"
    property url settingsLocation: ""
    property bool restored: false

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
        if (!enabled || !restored)
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
        if (positioned) {
            window.x = Math.max(screen.virtualX, Math.min(normalX,
                                    screen.virtualX + screen.width - window.width))
            window.y = Math.max(screen.virtualY, Math.min(normalY,
                                    screen.virtualY + screen.height - window.height))
        }
        if (savedFlag("maximized"))
            window.showMaximized()
        restored = true
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
        function onVisibilityChanged() { saveDelay.restart() }
        function onClosing(close) { placement.save() }
    }
    Component.onCompleted: Qt.callLater(restore)
}
