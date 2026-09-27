import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchpail

TestCase {
    id: testCase
    name: "BezelPicker"
    when: windowShown
    width: 1100; height: 850
    ApplicationWindow {
        id: window
        width: 1100; height: 850; visible: true
        QtObject {
            id: backend
            property string game_id: "arcade-game"
            property string display_scope: "game"
            property string display_bezel: "themed"
            property string display_fullscreen: ""
            property string display_inherited_bezel_label: "Inherit → game-specific artwork"
            property string bezel_catalog_json: "[]"
            property string bezel_preview_id: ""
            property string bezel_preview_url: ""
            property string bezel_status: ""
            property bool bezel_busy: false
            property int writes: 0
            property bool failPreview: false
            function load_bezel_choices() {
                bezel_preview_id=""; bezel_preview_url=""; bezel_status=""
                bezel_catalog_json=JSON.stringify([
                    {id:"",label:"Use inherited artwork",source:""},
                    {id:"off",label:"No artwork",source:""},
                    {id:"themed",label:"Bezel Project · game-specific arcade art",source:""},
                    {id:"arcade-duimon-vertical",label:"Duimon · vertical arcade · 21:9",source:"https://github.com/Duimon/Duimon-Mega-Bezel-Potato-21x9"}
                ])
            }
            function preview_bezel(id) {
                bezel_preview_id=id
                bezel_status=failPreview ? "Download failed — try again" : ""
                bezel_preview_url=failPreview ? "" : Qt.resolvedUrl("../fixtures/bezel-preview.svg").toString()
            }
            function set_display_setting(field,value) {
                writes++
                if (field==="bezel") display_bezel=value
                if (field==="fullscreen") display_fullscreen=value
            }
            function import_bezel(path) {}
        }
        Lunchpail.BezelPickerDialog { id: picker; parent: Overlay.overlay; backend: backend }
    }
    function init() {
        backend.game_id="arcade-game"; backend.display_scope="game"; backend.display_bezel="themed"
        backend.display_fullscreen=""; backend.writes=0; backend.failPreview=false
        window.width=1100; window.height=850
        picker.begin()
        tryVerify(function() { return picker.visible })
    }
    function cleanup() { picker.close() }
    function test_preview_and_cancel_do_not_save() {
        picker.selectedId="arcade-duimon-vertical"
        tryCompare(backend,"bezel_preview_id","arcade-duimon-vertical")
        compare(backend.writes,0)
        picker.reject()
        compare(backend.display_bezel,"themed")
    }
    function test_selection_applies_only_after_confirmation() {
        picker.selectedId="arcade-duimon-vertical"
        tryVerify(function() { return picker.ready })
        findChild(picker,"applyBezel").clicked()
        compare(backend.display_bezel,"arcade-duimon-vertical")
        compare(backend.display_fullscreen,"true")
        verify(!picker.visible)
    }
    function test_failed_download_cannot_be_applied() {
        backend.failPreview=true
        picker.selectedId="arcade-duimon-vertical"
        tryCompare(backend,"bezel_preview_id","arcade-duimon-vertical")
        verify(!findChild(picker,"applyBezel").enabled)
        compare(backend.writes,0)
        picker.selectedId="off"
        tryVerify(function() { return picker.ready })
        findChild(picker,"applyBezel").clicked()
        compare(backend.display_bezel,"off")
    }
    function test_changing_games_closes_picker_without_saving() {
        backend.game_id="other-game"
        tryCompare(picker,"visible",false)
        compare(backend.writes,0)
    }
    function test_responsive_layout_and_preview() {
        picker.selectedId="arcade-duimon-vertical"
        tryCompare(backend,"bezel_preview_id","arcade-duimon-vertical")
        wait(200)
        grabImage(window.contentItem).save("/tmp/lunchpail-bezel-picker-wide.png")
        window.width=620; window.height=720
        wait(100)
        verify(picker.width <= 580)
        verify(findChild(picker,"bezelDesigns").height > 100)
        grabImage(window.contentItem).save("/tmp/lunchpail-bezel-picker-narrow.png")
    }
}
