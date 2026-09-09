import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "SavedControllerReview"
    when: windowShown
    ApplicationWindow {
        width: 950; height: 900; visible: true
        QtObject {
            id: settings
            function controller_catalog_json() {
                return JSON.stringify({host_os:"linux", layouts:[{id:"nes", name:"NES", controls:[
                    {id:"b",label:"B",analog:false}, {id:"a",label:"A",analog:false}
                ]}],emulator_profiles:[]})
            }
            function controller_calibration_json(id) {
                return JSON.stringify({layout:"nes",os:"linux",bindings:{
                    b:{code:1,kind:"button",direction:0,logical:"South"},
                    a:{code:2,kind:"button",direction:0,logical:"East"}
                }})
            }
            function controller_diagram(layout, active) { return "" }
            function validate_controller_capture(layout, control, binding) { return "" }
        }
        QtObject {
            id: pad
            property int input_revision: 0
            property int neutral_revision: 0
            property string last_device_key: "pad"
            property string last_binding: '{"code":9,"kind":"button","direction":0,"logical":"South"}'
        }
        Lunchbox.ControllerCalibrationWizard { id: wizard; settingsModel: settings; gamepad: pad }
    }
    function test_saved_buttons_are_reviewed_without_recording() {
        wizard.openFor("pad", "Saved pad")
        tryCompare(wizard, "visible", true)
        compare(wizard.layout.id, "nes")
        verify(wizard.reviewingSaved)
        const before = JSON.stringify(wizard.bindings)
        pad.input_revision++
        compare(JSON.stringify(wizard.bindings), before)
        verify(!wizard.waitingForRelease)
        wizard.close()
        wizard.openFor("pad", "Saved pad")
        compare(JSON.stringify(wizard.bindings), before)
        wizard.resetLayout(0)
        verify(!wizard.reviewingSaved)
        wizard.close()
    }
}
