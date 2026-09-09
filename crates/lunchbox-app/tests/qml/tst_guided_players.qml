import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "GuidedPlayers"
    when: windowShown
    width: 1100; height: 900
    ApplicationWindow {
        visible: true; width: 1100; height: 900
        QtObject {
            id: settings
            property int controller_revision: 0
            property bool busy: false
            property bool controller_busy: false
            property string controller_error: ""
            property string controller_status: ""
            property var ids: ["sc2", "brawler", "unknown", "steam-virtual"]
            property var order: []
            property var calibrations: ({})
            property var models: ({})
            property int automaticCalls: 0
            property string failure: ""
            function controller_count() { return ids.length }
            function controller_key_at(i) { return ids[i] }
            function controller_name_at(i) { return ["Steam Controller 2", "Brawler64", "Same-name pad", "Steam output"][["sc2","brawler","unknown","steam-virtual"].indexOf(ids[i])] }
            function controller_alias_at(i) { return "" }
            function controller_key_for_input(key) { return key }
            function refresh_controllers() {}
            function controller_catalog_json() {
                return JSON.stringify({host_os:"linux", layouts:[{id:"nes",name:"NES",notes:"",family:"two-button",controls:[
                    {id:"b",label:"B",analog:false,x:20,y:30}, {id:"a",label:"A",analog:false,x:40,y:30}
                ]}, {id:"brawler64",name:"Brawler64",notes:"",family:"n64",controls:[{id:"a",label:"A",analog:false,x:40,y:30}]}],
                emulator_profiles:[{id:"nes-target",name:"NES controller",core:"fceumm",transport:"retropad",target_layout:"nes",
                    retroarch_launch:{platforms:["Nintendo Entertainment System"],max_players:4}}]})
            }
            function complete() { return {layout:"nes",os:"linux",bindings:{b:{code:1},a:{code:2}}} }
            function controller_calibration_json(id) { return JSON.stringify(calibrations[id] || {}) }
            function controller_player_order_json() { return JSON.stringify(order) }
            function save_controller_player_order(json) { if (failure) return failure; order=JSON.parse(json); return "" }
            function controller_model_review(id, query) {
                return JSON.stringify({native_sdl3:id === "sc2",steam_virtual:id === "steam-virtual",device_name:id,
                    selected:models[id] || null,detected:id === "sc2" ? {id:"native",name:"Steam Controller 2"} : null,
                    candidates:[{id:"brawler-model",name:"Brawler64",os:"linux",layout:"brawler64"}]})
            }
            function use_sdl3_controller_mapping(id) {
                automaticCalls++
                if (failure) return failure
                const next=Object.assign({},calibrations); next[id]=complete(); calibrations=next; controller_revision++
                return ""
            }
            function save_controller_model(id, model) {
                const next=Object.assign({},models); next[id]={id:model,name:"Brawler64",layout:"brawler64"}; models=next; controller_revision++; return ""
            }
            function guided_controller_preview(id, target, choices) { return '{"rows":[],"error":""}' }
            function controller_diagram(layout, active) { return "" }
            function validate_controller_capture(layout, control, binding) { return "" }
        }
        QtObject {
            id: pad
            property int input_revision: 0
            property int neutral_revision: 0
            property string last_device_key: ""
            property string last_binding: '{"code":9,"kind":"button","direction":0,"logical":"South"}'
        }
        Lunchbox.GuidedControllerSetup { id: workflow; width: 1040; settingsModel: settings; gamepad: pad }
    }
    function init() {
        workflow.dirty=false; workflow.stage=0; workflow.setupResults=({})
        settings.ids=["sc2","brawler","unknown","steam-virtual"]
        settings.order=[]; settings.calibrations={brawler:settings.complete()}; settings.models=({})
        settings.automaticCalls=0; settings.failure=""; settings.controller_revision++
        workflow.startForGame("", "", "")
        wait(1)
    }
    function buttonNamed(item, text) {
        if (item.text === text && typeof item.clicked === "function") return item
        for (const child of item.children || []) { const found=buttonNamed(child,text); if (found) return found }
        return null
    }
    function cleanup() {
        if (workflow.calibrationActive) buttonNamed(workflow.calibrationContentItem.parent,"Cancel").clicked()
        tryCompare(workflow,"calibrationActive",false)
    }
    function test_players_come_before_target_and_unknown_setup() {
        compare(workflow.stage,0)
        verify(!workflow.profile)
        verify(workflow.controllerChoices(0).some(item => item.id === "unknown"))
        workflow.assignPlayer(0,"unknown")
        compare(settings.order.join(","),"unknown")
        verify(!workflow.playersReady)
        workflow.addPlayer()
        compare(workflow.playerDevices.length,2)
        verify(!workflow.controllerChoices(1).some(item => item.id === "unknown"))
        workflow.assignPlayer(1,"brawler")
        compare(settings.order.join(","),"unknown,brawler")
    }
    function test_native_mapping_is_automatic_and_existing_buttons_survive() {
        workflow.assignPlayer(0,"sc2")
        compare(settings.automaticCalls,1)
        verify(workflow.playersReady)
        settings.calibrations.sc2.bindings.a.code=55
        workflow.prepareController("sc2")
        compare(settings.automaticCalls,1)
        compare(settings.calibrations.sc2.bindings.a.code,55)
        workflow.addPlayer(); workflow.assignPlayer(1,"brawler")
        verify(workflow.playersReady)
        compare(settings.calibrations.brawler.bindings.a.code,2)
    }
    function test_saved_players_restore_on_reopen_without_recalibrating() {
        workflow.assignPlayer(0,"brawler"); workflow.addPlayer(); workflow.assignPlayer(1,"sc2")
        workflow.startForGame("Game", "Nintendo Entertainment System", "RetroArch (fceumm)")
        compare(workflow.playerDevices.join(","),"brawler,sc2")
        compare(settings.automaticCalls,1)
        verify(workflow.playersReady)
    }
    function test_duplicate_assignment_and_save_failure_do_not_change_players() {
        workflow.assignPlayer(0,"brawler"); workflow.addPlayer()
        workflow.assignPlayer(1,"brawler")
        compare(workflow.playerDevices[1],"")
        settings.failure="Could not save"
        workflow.assignPlayer(1,"sc2")
        compare(workflow.playerDevices[1],"")
        compare(workflow.status,"Could not save")
        compare(settings.automaticCalls,0)
    }
    function test_disconnect_keeps_player_identity_and_blocks_next() {
        workflow.assignPlayer(0,"brawler")
        settings.ids=["sc2","unknown","steam-virtual"]; settings.controller_revision++
        compare(workflow.playerDevices[0],"brawler")
        verify(!workflow.playersReady)
        verify(workflow.controllerChoices(0).some(item => item.id === "brawler" && item.name.includes("disconnected")))
        compare(settings.order[0],"brawler")
    }
    function test_manual_model_prefills_layout_but_does_not_invent_numbers() {
        workflow.assignPlayer(0,"unknown")
        workflow.modelDevice="unknown"
        workflow.applyModel("brawler-model")
        verify(workflow.calibrationActive)
        compare(findChild(workflow.calibrationContentItem,"physicalLayoutDisplay").text,"Brawler64")
        verify(!settings.calibrations.unknown)
        verify(!workflow.playersReady)
        buttonNamed(workflow.calibrationContentItem.parent,"Cancel").clicked()
    }
    function test_input_identifies_exact_device_and_virtual_is_hidden() {
        verify(!workflow.controllerChoices(0).some(item => item.id === "steam-virtual"))
        pad.last_device_key="unknown"; pad.input_revision++
        compare(workflow.lastPressedDevice,"unknown")
        verify(!!workflow.activeDevices.unknown)
    }
    function test_removing_last_player_is_persisted() {
        workflow.assignPlayer(0,"brawler"); workflow.addPlayer(); workflow.assignPlayer(1,"unknown")
        workflow.removeLastPlayer()
        compare(settings.order.join(","),"brawler")
        compare(workflow.playerDevices.length,1)
        verify(workflow.playersReady)
    }
    function test_player_order_can_be_swapped_without_recalibrating() {
        workflow.assignPlayer(0,"brawler"); workflow.addPlayer(); workflow.assignPlayer(1,"sc2")
        workflow.movePlayerUp(1)
        compare(settings.order.join(","),"sc2,brawler")
        compare(workflow.playerDevices.join(","),"sc2,brawler")
        compare(settings.automaticCalls,1)
    }
    function test_foreign_os_is_not_ready_or_overwritten() {
        settings.calibrations={sc2:{layout:"nes",os:"windows",bindings:{a:{code:99},b:{code:98}}}}
        settings.controller_revision++
        workflow.assignPlayer(0,"sc2")
        verify(!workflow.playersReady)
        compare(settings.automaticCalls,0)
        compare(settings.calibrations.sc2.os,"windows")
    }
}
