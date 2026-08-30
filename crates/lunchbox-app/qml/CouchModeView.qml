pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: view

    required property var library
    required property var details
    required property var gamepad
    property bool active: false
    property int navigationZone: 2
    property int categoryIndex: 0
    property int actionIndex: 0
    property bool platformWheelOpen: false
    property int platformWheelIndex: 0
    property bool collectionWheelOpen: false
    property int collectionWheelIndex: 0
    property bool variantWheelOpen: false
    property int variantWheelIndex: 0
    property bool attractOpen: false
    property bool attractProbeEnabled: false
    property string attractReason: "manual"
    property real attractProgress: 0
    property bool launchStatusOverlayOpen: false
    property bool overlayOpen: false
    property string overlayMode: "details"
    property int menuActionIndex: 0
    property string preferredGameId: ""
    property string currentFilterKey: ""
    property string currentPlatformName: ""
    property string selectedGameId: ""
    property string loadedGameId: ""
    property int selectedDatabaseId: 0
    property string selectedTitle: ""
    property string selectedPlatform: ""
    property bool selectedLocal: false
    property bool selectedDownloadable: false
    property int mediaRevision: library.media_revision
    readonly property color background: library.couch_theme_background
    readonly property color panel: library.couch_theme_panel
    readonly property color panelRaised: library.couch_theme_panel_raised
    readonly property color ink: library.couch_theme_ink
    readonly property color muted: library.couch_theme_muted
    readonly property color accent: library.couch_theme_accent
    readonly property color accentCool: library.couch_theme_accent_cool
    readonly property color danger: library.couch_theme_danger
    readonly property int cardRadius: library.couch_theme_card_radius
    readonly property real heroScrimOpacity: library.couch_theme_hero_scrim_percent / 100.0
    readonly property bool cinematicWheel: library.couch_view_style === "wheel"
    readonly property int menuActionCount: 7
    readonly property var categories: [
        { label: "ALL GAMES", key: "" },
        { label: "PLATFORMS", key: "platform" },
        { label: "COLLECTIONS", key: "collections" },
        { label: "MY COLLECTION", key: "local" },
        { label: "MINERVA", key: "downloadable" },
        { label: "FAVORITES", key: "favorites" },
        { label: "RECENT", key: "recent" }
    ]
    readonly property string currentCollectionId:
        currentFilterKey.indexOf("collection:") === 0
        ? currentFilterKey.substring("collection:".length) : ""
    readonly property string currentShelfLabel: {
        library.collection_revision
        if (currentCollectionId.length > 0) {
            const index = collectionIndexForId(currentCollectionId)
            if (index >= 0)
                return library.collection_name_at(index).toUpperCase()
        }
        if (currentPlatformName.length > 0)
            return currentPlatformName.toUpperCase()
        for (let index = 0; index < categories.length; ++index) {
            if (categories[index].key === currentFilterKey)
                return categories[index].label
        }
        return "ALL GAMES"
    }
    readonly property url heroUrl: {
        mediaRevision
        return selectedDatabaseId > 0
               ? library.artwork_url(selectedDatabaseId, "fanart") : ""
    }
    readonly property url coverUrl: {
        mediaRevision
        return selectedDatabaseId > 0
               ? library.artwork_url(selectedDatabaseId, "box-front") : ""
    }
    readonly property bool favorite: {
        library.favorite_revision
        return selectedGameId.length > 0 && library.is_favorite(selectedGameId)
    }
    readonly property bool favoriteBusy: {
        library.favorite_pending_count
        return selectedGameId.length > 0 && library.favorite_pending(selectedGameId)
    }
    readonly property bool detailsCurrent: selectedGameId.length > 0
                                           && details.game_id === selectedGameId
    readonly property string primaryAction: !detailsCurrent || details.loading ? "LOADING…"
            : details.launch_busy ? "STARTING…"
            : details.game_running ? "GAME IS RUNNING"
            : details.can_launch ? "PLAY"
            : selectedLocal ? "SET UP PLAY"
            : selectedDownloadable ? "DOWNLOAD OPTIONS"
            : "VIEW DETAILS"

    signal exitRequested()
    signal filterRequested(string key)
    signal platformRequested(string platform)
    signal collectionRequested(string collectionId, string collectionName)
    signal variantRequested(string gameId, int databaseId, string title,
                            string platform, bool local, bool downloadable)
    signal gameSelected(string gameId, int databaseId, string title,
                        string platform, bool local, bool downloadable)
    signal detailsRequested(string gameId, int databaseId, string title,
                            string platform, bool local, bool downloadable)
    signal launchRequested()

    visible: active
    focus: active

    function accentFor(value) {
        const colors = [view.accent, view.accentCool,
                        Qt.lighter(view.accent, 1.22),
                        Qt.darker(view.accent, 1.28),
                        Qt.lighter(view.accentCool, 1.16),
                        Qt.darker(view.accentCool, 1.22)]
        if (!value || value.length === 0)
            return colors[0]
        return colors[value.charCodeAt(0) % colors.length]
    }

    function withAlpha(value, opacity) {
        return Qt.rgba(value.r, value.g, value.b, opacity)
    }

    function toggleViewStyle() {
        const selectedId = selectedGameId
        const nextStyle = cinematicWheel ? "shelf" : "wheel"
        if (!library.save_couch_view_style(nextStyle))
            return false
        Qt.callLater(function() {
            if (selectedId.length > 0)
                focusGameById(selectedId)
            else if (shelf.currentIndex >= 0)
                shelf.positionViewAtIndex(shelf.currentIndex, ListView.Center)
            forceActiveFocus()
        })
        return true
    }

    function moveShelf(delta) {
        if (shelf.count <= 0)
            return
        noteActivity()
        shelf.currentIndex = Math.max(0, Math.min(shelf.count - 1,
                                                 shelf.currentIndex + delta))
        shelf.positionViewAtIndex(shelf.currentIndex, ListView.Contain)
    }

    function noteActivity() {
        if (attractIdleTimer.running)
            attractIdleTimer.restart()
    }

    function startAttractMode(reason) {
        if (!active || shelf.count <= 0)
            return false
        overlayOpen = false
        platformWheelOpen = false
        collectionWheelOpen = false
        variantWheelOpen = false
        attractReason = reason === "idle" ? "idle" : "manual"
        navigationZone = 2
        if (shelf.currentIndex < 0)
            shelf.currentIndex = 0
        shelf.positionViewAtIndex(shelf.currentIndex, ListView.Center)
        captureCurrentGame()
        loadCurrentGame()
        attractOpen = true
        attractCycleTimer.restart()
        attractProgressAnimation.restart()
        forceActiveFocus()
        return true
    }

    function stopAttractMode() {
        if (!attractOpen)
            return
        attractOpen = false
        attractProgressAnimation.stop()
        if (active) {
            forceActiveFocus()
            noteActivity()
        }
    }

    function moveAttract(delta) {
        if (shelf.count <= 0)
            return
        const current = Math.max(0, shelf.currentIndex)
        shelf.currentIndex = ((current + delta) % shelf.count + shelf.count) % shelf.count
        shelf.positionViewAtIndex(shelf.currentIndex, ListView.Center)
        selectionDelay.restart()
        attractCycleTimer.restart()
        attractProgressAnimation.restart()
    }

    function chooseCategory(index) {
        noteActivity()
        categoryIndex = Math.max(0, Math.min(categories.length - 1, index))
        if (categories[categoryIndex].key === "platform") {
            openPlatformWheel()
            return
        }
        if (categories[categoryIndex].key === "collections") {
            openCollectionWheel()
            return
        }
        const shelfKey = categories[categoryIndex].key.length > 0
                         ? categories[categoryIndex].key : "all"
        library.save_couch_state(shelfKey, "")
        filterRequested(categories[categoryIndex].key)
        shelf.currentIndex = 0
    }

    function syncCategory() {
        if (currentCollectionId.length > 0) {
            for (let collectionCategory = 0;
                    collectionCategory < categories.length;
                    ++collectionCategory) {
                if (categories[collectionCategory].key === "collections") {
                    categoryIndex = collectionCategory
                    return
                }
            }
        }
        if (currentPlatformName.length > 0) {
            for (let platformIndex = 0; platformIndex < categories.length;
                    ++platformIndex) {
                if (categories[platformIndex].key === "platform") {
                    categoryIndex = platformIndex
                    return
                }
            }
        }
        for (let index = 0; index < categories.length; ++index) {
            if (categories[index].key === currentFilterKey) {
                categoryIndex = index
                return
            }
        }
        categoryIndex = 0
    }

    function platformIndexForName(name) {
        if (!name || name.length === 0)
            return 0
        for (let index = 0; index < library.platform_count; ++index) {
            if (library.platform_name_at(index) === name)
                return index
        }
        return 0
    }

    function openPlatformWheel() {
        if (library.platform_count <= 0)
            return
        overlayOpen = false
        collectionWheelOpen = false
        variantWheelOpen = false
        attractOpen = false
        platformWheelIndex = platformIndexForName(currentPlatformName)
        platformWheel.currentIndex = platformWheelIndex
        platformWheel.positionViewAtIndex(platformWheelIndex, ListView.Center)
        platformWheelOpen = true
        forceActiveFocus()
    }

    function focusPlatform(name) {
        if (!platformWheelOpen)
            openPlatformWheel()
        platformWheelIndex = platformIndexForName(name)
        platformWheel.currentIndex = platformWheelIndex
        platformWheel.positionViewAtIndex(platformWheelIndex, ListView.Center)
    }

    function closePlatformWheel() {
        platformWheelOpen = false
        if (active)
            forceActiveFocus()
    }

    function movePlatformWheel(delta) {
        if (library.platform_count <= 0)
            return
        platformWheelIndex = Math.max(0, Math.min(library.platform_count - 1,
                                                  platformWheelIndex + delta))
        platformWheel.currentIndex = platformWheelIndex
        platformWheel.positionViewAtIndex(platformWheelIndex, ListView.Center)
    }

    function choosePlatform(index) {
        noteActivity()
        const platform = library.platform_name_at(index)
        if (platform.length === 0
                || !library.save_couch_state("platform", platform))
            return
        platformWheelIndex = index
        platformWheelOpen = false
        navigationZone = 2
        platformRequested(platform)
        forceActiveFocus()
    }

    function collectionIndexForId(collectionId) {
        for (let index = 0; index < library.collection_count; ++index) {
            if (library.collection_id_at(index) === collectionId)
                return index
        }
        return -1
    }

    function openCollectionWheel() {
        overlayOpen = false
        platformWheelOpen = false
        variantWheelOpen = false
        attractOpen = false
        const selectedIndex = collectionIndexForId(currentCollectionId)
        collectionWheelIndex = selectedIndex >= 0 ? selectedIndex : 0
        collectionWheel.currentIndex = library.collection_count > 0
                                     ? collectionWheelIndex : -1
        if (collectionWheel.currentIndex >= 0)
            collectionWheel.positionViewAtIndex(collectionWheel.currentIndex,
                                                ListView.Center)
        collectionWheelOpen = true
        forceActiveFocus()
    }

    function focusCollection(collectionId) {
        if (!collectionWheelOpen)
            openCollectionWheel()
        const index = collectionIndexForId(collectionId)
        if (index < 0)
            return false
        collectionWheelIndex = index
        collectionWheel.currentIndex = index
        collectionWheel.positionViewAtIndex(index, ListView.Center)
        return true
    }

    function closeCollectionWheel() {
        collectionWheelOpen = false
        if (active)
            forceActiveFocus()
    }

    function currentVariantIndex() {
        for (let index = 0; index < view.details.variant_count; ++index) {
            if (view.details.variant_is_current_at(index))
                return index
        }
        return 0
    }

    function openVariantWheel() {
        if (view.details.loading || view.details.variant_count < 2)
            return false
        overlayOpen = false
        platformWheelOpen = false
        collectionWheelOpen = false
        attractOpen = false
        variantWheelIndex = currentVariantIndex()
        variantWheel.currentIndex = variantWheelIndex
        variantWheel.positionViewAtIndex(variantWheelIndex, ListView.Center)
        variantWheelOpen = true
        forceActiveFocus()
        return true
    }

    function closeVariantWheel() {
        variantWheelOpen = false
        if (active) {
            overlayMode = "menu"
            overlayOpen = true
            menuActionIndex = 3
            forceActiveFocus()
        }
    }

    function moveVariantWheel(delta) {
        if (view.details.variant_count <= 0)
            return
        variantWheelIndex = Math.max(0, Math.min(view.details.variant_count - 1,
                                                  variantWheelIndex + delta))
        variantWheel.currentIndex = variantWheelIndex
        variantWheel.positionViewAtIndex(variantWheelIndex, ListView.Center)
    }

    function focusVariant(index) {
        if (index < 0 || index >= view.details.variant_count)
            return false
        variantWheelIndex = index
        variantWheel.currentIndex = index
        variantWheel.positionViewAtIndex(index, ListView.Center)
        return true
    }

    function chooseVariant(index) {
        if (index < 0 || index >= view.details.variant_count
                || view.details.variant_is_current_at(index))
            return false
        const gameId = view.details.variant_game_id_at(index)
        const databaseId = view.details.variant_database_id_at(index)
        const title = view.details.variant_title_at(index)
        const platform = view.selectedPlatform
        const local = view.details.variant_is_local_at(index)
        const downloadable = view.details.variant_is_downloadable_at(index)
        if (gameId.length === 0 || title.length === 0)
            return false
        variantWheelOpen = false
        navigationZone = 2
        selectedGameId = gameId
        selectedDatabaseId = databaseId
        selectedTitle = title
        selectedLocal = local
        selectedDownloadable = downloadable
        loadedGameId = ""
        library.request_priority_artwork(databaseId, title, platform, "box-front")
        library.request_priority_artwork(databaseId, title, platform, "fanart")
        variantRequested(gameId, databaseId, title, platform, local, downloadable)
        focusGameById(gameId)
        forceActiveFocus()
        return true
    }

    function focusGameById(gameId) {
        const row = library.row_for_game(gameId)
        if (row < 0 || row >= shelf.count)
            return false
        shelf.currentIndex = row
        shelf.positionViewAtIndex(row, ListView.Center)
        return true
    }

    function moveCollectionWheel(delta) {
        if (library.collection_count <= 0)
            return
        collectionWheelIndex = Math.max(
                    0, Math.min(library.collection_count - 1,
                                collectionWheelIndex + delta))
        collectionWheel.currentIndex = collectionWheelIndex
        collectionWheel.positionViewAtIndex(collectionWheelIndex,
                                            ListView.Center)
    }

    function chooseCollection(index) {
        noteActivity()
        const collectionId = library.collection_id_at(index)
        const collectionName = library.collection_name_at(index)
        if (collectionId.length === 0
                || !library.save_couch_state("collection:" + collectionId, ""))
            return
        collectionWheelIndex = index
        collectionWheelOpen = false
        navigationZone = 2
        collectionRequested(collectionId, collectionName)
        forceActiveFocus()
    }

    function captureCurrentGame() {
        const item = shelf.currentItem
        if (!item) {
            selectionRetry.restart()
            return false
        }
        selectedGameId = item.gameId
        selectedDatabaseId = item.gameDatabaseId
        selectedTitle = item.gameTitle
        selectedPlatform = item.gamePlatform
        selectedLocal = item.gameLocal
        selectedDownloadable = item.gameDownloadable
        library.request_priority_artwork(selectedDatabaseId, selectedTitle,
                                         selectedPlatform, "box-front")
        library.request_priority_artwork(selectedDatabaseId, selectedTitle,
                                         selectedPlatform, "fanart")
        return true
    }

    function loadCurrentGame() {
        if (!captureCurrentGame() || loadedGameId === selectedGameId)
            return
        loadedGameId = selectedGameId
        gameSelected(selectedGameId, selectedDatabaseId, selectedTitle,
                     selectedPlatform, selectedLocal, selectedDownloadable)
    }

    function requestDetails() {
        if (selectedGameId.length === 0)
            return
        detailsRequested(selectedGameId, selectedDatabaseId, selectedTitle,
                         selectedPlatform, selectedLocal, selectedDownloadable)
    }

    function activateAction(index) {
        if (selectedGameId.length === 0)
            return
        if (index === 0) {
            if (!detailsCurrent || details.loading)
                return
            if (details.can_launch && !details.launch_busy
                    && !details.game_running) {
                launchStatusOverlayOpen = true
                launchRequested()
            }
            else
                requestDetails()
        } else if (index === 1) {
            requestDetails()
        } else if (!favoriteBusy) {
            library.set_favorite(selectedGameId, !favorite)
        }
    }

    function openOverlay(mode) {
        attractOpen = false
        platformWheelOpen = false
        collectionWheelOpen = false
        variantWheelOpen = false
        overlayMode = mode === "menu" ? "menu" : "details"
        overlayOpen = true
        menuActionIndex = 0
        detailsScroller.contentY = 0
        forceActiveFocus()
    }

    function closeOverlay() {
        overlayOpen = false
        if (active)
            forceActiveFocus()
    }

    function captureOverlay(path, callback) {
        couchOverlay.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    function captureTheme(path, callback) {
        view.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    function capturePlatformWheel(path, callback) {
        platformWheelOverlay.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    function captureCollectionWheel(path, callback) {
        collectionWheelOverlay.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    function captureVariantWheel(path, callback) {
        variantWheelOverlay.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    function captureAttract(path, callback) {
        attractOverlay.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    function captureLaunchStatus(path, callback) {
        launchStatusOverlay.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    function menuActionLabel(index) {
        if (index === 0)
            return primaryAction
        if (index === 1)
            return favorite ? "REMOVE FAVORITE" : "ADD FAVORITE"
        if (index === 2)
            return "DESKTOP DETAILS"
        if (index === 3)
            return "RELEASES & MEDIA"
        if (index === 4)
            return "START ATTRACT MODE"
        if (index === 5)
            return cinematicWheel ? "USE COVER SHELF" : "USE CINEMATIC WHEEL"
        return "RETURN TO BROWSING"
    }

    function menuActionDescription(index) {
        if (index === 0) {
            if (details.can_launch)
                return "Launch with the selected emulator and exact local file."
            if (selectedDownloadable)
                return "Review exact Minerva files and download options."
            if (selectedLocal)
                return "Choose an installed emulator and finish play setup."
            return "Open the complete management view for this catalog record."
        }
        if (index === 1)
            return favorite ? "Remove this exact game from Favorites."
                            : "Keep this exact game in Favorites."
        if (index === 2)
            return "Open releases, media, firmware, launch profiles, and metadata tools."
        if (index === 3)
            return "Browse exact regional and version releases before choosing one."
        if (index === 4)
            return "Let Couch Mode rotate through games on the current shelf."
        if (index === 5)
            return cinematicWheel
                   ? "Switch to the horizontal cover shelf without changing this game."
                   : "Switch to a vertical cinematic wheel without changing this game."
        return "Close this menu without changing the selected game."
    }

    function menuActionEnabled(index) {
        return index !== 3 || (!view.details.loading && view.details.variant_count > 1)
    }

    function nextMenuAction(start, delta) {
        let index = start
        for (let step = 0; step < menuActionCount; ++step) {
            index = Math.max(0, Math.min(menuActionCount - 1, index + delta))
            if (menuActionEnabled(index))
                return index
            if (index === 0 || index === menuActionCount - 1)
                break
        }
        return start
    }

    function activateMenuAction(index) {
        if (index === 0) {
            closeOverlay()
            activateAction(0)
        } else if (index === 1) {
            if (!favoriteBusy && selectedGameId.length > 0)
                library.set_favorite(selectedGameId, !favorite)
        } else if (index === 2) {
            closeOverlay()
            requestDetails()
        } else if (index === 3) {
            openVariantWheel()
        } else if (index === 4) {
            closeOverlay()
            startAttractMode("manual")
        } else if (index === 5) {
            closeOverlay()
            toggleViewStyle()
        } else {
            closeOverlay()
        }
    }

    function handleNavigation(action) {
        if (!active)
            return false
        forceActiveFocus()
        noteActivity()
        if (attractOpen) {
            if (action === "back") {
                stopAttractMode()
            } else if (action === "left" || action === "up") {
                moveAttract(-1)
            } else if (action === "right" || action === "down") {
                moveAttract(1)
            } else if (action === "page_left") {
                moveAttract(-5)
            } else if (action === "page_right") {
                moveAttract(5)
            } else if (action === "home") {
                shelf.currentIndex = 0
                shelf.positionViewAtBeginning()
                selectionDelay.restart()
                attractCycleTimer.restart()
                attractProgressAnimation.restart()
            } else if (action === "accept" || action === "menu") {
                stopAttractMode()
                openOverlay("menu")
            } else if (action === "details") {
                stopAttractMode()
                openOverlay("details")
            } else if (action === "favorite") {
                if (!favoriteBusy && selectedGameId.length > 0)
                    library.set_favorite(selectedGameId, !favorite)
            } else {
                return false
            }
            return true
        }
        if (launchStatusOverlayOpen) {
            if (action === "back") {
                launchStatusOverlayOpen = false
                forceActiveFocus()
            } else if (action === "details") {
                launchStatusOverlayOpen = false
                requestDetails()
            } else if (action === "menu") {
                launchStatusOverlayOpen = false
                openOverlay("menu")
            } else if (action === "accept") {
                launchStatusOverlayOpen = false
                forceActiveFocus()
            } else {
                return false
            }
            return true
        }
        if (platformWheelOpen) {
            if (action === "back") {
                closePlatformWheel()
            } else if (action === "up" || action === "left") {
                movePlatformWheel(-1)
            } else if (action === "down" || action === "right") {
                movePlatformWheel(1)
            } else if (action === "page_left") {
                movePlatformWheel(-5)
            } else if (action === "page_right") {
                movePlatformWheel(5)
            } else if (action === "home") {
                platformWheelIndex = 0
                platformWheel.currentIndex = 0
                platformWheel.positionViewAtBeginning()
            } else if (action === "accept") {
                choosePlatform(platformWheelIndex)
            } else {
                return false
            }
            return true
        }
        if (variantWheelOpen) {
            if (action === "back") {
                closeVariantWheel()
            } else if (action === "up" || action === "left") {
                moveVariantWheel(-1)
            } else if (action === "down" || action === "right") {
                moveVariantWheel(1)
            } else if (action === "page_left") {
                moveVariantWheel(-5)
            } else if (action === "page_right") {
                moveVariantWheel(5)
            } else if (action === "home") {
                variantWheelIndex = 0
                variantWheel.currentIndex = 0
                variantWheel.positionViewAtBeginning()
            } else if (action === "accept") {
                chooseVariant(variantWheelIndex)
            } else {
                return false
            }
            return true
        }
        if (collectionWheelOpen) {
            if (action === "back") {
                closeCollectionWheel()
            } else if (action === "up" || action === "left") {
                moveCollectionWheel(-1)
            } else if (action === "down" || action === "right") {
                moveCollectionWheel(1)
            } else if (action === "page_left") {
                moveCollectionWheel(-5)
            } else if (action === "page_right") {
                moveCollectionWheel(5)
            } else if (action === "home") {
                collectionWheelIndex = 0
                collectionWheel.currentIndex = library.collection_count > 0 ? 0 : -1
                if (collectionWheel.currentIndex >= 0)
                    collectionWheel.positionViewAtBeginning()
            } else if (action === "accept" && library.collection_count > 0) {
                chooseCollection(collectionWheelIndex)
            } else {
                return false
            }
            return true
        }
        if (overlayOpen) {
            if (action === "back") {
                closeOverlay()
            } else if (action === "details" || action === "page_left"
                       || action === "left") {
                overlayMode = "details"
                detailsScroller.contentY = 0
            } else if (action === "menu" || action === "page_right"
                       || action === "right") {
                overlayMode = "menu"
            } else if (action === "up") {
                if (overlayMode === "menu")
                    menuActionIndex = nextMenuAction(menuActionIndex, -1)
                else
                    detailsScroller.contentY = Math.max(0,
                                                        detailsScroller.contentY - 90)
            } else if (action === "down") {
                if (overlayMode === "menu")
                    menuActionIndex = nextMenuAction(menuActionIndex, 1)
                else
                    detailsScroller.contentY = Math.min(
                                Math.max(0, detailsScroller.contentHeight
                                         - detailsScroller.height),
                                detailsScroller.contentY + 90)
            } else if (action === "home") {
                if (overlayMode === "menu")
                    menuActionIndex = 0
                else
                    detailsScroller.contentY = 0
            } else if (action === "favorite") {
                if (!favoriteBusy && selectedGameId.length > 0)
                    library.set_favorite(selectedGameId, !favorite)
            } else if (action === "accept") {
                if (overlayMode === "menu")
                    activateMenuAction(menuActionIndex)
                else
                    overlayMode = "menu"
            } else {
                return false
            }
            return true
        }
        if (action === "back") {
            exitRequested()
        } else if (action === "up") {
            if (cinematicWheel && navigationZone === 2)
                moveShelf(-1)
            else
                navigationZone = Math.max(0, navigationZone - 1)
        } else if (action === "down") {
            if (cinematicWheel && navigationZone === 2)
                moveShelf(1)
            else
                navigationZone = Math.min(2, navigationZone + 1)
        } else if (action === "left") {
            if (navigationZone === 0)
                categoryIndex = Math.max(0, categoryIndex - 1)
            else if (navigationZone === 1)
                actionIndex = Math.max(0, actionIndex - 1)
            else if (cinematicWheel)
                navigationZone = 1
            else
                moveShelf(-1)
        } else if (action === "right") {
            if (navigationZone === 0)
                categoryIndex = Math.min(categories.length - 1,
                                         categoryIndex + 1)
            else if (navigationZone === 1) {
                if (cinematicWheel && actionIndex >= 2)
                    navigationZone = 2
                else
                    actionIndex = Math.min(2, actionIndex + 1)
            } else if (!cinematicWheel) {
                moveShelf(1)
            }
        } else if (action === "page_left") {
            navigationZone = 2
            moveShelf(-5)
        } else if (action === "page_right") {
            navigationZone = 2
            moveShelf(5)
        } else if (action === "home") {
            navigationZone = 2
            shelf.currentIndex = shelf.count > 0 ? 0 : -1
        } else if (action === "accept") {
            if (navigationZone === 0)
                chooseCategory(categoryIndex)
            else if (navigationZone === 1)
                activateAction(actionIndex)
            else
                activateAction(0)
        } else if (action === "favorite") {
            if (!favoriteBusy && selectedGameId.length > 0)
                library.set_favorite(selectedGameId, !favorite)
        } else if (action === "details") {
            openOverlay("details")
        } else if (action === "menu") {
            openOverlay("menu")
        } else if (action === "cycle_zone") {
            navigationZone = (navigationZone + 1) % 3
        } else {
            return false
        }
        return true
    }

    onActiveChanged: {
        if (active) {
            overlayOpen = false
            platformWheelOpen = false
            collectionWheelOpen = false
            variantWheelOpen = false
            attractOpen = false
            launchStatusOverlayOpen = details.game_running || details.launch_busy
            forceActiveFocus()
            syncCategory()
            const preferredRow = library.row_for_game(preferredGameId)
            if (preferredRow >= 0)
                shelf.currentIndex = preferredRow
            else if (shelf.currentIndex < 0 && shelf.count > 0)
                shelf.currentIndex = 0
            if (shelf.currentIndex >= 0)
                shelf.positionViewAtIndex(shelf.currentIndex, ListView.Center)
            selectionDelay.restart()
        } else {
            overlayOpen = false
            platformWheelOpen = false
            collectionWheelOpen = false
            variantWheelOpen = false
            attractOpen = false
            launchStatusOverlayOpen = false
        }
    }

    onSelectedGameIdChanged: {
        if (selectedGameId.length === 0 || !detailsCurrent)
            launchStatusOverlayOpen = false
    }

    Connections {
        target: view.details
        function onLaunch_busyChanged() {
            if (view.details.launch_busy)
                view.launchStatusOverlayOpen = true
        }
        function onGame_runningChanged() {
            if (view.details.game_running)
                view.launchStatusOverlayOpen = true
        }
    }

    onCurrentFilterKeyChanged: syncCategory()
    onCurrentPlatformNameChanged: syncCategory()

    onMediaRevisionChanged: {
        if (selectedDatabaseId > 0) {
            library.request_priority_artwork(selectedDatabaseId, selectedTitle,
                                             selectedPlatform, "box-front")
            library.request_priority_artwork(selectedDatabaseId, selectedTitle,
                                             selectedPlatform, "fanart")
        }
    }

    Keys.onPressed: event => {
        if (!active)
            return
        let action = ""
        if (event.key === Qt.Key_Escape || event.key === Qt.Key_Back) {
            action = "back"
        } else if (event.key === Qt.Key_Up) {
            action = "up"
        } else if (event.key === Qt.Key_Down) {
            action = "down"
        } else if (event.key === Qt.Key_Left) {
            action = "left"
        } else if (event.key === Qt.Key_Right) {
            action = "right"
        } else if (event.key === Qt.Key_PageUp) {
            action = "page_left"
        } else if (event.key === Qt.Key_PageDown) {
            action = "page_right"
        } else if (event.key === Qt.Key_Home) {
            action = "home"
        } else if (event.key === Qt.Key_End) {
            if (attractOpen) {
                shelf.currentIndex = Math.max(0, shelf.count - 1)
                shelf.positionViewAtEnd()
                selectionDelay.restart()
                attractCycleTimer.restart()
                attractProgressAnimation.restart()
            } else if (platformWheelOpen) {
                platformWheelIndex = Math.max(0, library.platform_count - 1)
                platformWheel.currentIndex = platformWheelIndex
                platformWheel.positionViewAtEnd()
            } else if (collectionWheelOpen) {
                collectionWheelIndex = Math.max(0, library.collection_count - 1)
                collectionWheel.currentIndex = collectionWheelIndex
                if (collectionWheel.currentIndex >= 0)
                    collectionWheel.positionViewAtEnd()
            } else if (variantWheelOpen) {
                variantWheelIndex = Math.max(0, view.details.variant_count - 1)
                variantWheel.currentIndex = variantWheelIndex
                variantWheel.positionViewAtEnd()
            } else {
                navigationZone = 2
                shelf.currentIndex = shelf.count - 1
            }
            event.accepted = true
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                   || event.key === Qt.Key_Space) {
            action = "accept"
        } else if (event.key === Qt.Key_F && !favoriteBusy) {
            action = "favorite"
        } else if (event.key === Qt.Key_D) {
            action = "details"
        } else if (event.key === Qt.Key_M) {
            action = "menu"
        } else if (event.key === Qt.Key_A) {
            if (attractOpen)
                action = "back"
            else {
                event.accepted = startAttractMode("manual")
                return
            }
        } else if (event.key === Qt.Key_V
                   && !overlayOpen && !attractOpen
                   && !platformWheelOpen && !collectionWheelOpen
                   && !variantWheelOpen && !launchStatusOverlayOpen) {
            event.accepted = toggleViewStyle()
            return
        } else if (event.key === Qt.Key_Tab) {
            action = "cycle_zone"
        }
        if (action.length > 0)
            event.accepted = handleNavigation(action)
    }

    Rectangle {
        anchors.fill: parent
        color: view.background
    }

    Image {
        id: themeBackgroundImage
        anchors.fill: parent
        source: view.library.couch_theme_background_image
        asynchronous: true
        cache: true
        autoTransform: true
        fillMode: Image.PreserveAspectCrop
        opacity: status === Image.Ready ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 220 } }
    }

    Image {
        id: heroImage
        anchors.fill: parent
        source: view.heroUrl
        asynchronous: true
        cache: true
        autoTransform: true
        fillMode: Image.PreserveAspectCrop
        sourceSize.width: Math.max(1, Math.round(width * 1.35))
        sourceSize.height: Math.max(1, Math.round(height * 1.35))
        opacity: status === Image.Ready ? 0.78 : 0
        Behavior on opacity { NumberAnimation { duration: 220 } }
    }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: view.withAlpha(view.background, Math.min(0.98, view.heroScrimOpacity + 0.28)) }
            GradientStop { position: 0.43; color: view.withAlpha(view.background, Math.min(0.94, view.heroScrimOpacity + 0.17)) }
            GradientStop { position: 0.75; color: view.withAlpha(view.background, view.heroScrimOpacity * 0.72) }
            GradientStop { position: 1.0; color: view.withAlpha(view.background, Math.min(0.86, view.heroScrimOpacity + 0.08)) }
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: parent.height * 0.43
        gradient: Gradient {
            GradientStop { position: 0.0; color: view.withAlpha(view.background, 0) }
            GradientStop { position: 0.35; color: view.withAlpha(view.background, 0.72) }
            GradientStop { position: 1.0; color: view.background }
        }
    }

    Row {
        id: brand
        anchors.left: parent.left
        anchors.leftMargin: 54
        anchors.top: parent.top
        anchors.topMargin: 34
        spacing: 13

        Rectangle {
            width: 40
            height: 40
            radius: Math.max(8, view.cardRadius - 4)
            color: view.accent
            Text {
                anchors.centerIn: parent
                text: "L"
                color: view.background
                font.pixelSize: 23
                font.weight: Font.Black
            }
        }
        Column {
            anchors.verticalCenter: parent.verticalCenter
            spacing: -1
            Text {
                text: "LUNCHBOX"
                color: view.ink
                font.pixelSize: 18
                font.weight: Font.Black
                font.letterSpacing: 1.8
            }
            Text {
                text: "COUCH MODE"
                color: view.muted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 2.0
            }
        }
    }

    Row {
        id: categoryRow
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: brand.verticalCenter
        spacing: 12

        Repeater {
            model: view.categories
            delegate: Rectangle {
                id: categoryButton
                required property int index
                required property var modelData
                width: categoryLabel.implicitWidth + 28
                height: 36
                radius: Math.max(8, view.cardRadius - 6)
                color: view.categoryIndex === index
                       ? (view.navigationZone === 0 ? view.withAlpha(view.accent, 0.9)
                                                    : view.withAlpha(view.panelRaised, 0.78))
                       : categoryHover.hovered ? view.withAlpha(view.panelRaised, 0.58)
                                               : "transparent"
                border.color: view.categoryIndex === index
                              ? view.accent : "transparent"
                border.width: 1

                Text {
                    id: categoryLabel
                    anchors.centerIn: parent
                    text: categoryButton.modelData.label
                    color: view.categoryIndex === categoryButton.index
                           && view.navigationZone === 0 ? view.background
                           : view.categoryIndex === categoryButton.index
                             ? view.ink : view.muted
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }
                HoverHandler { id: categoryHover }
                TapHandler {
                    onTapped: {
                        view.navigationZone = 0
                        view.chooseCategory(categoryButton.index)
                        view.forceActiveFocus()
                    }
                }
            }
        }
    }

    Row {
        anchors.right: parent.right
        anchors.rightMargin: 50
        anchors.verticalCenter: brand.verticalCenter
        spacing: 10

        Rectangle {
            visible: view.width >= 1500
            width: visible ? viewStyleLabel.implicitWidth + 34 : 0
            height: 40
            radius: Math.max(8, view.cardRadius - 4)
            color: viewStyleHover.hovered
                   ? view.withAlpha(view.panelRaised, 0.82)
                   : view.withAlpha(view.panel, 0.72)
            border.color: view.navigationZone === 2
                          ? view.withAlpha(view.accent, 0.72)
                          : view.withAlpha(view.muted, 0.48)

            Text {
                id: viewStyleLabel
                anchors.centerIn: parent
                text: view.cinematicWheel ? "▦  COVER SHELF" : "☷  CINEMATIC WHEEL"
                color: view.ink
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.7
            }
            HoverHandler { id: viewStyleHover }
            TapHandler {
                onTapped: {
                    view.noteActivity()
                    view.toggleViewStyle()
                }
            }
            Accessible.role: Accessible.Button
            Accessible.name: view.cinematicWheel
                             ? "Use Couch Mode cover shelf"
                             : "Use Couch Mode cinematic wheel"
        }

        Rectangle {
            visible: view.gamepad.ready
            width: visible ? gamepadStatus.implicitWidth + 26 : 0
            height: 40
            radius: Math.max(8, view.cardRadius - 4)
            color: view.withAlpha(view.panel, 0.72)
            border.color: view.gamepad.connected_count > 0
                          ? view.accentCool
                          : view.gamepad.available ? view.withAlpha(view.muted, 0.48)
                                                   : view.danger
            Text {
                id: gamepadStatus
                anchors.centerIn: parent
                text: view.gamepad.connected_count > 0
                      ? "●  " + view.gamepad.connected_count + " CONTROLLER"
                      : view.gamepad.available ? "○  CONTROLLER READY"
                        : "!  GAMEPAD UNAVAILABLE"
                color: view.gamepad.connected_count > 0
                       ? view.accentCool : view.muted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
        }

        Rectangle {
            width: 40
            height: 40
            radius: Math.max(8, view.cardRadius - 4)
            color: exitHover.hovered ? view.withAlpha(view.panelRaised, 0.72)
                                     : view.withAlpha(view.panel, 0.72)
            border.color: exitHover.hovered ? view.accent
                                            : view.withAlpha(view.muted, 0.48)
            Text {
                anchors.centerIn: parent
                text: "×"
                color: view.ink
                font.pixelSize: 23
            }
            HoverHandler { id: exitHover }
            TapHandler { onTapped: view.exitRequested() }
        }
    }

    Column {
        id: gameCopy
        anchors.left: parent.left
        anchors.leftMargin: 70
        anchors.top: brand.bottom
        anchors.topMargin: Math.max(42, parent.height * 0.055)
        width: Math.min(760, parent.width * 0.49)
        spacing: 14

        Row {
            spacing: 9
            Rectangle {
                width: platformText.implicitWidth + 18
                height: 26
                radius: Math.max(6, view.cardRadius - 8)
                color: view.withAlpha(view.panelRaised, 0.72)
                border.color: view.withAlpha(view.muted, 0.48)
                Text {
                    id: platformText
                    anchors.centerIn: parent
                    text: view.selectedPlatform.length > 0
                          ? view.selectedPlatform.toUpperCase() : "CATALOG"
                    color: view.ink
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }
            }
            Rectangle {
                width: stateText.implicitWidth + 18
                height: 26
                radius: Math.max(6, view.cardRadius - 8)
                color: view.selectedLocal ? view.withAlpha(view.accentCool, 0.24)
                       : view.selectedDownloadable ? view.withAlpha(view.accent, 0.24)
                                                   : view.withAlpha(view.panelRaised, 0.72)
                border.color: view.selectedLocal ? view.accentCool
                              : view.selectedDownloadable ? view.accent
                                                          : view.withAlpha(view.muted, 0.48)
                Text {
                    id: stateText
                    anchors.centerIn: parent
                    text: view.selectedLocal ? "INSTALLED"
                          : view.selectedDownloadable ? "MINERVA" : "CATALOG"
                    color: view.selectedLocal ? view.accentCool
                           : view.selectedDownloadable ? view.accent : view.muted
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 0.9
                }
            }
        }

        Text {
            width: parent.width
            text: view.selectedTitle.length > 0 ? view.selectedTitle : "Choose a game"
            color: view.ink
            font.pixelSize: Math.max(34, Math.min(54, view.width * 0.035))
            font.weight: Font.Black
            lineHeight: 0.94
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
        }

        Row {
            spacing: 18
            visible: !view.details.loading && view.selectedGameId.length > 0
            Text {
                visible: view.details.release_date.length > 0
                text: view.details.release_date
                color: view.muted
                font.pixelSize: 13
                font.weight: Font.Medium
            }
            Text {
                visible: view.details.genre.length > 0
                text: view.details.genre
                color: view.muted
                font.pixelSize: 13
                font.weight: Font.Medium
            }
            Text {
                visible: view.details.players.length > 0
                text: view.details.players + " players"
                color: view.muted
                font.pixelSize: 13
                font.weight: Font.Medium
            }
            Text {
                visible: view.details.cooperative === "yes"
                text: "CO-OP"
                color: view.accentCool
                font.pixelSize: 13
                font.weight: Font.DemiBold
            }
            Text {
                visible: view.details.rating.length > 0
                text: "★ " + view.details.rating
                color: view.accent
                font.pixelSize: 13
                font.weight: Font.DemiBold
            }
        }

        Text {
            width: parent.width
            height: Math.min(implicitHeight, 112)
            text: view.details.loading ? "Loading game details…"
                  : view.details.description.length > 0
                    ? view.details.description
                    : "Browse the catalog, play an installed game, or inspect its exact Minerva download options."
            color: view.withAlpha(view.ink, 0.86)
            font.pixelSize: 14
            lineHeight: 1.34
            wrapMode: Text.WordWrap
            maximumLineCount: 5
            elide: Text.ElideRight
        }

        Row {
            id: actionRow
            spacing: 10

            Repeater {
                model: 3
                delegate: Rectangle {
                    id: actionButton
                    required property int index
                    property bool selected: view.navigationZone === 1
                                            && view.actionIndex === index
                    width: index === 0 ? 190 : index === 1 ? 170 : 52
                    height: 50
                    radius: Math.max(8, view.cardRadius - 4)
                    color: index === 0
                           ? (selected ? Qt.lighter(view.accent, 1.12)
                                       : view.withAlpha(view.accent, 0.9))
                           : selected ? view.withAlpha(view.muted, 0.82)
                                      : view.withAlpha(view.panelRaised, 0.76)
                    border.color: selected ? view.ink : index === 0
                                  ? view.accent : view.withAlpha(view.muted, 0.62)
                    border.width: selected ? 2 : 1
                    opacity: index === 2 && view.favoriteBusy ? 0.55 : 1
                    scale: selected ? 1.035 : actionHover.hovered ? 1.018 : 1
                    Behavior on scale { NumberAnimation { duration: 100 } }

                    Text {
                        anchors.centerIn: parent
                        text: actionButton.index === 0 ? view.primaryAction
                              : actionButton.index === 1 ? "DESKTOP DETAILS"
                              : view.favoriteBusy ? "…" : view.favorite ? "★" : "☆"
                        color: actionButton.index === 0 ? view.background : view.ink
                        font.pixelSize: actionButton.index === 2 ? 22 : 11
                        font.weight: Font.Bold
                        font.letterSpacing: actionButton.index === 2 ? 0 : 0.7
                    }
                    HoverHandler { id: actionHover }
                    TapHandler {
                        enabled: actionButton.index !== 2 || !view.favoriteBusy
                        onTapped: {
                            view.navigationZone = 1
                            view.actionIndex = actionButton.index
                            view.activateAction(actionButton.index)
                            view.forceActiveFocus()
                        }
                    }
                }
            }
        }

        Text {
            width: parent.width
            visible: view.selectedLocal && view.details.launch_status.length > 0
            text: view.details.launch_status
            color: view.details.can_launch ? view.accentCool : view.muted
            font.pixelSize: 11
            maximumLineCount: 2
            elide: Text.ElideRight
            wrapMode: Text.WordWrap
        }
    }

    Rectangle {
        id: coverFrame
        visible: !view.cinematicWheel
        anchors.right: parent.right
        anchors.rightMargin: Math.max(74, parent.width * 0.075)
        anchors.top: brand.bottom
        anchors.topMargin: 58
        width: Math.min(330, parent.width * 0.21)
        height: width * 1.38
        radius: Math.min(32, view.cardRadius + 2)
        color: view.accentFor(view.selectedTitle)
        border.color: view.withAlpha(view.ink, 0.55)
        border.width: 1
        clip: true
        rotation: 1.5

        gradient: Gradient {
            GradientStop { position: 0; color: Qt.lighter(coverFrame.color, 1.2) }
            GradientStop { position: 1; color: Qt.darker(coverFrame.color, 1.65) }
        }

        Image {
            id: selectedCover
            anchors.fill: parent
            anchors.margins: 2
            source: view.coverUrl
            asynchronous: true
            cache: true
            mipmap: true
            autoTransform: true
            fillMode: Image.PreserveAspectFit
            sourceSize.width: Math.max(1, Math.round(width * 1.6))
            sourceSize.height: Math.max(1, Math.round(height * 1.6))
            opacity: status === Image.Ready ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: 160 } }
        }
        Text {
            anchors.centerIn: parent
            text: view.selectedTitle.length > 0
                  ? view.selectedTitle.charAt(0).toUpperCase() : "L"
            color: view.withAlpha(view.ink, 0.87)
            font.pixelSize: 104
            font.weight: Font.Black
            visible: selectedCover.status !== Image.Ready
        }
    }

    Item {
        id: shelfArea
        x: view.cinematicWheel ? parent.width - width - 44 : 0
        y: view.cinematicWheel ? brand.y + brand.height + 58
                               : footer.y - height
        width: view.cinematicWheel ? Math.min(650, parent.width * 0.42)
                                   : parent.width
        height: view.cinematicWheel
                ? Math.max(320, footer.y - y - 18)
                : Math.max(250, parent.height * 0.265)

        Text {
            anchors.left: parent.left
            anchors.leftMargin: view.cinematicWheel ? 8 : 70
            anchors.top: parent.top
            text: view.library.filtering ? "UPDATING…"
                  : view.currentShelfLabel + "  ·  "
                    + view.library.filtered_count + " GAMES"
            color: view.navigationZone === 2 ? view.accent : view.muted
            font.pixelSize: 10
            font.weight: Font.Bold
            font.letterSpacing: 1.4
        }

        CouchGameShelf {
            id: shelf
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.topMargin: 25
            anchors.bottom: parent.bottom
            leftMargin: view.cinematicWheel ? 8 : 70
            rightMargin: view.cinematicWheel ? 0 : 70
            topMargin: view.cinematicWheel ? 8 : 0
            bottomMargin: view.cinematicWheel ? 8 : 0
            library: view.library
            background: view.background
            panel: view.panel
            panelRaised: view.panelRaised
            ink: view.ink
            muted: view.muted
            accent: view.accent
            accentCool: view.accentCool
            cardRadius: view.cardRadius
            cinematic: view.cinematicWheel
            navigationActive: view.navigationZone === 2

            onCurrentGameChanged: {
                if (!view.active)
                    return
                view.captureCurrentGame()
                selectionDelay.restart()
            }
            onCardActivated: index => {
                view.noteActivity()
                view.navigationZone = 2
                shelf.currentIndex = index
                view.forceActiveFocus()
            }
        }
    }

    Row {
        id: footer
        anchors.left: parent.left
        anchors.leftMargin: 70
        anchors.right: parent.right
        anchors.rightMargin: 70
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 18
        height: 28
        spacing: 22

        Text {
            text: view.gamepad.connected_count > 0
                  ? "D-PAD / STICK  MOVE"
                  : view.cinematicWheel ? "↑ ↓  BROWSE" : "← →  BROWSE"
            color: view.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
        Text {
            text: view.gamepad.connected_count > 0
                  ? view.gamepad.button_label("accept") + "  SELECT"
                  : view.cinematicWheel ? "← →  MOVE" : "↑ ↓  MOVE"
            color: view.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
        Text {
            text: view.gamepad.connected_count > 0
                  ? view.gamepad.button_label("favorite") + "  FAVORITE"
                  : "ENTER  SELECT"
            color: view.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
        Text {
            text: view.gamepad.connected_count > 0
                  ? view.gamepad.button_label("details") + "  DETAILS"
                  : "F  FAVORITE"
            color: view.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
        Text {
            visible: view.gamepad.connected_count === 0
            text: "A  ATTRACT    V  CHANGE VIEW"
            color: view.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
        Item { width: Math.max(0, parent.width - 760); height: 1 }
        Text {
            text: view.gamepad.connected_count > 0
                  ? view.gamepad.button_label("back") + "  DESKTOP MODE"
                  : "ESC  DESKTOP MODE"
            color: view.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
    }

    Rectangle {
        id: attractOverlay
        anchors.fill: parent
        z: 42
        visible: view.attractOpen
        color: view.background

        Image {
            anchors.fill: parent
            source: view.library.couch_theme_background_image
            asynchronous: true
            cache: true
            autoTransform: true
            fillMode: Image.PreserveAspectCrop
            opacity: status === Image.Ready ? 1 : 0
        }

        Image {
            anchors.fill: parent
            source: view.heroUrl
            asynchronous: true
            cache: true
            autoTransform: true
            fillMode: Image.PreserveAspectCrop
            sourceSize.width: Math.max(1, Math.round(width * 1.4))
            sourceSize.height: Math.max(1, Math.round(height * 1.4))
            opacity: status === Image.Ready ? 0.88 : 0
            Behavior on opacity { NumberAnimation { duration: 420 } }
        }

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: view.withAlpha(view.background, Math.min(0.99, view.heroScrimOpacity + 0.3)) }
                GradientStop { position: 0.46; color: view.withAlpha(view.background, Math.min(0.9, view.heroScrimOpacity + 0.1)) }
                GradientStop { position: 0.78; color: view.withAlpha(view.background, view.heroScrimOpacity * 0.42) }
                GradientStop { position: 1; color: view.withAlpha(view.background, Math.min(0.8, view.heroScrimOpacity + 0.04)) }
            }
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: parent.height * 0.58
            gradient: Gradient {
                GradientStop { position: 0; color: view.withAlpha(view.background, 0) }
                GradientStop { position: 0.34; color: view.withAlpha(view.background, 0.72) }
                GradientStop { position: 1; color: view.background }
            }
        }

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 62
            anchors.top: parent.top
            anchors.topMargin: 42
            spacing: 13

            Rectangle {
                width: 42
                height: 42
                radius: Math.max(9, view.cardRadius - 3)
                color: view.accent
                Text {
                    anchors.centerIn: parent
                    text: "L"
                    color: view.background
                    font.pixelSize: 21
                    font.weight: Font.Black
                }
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                Text {
                    text: "LUNCHBOX"
                    color: view.ink
                    font.pixelSize: 17
                    font.weight: Font.Black
                    font.letterSpacing: 1.8
                }
                Text {
                    text: "ATTRACT MODE"
                    color: view.accent
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 1.4
                }
            }
        }

        Rectangle {
            anchors.top: parent.top
            anchors.topMargin: 46
            anchors.right: parent.right
            anchors.rightMargin: 62
            width: attractModeLabel.implicitWidth + 30
            height: 34
            radius: Math.max(8, view.cardRadius - 5)
            color: view.withAlpha(view.panel, 0.74)
            border.color: view.withAlpha(view.muted, 0.52)
            Text {
                id: attractModeLabel
                anchors.centerIn: parent
                text: view.attractReason === "idle" ? "IDLE SCREENSAVER" : "MANUAL SHOWCASE"
                color: view.muted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1
            }
        }

        Rectangle {
            id: attractCoverShadow
            anchors.right: parent.right
            anchors.rightMargin: Math.max(74, parent.width * 0.075)
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 30
            width: Math.min(360, parent.width * 0.22)
            height: width * 1.38
            radius: Math.min(32, view.cardRadius + 8)
            color: view.withAlpha(view.background, 0.7)
            rotation: 2.5

            Rectangle {
                id: attractCoverSurface
                anchors.fill: parent
                anchors.margins: 10
                radius: Math.min(32, view.cardRadius + 4)
                color: view.accentFor(view.selectedTitle)
                border.color: view.withAlpha(view.ink, 0.55)
                clip: true
                gradient: Gradient {
                    GradientStop {
                        position: 0
                        color: Qt.lighter(attractCoverSurface.color, 1.2)
                    }
                    GradientStop {
                        position: 1
                        color: Qt.darker(attractCoverSurface.color, 1.65)
                    }
                }
                Image {
                    id: attractCoverImage
                    anchors.fill: parent
                    anchors.margins: 2
                    source: view.coverUrl
                    asynchronous: true
                    cache: true
                    mipmap: true
                    autoTransform: true
                    fillMode: Image.PreserveAspectFit
                    sourceSize.width: Math.max(1, Math.round(width * 1.5))
                    sourceSize.height: Math.max(1, Math.round(height * 1.5))
                    opacity: status === Image.Ready ? 1 : 0
                    Behavior on opacity { NumberAnimation { duration: 280 } }
                }
                Text {
                    anchors.centerIn: parent
                    visible: attractCoverImage.status !== Image.Ready
                    text: view.selectedTitle.length > 0
                          ? view.selectedTitle.charAt(0).toUpperCase() : "L"
                    color: view.withAlpha(view.ink, 0.85)
                    font.pixelSize: 92
                    font.weight: Font.Black
                }
            }
        }

        Column {
            anchors.left: parent.left
            anchors.leftMargin: Math.max(62, parent.width * 0.065)
            anchors.right: attractCoverShadow.left
            anchors.rightMargin: Math.max(70, parent.width * 0.055)
            anchors.bottom: attractFooter.top
            anchors.bottomMargin: 44
            spacing: 14

            Rectangle {
                width: 74
                height: 5
                radius: 3
                color: view.accent
            }
            Text {
                width: parent.width
                text: view.selectedPlatform.toUpperCase()
                color: view.accent
                font.pixelSize: 11
                font.weight: Font.Bold
                font.letterSpacing: 1.5
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: view.selectedTitle
                color: view.ink
                font.pixelSize: Math.max(38, Math.min(64, attractOverlay.width / 30))
                font.weight: Font.Black
                lineHeight: 0.98
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }
            Text {
                width: Math.min(parent.width, 850)
                visible: view.details.description.length > 0
                text: view.details.description
                color: view.withAlpha(view.ink, 0.84)
                font.pixelSize: 15
                lineHeight: 1.28
                wrapMode: Text.WordWrap
                maximumLineCount: 3
                elide: Text.ElideRight
            }
            Row {
                spacing: 10
                Rectangle {
                    width: attractStateLabel.implicitWidth + 22
                    height: 30
                    radius: Math.max(8, view.cardRadius - 6)
                    color: view.selectedLocal ? view.withAlpha(view.accentCool, 0.24)
                           : view.selectedDownloadable ? view.withAlpha(view.accent, 0.24)
                                                       : view.withAlpha(view.panelRaised, 0.76)
                    border.color: view.selectedLocal ? view.accentCool
                                  : view.selectedDownloadable ? view.accent
                                                              : view.withAlpha(view.muted, 0.58)
                    Text {
                        id: attractStateLabel
                        anchors.centerIn: parent
                        text: view.details.can_launch ? "READY TO PLAY"
                              : view.selectedLocal ? "SETUP REQUIRED"
                              : view.selectedDownloadable ? "MINERVA AVAILABLE"
                                : "CATALOG"
                        color: view.details.can_launch || view.selectedLocal
                               ? view.accentCool
                               : view.selectedDownloadable ? view.accent : view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                }
                Rectangle {
                    visible: view.favorite
                    width: attractFavoriteLabel.implicitWidth + 22
                    height: 30
                    radius: Math.max(8, view.cardRadius - 6)
                    color: view.withAlpha(view.accent, 0.22)
                    border.color: view.accent
                    Text {
                        id: attractFavoriteLabel
                        anchors.centerIn: parent
                        text: "★ FAVORITE"
                        color: view.accent
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                }
            }
        }

        Rectangle {
            id: attractFooter
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 78
            color: view.withAlpha(view.panel, 0.86)

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 2
                color: view.withAlpha(view.muted, 0.5)
                Rectangle {
                    width: parent.width * view.attractProgress
                    height: parent.height
                    color: view.accent
                }
            }

            Row {
                anchors.left: parent.left
                anchors.leftMargin: 62
                anchors.verticalCenter: parent.verticalCenter
                spacing: 28
                Text {
                    text: view.gamepad.connected_count > 0
                          ? "D-PAD / STICK  NEXT GAME" : "← →  NEXT GAME"
                    color: view.muted
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }
                Text {
                    text: view.gamepad.connected_count > 0
                          ? view.gamepad.button_label("accept") + "  GAME MENU"
                          : "ENTER  GAME MENU"
                    color: view.ink
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }
            }

            Text {
                anchors.right: parent.right
                anchors.rightMargin: 62
                anchors.verticalCenter: parent.verticalCenter
                text: view.gamepad.connected_count > 0
                      ? view.gamepad.button_label("back") + "  RETURN TO BROWSING"
                      : "ESC  RETURN TO BROWSING"
                color: view.muted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
        }

        TapHandler {
            onTapped: {
                view.stopAttractMode()
                view.openOverlay("menu")
            }
        }
    }

    Rectangle {
        id: launchStatusOverlay
        anchors.fill: parent
        z: 60
        visible: view.launchStatusOverlayOpen && view.detailsCurrent
        color: view.withAlpha(view.background, 0.95)

        Image {
            anchors.fill: parent
            source: view.library.couch_theme_background_image
            asynchronous: true
            cache: true
            autoTransform: true
            fillMode: Image.PreserveAspectCrop
            opacity: status === Image.Ready ? 0.38 : 0
        }

        Image {
            anchors.fill: parent
            source: view.heroUrl
            asynchronous: true
            cache: true
            autoTransform: true
            fillMode: Image.PreserveAspectCrop
            sourceSize.width: Math.max(1, Math.round(width * 1.25))
            sourceSize.height: Math.max(1, Math.round(height * 1.25))
            opacity: status === Image.Ready ? 0.42 : 0
        }

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: view.withAlpha(view.background, 0.98) }
                GradientStop { position: 0.54; color: view.withAlpha(view.background, 0.91) }
                GradientStop { position: 1; color: view.withAlpha(view.background, 0.76) }
            }
        }

        Rectangle {
            id: launchStatusPanel
            anchors.centerIn: parent
            width: Math.min(1060, parent.width - 170)
            height: Math.min(610, parent.height - 170)
            radius: Math.min(30, view.cardRadius + 8)
            color: view.withAlpha(view.panel, 0.97)
            border.color: view.withAlpha(view.muted, 0.58)
            border.width: 1
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: 330
                color: view.withAlpha(view.background, 0.7)

                Rectangle {
                    id: launchStatusCover
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: 40
                    width: 210
                    height: 292
                    radius: view.cardRadius
                    color: view.accentFor(view.selectedTitle)
                    border.color: view.withAlpha(view.ink, 0.55)
                    clip: true
                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.lighter(launchStatusCover.color, 1.18) }
                        GradientStop { position: 1; color: Qt.darker(launchStatusCover.color, 1.62) }
                    }
                    Image {
                        id: launchStatusCoverImage
                        anchors.fill: parent
                        anchors.margins: 2
                        source: view.coverUrl
                        asynchronous: true
                        cache: true
                        mipmap: true
                        autoTransform: true
                        fillMode: Image.PreserveAspectFit
                        sourceSize.width: 420
                        sourceSize.height: 584
                        opacity: status === Image.Ready ? 1 : 0
                    }
                    Text {
                        anchors.centerIn: parent
                        text: view.selectedTitle.length > 0
                              ? view.selectedTitle.charAt(0).toUpperCase() : "L"
                        color: view.withAlpha(view.ink, 0.86)
                        font.pixelSize: 78
                        font.weight: Font.Black
                        visible: launchStatusCoverImage.status !== Image.Ready
                    }
                }

                Column {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: launchStatusCover.bottom
                    anchors.topMargin: 24
                    anchors.margins: 32
                    spacing: 7
                    Text {
                        width: parent.width
                        text: view.selectedTitle
                        color: view.ink
                        font.pixelSize: 23
                        font.weight: Font.Black
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                    }
                    Text {
                        width: parent.width
                        text: view.selectedPlatform.toUpperCase()
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 1.1
                        elide: Text.ElideRight
                    }
                }
            }

            Column {
                anchors.left: parent.left
                anchors.leftMargin: 378
                anchors.right: parent.right
                anchors.rightMargin: 46
                anchors.top: parent.top
                anchors.topMargin: 48
                spacing: 16

                Row {
                    spacing: 12
                    Text {
                        text: view.details.launch_busy ? "STARTING GAME"
                              : view.details.game_running ? "NOW PLAYING"
                                                           : "SESSION COMPLETE"
                        color: view.details.game_running ? view.accentCool : view.accent
                        font.pixelSize: 13
                        font.weight: Font.Bold
                        font.letterSpacing: 1.8
                    }
                    Rectangle {
                        width: 9
                        height: 9
                        radius: 5
                        visible: view.details.launch_busy
                        color: view.accent
                        anchors.verticalCenter: parent.verticalCenter
                        RotationAnimation on rotation {
                            running: view.details.launch_busy
                            from: 0
                            to: 360
                            duration: 900
                            loops: Animation.Infinite
                        }
                    }
                }
                Text {
                    width: parent.width
                    text: view.details.launch_busy
                          ? "Preparing the exact emulator command and writable runtime files."
                          : view.details.game_running
                            ? "The emulator is running independently. Couch Mode is ready when you return."
                            : "The emulator session has ended and play activity has been saved."
                    color: view.ink
                    font.pixelSize: 29
                    font.weight: Font.Black
                    lineHeight: 0.98
                    wrapMode: Text.WordWrap
                }

                Rectangle {
                    width: parent.width
                    height: 102
                    radius: Math.max(10, view.cardRadius - 2)
                    color: view.withAlpha(view.panelRaised, 0.52)
                    border.color: view.withAlpha(view.muted, 0.38)
                    Column {
                        anchors.fill: parent
                        anchors.margins: 17
                        spacing: 9
                        Text {
                            text: "LAUNCH STATUS"
                            color: view.muted
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 1.1
                        }
                        Text {
                            width: parent.width
                            text: view.details.launch_status.length > 0
                                  ? view.details.launch_status : "Waiting for the emulator supervisor…"
                            color: view.details.game_running ? view.accentCool : view.ink
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                            wrapMode: Text.WordWrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                    }
                }

                Row {
                    spacing: 10
                    Rectangle {
                        width: emulatorLabel.implicitWidth + 24
                        height: 32
                        radius: Math.max(8, view.cardRadius - 6)
                        color: view.withAlpha(view.accentCool, 0.16)
                        border.color: view.withAlpha(view.accentCool, 0.62)
                        Text {
                            id: emulatorLabel
                            anchors.centerIn: parent
                            text: view.details.emulator_name.length > 0
                                  ? view.details.emulator_name : "EMULATOR SUPERVISOR"
                            color: view.accentCool
                            font.pixelSize: 9
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                            elide: Text.ElideRight
                        }
                    }
                    Rectangle {
                        visible: view.details.game_running
                        width: runningLabel.implicitWidth + 24
                        height: 32
                        radius: Math.max(8, view.cardRadius - 6)
                        color: view.withAlpha(view.accent, 0.16)
                        border.color: view.withAlpha(view.accent, 0.62)
                        Text {
                            id: runningLabel
                            anchors.centerIn: parent
                            text: "PROCESS ACTIVE"
                            color: view.accent
                            font.pixelSize: 9
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8
                        }
                    }
                }

                Item { width: 1; height: 1 }

                Row {
                    spacing: 10
                    Rectangle {
                        width: desktopLaunchAction.implicitWidth + 34
                        height: 48
                        radius: Math.max(9, view.cardRadius - 5)
                        color: desktopLaunchHover.hovered
                               ? view.withAlpha(view.panelRaised, 0.9)
                               : view.withAlpha(view.panelRaised, 0.68)
                        border.color: desktopLaunchHover.hovered ? view.accent
                                                                 : view.withAlpha(view.muted, 0.58)
                        Text {
                            id: desktopLaunchAction
                            anchors.centerIn: parent
                            text: "DESKTOP DETAILS"
                            color: view.ink
                            font.pixelSize: 10
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                        }
                        HoverHandler { id: desktopLaunchHover }
                        TapHandler {
                            onTapped: {
                                view.launchStatusOverlayOpen = false
                                view.requestDetails()
                            }
                        }
                    }
                    Rectangle {
                        width: returnLaunchAction.implicitWidth + 34
                        height: 48
                        radius: Math.max(9, view.cardRadius - 5)
                        color: returnLaunchHover.hovered
                               ? view.withAlpha(view.accent, 0.92)
                               : view.withAlpha(view.accent, 0.78)
                        border.color: view.accent
                        Text {
                            id: returnLaunchAction
                            anchors.centerIn: parent
                            text: "RETURN TO BROWSING"
                            color: view.background
                            font.pixelSize: 10
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                        }
                        HoverHandler { id: returnLaunchHover }
                        TapHandler {
                            onTapped: {
                                view.launchStatusOverlayOpen = false
                                view.forceActiveFocus()
                            }
                        }
                    }
                }
            }

            Rectangle {
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.topMargin: 26
                anchors.rightMargin: 28
                width: 42
                height: 42
                radius: Math.max(8, view.cardRadius - 4)
                color: launchStatusCloseHover.hovered
                       ? view.withAlpha(view.panelRaised, 0.9)
                       : view.withAlpha(view.panelRaised, 0.62)
                border.color: launchStatusCloseHover.hovered ? view.accent
                                                               : view.withAlpha(view.muted, 0.54)
                Text {
                    anchors.centerIn: parent
                    text: "×"
                    color: view.ink
                    font.pixelSize: 22
                }
                HoverHandler { id: launchStatusCloseHover }
                TapHandler {
                    onTapped: {
                        view.launchStatusOverlayOpen = false
                        view.forceActiveFocus()
                    }
                }
            }

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 378
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 25
                text: view.gamepad.connected_count > 0
                      ? view.gamepad.button_label("accept") + "  CLOSE   ·   "
                        + view.gamepad.button_label("details") + "  DESKTOP DETAILS   ·   "
                        + view.gamepad.button_label("back") + "  CLOSE"
                      : "ENTER  CLOSE   ·   D  DESKTOP DETAILS   ·   ESC  CLOSE"
                color: view.muted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.7
            }
        }
    }

    Rectangle {
        id: platformWheelOverlay
        anchors.fill: parent
        z: 45
        visible: view.platformWheelOpen
        color: view.withAlpha(view.background, 0.93)

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: view.withAlpha(view.background, 0.96) }
                GradientStop { position: 0.55; color: view.withAlpha(view.panel, 0.89) }
                GradientStop { position: 1; color: view.withAlpha(view.background, 0.76) }
            }
        }

        Rectangle {
            id: platformWheelPanel
            anchors.centerIn: parent
            width: Math.min(880, parent.width - 180)
            height: Math.min(860, parent.height - 120)
            radius: Math.min(32, view.cardRadius + 10)
            color: view.withAlpha(view.panel, 0.97)
            border.color: view.withAlpha(view.muted, 0.58)
            border.width: 1
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 118
                color: view.withAlpha(view.panelRaised, 0.86)

                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: 42
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 5

                    Text {
                        text: "CHOOSE A PLATFORM"
                        color: view.ink
                        font.pixelSize: 24
                        font.weight: Font.Black
                        font.letterSpacing: 1.3
                    }
                    Text {
                        text: view.library.platform_count + " platforms · exact catalog filtering"
                        color: view.muted
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.4
                    }
                }

                Rectangle {
                    anchors.right: parent.right
                    anchors.rightMargin: 32
                    anchors.verticalCenter: parent.verticalCenter
                    width: 44
                    height: 44
                    radius: 13
                    color: platformCloseHover.hovered
                           ? view.withAlpha(view.panelRaised, 0.9)
                           : view.withAlpha(view.panel, 0.72)
                    border.color: platformCloseHover.hovered
                                  ? view.accent : view.withAlpha(view.muted, 0.55)
                    Text {
                        anchors.centerIn: parent
                        text: "×"
                        color: view.ink
                        font.pixelSize: 23
                    }
                    HoverHandler { id: platformCloseHover }
                    TapHandler { onTapped: view.closePlatformWheel() }
                }
            }

            ListView {
                id: platformWheel
                anchors.left: parent.left
                anchors.leftMargin: 34
                anchors.right: parent.right
                anchors.rightMargin: 34
                anchors.top: parent.top
                anchors.topMargin: 128
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 82
                model: view.library.platform_count
                clip: true
                reuseItems: true
                cacheBuffer: Math.max(0, Math.round(height))
                spacing: 6
                boundsBehavior: Flickable.StopAtBounds
                snapMode: ListView.SnapToItem
                preferredHighlightBegin: height * 0.5 - 42
                preferredHighlightEnd: height * 0.5 + 42
                highlightRangeMode: ListView.StrictlyEnforceRange
                highlightMoveDuration: 120
                onCurrentIndexChanged: {
                    if (currentIndex >= 0)
                        view.platformWheelIndex = currentIndex
                }

                delegate: Item {
                    id: platformRow
                    required property int index
                    property string platformName: view.library.platform_name_at(index)
                    property int gameCount: view.library.platform_game_count_at(index)
                    property bool selected: platformWheel.currentIndex === index
                    property int distance: Math.abs(index - platformWheel.currentIndex)
                    width: platformWheel.width
                    height: selected ? 84 : 70
                    opacity: selected ? 1 : distance <= 2 ? 0.74 : 0.42
                    scale: selected ? 1 : 0.975
                    Behavior on height { NumberAnimation { duration: 110 } }
                    Behavior on opacity { NumberAnimation { duration: 110 } }
                    Behavior on scale { NumberAnimation { duration: 110 } }

                    Rectangle {
                        anchors.fill: parent
                        anchors.leftMargin: platformRow.selected ? 0 : 16
                        anchors.rightMargin: platformRow.selected ? 0 : 16
                        radius: 16
                        color: platformRow.selected
                               ? view.withAlpha(view.panelRaised, 0.9)
                               : platformHover.hovered
                                 ? view.withAlpha(view.panelRaised, 0.55)
                                 : "transparent"
                        border.color: platformRow.selected ? view.accent : "transparent"
                        border.width: platformRow.selected ? 2 : 0

                        Rectangle {
                            anchors.left: parent.left
                            anchors.leftMargin: 16
                            anchors.verticalCenter: parent.verticalCenter
                            width: platformRow.selected ? 52 : 44
                            height: width
                            radius: 14
                            color: view.accentFor(platformRow.platformName)
                            border.color: view.withAlpha(view.ink, 0.51)
                            Text {
                                anchors.centerIn: parent
                                text: platformRow.platformName.length > 0
                                      ? platformRow.platformName.charAt(0).toUpperCase() : "?"
                                color: view.withAlpha(view.ink, 0.97)
                                font.pixelSize: platformRow.selected ? 23 : 19
                                font.weight: Font.Black
                            }
                        }

                        Column {
                            anchors.left: parent.left
                            anchors.leftMargin: platformRow.selected ? 88 : 78
                            anchors.right: countColumn.left
                            anchors.rightMargin: 24
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 4

                            Text {
                                width: parent.width
                                text: platformRow.platformName
                                color: view.ink
                                font.pixelSize: platformRow.selected ? 19 : 15
                                font.weight: platformRow.selected ? Font.Black : Font.DemiBold
                                elide: Text.ElideRight
                            }
                            Text {
                                visible: platformRow.platformName === view.currentPlatformName
                                text: "CURRENT SHELF"
                                color: view.accentCool
                                font.pixelSize: 8
                                font.weight: Font.Bold
                                font.letterSpacing: 1
                            }
                        }

                        Column {
                            id: countColumn
                            anchors.right: parent.right
                            anchors.rightMargin: 22
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 3
                            Text {
                                anchors.right: parent.right
                                text: platformRow.gameCount.toLocaleString(Qt.locale(), "f", 0)
                                color: platformRow.selected ? view.accent : view.ink
                                font.pixelSize: platformRow.selected ? 18 : 14
                                font.weight: Font.Black
                            }
                            Text {
                                anchors.right: parent.right
                                text: platformRow.gameCount === 1 ? "GAME" : "GAMES"
                                color: view.muted
                                font.pixelSize: 8
                                font.weight: Font.Bold
                                font.letterSpacing: 1
                            }
                        }

                        HoverHandler { id: platformHover }
                        TapHandler {
                            onTapped: view.choosePlatform(platformRow.index)
                        }
                    }
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 72
                color: view.withAlpha(view.background, 0.86)

                Row {
                    anchors.centerIn: parent
                    spacing: 28
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? "D-PAD / STICK  BROWSE" : "↑ ↓  BROWSE"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? view.gamepad.button_label("accept") + "  SELECT"
                              : "ENTER  SELECT"
                        color: view.accent
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? view.gamepad.button_label("back") + "  CANCEL"
                              : "ESC  CANCEL"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                }
            }
        }
    }

    Rectangle {
        id: collectionWheelOverlay
        anchors.fill: parent
        z: 46
        visible: view.collectionWheelOpen
        color: view.withAlpha(view.background, 0.93)

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: view.withAlpha(view.background, 0.96) }
                GradientStop { position: 0.55; color: view.withAlpha(view.panel, 0.89) }
                GradientStop { position: 1; color: view.withAlpha(view.background, 0.76) }
            }
        }

        Rectangle {
            id: collectionWheelPanel
            anchors.centerIn: parent
            width: Math.min(940, parent.width - 180)
            height: Math.min(860, parent.height - 120)
            radius: Math.min(32, view.cardRadius + 10)
            color: view.withAlpha(view.panel, 0.97)
            border.color: view.withAlpha(view.muted, 0.58)
            border.width: 1
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 118
                color: view.withAlpha(view.panelRaised, 0.86)

                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: 42
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 5

                    Text {
                        text: "CHOOSE A COLLECTION"
                        color: view.ink
                        font.pixelSize: 24
                        font.weight: Font.Black
                        font.letterSpacing: 1.3
                    }
                    Text {
                        text: view.library.collection_count + " collection"
                              + (view.library.collection_count === 1 ? "" : "s")
                              + " · exact saved identity"
                        color: view.muted
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.4
                    }
                }

                Rectangle {
                    anchors.right: parent.right
                    anchors.rightMargin: 32
                    anchors.verticalCenter: parent.verticalCenter
                    width: 44
                    height: 44
                    radius: 13
                    color: collectionCloseHover.hovered
                           ? view.withAlpha(view.panelRaised, 0.9)
                           : view.withAlpha(view.panel, 0.72)
                    border.color: collectionCloseHover.hovered
                                  ? view.accent : view.withAlpha(view.muted, 0.55)
                    Text {
                        anchors.centerIn: parent
                        text: "×"
                        color: view.ink
                        font.pixelSize: 23
                    }
                    HoverHandler { id: collectionCloseHover }
                    TapHandler { onTapped: view.closeCollectionWheel() }
                }
            }

            ListView {
                id: collectionWheel
                anchors.left: parent.left
                anchors.leftMargin: 34
                anchors.right: parent.right
                anchors.rightMargin: 34
                anchors.top: parent.top
                anchors.topMargin: 128
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 82
                model: view.library.collection_count
                clip: true
                reuseItems: true
                cacheBuffer: Math.max(0, Math.round(height))
                spacing: 7
                boundsBehavior: Flickable.StopAtBounds
                snapMode: ListView.SnapToItem
                preferredHighlightBegin: height * 0.5 - 49
                preferredHighlightEnd: height * 0.5 + 49
                highlightRangeMode: ListView.StrictlyEnforceRange
                highlightMoveDuration: 120
                onCurrentIndexChanged: {
                    if (currentIndex >= 0)
                        view.collectionWheelIndex = currentIndex
                }

                delegate: Item {
                    id: collectionRow
                    required property int index
                    property int revision: view.library.collection_revision
                    property string collectionId: {
                        revision
                        return view.library.collection_id_at(index)
                    }
                    property string collectionName: {
                        revision
                        return view.library.collection_name_at(index)
                    }
                    property string description: {
                        revision
                        return view.library.collection_description_at(index)
                    }
                    property string kind: {
                        revision
                        return view.library.collection_kind_at(index)
                    }
                    property string ruleSummary: {
                        revision
                        return view.library.collection_rule_summary_at(index)
                    }
                    property int gameCount: {
                        revision
                        return view.library.collection_game_count_at(index)
                    }
                    property bool selected: collectionWheel.currentIndex === index
                    property int distance: Math.abs(index - collectionWheel.currentIndex)
                    width: collectionWheel.width
                    height: selected ? 98 : 80
                    opacity: selected ? 1 : distance <= 2 ? 0.74 : 0.42
                    scale: selected ? 1 : 0.975
                    Accessible.name: collectionName + ", " + gameCount
                                     + (gameCount === 1 ? " game" : " games")
                    Behavior on height { NumberAnimation { duration: 110 } }
                    Behavior on opacity { NumberAnimation { duration: 110 } }
                    Behavior on scale { NumberAnimation { duration: 110 } }

                    Rectangle {
                        anchors.fill: parent
                        anchors.leftMargin: collectionRow.selected ? 0 : 16
                        anchors.rightMargin: collectionRow.selected ? 0 : 16
                        radius: 16
                        color: collectionRow.selected
                               ? view.withAlpha(view.panelRaised, 0.9)
                               : collectionHover.hovered
                                 ? view.withAlpha(view.panelRaised, 0.55)
                                 : "transparent"
                        border.color: collectionRow.selected ? view.accent : "transparent"
                        border.width: collectionRow.selected ? 2 : 0

                        Rectangle {
                            anchors.left: parent.left
                            anchors.leftMargin: 16
                            anchors.verticalCenter: parent.verticalCenter
                            width: collectionRow.selected ? 56 : 46
                            height: width
                            radius: 15
                            color: view.accentFor(collectionRow.collectionName)
                            border.color: view.withAlpha(view.ink, 0.51)
                            Text {
                                anchors.centerIn: parent
                                text: collectionRow.kind === "smart" ? "⚡" : "▣"
                                color: view.withAlpha(view.ink, 0.97)
                                font.pixelSize: collectionRow.selected ? 24 : 19
                                font.weight: Font.Black
                            }
                        }

                        Column {
                            anchors.left: parent.left
                            anchors.leftMargin: collectionRow.selected ? 94 : 80
                            anchors.right: collectionCountColumn.left
                            anchors.rightMargin: 28
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 4

                            Row {
                                spacing: 10
                                Text {
                                    text: collectionRow.collectionName
                                    color: view.ink
                                    font.pixelSize: collectionRow.selected ? 19 : 15
                                    font.weight: collectionRow.selected
                                                 ? Font.Black : Font.DemiBold
                                    elide: Text.ElideRight
                                    width: Math.min(implicitWidth,
                                                    collectionWheel.width * 0.46)
                                }
                                Text {
                                    visible: collectionRow.collectionId
                                             === view.currentCollectionId
                                    text: "CURRENT SHELF"
                                    color: view.accentCool
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                    font.letterSpacing: 1
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                            }
                            Text {
                                width: parent.width
                                text: collectionRow.description.length > 0
                                      ? collectionRow.description
                                      : collectionRow.kind === "smart"
                                        ? collectionRow.ruleSummary
                                        : "A manually curated game shelf"
                                color: view.muted
                                font.pixelSize: 10
                                maximumLineCount: collectionRow.selected ? 2 : 1
                                elide: Text.ElideRight
                                wrapMode: Text.WordWrap
                            }
                        }

                        Column {
                            id: collectionCountColumn
                            anchors.right: parent.right
                            anchors.rightMargin: 22
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 3
                            Text {
                                anchors.right: parent.right
                                text: collectionRow.gameCount.toLocaleString(
                                          Qt.locale(), "f", 0)
                                color: collectionRow.selected ? view.accent : view.ink
                                font.pixelSize: collectionRow.selected ? 18 : 14
                                font.weight: Font.Black
                            }
                            Text {
                                anchors.right: parent.right
                                text: collectionRow.kind === "smart" ? "SMART" : "CURATED"
                                color: view.muted
                                font.pixelSize: 8
                                font.weight: Font.Bold
                                font.letterSpacing: 1
                            }
                        }

                        HoverHandler { id: collectionHover }
                        TapHandler { onTapped: view.chooseCollection(collectionRow.index) }
                    }
                }

                Text {
                    anchors.centerIn: parent
                    visible: view.library.collection_count === 0
                    width: Math.min(520, parent.width - 80)
                    text: "No collections yet\n\nCreate a manual or smart collection in Desktop Mode, then it will appear here automatically."
                    color: view.muted
                    font.pixelSize: 15
                    lineHeight: 1.35
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 72
                color: view.withAlpha(view.background, 0.86)

                Row {
                    anchors.centerIn: parent
                    spacing: 28
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? "D-PAD / STICK  BROWSE" : "↑ ↓  BROWSE"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: view.library.collection_count > 0
                              ? (view.gamepad.connected_count > 0
                                 ? view.gamepad.button_label("accept") + "  SELECT"
                                 : "ENTER  SELECT")
                              : "DESKTOP MODE  CREATE COLLECTION"
                        color: view.accent
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? view.gamepad.button_label("back") + "  CANCEL"
                              : "ESC  CANCEL"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                }
            }
        }
    }

    Rectangle {
        id: variantWheelOverlay
        anchors.fill: parent
        z: 47
        visible: view.variantWheelOpen
        color: view.withAlpha(view.background, 0.94)

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: view.withAlpha(view.background, 0.97) }
                GradientStop { position: 0.55; color: view.withAlpha(view.panel, 0.9) }
                GradientStop { position: 1; color: view.withAlpha(view.background, 0.78) }
            }
        }

        Rectangle {
            id: variantWheelPanel
            anchors.centerIn: parent
            width: Math.min(1020, parent.width - 180)
            height: Math.min(860, parent.height - 120)
            radius: Math.min(32, view.cardRadius + 10)
            color: view.withAlpha(view.panel, 0.98)
            border.color: view.withAlpha(view.muted, 0.58)
            border.width: 1
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 132
                color: view.withAlpha(view.panelRaised, 0.87)

                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: 42
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 5

                    Text {
                        text: "CHOOSE A RELEASE"
                        color: view.ink
                        font.pixelSize: 24
                        font.weight: Font.Black
                        font.letterSpacing: 1.3
                    }
                    Text {
                        text: view.details.variant_count + " exact catalog releases · region and version preferences"
                        color: view.muted
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.35
                    }
                    Text {
                        text: view.details.media_visible
                              ? "Cached video or manual media is available for this record."
                              : "Media and acquisition state stay attached to each exact release."
                        color: view.details.media_visible ? view.accentCool : view.muted
                        font.pixelSize: 10
                        font.weight: Font.Medium
                    }
                }

                Rectangle {
                    anchors.right: parent.right
                    anchors.rightMargin: 32
                    anchors.verticalCenter: parent.verticalCenter
                    width: 44
                    height: 44
                    radius: 13
                    color: variantCloseHover.hovered
                           ? view.withAlpha(view.panelRaised, 0.9)
                           : view.withAlpha(view.panel, 0.72)
                    border.color: variantCloseHover.hovered
                                  ? view.accent : view.withAlpha(view.muted, 0.55)
                    Text {
                        anchors.centerIn: parent
                        text: "×"
                        color: view.ink
                        font.pixelSize: 23
                    }
                    HoverHandler { id: variantCloseHover }
                    TapHandler { onTapped: view.closeVariantWheel() }
                }
            }

            ListView {
                id: variantWheel
                anchors.left: parent.left
                anchors.leftMargin: 34
                anchors.right: parent.right
                anchors.rightMargin: 34
                anchors.top: parent.top
                anchors.topMargin: 142
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 82
                model: view.details.variant_count
                clip: true
                reuseItems: true
                cacheBuffer: Math.max(0, Math.round(height))
                spacing: 7
                boundsBehavior: Flickable.StopAtBounds
                snapMode: ListView.SnapToItem
                preferredHighlightBegin: height * 0.5 - 53
                preferredHighlightEnd: height * 0.5 + 53
                highlightRangeMode: ListView.StrictlyEnforceRange
                highlightMoveDuration: 120
                onCurrentIndexChanged: {
                    if (currentIndex >= 0)
                        view.variantWheelIndex = currentIndex
                }

                delegate: Item {
                    id: variantRow
                    required property int index
                    property int revision: view.details.detail_revision
                    property string releaseTitle: {
                        revision
                        return view.details.variant_title_at(index)
                    }
                    property string releaseLabel: {
                        revision
                        return view.details.variant_label_at(index)
                    }
                    property string releaseStatus: {
                        revision
                        const value = view.details.variant_status_at(index)
                        return value.length > 0 ? value : "CATALOG"
                    }
                    property bool current: {
                        revision
                        return view.details.variant_is_current_at(index)
                    }
                    property bool local: {
                        revision
                        return view.details.variant_is_local_at(index)
                    }
                    property bool downloadable: {
                        revision
                        return view.details.variant_is_downloadable_at(index)
                    }
                    property bool selected: variantWheel.currentIndex === index
                    property int distance: Math.abs(index - variantWheel.currentIndex)
                    width: variantWheel.width
                    height: selected ? 112 : 88
                    opacity: selected ? 1 : distance <= 2 ? 0.76 : 0.45
                    scale: selected ? 1 : 0.975
                    Accessible.name: releaseLabel + ", " + releaseStatus
                    Behavior on height { NumberAnimation { duration: 110 } }
                    Behavior on opacity { NumberAnimation { duration: 110 } }
                    Behavior on scale { NumberAnimation { duration: 110 } }

                    Rectangle {
                        anchors.fill: parent
                        anchors.leftMargin: variantRow.selected ? 0 : 16
                        anchors.rightMargin: variantRow.selected ? 0 : 16
                        radius: 16
                        color: variantRow.selected
                               ? view.withAlpha(view.panelRaised, 0.92)
                               : variantHover.hovered
                                 ? view.withAlpha(view.panelRaised, 0.56)
                                 : "transparent"
                        border.color: variantRow.selected ? view.accent : "transparent"
                        border.width: variantRow.selected ? 2 : 0

                        Rectangle {
                            anchors.left: parent.left
                            anchors.leftMargin: 16
                            anchors.verticalCenter: parent.verticalCenter
                            width: variantRow.selected ? 62 : 50
                            height: width
                            radius: 16
                            color: view.accentFor(variantRow.releaseTitle)
                            border.color: view.withAlpha(view.ink, 0.51)
                            Text {
                                anchors.centerIn: parent
                                text: variantRow.current ? "✓" : "◇"
                                color: view.withAlpha(view.ink, 0.97)
                                font.pixelSize: variantRow.selected ? 25 : 20
                                font.weight: Font.Black
                            }
                        }

                        Column {
                            anchors.left: parent.left
                            anchors.leftMargin: variantRow.selected ? 98 : 84
                            anchors.right: variantStatusColumn.left
                            anchors.rightMargin: 24
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 5

                            Row {
                                spacing: 10
                                Text {
                                    text: variantRow.releaseLabel
                                    color: view.ink
                                    font.pixelSize: variantRow.selected ? 19 : 15
                                    font.weight: variantRow.selected
                                                 ? Font.Black : Font.DemiBold
                                    elide: Text.ElideRight
                                    width: Math.min(implicitWidth,
                                                    variantWheel.width * 0.48)
                                }
                                Text {
                                    visible: variantRow.current
                                    text: "CURRENT"
                                    color: view.accentCool
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                    font.letterSpacing: 1
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                            }
                            Text {
                                width: parent.width
                                text: variantRow.releaseTitle
                                color: view.muted
                                font.pixelSize: 10
                                maximumLineCount: variantRow.selected ? 2 : 1
                                elide: Text.ElideRight
                                wrapMode: Text.WordWrap
                            }
                        }

                        Column {
                            id: variantStatusColumn
                            anchors.right: parent.right
                            anchors.rightMargin: 22
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 5

                            Rectangle {
                                anchors.right: parent.right
                                width: variantStatusText.implicitWidth + 16
                                height: 22
                                radius: 7
                                color: variantRow.releaseStatus === "CURRENT"
                                       ? view.withAlpha(view.accentCool, 0.2)
                                       : variantRow.releaseStatus === "INSTALLED"
                                         ? view.withAlpha(view.accentCool, 0.14)
                                         : variantRow.releaseStatus === "MINERVA"
                                           ? view.withAlpha(view.accent, 0.2)
                                           : view.withAlpha(view.muted, 0.14)
                                border.color: variantRow.releaseStatus === "CURRENT"
                                              ? view.accentCool
                                              : variantRow.releaseStatus === "MINERVA"
                                                ? view.accent : view.withAlpha(view.muted, 0.5)
                                Text {
                                    id: variantStatusText
                                    anchors.centerIn: parent
                                    text: variantRow.releaseStatus
                                    color: variantRow.releaseStatus === "CURRENT"
                                           || variantRow.releaseStatus === "INSTALLED"
                                           ? view.accentCool
                                           : variantRow.releaseStatus === "MINERVA"
                                             ? view.accent : view.muted
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                    font.letterSpacing: 0.55
                                }
                            }
                            Text {
                                anchors.right: parent.right
                                text: variantRow.local ? "INSTALLED"
                                      : variantRow.downloadable ? "DOWNLOADABLE"
                                                                : "CATALOG ONLY"
                                color: variantRow.local ? view.accentCool
                                      : variantRow.downloadable ? view.accent : view.muted
                                font.pixelSize: 8
                                font.weight: Font.Bold
                                font.letterSpacing: 0.7
                            }
                        }

                        HoverHandler { id: variantHover }
                        TapHandler {
                            enabled: !variantRow.current && !view.details.loading
                            onTapped: view.chooseVariant(variantRow.index)
                        }
                    }
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 72
                color: view.withAlpha(view.background, 0.86)

                Row {
                    anchors.centerIn: parent
                    spacing: 28
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? "D-PAD / STICK  BROWSE" : "↑ ↓  BROWSE"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? view.gamepad.button_label("accept") + "  SELECT"
                              : "ENTER  SELECT"
                        color: view.accent
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? view.gamepad.button_label("back") + "  CANCEL"
                              : "ESC  CANCEL"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                }
            }
        }
    }

    Rectangle {
        id: couchOverlay
        anchors.fill: parent
        z: 50
        visible: view.overlayOpen
        color: view.withAlpha(view.background, 0.91)

        Rectangle {
            id: overlayPanel
            anchors.centerIn: parent
            width: Math.min(1180, parent.width - 120)
            height: Math.min(790, parent.height - 110)
            radius: Math.min(32, view.cardRadius + 8)
            color: view.withAlpha(view.panel, 0.96)
            border.color: view.withAlpha(view.muted, 0.56)
            border.width: 1
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: 350
                color: view.withAlpha(view.background, 0.72)

                Rectangle {
                    id: overlayCover
                    anchors.top: parent.top
                    anchors.topMargin: 42
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 220
                    height: 304
                    radius: view.cardRadius
                    color: view.accentFor(view.selectedTitle)
                    border.color: view.withAlpha(view.ink, 0.55)
                    clip: true
                    gradient: Gradient {
                        GradientStop {
                            position: 0
                            color: Qt.lighter(overlayCover.color, 1.18)
                        }
                        GradientStop {
                            position: 1
                            color: Qt.darker(overlayCover.color, 1.62)
                        }
                    }
                    Image {
                        id: overlayCoverImage
                        anchors.fill: parent
                        anchors.margins: 2
                        source: view.coverUrl
                        asynchronous: true
                        cache: true
                        mipmap: true
                        autoTransform: true
                        fillMode: Image.PreserveAspectFit
                        sourceSize.width: 440
                        sourceSize.height: 608
                        opacity: status === Image.Ready ? 1 : 0
                    }
                    Text {
                        anchors.centerIn: parent
                        visible: overlayCoverImage.status !== Image.Ready
                        text: view.selectedTitle.length > 0
                              ? view.selectedTitle.charAt(0).toUpperCase() : "L"
                        color: view.withAlpha(view.ink, 0.87)
                        font.pixelSize: 82
                        font.weight: Font.Black
                    }
                }

                Column {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: overlayCover.bottom
                    anchors.topMargin: 28
                    anchors.margins: 34
                    spacing: 10

                    Text {
                        width: parent.width
                        text: view.selectedTitle
                        color: view.ink
                        font.pixelSize: 27
                        font.weight: Font.Black
                        wrapMode: Text.WordWrap
                        maximumLineCount: 3
                        elide: Text.ElideRight
                    }
                    Text {
                        width: parent.width
                        text: view.selectedPlatform.toUpperCase()
                        color: view.muted
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 1.1
                        elide: Text.ElideRight
                    }
                    Rectangle {
                        width: overlayStateText.implicitWidth + 20
                        height: 28
                        radius: Math.max(7, view.cardRadius - 7)
                        color: view.selectedLocal ? view.withAlpha(view.accentCool, 0.22)
                               : view.selectedDownloadable ? view.withAlpha(view.accent, 0.22)
                                                           : view.withAlpha(view.panelRaised, 0.66)
                        border.color: view.selectedLocal ? view.accentCool
                                      : view.selectedDownloadable ? view.accent
                                                                  : view.withAlpha(view.muted, 0.54)
                        Text {
                            id: overlayStateText
                            anchors.centerIn: parent
                            text: view.details.can_launch ? "READY TO PLAY"
                                  : view.selectedLocal ? "SETUP REQUIRED"
                                  : view.selectedDownloadable ? "MINERVA AVAILABLE"
                                    : "CATALOG RECORD"
                            color: view.details.can_launch || view.selectedLocal
                                   ? view.accentCool
                                   : view.selectedDownloadable ? view.accent : view.muted
                            font.pixelSize: 9
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8
                        }
                    }
                    Text {
                        width: parent.width
                        visible: view.details.emulator_name.length > 0
                        text: view.details.emulator_name
                        color: view.muted
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                }
            }

            Item {
                id: overlayContent
                anchors.left: parent.left
                anchors.leftMargin: 350
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.bottom: parent.bottom

                Row {
                    id: overlayHeader
                    anchors.left: parent.left
                    anchors.leftMargin: 38
                    anchors.right: parent.right
                    anchors.rightMargin: 30
                    anchors.top: parent.top
                    anchors.topMargin: 30
                    height: 44
                    spacing: 14

                    Text {
                        width: Math.max(0, parent.width - overlayClose.width - parent.spacing)
                        anchors.verticalCenter: parent.verticalCenter
                        text: view.overlayMode === "details"
                              ? "GAME DETAILS" : "GAME MENU"
                        color: view.ink
                        font.pixelSize: 18
                        font.weight: Font.Black
                        font.letterSpacing: 1.2
                    }
                    Rectangle {
                        id: overlayClose
                        width: 42
                        height: 42
                        radius: Math.max(8, view.cardRadius - 4)
                        color: overlayCloseHover.hovered
                               ? view.withAlpha(view.panelRaised, 0.9)
                               : view.withAlpha(view.panelRaised, 0.62)
                        border.color: overlayCloseHover.hovered ? view.accent
                                                               : view.withAlpha(view.muted, 0.54)
                        Text {
                            anchors.centerIn: parent
                            text: "×"
                            color: view.ink
                            font.pixelSize: 22
                        }
                        HoverHandler { id: overlayCloseHover }
                        TapHandler { onTapped: view.closeOverlay() }
                    }
                }

                Row {
                    id: overlayTabs
                    anchors.left: parent.left
                    anchors.leftMargin: 38
                    anchors.top: overlayHeader.bottom
                    anchors.topMargin: 12
                    spacing: 10

                    Repeater {
                        model: ["DETAILS", "GAME MENU"]
                        delegate: Rectangle {
                            id: overlayTab
                            required property int index
                            required property string modelData
                            property bool selected: view.overlayMode
                                                    === (index === 0 ? "details" : "menu")
                            width: tabText.implicitWidth + 28
                            height: 38
                            radius: Math.max(8, view.cardRadius - 5)
                            color: selected ? view.withAlpha(view.accent, 0.18)
                                            : view.withAlpha(view.panelRaised, 0.58)
                            border.color: selected ? view.accent
                                                   : view.withAlpha(view.muted, 0.44)
                            Text {
                                id: tabText
                                anchors.centerIn: parent
                                text: overlayTab.modelData
                                color: overlayTab.selected ? view.ink : view.muted
                                font.pixelSize: 10
                                font.weight: Font.Bold
                                font.letterSpacing: 0.8
                            }
                            TapHandler {
                                onTapped: {
                                    view.overlayMode = overlayTab.index === 0
                                                       ? "details" : "menu"
                                    if (view.overlayMode === "details")
                                        detailsScroller.contentY = 0
                                    view.forceActiveFocus()
                                }
                            }
                        }
                    }
                }

                Item {
                    id: overlayBody
                    anchors.left: parent.left
                    anchors.leftMargin: 38
                    anchors.right: parent.right
                    anchors.rightMargin: 38
                    anchors.top: overlayTabs.bottom
                    anchors.topMargin: 22
                    anchors.bottom: overlayHelp.top
                    anchors.bottomMargin: 18

                    Flickable {
                        id: detailsScroller
                        anchors.fill: parent
                        visible: view.overlayMode === "details"
                        clip: true
                        contentWidth: width
                        contentHeight: detailsContent.implicitHeight
                        boundsBehavior: Flickable.StopAtBounds

                        Column {
                            id: detailsContent
                            width: detailsScroller.width - 12
                            spacing: 18

                            Text {
                                width: parent.width
                                text: view.details.loading
                                      ? "Loading the exact game record…"
                                      : view.details.description.length > 0
                                        ? view.details.description
                                        : "No description is available for this exact release."
                                color: view.withAlpha(view.ink, 0.88)
                                font.pixelSize: 15
                                lineHeight: 1.35
                                wrapMode: Text.WordWrap
                            }

                            Grid {
                                width: parent.width
                                columns: 2
                                columnSpacing: 12
                                rowSpacing: 12

                                Repeater {
                                    model: [
                                        { label: "RELEASED", value: view.details.release_date },
                                        { label: "GENRE", value: view.details.genre },
                                        { label: "DEVELOPER", value: view.details.developer },
                                        { label: "PUBLISHER", value: view.details.publisher },
                                        { label: "PLAYERS", value: view.details.players },
                                        { label: "CO-OP", value: view.details.cooperative === "yes"
                                                               ? "Supported"
                                                               : view.details.cooperative === "no"
                                                                 ? "Not supported" : "" },
                                        { label: "RATING", value: view.details.rating.length > 0
                                                                  ? "★ " + view.details.rating
                                                                    + (view.details.rating_count > 0
                                                                       ? " · " + view.details.rating_count.toLocaleString(Qt.locale(), "f", 0)
                                                                         + " ratings" : "") : "" },
                                        { label: "AGE RATING", value: view.details.esrb },
                                        { label: "RELEASE TYPE", value: view.details.release_type },
                                        { label: "SERIES", value: view.details.series },
                                        { label: "REGION", value: view.details.region },
                                        { label: "VERSION", value: view.details.version },
                                        { label: "PLAY MODE", value: view.details.play_mode },
                                        { label: "RELEASE STATUS", value: view.details.release_status },
                                        { label: "METADATA", value: view.details.metadata_source }
                                    ]
                                    delegate: Rectangle {
                                        id: factCard
                                        required property var modelData
                                        visible: String(modelData.value).length > 0
                                        width: (detailsContent.width - 12) / 2
                                        height: visible ? 66 : 0
                                        radius: Math.max(8, view.cardRadius - 4)
                                        color: view.withAlpha(view.panelRaised, 0.4)
                                        border.color: view.withAlpha(view.muted, 0.35)
                                        Column {
                                            anchors.fill: parent
                                            anchors.margins: 12
                                            spacing: 5
                                            Text {
                                                text: factCard.modelData.label
                                                color: view.muted
                                                font.pixelSize: 8
                                                font.weight: Font.Bold
                                                font.letterSpacing: 1
                                            }
                                            Text {
                                                width: parent.width
                                                text: factCard.modelData.value
                                                color: view.ink
                                                font.pixelSize: 12
                                                font.weight: Font.DemiBold
                                                elide: Text.ElideRight
                                            }
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                width: parent.width
                                height: activityContent.implicitHeight + 28
                                radius: Math.max(9, view.cardRadius - 2)
                                color: view.withAlpha(view.panelRaised, 0.4)
                                border.color: view.withAlpha(view.muted, 0.35)
                                Column {
                                    id: activityContent
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: 14
                                    spacing: 9
                                    Text {
                                        text: "YOUR ACTIVITY"
                                        color: view.accent
                                        font.pixelSize: 9
                                        font.weight: Font.Bold
                                        font.letterSpacing: 1
                                    }
                                    Text {
                                        width: parent.width
                                        text: view.details.activity_visible
                                              ? view.details.play_count + " launches  ·  "
                                                + view.details.play_time + "  ·  "
                                                + view.details.completion_state.split("-").join(" ").toUpperCase()
                                              : "No play activity has been recorded for this game."
                                        color: view.ink
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        wrapMode: Text.WordWrap
                                    }
                                    Text {
                                        visible: view.details.last_played.length > 0
                                        text: "Last played " + view.details.last_played
                                        color: view.muted
                                        font.pixelSize: 11
                                    }
                                }
                            }

                            Rectangle {
                                width: parent.width
                                visible: view.details.notes.length > 0
                                height: visible ? notesContent.implicitHeight + 28 : 0
                                radius: Math.max(9, view.cardRadius - 2)
                                color: view.withAlpha(view.panelRaised, 0.4)
                                border.color: view.withAlpha(view.muted, 0.35)
                                Column {
                                    id: notesContent
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: 14
                                    spacing: 8
                                    Text {
                                        text: "NOTES"
                                        color: view.accent
                                        font.pixelSize: 9
                                        font.weight: Font.Bold
                                        font.letterSpacing: 1
                                    }
                                    Text {
                                        width: parent.width
                                        text: view.details.notes
                                        color: view.ink
                                        font.pixelSize: 12
                                        lineHeight: 1.25
                                        wrapMode: Text.WordWrap
                                    }
                                }
                            }

                            Flow {
                                width: parent.width
                                visible: view.details.tag_count > 0
                                height: visible ? childrenRect.height : 0
                                spacing: 8
                                Repeater {
                                    model: view.details.tag_count
                                    delegate: Rectangle {
                                        id: overlayTag
                                        required property int index
                                        width: overlayTagText.implicitWidth + 20
                                        height: 28
                                        radius: Math.max(7, view.cardRadius - 7)
                                        color: view.withAlpha(view.accentCool, 0.16)
                                        border.color: view.accentCool
                                        Text {
                                            id: overlayTagText
                                            anchors.centerIn: parent
                                            text: view.details.tag_at(overlayTag.index).toUpperCase()
                                            color: view.accentCool
                                            font.pixelSize: 9
                                            font.weight: Font.Bold
                                            font.letterSpacing: 0.6
                                        }
                                    }
                                }
                            }

                            Text {
                                width: parent.width
                                visible: view.details.firmware_rule_count > 0
                                         || view.details.launch_status.length > 0
                                text: [view.details.launch_status,
                                       view.details.firmware_summary]
                                      .filter(value => value.length > 0).join("\n")
                                color: view.details.can_launch ? view.accentCool : view.muted
                                font.pixelSize: 11
                                lineHeight: 1.3
                                wrapMode: Text.WordWrap
                            }
                        }

                        Rectangle {
                            anchors.right: parent.right
                            width: 3
                            radius: 2
                            visible: detailsScroller.contentHeight > detailsScroller.height
                            height: visible ? Math.max(36, detailsScroller.height
                                                      * detailsScroller.height
                                                      / detailsScroller.contentHeight) : 0
                            y: visible ? detailsScroller.contentY
                                         * (detailsScroller.height - height)
                                         / Math.max(1, detailsScroller.contentHeight
                                                    - detailsScroller.height) : 0
                            color: view.accent
                        }
                    }

                    Column {
                        id: gameMenu
                        anchors.fill: parent
                        visible: view.overlayMode === "menu"
                        spacing: 12

                        Text {
                            width: parent.width
                            text: "Choose what to do with this exact release. Download and management actions hand off to the complete desktop tools without losing selection."
                            color: view.muted
                            font.pixelSize: 13
                            lineHeight: 1.3
                            wrapMode: Text.WordWrap
                        }

                        Repeater {
                            model: view.menuActionCount
                            delegate: Rectangle {
                                id: menuAction
                                required property int index
                                property bool selected: view.menuActionIndex === index
                                width: gameMenu.width
                                height: 88
                                radius: Math.max(9, view.cardRadius - 2)
                                color: selected ? view.withAlpha(view.accent, 0.2)
                                       : menuHover.hovered
                                         ? view.withAlpha(view.panelRaised, 0.72)
                                         : view.withAlpha(view.panelRaised, 0.4)
                                border.color: selected ? view.accent
                                                       : view.withAlpha(view.muted, 0.35)
                                border.width: selected ? 2 : 1
                                opacity: view.menuActionEnabled(menuAction.index) ? 1 : 0.46
                                scale: selected ? 1.012 : 1
                                Behavior on scale { NumberAnimation { duration: 90 } }
                                Behavior on opacity { NumberAnimation { duration: 110 } }

                                Column {
                                    anchors.left: parent.left
                                    anchors.right: menuGlyph.left
                                    anchors.rightMargin: 20
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.leftMargin: 20
                                    spacing: 7
                                    Text {
                                        width: parent.width
                                        text: view.menuActionLabel(menuAction.index)
                                        color: menuAction.index === 0 ? view.accent : view.ink
                                        font.pixelSize: 13
                                        font.weight: Font.Bold
                                        font.letterSpacing: 0.6
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        width: parent.width
                                        text: view.menuActionDescription(menuAction.index)
                                        color: view.muted
                                        font.pixelSize: 11
                                        elide: Text.ElideRight
                                    }
                                }
                                Text {
                                    id: menuGlyph
                                    anchors.right: parent.right
                                    anchors.rightMargin: 20
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: menuAction.index === 0 ? "▶"
                                          : menuAction.index === 1
                                            ? (view.favorite ? "★" : "☆")
                                          : menuAction.index === 2 ? "↗"
                                          : menuAction.index === 3 ? "◈"
                                          : menuAction.index === 4 ? "◌" : "×"
                                    color: menuAction.selected ? view.accent : view.muted
                                    font.pixelSize: menuAction.index === 1 ? 24 : 18
                                    font.weight: Font.Bold
                                }
                                HoverHandler { id: menuHover }
                                TapHandler {
                                    enabled: view.menuActionEnabled(menuAction.index)
                                    onTapped: {
                                        view.menuActionIndex = menuAction.index
                                        view.activateMenuAction(menuAction.index)
                                        view.forceActiveFocus()
                                    }
                                }
                            }
                        }
                    }
                }

                Row {
                    id: overlayHelp
                    anchors.left: parent.left
                    anchors.leftMargin: 38
                    anchors.right: parent.right
                    anchors.rightMargin: 38
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 24
                    height: 24
                    spacing: 20

                    Text {
                        text: view.gamepad.connected_count > 0
                              ? view.gamepad.button_label("page_left") + " / "
                                + view.gamepad.button_label("page_right") + "  SWITCH PANEL"
                              : "← →  SWITCH PANEL"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? view.gamepad.button_label("accept") + "  SELECT"
                              : "ENTER  SELECT"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                    Item { width: Math.max(0, parent.width - 480); height: 1 }
                    Text {
                        text: view.gamepad.connected_count > 0
                              ? view.gamepad.button_label("back") + "  CLOSE"
                              : "ESC  CLOSE"
                        color: view.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                }
            }
        }
    }

    NumberAnimation {
        id: attractProgressAnimation
        target: view
        property: "attractProgress"
        from: 0
        to: 1
        duration: Math.max(5000, view.library.couch_attract_cycle_seconds * 1000)
    }

    Timer {
        id: attractIdleTimer
        interval: view.attractProbeEnabled
                  ? 350
                  : Math.max(30, view.library.couch_attract_idle_seconds) * 1000
        repeat: false
        running: view.active
                 && (view.library.couch_attract_enabled
                     || view.attractProbeEnabled)
                 && !view.attractOpen
                 && !view.overlayOpen
                 && !view.platformWheelOpen
                 && !view.collectionWheelOpen
                 && shelf.count > 0
        onTriggered: view.startAttractMode("idle")
    }

    Timer {
        id: attractCycleTimer
        interval: Math.max(5, view.library.couch_attract_cycle_seconds) * 1000
        repeat: false
        running: view.active && view.attractOpen && shelf.count > 0
        onTriggered: view.moveAttract(1)
    }

    Timer {
        id: selectionDelay
        interval: 120
        repeat: false
        onTriggered: {
            if (view.active)
                view.loadCurrentGame()
        }
    }

    Timer {
        id: selectionRetry
        interval: 40
        repeat: false
        onTriggered: {
            if (!view.active)
                return
            if (shelf.count > 0) {
                if (shelf.currentIndex < 0) {
                    const preferredRow = view.library.row_for_game(
                                               view.preferredGameId)
                    shelf.currentIndex = preferredRow >= 0
                                       && preferredRow < shelf.count
                                       ? preferredRow : 0
                    shelf.positionViewAtIndex(shelf.currentIndex,
                                              ListView.Center)
                }
                view.loadCurrentGame()
            } else if (view.library.filtered_count > 0) {
                selectionRetry.restart()
            }
        }
    }

    Connections {
        target: view.library
        function onFiltered_countChanged() {
            if (shelf.count <= 0) {
                view.stopAttractMode()
                shelf.currentIndex = -1
                view.selectedGameId = ""
                view.selectedDatabaseId = 0
                view.selectedTitle = ""
                return
            }
            shelf.currentIndex = Math.max(0, Math.min(shelf.currentIndex,
                                                      shelf.count - 1))
            if (view.active)
                selectionDelay.restart()
        }
        function onCollection_revisionChanged() {
            if (view.collectionWheelOpen) {
                view.collectionWheelIndex = view.library.collection_count > 0
                        ? Math.max(0, Math.min(view.collectionWheelIndex,
                                             view.library.collection_count - 1))
                        : 0
                collectionWheel.currentIndex = view.library.collection_count > 0
                        ? view.collectionWheelIndex : -1
            }
            view.syncCategory()
        }
    }

    Connections {
        target: view.gamepad
        function onNavigation_revisionChanged() {
            if (view.active)
                view.handleNavigation(view.gamepad.navigation_action)
        }
    }
}
