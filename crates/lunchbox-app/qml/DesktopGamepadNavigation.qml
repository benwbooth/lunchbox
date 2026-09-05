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

    // Work in window coordinates, intersecting every clipping ancestor. Cached
    // delegates outside a viewport must never attract controller focus.
    function visibleRect(item) {
        if (!item || item.width <= 0 || item.height <= 0) return null
        const root = applicationWindow.contentItem
        const p = item.mapToItem(root, 0, 0)
        let r = { x: p.x, y: p.y, right: p.x + item.width, bottom: p.y + item.height }
        for (let cursor = item; cursor; cursor = cursor.parent) {
            if (!cursor.visible || !cursor.enabled || cursor.opacity === 0) return null
            if (cursor.clip || cursor === root) {
                const q = cursor.mapToItem(root, 0, 0)
                r.x = Math.max(r.x, q.x); r.y = Math.max(r.y, q.y)
                r.right = Math.min(r.right, q.x + cursor.width)
                r.bottom = Math.min(r.bottom, q.y + cursor.height)
            }
        }
        return r.right > r.x && r.bottom > r.y ? r : null
    }

    function candidates(scope) {
        let result = []
        function visit(item) {
            if (!item.visible || !item.enabled || item.opacity === 0) return
            if (item.activeFocusOnTab && visibleRect(item)
                    && typeof item.positionViewAtIndex !== "function") {
                result.push(item)
                // A focusable card/control is one spatial target, not its internals.
                return
            }
            for (let child of item.children) visit(child)
        }
        visit(scope || applicationWindow.contentItem)
        return result
    }

    function ancestorView(item) {
        for (let cursor = item; cursor; cursor = cursor.parent)
            if (typeof cursor.positionViewAtIndex === "function") return cursor
        return null
    }

    function delegateContext(item) {
        const view = ancestorView(item)
        if (!view) return null
        for (let cursor = item; cursor && cursor !== view; cursor = cursor.parent) {
            if (cursor.index !== undefined && view.itemAtIndex(cursor.index) === cursor)
                return { view: view, item: cursor }
        }
        return null
    }

    function owningView(item) {
        const context = delegateContext(item)
        return context ? context.view : null
    }

    function focusTarget(item) {
        const context = delegateContext(item)
        if (context) context.view.currentIndex = context.item.index
        item.forceActiveFocus(Qt.OtherFocusReason)
    }

    function focusViewIndex(view, index) {
        if (!view || view.count <= 0) return false
        index = Math.max(0, Math.min(view.count - 1, index))
        view.currentIndex = index
        view.positionViewAtIndex(index, ListView.Contain)
        view.forceLayout()
        const item = view.itemAtIndex(index)
        if (item) focusTarget(item)
        return !!item
    }

    function moveFocus(direction) {
        const scope = currentScope()
        const targets = candidates(scope)
        let start = control(applicationWindow.activeFocusItem)
        if ((!start || start === applicationWindow.contentItem) && !scope && libraryView)
            start = libraryView.currentItem
        const r = visibleRect(start)
        if (!r || start === applicationWindow.contentItem) {
            if (targets.length) focusTarget(targets[0])
            return
        }
        const horizontal = direction === "left" || direction === "right"
        const positive = direction === "right" || direction === "down"
        const center = horizontal ? (r.x + r.right) / 2 : (r.y + r.bottom) / 2
        const cross = horizontal ? (r.y + r.bottom) / 2 : (r.x + r.right) / 2
        let best = null, bestScore = Infinity
        for (let candidate of targets) {
            if (candidate === start || within(start, candidate) || within(candidate, start)) continue
            const q = visibleRect(candidate)
            const next = horizontal ? (q.x + q.right) / 2 : (q.y + q.bottom) / 2
            if ((positive ? next - center : center - next) <= 1) continue
            const crossNext = horizontal ? (q.y + q.bottom) / 2 : (q.x + q.right) / 2
            const overlap = horizontal ? Math.min(r.bottom, q.bottom) - Math.max(r.y, q.y)
                                       : Math.min(r.right, q.right) - Math.max(r.x, q.x)
            const gap = horizontal
                ? (positive ? q.x - r.right : r.x - q.right)
                : (positive ? q.y - r.bottom : r.y - q.bottom)
            const score = (overlap > 0 ? 0 : 1000000) + Math.max(0, gap) * 100
                          + Math.abs(crossNext - cross)
            if (score < bestScore) { best = candidate; bestScore = score }
        }
        // At a virtual viewport edge, reveal the next visual row before leaving
        // the pane vertically. Horizontal movement never wraps to another row.
        const view = owningView(start)
        if (!horizontal && view && owningView(best) !== view) {
            const index = delegateContext(start).item.index
            const next = index + (positive ? 1 : -1) * (view.columnCount || 1)
            if (next >= 0 && next < view.count && focusViewIndex(view, next)) return
        }
        if (best) focusTarget(best)
    }

    function scroll(action) {
        const item = control(applicationWindow.activeFocusItem)
        const view = ancestorView(item)
        if (view) {
            const first = action === "scroll_first", last = action === "scroll_last"
            const rowHeight = view.cellHeight || (item ? item.height : 0) || 40
            const page = Math.max(1, Math.floor(view.height / rowHeight)) * (view.columnCount || 1)
            const context = delegateContext(item)
            const index = context ? context.item.index : Math.max(0, view.currentIndex)
            return focusViewIndex(view, first ? 0 : last ? view.count - 1
                                  : index + (action === "page_left" ? -page : page))
        }
        for (let cursor = item; cursor; cursor = cursor.parent) {
            if (cursor.contentY === undefined || cursor.contentHeight === undefined) continue
            const beginning = cursor.originY || 0
            const end = beginning + Math.max(0, cursor.contentHeight - cursor.height)
            cursor.contentY = action === "scroll_first" ? beginning : action === "scroll_last" ? end
                : Math.max(beginning, Math.min(end, cursor.contentY
                    + (action === "page_left" ? -cursor.height : cursor.height)))
            const targets = candidates(cursor)
            if (targets.length) focusTarget(targets[action === "scroll_last" ? targets.length - 1 : 0])
            return true
        }
        return false
    }

    function currentScope() {
        for (let item = applicationWindow.activeFocusItem; item; item = item.parent)
            if (overlayItem && item.parent === overlayItem) return item
        return focusScope
    }

    function control(item) {
        if (item && typeof item.positionViewAtIndex === "function" && item.currentItem)
            return item.currentItem
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
        if (["page_left", "page_right", "scroll_first", "scroll_last"].indexOf(action) >= 0) {
            scroll(action); return true
        }
        const item = openCombo && openCombo.popup.visible ? openCombo : control(applicationWindow.activeFocusItem)
        if (libraryView && !currentScope() && (!item || item === applicationWindow.contentItem
                                              || owningView(item) === libraryView)) {
            if (action === "accept" || action === "details") {
                if (libraryView.currentItem) openGame(libraryView.currentItem)
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
            moveFocus(action); return true
        }
        return false
    }

    Connections {
        target: router.gamepad
        function onNavigation_revisionChanged() { router.handle(router.gamepad.navigation_action) }
    }

    Rectangle {
        id: focusRing
        objectName: "desktopGamepadFocusRing"
        readonly property var targetItem: router.control(router.applicationWindow.activeFocusItem)
        // Follow the actual control's transforms and clipping, not a detached
        // snapshot of its window position. Already-decorated controls opt out.
        parent: targetItem || router.applicationWindow.contentItem
        visible: router.enabled && router.applicationWindow.active && targetItem
                 && targetItem !== router.applicationWindow.contentItem && targetItem.visible
                 && targetItem.providesFocusIndicator !== true
                 && router.gamepad.connected_count > 0
        anchors.fill: parent
        radius: 6
        color: "transparent"
        border.color: "#ffb454"
        border.width: 2
        z: 10000
    }
}
