import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "ControllerCalibrationWizard"
    when: windowShown
    ApplicationWindow {
        id: host
        width: 950; height: 900; visible: true
        QtObject {
            id: pad
            property string last_device_key: ""
            property string last_binding: ""
            property int input_revision: 0
            property string neutral_device_key: ""
            property string neutral_binding: ""
            property string neutral_error: ""
            property int neutral_revision: 0
        }
        QtObject {
            id: settings
            function controller_catalog_json() {
                return JSON.stringify({host_os:"linux", layouts:[{id:"nes",name:"NES",notes:"",controls:[
                    {id:"b",label:"B",analog:false,optional:false},
                    {id:"a",label:"A",analog:false,optional:false}
                ]},{id:"n30-turbo",name:"N30 turbo",notes:"",controls:[
                    {id:"b",label:"B",analog:false,optional:false},
                    {id:"a",label:"A",analog:false,optional:false},
                    {id:"turbo_b",label:"Turbo B",analog:false,optional:true,repeat_of:"b"}
                ]}],emulator_profiles:[]})
            }
            function controller_key_for_input(key) { return key === "event1" ? "n30" : "other" }
            function controller_calibration_json(key) { return "{}" }
            function controller_diagram(layout, active) { return "" }
            function save_controller_calibration(device, layout, bindings) { return "" }
            function controller_mapping_preview(layout, bindings, profile) { return '{"rows":[],"warnings":[]}' }
        }
        Lunchbox.ControllerCalibrationWizard { id: wizard; settingsModel: settings; gamepad: pad }
    }
    function input(key, code) {
        pad.last_device_key = key
        pad.last_binding = JSON.stringify({code:code,kind:"button",direction:0,logical:"South"})
        pad.input_revision++
    }
    function release(key) {
        pad.neutral_device_key = key
        pad.neutral_revision++
    }
    function init() { pad.neutral_binding = ""; pad.neutral_error = ""; wizard.openFor("n30", "N30"); tryCompare(wizard, "visible", true) }
    function cleanup() { wizard.close() }
    function test_wrong_controller_cannot_calibrate_selected_pad() {
        input("event2", 1)
        compare(Object.keys(wizard.bindings).length, 0)
        verify(wizard.status.indexOf("another controller") >= 0)
    }
    function test_release_required_and_duplicate_inputs_rejected() {
        input("event1", 1)
        compare(wizard.step, 0); verify(wizard.waitingForRelease)
        input("event1", 2)
        compare(Object.keys(wizard.bindings).length, 1)
        release("event2"); compare(wizard.step, 0)
        release("event1"); compare(wizard.step, 1)
        input("event1", 1)
        verify(wizard.status.indexOf("already assigned") >= 0)
        compare(Object.keys(wizard.bindings).length, 1)
        input("event1", 2); release("event1")
        compare(wizard.step, 2)
        compare(wizard.bindings.a.code, 2)
    }
    function test_skip_records_no_fake_binding_and_restart_clears_draft() {
        wizard.skip()
        compare(wizard.step, 1); compare(Object.keys(wizard.bindings).length, 0)
        input("event1", 2); release("event1")
        wizard.resetLayout(0)
        compare(wizard.step, 0); compare(Object.keys(wizard.bindings).length, 0)
    }
    function test_hardware_turbo_is_not_calibrated_as_an_independent_button() {
        wizard.resetLayout(1)
        compare(wizard.layout.controls.length, 3)
        compare(wizard.calibrationControls.length, 2)
        input("event1", 1); release("event1")
        input("event1", 2); release("event1")
        compare(wizard.currentControl, null)
        compare(Object.keys(wizard.bindings).length, 2)
        verify(!wizard.bindings.turbo_b)
    }
    function test_axis_keeps_measured_rest_peak_and_corrected_physical_direction() {
        const raw = {code:196608,kind:"axis",direction:1,logical:"LeftStickRight",
            native:{code:196608,direction:1}}
        pad.last_device_key = "event1"; pad.last_binding = JSON.stringify(raw); pad.input_revision++
        verify(wizard.waitingForRelease)
        const completed = Object.assign({}, raw, {native:{code:196608,direction:-1},
            axis:{minimum:0,maximum:255,flat:0,fuzz:0,resolution:0,released:128,pressed:0}})
        pad.neutral_binding = JSON.stringify(completed)
        release("event1")
        compare(wizard.step, 1)
        compare(wizard.bindings.b.native.direction, -1)
        compare(wizard.bindings.b.axis.released, 128)
        compare(wizard.bindings.b.axis.pressed, 0)
    }
    function test_missing_or_mismatched_axis_measurement_does_not_advance() {
        const raw = {code:196608,kind:"axis",direction:1,logical:"LeftStickRight",
            native:{code:196608,direction:1}}
        pad.last_device_key = "event1"; pad.last_binding = JSON.stringify(raw); pad.input_revision++
        release("event1")
        compare(wizard.step, 0)
        verify(!wizard.waitingForRelease)
        verify(!wizard.bindings.b)
        verify(wizard.status.indexOf("measure") >= 0)
    }
    function test_capture_error_does_not_save_incomplete_input() {
        input("event1", 1)
        pad.neutral_error = "Controller disconnected."
        release("event1")
        compare(wizard.step, 0)
        verify(!wizard.bindings.b)
        compare(wizard.status, "Controller disconnected.")
    }
}
