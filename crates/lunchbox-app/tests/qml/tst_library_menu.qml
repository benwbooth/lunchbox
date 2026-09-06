import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "LibraryMenu"
    when: windowShown
    property int selections: 0
    Window {
        id: testWindow
        visible: true
        width: 500
        height: 360
        Lunchbox.LibraryMenu {
            id: navigation
            x: 13
            y: 13
            width: 230
            Lunchbox.SidebarNavButton {
                id: allGames
                label: "All Games"
                glyph: ""
                iconName: "games"
                onClicked: { testCase.selections++; navigation.close() }
            }
            Repeater {
                model: 10
                Lunchbox.SidebarNavButton {
                    required property int index
                    label: "Action " + index
                    glyph: ""
                    iconName: "firmware"
                }
            }
        }
    }
    function init() {
        navigation.close()
        selections = 0
        testWindow.requestActivate()
        tryCompare(testWindow, "active", true)
    }
    function test_collapsed_and_mouse_selection() {
        compare(navigation.menuVisible, false)
        compare(navigation.height, 43)
        mouseClick(navigation)
        tryCompare(navigation, "menuVisible", true)
        verify(allGames.width > 240)
        mouseClick(allGames)
        compare(selections, 1)
        tryCompare(navigation, "menuVisible", false)
    }
    function test_keyboard_open_select_and_escape() {
        navigation.forceActiveFocus()
        keyClick(Qt.Key_Space)
        tryCompare(navigation, "menuVisible", true)
        tryCompare(allGames, "activeFocus", true)
        keyClick(Qt.Key_Return)
        compare(selections, 1)
        tryCompare(navigation, "menuVisible", false)
        navigation.clicked()
        tryCompare(navigation, "menuVisible", true)
        keyClick(Qt.Key_Escape)
        tryCompare(navigation, "menuVisible", false)
    }
    function test_popup_fits_small_window() {
        navigation.clicked()
        const popup = findChild(navigation, "libraryMenuPopup")
        verify(popup !== null)
        verify(popup.height <= 250)
    }
    function test_arrows_scroll_to_last_action() {
        navigation.clicked()
        tryCompare(allGames, "activeFocus", true)
        keyClick(Qt.Key_Up)
        const last = testWindow.activeFocusItem
        compare(last.label, "Action 9")
        const point = last.mapToItem(testWindow.contentItem, 0, 0)
        verify(point.y >= 0)
        verify(point.y + last.height <= testWindow.height)
        keyClick(Qt.Key_Down)
        tryCompare(allGames, "activeFocus", true)
    }
}
