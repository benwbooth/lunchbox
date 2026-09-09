import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "ControllerTargetFilter"
    Lunchbox.ControllerTargetFilter { id: filter }
    property var profiles: [
        {id: "ps", core: "duckstation", transport: "duckstation-settings", target_layout: "dualshock"},
        {id: "ps-digital", core: "duckstation", transport: "duckstation-settings", target_layout: "playstation-digital"},
        {id: "other-emulator", core: "mednafen", transport: "mednafen", target_layout: "dualshock"},
        {id: "dc", core: "flycast", transport: "retropad", target_layout: "dreamcast", retroarch_launch: {platforms: ["Sega Dreamcast"]}},
        {id: "arcade", core: "flycast", transport: "retropad", target_layout: "arcade-six-button", retroarch_launch: {platforms: ["Arcade"]}},
        {id: "wheel", core: "flycast", transport: "retropad", target_layout: "wheel", retroarch_launch: {platforms: ["Arcade"]}}
    ]
    function test_only_selected_emulator_and_system() {
        compare(filter.applicable(profiles, "DuckStation", "Sony Playstation").map(p => p.id), ["ps", "ps-digital"])
        compare(filter.applicable(profiles, "DuckStation", "Sega Genesis"), [])
    }
    function test_multisystem_core() {
        compare(filter.applicable(profiles, "RetroArch · Flycast (flycast)", "Sega Dreamcast").map(p => p.id), ["dc"])
        compare(filter.applicable(profiles, "RetroArch · Flycast (flycast)", "Arcade").map(p => p.id), ["arcade"])
    }
    function test_unknown_never_opens_catalog() {
        compare(filter.applicable(profiles, "", "Sony Playstation"), [])
        compare(filter.applicable(profiles, "unknown", "Sony Playstation"), [])
        compare(filter.applicable(profiles, "RetroArch", "Sony Playstation"), [])
    }
    function test_settings_can_choose_context() {
        verify(filter.emulators(profiles).indexOf("duckstation") >= 0)
        compare(filter.systems(profiles, "duckstation"), ["sony playstation"])
        compare(filter.applicable(profiles, "duckstation", filter.systems(profiles, "duckstation")[0]).length, 2)
        compare(filter.systems(profiles, "RetroArch · flycast (flycast)"), ["Arcade", "Sega Dreamcast"])
    }
    function test_player_limits() {
        compare(filter.playerLimit(profiles[0]), 2)
        compare(filter.playerLimit({target_layout: "psp"}), 1)
        compare(filter.playerLimit({retroarch_launch: {max_players: 4}}), 4)
        compare(filter.playerLimit(null), 0)
    }
    function test_ares_metadata_filters_system_and_players() {
        const profiles = [
            {id: "ares-n64", core: "ares", transport: "ares-settings", target_layout: "n64", native_launch: {platforms: ["Nintendo 64"], max_players: 4}},
            {id: "ares-nes", core: "ares", transport: "ares-settings", target_layout: "nes", native_launch: {platforms: ["Nintendo Entertainment System"], max_players: 2}}
        ]
        const targets = filter.applicable(profiles, "ares", "Nintendo 64")
        compare(targets.map(p => p.id), ["ares-n64"])
        compare(filter.playerLimit(targets[0]), 4)
        compare(filter.applicable(profiles, "RetroArch (ares)", "Nintendo 64"), [])
    }
}
