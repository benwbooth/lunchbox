pragma ComponentBehavior: Bound
import QtQuick

Column {
    id: pane
    property var backend: null
    property string gameId: ""
    property bool retroarch: false
    property bool locked: false
    property bool expanded: false
    signal setupRequested()
    readonly property var modes: ["inherit", "emulator", "off", "casual", "hardcore"]
    function label(mode) {
        return mode === "off" ? "Off" : mode === "casual" ? "Casual" : mode === "hardcore" ? "Hardcore" : "RetroArch's own settings"
    }
    readonly property string effectiveMode: backend ? (backend.game_mode === "inherit" ? backend.default_mode : backend.game_mode) : "emulator"
    spacing: 8
    onGameIdChanged: { expanded = false; if (backend) backend.select_game(gameId) }
    Component.onCompleted: if (backend) backend.select_game(gameId)
    LbButton {
        objectName: "achievementAccordion"
        width: parent.width; flat: true
        text: (pane.expanded ? "▾  " : "▸  ") + "RetroAchievements"
        onClicked: pane.expanded = !pane.expanded
    }
    Column {
        width: parent.width; spacing: 8
        visible: pane.expanded
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
            text: pane.retroarch ? "For this game: " + pane.label(pane.effectiveMode) : "This emulator keeps its own achievement setup. Select a RetroArch core to use Lunchbox's account."
        }
        LbComboBox {
            objectName: "achievementGameMode"
            width: parent.width
            model: ["Use default — " + pane.label(pane.backend ? pane.backend.default_mode : "emulator"), "Keep RetroArch's own settings", "Off", "Casual — save states allowed", "Hardcore — no state loading or cheats"]
            currentIndex: pane.backend ? Math.max(0, pane.modes.indexOf(pane.backend.game_mode)) : 0
            enabled: !!pane.backend && !pane.backend.busy && !pane.locked
            onActivated: pane.backend.choose_game(pane.modes[currentIndex])
        }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
            text: pane.effectiveMode === "hardcore"
                  ? "Hardcore: no auto-resume, state loading, rewind or cheats. SRAM and creating save states remain available."
                  : "Progress and unlocks appear in RetroArch → Quick Menu → Achievements. Supported core and exact ROM revision required. Patches can change achievement compatibility."
        }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: "#ffb454"
            visible: !!pane.backend && !pane.backend.username && (pane.effectiveMode === "casual" || pane.effectiveMode === "hardcore")
            text: "Sign in under Settings → RetroAchievements before launching."
        }
        LbButton { text: "Account & achievement setup"; width: parent.width; onClicked: pane.setupRequested() }
        Text { width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"; text: pane.locked ? "Changes are locked while a game is running or launching." : (pane.backend ? pane.backend.message : ""); visible: text.length > 0 }
    }
}
