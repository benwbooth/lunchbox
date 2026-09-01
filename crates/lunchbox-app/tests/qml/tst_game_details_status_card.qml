import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GameDetailsStatusCard"
    when: windowShown

    Window {
        id: testWindow
        visible: true
        width: 520
        height: 260

        Lunchbox.GameDetailsStatusCard {
            id: status
            width: 440
            busy: false
            message: "No matching download source is registered for this game."
            importTorrentAvailable: true
            ink: "#f4f7fb"
            muted: "#94a0b3"
            panel: "#151d29"
            line: "#2b384b"
            accent: "#f1a23b"
        }
    }

    SignalSpy {
        id: importSpy
        target: status
        signalName: "importTorrentRequested"
    }

    function init() {
        status.busy = false
        status.message = "No matching download source is registered for this game."
        status.importTorrentAvailable = true
        importSpy.clear()
    }

    function test_empty_download_state_has_immediate_import_action() {
        const button = findChild(status, "importTorrentForGameButton")
        verify(button)
        verify(status.visible)
        verify(button.visible)
        verify(button.enabled)
        compare(button.text, "IMPORT TORRENT FOR THIS GAME")

        mouseClick(button, button.width / 2, button.height / 2)
        compare(importSpy.count, 1)
    }

    function test_ordinary_status_remains_compact_without_duplicate_action() {
        const actionableHeight = status.implicitHeight
        status.importTorrentAvailable = false
        status.message = "Installed in your collection"
        const button = findChild(status, "importTorrentForGameButton")
        verify(button)
        verify(!button.visible)
        tryVerify(function() {
            return status.implicitHeight < actionableHeight
        }, 100)
    }
}
