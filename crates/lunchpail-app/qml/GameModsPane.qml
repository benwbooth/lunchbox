pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Column {
    id: pane
    property var backend: null
    property string gameId: ""
    property bool locked: false
    property bool retroarch: false
    property var pickPatchFile: function() { return "" }
    property var pickCheatFile: function() { return "" }
    property var pickCheatExport: function() { return "" }
    property bool expanded: false
    property int editingCheat: -1
    readonly property var profile: backend ? JSON.parse(backend.profile_json) : ({ patches: [], cheats: [], cheats_enabled: false, base_sha256: "" })
    spacing: 8
    onGameIdChanged: { expanded = false; editingCheat = -1; if (backend) backend.select_game(gameId) }
    Component.onCompleted: { if (backend) backend.select_game(gameId) }

    LbButton {
        objectName: "modsAccordionButton"
        width: parent.width
        flat: true
        text: (pane.expanded ? "▾  " : "▸  ") + "Patches & cheats"
        onClicked: pane.expanded = !pane.expanded
    }
    Column {
        width: parent.width
        spacing: 8
        visible: pane.expanded
        enabled: pane.backend && !pane.backend.busy && !pane.locked
        Text { text: "Translation & mod patches"; color: "#f4f7fb"; font.bold: true }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
            text: "Original files stay untouched. Enabled patches apply from top to bottom; later launches reuse the cached result. Patched saves are separate."
        }
        LbButton {
            width: parent.width; text: "Import patch…"
            onClicked: { const path = pane.pickPatchFile(); if (path) pane.backend.import_patch(path) }
        }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
            text: "IPS / IPS32 · BPS · UPS · PPF · xdelta (requires xdelta3). Extract downloaded archives first. Disc patches require the author's raw ISO/BIN, not CHD or a cue sheet."
        }
        Repeater {
            model: pane.profile.patches
            delegate: Column {
                id: patchRow
                required property var modelData
                required property int index
                width: parent.width; spacing: 4
                LbCheckBox {
                    width: parent.width
                    text: (patchRow.index + 1) + ". " + patchRow.modelData.name
                    checked: patchRow.modelData.enabled
                    onClicked: pane.backend.change_patch(patchRow.index, "toggle")
                }
                RowLayout {
                    width: parent.width
                    Text { text: patchRow.modelData.format; color: "#a7b4c4"; Layout.fillWidth: true }
                    LbButton { text: "Up"; enabled: patchRow.index > 0; onClicked: pane.backend.change_patch(patchRow.index, "up") }
                    LbButton { text: "Down"; enabled: patchRow.index + 1 < pane.profile.patches.length; onClicked: pane.backend.change_patch(patchRow.index, "down") }
                    LbButton { text: "Remove"; onClicked: pane.backend.change_patch(patchRow.index, "remove") }
                }
            }
        }
        Text { text: "Base SHA256 (optional)"; color: "#a7b4c4" }
        LbTextField {
            width: parent.width
            text: pane.profile.base_sha256
            placeholderText: "Optional base ROM/disc SHA256 from patch author"
            onEditingFinished: pane.backend.set_base_hash(text)
        }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
            text: "Match the release, region and header in the patch readme. BPS/UPS check both input and output; IPS has no built-in base checksum. Stacking unrelated patches is not always compatible."
        }
        Flow {
            width: parent.width; spacing: 6
            LbButton { text: "Find patches"; onClicked: Qt.openUrlExternally("https://romhackplaza.org/") }
            LbButton { text: "Romhacking archive"; onClicked: Qt.openUrlExternally("https://www.romhacking.net/") }
        }
        Text { text: "Cheat codes"; color: "#f4f7fb"; font.bold: true }
        LbCheckBox {
            text: "Enable cheats for this game"
            checked: pane.profile.cheats_enabled
            enabled: pane.retroarch || checked
            onClicked: pane.backend.enable_cheats(checked)
        }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
            text: pane.retroarch
                  ? "Codes apply on the next launch. Game Genie / Action Replay support depends on the selected core and exact game revision. Cheats can affect saves and disable achievement eligibility."
                  : "Automatic cheat loading currently supports RetroArch. You can manage/export codes here, or use the selected emulator's own cheat interface."
        }
        Flow {
            width: parent.width; spacing: 6
            LbButton { text: "Import .cht…"; onClicked: { const path = pane.pickCheatFile(); if (path) pane.backend.import_cheats(path) } }
            LbButton { text: "Export .cht…"; enabled: pane.profile.cheats.length > 0; onClicked: { const path = pane.pickCheatExport(); if (path) pane.backend.export_cheats(path) } }
            LbButton { text: "Find cheats"; onClicked: Qt.openUrlExternally("https://github.com/libretro/libretro-database/tree/master/cht") }
        }
        Repeater {
            model: pane.profile.cheats
            delegate: Column {
                id: cheatRow
                required property var modelData
                required property int index
                width: parent.width
                LbCheckBox { width: parent.width; text: cheatRow.modelData.name; checked: cheatRow.modelData.enabled; onClicked: pane.backend.change_cheat(cheatRow.index, "toggle") }
                RowLayout {
                    width: parent.width
                    Text { Layout.fillWidth: true; Layout.minimumWidth: 0; text: cheatRow.modelData.code || "RetroArch memory code"; wrapMode: Text.WrapAnywhere; color: "#a7b4c4" }
                    LbButton { text: "Edit"; onClicked: { pane.editingCheat = cheatRow.index; cheatName.text = cheatRow.modelData.name; cheatCode.text = cheatRow.modelData.code } }
                    LbButton { text: "Remove"; onClicked: { pane.backend.change_cheat(cheatRow.index, "remove"); pane.editingCheat = -1 } }
                }
            }
        }
        Text { text: "Cheat name"; color: "#a7b4c4" }
        LbTextField { id: cheatName; width: parent.width; placeholderText: "Cheat name" }
        Text { width: parent.width; wrapMode: Text.WordWrap; text: "Code (combine related codes with +)"; color: "#a7b4c4" }
        LbTextField { id: cheatCode; width: parent.width; placeholderText: "Code" }
        RowLayout {
            width: parent.width
            LbButton {
                Layout.fillWidth: true
                text: pane.editingCheat < 0 ? "Add code" : "Update code"
                enabled: cheatName.text.trim().length > 0 && (cheatCode.text.trim().length > 0 || pane.editingCheat >= 0)
                onClicked: pane.backend.save_cheat(pane.editingCheat, cheatName.text, cheatCode.text)
            }
            LbButton { text: "New code"; visible: pane.editingCheat >= 0; onClicked: { pane.editingCheat = -1; cheatName.text = ""; cheatCode.text = "" } }
        }
    }
    Text {
        width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
        visible: pane.expanded && text.length > 0
        text: pane.locked ? "Stop the game before changing patches or cheats." : pane.backend ? pane.backend.message : ""
    }
}
