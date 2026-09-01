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
                index: 0
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

        host.manager.busy = false
        tryCompare(host.emulatorRow, "managerBusy", false)
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
}
