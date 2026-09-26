import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "PreviewMuteFocus"
    when: windowShown
    visible: true
    width: 600
    height: 350

    GridView {
        id: grid
        width: 500
        height: 300
        cellWidth: 200
        cellHeight: 200
        model: 2
        delegate: Item {
            width: grid.cellWidth
            height: grid.cellHeight
            activeFocusOnTab: true
            readonly property bool controllerFocusElsewhere:
                grid.activeFocus && grid.currentIndex !== index

            Lunchbox.LbRoundButton {
                objectName: "mute" + index
                x: 150
                y: 130
                width: 40
                height: 40
                text: "M"
                focusPolicy: Qt.TabFocus
            }
        }
    }

    function test_pointer_mute_keeps_other_cards_hoverable() {
        const mute = findChild(grid, "mute0")
        verify(mute)
        mouseClick(mute)
        verify(!grid.activeFocus)
        const other = grid.itemAtIndex(1)
        verify(other)
        verify(!other.controllerFocusElsewhere)
    }
}
