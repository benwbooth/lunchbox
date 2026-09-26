import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "GridHoverFocusState"
    when: windowShown
    visible: true

    Lunchbox.GridHoverFocusState { id: state }
    readonly property bool otherCardBlocked:
        state.controllerFocusElsewhere(true, 0, 1)
    readonly property bool focusedCardShown:
        state.focusedCard(true, 0, 0)

    Item {
        id: card
        width: 160
        height: 120
        HoverHandler {
            id: cardHover
            onPointChanged: {
                if (hovered)
                    state.pointerMoved(point.scenePosition.x,
                                       point.scenePosition.y)
            }
        }
    }

    function init() {
        state.pointerActive = false
        state.pointerPositionKnown = false
        state.lastSceneX = 0
        state.lastSceneY = 0
    }

    function test_pointer_motion_restores_hover_after_controller_focus() {
        state.navigationFocused()
        verify(state.controllerFocusElsewhere(true, 0, 1))
        verify(state.focusedCard(true, 0, 0))
        compare(otherCardBlocked, true)
        compare(focusedCardShown, true)

        state.pointerMoved(100, 100)
        compare(state.pointerActive, true)
        verify(!state.controllerFocusElsewhere(true, 0, 1))
        verify(!state.focusedCard(true, 0, 0))
        compare(otherCardBlocked, false)
        compare(focusedCardShown, false)

        state.navigationFocused()
        compare(state.pointerActive, false)
        compare(otherCardBlocked, true)
        compare(focusedCardShown, true)
        // Card motion under a stationary cursor must not steal focus back.
        state.pointerMoved(100, 100)
        compare(state.pointerActive, false)
        state.pointerMoved(102, 100)
        compare(state.pointerActive, true)
    }

    function test_pointer_click_does_not_leave_another_card_locked_out() {
        state.navigationFocused()
        state.pointerActivated()
        verify(!state.controllerFocusElsewhere(true, 0, 1))
        verify(!state.focusedCard(true, 0, 0))
    }

    function test_hover_handler_wakes_pointer_mode() {
        state.navigationFocused()
        mouseMove(card, 10, 10)
        mouseMove(card, 14, 10)
        tryCompare(state, "pointerActive", true)
        verify(cardHover.hovered)
    }
}
