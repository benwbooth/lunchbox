import QtQml

QtObject {
    function key(value) { return (value || "").trim().toLowerCase() }
    function applicable(profiles, emulator, platform) {
        const host = key(emulator)
        const system = key(platform)
        if (!host || !system) return []
        const retroarch = host.startsWith("retroarch")
        const coreMatch = host.match(/\(([^()]+)\)$/)
        const rawCore = retroarch && coreMatch ? key(coreMatch[1]).replace(/_libretro$/, "") : host
        const aliases = {mednafen_wswan: "beetle_cygne", mednafen_lynx: "beetle_lynx",
            mednafen_ngp: "beetle_ngp", mednafen_pce_fast: "beetle_pce_fast",
            mednafen_supergrafx: "beetle_supergrafx", mednafen_psx: "beetle_psx",
            mednafen_psx_hw: "beetle_psx_hw", mednafen_vb: "beetle_vb"}
        const core = retroarch ? (aliases[rawCore] || rawCore) : rawCore
        const nativeSystems = {
            "dualshock": ["sony playstation"], "playstation-digital": ["sony playstation"],
            "dolphin-native-gamecube": ["nintendo gamecube"], "psp": ["sony psp"],
            "genesis-3": ["sega genesis", "sega mega drive"], "genesis-6": ["sega genesis", "sega mega drive"],
            "saturn-digital": ["sega saturn"], "nes": ["nintendo entertainment system"],
            "snes": ["super nintendo entertainment system"], "gameboy": ["nintendo game boy", "nintendo game boy color"],
            "gba": ["nintendo game boy advance"], "gamegear": ["sega game gear"],
            "mednafen-master-system": ["sega master system"], "lynx": ["atari lynx"],
            "ngp": ["snk neo geo pocket", "snk neo geo pocket color"],
            "pce-2": ["nec turbografx-16", "nec turbografx-cd", "pc engine", "nec pc engine supergrafx"],
            "pce-6": ["nec turbografx-16", "nec turbografx-cd", "pc engine", "nec pc engine supergrafx"],
            "virtualboy": ["nintendo virtual boy"], "wonderswan": ["wonderswan", "wonderswan color"]
        }
        return profiles.filter(function(profile) {
            if ((profile.transport === "retropad") !== retroarch) return false
            if (key(profile.core) !== core && !(retroarch && key(profile.retroarch_library) === core)) return false
            const platforms = retroarch ? ((profile.retroarch_launch || {}).platforms || [])
                : (nativeSystems[profile.target_layout] || [])
            if (!platforms.some(value => key(value) === system)) return false
            if (["arcade", "sega naomi", "sega naomi 2", "sammy atomiswave"].indexOf(system) >= 0)
                return ["arcade-six-button", "arcade-eight-button"].indexOf(profile.target_layout) >= 0
            return true
        })
    }
}
