pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: pane
    required property var backend
    property string gameId: ""
    property string gameTitle: ""
    property string platform: ""
    property string romPath: ""
    property bool locked: false
    property bool expanded: false
    property bool showConnection: false
    property bool showReadme: false
    readonly property var results: JSON.parse(backend.results_json)
    readonly property var details: JSON.parse(backend.details_json)
    readonly property var downloaded: JSON.parse(backend.package_json)
    readonly property var sources: ["archive", "plaza", "github"]
    color: "#17212e"; border.color: "#344358"; radius: 12
    implicitHeight: contents.implicitHeight + 24
    height: visible ? implicitHeight : 0
    onGameIdChanged: {
        expanded = false
        query.text = gameTitle.split("(")[0].split("[")[0].trim()
        if (backend) backend.select_game(gameId, platform)
    }
    onPlatformChanged: if (backend) backend.select_game(gameId, platform)
    onGameTitleChanged: query.text = gameTitle.split("(")[0].split("[")[0].trim()
    Component.onCompleted: backend.select_game(gameId, platform)
    Column {
        id: contents
        x: 12; y: 12; width: parent.width - 24; spacing: 10
        LbButton {
            objectName: "communityPatchesAccordion"
            width: parent.width; flat: true
            text: (pane.expanded ? "▾  " : "▸  ") + "Translations & mods"
            onClicked: pane.expanded = !pane.expanded
        }
        Column {
            width: parent.width; spacing: 10; visible: pane.expanded
            Text {
                width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
                text: "Find a community patch, download it here, then apply it to a separate copy of your game. No original ROMs or executable patchers are downloaded."
            }
            LbComboBox {
                id: source
                objectName: "patchSource"
                width: parent.width
                model: ["RHDN community archive (2019, partial)", "Romhack Plaza (API key)", "GitHub author releases"]
                enabled: !pane.backend.busy
            }
            Text {
                width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
                text: source.currentIndex === 0
                      ? "An older, partial Romhacking.net collection preserved by zach-morris/romhack_db. It includes translations for NES, SNES and Game Boy, plus selected mods. Versions and language metadata may be incomplete; read the author notes."
                      : source.currentIndex === 1
                      ? "Current community translations and mods. Connect your Romhack Plaza API key below (entries:read and entries:download). Requests follow the site's rate limits."
                      : "Searches public translation/mod projects, then lists their latest stable release files. Results may target another system; verify the game and required revision."
            }
            RowLayout {
                width: parent.width
                LbTextField {
                    id: query
                    objectName: "patchQuery"
                    Layout.fillWidth: true; Layout.minimumWidth: 0
                    placeholderText: "Game title or alternate/Japanese title"
                    enabled: !pane.backend.busy
                    onAccepted: searchButton.clicked()
                }
                LbComboBox {
                    id: kind
                    Layout.preferredWidth: 132
                    model: ["Translations", "Mods"]
                    enabled: !pane.backend.busy
                }
            }
            LbButton {
                id: searchButton
                objectName: "searchCommunityPatches"
                width: parent.width; text: "Find patches"
                enabled: !pane.backend.busy && query.text.trim().length >= 2
                onClicked: if (enabled) pane.backend.search(pane.sources[source.currentIndex], query.text, kind.currentIndex === 0 ? "translations" : "romhacks")
            }
            LbButton {
                width: parent.width; flat: true
                visible: source.currentIndex === 1
                text: (pane.showConnection ? "▾  " : "▸  ") + "Connect Romhack Plaza"
                onClicked: pane.showConnection = !pane.showConnection
            }
            Column {
                width: parent.width; spacing: 6; visible: pane.showConnection && source.currentIndex === 1
                Text { width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"; text: "Create a key in your Romhack Plaza account → Settings → API Keys. Lunchbox keeps it in the OS credential store. Your existing key remains saved when this field is blank." }
                LbTextField { id: apiKey; width: parent.width; echoMode: TextInput.Password; placeholderText: "Romhack Plaza API key"; onVisibleChanged: if (!visible) clear() }
                Flow {
                    width: parent.width; spacing: 6
                    LbButton { text: "Connect"; enabled: !pane.backend.busy && apiKey.text.length > 0; onClicked: { pane.backend.save_plaza_key(apiKey.text); apiKey.clear() } }
                    LbButton { text: "Disconnect"; enabled: !pane.backend.busy; onClicked: { pane.backend.save_plaza_key(""); apiKey.clear() } }
                    LbButton { text: "Account / API instructions"; onClicked: Qt.openUrlExternally("https://community.romhackplaza.org/threads/api-usage.4782/") }
                }
            }
            MomentumListView {
                id: matches
                objectName: "communityPatchResults"
                width: parent.width; height: Math.min(280, contentHeight); clip: true; spacing: 6
                model: pane.results
                ScrollBar.vertical: LbScrollBar { visible: matches.contentHeight > matches.height }
                delegate: LbButton {
                    required property var modelData
                    required property int index
                    width: matches.verticalContentWidth; height: 48
                    text: modelData.title
                    enabled: !pane.backend.busy
                    onClicked: pane.backend.show_entry(index)
                }
            }
            Column {
                width: parent.width; spacing: 8; visible: !!pane.details
                Text { width: parent.width; wrapMode: Text.WordWrap; textFormat: Text.PlainText; color: "#f4f7fb"; font.bold: true; text: pane.details ? pane.details.title : "" }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap; textFormat: Text.PlainText; color: "#6bd4c7"
                    text: pane.details ? [pane.details.platform, pane.details.language, pane.details.version].filter(Boolean).join(" · ") : ""
                }
                MomentumScrollView {
                    id: descriptionScroll
                    width: parent.width; height: Math.min(180, patchDescription.implicitHeight + 16)
                    clip: true
                    contentWidth: availableWidth
                    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                    ScrollBar.vertical: LbScrollBar {
                        parent: descriptionScroll
                        x: descriptionScroll.width - width; y: 0
                        height: descriptionScroll.height
                        visible: descriptionScroll.contentHeight > descriptionScroll.availableHeight
                    }
                    Text {
                        id: patchDescription
                        width: descriptionScroll.availableWidth; wrapMode: Text.WordWrap; textFormat: Text.PlainText; color: "#a7b4c4"
                        text: pane.details ? pane.details.description : ""
                    }
                }
                Text {
                    width: parent.width; wrapMode: Text.WrapAnywhere; textFormat: Text.PlainText; color: "#a7b4c4"
                    text: pane.details && pane.details.bases.length > 0
                          ? "Required input (checked before applying):\n" + pane.details.bases.map(function(b) { return b.name + (b.crc32 ? "\nCRC32: " + b.crc32 : "") + (b.sha1 ? "\nSHA1: " + b.sha1 : "") + (b.md5 ? "\nMD5: " + b.md5 : "") }).join("\n")
                          : "No author-supplied input hash. Check the exact ROM revision and header in the notes. BPS/UPS also validate their embedded checksums; IPS cannot identify the correct base by itself."
                }
                LbButton {
                    text: "View source & credits"; visible: !!pane.details && pane.details.source_url.length > 0
                    onClicked: Qt.openUrlExternally(pane.details.source_url)
                }
                Repeater {
                    model: pane.details ? pane.details.files : []
                    delegate: LbButton {
                        required property var modelData
                        required property int index
                        width: parent.width
                        text: "Download " + modelData.name + (modelData.size > 0 ? " (" + (modelData.size / 1048576).toFixed(1) + " MiB)" : "")
                        enabled: !pane.backend.busy
                        onClicked: pane.backend.download(index)
                    }
                }
            }
            Column {
                width: parent.width; spacing: 8; visible: !!pane.downloaded
                Text { text: "Choose a patch variant"; font.bold: true; color: "#f4f7fb" }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
                    text: "Apply & enable checks the full active patch stack and builds the playable copy now. Only this variant is added; other enabled patches remain in their existing order. Manage them under Settings & mappings → Patches & cheats."
                }
                Repeater {
                    model: pane.downloaded ? pane.downloaded.variants : []
                    delegate: Column {
                        id: variant
                        required property var modelData
                        required property int index
                        width: parent.width; spacing: 4
                        Text { width: parent.width; wrapMode: Text.WrapAnywhere; textFormat: Text.PlainText; text: variant.modelData.name + " · " + variant.modelData.format; color: "#f4f7fb" }
                        RowLayout {
                            width: parent.width
                            LbButton {
                                objectName: "applyPatchVariant" + variant.index
                                Layout.fillWidth: true; text: "Apply & enable"
                                enabled: !pane.backend.busy && !pane.locked && pane.romPath.length > 0
                                onClicked: pane.backend.use_variant(variant.index, pane.romPath, true)
                            }
                            LbButton {
                                text: "Import disabled"; enabled: !pane.backend.busy && !pane.locked
                                onClicked: pane.backend.use_variant(variant.index, pane.romPath, false)
                            }
                        }
                    }
                }
                Text { width: parent.width; wrapMode: Text.WordWrap; color: "#ffb454"; visible: pane.romPath.length === 0; text: "Install or select the matching ROM to apply a patch." }
                LbButton { text: pane.showReadme ? "Hide package readme" : "Read package notes"; visible: !!pane.downloaded && pane.downloaded.readme.length > 0; onClicked: pane.showReadme = !pane.showReadme }
                MomentumScrollView {
                    id: readmeScroll
                    width: parent.width; height: 220; visible: pane.showReadme && !!pane.downloaded && pane.downloaded.readme.length > 0
                    clip: true
                    contentWidth: availableWidth
                    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                    ScrollBar.vertical: LbScrollBar {
                        parent: readmeScroll
                        x: readmeScroll.width - width; y: 0
                        height: readmeScroll.height
                        visible: readmeScroll.contentHeight > readmeScroll.availableHeight
                    }
                    Text { width: readmeScroll.availableWidth; wrapMode: Text.WrapAnywhere; textFormat: Text.PlainText; color: "#a7b4c4"; text: pane.downloaded ? pane.downloaded.readme : "" }
                }
            }
            Text { width: parent.width; wrapMode: Text.WordWrap; textFormat: Text.PlainText; color: "#6bd4c7"; text: pane.backend.message; visible: text.length > 0 }
            LbButton { text: "Cancel"; visible: pane.backend.busy; onClicked: pane.backend.cancel() }
        }
    }
}
