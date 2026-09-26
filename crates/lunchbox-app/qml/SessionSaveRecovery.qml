import QtQuick

// Session exit work is persisted by Rust until acknowledged here. Wait for
// credentials and other sync operations; never race an emulator's save writes.
QtObject {
    id: recovery
    required property var backend
    property bool gameBusy: false
    property bool enabled: true
    property var pending: []
    property var seen: ({})
    property var active: null
    signal notice(string title, string message, bool success)
    signal syncStarting(var target)
    signal acknowledge(string token)

    function enqueue(report) {
        if (!enabled || !report.token || seen[report.token]) return
        seen[report.token] = true
        if (report.notice) notice(report.title, report.notice, report.success)
        pending = pending.concat([report])
        Qt.callLater(drain)
    }

    function drain() {
        if (!enabled || gameBusy || active || !backend.initialized
                || backend.busy || pending.length === 0) return
        if (["error", "conflicts", "remote_devices"].indexOf(backend.status) >= 0) return
        const next = pending[0]
        pending = pending.slice(1)
        if (!backend.credentials_saved || !backend.automatic_enabled
                || !next.target || !next.target.available) {
            acknowledge(next.token)
            Qt.callLater(drain)
            return
        }
        active = next
        const target = Object.assign({}, next.target, {title: next.title})
        syncStarting(target)
        backend.begin_sync(target.emulator_slug, target.runtime_platform, "post_exit")
    }

    onGameBusyChanged: Qt.callLater(drain)
    property Connections connection: Connections {
        target: recovery.backend
        function onInitializedChanged() { Qt.callLater(recovery.drain) }
        function onRevisionChanged() {
            if (recovery.active && recovery.backend.operation === "post_exit") {
                const status = recovery.backend.status
                if (status === "complete" || status === "skipped") {
                    recovery.acknowledge(recovery.active.token)
                    recovery.active = null
                } else if (status === "error" || status === "cancelled") {
                    // The normal sync UI reports the error. Keep the persisted
                    // record for next startup, without an automatic retry loop.
                    recovery.active = null
                }
            }
            Qt.callLater(recovery.drain)
        }
    }
}
