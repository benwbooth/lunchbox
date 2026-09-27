import QtQuick

// One-shot presentation barrier. Readiness is state-driven; there is no minimum
// splash duration or timeout that exposes an unfinished window.
Item {
    id: gate
    property bool ready: false
    property bool presented: false
    property var prepare: function() { return true }
    property bool commitPending: false
    signal present()

    function check() {
        if (!enabled || !ready || presented || commitPending)
            return
        if (!prepare())
            return
        commitPending = true
        Qt.callLater(function() {
            commitPending = false
            // A queued filter/model/layout change can invalidate readiness.
            if (!gate.enabled || !gate.ready || gate.presented)
                return
            if (!gate.prepare() || !gate.ready || !gate.enabled)
                return
            gate.presented = true
            gate.present()
        })
    }

    onReadyChanged: Qt.callLater(check)
    onEnabledChanged: Qt.callLater(check)
    Component.onCompleted: Qt.callLater(check)
    Timer {
        interval: 16
        repeat: true
        running: gate.enabled && gate.ready && !gate.presented
        onTriggered: gate.check()
    }
}
