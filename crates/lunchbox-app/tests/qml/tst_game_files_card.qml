import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GameFilesCard"
    when: windowShown

    QtObject {
        id: details

        property int local_file_count: 2
        property int selected_local_file: 0
        property url selected_local_directory_url: "file:///roms"
        property bool local_file_preference_configured: false
        property bool selected_local_file_is_preferred: false
        property bool local_file_preference_stale: false
        property string local_file_preference_message: "Automatic selection is active."
        property int local_file_revision: 1
        property bool managed_install_present: false
        property bool managed_install_can_delete: false
        property bool install_management_busy: false
        property string install_management_message: ""
        property bool launch_busy: false
        property bool game_running: false
        property string game_id: "game-one"
        property int preferredIndex: -1

        signal localFileSelected(int index)
        signal defaultSaved()
        signal defaultCleared()

        function local_file_name_at(index) {
            return index === 0 ? "Game (USA).xci" : "Game (Europe).xci"
        }
        function local_file_path_at(index) {
            return index === 0 ? "/roms/Game (USA).xci" : "/roms/Game (Europe).xci"
        }
        function local_file_is_preferred_at(index) {
            return preferredIndex === index
        }
        function select_local_file(index) {
            localFileSelected(index)
        }
        function set_selected_local_file_preferred() {
            defaultSaved()
        }
        function clear_local_file_preference() {
            defaultCleared()
        }
    }

    Window {
        id: testWindow
        visible: true
        width: 520
        height: 520

        Lunchbox.GameFilesCard {
            id: card
            width: 440
            detailsModel: details
            ink: "#f4f7fb"
            muted: "#94a0b3"
            line: "#2b384b"
            accent: "#f1a23b"
            accentCool: "#6fd4c2"
        }
    }

    SignalSpy {
        id: selectSpy
        target: details
        signalName: "localFileSelected"
    }

    SignalSpy {
        id: saveSpy
        target: details
        signalName: "defaultSaved"
    }

    SignalSpy {
        id: clearSpy
        target: details
        signalName: "defaultCleared"
    }

    function init() {
        details.local_file_count = 2
        details.selected_local_file = 0
        details.local_file_preference_configured = false
        details.selected_local_file_is_preferred = false
        details.local_file_preference_stale = false
        details.local_file_preference_message = "Automatic selection is active."
        details.local_file_revision++
        details.preferredIndex = -1
        const list = findChild(card, "gameVersionList")
        if (list)
            list.expanded = false
        selectSpy.clear()
        saveSpy.clear()
        clearSpy.clear()
    }

    function test_all_versions_are_visible_and_clickable_without_a_dropdown() {
        const list = findChild(card, "gameVersionList")
        const second = findChild(card, "gameVersionRow1")
        verify(list)
        compare(list.count, 2)
        verify(second)

        mouseClick(second, second.width / 2, second.height / 2)
        compare(selectSpy.count, 1)
        compare(selectSpy.signalArguments[0][0], 1)
    }

    function test_default_and_automatic_actions_are_explicit() {
        const save = findChild(card, "makeDefaultVersionButton")
        verify(save)
        verify(save.visible)
        mouseClick(save, save.width / 2, save.height / 2)
        compare(saveSpy.count, 1)

        details.local_file_preference_configured = true
        details.selected_local_file_is_preferred = true
        details.preferredIndex = 0
        details.local_file_revision++
        const clear = findChild(card, "automaticVersionButton")
        verify(clear)
        tryVerify(function() { return clear.visible }, 100)
        verify(clear.enabled)
        clear.clicked()
        compare(clearSpy.count, 1)
    }

    function test_long_version_lists_expand_without_trapping_outer_scroll() {
        details.local_file_count = 5
        const list = findChild(card, "gameVersionList")
        const expand = findChild(card, "showAllGameVersionsButton")
        verify(list)
        verify(expand)
        verify(expand.visible)
        verify(!list.interactive)
        const collapsedHeight = list.height

        expand.clicked()
        tryVerify(function() { return list.height > collapsedHeight }, 100)
        compare(expand.text, "SHOW FEWER VERSIONS")
    }

    function test_saved_but_missing_default_is_actionable() {
        details.local_file_preference_configured = true
        details.local_file_preference_stale = true
        details.local_file_preference_message = "Saved default Game.xci is unavailable."
        compare(card.border.color.toString(), details.local_file_preference_stale
                ? "#f1a23b" : "#2b384b")
        const clear = findChild(card, "automaticVersionButton")
        verify(clear.visible)
        verify(clear.enabled)
    }
}
