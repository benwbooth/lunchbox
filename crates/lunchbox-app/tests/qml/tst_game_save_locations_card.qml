import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GameSaveLocationsCard"
    when: windowShown

    Window {
        visible: true
        width: 540
        height: 780

        Lunchbox.GameSaveLocationsCard {
            id: card
            width: 480
            ink: "#f4f7fb"
            muted: "#94a0b3"
            panel: "#171f2b"
            line: "#2b384b"
        }
    }

    SignalSpy {
        id: openSpy
        target: card
        signalName: "openFolderRequested"
    }

    function init() {
        card.width = 480
        card.target = { state_locations: [{
            path: "/home/player/.local/share/lunchbox/retroarch-launch/mame/states",
            url: "file:///home/player/.local/share/lunchbox/retroarch-launch/mame/states",
            exists: true
        }] }
        card.backup = {
            configured: true, local: true, automatic: true, exists: true,
            path: "/home/player/Insync/player@example.com/Google Drive/lunchbox/saves/v1/retroarch-core-mame/linux-flatpak/current",
            url: "file:///home/player/Insync/player@example.com/Google%20Drive/lunchbox/saves/v1/retroarch-core-mame/linux-flatpak/current"
        }
        openSpy.clear()
    }

    function test_paths_are_selectable_and_open_the_correct_folder() {
        const path = findChild(card, "saveStateLocation0Path")
        verify(path)
        verify(path.readOnly && path.selectByMouse)
        compare(path.text, card.target.state_locations[0].path)
        waitForRendering(card)
        grabImage(card).save("/tmp/lunchbox-save-locations.png")
        const button = findChild(card, "saveBackupLocationOpenFolder")
        verify(button.enabled)
        mouseClick(button)
        compare(openSpy.count, 1)
        compare(openSpy.signalArguments[0][0].toString(), decodeURI(card.backup.url))
    }

    function test_long_paths_wrap_in_narrow_pane() {
        const path = findChild(card, "saveBackupLocationPath")
        const wideHeight = path.height
        card.width = 280
        tryVerify(function() { return path.height > wideHeight })
        verify(path.contentWidth <= path.width + 1)
        compare(path.text, card.backup.path)
        verify(card.implicitHeight > path.height)
        waitForRendering(card)
        grabImage(card).save("/tmp/lunchbox-save-locations-narrow.png")
    }

    function test_missing_folders_do_not_offer_broken_open_action() {
        card.target = { state_locations: [{ path: "/not-created/states", url: "file:///not-created/states", exists: false }] }
        card.backup = { configured: true, local: true, automatic: false, path: "/not-created/backup", url: "file:///not-created/backup", exists: false }
        verify(!findChild(card, "saveStateLocation0OpenFolder").enabled)
        verify(!findChild(card, "saveBackupLocationOpenFolder").enabled)
    }

    function test_cloud_backup_has_no_fake_local_folder_link() {
        card.backup = { configured: true, local: false, provider: "Dropbox", path: "/Lunchbox Save Sync/saves/v1/duckstation/linux", automatic: true }
        verify(!findChild(card, "saveBackupLocationOpenFolder").visible)
        compare(findChild(card, "saveBackupLocationPath").text, card.backup.path)
    }

    function test_switching_to_unsupported_emulator_clears_old_paths() {
        card.target = { available: false }
        card.backup = { configured: false }
        tryVerify(function() { return findChild(card, "saveStateLocation0Path") === null })
        verify(findChild(card, "saveLocationUnavailable").visible)
        verify(findChild(card, "saveBackupUnavailable").visible)
        verify(!findChild(card, "saveBackupLocation").visible)
    }
}
