import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

LbDialog {
    id: picker
    required property var backend
    property var pickFile: function() { return "" }
    property string selectedId: ""
    property string contextGame: ""
    property string contextScope: "game"
    property bool importing: false
    readonly property var choices: { try { return JSON.parse(backend.bezel_catalog_json) } catch (_) { return [] } }
    readonly property var selected: choices.find(row => row.id === selectedId) || ({label: "", source: ""})
    readonly property bool fitted: selectedId.startsWith("custom:") || selectedId.startsWith("arcade-duimon-") || selectedId.startsWith("ultrawide")
    readonly property bool ready: !backend.bezel_busy && (selectedId === "" || selectedId === "off"
        || (backend.bezel_preview_id === selectedId && backend.bezel_preview_url.length > 0 && !backend.bezel_status))
    title: "Choose a bezel"
    modal: true
    width: Math.min(parent.width - 40, 1040)
    height: Math.min(parent.height - 40, 760)
    anchors.centerIn: parent
    function begin() {
        contextGame = backend.game_id
        contextScope = backend.display_scope
        selectedId = backend.display_bezel
        importing = false
        backend.load_bezel_choices()
        open()
    }
    function requestPreview() {
        if (visible && !backend.bezel_busy && selectedId !== backend.bezel_preview_id)
            backend.preview_bezel(selectedId)
    }
    onSelectedIdChanged: previewDelay.restart()
    onOpened: requestPreview()
    Timer { id: previewDelay; interval: 180; onTriggered: picker.requestPreview() }
    Connections {
        target: picker.backend
        function onBezel_busyChanged() {
            if (picker.backend.bezel_busy || !picker.visible) return
            if (picker.importing) {
                picker.importing = false
                if (picker.backend.bezel_status) return
                if (picker.backend.bezel_preview_url.length) picker.selectedId = picker.backend.bezel_preview_id
            }
            picker.requestPreview()
        }
        function onGame_idChanged() { if (picker.visible && picker.backend.game_id !== picker.contextGame) picker.close() }
    }
    contentItem: ColumnLayout {
        spacing: 12
        Text {
            Layout.fillWidth: true
            text: "Preview artwork before applying it. " + (picker.contextScope === "game" ? "Only this game will change." : "This platform’s default will change.")
            color: "#a7b4c4"; font.pixelSize: 14; wrapMode: Text.WordWrap
        }
        GridLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            columns: picker.width >= 760 ? 2 : 1
            columnSpacing: 16; rowSpacing: 12
            ListView {
                id: designs
                objectName: "bezelDesigns"
                Layout.preferredWidth: picker.width >= 760 ? 300 : -1
                Layout.fillWidth: picker.width < 760; Layout.fillHeight: picker.width >= 760
                Layout.preferredHeight: picker.width < 760 ? 145 : -1
                clip: true; spacing: 6
                model: picker.choices
                currentIndex: Math.max(0, picker.choices.findIndex(row => row.id === picker.selectedId))
                keyNavigationEnabled: true; activeFocusOnTab: true
                Keys.onReturnPressed: { if (currentIndex >= 0) picker.selectedId = picker.choices[currentIndex].id }
                Keys.onSpacePressed: { if (currentIndex >= 0) picker.selectedId = picker.choices[currentIndex].id }
                delegate: LbItemDelegate {
                    required property var modelData
                    width: designs.width - 12
                    implicitHeight: Math.max(48, name.implicitHeight + 20)
                    highlighted: picker.selectedId === modelData.id
                    onClicked: picker.selectedId = modelData.id
                    contentItem: Text { id: name; text: modelData.label; color: "#f4f7fb"; font.pixelSize: 14; wrapMode: Text.WordWrap; verticalAlignment: Text.AlignVCenter }
                }
                ScrollBar.vertical: LbScrollBar { policy: ScrollBar.AsNeeded }
            }
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; spacing: 10
                Rectangle {
                    Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 160
                    color: "#070b10"; radius: 8; border.color: "#344358"
                    Image {
                        anchors.fill: parent; anchors.margins: 6
                        source: picker.backend.bezel_preview_id === picker.selectedId ? picker.backend.bezel_preview_url : ""
                        asynchronous: true; fillMode: Image.PreserveAspectFit
                        sourceSize.width: 1000
                    }
                    Text {
                        anchors.centerIn: parent; width: parent.width - 32
                        visible: picker.backend.bezel_busy || !picker.backend.bezel_preview_url || picker.backend.bezel_preview_id !== picker.selectedId
                        text: picker.backend.bezel_busy ? "Loading preview…" : picker.selectedId === "off" ? "No artwork" : picker.selectedId === "" ? picker.backend.display_inherited_bezel_label : "Preview unavailable"
                        color: "#a7b4c4"; font.pixelSize: 16; wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter
                    }
                }
                Text { Layout.fillWidth: true; text: picker.selected.label; color: "#f4f7fb"; font.pixelSize: 16; wrapMode: Text.WordWrap }
                Text {
                    Layout.fillWidth: true
                    text: picker.fitted ? "Uses fullscreen to align the game with the transparent opening. Artwork keeps its proportions, with black bars where needed." : "Artwork keeps its proportions. Changes apply on the next game launch."
                    color: "#a7b4c4"; font.pixelSize: 13; wrapMode: Text.WordWrap
                }
                LbButton {
                    visible: picker.selected.source.startsWith("https://")
                    text: "Artist & license"
                    onClicked: Qt.openUrlExternally(picker.selected.source)
                }
            }
        }
        Text { Layout.fillWidth: true; visible: !!picker.backend.bezel_status; text: picker.backend.bezel_status; color: "#ffb454"; font.pixelSize: 13; wrapMode: Text.WordWrap }
        LbButton {
            visible: !!picker.backend.bezel_status && picker.selectedId !== "" && picker.selectedId !== "off"
            enabled: !picker.backend.bezel_busy; text: "Retry preview"
            onClicked: picker.backend.preview_bezel(picker.selectedId)
        }
        Text { Layout.fillWidth: true; text: "Import a PNG with an enclosed transparent screen opening through its center. Lunchpail keeps its own copy."; color: "#a7b4c4"; font.pixelSize: 12; wrapMode: Text.WordWrap }
    }
    footer: RowLayout {
        spacing: 10
        LbButton {
            Layout.leftMargin: 20; Layout.topMargin: 12; Layout.bottomMargin: 20
            text: "Import PNG…"; enabled: !picker.backend.bezel_busy
            onClicked: {
                const path = picker.pickFile()
                if (path) { picker.importing = true; picker.backend.import_bezel(path) }
            }
        }
        Item { Layout.fillWidth: true }
        LbButton { text: "Cancel"; onClicked: picker.reject() }
        LbButton {
            objectName: "applyBezel"
            Layout.rightMargin: 20
            text: "Use this bezel"
            enabled: picker.ready && picker.contextGame === picker.backend.game_id && picker.contextScope === picker.backend.display_scope
            onClicked: {
                if (picker.fitted) picker.backend.set_display_setting("fullscreen", "true")
                picker.backend.set_display_setting("bezel", picker.selectedId)
                if (picker.backend.display_bezel === picker.selectedId) picker.accept()
            }
        }
    }
}
