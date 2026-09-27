pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Column {
    id: tools
    required property var details
    required property var mods
    required property var patches
    required property var achievements
    required property var saveSync
    property string section: "display"
    property var pickPatchFile: function() { return "" }
    property var pickCheatFile: function() { return "" }
    property var pickCheatExport: function() { return "" }
    readonly property bool locked: details.launch_busy || details.game_running
    readonly property bool retroarch: (details.emulator_name || "").toLowerCase().includes("retroarch")
    signal achievementsSetupRequested()
    signal removeInstallationRequested()
    signal manageIdentityRequested()
    signal torrentRequested()
    signal reviewCandidateRequested(int index)
    spacing: 20

    Loader {
        width: parent.width
        active: tools.section === "display"
        sourceComponent: Column {
            spacing: 18
            enabled: !tools.locked
            Text { width: parent.width; text: tools.details.display_effective_summary; color: "#a7b4c4"; font.pixelSize: 18; wrapMode: Text.WordWrap }
            RowLayout {
                width: parent.width
                LbButton { Layout.fillWidth: true; implicitHeight: 50; text: "This game"; highlighted: tools.details.display_scope === "game"; onClicked: tools.details.select_display_scope("game") }
                LbButton { Layout.fillWidth: true; implicitHeight: 50; text: "Whole system"; highlighted: tools.details.display_scope === "platform"; onClicked: tools.details.select_display_scope("platform") }
            }
            Repeater {
                model: [
                    {key: "fullscreen", title: "Fullscreen", current: tools.details.display_fullscreen, supported: tools.details.display_fullscreen_supported, inherited: tools.details.display_inherited_fullscreen_label},
                    {key: "shader", title: "CRT shader", current: tools.details.display_shader, supported: tools.details.display_shader_supported, inherited: tools.details.display_inherited_shader_label},
                    {key: "bezel", title: "Bezel artwork", current: tools.details.display_bezel, supported: tools.details.display_bezel_supported, inherited: tools.details.display_inherited_bezel_label},
                    {key: "save_states", title: "Save states", current: tools.details.display_save_states, supported: tools.details.display_save_states_supported, inherited: tools.details.display_inherited_save_states_label}
                ]
                delegate: Column {
                    id: setting
                    required property var modelData
                    width: parent.width; spacing: 8; visible: modelData.supported
                    Text { text: setting.modelData.title; color: "#f4f7fb"; font.pixelSize: 20 }
                    LbComboBox {
                        id: choice
                        width: parent.width; implicitHeight: 52
                        textRole: "label"; valueRole: "value"
                        model: {
                            const rev = tools.details.display_revision
                            const items = [{ value: "", label: setting.modelData.inherited }]
                            const key = setting.modelData.key
                            if (key === "fullscreen") return items.concat([{value:"true", label:"Fullscreen"}, {value:"false", label:"Windowed"}])
                            if (key === "save_states") return items.concat([{value:"off", label:"Off"}, {value:"on", label:"Save + resume"}])
                            const shader = key === "shader"
                            if (!shader) items.push({value: "off", label: "Off"})
                            const count = shader ? tools.details.display_shader_preset_count() : tools.details.display_bezel_choice_count()
                            for (let i = 0; i < count; i++) items.push({
                                value: shader ? tools.details.display_shader_preset_id_at(i) : tools.details.display_bezel_choice_id_at(i),
                                label: shader ? tools.details.display_shader_preset_label_at(i) : tools.details.display_bezel_choice_label_at(i)
                            })
                            return items
                        }
                        function sync() { currentIndex = Math.max(0, indexOfValue(setting.modelData.current)) }
                        onModelChanged: sync()
                        Component.onCompleted: sync()
                        Connections { target: setting; function onModelDataChanged() { choice.sync() } }
                        onActivated: tools.details.set_display_setting(setting.modelData.key, currentValue)
                    }
                }
            }
            LbCheckBox {
                width: parent.width
                text: "Translate this game"; checked: tools.details.translation_opted_in
                onClicked: tools.details.save_translation_opt_in(checked)
            }
        }
    }
    Loader {
        width: parent.width
        active: tools.section === "mods"
        sourceComponent: Column {
            spacing: 20
            CommunityPatchesPane {
                width: parent.width; backend: tools.patches
                gameId: tools.details.game_id; gameTitle: tools.details.title; platform: tools.details.platform
                romPath: { tools.details.local_file_revision; return tools.details.local_file_path_at(tools.details.selected_local_file) }
                locked: tools.locked; Component.onCompleted: expanded = true
            }
            GameModsPane {
                width: parent.width; backend: tools.mods; gameId: tools.details.game_id
                retroarch: tools.retroarch; locked: tools.locked || tools.patches.applying
                pickPatchFile: tools.pickPatchFile; pickCheatFile: tools.pickCheatFile; pickCheatExport: tools.pickCheatExport
                Component.onCompleted: expanded = true
            }
        }
    }
    Loader {
        width: parent.width
        active: tools.section === "achievements"
        sourceComponent: RetroAchievementsPane {
            backend: tools.achievements; gameId: tools.details.game_id
            retroarch: tools.retroarch; locked: tools.locked
            Component.onCompleted: expanded = true
            onSetupRequested: tools.achievementsSetupRequested()
        }
    }
    Loader {
        width: parent.width
        active: tools.section === "files"
        sourceComponent: Column {
            spacing: 20
            GameFilesCard {
                width: parent.width; visible: tools.details.local_file_count > 0
                height: visible ? implicitHeight : 0
                detailsModel: tools.details
                ink: "#f4f7fb"; muted: "#95a2b6"; line: "#344358"; accent: "#ffb454"; accentCool: "#62dac8"
                onRemoveInstallationRequested: tools.removeInstallationRequested()
                onManageIdentityRequested: tools.manageIdentityRequested()
            }
            GameTorrentSources {
                width: parent.width; detailsModel: tools.details
                installed: tools.details.local; alternativesExpanded: true; showAddSource: true
                onAddSourceRequested: tools.torrentRequested()
                onReviewCandidateRequested: index => tools.reviewCandidateRequested(index)
            }
            GameSaveLocationsCard {
                width: parent.width
                ink: "#f4f7fb"; muted: "#95a2b6"; panel: "#182230"; line: "#344358"
                target: {
                    const changes = [tools.details.detail_revision, tools.details.local_file_revision, tools.details.selected_emulator_option, tools.saveSync.revision]
                    try { return JSON.parse(tools.details.save_sync_target_json()) } catch (_) { return {} }
                }
                backup: {
                    const revision = tools.saveSync.revision
                    try { return JSON.parse(tools.saveSync.backup_location_json(target.emulator_slug || "", target.runtime_platform || "")) } catch (_) { return {} }
                }
                onOpenFolderRequested: folder => Qt.openUrlExternally(folder)
            }
        }
    }
}
