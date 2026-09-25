import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "GuidedPlayers"
    when: windowShown
    width: 1100; height: 900
    ApplicationWindow {
        id: appWindow
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
            property int refreshCalls: 0
            property string failure: ""
            function controller_count() { return ids.length }
            function controller_key_at(i) { return ids[i] }
            function controller_name_at(i) { return ["Steam Controller 2", "Brawler64", "Same-name pad", "Steam output"][["sc2","brawler","unknown","steam-virtual"].indexOf(ids[i])] }
            function controller_alias_at(i) { return "" }
            function controller_key_for_input(key) { return key }
            function refresh_controllers() { refreshCalls++; controller_revision++ }
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
            property var targetProfiles: ({})
            property var guidedMappings: ({})
            property var controllerNames: ({})
            function controller_target_profile(emulator, platform) { return targetProfiles[emulator + "/" + platform] || "" }
            function save_controller_target_profile(emulator, platform, profile) {
                if (failure) return failure
                const next=Object.assign({},targetProfiles); next[emulator + "/" + platform]=profile; targetProfiles=next
                return ""
            }
            function save_controller_name(id, name) {
                if (failure) return failure
                const next=Object.assign({},controllerNames); next[id]=name; controllerNames=next; return ""
            }
            function save_guided_controller_mapping(id, profile, choices, expected) {
                if (failure) return failure
                if (controller_calibration_json(id) !== expected) return "Controller setup changed"
                const next=Object.assign({},guidedMappings); next[id]={profile:profile,choices:JSON.parse(choices)}; guidedMappings=next
                const calibration=JSON.parse(controller_calibration_json(id))
                calibration.target_mappings=Object.assign({},calibration.target_mappings)
                calibration.target_mappings[profile]=JSON.parse(choices)
                const updated=Object.assign({},calibrations); updated[id]=calibration; calibrations=updated
                controller_revision++
                return ""
            }
            function scopedKey(id, profile, level, gameUid, platform, emulator) {
                return [id, profile, level, level === "game" ? gameUid : level === "system" ? platform : emulator].join("/")
            }
            function scoped_guided_mapping_json(id, profile, level, gameUid, platform, emulator) {
                const levels=level === "game" ? ["game","system","core"] : level === "system" ? ["system","core"] : ["core"]
                for (const candidate of levels) {
                    const key=scopedKey(id,profile,candidate,gameUid,platform,emulator)
                    if (guidedMappings[key]) return JSON.stringify({choices:guidedMappings[key],source:candidate})
                }
                return JSON.stringify({choices:(calibrations[id] || {}).target_mappings?.[profile] || {},source:"existing core/system mapping"})
            }
            function save_scoped_guided_mapping(id, profile, choices, expected, level, gameUid, platform, emulator) {
                if (failure) return failure
                if (controller_calibration_json(id) !== expected) return "Controller setup changed"
                const next=Object.assign({},guidedMappings)
                next[scopedKey(id,profile,level,gameUid,platform,emulator)]=JSON.parse(choices)
                guidedMappings=next; controller_revision++
                return ""
            }
            function clear_scoped_guided_mapping(id, profile, level, gameUid, platform, emulator) {
                if (failure) return failure
                const next=Object.assign({},guidedMappings)
                delete next[scopedKey(id,profile,level,gameUid,platform,emulator)]
                guidedMappings=next; controller_revision++
                return ""
            }
            function guided_controller_preview(id, target, choices) {
                if (!id || !target) return '{"rows":[],"error":""}'
                return JSON.stringify({rows:[
                    {physical_id:"b",physical:"B",target_id:"b",target:"B",reason:"Mapped"},
                    {physical_id:"a",physical:"A",target_id:"a",target:"A",reason:"Mapped"}
                ], twins:[], error:""})
            }
            function controller_diagram(layout, active) { return "" }
            function validate_controller_capture(layout, control, binding) { return "" }
        }
        QtObject {
            id: pad
            property int input_revision: 0
            property int sdl3_device_revision: 0
            property int neutral_revision: 0
            property string last_device_key: ""
            property string last_binding: '{"code":9,"kind":"button","direction":0,"logical":"South"}'
        }
        Lunchbox.GuidedControllerSetup { id: workflow; width: 1040; settingsModel: settings; gamepad: pad }
        Lunchbox.ControllerLayoutExplorer { id: explorer; settingsModel: settings; gamepad: pad }
    }
    function init() {
        appWindow.height=900
        workflow.dirty=false; workflow.stage=0; workflow.setupResults=({})
        settings.ids=["sc2","brawler","unknown","steam-virtual"]
        settings.order=[]; settings.calibrations={brawler:settings.complete()}; settings.models=({})
        settings.guidedMappings=({})
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
        if (explorer.visible) explorer.close()
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
    function test_sdl3_hotplug_refreshes_controller_choices() {
        settings.ids = ["brawler"]
        settings.controller_revision++
        verify(!workflow.controllerChoices(0).some(item => item.id === "sc2"))
        const before = settings.refreshCalls
        settings.ids = ["brawler", "sc2"]
        pad.sdl3_device_revision++
        tryCompare(settings, "refreshCalls", before + 1)
        verify(workflow.controllerChoices(0).some(item => item.id === "sc2"))
    }
    function test_player_dropdown_in_modal_explorer_shows_all_connected_controllers() {
        explorer.openForGame("Metroid", "Nintendo Entertainment System", "RetroArch (fceumm)", "metroid-id")
        tryVerify(function() { return explorer.visible })
        const guided = findChild(explorer, "controllerSetupWorkflow")
        verify(guided)
        const combo = findChild(guided, "playerController0")
        verify(combo)
        compare(combo.model.length, 4)
        mouseClick(combo, combo.width - 12, combo.height / 2)
        tryVerify(function() { return combo.popup.visible })
        verify(combo.popup.parent !== combo)
        verify(combo.popup.z > explorer.z)
        compare(combo.popup.contentItem.count, 4)
        compare(combo.textAt(1), "Steam Controller 2")
        compare(combo.textAt(2), "Brawler64")
        tryVerify(function() { return combo.popup.contentItem.itemAtIndex(1) !== null })
        const steamController = combo.popup.contentItem.itemAtIndex(1)
        mouseClick(steamController, steamController.width / 2,
                   steamController.height / 2)
        tryCompare(combo, "currentIndex", 1)
        compare(guided.playerDevices[0], "sc2")
        verify(!combo.popup.visible)
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
    function test_mapping_scope_can_be_game_system_or_core() {
        workflow.startForGame("Metroid", "Nintendo Entertainment System", "RetroArch (fceumm)")
        workflow.stage=1
        const scope=findChild(workflow,"controllerMappingScope")
        verify(scope.visible)
        compare(workflow.mappingScope, "all Nintendo Entertainment System games")
        verify(workflow.saveTarget("nes-target"))
        compare(settings.targetProfiles["RetroArch (fceumm)/Nintendo Entertainment System"],"nes-target")
        workflow.startForGame("Castlevania", "Nintendo Entertainment System", "RetroArch (fceumm)")
        compare(workflow.selectedProfile,"nes-target")
        compare(workflow.mappingScope, "all Nintendo Entertainment System games")
        workflow.startForGame("Metroid", "Nintendo Entertainment System", "RetroArch (fceumm)", "metroid-id")
        compare(workflow.selectedMappingLevel,"game")
        compare(workflow.mappingScope,"Metroid")
        workflow.selectedMappingLevel="core"
        compare(workflow.mappingScope,"all games using RetroArch (fceumm)")
    }
    function test_saved_scope_choices_fall_back_without_leaking_between_games() {
        workflow.startForGame("Metroid", "Nintendo Entertainment System", "RetroArch (fceumm)", "metroid-id")
        workflow.assignPlayer(0,"brawler")
        verify(workflow.saveTarget("nes-target"))
        workflow.stage=2
        workflow.selectedMappingLevel="core"
        verify(!settings.save_scoped_guided_mapping("brawler","nes-target",JSON.stringify({a:"a"}),
            settings.controller_calibration_json("brawler"),"core","metroid-id",workflow.gamePlatform,workflow.gameEmulator))
        workflow.selectedMappingLevel="system"; workflow.loadMapping()
        compare(workflow.mappingSource,"core")
        compare(workflow.choices.a,"a")
        verify(!settings.save_scoped_guided_mapping("brawler","nes-target",JSON.stringify({b:"b"}),
            settings.controller_calibration_json("brawler"),"system","metroid-id",workflow.gamePlatform,workflow.gameEmulator))
        workflow.selectedMappingLevel="game"; workflow.loadMapping()
        compare(workflow.mappingSource,"system")
        compare(workflow.choices.b,"b")
        verify(!settings.save_scoped_guided_mapping("brawler","nes-target",JSON.stringify({a:"b"}),
            settings.controller_calibration_json("brawler"),"game","metroid-id",workflow.gamePlatform,workflow.gameEmulator))
        workflow.loadMapping()
        compare(workflow.mappingSource,"game")
        compare(workflow.choices.a,"b")
        const remove=findChild(workflow,"removeMappingOverride")
        verify(remove.visible)
        remove.clicked()
        compare(workflow.mappingSource,"system")
        compare(workflow.choices.b,"b")
        workflow.startForGame("Castlevania", "Nintendo Entertainment System", "RetroArch (fceumm)", "castlevania-id")
        workflow.assignPlayer(0,"brawler")
        workflow.loadMapping()
        compare(workflow.mappingSource,"system")
        compare(workflow.choices.b,"b")
        verify(workflow.choices.a === undefined)
    }
    function test_save_does_not_oscillate_dialog_scroll_width() {
        explorer.openForGame("Metroid", "Nintendo Entertainment System", "RetroArch (fceumm)", "metroid-id")
        const nested=findChild(explorer,"controllerSetupWorkflow")
        const scroll=findChild(explorer,"controllerSetupScroll")
        verify(nested); verify(scroll)
        nested.assignPlayer(0,"brawler")
        verify(nested.saveTarget("nes-target"))
        nested.stage=2
        tryCompare(nested,"missing",0)
        const save=findChild(explorer,"savePlayerMapping")
        verify(save)
        wait(100)
        const width=scroll.availableWidth
        const spy=createTemporaryObject(widthSpyComponent,workflow,{target:scroll})
        verify(spy)
        spy.clear()
        save.clicked()
        wait(600)
        compare(scroll.availableWidth,width)
        verify(spy.count < 5,"Dialog scroll width kept changing: " + spy.count)
    }
    function test_review_uses_tall_screen_without_exceeding_small_screen() {
        appWindow.height=1600
        explorer.openForGame("Metroid", "Nintendo Entertainment System", "RetroArch (fceumm)", "metroid-id")
        const nested=findChild(explorer,"controllerSetupWorkflow")
        const scroll=findChild(explorer,"controllerSetupScroll")
        verify(nested)
        tryCompare(explorer,"height",900)
        nested.assignPlayer(0,"brawler")
        verify(nested.saveTarget("nes-target"))
        nested.stage=2
        tryCompare(explorer,"height",1400)
        verify(scroll.contentHeight <= scroll.height,
               "Review still needs scrolling: " + scroll.contentHeight + " > " + scroll.height)
        appWindow.height=800
        tryCompare(explorer,"height",760)
    }
    Component {
        id: widthSpyComponent
        SignalSpy { signalName: "availableWidthChanged" }
    }
    function test_save_keeps_preview_and_selected_wire_stable() {
        workflow.startForGame("Metroid", "Nintendo Entertainment System", "RetroArch (fceumm)")
        workflow.assignPlayer(0,"brawler")
        workflow.stage=1
        verify(workflow.saveTarget("nes-target"))
        workflow.stage=2
        const mapping=findChild(workflow,"guidedMappingView")
        const saveButton=findChild(workflow,"savePlayerMapping")
        verify(mapping !== null)
        verify(saveButton !== null)
        compare(workflow.preview.rows.length,2)
        workflow.choices={a:"a"}
        workflow.dirty=true
        workflow.generate()
        mapping.chooseControl(1,"a")
        compare(mapping.selected.target_id,"a")
        const preview=workflow.preview
        saveButton.clicked()
        compare(workflow.status,
            "Player 1 mapping saved for all Nintendo Entertainment System games.")
        compare(workflow.preview,preview)
        compare(mapping.selected.target_id,"a")
        compare(workflow.calibrationBaseline,settings.controller_calibration_json("brawler"))
        verify(!workflow.dirty)
        wait(100)
        compare(workflow.preview,preview)
        compare(mapping.selected.target_id,"a")
        saveButton.clicked()
        compare(workflow.preview,preview)
        compare(mapping.selected.target_id,"a")
        workflow.choices={b:"a"}
        workflow.loadMapping()
        compare(workflow.choices.a,"a")
        verify(workflow.choices.b === undefined)
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
        compare(findChild(workflow.calibrationContentItem,"physicalLayout").displayText,"Brawler64")
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
