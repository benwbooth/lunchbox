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
                property string firmware_summary: "prod.keys and Switch firmware are required."
                property string firmware_next_package: "prod.keys"
                property string firmware_setup_action: "choose"
                property string firmware_setup_label: "SET UP EMULATOR"
                property bool switch_prod_keys_ready: false
                property bool switch_title_keys_ready: false
                property bool switch_firmware_ready: false
                property string launch_status: "Next: CHOOSE prod.keys."
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
        const action = findChild(host.page, "firmwarePackageAction-prod.keys")
        verify(packageLabel)
        verify(action)
        compare(packageLabel.text, "prod.keys")
        compare(action.text, "CHOOSE FILE")
        action.click()
        compare(host.chooseSpy.count, 1)
        compare(host.chooseSpy.signalArguments[0][0], "prod.keys")
    }

    function test_switch_setup_has_independent_optional_title_keys_selector() {
        const host = createTemporaryObject(pageComponent, testCase)
        verify(host)
        host.details.switch_prod_keys_ready = true
        wait(0)
        const prodAction = findChild(host.page, "firmwarePackageAction-prod.keys")
        const titleAction = findChild(host.page, "firmwarePackageAction-title.keys")
        const firmwareAction = findChild(host.page,
                                         "firmwarePackageAction-switch-firmware.zip")
        verify(prodAction)
        verify(titleAction)
        verify(firmwareAction)
        compare(prodAction.selectorAvailable, false)
        compare(titleAction.selectorAvailable, true)
        compare(firmwareAction.selectorAvailable, true)
        titleAction.click()
        compare(host.chooseSpy.count, 1)
        compare(host.chooseSpy.signalArguments[0][0], "title.keys")
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
