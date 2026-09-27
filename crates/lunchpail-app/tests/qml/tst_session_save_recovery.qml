import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    id: testCase
    name: "SessionSaveRecovery"
    QtObject {
        id: backend
        property bool initialized: false
        property bool busy: false
        property bool credentials_saved: true
        property bool automatic_enabled: true
        property string operation: ""
        property string status: ""
        property int revision: 0
        property var calls: []
        function begin_sync(slug, runtime, operation) {
            calls = calls.concat([[slug, runtime, operation]])
            backend.operation = operation
            backend.busy = true
            backend.status = "busy"
            backend.revision++
        }
    }
    property var notices: []
    readonly property var fakeBackend: backend
    property var acknowledgements: []
    property var started: []
    Component {
        id: component
        Lunchpail.SessionSaveRecovery {
            backend: testCase.fakeBackend
            onNotice: (title, message, success) => testCase.notices.push([title, message, success])
            onAcknowledge: token => testCase.acknowledgements.push(token)
            onSyncStarting: target => testCase.started.push(target)
        }
    }
    function report() {
        return {token:"dkc2-session", title:"Donkey Kong Country 2",
                notice:"save state written; saved RAM written", success:true,
                target:{available:true, emulator_slug:"retroarch-core-mesen-s", runtime_platform:"linux-flatpak"}}
    }
    function init() {
        backend.initialized = false
        backend.busy = false
        backend.credentials_saved = true
        backend.automatic_enabled = true
        backend.operation = ""
        backend.status = ""
        backend.calls = []
        notices = []
        acknowledgements = []
        started = []
    }
    function finish(status) {
        backend.busy = false
        backend.status = status
        backend.revision++
    }
    function test_waits_for_startup_and_sync_without_losing_or_repeating_notice() {
        const recovery = createTemporaryObject(component, testCase)
        recovery.enqueue(report())
        recovery.enqueue(report())
        compare(notices.length, 1)
        compare(backend.calls.length, 0)
        backend.busy = true
        backend.initialized = true
        wait(0)
        compare(backend.calls.length, 0)
        finish("ready")
        tryCompare(backend, "busy", true)
        compare(backend.calls.length, 1)
        compare(started[0].title, "Donkey Kong Country 2")
        compare(acknowledgements.length, 0)
        finish("complete")
        compare(acknowledgements.join(""), "dkc2-session")
        recovery.enqueue(report())
        wait(0)
        compare(backend.calls.length, 1)
    }
    function test_does_not_sync_while_a_game_is_running() {
        const recovery = createTemporaryObject(component, testCase)
        backend.initialized = true
        recovery.gameBusy = true
        recovery.enqueue(report())
        wait(0)
        compare(backend.calls.length, 0)
        recovery.gameBusy = false
        tryCompare(backend, "busy", true)
        finish("error")
        compare(acknowledgements.length, 0)
        recovery.enqueue(report())
        wait(0)
        compare(backend.calls.length, 1) // no retry loop; persisted for next startup
    }
    function test_disabled_backup_still_notifies_about_local_saves() {
        const recovery = createTemporaryObject(component, testCase)
        backend.initialized = true
        backend.automatic_enabled = false
        recovery.enqueue(report())
        tryCompare(recovery, "pending", [])
        compare(notices.length, 1)
        compare(backend.calls.length, 0)
        compare(acknowledgements.join(""), "dkc2-session")
    }
    function test_settings_error_does_not_discard_pending_backup() {
        const recovery = createTemporaryObject(component, testCase)
        backend.initialized = true
        backend.credentials_saved = false
        backend.status = "error"
        recovery.enqueue(report())
        wait(0)
        compare(acknowledgements.length, 0)
        compare(recovery.pending.length, 1)
        backend.credentials_saved = true
        finish("idle")
        tryCompare(backend, "busy", true)
        compare(backend.calls.length, 1)
    }
}
