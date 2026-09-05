import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "DesktopGamepadNavigation"
    when: windowShown
    width: 800
    height: 600
    property int opened: 0
    property int backs: 0
    property int clicked: 0
    property int activated: -1

    Item { id: contents; anchors.fill: parent }
    QtObject {
        id: host
        property bool active: true
        property var contentItem: contents
        property var activeFocusItem: null
    }
    QtObject {
        id: gamepad
        property int connected_count: 1
        property int navigation_revision: 0
        property string navigation_action: ""
    }
    Item {
        id: grid
        parent: contents
        property int count: 20
        property int currentIndex: 0
        property int columnCount: 4
        property var currentItem: card
        function focusIndex(index, mode) { currentIndex = Math.max(0, Math.min(count - 1, index)) }
        Item { id: card; width: 50; height: 60 }
    }
    Button { id: button; parent: contents; text: "Play"; onClicked: testCase.clicked++ }
    CheckBox { id: checkbox; parent: contents }
    ComboBox {
        id: combo
        parent: contents
        model: ["One", "Two", "Three"]
        onActivated: index => testCase.activated = index
    }
    Lunchbox.DesktopGamepadNavigation {
        id: router
        applicationWindow: host
        gamepad: gamepad
        libraryView: grid
        onOpenGame: item => testCase.opened++
        onBackRequested: testCase.backs++
    }
    function init() {
        router.enabled = true
        host.active = true
        host.activeFocusItem = card
        router.openCombo = null
        grid.currentIndex = 0
        testCase.opened = 0; testCase.backs = 0; testCase.clicked = 0; testCase.activated = -1
        checkbox.checked = false
        combo.currentIndex = 0
    }
    function cleanup() { combo.popup.close() }
    function test_grid_navigation_and_open() {
        verify(router.handle("right")); compare(grid.currentIndex, 1)
        router.handle("down"); compare(grid.currentIndex, 5)
        router.handle("accept"); compare(testCase.opened, 1)
    }
    function test_inactive_window_and_disabled_router_ignore_input() {
        host.active = false
        verify(!router.handle("accept")); compare(testCase.opened, 0)
        host.active = true; router.enabled = false
        verify(!router.handle("right")); compare(grid.currentIndex, 0)
    }
    function test_buttons_and_checkboxes_activate() {
        host.activeFocusItem = button
        router.handle("accept"); compare(testCase.clicked, 1)
        host.activeFocusItem = checkbox
        router.handle("accept"); compare(checkbox.checked, true)
        router.handle("accept"); compare(checkbox.checked, false)
    }
    function test_combo_navigation_commits_and_closes() {
        host.activeFocusItem = combo
        router.handle("accept"); tryCompare(combo.popup, "visible", true)
        router.handle("down"); router.handle("down")
        router.handle("accept")
        compare(testCase.activated, 2); compare(combo.popup.visible, false)
    }
    function test_back_closes_combo_before_leaving_page() {
        host.activeFocusItem = combo
        router.handle("accept"); router.handle("back")
        compare(combo.popup.visible, false); compare(testCase.backs, 0)
        router.handle("back"); compare(testCase.backs, 1)
    }
}
