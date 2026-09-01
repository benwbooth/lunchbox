import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: dialog

    property var igdbModel
    property var screenScraperModel
    property var gameDetailsModel
    property string canonicalTitle: ""
    property int providerIndex: 0
    property var activeModel: providerIndex === 0 ? igdbModel : screenScraperModel
    property string providerName: providerIndex === 0 ? "IGDB" : "ScreenScraper"
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color panel: "#121923"
    property color panelRaised: "#182230"
    property color line: "#2b3648"
    property color accent: "#ffb454"
    property color accentCool: "#6edbd1"
    readonly property int selectedFieldCount: {
        let count = 0
        for (let index = 0; index < fieldModel.count; ++index) {
            if (fieldModel.get(index).checked)
                ++count
        }
        return count
    }
    readonly property int fieldCount: fieldModel.count

    signal settingsRequested(string section)

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(940, parent.width - 64)
    height: Math.min(720, parent.height - 64)
    modal: true
    padding: 0
    closePolicy: Popup.CloseOnEscape

    function currentValue(key) {
        if (!gameDetailsModel)
            return ""
        switch (key) {
        case "title": return gameDetailsModel.metadata_title
        case "description": return gameDetailsModel.metadata_description
        case "release_date": return gameDetailsModel.metadata_release_date
        case "developer": return gameDetailsModel.metadata_developer
        case "publisher": return gameDetailsModel.metadata_publisher
        case "genre": return gameDetailsModel.metadata_genre
        case "players": return gameDetailsModel.metadata_players
        case "rating": return gameDetailsModel.metadata_rating
        default: return ""
        }
    }

    function setCurrentValue(key, value) {
        switch (key) {
        case "title": gameDetailsModel.metadata_title = value; break
        case "description": gameDetailsModel.metadata_description = value; break
        case "release_date": gameDetailsModel.metadata_release_date = value; break
        case "developer": gameDetailsModel.metadata_developer = value; break
        case "publisher": gameDetailsModel.metadata_publisher = value; break
        case "genre": gameDetailsModel.metadata_genre = value; break
        case "players": gameDetailsModel.metadata_players = value; break
        case "rating": gameDetailsModel.metadata_rating = value; break
        }
    }

    function openReview() {
        if (!gameDetailsModel || gameDetailsModel.database_id <= 0)
            return
        igdbModel.initialize()
        screenScraperModel.initialize()
        providerIndex = igdbModel.credentials_saved || !screenScraperModel.credentials_saved ? 0 : 1
        searchField.text = canonicalTitle.length > 0
                           ? canonicalTitle : gameDetailsModel.title
        fieldModel.clear()
        open()
        beginProviderReview()
    }

    function beginProviderReview() {
        if (!activeModel || !gameDetailsModel)
            return
        fieldModel.clear()
        activeModel.begin_selection(gameDetailsModel.database_id,
                                    canonicalTitle.length > 0
                                    ? canonicalTitle : gameDetailsModel.title,
                                    gameDetailsModel.platform, "fanart")
    }

    function rebuildFields() {
        fieldModel.clear()
        if (!activeModel || activeModel.selected_game_name.length === 0)
            return
        const definitions = [
            { key: "title", label: "Title" },
            { key: "description", label: "Description" },
            { key: "release_date", label: "Release date" },
            { key: "developer", label: "Developer" },
            { key: "publisher", label: "Publisher" },
            { key: "genre", label: "Genre" },
            { key: "players", label: "Players" },
            { key: "rating", label: "Rating" }
        ]
        for (let index = 0; index < definitions.length; ++index) {
            const definition = definitions[index]
            const incoming = activeModel.selected_metadata_value(definition.key).trim()
            if (incoming.length === 0)
                continue
            const current = currentValue(definition.key).trim()
            const same = current === incoming
            fieldModel.append({
                key: definition.key,
                label: definition.label,
                current: current,
                incoming: incoming,
                same: same,
                missing: current.length === 0,
                checked: !same && current.length === 0
            })
        }
    }

    function applySelectedFields() {
        if (!gameDetailsModel)
            return
        for (let index = 0; index < fieldModel.count; ++index) {
            const row = fieldModel.get(index)
            if (row.checked)
                setCurrentValue(row.key, row.incoming)
        }
        close()
    }

    function selectChanges(includeConflicts) {
        for (let index = 0; index < fieldModel.count; ++index) {
            const row = fieldModel.get(index)
            fieldModel.setProperty(index, "checked",
                                   !row.same && (includeConflicts || row.missing))
        }
    }

    ListModel { id: fieldModel }

    Connections {
        target: dialog.igdbModel
        function onRevisionChanged() {
            if (dialog.providerIndex === 0)
                dialog.rebuildFields()
        }
    }
    Connections {
        target: dialog.screenScraperModel
        function onRevisionChanged() {
            if (dialog.providerIndex === 1)
                dialog.rebuildFields()
        }
    }

    background: Rectangle {
        radius: 16
        color: "#0e141d"
        border.color: "#354155"
    }

    header: Rectangle {
        implicitHeight: 94
        color: dialog.panelRaised
        radius: 16
        border.color: dialog.line
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 16
            color: parent.color
        }
        Column {
            anchors.left: parent.left
            anchors.right: closeButton.left
            anchors.leftMargin: 24
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4
            Text {
                text: "REVIEW ONLINE METADATA"
                color: dialog.accent
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
            Text {
                width: parent.width
                text: dialog.gameDetailsModel ? dialog.gameDetailsModel.metadata_title : ""
                color: dialog.ink
                font.pixelSize: 20
                font.weight: Font.Bold
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: "Choose the exact provider record, then review every field before it enters your local profile."
                color: dialog.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }
        }
        RoundButton {
            id: closeButton
            anchors.right: parent.right
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            width: 36
            height: 36
            text: "×"
            flat: true
            font.pixelSize: 20
            onClicked: dialog.close()
        }
    }

    contentItem: ColumnLayout {
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 20
            Layout.rightMargin: 20
            Layout.topMargin: 18
            spacing: 10
            ComboBox {
                id: providerBox
                Layout.preferredWidth: 170
                model: ["IGDB", "ScreenScraper"]
                currentIndex: dialog.providerIndex
                enabled: !dialog.activeModel || !dialog.activeModel.busy
                onActivated: {
                    dialog.providerIndex = currentIndex
                    dialog.beginProviderReview()
                }
                Accessible.name: "Metadata provider"
            }
            ClearableSearchField {
                id: searchField
                Layout.fillWidth: true
                placeholderText: "Search for the exact game record"
                searchIconVisible: true
                leftPadding: 40
                color: dialog.ink
                placeholderTextColor: "#657186"
                enabled: !dialog.activeModel || !dialog.activeModel.busy
                onAccepted: if (text.trim().length >= 2)
                                dialog.activeModel.search_games(text)
                onClearRequested: text = dialog.canonicalTitle
                background: Rectangle {
                    implicitHeight: 42
                    radius: 9
                    color: "#101721"
                    border.color: parent.activeFocus ? dialog.accent : dialog.line
                }
            }
            Button {
                text: dialog.activeModel && dialog.activeModel.busy ? "Searching…" : "Search"
                enabled: dialog.activeModel && !dialog.activeModel.busy
                         && searchField.text.trim().length >= 2
                onClicked: dialog.activeModel.search_games(searchField.text)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.leftMargin: 20
            Layout.rightMargin: 20
            implicitHeight: statusText.implicitHeight + 22
            radius: 9
            color: "#131c28"
            border.color: dialog.line
            RowLayout {
                anchors.fill: parent
                anchors.margins: 11
                spacing: 10
                BusyIndicator {
                    Layout.preferredWidth: 20
                    Layout.preferredHeight: 20
                    running: dialog.activeModel && dialog.activeModel.busy
                    visible: running
                }
                Text {
                    id: statusText
                    Layout.fillWidth: true
                    text: !dialog.activeModel ? "Provider unavailable"
                          : dialog.activeModel.selected_game_name.length > 0
                            ? "Linked by explicit review to " + dialog.activeModel.selected_game_name
                          : dialog.activeModel.message
                    color: dialog.muted
                    font.pixelSize: 10
                    wrapMode: Text.WordWrap
                }
                Button {
                    visible: dialog.activeModel && dialog.activeModel.initialized
                             && !dialog.activeModel.credentials_saved
                    text: "Set up account"
                    flat: true
                    onClicked: {
                        dialog.close()
                        dialog.settingsRequested(dialog.providerIndex === 0
                                                 ? "igdb" : "screenscraper")
                    }
                }
            }
        }

        ListView {
            id: candidateList
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 20
            Layout.rightMargin: 20
            clip: true
            visible: dialog.activeModel
                     && dialog.activeModel.selected_game_name.length === 0
            model: dialog.activeModel ? dialog.activeModel.game_count : 0
            spacing: 7
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            delegate: Button {
                required property int index
                width: candidateList.width
                height: 62
                flat: true
                enabled: !dialog.activeModel.busy
                onClicked: dialog.activeModel.choose_game(index)
                background: Rectangle {
                    radius: 9
                    color: parent.down ? "#2b384b"
                          : parent.hovered ? "#202b3a" : "#151e2a"
                    border.color: parent.hovered ? dialog.accentCool : dialog.line
                }
                contentItem: Column {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 3
                    Text {
                        width: parent.width
                        text: dialog.activeModel.game_name_at(index)
                        color: dialog.ink
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Text {
                        width: parent.width
                        text: dialog.activeModel.game_detail_at(index)
                        color: dialog.muted
                        font.pixelSize: 10
                        elide: Text.ElideRight
                    }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 20
            Layout.rightMargin: 20
            visible: dialog.activeModel
                     && dialog.activeModel.selected_game_name.length > 0
            spacing: 8
            RowLayout {
                Layout.fillWidth: true
                Text {
                    Layout.fillWidth: true
                    text: fieldModel.count === 0
                          ? "This provider record has no supported metadata fields."
                          : "Missing values are selected automatically. Existing values remain unchanged until you opt in."
                    color: dialog.muted
                    font.pixelSize: 10
                    wrapMode: Text.WordWrap
                }
                Button {
                    text: "Missing only"
                    enabled: fieldModel.count > 0
                    flat: true
                    onClicked: dialog.selectChanges(false)
                }
                Button {
                    text: "All changes"
                    enabled: fieldModel.count > 0
                    flat: true
                    onClicked: dialog.selectChanges(true)
                }
                Button {
                    text: "Change record"
                    flat: true
                    onClicked: dialog.activeModel.search_games(searchField.text)
                }
            }
            ListView {
                id: fieldList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: fieldModel
                spacing: 7
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                delegate: Rectangle {
                    required property int index
                    required property string key
                    required property string label
                    required property string current
                    required property string incoming
                    required property bool same
                    required property bool missing
                    required property bool checked
                    width: fieldList.width
                    height: comparisonRow.implicitHeight + 20
                    radius: 9
                    color: checked ? "#172a2b" : "#151e2a"
                    border.color: checked ? dialog.accentCool : dialog.line
                    RowLayout {
                        id: comparisonRow
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 12
                        CheckBox {
                            checked: parent.parent.checked
                            enabled: !parent.parent.same
                            onToggled: fieldModel.setProperty(index, "checked", checked)
                            Accessible.name: "Use provider " + label
                        }
                        ColumnLayout {
                            Layout.preferredWidth: 120
                            spacing: 3
                            Text {
                                text: label.toUpperCase()
                                color: dialog.ink
                                font.pixelSize: 9
                                font.weight: Font.Bold
                                font.letterSpacing: 0.7
                            }
                            Text {
                                text: same ? "UNCHANGED" : missing ? "MISSING" : "DIFFERENT"
                                color: same ? dialog.muted
                                      : missing ? dialog.accentCool : dialog.accent
                                font.pixelSize: 8
                                font.weight: Font.Bold
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3
                            Text {
                                text: "CURRENT"
                                color: dialog.muted
                                font.pixelSize: 8
                                font.weight: Font.Bold
                            }
                            Text {
                                Layout.fillWidth: true
                                text: current.length > 0 ? current : "Not set"
                                color: current.length > 0 ? dialog.ink : "#667387"
                                font.pixelSize: 10
                                maximumLineCount: key === "description" ? 3 : 2
                                elide: Text.ElideRight
                                wrapMode: Text.WordWrap
                            }
                        }
                        Text {
                            text: "→"
                            color: dialog.muted
                            font.pixelSize: 16
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3
                            Text {
                                text: dialog.providerName.toUpperCase()
                                color: dialog.muted
                                font.pixelSize: 8
                                font.weight: Font.Bold
                            }
                            Text {
                                Layout.fillWidth: true
                                text: incoming
                                color: dialog.ink
                                font.pixelSize: 10
                                maximumLineCount: key === "description" ? 3 : 2
                                elide: Text.ElideRight
                                wrapMode: Text.WordWrap
                            }
                        }
                    }
                }
            }
        }
    }

    footer: Rectangle {
        implicitHeight: 68
        color: dialog.panel
        radius: 16
        border.color: dialog.line
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 20
            anchors.rightMargin: 20
            spacing: 10
            Text {
                Layout.fillWidth: true
                text: dialog.activeModel && dialog.activeModel.selected_game_name.length > 0
                      ? dialog.selectedFieldCount + " field"
                        + (dialog.selectedFieldCount === 1 ? "" : "s")
                        + " selected · Save changes in the game editor to commit"
                      : "No metadata changes are made until a reviewed record and fields are selected."
                color: dialog.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }
            Button {
                text: "Cancel"
                onClicked: dialog.close()
            }
            Button {
                text: "Apply to editor"
                highlighted: true
                enabled: dialog.selectedFieldCount > 0
                         && dialog.activeModel && !dialog.activeModel.busy
                onClicked: dialog.applySelectedFields()
            }
        }
    }
}
