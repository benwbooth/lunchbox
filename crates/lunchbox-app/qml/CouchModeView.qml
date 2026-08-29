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
    property bool attractOpen: false
    property bool attractProbeEnabled: false
    property string attractReason: "manual"
    property real attractProgress: 0
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
    readonly property color ink: "#f8fafc"
    readonly property color muted: "#a4adbb"
    readonly property color accent: "#ffab52"
    readonly property color accentCool: "#64d8c8"
    readonly property int menuActionCount: 5
    readonly property var categories: [
        { label: "ALL GAMES", key: "" },
        { label: "PLATFORMS", key: "platform" },
        { label: "MY COLLECTION", key: "local" },
        { label: "MINERVA", key: "downloadable" },
        { label: "FAVORITES", key: "favorites" },
        { label: "RECENT", key: "recent" }
    ]
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
    signal gameSelected(string gameId, int databaseId, string title,
                        string platform, bool local, bool downloadable)
    signal detailsRequested(string gameId, int databaseId, string title,
                            string platform, bool local, bool downloadable)
    signal launchRequested()

    visible: active
    focus: active

    function accentFor(value) {
        const colors = ["#f4775f", "#5f9df7", "#9b7cf7", "#4bc29b", "#e39f45", "#e26fa0"]
        if (!value || value.length === 0)
            return colors[0]
        return colors[value.charCodeAt(0) % colors.length]
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
        const shelfKey = categories[categoryIndex].key.length > 0
                         ? categories[categoryIndex].key : "all"
        library.save_couch_state(shelfKey, "")
        filterRequested(categories[categoryIndex].key)
        shelf.currentIndex = 0
    }

    function syncCategory() {
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
        library.request_artwork(selectedDatabaseId, selectedTitle,
                                selectedPlatform, "box-front")
        library.request_artwork(selectedDatabaseId, selectedTitle,
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
                    && !details.game_running)
                launchRequested()
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

    function capturePlatformWheel(path, callback) {
        platformWheelOverlay.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    function captureAttract(path, callback) {
        attractOverlay.grabToImage(function(result) {
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
            return "START ATTRACT MODE"
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
            return "Let Couch Mode rotate through games on the current shelf."
        return "Close this menu without changing the selected game."
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
            closeOverlay()
            startAttractMode("manual")
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
                    menuActionIndex = Math.max(0, menuActionIndex - 1)
                else
                    detailsScroller.contentY = Math.max(0,
                                                        detailsScroller.contentY - 90)
            } else if (action === "down") {
                if (overlayMode === "menu")
                    menuActionIndex = Math.min(menuActionCount - 1,
                                               menuActionIndex + 1)
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
            navigationZone = Math.max(0, navigationZone - 1)
        } else if (action === "down") {
            navigationZone = Math.min(2, navigationZone + 1)
        } else if (action === "left") {
            if (navigationZone === 0)
                categoryIndex = Math.max(0, categoryIndex - 1)
            else if (navigationZone === 1)
                actionIndex = Math.max(0, actionIndex - 1)
            else
                moveShelf(-1)
        } else if (action === "right") {
            if (navigationZone === 0)
                categoryIndex = Math.min(categories.length - 1,
                                         categoryIndex + 1)
            else if (navigationZone === 1)
                actionIndex = Math.min(2, actionIndex + 1)
            else
                moveShelf(1)
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
            attractOpen = false
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
            attractOpen = false
        }
    }

    onCurrentFilterKeyChanged: syncCategory()
    onCurrentPlatformNameChanged: syncCategory()

    onMediaRevisionChanged: {
        if (selectedDatabaseId > 0) {
            library.request_artwork(selectedDatabaseId, selectedTitle,
                                    selectedPlatform, "box-front")
            library.request_artwork(selectedDatabaseId, selectedTitle,
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
        } else if (event.key === Qt.Key_Tab) {
            action = "cycle_zone"
        }
        if (action.length > 0)
            event.accepted = handleNavigation(action)
    }

    Rectangle {
        anchors.fill: parent
        color: "#080b11"
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
            GradientStop { position: 0.0; color: "#f2080b11" }
            GradientStop { position: 0.43; color: "#d6080b11" }
            GradientStop { position: 0.75; color: "#82080b11" }
            GradientStop { position: 1.0; color: "#b8080b11" }
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: parent.height * 0.43
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#00080b11" }
            GradientStop { position: 0.35; color: "#b8080b11" }
            GradientStop { position: 1.0; color: "#ff080b11" }
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
            radius: 12
            color: view.accent
            Text {
                anchors.centerIn: parent
                text: "L"
                color: "#171009"
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
                radius: 10
                color: view.categoryIndex === index
                       ? (view.navigationZone === 0 ? "#e6ffab52" : "#67372f29")
                       : categoryHover.hovered ? "#4a242a35" : "transparent"
                border.color: view.categoryIndex === index
                              ? view.accent : "transparent"
                border.width: 1

                Text {
                    id: categoryLabel
                    anchors.centerIn: parent
                    text: categoryButton.modelData.label
                    color: view.categoryIndex === categoryButton.index
                           && view.navigationZone === 0 ? "#171009"
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
            visible: view.gamepad.ready
            width: visible ? gamepadStatus.implicitWidth + 26 : 0
            height: 40
            radius: 12
            color: "#2f161b24"
            border.color: view.gamepad.connected_count > 0
                          ? view.accentCool
                          : view.gamepad.available ? "#5b303846" : "#8b4651"
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
            radius: 12
            color: exitHover.hovered ? "#4a242a35" : "#2f161b24"
            border.color: exitHover.hovered ? view.accent : "#5b303846"
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
                radius: 8
                color: "#b51c2430"
                border.color: "#68515d6d"
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
                radius: 8
                color: view.selectedLocal ? "#c51a3c35"
                       : view.selectedDownloadable ? "#c5423425" : "#b51c2430"
                border.color: view.selectedLocal ? view.accentCool
                              : view.selectedDownloadable ? view.accent : "#68515d6d"
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
            color: "#d4d9e1"
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
                    radius: 12
                    color: index === 0
                           ? (selected ? "#ffffbb70" : "#e6ffab52")
                           : selected ? "#e63b4655" : "#b5222b37"
                    border.color: selected ? "#ffffff" : index === 0
                                  ? view.accent : "#687483"
                    border.width: selected ? 2 : 1
                    opacity: index === 2 && view.favoriteBusy ? 0.55 : 1
                    scale: selected ? 1.035 : actionHover.hovered ? 1.018 : 1
                    Behavior on scale { NumberAnimation { duration: 100 } }

                    Text {
                        anchors.centerIn: parent
                        text: actionButton.index === 0 ? view.primaryAction
                              : actionButton.index === 1 ? "DESKTOP DETAILS"
                              : view.favoriteBusy ? "…" : view.favorite ? "★" : "☆"
                        color: actionButton.index === 0 ? "#171009" : view.ink
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
        anchors.right: parent.right
        anchors.rightMargin: Math.max(74, parent.width * 0.075)
        anchors.top: brand.bottom
        anchors.topMargin: 58
        width: Math.min(330, parent.width * 0.21)
        height: width * 1.38
        radius: 18
        color: view.accentFor(view.selectedTitle)
        border.color: "#8cffffff"
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
            color: "#ddffffff"
            font.pixelSize: 104
            font.weight: Font.Black
            visible: selectedCover.status !== Image.Ready
        }
    }

    Item {
        id: shelfArea
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: footer.top
        height: Math.max(250, parent.height * 0.265)

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 70
            anchors.top: parent.top
            text: view.library.filtering ? "UPDATING…"
                  : view.library.filtered_count + " GAMES"
            color: view.navigationZone === 2 ? view.accent : view.muted
            font.pixelSize: 10
            font.weight: Font.Bold
            font.letterSpacing: 1.4
        }

        ListView {
            id: shelf
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.topMargin: 25
            anchors.bottom: parent.bottom
            orientation: ListView.Horizontal
            leftMargin: 70
            rightMargin: 70
            spacing: 13
            clip: true
            reuseItems: true
            cacheBuffer: width
            model: view.library
            boundsBehavior: Flickable.StopAtBounds
            highlightMoveDuration: 130
            preferredHighlightBegin: width * 0.12
            preferredHighlightEnd: width * 0.64
            highlightRangeMode: ListView.ApplyRange
            onCurrentIndexChanged: {
                if (!view.active)
                    return
                view.captureCurrentGame()
                selectionDelay.restart()
            }
            onCurrentItemChanged: {
                if (!view.active)
                    return
                view.captureCurrentGame()
                selectionDelay.restart()
            }

            delegate: Item {
                id: gameTile
                required property int index
                required property string gameId
                required property string gameTitle
                required property string gameCanonicalTitle
                required property string gamePlatform
                required property string gameStatus
                required property bool gameLocal
                required property bool gameDownloadable
                required property int gameDatabaseId
                property int artworkRevision: view.library.media_revision
                property url artworkUrl: {
                    artworkRevision
                    return view.library.artwork_url(gameDatabaseId, "box-front")
                }
                property bool current: ListView.isCurrentItem
                width: current ? 162 : 132
                height: shelf.height - 8

                function requestArtwork() {
                    view.library.request_artwork(gameDatabaseId, gameCanonicalTitle,
                                                 gamePlatform, "box-front")
                }

                Component.onCompleted: requestArtwork()
                onGameDatabaseIdChanged: requestArtwork()
                onGameCanonicalTitleChanged: requestArtwork()
                onArtworkRevisionChanged: requestArtwork()

                Rectangle {
                    id: coverCard
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: gameTile.current ? 10 : 18
                    width: gameTile.current ? 158 : 128
                    height: gameTile.current ? 214 : 174
                    radius: 12
                    color: view.accentFor(gameTile.gameTitle)
                    border.color: gameTile.current
                                  ? (view.navigationZone === 2 ? "#ffffff" : view.accent)
                                  : "#646b7582"
                    border.width: gameTile.current ? 3 : 1
                    clip: true
                    scale: gameTile.current ? 1 : shelfHover.hovered ? 1.03 : 1
                    Behavior on width { NumberAnimation { duration: 120 } }
                    Behavior on height { NumberAnimation { duration: 120 } }
                    Behavior on scale { NumberAnimation { duration: 90 } }

                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.lighter(coverCard.color, 1.15) }
                        GradientStop { position: 1; color: Qt.darker(coverCard.color, 1.6) }
                    }
                    Image {
                        id: tileCover
                        anchors.fill: parent
                        anchors.margins: 2
                        source: gameTile.artworkUrl
                        asynchronous: true
                        cache: true
                        mipmap: true
                        autoTransform: true
                        fillMode: Image.PreserveAspectFit
                        sourceSize.width: 300
                        sourceSize.height: 420
                        opacity: status === Image.Ready ? 1 : 0
                    }
                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: 54
                        color: "#d70a0d12"
                    }
                    Text {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.margins: 9
                        text: gameTile.gameTitle
                        color: view.ink
                        font.pixelSize: gameTile.current ? 11 : 9
                        font.weight: Font.Bold
                        maximumLineCount: 2
                        wrapMode: Text.WordWrap
                        elide: Text.ElideRight
                    }
                    Rectangle {
                        anchors.top: parent.top
                        anchors.right: parent.right
                        anchors.margins: 8
                        width: 10
                        height: 10
                        radius: 5
                        visible: gameTile.gameLocal || gameTile.gameDownloadable
                        color: gameTile.gameLocal ? view.accentCool : view.accent
                    }
                    Text {
                        anchors.centerIn: parent
                        text: gameTile.gameTitle.length > 0
                              ? gameTile.gameTitle.charAt(0).toUpperCase() : "?"
                        color: "#d9ffffff"
                        font.pixelSize: 48
                        font.weight: Font.Black
                        visible: tileCover.status !== Image.Ready
                    }
                    HoverHandler { id: shelfHover }
                    TapHandler {
                        onTapped: {
                            view.noteActivity()
                            view.navigationZone = 2
                            shelf.currentIndex = gameTile.index
                            view.forceActiveFocus()
                        }
                    }
                }
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
                  ? "D-PAD / STICK  MOVE" : "← →  BROWSE"
            color: view.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
        Text {
            text: view.gamepad.connected_count > 0
                  ? view.gamepad.button_label("accept") + "  SELECT"
                  : "↑ ↓  MOVE"
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
            text: "A  ATTRACT"
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
        color: "#080b11"

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
                GradientStop { position: 0; color: "#f5080b11" }
                GradientStop { position: 0.46; color: "#b8080b11" }
                GradientStop { position: 0.78; color: "#52080b11" }
                GradientStop { position: 1; color: "#a8080b11" }
            }
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: parent.height * 0.58
            gradient: Gradient {
                GradientStop { position: 0; color: "#00080b11" }
                GradientStop { position: 0.34; color: "#b7080b11" }
                GradientStop { position: 1; color: "#ff080b11" }
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
                radius: 13
                color: view.accent
                Text {
                    anchors.centerIn: parent
                    text: "L"
                    color: "#10141c"
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
            radius: 11
            color: "#9a121923"
            border.color: "#68768494"
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
            radius: 24
            color: "#97000000"
            rotation: 2.5

            Rectangle {
                id: attractCoverSurface
                anchors.fill: parent
                anchors.margins: 10
                radius: 20
                color: view.accentFor(view.selectedTitle)
                border.color: "#8dffffff"
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
                    color: "#d9ffffff"
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
                color: "#d2d8e1"
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
                    radius: 10
                    color: view.selectedLocal ? "#5130584f"
                           : view.selectedDownloadable ? "#554b3d27" : "#4c28313d"
                    border.color: view.selectedLocal ? view.accentCool
                                  : view.selectedDownloadable ? view.accent : "#657282"
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
                    radius: 10
                    color: "#554b3d27"
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
            color: "#cf0a0f16"

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 2
                color: "#52606f7f"
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
        id: platformWheelOverlay
        anchors.fill: parent
        z: 45
        visible: view.platformWheelOpen
        color: "#ec070a0f"

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: "#f60a0e15" }
                GradientStop { position: 0.55; color: "#e20a0e15" }
                GradientStop { position: 1; color: "#c20a0e15" }
            }
        }

        Rectangle {
            id: platformWheelPanel
            anchors.centerIn: parent
            width: Math.min(880, parent.width - 180)
            height: Math.min(860, parent.height - 120)
            radius: 26
            color: "#f7131923"
            border.color: "#71808f9f"
            border.width: 1
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 118
                color: "#d918202c"

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
                    color: platformCloseHover.hovered ? "#54313b49" : "#30232c38"
                    border.color: platformCloseHover.hovered ? view.accent : "#596574"
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
                cacheBuffer: height
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
                        color: platformRow.selected ? "#e6333c49"
                              : platformHover.hovered ? "#55303a47" : "transparent"
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
                            border.color: "#82ffffff"
                            Text {
                                anchors.centerIn: parent
                                text: platformRow.platformName.length > 0
                                      ? platformRow.platformName.charAt(0).toUpperCase() : "?"
                                color: "#f8ffffff"
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
                color: "#dc101620"

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
        color: "#e8070a0f"

        Rectangle {
            id: overlayPanel
            anchors.centerIn: parent
            width: Math.min(1180, parent.width - 120)
            height: Math.min(790, parent.height - 110)
            radius: 24
            color: "#f5121822"
            border.color: "#6b778595"
            border.width: 1
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: 350
                color: "#b90c1119"

                Rectangle {
                    id: overlayCover
                    anchors.top: parent.top
                    anchors.topMargin: 42
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 220
                    height: 304
                    radius: 16
                    color: view.accentFor(view.selectedTitle)
                    border.color: "#8cffffff"
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
                        color: "#ddffffff"
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
                        radius: 9
                        color: view.selectedLocal ? "#351f554d"
                               : view.selectedDownloadable ? "#3f583a22" : "#38232c38"
                        border.color: view.selectedLocal ? view.accentCool
                                      : view.selectedDownloadable ? view.accent : "#596574"
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
                        radius: 12
                        color: overlayCloseHover.hovered ? "#54313b49" : "#2e222b37"
                        border.color: overlayCloseHover.hovered ? view.accent : "#596574"
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
                            radius: 11
                            color: selected ? "#3d4b362b" : "#29222a34"
                            border.color: selected ? view.accent : "#4c596675"
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
                                color: "#dde2e9"
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
                                        { label: "RATING", value: view.details.rating.length > 0
                                                                  ? "★ " + view.details.rating : "" },
                                        { label: "AGE RATING", value: view.details.esrb },
                                        { label: "RELEASE TYPE", value: view.details.release_type }
                                    ]
                                    delegate: Rectangle {
                                        id: factCard
                                        required property var modelData
                                        visible: String(modelData.value).length > 0
                                        width: (detailsContent.width - 12) / 2
                                        height: visible ? 66 : 0
                                        radius: 12
                                        color: "#66202935"
                                        border.color: "#48546370"
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
                                radius: 14
                                color: "#66202935"
                                border.color: "#48546370"
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
                                radius: 14
                                color: "#66202935"
                                border.color: "#48546370"
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
                                        radius: 9
                                        color: "#3136473e"
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
                                radius: 14
                                color: selected ? "#5946382c" : menuHover.hovered
                                       ? "#43303a47" : "#65202935"
                                border.color: selected ? view.accent : "#48546370"
                                border.width: selected ? 2 : 1
                                scale: selected ? 1.012 : 1
                                Behavior on scale { NumberAnimation { duration: 90 } }

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
                                          : menuAction.index === 3 ? "◈" : "×"
                                    color: menuAction.selected ? view.accent : view.muted
                                    font.pixelSize: menuAction.index === 1 ? 24 : 18
                                    font.weight: Font.Bold
                                }
                                HoverHandler { id: menuHover }
                                TapHandler {
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
    }

    Connections {
        target: view.gamepad
        function onNavigation_revisionChanged() {
            if (view.active)
                view.handleNavigation(view.gamepad.navigation_action)
        }
    }
}
