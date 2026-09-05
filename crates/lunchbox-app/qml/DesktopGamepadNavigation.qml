import QtQuick
import QtQuick.Controls

Item {
    id: router
    required property var applicationWindow
    required property var gamepad
    property var libraryView: null
    property var focusScope: null
    property var overlayItem: null
    property var openCombo: null
    signal openGame(var item)
    signal backRequested()
    signal menuRequested()

    function within(item, parentItem) {
        for (let cursor = item; cursor; cursor = cursor.parent)
            if (cursor === parentItem) return true
        return false
    }

    function moveFocus(forward) {
        const scope = currentScope()
        const start = applicationWindow.activeFocusItem || scope || applicationWindow.contentItem
        let item = start
        for (let attempts = 0; attempts < 1000; ++attempts) {
            item = item.nextItemInFocusChain(forward)
            if (!item || item === start) break
            if (item.visible && item.enabled && item.activeFocusOnTab
                    && (!scope || within(item, scope))) {
                item.forceActiveFocus(Qt.TabFocusReason)
                reveal(item)
                return
            }
        }
    }

    function currentScope() {
        for (let item = applicationWindow.activeFocusItem; item; item = item.parent)
            if (overlayItem && item.parent === overlayItem) return item
        return focusScope
    }

    function reveal(item) {
        for (let parent = item.parent; parent; parent = parent.parent) {
            if (parent.contentY !== undefined && parent.contentHeight !== undefined) {
                const point = item.mapToItem(parent.contentItem, 0, 0)
                parent.contentY = Math.max(0, Math.min(parent.contentHeight - parent.height,
                    point.y < parent.contentY ? point.y :
                    point.y + item.height > parent.contentY + parent.height
                    ? point.y + item.height - parent.height : parent.contentY))
                break
            }
        }
    }

    function control(item) {
        for (let cursor = item; cursor && cursor !== applicationWindow.contentItem; cursor = cursor.parent) {
            if (cursor.popup !== undefined && cursor.currentIndex !== undefined) return cursor
            if (typeof cursor.clicked === "function" || typeof cursor.increase === "function") return cursor
        }
        return item
    }

    function handle(action) {
        if (!enabled || !applicationWindow.active) return false
        if (action === "back") {
            if (openCombo && openCombo.popup.visible) {
                openCombo.popup.close(); openCombo.forceActiveFocus(); openCombo = null
            } else backRequested()
            return true
        }
        if (action === "menu" || action === "home") { menuRequested(); return true }
        if (action === "page_left" || action === "page_right") {
            moveFocus(action === "page_right"); return true
        }
        const item = openCombo && openCombo.popup.visible ? openCombo : control(applicationWindow.activeFocusItem)
        if (libraryView && !currentScope() && (!item || within(item, libraryView))) {
            if (action === "accept" || action === "details") {
                if (libraryView.currentItem) openGame(libraryView.currentItem)
                return true
            }
            const columns = libraryView.columnCount || 1
            const delta = action === "left" ? -1 : action === "right" ? 1 :
                          action === "up" ? -columns : action === "down" ? columns : 0
            if (delta) {
                libraryView.focusIndex(Math.max(0, libraryView.currentIndex) + delta, ListView.Contain)
                return true
            }
        }
        if (item && item.popup !== undefined && item.currentIndex !== undefined) {
            if (action === "accept") {
                if (item.popup.visible) {
                    item.popup.close(); item.activated(item.currentIndex); item.forceActiveFocus(); openCombo = null
                } else { openCombo = item; item.popup.open() }
                return true
            }
            if (item.popup.visible && ["up", "down", "left", "right"].indexOf(action) >= 0) {
                item.currentIndex = Math.max(0, Math.min(item.count - 1,
                    item.currentIndex + (action === "up" || action === "left" ? -1 : 1)))
                return true
            }
        }
        if (item && typeof item.increase === "function" && (action === "left" || action === "right")) {
            if (action === "left") item.decrease(); else item.increase()
            return true
        }
        if (item && action === "accept" && typeof item.clicked === "function") {
            if (item.checkable) { item.toggle(); item.toggled() }
            item.clicked()
            return true
        }
        if (["left", "right", "up", "down"].indexOf(action) >= 0) {
            moveFocus(action === "right" || action === "down"); return true
        }
        return false
    }

    Connections {
        target: router.gamepad
        function onNavigation_revisionChanged() { router.handle(router.gamepad.navigation_action) }
    }

    Rectangle {
        id: focusRing
        parent: router.overlayItem || router.applicationWindow.contentItem
        readonly property var targetItem: router.applicationWindow.activeFocusItem
        visible: router.enabled && router.applicationWindow.active && targetItem
                 && targetItem !== router.applicationWindow.contentItem && targetItem.visible
                 && router.gamepad.connected_count > 0
        x: targetItem ? targetItem.mapToItem(parent, 0, 0).x - 2 : 0
        y: targetItem ? targetItem.mapToItem(parent, 0, 0).y - 2 : 0
        width: targetItem ? targetItem.width + 4 : 0
        height: targetItem ? targetItem.height + 4 : 0
        radius: 6
        color: "transparent"
        border.color: "#ffb454"
        border.width: 2
        z: 10000
    }
}
