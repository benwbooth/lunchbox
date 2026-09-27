import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "FavoriteButton"
    when: windowShown
    visible: true
    width: 220; height: 180

    Component {
        id: buttonComponent
        Lunchpail.FavoriteButton {
            x: 40; y: 40
            gameTitle: "Metroid"
            onToggleRequested: favorite => this.favorite = favorite
        }
    }
    SignalSpy { id: toggles; signalName: "toggleRequested" }

    function test_always_visible_and_toggleable() {
        const button = createTemporaryObject(buttonComponent, this)
        toggles.target = button
        toggles.clear()
        mouseMove(this, 200, 160)
        verify(button.visible)
        verify(!button.hovered)
        compare(button.text, "☆")
        compare(button.Accessible.name, "Add to Favorites: Metroid")
        mouseClick(button)
        compare(toggles.count, 1)
        compare(button.favorite, true)
        compare(button.text, "★")
        compare(button.Accessible.name, "Remove from Favorites: Metroid")
        button.forceActiveFocus()
        keyClick(Qt.Key_Space)
        compare(toggles.count, 2)
        compare(button.favorite, false)
        toggles.target = null
    }

    function test_busy_keeps_star_and_blocks_duplicate_changes() {
        const button = createTemporaryObject(buttonComponent, this, { favorite: true, busy: true })
        toggles.target = button
        toggles.clear()
        verify(button.visible)
        compare(button.text, "★")
        mouseClick(button)
        compare(toggles.count, 0)
        button.busy = false
        mouseClick(button)
        compare(toggles.count, 1)
        compare(button.favorite, false)
        toggles.target = null
    }
}
