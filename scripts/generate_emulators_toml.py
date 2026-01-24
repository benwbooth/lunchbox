#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "tomlkit",
# ]
# ///
"""
Generate emulators.toml from scraped emulator data and platform mapping.

This script combines the wiki scrape data with known emulator download sources
to produce a comprehensive emulator configuration file.
"""

import json
from pathlib import Path
import tomlkit
from tomlkit import document, table, array, comment, nl

# Known emulator information with download sources
# This is manually curated based on official sources
KNOWN_EMULATORS = {
    "RetroArch": {
        "name": "RetroArch",
        "description": "Multi-system emulator frontend with libretro cores",
        "homepage": "https://www.retroarch.com",
        "multi_system": True,
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "org.libretro.RetroArch",
                "appimage_url": "https://buildbot.libretro.com/stable/{version}/linux/x86_64/RetroArch.AppImage",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://buildbot.libretro.com/stable/{version}/windows/x86_64/RetroArch.7z",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://buildbot.libretro.com/stable/{version}/apple/osx/universal/RetroArch.dmg",
            },
        },
        "cores": {
            "nes": ["mesen", "fceumm", "nestopia"],
            "snes": ["bsnes", "snes9x", "mesen-s"],
            "gb": ["gambatte", "mgba", "sameboy"],
            "gbc": ["gambatte", "mgba", "sameboy"],
            "gba": ["mgba", "vba_next"],
            "genesis": ["genesis_plus_gx", "picodrive", "blastem"],
            "sms": ["genesis_plus_gx", "picodrive"],
            "gg": ["genesis_plus_gx", "smsplus"],
            "n64": ["mupen64plus_next", "parallel_n64"],
            "psx": ["beetle_psx_hw", "pcsx_rearmed", "swanstation"],
            "pce": ["beetle_pce", "beetle_pce_fast"],
            "pce_cd": ["beetle_pce", "beetle_pce_fast"],
            "saturn": ["beetle_saturn", "kronos"],
            "32x": ["picodrive"],
            "sega_cd": ["genesis_plus_gx", "picodrive"],
            "atari2600": ["stella"],
            "atari7800": ["prosystem"],
            "lynx": ["handy", "beetle_lynx"],
            "jaguar": ["virtualjaguar"],
            "ws": ["beetle_wswan"],
            "wsc": ["beetle_wswan"],
            "ngp": ["beetle_ngp"],
            "ngpc": ["beetle_ngp"],
            "vb": ["beetle_vb"],
            "3do": ["opera"],
            "vectrex": ["vecx"],
            "coleco": ["bluemsx", "gearcoleco"],
            "intellivision": ["freeintv"],
            "msx": ["bluemsx", "fmsx"],
            "pcfx": ["beetle_pcfx"],
            "pokemini": ["pokemini"],
            "c64": ["vice_x64"],
            "amiga": ["puae"],
        },
    },
    "Dolphin": {
        "name": "Dolphin",
        "description": "GameCube and Wii emulator",
        "homepage": "https://dolphin-emu.org",
        "systems": ["Nintendo GameCube", "Nintendo Wii"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "org.DolphinEmu.dolphin-emu",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://dolphin-emu.org/download/",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://dolphin-emu.org/download/",
            },
        },
    },
    "PCSX2": {
        "name": "PCSX2",
        "description": "PlayStation 2 emulator",
        "homepage": "https://pcsx2.net",
        "systems": ["Sony Playstation 2"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "net.pcsx2.PCSX2",
                "appimage_url": "https://pcsx2.net/downloads/",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://pcsx2.net/downloads/",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://pcsx2.net/downloads/",
            },
        },
    },
    "RPCS3": {
        "name": "RPCS3",
        "description": "PlayStation 3 emulator",
        "homepage": "https://rpcs3.net",
        "systems": ["Sony Playstation 3"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "net.rpcs3.RPCS3",
                "appimage_url": "https://rpcs3.net/download",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://rpcs3.net/download",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://rpcs3.net/download",
            },
        },
    },
    "PPSSPP": {
        "name": "PPSSPP",
        "description": "PlayStation Portable emulator",
        "homepage": "https://www.ppsspp.org",
        "systems": ["Sony PSP"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "org.ppsspp.PPSSPP",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://www.ppsspp.org/download",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://www.ppsspp.org/download",
            },
        },
    },
    "Vita3K": {
        "name": "Vita3K",
        "description": "PlayStation Vita emulator",
        "homepage": "https://vita3k.org",
        "systems": ["Sony Playstation Vita"],
        "platforms": {
            "linux": {
                "install_type": "appimage",
                "appimage_url": "https://vita3k.org/",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://vita3k.org/",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://vita3k.org/",
            },
        },
    },
    "DuckStation": {
        "name": "DuckStation",
        "description": "PlayStation 1 emulator with focus on playability and accuracy",
        "homepage": "https://www.duckstation.org",
        "systems": ["Sony Playstation"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "org.duckstation.DuckStation",
                "appimage_url": "https://www.duckstation.org/",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://www.duckstation.org/",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://www.duckstation.org/",
            },
        },
    },
    "melonDS": {
        "name": "melonDS",
        "description": "Nintendo DS and DSi emulator",
        "homepage": "https://melonds.kuribo64.net",
        "systems": ["Nintendo DS"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "net.kuribo64.melonDS",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://melonds.kuribo64.net/downloads.php",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://melonds.kuribo64.net/downloads.php",
            },
        },
    },
    "Citra": {
        "name": "Citra",
        "description": "Nintendo 3DS emulator",
        "homepage": "https://citra-emu.org",
        "systems": ["Nintendo 3DS"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "org.citra_emu.citra",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://citra-emu.org/download/",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://citra-emu.org/download/",
            },
        },
    },
    "Lime3DS": {
        "name": "Lime3DS",
        "description": "Nintendo 3DS emulator (Citra fork)",
        "homepage": "https://lime3ds.github.io",
        "systems": ["Nintendo 3DS"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "io.github.lime3ds.Lime3DS",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://github.com/Lime3DS/Lime3DS/releases",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://github.com/Lime3DS/Lime3DS/releases",
            },
        },
    },
    "Ryujinx": {
        "name": "Ryujinx",
        "description": "Nintendo Switch emulator",
        "homepage": "https://ryujinx.org",
        "systems": ["Nintendo Switch"],
        "platforms": {
            "linux": {
                "install_type": "appimage",
                "appimage_url": "https://ryujinx.org/download",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://ryujinx.org/download",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://ryujinx.org/download",
            },
        },
    },
    "Yuzu": {
        "name": "Yuzu",
        "description": "Nintendo Switch emulator",
        "homepage": "https://yuzu-emu.org",
        "systems": ["Nintendo Switch"],
        "discontinued": True,
        "platforms": {},
    },
    "mGBA": {
        "name": "mGBA",
        "description": "Game Boy Advance emulator with Game Boy/Color support",
        "homepage": "https://mgba.io",
        "systems": ["Nintendo Game Boy", "Nintendo Game Boy Color", "Nintendo Game Boy Advance"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "io.mgba.mGBA",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://mgba.io/downloads.html",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://mgba.io/downloads.html",
            },
        },
    },
    "Flycast": {
        "name": "Flycast",
        "description": "Sega Dreamcast, Naomi, and Atomiswave emulator",
        "homepage": "https://github.com/flyinghead/flycast",
        "systems": ["Sega Dreamcast", "Sega Naomi", "Sammy Atomiswave"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "org.flycast.Flycast",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://github.com/flyinghead/flycast/releases",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://github.com/flyinghead/flycast/releases",
            },
        },
    },
    "Redream": {
        "name": "Redream",
        "description": "Sega Dreamcast emulator",
        "homepage": "https://redream.io",
        "systems": ["Sega Dreamcast"],
        "platforms": {
            "linux": {
                "install_type": "portable",
                "download_url": "https://redream.io/download",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://redream.io/download",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://redream.io/download",
            },
        },
    },
    "Mednafen": {
        "name": "Mednafen",
        "description": "Multi-system emulator (basis for many libretro cores)",
        "homepage": "https://mednafen.github.io",
        "multi_system": True,
        "platforms": {
            "linux": {
                "install_type": "package",
                "package_name": "mednafen",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://mednafen.github.io/releases/",
            },
            "macos": {
                "install_type": "homebrew",
                "homebrew_formula": "mednafen",
            },
        },
    },
    "Ares": {
        "name": "Ares",
        "description": "Multi-system emulator (spiritual successor to higan)",
        "homepage": "https://ares-emu.net",
        "multi_system": True,
        "systems": [
            "Nintendo Entertainment System",
            "Super Nintendo Entertainment System",
            "Nintendo 64",
            "Nintendo Game Boy",
            "Nintendo Game Boy Color",
            "Nintendo Game Boy Advance",
            "Sega Master System",
            "Sega Game Gear",
            "Sega Genesis",
            "Sega CD",
            "Sega 32X",
            "NEC TurboGrafx-16",
            "NEC TurboGrafx-CD",
            "SNK Neo Geo AES",
            "SNK Neo Geo Pocket",
            "SNK Neo Geo Pocket Color",
            "WonderSwan",
            "WonderSwan Color",
            "ColecoVision",
            "Microsoft MSX",
            "Microsoft MSX2",
        ],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "dev.ares.ares",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://ares-emu.net/download",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://ares-emu.net/download",
            },
        },
    },
    "Mesen": {
        "name": "Mesen",
        "description": "High-accuracy NES/Famicom, SNES, Game Boy, and PC Engine emulator",
        "homepage": "https://www.mesen.ca",
        "systems": [
            "Nintendo Entertainment System",
            "Super Nintendo Entertainment System",
            "Nintendo Game Boy",
            "Nintendo Game Boy Color",
            "NEC TurboGrafx-16",
        ],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "ca.mesen.Mesen",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://www.mesen.ca/#Downloads",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://www.mesen.ca/#Downloads",
            },
        },
    },
    "bsnes": {
        "name": "bsnes",
        "description": "High-accuracy SNES emulator",
        "homepage": "https://bsnes.org",
        "systems": ["Super Nintendo Entertainment System"],
        "platforms": {
            "linux": {
                "install_type": "package",
                "package_name": "bsnes",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://github.com/bsnes-emu/bsnes/releases",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://github.com/bsnes-emu/bsnes/releases",
            },
        },
    },
    "Snes9x": {
        "name": "Snes9x",
        "description": "Portable SNES emulator",
        "homepage": "https://www.snes9x.com",
        "systems": ["Super Nintendo Entertainment System"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "com.snes9x.Snes9x",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://www.snes9x.com/downloads.php",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://www.snes9x.com/downloads.php",
            },
        },
    },
    "Simple64": {
        "name": "simple64",
        "description": "Nintendo 64 emulator based on mupen64plus",
        "homepage": "https://simple64.github.io",
        "systems": ["Nintendo 64"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "io.github.simple64.simple64",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://github.com/simple64/simple64/releases",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://github.com/simple64/simple64/releases",
            },
        },
    },
    "Project64": {
        "name": "Project64",
        "description": "Nintendo 64 emulator for Windows",
        "homepage": "https://www.pj64-emu.com",
        "systems": ["Nintendo 64"],
        "platforms": {
            "windows": {
                "install_type": "installer",
                "download_url": "https://www.pj64-emu.com/download",
            },
        },
    },
    "BlastEm": {
        "name": "BlastEm",
        "description": "Highly accurate Sega Genesis/Mega Drive emulator",
        "homepage": "https://www.retrodev.com/blastem/",
        "systems": ["Sega Genesis"],
        "platforms": {
            "linux": {
                "install_type": "portable",
                "download_url": "https://www.retrodev.com/blastem/",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://www.retrodev.com/blastem/",
            },
            "macos": {
                "install_type": "app",
                "download_url": "https://www.retrodev.com/blastem/",
            },
        },
    },
    "Xemu": {
        "name": "xemu",
        "description": "Original Xbox emulator",
        "homepage": "https://xemu.app",
        "systems": ["Microsoft Xbox"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "app.xemu.xemu",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://xemu.app/#download",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://xemu.app/#download",
            },
        },
    },
    "Xenia": {
        "name": "Xenia",
        "description": "Xbox 360 emulator",
        "homepage": "https://xenia.jp",
        "systems": ["Microsoft Xbox 360"],
        "platforms": {
            "windows": {
                "install_type": "portable",
                "download_url": "https://xenia.jp/download/",
            },
        },
    },
    "MAME": {
        "name": "MAME",
        "description": "Multiple Arcade Machine Emulator",
        "homepage": "https://www.mamedev.org",
        "systems": ["Arcade"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "org.mamedev.MAME",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://www.mamedev.org/release.html",
            },
            "macos": {
                "install_type": "homebrew",
                "homebrew_formula": "mame",
            },
        },
    },
    "Stella": {
        "name": "Stella",
        "description": "Atari 2600 emulator",
        "homepage": "https://stella-emu.github.io",
        "systems": ["Atari 2600"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "io.github.stella_emu.Stella",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://stella-emu.github.io/downloads.html",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://stella-emu.github.io/downloads.html",
            },
        },
    },
    "BigPEmu": {
        "name": "BigPEmu",
        "description": "Atari Jaguar emulator",
        "homepage": "https://www.richwhitehouse.com/jaguar/",
        "systems": ["Atari Jaguar"],
        "platforms": {
            "windows": {
                "install_type": "portable",
                "download_url": "https://www.richwhitehouse.com/jaguar/",
            },
        },
    },
    "DOSBox-X": {
        "name": "DOSBox-X",
        "description": "DOS and Windows 3.x/9x emulator",
        "homepage": "https://dosbox-x.com",
        "systems": ["MS-DOS", "Windows 3.X"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "com.dosbox_x.DOSBox-X",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://dosbox-x.com/",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://dosbox-x.com/",
            },
        },
    },
    "DOSBox Staging": {
        "name": "DOSBox Staging",
        "description": "Modern DOS emulator",
        "homepage": "https://dosbox-staging.github.io",
        "systems": ["MS-DOS"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "io.github.dosbox-staging",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://dosbox-staging.github.io/downloads/",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://dosbox-staging.github.io/downloads/",
            },
        },
    },
    "ScummVM": {
        "name": "ScummVM",
        "description": "Adventure game engine interpreter",
        "homepage": "https://www.scummvm.org",
        "systems": ["ScummVM"],
        "platforms": {
            "linux": {
                "install_type": "flatpak",
                "flatpak_id": "org.scummvm.ScummVM",
            },
            "windows": {
                "install_type": "portable",
                "download_url": "https://www.scummvm.org/downloads/",
            },
            "macos": {
                "install_type": "dmg",
                "download_url": "https://www.scummvm.org/downloads/",
            },
        },
    },
}

# System to preferred emulator mapping
SYSTEM_EMULATOR_MAPPING = {
    # Nintendo consoles
    "Nintendo Entertainment System": {
        "retroarch_cores": ["mesen", "fceumm", "nestopia"],
        "standalone": ["Mesen", "Ares"],
        "preferred": "retroarch:mesen",
    },
    "Super Nintendo Entertainment System": {
        "retroarch_cores": ["bsnes", "snes9x", "mesen-s"],
        "standalone": ["bsnes", "Snes9x", "Mesen", "Ares"],
        "preferred": "retroarch:bsnes",
    },
    "Nintendo 64": {
        "retroarch_cores": ["mupen64plus_next", "parallel_n64"],
        "standalone": ["Simple64", "Project64", "Ares"],
        "preferred": "Simple64",
    },
    "Nintendo Game Boy": {
        "retroarch_cores": ["gambatte", "mgba", "sameboy"],
        "standalone": ["mGBA", "Mesen", "Ares"],
        "preferred": "retroarch:gambatte",
    },
    "Nintendo Game Boy Color": {
        "retroarch_cores": ["gambatte", "mgba", "sameboy"],
        "standalone": ["mGBA", "Mesen", "Ares"],
        "preferred": "retroarch:gambatte",
    },
    "Nintendo Game Boy Advance": {
        "retroarch_cores": ["mgba", "vba_next"],
        "standalone": ["mGBA", "Ares"],
        "preferred": "mGBA",
    },
    "Nintendo DS": {
        "retroarch_cores": ["melonds", "desmume"],
        "standalone": ["melonDS"],
        "preferred": "melonDS",
    },
    "Nintendo 3DS": {
        "retroarch_cores": [],
        "standalone": ["Lime3DS", "Citra"],
        "preferred": "Lime3DS",
    },
    "Nintendo GameCube": {
        "retroarch_cores": [],
        "standalone": ["Dolphin"],
        "preferred": "Dolphin",
    },
    "Nintendo Wii": {
        "retroarch_cores": [],
        "standalone": ["Dolphin"],
        "preferred": "Dolphin",
    },
    "Nintendo Wii U": {
        "retroarch_cores": [],
        "standalone": [],
        "preferred": None,
        "notes": "Cemu available but not yet integrated",
    },
    "Nintendo Switch": {
        "retroarch_cores": [],
        "standalone": ["Ryujinx"],
        "preferred": "Ryujinx",
    },
    "Nintendo Virtual Boy": {
        "retroarch_cores": ["beetle_vb"],
        "standalone": [],
        "preferred": "retroarch:beetle_vb",
    },
    "Nintendo Pokemon Mini": {
        "retroarch_cores": ["pokemini"],
        "standalone": [],
        "preferred": "retroarch:pokemini",
    },

    # Sega consoles
    "Sega Master System": {
        "retroarch_cores": ["genesis_plus_gx", "picodrive"],
        "standalone": ["Ares"],
        "preferred": "retroarch:genesis_plus_gx",
    },
    "Sega Game Gear": {
        "retroarch_cores": ["genesis_plus_gx", "smsplus"],
        "standalone": ["Ares"],
        "preferred": "retroarch:genesis_plus_gx",
    },
    "Sega Genesis": {
        "retroarch_cores": ["genesis_plus_gx", "picodrive", "blastem"],
        "standalone": ["BlastEm", "Ares"],
        "preferred": "BlastEm",
    },
    "Sega CD": {
        "retroarch_cores": ["genesis_plus_gx", "picodrive"],
        "standalone": ["Ares"],
        "preferred": "retroarch:genesis_plus_gx",
    },
    "Sega 32X": {
        "retroarch_cores": ["picodrive"],
        "standalone": ["Ares"],
        "preferred": "retroarch:picodrive",
    },
    "Sega Saturn": {
        "retroarch_cores": ["beetle_saturn", "kronos"],
        "standalone": ["Mednafen"],
        "preferred": "retroarch:beetle_saturn",
    },
    "Sega Dreamcast": {
        "retroarch_cores": ["flycast"],
        "standalone": ["Flycast", "Redream"],
        "preferred": "Flycast",
    },
    "Sega SG-1000": {
        "retroarch_cores": ["genesis_plus_gx"],
        "standalone": [],
        "preferred": "retroarch:genesis_plus_gx",
    },

    # Sony consoles
    "Sony Playstation": {
        "retroarch_cores": ["beetle_psx_hw", "pcsx_rearmed", "swanstation"],
        "standalone": ["DuckStation"],
        "preferred": "DuckStation",
    },
    "Sony Playstation 2": {
        "retroarch_cores": [],
        "standalone": ["PCSX2"],
        "preferred": "PCSX2",
    },
    "Sony Playstation 3": {
        "retroarch_cores": [],
        "standalone": ["RPCS3"],
        "preferred": "RPCS3",
    },
    "Sony Playstation 4": {
        "retroarch_cores": [],
        "standalone": [],
        "preferred": None,
        "notes": "No mature emulator available yet",
    },
    "Sony PSP": {
        "retroarch_cores": ["ppsspp"],
        "standalone": ["PPSSPP"],
        "preferred": "PPSSPP",
    },
    "Sony Playstation Vita": {
        "retroarch_cores": [],
        "standalone": ["Vita3K"],
        "preferred": "Vita3K",
    },

    # NEC consoles
    "NEC TurboGrafx-16": {
        "retroarch_cores": ["beetle_pce", "beetle_pce_fast"],
        "standalone": ["Mesen", "Mednafen"],
        "preferred": "retroarch:beetle_pce",
    },
    "NEC TurboGrafx-CD": {
        "retroarch_cores": ["beetle_pce", "beetle_pce_fast"],
        "standalone": ["Mednafen"],
        "preferred": "retroarch:beetle_pce",
    },
    "NEC PC-FX": {
        "retroarch_cores": ["beetle_pcfx"],
        "standalone": [],
        "preferred": "retroarch:beetle_pcfx",
    },

    # SNK consoles
    "SNK Neo Geo AES": {
        "retroarch_cores": ["fbneo"],
        "standalone": ["Ares"],
        "preferred": "retroarch:fbneo",
    },
    "SNK Neo Geo MVS": {
        "retroarch_cores": ["fbneo"],
        "standalone": [],
        "preferred": "retroarch:fbneo",
    },
    "SNK Neo Geo CD": {
        "retroarch_cores": ["neocd"],
        "standalone": [],
        "preferred": "retroarch:neocd",
    },
    "SNK Neo Geo Pocket": {
        "retroarch_cores": ["beetle_ngp"],
        "standalone": ["Ares"],
        "preferred": "retroarch:beetle_ngp",
    },
    "SNK Neo Geo Pocket Color": {
        "retroarch_cores": ["beetle_ngp"],
        "standalone": ["Ares"],
        "preferred": "retroarch:beetle_ngp",
    },

    # Atari consoles
    "Atari 2600": {
        "retroarch_cores": ["stella"],
        "standalone": ["Stella"],
        "preferred": "Stella",
    },
    "Atari 5200": {
        "retroarch_cores": ["atari800"],
        "standalone": [],
        "preferred": "retroarch:atari800",
    },
    "Atari 7800": {
        "retroarch_cores": ["prosystem"],
        "standalone": [],
        "preferred": "retroarch:prosystem",
    },
    "Atari Jaguar": {
        "retroarch_cores": ["virtualjaguar"],
        "standalone": ["BigPEmu"],
        "preferred": "BigPEmu",
    },
    "Atari Lynx": {
        "retroarch_cores": ["handy", "beetle_lynx"],
        "standalone": [],
        "preferred": "retroarch:handy",
    },

    # Microsoft consoles
    "Microsoft Xbox": {
        "retroarch_cores": [],
        "standalone": ["Xemu"],
        "preferred": "Xemu",
    },
    "Microsoft Xbox 360": {
        "retroarch_cores": [],
        "standalone": ["Xenia"],
        "preferred": "Xenia",
    },

    # Bandai/WonderSwan
    "WonderSwan": {
        "retroarch_cores": ["beetle_wswan"],
        "standalone": ["Ares"],
        "preferred": "retroarch:beetle_wswan",
    },
    "WonderSwan Color": {
        "retroarch_cores": ["beetle_wswan"],
        "standalone": ["Ares"],
        "preferred": "retroarch:beetle_wswan",
    },

    # Other consoles
    "3DO Interactive Multiplayer": {
        "retroarch_cores": ["opera"],
        "standalone": [],
        "preferred": "retroarch:opera",
    },
    "Philips CD-i": {
        "retroarch_cores": ["same_cdi"],
        "standalone": [],
        "preferred": "retroarch:same_cdi",
    },
    "ColecoVision": {
        "retroarch_cores": ["bluemsx", "gearcoleco"],
        "standalone": ["Ares"],
        "preferred": "retroarch:gearcoleco",
    },
    "Mattel Intellivision": {
        "retroarch_cores": ["freeintv"],
        "standalone": [],
        "preferred": "retroarch:freeintv",
    },
    "GCE Vectrex": {
        "retroarch_cores": ["vecx"],
        "standalone": [],
        "preferred": "retroarch:vecx",
    },
    "Fairchild Channel F": {
        "retroarch_cores": ["freechaf"],
        "standalone": [],
        "preferred": "retroarch:freechaf",
    },
    "Magnavox Odyssey 2": {
        "retroarch_cores": ["o2em"],
        "standalone": [],
        "preferred": "retroarch:o2em",
    },

    # Computers
    "MS-DOS": {
        "retroarch_cores": ["dosbox_pure"],
        "standalone": ["DOSBox-X", "DOSBox Staging"],
        "preferred": "DOSBox Staging",
    },
    "Commodore 64": {
        "retroarch_cores": ["vice_x64"],
        "standalone": [],
        "preferred": "retroarch:vice_x64",
    },
    "Commodore Amiga": {
        "retroarch_cores": ["puae"],
        "standalone": [],
        "preferred": "retroarch:puae",
    },
    "Microsoft MSX": {
        "retroarch_cores": ["bluemsx", "fmsx"],
        "standalone": ["Ares"],
        "preferred": "retroarch:bluemsx",
    },
    "Microsoft MSX2": {
        "retroarch_cores": ["bluemsx", "fmsx"],
        "standalone": [],
        "preferred": "retroarch:bluemsx",
    },
    "Amstrad CPC": {
        "retroarch_cores": ["cap32"],
        "standalone": [],
        "preferred": "retroarch:cap32",
    },

    # Arcade
    "Arcade": {
        "retroarch_cores": ["fbneo", "mame"],
        "standalone": ["MAME"],
        "preferred": "MAME",
    },

    # Other
    "ScummVM": {
        "retroarch_cores": ["scummvm"],
        "standalone": ["ScummVM"],
        "preferred": "ScummVM",
    },
}


def generate_emulator_id(name: str) -> str:
    """Convert emulator name to a valid TOML key."""
    return name.lower().replace(" ", "_").replace("-", "_").replace(".", "")


def generate_toml() -> str:
    """Generate the emulators.toml content."""
    doc = document()

    # Header comment
    doc.add(comment("Lunchbox Emulator Configuration"))
    doc.add(comment("Auto-generated from emulation.gametechwiki.com, then manually curated"))
    doc.add(comment(""))
    doc.add(comment("Install types by platform:"))
    doc.add(comment("  Linux: flatpak (preferred), appimage, package"))
    doc.add(comment("  Windows: portable (preferred), installer"))
    doc.add(comment("  macOS: dmg, app, homebrew"))
    doc.add(nl())

    # Settings section
    settings = table()
    settings.add("default_install_path", "~/.local/share/lunchbox/emulators")
    settings.add("auto_update", True)
    settings.add("prefer_retroarch", True)
    doc.add("settings", settings)
    doc.add(nl())

    # Emulators section
    emulators = table()

    for emu_name, emu_data in sorted(KNOWN_EMULATORS.items()):
        emu_id = generate_emulator_id(emu_name)
        emu_table = table()

        emu_table.add("name", emu_data["name"])
        emu_table.add("description", emu_data["description"])
        emu_table.add("homepage", emu_data["homepage"])

        if emu_data.get("multi_system"):
            emu_table.add("multi_system", True)

        if emu_data.get("discontinued"):
            emu_table.add("discontinued", True)

        if emu_data.get("systems"):
            systems_arr = array()
            for sys in emu_data["systems"]:
                systems_arr.append(sys)
            emu_table.add("systems", systems_arr)

        # Platform-specific install info
        platforms = table()
        for plat_name, plat_data in emu_data.get("platforms", {}).items():
            plat_table = table()
            plat_table.add("install_type", plat_data["install_type"])

            for key, value in plat_data.items():
                if key != "install_type":
                    plat_table.add(key, value)

            platforms.add(plat_name, plat_table)

        if platforms:
            emu_table.add("platforms", platforms)

        # RetroArch cores
        if emu_data.get("cores"):
            cores = table()
            for core_system, core_list in sorted(emu_data["cores"].items()):
                core_arr = array()
                for core in core_list:
                    core_arr.append(core)
                cores.add(core_system, core_arr)
            emu_table.add("cores", cores)

        emulators.add(emu_id, emu_table)

    doc.add("emulators", emulators)
    doc.add(nl())

    # Systems section
    systems = table()

    for sys_name, sys_data in sorted(SYSTEM_EMULATOR_MAPPING.items()):
        sys_table = table()

        if sys_data.get("retroarch_cores"):
            cores_arr = array()
            for core in sys_data["retroarch_cores"]:
                cores_arr.append(core)
            sys_table.add("retroarch_cores", cores_arr)
        else:
            sys_table.add("retroarch_cores", array())

        if sys_data.get("standalone"):
            standalone_arr = array()
            for emu in sys_data["standalone"]:
                standalone_arr.append(emu)
            sys_table.add("standalone", standalone_arr)
        else:
            sys_table.add("standalone", array())

        if sys_data.get("preferred"):
            sys_table.add("preferred", sys_data["preferred"])

        if sys_data.get("notes"):
            sys_table.add("notes", sys_data["notes"])

        systems.add(sys_name, sys_table)

    doc.add("systems", systems)

    return tomlkit.dumps(doc)


def main():
    toml_content = generate_toml()

    output_path = Path("src-tauri/emulators.toml")
    output_path.write_text(toml_content)
    print(f"Generated {output_path}")

    # Also print some stats
    print(f"  Emulators: {len(KNOWN_EMULATORS)}")
    print(f"  Systems mapped: {len(SYSTEM_EMULATOR_MAPPING)}")


if __name__ == "__main__":
    main()
