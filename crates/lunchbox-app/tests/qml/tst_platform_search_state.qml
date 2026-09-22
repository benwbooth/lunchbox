import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "PlatformSearchState"

    Lunchbox.PlatformSearchState { id: state }

    function test_platforms_keep_independent_searches() {
        compare(state.initialize("", "", ""), "")
        state.update("metroid")
        compare(state.switchTo("Nintendo Entertainment System", "metroid"), "")
        state.update("mario")
        compare(state.switchTo("Super Nintendo Entertainment System", "mario"), "")
        state.update("zelda")
        compare(state.switchTo("Nintendo Entertainment System", "zelda"), "mario")
        compare(state.switchTo("", "mario"), "metroid")
        compare(state.switchTo("Super Nintendo Entertainment System", "metroid"), "zelda")
    }

    function test_searches_survive_serialization_and_clearing() {
        state.initialize("", "Nintendo Entertainment System", "")
        state.update("mario")
        state.switchTo("Super Nintendo Entertainment System", "mario")
        state.update("zelda")
        state.switchTo("Nintendo Entertainment System", "zelda")
        state.update("")
        const saved = state.serialized()
        compare(state.initialize(saved, "Super Nintendo Entertainment System", "stale"), "zelda")
        compare(state.switchTo("Nintendo Entertainment System", "zelda"), "")
    }

    function test_legacy_global_search_moves_to_restored_platform() {
        compare(state.initialize("", "Nintendo Entertainment System", "Faxanadu"),
                "Faxanadu")
        compare(state.switchTo("Super Nintendo Entertainment System", "Faxanadu"), "")
        compare(state.switchTo("Nintendo Entertainment System", ""), "Faxanadu")
    }

    function test_invalid_saved_map_does_not_break_restore() {
        compare(state.initialize("{broken", "Nintendo Entertainment System", "Faxanadu"),
                "Faxanadu")
        compare(state.initialize('{"Nintendo Entertainment System":42}',
                                 "Nintendo Entertainment System", ""), "")
    }
}
