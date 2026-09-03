import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "FirmwareSetupPage"
    when: windowShown

    Component {
        id: pageComponent

        Item {
            width: 1100
            height: 760

            QtObject {
                id: detailsState
                property string platform: "Nintendo Switch"
                property string title: "Super Mario Odyssey"
                property string emulator_name: "Ryubing"
                property int firmware_missing_count: 2
                property bool can_launch: false
                property bool firmware_busy: false
                property int firmware_progress: -1
                property string firmware_summary: "Switch keys and firmware are required."
                property string firmware_next_package: "switch-keys.zip"
                property string firmware_setup_action: "choose"
                property string firmware_setup_label: "SET UP EMULATOR"
                property bool switch_prod_keys_ready: false
                property bool switch_firmware_ready: false
                property string launch_status: "Next: CHOOSE switch-keys.zip."
            }

            Lunchbox.FirmwareSetupPage {
                id: setupPage
                anchors.fill: parent
                detailsModel: detailsState
                ink: "#f4f7fb"
                muted: "#8d99aa"
                panel: "#131923"
                panelRaised: "#1a2230"
                line: "#283244"
                accent: "#ffb454"
                accentCool: "#62d6c6"
            }

            SignalSpy {
                id: chooseSpy
                target: setupPage
                signalName: "choosePackageRequested"
            }

            property alias details: detailsState
            property alias page: setupPage
            property alias chooseSpy: chooseSpy
        }
    }

    function test_switch_setup_names_the_exact_next_file() {
        const host = createTemporaryObject(pageComponent, testCase)
        verify(host)
        const packageLabel = findChild(host.page, "firmwareNextPackage")
        const action = findChild(host.page, "firmwarePackageAction-switch-keys.zip")
        verify(packageLabel)
        verify(action)
        compare(packageLabel.text, "switch-keys.zip")
        compare(action.text, "CHOOSE FILE")
        action.click()
        compare(host.chooseSpy.count, 1)
        compare(host.chooseSpy.signalArguments[0][0], "switch-keys.zip")
    }

    function test_switch_setup_uses_one_keys_selector_for_prod_and_title_keys() {
        const host = createTemporaryObject(pageComponent, testCase)
        verify(host)
        host.details.switch_prod_keys_ready = true
        wait(0)
        const keysAction = findChild(host.page,
                                     "firmwarePackageAction-switch-keys.zip")
        const firmwareAction = findChild(host.page,
                                         "firmwarePackageAction-switch-firmware.zip")
        verify(keysAction)
        verify(firmwareAction)
        compare(keysAction.selectorAvailable, false)
        compare(firmwareAction.selectorAvailable, true)
        compare(findChild(host.page, "firmwarePackageAction-title.keys"), null)
    }

    function test_switch_setup_shows_import_failures() {
        const host = createTemporaryObject(pageComponent, testCase)
        verify(host)
        host.details.launch_status = "Could not import or sync firmware: invalid keys"
        wait(0)
        const status = findChild(host.page, "firmwareOperationStatus")
        verify(status)
        verify(status.hasStatus)
    }

    function test_long_firmware_work_shows_determinate_progress() {
        const host = createTemporaryObject(pageComponent, testCase)
        verify(host)
        host.details.firmware_busy = true
        host.details.firmware_progress = 42
        host.details.launch_status = "Validating firmware package · 100/238 entries"
        wait(0)
        const row = findChild(host.page, "firmwareProgressRow")
        const bar = findChild(host.page, "firmwareProgressBar")
        verify(row)
        compare(row.operationActive, true)
        verify(bar)
        compare(bar.indeterminate, false)
        compare(bar.value, 42)

        host.details.firmware_progress = -1
        wait(0)
        compare(bar.indeterminate, true)
    }

    function test_completed_setup_turns_the_primary_action_into_play() {
        const host = createTemporaryObject(pageComponent, testCase)
        verify(host)
        host.details.firmware_missing_count = 0
        host.details.can_launch = true
        host.details.switch_prod_keys_ready = true
        host.details.switch_firmware_ready = true
        const action = findChild(host.page, "firmwarePrimaryAction")
        verify(action)
        compare(action.text, "▶  PLAY")
    }
}
