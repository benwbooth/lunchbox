import QtQuick
import QtQuick.Controls

Button {
    id: root
    default property alias entries: menuColumn.data
    readonly property bool menuVisible: menu.visible
    function close() { menu.close() }
    text: "Library"
    implicitHeight: 43
    Accessible.name: "Library menu"
    Accessible.description: "Browse games and open library tools"
    onClicked: menu.visible ? menu.close() : menu.open()
    contentItem: Row {
        spacing: 14
        SemanticIcon { name: "menu"; color: "#c0c8d4"; anchors.verticalCenter: parent.verticalCenter }
        Text {
            text: root.text
            color: "#f4f7fb"
            font.pixelSize: 14
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }
    }
    leftPadding: 15
    background: Rectangle {
        radius: 9
        color: root.hovered || menu.visible ? "#272c34" : "#1b2330"
        border.color: root.visualFocus ? "#ffb454" : "#303b4b"
    }
    Popup {
        id: menu
        objectName: "libraryMenuPopup"
        x: 0
        y: root.height + 5
        width: Math.min(300, root.Window.window ? root.Window.window.width - 26 : 300)
        height: Math.min(menuColumn.implicitHeight + 16,
                         root.Window.window ? root.Window.window.height - 110 : 540)
        padding: 8
        modal: true
        dim: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        onOpened: {
            if (menuColumn.children.length > 0)
                menuColumn.children[0].forceActiveFocus()
        }
        onClosed: root.forceActiveFocus()
        function moveFocus(direction) {
            const items = []
            for (let child of menuColumn.children) {
                if (child.visible && child.enabled && child.activeFocusOnTab)
                    items.push(child)
            }
            if (!items.length) return
            let index = -1
            for (let i = 0; i < items.length; ++i) {
                if (items[i].activeFocus) index = i
            }
            index = (index + direction + items.length) % items.length
            items[index].forceActiveFocus()
        }
        background: Rectangle { color: "#141c28"; radius: 12; border.color: "#394559" }
        contentItem: ScrollView {
            id: menuScroll
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            Column {
                id: menuColumn
                width: menuScroll.availableWidth
                spacing: 3
                Keys.onDownPressed: menu.moveFocus(1)
                Keys.onUpPressed: menu.moveFocus(-1)
            }
        }
    }
    Connections {
        target: root.Window.window
        function onActiveFocusItemChanged() {
            if (!menu.visible) return
            const item = root.Window.window.activeFocusItem
            if (!item || item.parent !== menuColumn) return
            const flick = menuScroll.contentItem
            if (item.y < flick.contentY)
                flick.contentY = item.y
            else if (item.y + item.height > flick.contentY + menuScroll.availableHeight)
                flick.contentY = item.y + item.height - menuScroll.availableHeight
        }
    }
}
