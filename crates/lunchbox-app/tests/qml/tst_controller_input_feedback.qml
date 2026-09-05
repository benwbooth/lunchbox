import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "ControllerInputFeedback"
    when: windowShown
    visible: true
    width: 800
    height: 600
    QtObject {
        id: pad
        property int input_revision: 0
        property string last_control: ""
        property string last_device_key: ""
    }
    Lunchbox.ControllerInputFeedback {
        id: first
        width: 650
        gamepad: pad
        controllerName: "N30"
        matchesInput: pad.last_device_key === "event259"
    }
    Lunchbox.ControllerInputFeedback {
        id: second
        y: 250
        width: 650
        gamepad: pad
        controllerName: "Retro Fighters"
        matchesInput: pad.last_device_key === "event264"
    }
    function input(key, button) {
        pad.last_device_key = key
        pad.last_control = button
        pad.input_revision++
    }
    function init() {
        first.visible = true
        second.visible = true
        first.lastControl = ""; second.lastControl = ""
        wait(1250)
    }
    function test_only_matching_controller_and_button_flash() {
        verify(first.visible)
        input("event259", "East")
        verify(first.flashing); verify(!second.flashing)
        compare(first.lastControl, "East"); compare(second.lastControl, "")
        input("event264", "RightStickUp")
        verify(second.flashing)
        compare(first.lastControl, "East"); compare(second.lastControl, "RightStickUp")
    }
    function test_repeated_button_restarts_feedback_and_keeps_last_input() {
        input("event259", "South")
        wait(800)
        input("event259", "South")
        wait(600)
        verify(first.flashing)
        tryCompare(first, "flashing", false, 1500)
        compare(first.lastControl, "South")
    }
    function test_unmatched_and_hidden_devices_do_not_light_up() {
        input("disconnected", "South")
        verify(!first.flashing); verify(!second.flashing)
        first.visible = false
        input("event259", "South")
        verify(!first.flashing)
    }
}
