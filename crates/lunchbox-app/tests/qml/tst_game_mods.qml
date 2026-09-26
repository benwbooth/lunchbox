import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GameMods"
    when: windowShown
    visible: true
    width: 540; height: 1500
    Rectangle { anchors.fill: parent; z: -1; color: "#112d24" }
    QtObject {
        id: mockMods
        property string profile_json: JSON.stringify({patches: [{name: "English translation.bps", format: "BPS", enabled: false}], cheats: [], cheats_enabled: false, base_sha256: ""})
        property bool busy: false
        property string message: ""
        property string selected: ""
        function select_game(game) { selected = game }
    }
    Component {
        id: paneComponent
        Lunchbox.GameModsPane {
            width: 480; backend: mockMods; gameId: "test-game"; retroarch: true
        }
    }
    function test_collapsed_by_default_and_game_change_resets() {
        const pane = createTemporaryObject(paneComponent, testCase)
        verify(pane !== null)
        compare(pane.expanded, false)
        compare(mockMods.selected, "test-game")
        const button = findChild(pane, "modsAccordionButton")
        mouseClick(button)
        compare(pane.expanded, true)
        wait(50)
        verify(pane.height > 300)
        pane.gameId = "next-game"
        compare(pane.expanded, false)
        compare(mockMods.selected, "next-game")
    }
    function test_panel_layout() {
        const pane = createTemporaryObject(paneComponent, testCase)
        pane.expanded = true
        wait(50)
        verify(pane.height > 500)
        verify(pane.height < 1400)
        const picture = grabImage(testCase)
        picture.save("/tmp/lunchbox-game-mods-panel.png")
    }
}
