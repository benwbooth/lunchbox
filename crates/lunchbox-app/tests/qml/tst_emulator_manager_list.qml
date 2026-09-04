import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "EmulatorManagerList"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 900
            height: 500

            QtObject {
                id: manager
                property int revision: 1
                property bool busy: false
                property int busy_index: -1

                function name_at(index) { return "Emulator " + index }
                function status_at(index) { return "AVAILABLE" }
                function source_at(index) { return "Flatpak" }
                function detail_at(index) { return "Ready to install" }
                function recommended_at(index) { return index === 0 }
                function can_install_at(index) { return true }
                function can_uninstall_at(index) { return false }
                function uninstall_confirmation_at(index) { return "" }
            }

            ListView {
                id: emulatorList
                anchors.fill: parent
                model: 4
                spacing: 7

                delegate: Lunchbox.EmulatorManagerRow {
                    required property int index
                    rowIndex: index
                    emulatorModel: manager
                    ink: "#f4f7fb"
                    muted: "#94a0b3"
                    line: "#2b384b"
                    accent: "#ffad4a"
                    accentCool: "#5de2d2"
                }
            }

            property alias emulatorList: emulatorList
        }
    }

    function test_integer_model_renders_emulator_rows() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        compare(host.emulatorList.count, 4)
        tryVerify(function() {
            return host.emulatorList.itemAtIndex(0) !== null
        })
        compare(host.emulatorList.itemAtIndex(0).emulatorName, "Emulator 0")
    }
}
