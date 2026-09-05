import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "EmulatorManagerRow"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 900
            height: 120

            QtObject {
                id: manager
                property int revision: 1
                property bool busy: true
                property int busy_index: 0
                property string state: "AVAILABLE"

                function name_at(index) { return "Ryubing (Ryujinx fork)" }
                function status_at(index) { return state }
                function source_at(index) { return "Flatpak" }
                function detail_at(index) {
                    return state === "MANAGED"
                        ? "Flatpak · io.github.ryubing.Ryujinx · installed by Lunchbox"
                        : "Flatpak · io.github.ryubing.Ryujinx · ready to install"
                }
                function recommended_at(index) { return true }
                function can_install_at(index) { return state === "AVAILABLE" }
                function can_uninstall_at(index) { return state === "MANAGED" }
                function uninstall_confirmation_at(index) { return "Remove Ryubing" }
            }

            Lunchbox.EmulatorManagerRow {
                id: emulatorRow
                rowIndex: 0
                emulatorModel: manager
                ink: "#f4f7fb"
                muted: "#94a0b3"
                line: "#2b384b"
                accent: "#ffad4a"
                accentCool: "#5de2d2"
            }

            property alias manager: manager
            property alias emulatorRow: emulatorRow
        }
    }

    function test_finished_install_replaces_available_action() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const installButton = findChild(host.emulatorRow, "emulatorInstallButton")
        const uninstallButton = findChild(host.emulatorRow, "emulatorUninstallButton")
        verify(installButton && uninstallButton)
        const progress = findChild(host.emulatorRow, "emulatorInstallProgress")
        verify(progress)
        verify(progress.indeterminate)

        host.manager.busy = false
        tryCompare(host.emulatorRow, "managerBusy", false)
        verify(!progress.indeterminate)
        compare(host.manager.can_install_at(0), true)
        tryCompare(host.emulatorRow, "installAvailable", true)
        tryCompare(host.emulatorRow, "installActionVisible", true)
        compare(host.emulatorRow.emulatorStatus, "AVAILABLE")

        host.manager.state = "MANAGED"
        host.manager.revision += 1

        tryCompare(host.emulatorRow, "emulatorStatus", "MANAGED")
        tryCompare(host.emulatorRow, "installActionVisible", false)
        tryCompare(host.emulatorRow, "uninstallActionVisible", true)
        verify(host.emulatorRow.emulatorDetail.indexOf("installed by Lunchbox") >= 0)
    }

    function test_installed_emulator_offers_scoped_defaults() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const gameButton = findChild(host.emulatorRow, "emulatorGameDefaultButton")
        const systemButton = findChild(host.emulatorRow, "emulatorPlatformDefaultButton")
        verify(gameButton && systemButton)

        compare(host.emulatorRow.gameDefaultActionVisible, false)
        compare(host.emulatorRow.platformDefaultActionVisible, false)
        host.manager.busy = false
        host.emulatorRow.defaultActionsAvailable = true
        host.emulatorRow.gameDefaultActionsAvailable = true
        host.emulatorRow.defaultTargetAvailable = true
        host.emulatorRow.platformName = "Nintendo Switch"
        compare(host.emulatorRow.defaultActionsAvailable, true)
        tryCompare(host.emulatorRow, "gameDefaultActionVisible", true)
        tryCompare(host.emulatorRow, "platformDefaultActionVisible", true)
        compare(gameButton.text, "USE FOR GAME")
        compare(systemButton.text, "USE FOR SYSTEM")

        host.emulatorRow.platformDefault = true
        tryCompare(systemButton, "text", "✓  SYSTEM DEFAULT")
        host.emulatorRow.gameDefault = true
        tryCompare(gameButton, "text", "✓  GAME DEFAULT")
    }

    function test_uninstalled_emulator_keeps_default_path_discoverable() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.manager.busy = false
        host.emulatorRow.defaultActionsAvailable = true
        host.emulatorRow.gameDefaultActionsAvailable = true
        host.emulatorRow.defaultTargetAvailable = false

        compare(host.emulatorRow.height, 116)
        compare(host.emulatorRow.gameDefaultActionVisible, false)
        compare(host.emulatorRow.platformDefaultActionVisible, false)
    }
}
