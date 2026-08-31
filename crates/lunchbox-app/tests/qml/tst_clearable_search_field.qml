import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "ClearableSearchField"
    when: windowShown

    property int clearCount: 0

    Window {
        id: testWindow
        visible: true
        width: 420
        height: 120

        Lunchbox.ClearableSearchField {
            id: search
            anchors.centerIn: parent
            width: 360
            placeholderText: "Search games"
            searchIconVisible: true
            onClearRequested: testCase.clearCount += 1
        }
    }

    function init() {
        clearCount = 0
        search.text = "Faxanadu"
        search.forceActiveFocus()
    }

    function test_clear_button_clears_and_notifies_filter_owner() {
        const clearButton = findChild(search, "clearSearchButton")
        const searchIcon = findChild(search, "searchIcon")
        verify(clearButton !== null)
        verify(searchIcon !== null)
        verify(searchIcon.visible)
        verify(clearButton.visible)
        mouseClick(clearButton, clearButton.width / 2, clearButton.height / 2)
        compare(search.text, "")
        compare(clearCount, 1)
        verify(!clearButton.visible)
        verify(!clearButton.enabled)
    }

    function test_search_text_uses_native_hinted_rendering() {
        compare(search.renderType, Text.NativeRendering)
        compare(search.font.hintingPreference, Font.PreferVerticalHinting)
        verify(search.font.kerning)
    }
}
