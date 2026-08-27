pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import Lunchbox

ApplicationWindow {
    id: root
    visible: true
    width: 1440
    height: 900
    minimumWidth: 1040
    minimumHeight: 680
    title: "Lunchbox"
    color: palette.window

    readonly property color ink: "#f4f7fb"
    readonly property color muted: "#8d99aa"
    readonly property color panel: "#131923"
    readonly property color panelRaised: "#1a2230"
    readonly property color line: "#283244"
    readonly property color accent: "#ffb454"
    readonly property color accentCool: "#62d6c6"
    property string selectedPlatform: ""
    property string availability: ""
    property bool gridMode: true
    property string selectedGameId: ""
    property int selectedDatabaseId: 0
    property url selectedArtworkUrl: ""
    property url selectedFanartUrl: ""
    property string selectedArtworkSource: ""
    property string directoryTarget: ""
    readonly property var artworkChoices: [
        { key: "box-front", label: "Box front" },
        { key: "box-back", label: "Box back" },
        { key: "box-3d", label: "3D box" },
        { key: "screenshot", label: "Gameplay" },
        { key: "title-screen", label: "Title screen" },
        { key: "fanart", label: "Fan art" },
        { key: "clear-logo", label: "Clear logo" }
    ]
    readonly property int activeFilterCount: (availability.length > 0 ? 1 : 0)
                                               + (library.hide_non_retail ? 1 : 0)
                                               + (library.hide_adult ? 1 : 0)
    readonly property bool importUiProbe: Qt.application.arguments.indexOf("--import-ui-probe") >= 0
    readonly property bool importCommitProbe: Qt.application.arguments.indexOf("--import-commit-probe") >= 0
    readonly property bool filterUiProbe: Qt.application.arguments.indexOf("--filter-ui-probe") >= 0
    readonly property bool artworkUiProbe: Qt.application.arguments.indexOf("--artwork-ui-probe") >= 0

    palette.window: "#0c1119"
    palette.windowText: ink
    palette.text: ink
    palette.buttonText: ink
    palette.highlight: accent

    function scheduleFilter() {
        filterDelay.restart()
    }

    function selectLibrary(availabilityKey) {
        selectedPlatform = ""
        availability = availabilityKey
        scheduleFilter()
    }

    function platformHeading() {
        if (selectedPlatform.length > 0)
            return selectedPlatform
        if (availability === "local")
            return "My Collection"
        if (availability === "downloadable")
            return "Minerva Downloads"
        return "All Games"
    }

    function accentFor(value) {
        const colors = ["#f4775f", "#5f9df7", "#9b7cf7", "#4bc29b", "#e39f45", "#e26fa0"]
        if (!value || value.length === 0)
            return colors[0]
        return colors[value.charCodeAt(0) % colors.length]
    }

    function openGame(gameId, databaseId, title, platform, local, downloadable) {
        selectedGameId = gameId
        selectedDatabaseId = databaseId
        refreshSelectedArtwork()
        gameDetails.select_game(gameId, title, platform, local, downloadable)
    }

    function refreshSelectedArtwork() {
        selectedArtworkUrl = library.artwork_url(selectedDatabaseId,
                                                 library.artwork_type)
        selectedFanartUrl = library.exact_artwork_url(selectedDatabaseId, "fanart")
        selectedArtworkSource = selectedFanartUrl.toString().length > 0
                                ? library.artwork_source(selectedDatabaseId, "fanart")
                                : library.artwork_source(selectedDatabaseId,
                                                         library.artwork_type)
    }

    function artworkLabel(key) {
        for (let index = 0; index < artworkChoices.length; ++index) {
            if (artworkChoices[index].key === key)
                return artworkChoices[index].label
        }
        return "Artwork"
    }

    function artworkIndex(key) {
        for (let index = 0; index < artworkChoices.length; ++index) {
            if (artworkChoices[index].key === key)
                return index
        }
        return 0
    }

    function wideArtwork(key) {
        return key === "screenshot" || key === "title-screen" || key === "fanart"
    }

    function openImportDialog() {
        importDialogLoader.active = true
        importDialogLoader.item.open()
    }

    LibraryModel {
        id: library
    }

    GameDetailsModel {
        id: gameDetails
    }

    SettingsModel {
        id: appSettings
    }

    DownloadQueueModel {
        id: downloadQueue
    }

    LocalImportModel {
        id: localImport
    }

    Connections {
        target: library
        function onReadyChanged() {
            if (library.ready) {
                if (library.catalog_probe)
                    Qt.quit()
                else if (library.filter_probe)
                    library.apply_filter("mario", "", "downloadable")
                else {
                    root.scheduleFilter()
                    if (root.filterUiProbe)
                        filterPopup.open()
                }
            }
        }
        function onFilteringChanged() {
            if (library.filter_probe && library.ready && !library.filtering)
                Qt.quit()
        }
        function onMedia_revisionChanged() {
            if (library.media_probe)
                Qt.quit()
            else if (root.artworkUiProbe && library.media_revision === 1) {
                searchField.text = "A Nightmare on Elm Street"
                library.apply_filter(searchField.text, "", "")
                viewPopup.open()
            } else if (root.selectedGameId.length > 0)
                root.refreshSelectedArtwork()
        }
        function onArtwork_typeChanged() {
            if (root.selectedGameId.length > 0)
                root.refreshSelectedArtwork()
        }
    }

    Connections {
        target: gameDetails
        function onDownload_busyChanged() {
            if (!gameDetails.download_busy)
                downloadQueue.refresh()
        }
    }

    Connections {
        target: downloadQueue
        function onCollection_revisionChanged() {
            if (downloadQueue.collection_revision > 0)
                library.reload()
        }
    }

    Connections {
        target: localImport
        function onCollection_revisionChanged() {
            if (localImport.collection_revision > 0) {
                root.selectLibrary("local")
                library.reload()
            }
        }
        function onInitializedChanged() {
            if (localImport.initialized && root.importUiProbe
                    && localImport.directory.length > 0)
                localImport.start_scan("", true)
        }
        function onResult_countChanged() {
            if (root.importCommitProbe && localImport.result_count > 0
                    && localImport.selected_count === 0) {
                localImport.select_visible("all")
                localImport.import_selected()
            }
        }
    }

    Timer {
        id: startupTimer
        interval: 0
        running: true
        repeat: false
        onTriggered: {
            library.shell_ready()
            if (library.startup_probe)
                Qt.quit()
            else if (root.importUiProbe)
                root.openImportDialog()
        }
    }

    Timer {
        id: catalogTimer
        interval: 0
        running: true
        repeat: false
        onTriggered: {
            library.initialize()
            appSettings.initialize()
            downloadQueue.initialize()
            localImport.initialize()
        }
    }

    Timer {
        interval: 2000
        running: downloadQueue.active_count > 0 || downloadsDrawer.opened
        repeat: true
        onTriggered: downloadQueue.refresh()
    }

    Timer {
        id: filterDelay
        interval: 75
        repeat: false
        onTriggered: library.apply_filter(searchField.text, root.selectedPlatform,
                                           root.availability)
    }

    Popup {
        id: filterPopup
        parent: Overlay.overlay
        x: Math.max(sidebar.width + 18,
                    root.width - detailsPane.width - width - 28)
        y: header.height + 76
        width: 326
        padding: 10
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnReleaseOutside
        implicitHeight: filterColumn.implicitHeight + topPadding + bottomPadding

        background: Rectangle {
            radius: 12
            color: "#111925"
            border.color: root.line
            border.width: 1
        }
        contentItem: Column {
            id: filterColumn
            spacing: 2
            Item {
                width: parent.width
                height: 40
                Text {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    text: "LIBRARY FILTERS"
                    color: root.ink
                    font.pixelSize: 11
                    font.weight: Font.Bold
                    font.letterSpacing: 1
                }
                Button {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Clear"
                    flat: true
                    enabled: root.activeFilterCount > 0
                    onClicked: {
                        library.set_content_filters(false, false)
                        root.selectLibrary("")
                    }
                }
            }
            FilterToggle {
                label: "Installed"
                description: "Games already present in My Collection"
                checked: root.availability === "local"
                onToggled: root.selectLibrary(checked ? "" : "local")
            }
            FilterToggle {
                label: "Downloadable from Minerva"
                description: "Not installed and covered by an exact platform offer"
                checked: root.availability === "downloadable"
                onToggled: root.selectLibrary(checked ? "" : "downloadable")
            }
            Rectangle {
                width: parent.width
                height: 1
                color: root.line
            }
            FilterToggle {
                label: "Hide non-retail"
                description: "Homebrew, hacks, unlicensed and pirate releases"
                checked: library.hide_non_retail
                onToggled: {
                    library.set_content_filters(!checked, library.hide_adult)
                    root.scheduleFilter()
                }
            }
            FilterToggle {
                label: "Hide adult"
                description: "Adult-only ratings, genres and explicit title tags"
                checked: library.hide_adult
                onToggled: {
                    library.set_content_filters(library.hide_non_retail, !checked)
                    root.scheduleFilter()
                }
            }
            Text {
                width: parent.width
                topPadding: 7
                bottomPadding: 3
                text: "Filters combine with search and platform selection. Minerva always excludes installed games."
                color: "#6f7c90"
                font.pixelSize: 9
                wrapMode: Text.WordWrap
            }
        }
    }

    Popup {
        id: viewPopup
        parent: Overlay.overlay
        x: Math.max(sidebar.width + 18,
                    root.width - detailsPane.width - width - 28)
        y: header.height + 76
        width: 334
        padding: 16
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnReleaseOutside
        implicitHeight: viewColumn.implicitHeight + topPadding + bottomPadding
        onOpened: {
            artworkCombo.currentIndex = root.artworkIndex(library.artwork_type)
            zoomSlider.value = library.grid_zoom
        }

        background: Rectangle {
            radius: 12
            color: "#111925"
            border.color: root.line
            border.width: 1
        }

        contentItem: Column {
            id: viewColumn
            spacing: 11
            Text {
                text: "LIBRARY APPEARANCE"
                color: root.ink
                font.pixelSize: 11
                font.weight: Font.Bold
                font.letterSpacing: 1
            }
            Text {
                text: "ARTWORK"
                color: "#687488"
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1
            }
            ComboBox {
                id: artworkCombo
                width: parent.width
                height: 40
                model: root.artworkChoices
                textRole: "label"
                currentIndex: root.artworkIndex(library.artwork_type)
                onActivated: library.set_presentation_preferences(
                                 root.artworkChoices[index].key,
                                 Math.round(zoomSlider.value))
            }
            Row {
                width: parent.width
                spacing: 8
                Text {
                    text: "CARD SIZE"
                    color: "#687488"
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 1
                }
                Text {
                    width: parent.width - 76
                    text: Math.round(zoomSlider.value) + "%"
                    horizontalAlignment: Text.AlignRight
                    color: root.ink
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                }
            }
            Row {
                width: parent.width
                spacing: 9
                Button {
                    width: 35
                    height: 35
                    text: "−"
                    enabled: zoomSlider.value > zoomSlider.from
                    onClicked: {
                        const next = Math.max(zoomSlider.from, zoomSlider.value - 10)
                        zoomSlider.value = next
                        library.set_presentation_preferences(library.artwork_type,
                                                             Math.round(next))
                    }
                }
                Slider {
                    id: zoomSlider
                    width: parent.width - 88
                    anchors.verticalCenter: parent.verticalCenter
                    from: 50
                    to: 200
                    stepSize: 5
                    value: library.grid_zoom
                    snapMode: Slider.SnapAlways
                    onPressedChanged: {
                        if (!pressed && Math.round(value) !== library.grid_zoom)
                            library.set_presentation_preferences(
                                        library.artwork_type, Math.round(value))
                    }
                }
                Button {
                    width: 35
                    height: 35
                    text: "+"
                    enabled: zoomSlider.value < zoomSlider.to
                    onClicked: {
                        const next = Math.min(zoomSlider.to, zoomSlider.value + 10)
                        zoomSlider.value = next
                        library.set_presentation_preferences(library.artwork_type,
                                                             Math.round(next))
                    }
                }
            }
            Rectangle {
                width: parent.width
                height: 1
                color: root.line
            }
            Text {
                width: parent.width
                text: library.media_loading
                      ? "Indexing artwork without blocking the library…"
                      : library.media_asset_count > 0
                        ? library.media_asset_count + " preferred assets for "
                          + library.media_game_count + " games"
                        : "No local media cache found. Cards use a fast generated fallback."
                color: root.muted
                font.pixelSize: 10
                wrapMode: Text.WordWrap
            }
            Text {
                width: parent.width
                visible: library.media_directory.length > 0
                text: library.media_directory
                color: "#657186"
                font.pixelSize: 9
                elide: Text.ElideMiddle
            }
        }
    }

    component HeaderButton: Button {
        id: control
        property bool active: false
        implicitHeight: 38
        leftPadding: 16
        rightPadding: 16
        font.pixelSize: 13
        font.weight: Font.DemiBold
        background: Rectangle {
            radius: 9
            color: control.down ? "#303b4d" : control.active ? "#34303a" : "#1b2330"
            border.color: control.active ? root.accent : root.line
            border.width: 1
        }
        contentItem: Text {
            text: control.text
            color: control.active ? root.accent : root.ink
            font: control.font
            verticalAlignment: Text.AlignVCenter
            horizontalAlignment: Text.AlignHCenter
        }
    }

    component NavButton: Rectangle {
        id: nav
        required property string label
        required property string glyph
        property string count: ""
        property bool active: false
        signal clicked()
        width: ListView.view ? ListView.view.width : 228
        height: 43
        radius: 9
        color: active ? "#272c34" : hover.hovered ? "#1b2330" : "transparent"
        border.color: active ? "#443b31" : "transparent"

        HoverHandler { id: hover }
        TapHandler { onTapped: nav.clicked() }

        Rectangle {
            visible: nav.active
            width: 3
            height: 23
            radius: 2
            color: root.accent
            anchors.left: parent.left
            anchors.leftMargin: 1
            anchors.verticalCenter: parent.verticalCenter
        }
        Text {
            text: nav.glyph
            width: 28
            anchors.left: parent.left
            anchors.leftMargin: 13
            anchors.verticalCenter: parent.verticalCenter
            color: nav.active ? root.accent : root.muted
            font.pixelSize: 16
            horizontalAlignment: Text.AlignHCenter
        }
        Text {
            text: nav.label
            anchors.left: parent.left
            anchors.leftMargin: 54
            anchors.right: countText.left
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            elide: Text.ElideRight
            color: nav.active ? root.ink : "#c0c8d4"
            font.pixelSize: 14
            font.weight: nav.active ? Font.DemiBold : Font.Medium
        }
        Text {
            id: countText
            text: nav.count
            anchors.right: parent.right
            anchors.rightMargin: 13
            anchors.verticalCenter: parent.verticalCenter
            color: root.muted
            font.pixelSize: 12
            font.features: { "tnum": 1 }
        }
    }

    component StatusPill: Rectangle {
        id: pill
        required property string label
        required property string value
        implicitWidth: statusRow.implicitWidth + 24
        implicitHeight: 34
        radius: 9
        color: "#151d29"
        border.color: root.line
        Row {
            id: statusRow
            anchors.centerIn: parent
            spacing: 7
            Text {
                text: pill.value
                color: root.ink
                font.pixelSize: 13
                font.weight: Font.Bold
            }
            Text {
                text: pill.label
                color: root.muted
                font.pixelSize: 12
            }
        }
    }

    component FilterToggle: Rectangle {
        id: filterToggle
        required property string label
        required property string description
        property bool checked: false
        signal toggled()
        width: filterPopup.availableWidth
        height: 58
        radius: 8
        color: filterHover.hovered ? "#1b2432" : "transparent"

        HoverHandler { id: filterHover }
        TapHandler { onTapped: filterToggle.toggled() }

        Rectangle {
            anchors.left: parent.left
            anchors.leftMargin: 11
            anchors.verticalCenter: parent.verticalCenter
            width: 18
            height: 18
            radius: 5
            color: filterToggle.checked ? root.accentCool : "#101721"
            border.color: filterToggle.checked ? root.accentCool : "#455166"
            Text {
                anchors.centerIn: parent
                text: filterToggle.checked ? "✓" : ""
                color: "#0c1716"
                font.pixelSize: 13
                font.weight: Font.Black
            }
        }
        Column {
            anchors.left: parent.left
            anchors.leftMargin: 42
            anchors.right: parent.right
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text {
                width: parent.width
                text: filterToggle.label
                color: root.ink
                font.pixelSize: 12
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: filterToggle.description
                color: root.muted
                font.pixelSize: 9
                elide: Text.ElideRight
            }
        }
    }

    component GameGrid: GridView {
        id: grid
        reuseItems: true
        clip: true
        cacheBuffer: height
        keyNavigationEnabled: true
        activeFocusOnTab: true
        boundsBehavior: Flickable.StopAtBounds
        property real preferredWidth: (root.width >= 1500 ? 230 : 205)
                                      * library.grid_zoom / 100
        cellWidth: Math.floor(width / Math.max(1, Math.floor(width / preferredWidth)))
        cellHeight: Math.round(cellWidth * 1.36)
        model: library
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        delegate: Item {
            id: tile
            required property int index
            required property string gameId
            required property string gameTitle
            required property string gamePlatform
            required property bool gameLocal
            required property bool gameDownloadable
            required property int gameDatabaseId
            property int mediaRevision: library.media_revision
            property url artworkUrl: {
                mediaRevision
                return library.artwork_url(gameDatabaseId, library.artwork_type)
            }
            property string artworkSource: {
                mediaRevision
                return library.artwork_source(gameDatabaseId, library.artwork_type)
            }
            width: grid.cellWidth
            height: grid.cellHeight
            activeFocusOnTab: true

            TapHandler {
                onTapped: {
                    tile.forceActiveFocus()
                    root.openGame(tile.gameId, tile.gameDatabaseId,
                                  tile.gameTitle, tile.gamePlatform,
                                  tile.gameLocal, tile.gameDownloadable)
                }
            }
            Keys.onPressed: event => {
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                        || event.key === Qt.Key_Space) {
                    root.openGame(tile.gameId, tile.gameDatabaseId,
                                  tile.gameTitle, tile.gamePlatform,
                                  tile.gameLocal, tile.gameDownloadable)
                    event.accepted = true
                }
            }

            Rectangle {
                id: card
                anchors.fill: parent
                anchors.margins: 8
                radius: 12
                color: tile.activeFocus ? "#263142" : cardHover.hovered ? "#202a39" : root.panelRaised
                border.color: tile.activeFocus || root.selectedGameId === tile.gameId ? root.accent : cardHover.hovered ? "#3a485d" : root.line
                border.width: tile.activeFocus || root.selectedGameId === tile.gameId ? 2 : 1
                scale: cardHover.hovered ? 1.012 : 1
                Behavior on scale { NumberAnimation { duration: 90 } }

                Rectangle {
                    id: artwork
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 9
                    height: parent.height - 79
                    radius: 9
                    color: root.accentFor(tile.gameTitle)
                    clip: true

                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.lighter(artwork.color, 1.17) }
                        GradientStop { position: 1; color: Qt.darker(artwork.color, 1.55) }
                    }

                    Image {
                        id: coverImage
                        anchors.fill: parent
                        anchors.margins: 1
                        source: tile.artworkUrl
                        asynchronous: true
                        cache: true
                        mipmap: true
                        autoTransform: true
                        fillMode: root.wideArtwork(library.artwork_type)
                                  ? Image.PreserveAspectCrop : Image.PreserveAspectFit
                        sourceSize.width: Math.max(1, Math.round(width * 2))
                        sourceSize.height: Math.max(1, Math.round(height * 2))
                        opacity: status === Image.Ready ? 1 : 0
                        Behavior on opacity { NumberAnimation { duration: 130 } }
                    }

                    Text {
                        anchors.centerIn: parent
                        width: parent.width - 28
                        text: tile.gameTitle.length > 0 ? tile.gameTitle.charAt(0).toUpperCase() : "?"
                        horizontalAlignment: Text.AlignHCenter
                        color: "#ddffffff"
                        font.pixelSize: Math.min(72, parent.height * 0.38)
                        font.weight: Font.Black
                        visible: coverImage.status !== Image.Ready
                    }
                    Rectangle {
                        visible: tile.gameLocal || tile.gameDownloadable
                        anchors.top: parent.top
                        anchors.right: parent.right
                        anchors.margins: 9
                        implicitWidth: badgeText.implicitWidth + 14
                        implicitHeight: 25
                        radius: 7
                        color: tile.gameLocal ? "#d91d3d35" : "#d9303540"
                        border.color: tile.gameLocal ? root.accentCool : root.accent
                        Text {
                            id: badgeText
                            anchors.centerIn: parent
                            text: tile.gameLocal ? "READY" : "GET"
                            color: tile.gameLocal ? root.accentCool : root.accent
                            font.pixelSize: 10
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8
                        }
                    }
                }
                ToolTip.visible: cardHover.hovered && tile.artworkSource.length > 0
                ToolTip.text: root.artworkLabel(library.artwork_type)
                              + " · " + tile.artworkSource

                Text {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    anchors.top: artwork.bottom
                    anchors.topMargin: 10
                    text: tile.gameTitle
                    color: root.ink
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
                Text {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 9
                    text: tile.gamePlatform.length > 0 ? tile.gamePlatform : "Unassigned platform"
                    color: root.muted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }
            HoverHandler { id: cardHover }
        }
    }

    component GameList: ListView {
        id: list
        reuseItems: true
        clip: true
        cacheBuffer: height
        spacing: 6
        model: library
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
        delegate: Rectangle {
            id: row
            required property int index
            required property string gameId
            required property string gameTitle
            required property string gamePlatform
            required property string gameStatus
            required property bool gameLocal
            required property bool gameDownloadable
            required property int gameDatabaseId
            property int mediaRevision: library.media_revision
            property url artworkUrl: {
                mediaRevision
                return library.artwork_url(gameDatabaseId, library.artwork_type)
            }
            width: list.width - 8
            height: 62
            radius: 9
            activeFocusOnTab: true
            color: row.activeFocus ? "#263142" : rowHover.hovered ? "#202a39" : root.panelRaised
            border.color: row.activeFocus || root.selectedGameId === row.gameId ? root.accent : rowHover.hovered ? "#3a485d" : root.line
            border.width: row.activeFocus || root.selectedGameId === row.gameId ? 2 : 1
            HoverHandler { id: rowHover }
            TapHandler {
                onTapped: {
                    row.forceActiveFocus()
                    root.openGame(row.gameId, row.gameDatabaseId,
                                  row.gameTitle, row.gamePlatform,
                                  row.gameLocal, row.gameDownloadable)
                }
            }
            Keys.onPressed: event => {
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                        || event.key === Qt.Key_Space) {
                    root.openGame(row.gameId, row.gameDatabaseId,
                                  row.gameTitle, row.gamePlatform,
                                  row.gameLocal, row.gameDownloadable)
                    event.accepted = true
                }
            }
            Rectangle {
                id: listArtwork
                width: 38
                height: 38
                radius: 8
                anchors.left: parent.left
                anchors.leftMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                color: root.accentFor(row.gameTitle)
                clip: true
                Image {
                    id: listImage
                    anchors.fill: parent
                    source: row.artworkUrl
                    asynchronous: true
                    cache: true
                    mipmap: true
                    autoTransform: true
                    fillMode: Image.PreserveAspectCrop
                    sourceSize.width: 76
                    sourceSize.height: 76
                    opacity: status === Image.Ready ? 1 : 0
                }
                Text {
                    anchors.centerIn: parent
                    text: row.gameTitle.length > 0 ? row.gameTitle.charAt(0).toUpperCase() : "?"
                    color: "white"
                    font.weight: Font.Black
                    font.pixelSize: 18
                    visible: listImage.status !== Image.Ready
                }
            }
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 62
                anchors.right: stateLabel.left
                anchors.rightMargin: 20
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3
                Text {
                    width: parent.width
                    text: row.gameTitle
                    color: root.ink
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
                Text {
                    width: parent.width
                    text: row.gamePlatform.length > 0 ? row.gamePlatform : "Unassigned platform"
                    color: root.muted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }
            Text {
                id: stateLabel
                anchors.right: parent.right
                anchors.rightMargin: 18
                anchors.verticalCenter: parent.verticalCenter
                text: row.gameLocal ? "INSTALLED" : row.gameDownloadable ? "AVAILABLE" : row.gameStatus.toUpperCase()
                color: row.gameLocal ? root.accentCool : row.gameDownloadable ? root.accent : root.muted
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
        }
    }

    Rectangle {
        id: header
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 70
        color: "#101620"
        border.color: root.line

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 22
            anchors.verticalCenter: parent.verticalCenter
            spacing: 11
            Rectangle {
                width: 35
                height: 35
                radius: 10
                color: root.accent
                Text {
                    anchors.centerIn: parent
                    text: "L"
                    color: "#18120b"
                    font.pixelSize: 20
                    font.weight: Font.Black
                }
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: -2
                Text {
                    text: "LUNCHBOX"
                    color: root.ink
                    font.pixelSize: 17
                    font.weight: Font.Black
                    font.letterSpacing: 1.2
                }
                Text {
                    text: "PLAY WITHOUT FRICTION"
                    color: root.muted
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    font.letterSpacing: 1.1
                }
            }
        }

        TextField {
            id: searchField
            anchors.centerIn: parent
            width: Math.min(460, root.width * 0.34)
            height: 40
            leftPadding: 42
            rightPadding: 14
            placeholderText: "Search games and platforms"
            placeholderTextColor: "#687488"
            color: root.ink
            selectionColor: root.accent
            selectedTextColor: "#15100a"
            font.pixelSize: 13
            onTextEdited: root.scheduleFilter()
            background: Rectangle {
                radius: 11
                color: "#0a0f16"
                border.color: searchField.activeFocus ? root.accent : root.line
                border.width: searchField.activeFocus ? 2 : 1
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 15
                    anchors.verticalCenter: parent.verticalCenter
                    text: "⌕"
                    color: root.muted
                    font.pixelSize: 20
                }
            }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9
            HeaderButton {
                text: "Minerva  " + library.downloadable_game_count
                visible: root.width >= 1280
                active: root.availability === "downloadable"
                onClicked: root.selectLibrary(active ? "" : "downloadable")
            }
            HeaderButton {
                text: "↓  " + downloadQueue.active_count
                active: downloadsDrawer.opened
                onClicked: downloadsDrawer.open()
            }
            HeaderButton {
                text: "⚙"
                implicitWidth: 42
                leftPadding: 0
                rightPadding: 0
                onClicked: settingsDialog.open()
                ToolTip.visible: hovered
                ToolTip.text: "Settings"
            }
            HeaderButton {
                text: library.loading ? "…" : "↻"
                implicitWidth: 42
                leftPadding: 0
                rightPadding: 0
                enabled: !library.loading
                onClicked: library.reload()
                ToolTip.visible: hovered
                ToolTip.text: "Refresh library"
            }
        }
    }

    Rectangle {
        id: sidebar
        anchors.left: parent.left
        anchors.top: header.bottom
        anchors.bottom: statusBar.top
        width: 254
        color: root.panel
        border.color: root.line

        Column {
            id: libraryNav
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 13
            spacing: 3

            Text {
                text: "LIBRARY"
                color: "#687488"
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.4
                leftPadding: 13
                topPadding: 8
                bottomPadding: 7
            }
            NavButton {
                label: "All Games"
                glyph: "▦"
                count: library.game_count.toString()
                active: root.selectedPlatform === "" && root.availability === ""
                onClicked: root.selectLibrary("")
            }
            NavButton {
                label: "My Collection"
                glyph: "◆"
                count: library.local_game_count.toString()
                active: root.selectedPlatform === "" && root.availability === "local"
                onClicked: root.selectLibrary("local")
            }
            NavButton {
                label: "Minerva"
                glyph: "↓"
                count: library.downloadable_game_count.toString()
                active: root.availability === "downloadable"
                onClicked: root.selectLibrary("downloadable")
            }
            NavButton {
                label: "Import ROMs"
                glyph: "+"
                onClicked: root.openImportDialog()
            }
        }

        Text {
            id: platformsLabel
            anchors.left: parent.left
            anchors.leftMargin: 26
            anchors.top: libraryNav.bottom
            anchors.topMargin: 19
            text: "PLATFORMS"
            color: "#687488"
            font.pixelSize: 10
            font.weight: Font.Bold
            font.letterSpacing: 1.4
        }

        ListView {
            id: platformList
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: platformsLabel.bottom
            anchors.bottom: parent.bottom
            anchors.margins: 13
            anchors.topMargin: 10
            clip: true
            reuseItems: true
            spacing: 3
            model: library.platform_count
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            delegate: NavButton {
                required property int index
                property int revision: library.platform_revision
                label: {
                    revision
                    return library.platform_name_at(index)
                }
                glyph: "·"
                count: library.platform_game_count_at(index).toString()
                active: root.selectedPlatform === label
                onClicked: {
                    root.selectedPlatform = label
                    root.scheduleFilter()
                }
            }
        }
    }

    Item {
        id: content
        anchors.left: sidebar.right
        anchors.right: detailsPane.left
        anchors.top: header.bottom
        anchors.bottom: statusBar.top

        ColumnLayout {
            anchors.fill: parent
            anchors.leftMargin: 27
            anchors.rightMargin: 27
            anchors.topMargin: 23
            anchors.bottomMargin: 16
            spacing: 15

            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                Column {
                    Layout.fillWidth: true
                    spacing: 3
                    Text {
                        text: root.platformHeading()
                        color: root.ink
                        font.pixelSize: 27
                        font.weight: Font.Bold
                    }
                    Text {
                        text: library.filtering ? "Updating results…" : library.filtered_count + " games"
                        color: root.muted
                        font.pixelSize: 12
                    }
                }
                StatusPill {
                    visible: content.width > 700
                    label: "local files"
                    value: library.local_file_count.toString()
                }
                StatusPill {
                    visible: content.width > 700
                    label: "offers"
                    value: library.offer_count.toString()
                }
                StatusPill {
                    visible: content.width > 700
                    label: library.media_loading ? "artwork" : "with art"
                    value: library.media_loading ? "…" : library.media_game_count.toString()
                }
                HeaderButton {
                    text: root.activeFilterCount > 0
                          ? "Filters  " + root.activeFilterCount : "Filters"
                    active: root.activeFilterCount > 0 || filterPopup.opened
                    onClicked: filterPopup.opened ? filterPopup.close() : filterPopup.open()
                }
                HeaderButton {
                    text: "View"
                    active: viewPopup.opened
                    onClicked: viewPopup.opened ? viewPopup.close() : viewPopup.open()
                    ToolTip.visible: hovered
                    ToolTip.text: root.artworkLabel(library.artwork_type)
                                  + " · " + library.grid_zoom + "%"
                }
                Row {
                    spacing: 4
                    HeaderButton {
                        text: "▦"
                        active: root.gridMode
                        implicitWidth: 41
                        leftPadding: 0
                        rightPadding: 0
                        onClicked: root.gridMode = true
                    }
                    HeaderButton {
                        text: "☷"
                        active: !root.gridMode
                        implicitWidth: 41
                        leftPadding: 0
                        rightPadding: 0
                        onClicked: root.gridMode = false
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: root.line
            }

            Loader {
                id: gameViewLoader
                Layout.fillWidth: true
                Layout.fillHeight: true
                active: library.ready && library.filtered_count > 0
                sourceComponent: root.gridMode ? gameGridComponent : gameListComponent
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                visible: !gameViewLoader.active

                Column {
                    anchors.centerIn: parent
                    width: Math.min(580, parent.width - 60)
                    spacing: 13
                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 62
                        height: 62
                        radius: 19
                        color: "#1a2230"
                        border.color: root.line
                        Text {
                            anchors.centerIn: parent
                            text: library.loading ? "↻" : library.ready ? "◇" : "!"
                            color: library.loading ? root.accent : root.accentCool
                            font.pixelSize: 29
                            font.weight: Font.Bold
                        }
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: library.loading ? "Opening your library" :
                              !library.ready ? "Catalog unavailable" :
                              library.game_count === 0 ? "Your library is ready for games" :
                              "No games match these filters"
                        color: root.ink
                        font.pixelSize: 21
                        font.weight: Font.Bold
                    }
                    Text {
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        text: library.loading ? "The window stays responsive while SQLite is read on a worker thread." :
                              !library.ready ? library.status_message :
                              library.game_count === 0 ?
                                  "Add a discovery database or import local games. Minerva availability is layered over the catalog without treating provider filenames as canonical identities." :
                                  "Try a different platform, availability option, or search."
                        color: root.muted
                        font.pixelSize: 13
                        lineHeight: 1.35
                    }
                    Text {
                        visible: !library.ready
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        text: library.database_path.length > 0 ? library.database_path :
                              "Use --database PATH or LUNCHBOX_DATABASE"
                        color: "#657186"
                        font.pixelSize: 11
                        elide: Text.ElideMiddle
                    }
                }
            }
        }

        Component { id: gameGridComponent; GameGrid {} }
        Component { id: gameListComponent; GameList {} }
    }

    Rectangle {
        id: detailsPane
        anchors.right: parent.right
        anchors.top: header.bottom
        anchors.bottom: statusBar.top
        width: gameDetails.panel_open ? Math.min(440, Math.max(350, root.width * 0.32)) : 0
        visible: width > 0
        clip: true
        color: "#101620"
        border.color: root.line

        Behavior on width {
            NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
        }

        Rectangle {
            id: detailHeader
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 56
            color: root.panel
            border.color: root.line

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 20
                anchors.verticalCenter: parent.verticalCenter
                text: "GAME DETAILS"
                color: root.muted
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.4
            }
            RoundButton {
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                width: 34
                height: 34
                text: "×"
                flat: true
                font.pixelSize: 20
                onClicked: {
                    root.selectedGameId = ""
                    root.selectedDatabaseId = 0
                    root.selectedArtworkUrl = ""
                    root.selectedFanartUrl = ""
                    root.selectedArtworkSource = ""
                    gameDetails.close_panel()
                }
                ToolTip.visible: hovered
                ToolTip.text: "Close details"
            }
        }

        ScrollView {
            id: detailScroll
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: detailHeader.bottom
            anchors.bottom: parent.bottom
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical.policy: ScrollBar.AsNeeded

            Column {
                width: detailScroll.availableWidth
                spacing: 0

                Rectangle {
                    id: detailArtwork
                    width: parent.width
                    height: 214
                    color: root.accentFor(gameDetails.title)
                    clip: true
                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.lighter(detailArtwork.color, 1.15) }
                        GradientStop { position: 1; color: Qt.darker(detailArtwork.color, 1.8) }
                    }
                    Image {
                        id: detailImage
                        anchors.fill: parent
                        source: root.selectedFanartUrl.toString().length > 0
                                ? root.selectedFanartUrl : root.selectedArtworkUrl
                        asynchronous: true
                        cache: true
                        mipmap: true
                        autoTransform: true
                        fillMode: root.selectedFanartUrl.toString().length > 0
                                  ? Image.PreserveAspectCrop : Image.PreserveAspectFit
                        sourceSize.width: Math.max(1, Math.round(width * 2))
                        sourceSize.height: Math.max(1, Math.round(height * 2))
                        opacity: status === Image.Ready ? 1 : 0
                        Behavior on opacity { NumberAnimation { duration: 150 } }
                    }
                    Text {
                        anchors.centerIn: parent
                        text: gameDetails.title.length > 0 ? gameDetails.title.charAt(0).toUpperCase() : "?"
                        color: "#35ffffff"
                        font.pixelSize: 118
                        font.weight: Font.Black
                        visible: detailImage.status !== Image.Ready
                    }
                    Rectangle {
                        anchors.left: parent.left
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 112
                        gradient: Gradient {
                            GradientStop { position: 0; color: "transparent" }
                            GradientStop { position: 1; color: "#d9101620" }
                        }
                    }
                    Rectangle {
                        visible: detailImage.status === Image.Ready
                                 && root.selectedArtworkSource.length > 0
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.margins: 12
                        width: detailSource.implicitWidth + 16
                        height: 24
                        radius: 7
                        color: "#c9101620"
                        border.color: "#5b687d"
                        Text {
                            id: detailSource
                            anchors.centerIn: parent
                            text: root.selectedArtworkSource.toUpperCase()
                            color: "#d7deea"
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                        }
                    }
                    BusyIndicator {
                        anchors.centerIn: parent
                        running: gameDetails.loading
                        visible: running
                    }
                }

                Column {
                    x: 20
                    width: parent.width - 40
                    spacing: 12

                    Text {
                        width: parent.width
                        text: gameDetails.title
                        color: root.ink
                        font.pixelSize: 24
                        font.weight: Font.Bold
                        wrapMode: Text.WordWrap
                    }
                    Row {
                        width: parent.width
                        spacing: 8
                        Text {
                            text: gameDetails.platform.length > 0 ? gameDetails.platform : "Unassigned platform"
                            color: root.muted
                            font.pixelSize: 12
                        }
                        Rectangle {
                            width: stateText.implicitWidth + 14
                            height: 24
                            radius: 7
                            color: gameDetails.local ? "#1d3d35" : gameDetails.downloadable ? "#303540" : "#242b36"
                            border.color: gameDetails.local ? root.accentCool : gameDetails.downloadable ? root.accent : root.line
                            Text {
                                id: stateText
                                anchors.centerIn: parent
                                text: gameDetails.local ? "INSTALLED" : gameDetails.downloadable ? "MINERVA" : "CATALOG"
                                color: gameDetails.local ? root.accentCool : gameDetails.downloadable ? root.accent : root.muted
                                font.pixelSize: 9
                                font.weight: Font.Bold
                                font.letterSpacing: 0.7
                            }
                        }
                    }

                    Rectangle {
                        width: parent.width
                        height: statusMessage.implicitHeight + 22
                        radius: 9
                        color: "#151d29"
                        border.color: root.line
                        Row {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: 11
                            anchors.rightMargin: 11
                            spacing: 9
                            BusyIndicator {
                                id: statusBusy
                                width: 18
                                height: 18
                                running: gameDetails.loading || gameDetails.torrent_loading
                                visible: running
                            }
                            Text {
                                id: statusMessage
                                width: parent.width - (statusBusy.visible ? statusBusy.width + 9 : 0)
                                text: gameDetails.message
                                color: root.muted
                                font.pixelSize: 11
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    GridLayout {
                        width: parent.width
                        columns: 2
                        columnSpacing: 14
                        rowSpacing: 9
                        visible: !gameDetails.loading

                        Text { visible: gameDetails.release_date.length > 0; text: "RELEASED"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.release_date.length > 0; text: gameDetails.release_date; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.developer.length > 0; text: "DEVELOPER"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.developer.length > 0; text: gameDetails.developer; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.publisher.length > 0; text: "PUBLISHER"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.publisher.length > 0; text: gameDetails.publisher; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.genre.length > 0; text: "GENRE"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.genre.length > 0; text: gameDetails.genre; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.players.length > 0; text: "PLAYERS"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.players.length > 0; text: gameDetails.players; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.rating.length > 0; text: "RATING"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.rating.length > 0; text: gameDetails.rating + " / 5"; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.esrb.length > 0; text: "RATED"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.esrb.length > 0; text: gameDetails.esrb; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.release_type.length > 0; text: "TYPE"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.release_type.length > 0; text: gameDetails.release_type; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                    }

                    Text {
                        width: parent.width
                        visible: gameDetails.description.length > 0
                        text: gameDetails.description
                        color: "#c0c8d4"
                        font.pixelSize: 12
                        lineHeight: 1.35
                        wrapMode: Text.WordWrap
                    }

                    Text {
                        width: parent.width
                        visible: gameDetails.notes.length > 0
                        text: gameDetails.notes
                        color: root.muted
                        font.pixelSize: 10
                        lineHeight: 1.3
                        wrapMode: Text.WrapAnywhere
                    }

                    Rectangle {
                        visible: !gameDetails.loading && gameDetails.bundle_count > 0
                        width: parent.width
                        height: 1
                        color: root.line
                    }
                    Text {
                        visible: !gameDetails.loading && gameDetails.bundle_count > 0
                        width: parent.width
                        text: "MINERVA SOURCES"
                        color: "#687488"
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 1.2
                    }
                    Text {
                        visible: !gameDetails.loading && gameDetails.bundle_count > 0
                        width: parent.width
                        text: "Choose a verified platform bundle, then inspect title candidates before adding anything to the download queue."
                        color: root.muted
                        font.pixelSize: 11
                        lineHeight: 1.25
                        wrapMode: Text.WordWrap
                    }

                    Repeater {
                        model: gameDetails.bundle_count
                        delegate: Rectangle {
                            id: sourceRow
                            required property int index
                            property int revision: gameDetails.detail_revision
                            width: detailsPane.width - 40
                            height: 66
                            radius: 9
                            color: gameDetails.selected_bundle === index ? "#2a2e32" : sourceHover.hovered ? "#202a39" : root.panelRaised
                            border.color: gameDetails.selected_bundle === index ? root.accent : root.line
                            HoverHandler { id: sourceHover }
                            TapHandler { onTapped: gameDetails.load_bundle_files(sourceRow.index) }
                            Column {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.leftMargin: 12
                                anchors.rightMargin: 12
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4
                                Text {
                                    width: parent.width
                                    text: { sourceRow.revision; return gameDetails.bundle_title_at(sourceRow.index) }
                                    color: root.ink
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    elide: Text.ElideRight
                                }
                                Text {
                                    width: parent.width
                                    text: { sourceRow.revision; return gameDetails.bundle_detail_at(sourceRow.index) }
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }

                    Text {
                        visible: gameDetails.file_count > 0
                        width: parent.width
                        topPadding: 6
                        text: "TITLE CANDIDATES — REVIEW REQUIRED"
                        color: root.accent
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Repeater {
                        model: gameDetails.file_count
                        delegate: Rectangle {
                            id: fileRow
                            required property int index
                            property int revision: gameDetails.detail_revision
                            width: detailsPane.width - 40
                            height: Math.max(68, fileName.implicitHeight + fileInfo.implicitHeight + 24)
                            radius: 8
                            color: "#151d29"
                            border.color: root.line
                            Column {
                                anchors.left: parent.left
                                anchors.right: getButton.left
                                anchors.leftMargin: 11
                                anchors.rightMargin: 9
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4
                                Text {
                                    id: fileName
                                    width: parent.width
                                    text: { fileRow.revision; return gameDetails.file_name_at(fileRow.index) }
                                    color: root.ink
                                    font.pixelSize: 11
                                    wrapMode: Text.WrapAnywhere
                                }
                                Text {
                                    id: fileInfo
                                    width: parent.width
                                    text: { fileRow.revision; return gameDetails.file_detail_at(fileRow.index) }
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                            }
                            HeaderButton {
                                id: getButton
                                anchors.right: parent.right
                                anchors.rightMargin: 9
                                anchors.verticalCenter: parent.verticalCenter
                                text: gameDetails.download_busy ? "…" : "GET"
                                enabled: !gameDetails.download_busy && !gameDetails.torrent_loading
                                implicitWidth: 62
                                implicitHeight: 34
                                leftPadding: 8
                                rightPadding: 8
                                onClicked: gameDetails.queue_file(fileRow.index)
                            }
                        }
                    }

                    Item { width: 1; height: 10 }
                }
            }
        }
    }

    FolderDialog {
        id: directoryDialog
        title: root.directoryTarget === "rom" ? "Choose ROM library" : "Choose torrent library"
        onAccepted: appSettings.set_directory(root.directoryTarget, selectedFolder)
    }

    FolderDialog {
        id: importDirectoryDialog
        title: "Choose a ROM collection to scan"
        onAccepted: localImport.choose_directory(selectedFolder)
    }

    Loader {
        id: importDialogLoader
        anchors.fill: parent
        active: false
        sourceComponent: importDialogComponent
    }

    Component {
        id: importDialogComponent
        Dialog {
            id: importDialog
            modal: true
            anchors.centerIn: parent
            width: Math.min(1060, root.width - 50)
            height: Math.min(790, root.height - 42)
            padding: 0
            closePolicy: Popup.CloseOnEscape

            Timer {
                id: importFilterDelay
                interval: 75
                repeat: false
                onTriggered: localImport.apply_result_filter(
                                 importSearch.text,
                                 importStatus.currentValue,
                                 importSort.currentValue,
                                 importSortAscending.checked)
            }

            Connections {
                target: localImport
                function onResult_countChanged() {
                    if (localImport.result_count > 0)
                        importFilterDelay.restart()
                }
            }

            background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
            border.width: 1
        }

            header: Rectangle {
            width: parent.width
            height: 64
            color: root.panelRaised
            radius: 14
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 22
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                Text {
                    text: "IMPORT LOCAL ROMS"
                    color: root.ink
                    font.pixelSize: 17
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }
                Text {
                    text: "Review exact identities before adding files to My Collection"
                    color: root.muted
                    font.pixelSize: 10
                }
            }
            RoundButton {
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "×"
                flat: true
                font.pixelSize: 20
                onClicked: importDialog.close()
            }
        }

            contentItem: Item {
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 10

                RowLayout {
                Layout.fillWidth: true
                spacing: 9
                TextField {
                    Layout.fillWidth: true
                    text: localImport.directory
                    readOnly: true
                    placeholderText: "Choose a folder containing ROMs"
                }
                Button {
                    text: "Choose…"
                    enabled: !localImport.busy
                    onClicked: importDirectoryDialog.open()
                }
                ComboBox {
                    id: importPlatform
                    Layout.preferredWidth: 245
                    model: {
                        const values = ["Auto-detect platform"]
                        const revision = localImport.platform_count
                        for (let index = 0; index < revision; ++index)
                            values.push(localImport.platform_name_at(index))
                        return values
                    }
                }
                CheckBox {
                    id: importChecksums
                    text: "Exact checksums"
                    checked: true
                    enabled: !localImport.busy
                    ToolTip.visible: hovered
                    ToolTip.text: checked
                                      ? "Use SHA-1 and MD5 to prove catalog identity"
                                      : "Inventory files without assigning catalog identity"
                }
                HeaderButton {
                    text: localImport.scanning ? "Scanning…" : "Scan"
                    active: true
                    enabled: !localImport.busy && localImport.directory.length > 0
                    onClicked: localImport.start_scan(
                                   importPlatform.currentIndex === 0
                                       ? ""
                                       : localImport.platform_name_at(importPlatform.currentIndex - 1),
                                   importChecksums.checked)
                }
                Button {
                    text: "Cancel"
                    visible: localImport.scanning
                    onClicked: localImport.cancel_scan()
                }
                }

                Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 58
                radius: 9
                color: "#111824"
                border.color: root.line
                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 11
                    BusyIndicator {
                        visible: localImport.busy
                        running: visible
                        Layout.preferredWidth: 22
                        Layout.preferredHeight: 22
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        Text {
                            Layout.fillWidth: true
                            text: localImport.message
                            color: root.muted
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 6
                            visible: localImport.scanning
                            radius: 3
                            color: root.line
                            clip: true
                            Rectangle {
                                id: importProgressFill
                                height: parent.height
                                radius: 3
                                color: root.accentCool
                                width: localImport.total_files > 0
                                       ? parent.width * Math.min(1, localImport.scanned_files / localImport.total_files)
                                       : parent.width * 0.28
                                x: localImport.total_files > 0 ? 0 : -width
                                NumberAnimation on x {
                                    running: localImport.scanning && localImport.total_files === 0
                                    from: -importProgressFill.width
                                    to: importProgressFill.parent.width
                                    duration: 850
                                    loops: Animation.Infinite
                                }
                            }
                        }
                    }
                    Text {
                        visible: localImport.total_files > 0
                        text: localImport.matched_count + " exact  ·  "
                              + localImport.unmatched_count + " other"
                        color: root.accentCool
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                    }
                }
                }

                RowLayout {
                Layout.fillWidth: true
                visible: localImport.total_files > 0
                spacing: 8
                TextField {
                    id: importSearch
                    Layout.fillWidth: true
                    placeholderText: "Filter scan results"
                    onTextEdited: importFilterDelay.restart()
                }
                ComboBox {
                    id: importStatus
                    Layout.preferredWidth: 150
                    textRole: "label"
                    valueRole: "value"
                    model: [
                        { label: "All results", value: "all" },
                        { label: "Exact", value: "exact" },
                        { label: "Ambiguous", value: "ambiguous" },
                        { label: "Unmatched", value: "unmatched" },
                        { label: "Inventory only", value: "inventory_only" },
                        { label: "Errors", value: "error" }
                    ]
                    onActivated: importFilterDelay.restart()
                }
                ComboBox {
                    id: importSort
                    Layout.preferredWidth: 135
                    textRole: "label"
                    valueRole: "value"
                    model: [
                        { label: "File name", value: "name" },
                        { label: "Match status", value: "status" },
                        { label: "Platform", value: "platform" },
                        { label: "File size", value: "size" }
                    ]
                    currentIndex: 1
                    onActivated: importFilterDelay.restart()
                }
                ToolButton {
                    id: importSortAscending
                    checkable: true
                    checked: true
                    text: checked ? "↑" : "↓"
                    onClicked: importFilterDelay.restart()
                    ToolTip.visible: hovered
                    ToolTip.text: checked ? "Ascending" : "Descending"
                }
                Button {
                    text: "Exact"
                    onClicked: localImport.select_visible("exact")
                    ToolTip.visible: hovered
                    ToolTip.text: "Select only proven matches in the current results"
                }
                Button {
                    text: "All"
                    onClicked: localImport.select_visible("all")
                }
                Button {
                    text: "None"
                    onClicked: localImport.select_visible("none")
                }
                }

                Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 34
                visible: localImport.total_files > 0
                color: root.panelRaised
                border.color: root.line
                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 45
                    spacing: 0
                    Text { width: 250; anchors.verticalCenter: parent.verticalCenter; text: "ROM FILE"; color: root.muted; font.pixelSize: 9; font.weight: Font.Bold }
                    Text { width: 155; anchors.verticalCenter: parent.verticalCenter; text: "STATUS"; color: root.muted; font.pixelSize: 9; font.weight: Font.Bold }
                    Text { width: 190; anchors.verticalCenter: parent.verticalCenter; text: "PLATFORM"; color: root.muted; font.pixelSize: 9; font.weight: Font.Bold }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "MATCH / DETAILS"; color: root.muted; font.pixelSize: 9; font.weight: Font.Bold }
                }
                }

                ListView {
                id: importResults
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                reuseItems: true
                spacing: 3
                model: localImport.result_count
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                delegate: Rectangle {
                    id: importRow
                    required property int index
                    property int importRevision: localImport.revision
                    width: importResults.width
                    height: 61
                    radius: 7
                    color: rowHover.hovered ? "#202a38" : index % 2 ? "#141b26" : "#111824"
                    border.color: root.line
                    HoverHandler { id: rowHover }
                    ToolTip.visible: rowHover.hovered
                    ToolTip.text: { importRevision; return localImport.result_path_at(index) }
                    CheckBox {
                        anchors.left: parent.left
                        anchors.leftMargin: 9
                        anchors.verticalCenter: parent.verticalCenter
                        checked: { importRow.importRevision; return localImport.result_selected_at(importRow.index) }
                        onClicked: localImport.toggle_selected(importRow.index)
                    }
                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 45
                        anchors.verticalCenter: parent.verticalCenter
                        width: 240
                        text: { importRow.importRevision; return localImport.result_file_name_at(importRow.index) }
                        color: root.ink
                        font.pixelSize: 11
                        elide: Text.ElideMiddle
                    }
                    Rectangle {
                        anchors.left: parent.left
                        anchors.leftMargin: 295
                        anchors.verticalCenter: parent.verticalCenter
                        width: 125
                        height: 24
                        radius: 12
                        color: statusText.text === "EXACT" ? "#18302c"
                             : statusText.text === "ERROR" ? "#351d24" : "#292631"
                        border.color: statusText.text === "EXACT" ? root.accentCool
                                    : statusText.text === "ERROR" ? "#ef7182" : root.accent
                        Text {
                            id: statusText
                            anchors.centerIn: parent
                            text: { importRow.importRevision; return localImport.result_status_at(importRow.index) }
                            color: parent.border.color
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                        }
                    }
                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 430
                        anchors.verticalCenter: parent.verticalCenter
                        width: 180
                        text: { importRow.importRevision; return localImport.result_platform_at(importRow.index) }
                        color: root.muted
                        font.pixelSize: 10
                        elide: Text.ElideRight
                    }
                    Column {
                        anchors.left: parent.left
                        anchors.leftMargin: 620
                        anchors.right: sizeLabel.left
                        anchors.rightMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 3
                        Text {
                            width: parent.width
                            text: { importRow.importRevision; return localImport.result_title_at(importRow.index) }
                            color: root.ink
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: { importRow.importRevision; return localImport.result_detail_at(importRow.index) }
                            color: root.muted
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                    }
                    Text {
                        id: sizeLabel
                        anchors.right: parent.right
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        width: 72
                        horizontalAlignment: Text.AlignRight
                        text: { importRow.importRevision; return localImport.result_size_at(importRow.index) }
                        color: root.muted
                        font.pixelSize: 9
                    }
                }
                }

                RowLayout {
                Layout.fillWidth: true
                Text {
                    Layout.fillWidth: true
                    text: localImport.selected_count + " selected · unmatched files are imported as local-only games"
                    color: root.muted
                    font.pixelSize: 11
                }
                Button {
                    text: "Close"
                    onClicked: importDialog.close()
                }
                HeaderButton {
                    text: localImport.importing ? "Importing…" : "Import " + localImport.selected_count
                    active: true
                    enabled: !localImport.busy && localImport.selected_count > 0
                    onClicked: localImport.import_selected()
                }
                }
            }
            }
        }
    }

    Dialog {
        id: settingsDialog
        modal: true
        anchors.centerIn: parent
        width: Math.min(760, root.width - 60)
        height: Math.min(680, root.height - 60)
        padding: 0
        closePolicy: Popup.CloseOnEscape

        background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
            border.width: 1
        }

        header: Rectangle {
            width: parent.width
            height: 62
            color: root.panelRaised
            radius: 14
            Text {
                anchors.left: parent.left
                anchors.leftMargin: 22
                anchors.verticalCenter: parent.verticalCenter
                text: "SETTINGS"
                color: root.ink
                font.pixelSize: 17
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            RoundButton {
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "×"
                flat: true
                font.pixelSize: 20
                onClicked: settingsDialog.close()
            }
        }

        contentItem: ScrollView {
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ColumnLayout {
                x: 24
                width: settingsDialog.availableWidth - 48
                spacing: 11

                Text {
                    text: "QBITTORRENT"
                    color: root.accent
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.2
                }
                Text {
                    Layout.fillWidth: true
                    text: "Lunchbox uses the Web API and an isolated ‘lunchbox’ category. Credentials are stored in the operating system credential store, never in SQLite."
                    color: root.muted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 9
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Host"
                        text: appSettings.qbittorrent_host
                        onTextEdited: appSettings.qbittorrent_host = text
                    }
                    SpinBox {
                        Layout.preferredWidth: 125
                        from: 1
                        to: 65535
                        editable: true
                        value: appSettings.qbittorrent_port
                        onValueModified: appSettings.qbittorrent_port = value
                    }
                    Switch {
                        text: "HTTPS"
                        checked: appSettings.qbittorrent_use_https
                        onToggled: appSettings.qbittorrent_use_https = checked
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 9
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Username"
                        text: appSettings.qbittorrent_username
                        onTextEdited: appSettings.qbittorrent_username = text
                    }
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: appSettings.password_saved ? "Saved password (leave blank to keep)" : "Password"
                        echoMode: TextInput.Password
                        text: appSettings.qbittorrent_password
                        onTextEdited: appSettings.qbittorrent_password = text
                    }
                    Button {
                        visible: appSettings.password_saved
                        text: "Clear saved"
                        enabled: !appSettings.busy
                        onClicked: appSettings.clear_password()
                    }
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: root.line }
                Text {
                    text: "LIBRARY PATHS"
                    color: root.accent
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.2
                }
                Text {
                    Layout.fillWidth: true
                    text: "Native paths are interpreted by this operating system. Client paths are passed verbatim to qBittorrent, which supports containers and remote clients without Linux-specific path assumptions."
                    color: root.muted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }
                RowLayout {
                    Layout.fillWidth: true
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Native ROM library"
                        text: appSettings.rom_directory
                        readOnly: true
                    }
                    Button {
                        text: "Choose…"
                        onClicked: {
                            root.directoryTarget = "rom"
                            directoryDialog.open()
                        }
                    }
                }
                TextField {
                    Layout.fillWidth: true
                    placeholderText: "qBittorrent/container ROM path (for emulator workflows)"
                    text: appSettings.qbittorrent_container_rom_directory
                    onTextEdited: appSettings.qbittorrent_container_rom_directory = text
                }
                RowLayout {
                    Layout.fillWidth: true
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Native torrent library"
                        text: appSettings.torrent_library_directory
                        readOnly: true
                    }
                    Button {
                        text: "Choose…"
                        onClicked: {
                            root.directoryTarget = "torrent"
                            directoryDialog.open()
                        }
                    }
                }
                TextField {
                    Layout.fillWidth: true
                    placeholderText: "qBittorrent/container torrent library path"
                    text: appSettings.qbittorrent_container_torrent_library_directory
                    onTextEdited: appSettings.qbittorrent_container_torrent_library_directory = text
                }
                RowLayout {
                    Layout.fillWidth: true
                    Text { text: "Install completed files using"; color: root.ink; font.pixelSize: 12 }
                    ComboBox {
                        id: linkMode
                        Layout.fillWidth: true
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { label: "Symbolic link", value: "symlink" },
                            { label: "Hard link", value: "hardlink" },
                            { label: "Copy-on-write reflink", value: "reflink" },
                            { label: "Full copy", value: "copy" },
                            { label: "Leave in download library", value: "leave_in_place" }
                        ]
                        currentIndex: {
                            for (let i = 0; i < model.length; ++i) {
                                if (model[i].value === appSettings.file_link_mode) {
                                    return i
                                }
                            }
                            return 0
                        }
                        onActivated: appSettings.file_link_mode = currentValue
                    }
                    CheckBox {
                        text: "Entire torrent"
                        checked: appSettings.download_entire_torrent
                        onToggled: appSettings.download_entire_torrent = checked
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: settingsMessage.implicitHeight + 22
                    radius: 9
                    color: appSettings.connection_ok ? "#172c28" : "#151d29"
                    border.color: appSettings.connection_ok ? root.accentCool : root.line
                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 11
                        BusyIndicator {
                            visible: appSettings.busy
                            running: visible
                            Layout.preferredWidth: 18
                            Layout.preferredHeight: 18
                        }
                        Text {
                            id: settingsMessage
                            Layout.fillWidth: true
                            text: appSettings.message
                            color: appSettings.connection_ok ? root.accentCool : root.muted
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Item { Layout.fillWidth: true }
                    Button {
                        text: "Test connection"
                        enabled: !appSettings.busy
                        onClicked: appSettings.test_connection()
                    }
                    HeaderButton {
                        text: "Save settings"
                        enabled: !appSettings.busy
                        active: true
                        onClicked: appSettings.save()
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: appSettings.state_database_path
                    color: "#566175"
                    font.pixelSize: 9
                    elide: Text.ElideMiddle
                    horizontalAlignment: Text.AlignRight
                }
            }
        }
    }

    Drawer {
        id: downloadsDrawer
        edge: Qt.RightEdge
        width: Math.min(480, root.width * 0.42)
        height: root.height
        modal: false
        interactive: true
        padding: 0

        background: Rectangle {
            color: "#101620"
            border.color: root.line
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 66
                color: root.panel
                border.color: root.line
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 20
                    anchors.verticalCenter: parent.verticalCenter
                    text: "DOWNLOADS"
                    color: root.ink
                    font.pixelSize: 16
                    font.weight: Font.Bold
                    font.letterSpacing: 1
                }
                Row {
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 5
                    RoundButton {
                        text: "↻"
                        flat: true
                        enabled: !downloadQueue.busy
                        onClicked: downloadQueue.refresh()
                    }
                    RoundButton {
                        text: "×"
                        flat: true
                        font.pixelSize: 20
                        onClicked: downloadsDrawer.close()
                    }
                }
            }
            Text {
                Layout.fillWidth: true
                Layout.margins: 16
                text: downloadQueue.message
                color: root.muted
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                Column {
                    width: downloadsDrawer.width
                    spacing: 9
                    leftPadding: 14
                    rightPadding: 14
                    Repeater {
                        model: downloadQueue.job_count
                        delegate: Rectangle {
                            id: downloadRow
                            required property int index
                            property int queueRevision: downloadQueue.revision
                            width: downloadsDrawer.width - 28
                            height: 136
                            radius: 11
                            color: root.panelRaised
                            border.color: root.line
                            ColumnLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 5
                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        Layout.fillWidth: true
                                        text: { downloadRow.queueRevision; return downloadQueue.job_title_at(downloadRow.index) }
                                        color: root.ink
                                        font.pixelSize: 13
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        text: { downloadRow.queueRevision; return downloadQueue.job_state_at(downloadRow.index) }
                                        color: text === "IMPORTED" ? root.accentCool : root.accent
                                        font.pixelSize: 9
                                        font.weight: Font.Bold
                                    }
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: { downloadRow.queueRevision; return downloadQueue.job_platform_at(downloadRow.index) }
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                                ProgressBar {
                                    Layout.fillWidth: true
                                    value: { downloadRow.queueRevision; return downloadQueue.job_progress_at(downloadRow.index) }
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: { downloadRow.queueRevision; return downloadQueue.job_detail_at(downloadRow.index) }
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                                RowLayout {
                                    Layout.fillWidth: true
                                    Item { Layout.fillWidth: true }
                                    Button {
                                        text: "Pause"
                                        visible: { downloadRow.queueRevision; return downloadQueue.job_can_pause(downloadRow.index) }
                                        enabled: !downloadQueue.busy
                                        onClicked: downloadQueue.pause_job(downloadRow.index)
                                    }
                                    Button {
                                        text: "Resume"
                                        visible: { downloadRow.queueRevision; return downloadQueue.job_can_resume(downloadRow.index) }
                                        enabled: !downloadQueue.busy
                                        onClicked: downloadQueue.resume_job(downloadRow.index)
                                    }
                                    Button {
                                        text: "Cancel"
                                        visible: { downloadRow.queueRevision; return downloadQueue.job_can_cancel(downloadRow.index) }
                                        enabled: !downloadQueue.busy
                                        onClicked: downloadQueue.cancel_job(downloadRow.index)
                                    }
                                }
                            }
                        }
                    }
                    Item { width: 1; height: 14 }
                }
            }
        }
    }

    Rectangle {
        id: statusBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 34
        color: "#0a0f16"
        border.color: root.line
        Row {
            anchors.left: parent.left
            anchors.leftMargin: 15
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9
            Rectangle {
                width: 7
                height: 7
                radius: 4
                color: library.loading || library.filtering ? root.accent :
                       library.ready ? root.accentCool : "#ef6a6a"
            }
            Text {
                text: library.status_message
                color: root.muted
                font.pixelSize: 11
            }
        }
        Text {
            anchors.right: parent.right
            anchors.rightMargin: 15
            anchors.verticalCenter: parent.verticalCenter
            text: library.startup_ms > 0 ? "native shell " + library.startup_ms + " ms" : "native Qt"
            color: "#566175"
            font.pixelSize: 10
        }
    }
}
