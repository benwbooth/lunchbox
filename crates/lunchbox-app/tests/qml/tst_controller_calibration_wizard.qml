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
            property bool last_capture_started: true
            property string neutral_device_key: ""
            property string neutral_binding: ""
            property string neutral_error: ""
            property int neutral_revision: 0
        }
        QtObject {
            id: settings
            property string savedCalibration: "{}"
            property string captureError: ""
            function validate_controller_capture(layout, control, binding) {
                const input = JSON.parse(binding)
                return captureError || (typeof input.code !== "number" ? "Missing input code" : "")
            }
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
            function controller_count() { return 2 }
            function controller_key_at(index) { return index === 0 ? "n30" : "other" }
            function controller_name_at(index) { return index === 0 ? "N30" : "Brawler64" }
            function controller_calibration_json(key) { return savedCalibration }
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
        // The input backend sends a completed binding on release, even for a
        // digital button. Axis tests supply their measured completion explicitly.
        const recorded = wizard.bindings[wizard.pendingControlId]
        if (!pad.neutral_binding && recorded && recorded.kind === "button")
            pad.neutral_binding = JSON.stringify(recorded)
        pad.neutral_device_key = key
        pad.neutral_revision++
        pad.neutral_binding = ""
    }
    function init() {
        pad.neutral_binding = ""; pad.neutral_error = ""; settings.captureError = ""
        wizard.openFor("n30", "N30"); tryCompare(wizard, "visible", true)
        // Recording tests begin after the user explicitly chooses a layout.
        compare(wizard.layoutIndex, -1)
        wizard.resetLayout(0)
    }
    function cleanup() { wizard.close(); settings.savedCalibration = "{}" }
    function test_saved_setup_opens_paused_and_preserves_buttons() {
        const saved = {layout:"nes", os:"linux", bindings:{
            b:{code:1,kind:"button",direction:0,logical:"South"},
            a:{code:2,kind:"button",direction:0,logical:"East"}
        }}
        settings.savedCalibration = JSON.stringify(saved)
        wizard.close()
        wizard.openFor("n30", "N30")
        compare(wizard.layout.id, "nes")
        verify(wizard.reviewingSaved)
        verify(wizard.targetedComplete)
        input("event1", 9)
        compare(JSON.stringify(wizard.bindings), JSON.stringify(saved.bindings))
        verify(!wizard.waitingForRelease)
        wizard.focusSavedControl("nes", "b")
        verify(!wizard.reviewingSaved)
        verify(!wizard.targetedComplete)
        compare(wizard.bindings.a.code, 2)
    }
    function test_layout_display_uses_native_label_for_placeholder_and_selection() {
        wizard.guided = true
        wizard.layoutIndex = -1
        const label = findChild(wizard.contentItem, "physicalLayoutDisplay")
        verify(label !== null)
        compare(label.text, "Select a physical layout")
        const glyphs = findChild(label, "pixelAlignedGlyphs")
        verify(glyphs !== null)
        compare(glyphs.renderType, Text.NativeRendering)
        compare(glyphs.textFormat, Text.PlainText)
        compare(label.elide, Text.ElideRight)
        wizard.resetLayout(0)
        compare(label.text, "NES")
        wizard.guided = false
    }
    function test_wrong_controller_cannot_calibrate_selected_pad() {
        wizard.resetLayout(0)
        input("event2", 1)
        compare(Object.keys(wizard.bindings).length, 0)
        verify(wizard.status.indexOf("Brawler64") >= 0)
        verify(wizard.status.indexOf("recording N30") >= 0)
    }
    function test_selected_controller_press_starts_recording() {
        wizard.resetLayout(0)
        input("event1", 1)
        verify(wizard.waitingForRelease)
        compare(wizard.bindings.b.code, 1)
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
        verify(wizard.status.startsWith("Controller disconnected."))
    }
    function test_backend_rejects_completed_recording_without_advancing() {
        input("event1", 1)
        settings.captureError = "Incompatible control"
        release("event1")
        compare(wizard.step, 0)
        verify(!wizard.bindings.b)
        verify(wizard.status.includes("Incompatible control"))
    }
}
