import QtQml

QtObject {
    function key(value) { return (value || "").trim().toLowerCase() }
    function canonicalCore(value) {
        const aliases = {beetle_cygne: "mednafen_wswan", beetle_lynx: "mednafen_lynx",
            beetle_ngp: "mednafen_ngp", beetle_pce_fast: "mednafen_pce_fast",
            beetle_supergrafx: "mednafen_supergrafx", beetle_psx: "mednafen_psx",
            beetle_psx_hw: "mednafen_psx_hw", beetle_vb: "mednafen_vb"}
        const core = key(value).replace(/_libretro$/, "")
        return aliases[core] || core
    }
    function applicable(profiles, emulator, platform) {
        const host = key(emulator)
        const system = key(platform)
        if (!host || !system) return []
        const retroarch = host.startsWith("retroarch")
        const coreMatch = host.match(/\(([^()]+)\)$/)
        const rawCore = retroarch && coreMatch ? key(coreMatch[1]).replace(/_libretro$/, "") : host
        const core = retroarch ? canonicalCore(rawCore) : rawCore
        return profiles.filter(function(profile) {
            if ((profile.transport === "retropad") !== retroarch) return false
            if ((retroarch ? canonicalCore(profile.core) : key(profile.core)) !== core
                    && !(retroarch && canonicalCore(profile.retroarch_library) === core)) return false
            if (!platformsFor(profile).some(value => key(value) === system)) return false
            if (["arcade", "sega naomi", "sega naomi 2", "sammy atomiswave"].indexOf(system) >= 0)
                return ["arcade-six-button", "arcade-eight-button"].indexOf(profile.target_layout) >= 0
            return true
        })
    }
    function platformsFor(profile) {
        if (profile.native_launch) return profile.native_launch.platforms || []
        if (profile.transport === "retropad") return (profile.retroarch_launch || {}).platforms || []
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
        return nativeSystems[profile.target_layout] || []
    }
    function emulatorFor(profile) {
        return profile.transport === "retropad" ? "RetroArch · " + (profile.retroarch_library || profile.core) + " (" + profile.core + ")" : profile.core
    }
    function emulators(profiles) {
        return Array.from(new Set(profiles.filter(p => platformsFor(p).length > 0).map(p => emulatorFor(p)))).sort()
    }
    function systems(profiles, emulator) {
        return Array.from(new Set(profiles.filter(p => emulatorFor(p) === emulator).reduce((all, p) => all.concat(platformsFor(p)), []))).sort()
    }
    function playerLimit(profile) {
        if (!profile) return 0
        if (profile.native_launch) return profile.native_launch.max_players || 1
        if (profile.retroarch_launch) return profile.retroarch_launch.max_players || 1
        if (profile.target_layout === "dolphin-native-gamecube") return 4
        if (["psp", "gameboy", "gba", "gamegear", "lynx", "ngp", "virtualboy", "wonderswan"].indexOf(profile.target_layout) >= 0) return 1
        return 2
    }
}
