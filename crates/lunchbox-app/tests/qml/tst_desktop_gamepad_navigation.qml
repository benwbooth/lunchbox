import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "DesktopGamepadNavigation"
    when: windowShown
    visible: true
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
    GridView {
        id: grid
        parent: contents
        x: 250; y: 80; width: 400; height: 240
        cellWidth: 100; cellHeight: 80
        clip: true
        model: 100
        property int columnCount: 4
        delegate: Item {
            required property int index
            readonly property bool providesFocusIndicator: true
            width: 90; height: 70
            activeFocusOnTab: true
            onActiveFocusChanged: if (activeFocus) host.activeFocusItem = this
        }
        Button {
            id: indexedOverlay
            // Like AlphabetRail, this is view-owned chrome, not a delegate.
            parent: grid
            visible: false
            property int index: 2
            x: 360; y: 215; width: 30; height: 25
            text: "C"
            onActiveFocusChanged: if (activeFocus) host.activeFocusItem = this
            onClicked: testCase.clicked++
        }
    }
    ListView {
        id: platforms
        parent: contents
        x: 10; y: 80; width: 200; height: 240
        clip: true
        model: 50
        delegate: Lunchbox.SidebarNavButton {
            required property int index
            height: 40
            label: "Platform " + index; glyph: ""
            onActiveFocusChanged: if (activeFocus) host.activeFocusItem = this
        }
    }
    Button { id: button; parent: contents; y: 400; text: "Play"; onClicked: testCase.clicked++ }
    CheckBox { id: checkbox; parent: contents; x: 120; y: 400 }
    Lunchbox.ClearableSearchField {
        id: searchField
        parent: contents
        x: 10; y: 500; width: 200; height: 40
    }
    ComboBox {
        id: combo
        parent: contents
        x: 240; y: 400
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
        router.openCombo = null
        grid.currentIndex = 0
        grid.positionViewAtBeginning(); grid.forceLayout()
        platforms.currentIndex = 0
        platforms.positionViewAtBeginning(); platforms.forceLayout()
        wait(0)
        host.activeFocusItem = grid.itemAtIndex(0)
        host.activeFocusItem.forceActiveFocus()
        testCase.opened = 0; testCase.backs = 0; testCase.clicked = 0; testCase.activated = -1
        checkbox.checked = false
        combo.currentIndex = 0
    }
    function cleanup() { combo.popup.close(); indexedOverlay.visible = false }
    function test_grid_navigation_and_open() {
        verify(router.handle("right")); compare(grid.currentIndex, 1)
        router.handle("down"); compare(grid.currentIndex, 5)
        router.handle("accept"); compare(testCase.opened, 1)
    }
    function test_existing_focus_borders_suppress_fallback_outline() {
        host.activeFocusItem = grid.itemAtIndex(0)
        const ring = findChild(host.activeFocusItem, "desktopGamepadFocusRing")
        verify(ring !== null)
        verify(!ring.visible)
        host.activeFocusItem = searchField
        compare(ring.parent, searchField)
        verify(!ring.visible)
        host.activeFocusItem = button
        verify(ring.visible)
        compare(ring.parent, button)
        compare(ring.x, 0); compare(ring.y, 0)
        compare(ring.width, button.width); compare(ring.height, button.height)
        const originalX = button.x
        button.x += 50
        compare(ring.x, 0)
        compare(ring.mapToItem(contents, 0, 0).x, button.x)
        button.x = originalX
    }
    function test_fallback_outline_survives_focused_delegate_removal() {
        host.activeFocusItem = grid.itemAtIndex(0)
        grid.model = 0
        grid.forceLayout()
        wait(0)
        host.activeFocusItem = button
        const ring = findChild(button, "desktopGamepadFocusRing")
        verify(ring !== null)
        verify(ring.visible)
        grid.model = 100
        grid.forceLayout()
        wait(0)
    }
    function test_left_crosses_to_platform_not_previous_grid_row() {
        router.focusViewIndex(grid, 4)
        router.handle("left")
        verify(router.within(host.activeFocusItem, platforms))
        compare(platforms.currentIndex, 2)
        router.handle("right")
        compare(host.activeFocusItem, grid.itemAtIndex(4))
    }
    function test_right_edge_does_not_wrap() {
        router.focusViewIndex(grid, 3)
        router.handle("right")
        compare(host.activeFocusItem, grid.itemAtIndex(3))
    }
    function test_view_container_focus_starts_at_current_card() {
        host.activeFocusItem = grid
        router.handle("right")
        compare(grid.currentIndex, 1)
    }
    function test_unfocused_window_navigation_starts_at_selected_card() {
        host.activeFocusItem = contents
        router.handle("right")
        compare(grid.currentIndex, 1)
    }
    function test_offscreen_cached_delegates_are_excluded() {
        grid.cacheBuffer = 1000
        grid.forceLayout()
        wait(0)
        tryVerify(() => grid.itemAtIndex(20) !== null)
        compare(router.visibleRect(grid.itemAtIndex(20)), null)
        verify(router.candidates(null).indexOf(grid.itemAtIndex(20)) < 0)
        grid.cacheBuffer = 320
    }
    function test_virtual_grid_scrolls_to_next_row() {
        router.focusViewIndex(grid, 8)
        router.handle("down")
        compare(grid.currentIndex, 12)
        verify(router.visibleRect(host.activeFocusItem) !== null)
    }
    function test_indexed_overlay_focus_and_activation_preserve_game_index() {
        indexedOverlay.visible = true
        router.focusViewIndex(grid, 9)
        router.focusTarget(indexedOverlay)
        compare(grid.currentIndex, 9)
        compare(router.owningView(indexedOverlay), null)
        router.handle("accept")
        compare(testCase.clicked, 1)
        compare(testCase.opened, 0)
        compare(grid.currentIndex, 9)
    }
    function test_overlay_does_not_block_virtual_next_row() {
        indexedOverlay.visible = true
        router.focusViewIndex(grid, 11)
        router.handle("down")
        compare(grid.currentIndex, 15)
        compare(host.activeFocusItem, grid.itemAtIndex(15))
    }
    function test_overlay_paging_uses_game_index_not_letter_index() {
        indexedOverlay.visible = true
        router.focusViewIndex(grid, 9)
        router.focusTarget(indexedOverlay)
        router.handle("scroll_last")
        compare(grid.currentIndex, 99)
    }
    function test_page_and_extremes_stay_in_focused_pane() {
        router.handle("page_right"); compare(grid.currentIndex, 12)
        router.handle("page_left"); compare(grid.currentIndex, 0)
        router.handle("scroll_last"); compare(grid.currentIndex, 99)
        verify(router.visibleRect(host.activeFocusItem) !== null)
        router.handle("scroll_first"); compare(grid.currentIndex, 0)
        router.focusViewIndex(platforms, 0)
        router.handle("page_right"); compare(platforms.currentIndex, 6)
        router.handle("scroll_last"); compare(platforms.currentIndex, 49)
        compare(grid.currentIndex, 0)
        router.handle("scroll_first"); compare(platforms.currentIndex, 0)
    }
    function test_scope_traps_spatial_focus() {
        router.focusScope = grid
        router.focusViewIndex(grid, 4)
        router.handle("left")
        verify(router.within(host.activeFocusItem, grid))
        router.focusScope = null
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
