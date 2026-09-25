import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "MomentumScrollSurfaces"
    when: windowShown
    visible: true
    width: 900
    height: 600

    Lunchbox.MomentumListView {
        id: list
        x: 0; y: 0; width: 240; height: 200
        clip: true
        model: 100
        delegate: Rectangle { width: ListView.view.verticalContentWidth; height: 35 }
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AlwaysOn }
    }
    Lunchbox.MomentumGridView {
        id: grid
        x: 260; y: 0; width: 240; height: 200
        clip: true
        model: 100
        cellWidth: verticalContentWidth / 3; cellHeight: 80
        delegate: Rectangle { width: 70; height: 70 }
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AlwaysOn }
    }
    Lunchbox.MomentumFlickable {
        id: flick
        x: 520; y: 0; width: 240; height: 200
        blockNativeWheel: true
        clip: true
        contentWidth: width; contentHeight: 2000
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AlwaysOn }
        Rectangle { width: 220; height: 2000 }
    }
    Lunchbox.MomentumScrollView {
        id: scrollView
        x: 0; y: 250; width: 260; height: 200
        ScrollBar.vertical.policy: ScrollBar.AlwaysOn
        Column {
            width: scrollView.availableWidth
            Repeater {
                model: 50
                delegate: Rectangle { width: 220; height: 30 }
            }
        }
    }
    Lunchbox.MomentumScrollView {
        id: nestedScroll
        x: 300; y: 250; width: 260; height: 200
        Column {
            width: nestedScroll.availableWidth
            Rectangle { width: parent.width; height: 20 }
            Lunchbox.MomentumListView {
                id: nestedList
                width: parent.width
                height: 120
                clip: true
                model: 100
                delegate: Rectangle { width: ListView.view.width; height: 30 }
            }
            Rectangle { width: parent.width; height: 500 }
        }
    }
    Lunchbox.MomentumScrollView {
        id: textScroll
        x: 590; y: 250; width: 260; height: 200
        TextArea {
            id: longText
            width: textScroll.availableWidth
            text: Array(80).fill("A long line of editor text").join("\n")
        }
    }
    Lunchbox.MomentumScrollView {
        id: shortNestedScroll
        x: 300; y: 470; width: 260; height: 120
        Column {
            width: shortNestedScroll.availableWidth
            Lunchbox.MomentumListView {
                id: shortNestedList
                width: parent.width
                height: 80
                model: 2
                delegate: Rectangle { width: ListView.view.width; height: 30 }
            }
            Rectangle { width: parent.width; height: 500 }
        }
    }
    Item {
        id: gutterFixture
        x: 590; y: 470; width: 260; height: 120
        Lunchbox.MomentumFlickable {
            id: gutterList
            width: parent.width - 20
            height: parent.height
            blockNativeWheel: true
            contentWidth: width
            contentHeight: gutterContent.height
            Column {
                id: gutterContent
                width: gutterList.width
                Repeater {
                    model: 100
                    delegate: Rectangle {
                        width: gutterContent.width
                        height: 24 + (index % 5) * 17
                    }
                }
            }
            ScrollBar.vertical: ScrollBar {
                id: gutterBar
                parent: gutterFixture
                x: gutterFixture.width - width
                y: 0
                height: gutterFixture.height
                width: 10
                policy: ScrollBar.AlwaysOn
                visible: gutterList.contentHeight > gutterList.height
            }
        }
    }

    function test_wheel_momentum_and_scrollbar_gutters() {
        verify(list.verticalScrollBarGutter > 0)
        verify(list.itemAtIndex(0).width < list.ScrollBar.vertical.x)
        verify(grid.verticalScrollBarGutter > 0)
        const rightCard = grid.itemAtIndex(2)
        verify(rightCard.x + rightCard.width <= grid.ScrollBar.vertical.x)
        verify(flick.verticalScrollBarGutter > 0)
        verify(scrollView.rightPadding >= scrollView.effectiveScrollBarWidth)
        verify(scrollView.availableWidth <= scrollView.width - scrollView.effectiveScrollBarWidth)
        verify(gutterBar.x >= gutterList.x + gutterList.width + 8)
        compare(gutterBar.height, gutterList.height)
        verify(gutterBar.visible)
        mouseWheel(gutterList, 90, 90, 0, -120, Qt.LeftButton, Qt.NoModifier)
        const gutterStart = gutterList.contentY
        verify(gutterStart > 0)
        wait(100)
        verify(gutterList.contentY > gutterStart)
        mouseWheel(list, 90, 90, 0, -120, Qt.LeftButton, Qt.NoModifier)
        mouseWheel(grid, 90, 90, 0, -120, Qt.LeftButton, Qt.NoModifier)
        mouseWheel(flick, 90, 90, 0, -120, Qt.LeftButton, Qt.NoModifier)
        mouseWheel(scrollView, 90, 90, 0, -120, Qt.LeftButton, Qt.NoModifier)
        verify(list.contentY > 0)
        verify(grid.contentY > 0)
        verify(flick.contentY > 0)
        verify(scrollView.contentItem.contentY > 0)
        const first = list.contentY
        wait(100)
        verify(list.contentY > first)
    }

    function test_nested_wheel_moves_only_inner_list() {
        nestedList.contentY = 0
        nestedScroll.contentItem.contentY = 0
        mouseWheel(nestedList, 90, 60, 0, -120, Qt.LeftButton, Qt.NoModifier)
        verify(nestedList.contentY > 0)
        compare(nestedScroll.contentItem.contentY, 0)
    }

    function test_inner_list_at_bottom_passes_wheel_to_outer_scroll() {
        nestedList.contentY = nestedList.contentHeight - nestedList.height
        nestedScroll.contentItem.contentY = 0
        mouseWheel(nestedList, 90, 60, 0, -120, Qt.LeftButton, Qt.NoModifier)
        verify(nestedScroll.contentItem.contentY > 0)
    }

    function test_text_area_scroll_view_keeps_momentum() {
        mouseWheel(longText, 90, 90, 0, -120, Qt.LeftButton, Qt.NoModifier)
        verify(textScroll.contentItem.contentY > 0)
    }

    function test_short_inner_list_leaves_wheel_to_outer_scroll() {
        mouseWheel(shortNestedList, 90, 40, 0, -120, Qt.LeftButton, Qt.NoModifier)
        verify(shortNestedScroll.contentItem.contentY > 0)
    }
}
