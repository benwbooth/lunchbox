import QtQuick

// Report the actual viewport, not all instantiated/cache-buffer delegates.
// Each view owns its snapshot so hiding a view cannot erase another's interests.
Item {
    id: priority
    required property var libraryModel
    required property Item view
    required property string viewId
    property string artworkType: "box-front"
    property real topInset: 0
    property string lastSnapshot: ""
    property bool forcePublish: false

    function schedule(force) {
        forcePublish = forcePublish || force === true
        // Throttle, don't debounce: a continuous scroll must keep promoting
        // newly visible rows instead of waiting until the gesture finishes.
        if (!refresh.running) refresh.start()
    }

    function publish() {
        if (!libraryModel || !view) return
        let games = []
        if (enabled && view.visible && view.width > 0 && view.height > 0) {
            const container = view.contentItem || view
            for (const item of container.children) {
                if (!item.gameId || !item.visible || item.width <= 0 || item.height <= 0)
                    continue
                const rect = item.mapToItem(view, 0, 0, item.width, item.height)
                if (rect.x + rect.width <= 0 || rect.x >= view.width
                        || rect.y + rect.height <= topInset || rect.y >= view.height)
                    continue
                const dx = rect.x + rect.width / 2 - view.width / 2
                const dy = rect.y + rect.height / 2 - view.height / 2
                games.push({ id: item.gameId, distance: dx * dx + dy * dy })
            }
        }
        games.sort((a, b) => a.distance - b.distance || a.id.localeCompare(b.id))
        const ids = [...new Set(games.map(game => game.id))]
        const snapshot = artworkType + ":" + JSON.stringify(ids)
        if (forcePublish || snapshot !== lastSnapshot) {
            libraryModel.set_visible_artwork_games(viewId, JSON.stringify(ids), artworkType)
            lastSnapshot = snapshot
        }
        forcePublish = false
    }

    Timer { id: refresh; interval: 50; onTriggered: priority.publish() }
    Connections {
        target: priority.view
        ignoreUnknownSignals: true // Flickable and PathView expose different geometry.
        function onContentXChanged() { priority.schedule() }
        function onContentYChanged() { priority.schedule() }
        function onOffsetChanged() { priority.schedule() }
        function onCurrentIndexChanged() { priority.schedule() }
        function onCountChanged() { priority.schedule() }
        function onWidthChanged() { priority.schedule() }
        function onHeightChanged() { priority.schedule() }
        function onVisibleChanged() { priority.publish(); priority.schedule() }
    }
    Connections {
        target: priority.view ? (priority.view.contentItem || priority.view) : null
        function onChildrenChanged() { priority.schedule() }
    }
    Connections {
        target: priority.libraryModel
        ignoreUnknownSignals: true
        function onModelReset() { priority.schedule(true) }
        function onDataChanged() { priority.schedule() }
        function onMedia_loadingChanged() { priority.schedule(true) }
        function onMedia_retrieval_enabledChanged() { priority.schedule(true) }
    }
    onArtworkTypeChanged: schedule(true)
    onEnabledChanged: { publish(); schedule() }
    Component.onCompleted: schedule(true)
    Component.onDestruction: {
        if (libraryModel) libraryModel.set_visible_artwork_games(viewId, "[]", artworkType)
    }
}
