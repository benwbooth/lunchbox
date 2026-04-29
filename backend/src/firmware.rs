use crate::db::schema::EmulatorInfo;
use crate::emulator;
use crate::emulator::LaunchArg;
use crate::state::AppSettings;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedFirmwarePackage {
    pub source: String,
    pub package_name: String,
    pub file_count: usize,
    pub extracted_root: String,
}

#[derive(Debug, Clone, FromRow)]
struct FirmwareRuleRow {
    rule_key: String,
    source: String,
    source_package_name: String,
    target_subdir: String,
    install_mode: String,
    required: i64,
}

#[derive(Debug, Clone, FromRow)]
struct ImportedPackageRow {
    id: i64,
    archive_path: String,
    extracted_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareStatus {
    pub rule_key: String,
    pub source: String,
    pub package_name: String,
    pub required: bool,
    pub supports_hle_fallback: bool,
    pub target_strategy: String,
    pub imported: bool,
    pub synced: bool,
    pub launch_scoped: bool,
    pub runtime_path: String,
}

#[derive(Debug, Clone, Copy)]
struct BuiltinFirmwareRule {
    rule_key: &'static str,
    runtime_kind: &'static str,
    runtime_name: &'static str,
    platform_name: &'static str,
    source: &'static str,
    source_package_name: &'static str,
    target_subdir: &'static str,
    install_mode: &'static str,
    required: bool,
    notes: &'static str,
}

#[derive(Debug, Clone)]
struct FirmwareRuntimeContext {
    runtime_kind: String,
    runtime_name: String,
    runtime_dir: Option<PathBuf>,
    display_name: String,
    runtime_path_display: String,
    launch_scoped: bool,
}

const BUILTIN_FIRMWARE_RULES: &[BuiltinFirmwareRule] = &[
    BuiltinFirmwareRule {
        rule_key: "retroarch:bk:Elektronika BK",
        runtime_kind: "retroarch",
        runtime_name: "bk",
        platform_name: "Elektronika BK",
        source: "minerva:retroarch-system-files",
        source_package_name: "Elektronika - BK-0010-BK-0011(M).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "RetroArch bk core BIOS pack.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:freechaf:Fairchild Channel F",
        runtime_kind: "retroarch",
        runtime_name: "freechaf",
        platform_name: "Fairchild Channel F",
        source: "minerva:retroarch-system-files",
        source_package_name: "Fairchild ChannelF (FreeChaF).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "FreeChaF BIOS files for Channel F.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:np2kai:NEC PC-9801",
        runtime_kind: "retroarch",
        runtime_name: "np2kai",
        platform_name: "NEC PC-9801",
        source: "minerva:retroarch-system-files",
        source_package_name: "NEC - PC-98 (Neko Project II - Kai).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "NP2kai BIOS and support files.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mupen64plus_next:Nintendo 64DD",
        runtime_kind: "retroarch",
        runtime_name: "mupen64plus_next",
        platform_name: "Nintendo 64DD",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - Nintendo 64 (Mupen64Plus-Next).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Includes IPL files needed for 64DD.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:pokemini:Nintendo Pokemon Mini",
        runtime_kind: "retroarch",
        runtime_name: "pokemini",
        platform_name: "Nintendo Pokemon Mini",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - Pokemon Mini (PokeMini).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional external PokeMini BIOS pack; FreeBIOS fallback remains available.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:px68k:Sharp X68000",
        runtime_kind: "retroarch",
        runtime_name: "px68k",
        platform_name: "Sharp X68000",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sharp - X68000 (PX68k).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PX68k BIOS files for X68000.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:fbneo:ColecoVision",
        runtime_kind: "retroarch",
        runtime_name: "fbneo",
        platform_name: "ColecoVision",
        source: "minerva:retroarch-system-files",
        source_package_name: "Coleco - ColecoVision (Gearcoleco).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Coleco BIOS files used by RetroArch cores.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:neocd_libretro:SNK Neo Geo CD",
        runtime_kind: "retroarch",
        runtime_name: "neocd_libretro",
        platform_name: "SNK Neo Geo CD",
        source: "minerva:retroarch-system-files",
        source_package_name: "SNK - Neo Geo CD (NeoCD).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Neo Geo CD BIOS pack for RetroArch NeoCD core.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:pcsx_rearmed:Sony Playstation",
        runtime_kind: "retroarch",
        runtime_name: "pcsx_rearmed",
        platform_name: "Sony Playstation",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (PCSX ReARMed).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "PS1 BIOS pack for PCSX-ReARMed.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:swanstation:Sony Playstation",
        runtime_kind: "retroarch",
        runtime_name: "swanstation",
        platform_name: "Sony Playstation",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (SwanStation).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS1 BIOS pack for SwanStation.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:swanstation:Sony - PlayStation Portable (PSX2PSP)",
        runtime_kind: "retroarch",
        runtime_name: "swanstation",
        platform_name: "Sony - PlayStation Portable (PSX2PSP)",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (SwanStation).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS1 BIOS pack for SwanStation PBP playback.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:beetle_psx:Sony Playstation",
        runtime_kind: "retroarch",
        runtime_name: "beetle_psx",
        platform_name: "Sony Playstation",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (Beetle PSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS1 BIOS pack for Beetle PSX.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:beetle_psx:Sony - PlayStation Portable (PSX2PSP)",
        runtime_kind: "retroarch",
        runtime_name: "beetle_psx",
        platform_name: "Sony - PlayStation Portable (PSX2PSP)",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (Beetle PSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS1 BIOS pack for Beetle PSX PBP playback.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:beetle_psx_hw:Sony Playstation",
        runtime_kind: "retroarch",
        runtime_name: "beetle_psx_hw",
        platform_name: "Sony Playstation",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (Beetle PSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS1 BIOS pack for Beetle PSX HW.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:beetle_psx_hw:Sony - PlayStation Portable (PSX2PSP)",
        runtime_kind: "retroarch",
        runtime_name: "beetle_psx_hw",
        platform_name: "Sony - PlayStation Portable (PSX2PSP)",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (Beetle PSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS1 BIOS pack for Beetle PSX HW PBP playback.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:pcsx2_libretro:Sony Playstation 2",
        runtime_kind: "retroarch",
        runtime_name: "pcsx2_libretro",
        platform_name: "Sony Playstation 2",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation 2 (LRPS2).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS2 BIOS pack for LRPS2.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:fbneo:SNK Neo Geo CD",
        runtime_kind: "retroarch",
        runtime_name: "fbneo",
        platform_name: "SNK Neo Geo CD",
        source: "minerva:retroarch-system-files",
        source_package_name: "Arcade (FinalBurn Neo).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "FBNeo BIOS pack for Neo Geo CD.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:flycast:Sega Dreamcast",
        runtime_kind: "retroarch",
        runtime_name: "flycast",
        platform_name: "Sega Dreamcast",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sega - Dreamcast - NAOMI (Flycast).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional Flycast firmware pack for Dreamcast; HLE boot remains possible without it.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:flycast:Sega Naomi 2",
        runtime_kind: "retroarch",
        runtime_name: "flycast",
        platform_name: "Sega Naomi 2",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sega - Dreamcast - NAOMI (Flycast).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Flycast BIOS pack for Naomi 2.",
    },
    BuiltinFirmwareRule {
        rule_key: "flycast:Flycast:Sega Dreamcast",
        runtime_kind: "flycast",
        runtime_name: "Flycast",
        platform_name: "Sega Dreamcast",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sega - Dreamcast - NAOMI (Flycast).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional standalone Flycast firmware pack for Dreamcast; HLE boot remains possible without it.",
    },
    BuiltinFirmwareRule {
        rule_key: "flycast:Flycast:Sega Naomi",
        runtime_kind: "flycast",
        runtime_name: "Flycast",
        platform_name: "Sega Naomi",
        source: "minerva:flycast-bios-files",
        source_package_name: "Arcade (Flycast) BIOS Files.zip",
        target_subdir: "data",
        install_mode: "merge_tree",
        required: true,
        notes: "Standalone Flycast BIOS pack for Naomi.",
    },
    BuiltinFirmwareRule {
        rule_key: "flycast:Flycast:Sega Naomi 2",
        runtime_kind: "flycast",
        runtime_name: "Flycast",
        platform_name: "Sega Naomi 2",
        source: "minerva:flycast-bios-files",
        source_package_name: "Arcade (Flycast) BIOS Files.zip",
        target_subdir: "data",
        install_mode: "merge_tree",
        required: true,
        notes: "Standalone Flycast BIOS pack for Naomi 2.",
    },
    BuiltinFirmwareRule {
        rule_key: "flycast:Flycast:Sammy Atomiswave",
        runtime_kind: "flycast",
        runtime_name: "Flycast",
        platform_name: "Sammy Atomiswave",
        source: "minerva:flycast-bios-files",
        source_package_name: "Arcade (Flycast) BIOS Files.zip",
        target_subdir: "data",
        install_mode: "merge_tree",
        required: true,
        notes: "Standalone Flycast BIOS pack for Atomiswave.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:bluemsx_libretro:Microsoft MSX",
        runtime_kind: "retroarch",
        runtime_name: "bluemsx_libretro",
        platform_name: "Microsoft MSX",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "blueMSX Databases and Machines pack for MSX.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:bluemsx_libretro:Microsoft MSX2",
        runtime_kind: "retroarch",
        runtime_name: "bluemsx_libretro",
        platform_name: "Microsoft MSX2",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "blueMSX Databases and Machines pack for MSX2.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:bluemsx:Microsoft MSX2",
        runtime_kind: "retroarch",
        runtime_name: "bluemsx",
        platform_name: "Microsoft MSX2",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "blueMSX Databases and Machines pack for MSX2.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:bluemsx_libretro:Microsoft MSX2+",
        runtime_kind: "retroarch",
        runtime_name: "bluemsx_libretro",
        platform_name: "Microsoft MSX2+",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "blueMSX Databases and Machines pack for MSX2+.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:bluemsx:Microsoft MSX2+",
        runtime_kind: "retroarch",
        runtime_name: "bluemsx",
        platform_name: "Microsoft MSX2+",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "blueMSX Databases and Machines pack for MSX2+.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:bluemsx_libretro:Spectravideo",
        runtime_kind: "retroarch",
        runtime_name: "bluemsx_libretro",
        platform_name: "Spectravideo",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "blueMSX Databases and Machines pack for Spectravideo.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:bluemsx:Spectravideo",
        runtime_kind: "retroarch",
        runtime_name: "bluemsx",
        platform_name: "Spectravideo",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "blueMSX Databases and Machines pack for Spectravideo.",
    },
    BuiltinFirmwareRule {
        rule_key: "openmsx:openMSX:Microsoft MSX",
        runtime_kind: "openmsx",
        runtime_name: "openMSX",
        platform_name: "Microsoft MSX",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional real-machine MSX system ROM pool for openMSX. C-BIOS remains the default fallback.",
    },
    BuiltinFirmwareRule {
        rule_key: "openmsx:openMSX:Microsoft MSX2",
        runtime_kind: "openmsx",
        runtime_name: "openMSX",
        platform_name: "Microsoft MSX2",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional real-machine MSX2 system ROM pool for openMSX. C-BIOS remains the default fallback.",
    },
    BuiltinFirmwareRule {
        rule_key: "openmsx:openMSX:Microsoft MSX2+",
        runtime_kind: "openmsx",
        runtime_name: "openMSX",
        platform_name: "Microsoft MSX2+",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional real-machine MSX2+ system ROM pool for openMSX. C-BIOS remains the default fallback.",
    },
    BuiltinFirmwareRule {
        rule_key: "openmsx:openMSX:Spectravideo",
        runtime_kind: "openmsx",
        runtime_name: "openMSX",
        platform_name: "Spectravideo",
        source: "minerva:retroarch-system-files",
        source_package_name: "MSX-SVI-ColecoVision-SG1000 (blueMSX).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Required Spectravideo SVI system ROM pool for openMSX.",
    },
    BuiltinFirmwareRule {
        rule_key: "snes9x:Snes9x:Nintendo Satellaview",
        runtime_kind: "snes9x",
        runtime_name: "Snes9x",
        platform_name: "Nintendo Satellaview",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - SNES - SFC (Snes9x).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Standalone Snes9x BIOS folder for Satellaview BS-X support.",
    },
    BuiltinFirmwareRule {
        rule_key: "o2em:O2EM:Philips Videopac+",
        runtime_kind: "o2em",
        runtime_name: "O2EM",
        platform_name: "Philips Videopac+",
        source: "minerva:retroarch-system-files",
        source_package_name: "Magnavox - Odyssey2 - Philips Videopac+ (O2EM).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Standalone O2EM BIOS files for Odyssey2/Videopac+.",
    },
    BuiltinFirmwareRule {
        rule_key: "loopymse:LoopyMSE:Casio Loopy",
        runtime_kind: "loopymse",
        runtime_name: "LoopyMSE",
        platform_name: "Casio Loopy",
        source: "minerva:mame-merged-romsets",
        source_package_name: "casloopy.zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "LoopyMSE BIOS package. The main BIOS is required; the sound BIOS is optional and passed when present.",
    },
    BuiltinFirmwareRule {
        rule_key: "dolphin:Dolphin:Nintendo GameCube",
        runtime_kind: "dolphin",
        runtime_name: "Dolphin",
        platform_name: "Nintendo GameCube",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - GameCube - Wii (Dolphin).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional Dolphin user-data package for GameCube IPL/fonts and related assets.",
    },
    BuiltinFirmwareRule {
        rule_key: "dolphin:Dolphin:Nintendo Wii",
        runtime_kind: "dolphin",
        runtime_name: "Dolphin",
        platform_name: "Nintendo Wii",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - GameCube - Wii (Dolphin).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional Dolphin user-data package for GameCube/Wii system assets.",
    },
    BuiltinFirmwareRule {
        rule_key: "dolphin:Dolphin:Nintendo - Wii (Digital)",
        runtime_kind: "dolphin",
        runtime_name: "Dolphin",
        platform_name: "Nintendo - Wii (Digital)",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - GameCube - Wii (Dolphin).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: false,
        notes: "Optional Dolphin user-data package for GameCube/Wii system assets.",
    },
    BuiltinFirmwareRule {
        rule_key: "mgba:mGBA:Nintendo - e-Reader",
        runtime_kind: "mgba",
        runtime_name: "mGBA",
        platform_name: "Nintendo - e-Reader",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - Game Boy Advance (mGBA - VBA-M).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Standalone mGBA BIOS pack resolved and passed as --bios at launch for e-Reader support.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:melonds_ds:Nintendo DS",
        runtime_kind: "retroarch",
        runtime_name: "melonds_ds",
        platform_name: "Nintendo DS",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - DS (DeSmuME - melonDS).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "DS BIOS and firmware pack for melonDS DS.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:melonds_ds:Nintendo - Nintendo DS (Download Play)",
        runtime_kind: "retroarch",
        runtime_name: "melonds_ds",
        platform_name: "Nintendo - Nintendo DS (Download Play)",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - DS (DeSmuME - melonDS).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "DS BIOS and firmware pack for melonDS DS Download Play.",
    },
    BuiltinFirmwareRule {
        rule_key: "duckstation:DuckStation:Sony Playstation",
        runtime_kind: "duckstation",
        runtime_name: "DuckStation",
        platform_name: "Sony Playstation",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (SwanStation).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS1 BIOS pack for DuckStation.",
    },
    BuiltinFirmwareRule {
        rule_key: "duckstation:DuckStation:Sony - PlayStation Portable (PSX2PSP)",
        runtime_kind: "duckstation",
        runtime_name: "DuckStation",
        platform_name: "Sony - PlayStation Portable (PSX2PSP)",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation (SwanStation).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS1 BIOS pack for DuckStation PBP playback.",
    },
    BuiltinFirmwareRule {
        rule_key: "pcsx2:PCSX2:Sony Playstation 2",
        runtime_kind: "pcsx2",
        runtime_name: "PCSX2",
        platform_name: "Sony Playstation 2",
        source: "minerva:retroarch-system-files",
        source_package_name: "Sony - PlayStation 2 (LRPS2).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PS2 BIOS pack for PCSX2.",
    },
    BuiltinFirmwareRule {
        rule_key: "melonds:melonDS:Nintendo DS",
        runtime_kind: "melonds",
        runtime_name: "melonDS",
        platform_name: "Nintendo DS",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - DS (DeSmuME - melonDS).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "DS BIOS and firmware pack for melonDS.",
    },
    BuiltinFirmwareRule {
        rule_key: "melonds:melonDS:Nintendo - Nintendo DS (Download Play)",
        runtime_kind: "melonds",
        runtime_name: "melonDS",
        platform_name: "Nintendo - Nintendo DS (Download Play)",
        source: "minerva:retroarch-system-files",
        source_package_name: "Nintendo - DS (DeSmuME - melonDS).zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "DS BIOS and firmware pack for melonDS Download Play.",
    },
    BuiltinFirmwareRule {
        rule_key: "melonds:melonDS:Nintendo - Nintendo DSi",
        runtime_kind: "melonds",
        runtime_name: "melonDS",
        platform_name: "Nintendo - Nintendo DSi",
        source: "manual:melonds-dsi-system",
        source_package_name: "DSi BIOS, firmware, and NAND image",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Standalone melonDS DSi mode requires manually dumped DSi BIOS, firmware, and NAND files; Lunchbox does not have a Minerva source package for these yet.",
    },
    BuiltinFirmwareRule {
        rule_key: "m88kai:M88kai:NEC PC-8801",
        runtime_kind: "m88kai",
        runtime_name: "M88kai",
        platform_name: "NEC PC-8801",
        source: "manual:m88kai-bios",
        source_package_name: "pc8801.zip / pc8801mk2sr.zip BIOS ROMs",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "M88kai needs PC-8801 BIOS ROMs in the emulator directory. Lunchbox exposes a managed import folder, but does not auto-install this standalone BIOS layout yet.",
    },
    BuiltinFirmwareRule {
        rule_key: "tsugaru:Tsugaru:Fujitsu FM Towns Marty",
        runtime_kind: "tsugaru",
        runtime_name: "Tsugaru",
        platform_name: "Fujitsu FM Towns Marty",
        source: "manual:tsugaru-fmtowns",
        source_package_name: "fmtowns.zip and CMOS.BIN",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Tsugaru needs FM Towns ROM assets plus a CMOS image. Lunchbox exposes a managed import folder, but Tsugaru's standalone ROM/CMOS layout remains manual.",
    },
    BuiltinFirmwareRule {
        rule_key: "unz:UNZ:Fujitsu FM Towns Marty",
        runtime_kind: "unz",
        runtime_name: "UNZ",
        platform_name: "Fujitsu FM Towns Marty",
        source: "manual:unz-fmtowns",
        source_package_name: "fmtowns.zip and cmos.dat",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "UNZ needs FM Towns ROM assets plus cmos.dat in its emulator directory. Lunchbox exposes a managed import folder, but UNZ remains manual-only.",
    },
    BuiltinFirmwareRule {
        rule_key: "emu5:Emu5 (Common Source Code Project):Sord M5",
        runtime_kind: "emu5",
        runtime_name: "Emu5 (Common Source Code Project)",
        platform_name: "Sord M5",
        source: "manual:emu5-sordm5",
        source_package_name: "m5.zip / m5p.zip BIOS ROMs",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "The standalone Emu5 Sord M5 path still expects BIOS ROMs in the emulator directory. Lunchbox exposes a managed import folder instead of guessing the final layout.",
    },
    BuiltinFirmwareRule {
        rule_key: "demul:DEmul:Sammy Atomiswave",
        runtime_kind: "demul",
        runtime_name: "DEmul",
        platform_name: "Sammy Atomiswave",
        source: "manual:demul-awbios",
        source_package_name: "awbios.zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "DEmul expects the Atomiswave BIOS romset to be added through its Roms and Bioses Paths configuration. Lunchbox exposes a managed import folder instead of mutating DEmul directly.",
    },
    BuiltinFirmwareRule {
        rule_key: "demul:DEmul:Sega Hikaru",
        runtime_kind: "demul",
        runtime_name: "DEmul",
        platform_name: "Sega Hikaru",
        source: "manual:demul-hikaru",
        source_package_name: "hikaru.zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "DEmul expects the Hikaru BIOS romset to be added through its Roms and Bioses Paths configuration. Lunchbox exposes a managed import folder instead of mutating DEmul directly.",
    },
    BuiltinFirmwareRule {
        rule_key: "demul:DEmul:Sega Naomi",
        runtime_kind: "demul",
        runtime_name: "DEmul",
        platform_name: "Sega Naomi",
        source: "manual:demul-naomi",
        source_package_name: "naomi.zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "DEmul expects the NAOMI BIOS romset to be added through its Roms and Bioses Paths configuration. Lunchbox exposes a managed import folder instead of mutating DEmul directly.",
    },
    BuiltinFirmwareRule {
        rule_key: "demul:DEmul:Sega Naomi 2",
        runtime_kind: "demul",
        runtime_name: "DEmul",
        platform_name: "Sega Naomi 2",
        source: "manual:demul-naomi2",
        source_package_name: "naomi2.zip",
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "DEmul expects the NAOMI 2 BIOS romset to be added through its Roms and Bioses Paths configuration. Lunchbox exposes a managed import folder instead of mutating DEmul directly.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Aamber Pegasus",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Aamber Pegasus",
        source: "minerva:mame-merged-romsets",
        source_package_name: "pegasus.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Aamber Pegasus.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Casio Loopy",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Casio Loopy",
        source: "minerva:mame-merged-romsets",
        source_package_name: "casloopy.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Casio Loopy.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Emerson Arcadia 2001:ar_bios",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Emerson Arcadia 2001",
        source: "minerva:mame-merged-romsets",
        source_package_name: "ar_bios.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Emerson Arcadia 2001.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Emerson Arcadia 2001:arcadia_hash",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Emerson Arcadia 2001",
        source: "github:mame-hash-files",
        source_package_name: "arcadia.xml",
        target_subdir: "@hash",
        install_mode: "merge_tree",
        required: true,
        notes: "MAME software-list hash file for Emerson Arcadia 2001.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:TRS-80 Color Computer",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "TRS-80 Color Computer",
        source: "minerva:mame-merged-romsets",
        source_package_name: "coco.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Primary MAME machine romset for TRS-80 Color Computer.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Coleco ADAM:adam",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Coleco ADAM.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Coleco ADAM:adam_ddp",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam_ddp.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME DDP romset for Coleco ADAM.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Coleco ADAM:adam_fdc",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam_fdc.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME FDC romset for Coleco ADAM.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Coleco ADAM:adam_kb",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam_kb.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME keyboard romset for Coleco ADAM.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Coleco ADAM:adam_prn",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam_prn.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME printer romset for Coleco ADAM.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Elektronika BK:bk0010",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Elektronika BK",
        source: "minerva:mame-merged-romsets",
        source_package_name: "bk0010.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Primary MAME machine romset for Elektronika BK.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Elektronika BK:bk0011m",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Elektronika BK",
        source: "minerva:mame-non-merged-romsets",
        source_package_name: "bk0011m.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Secondary MAME machine romset for Elektronika BK.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Entex Adventure Vision",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Entex Adventure Vision",
        source: "minerva:mame-merged-romsets",
        source_package_name: "advision.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Entex Adventure Vision.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Epoch Game Pocket Computer",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Epoch Game Pocket Computer",
        source: "minerva:mame-merged-romsets",
        source_package_name: "gamepock.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Epoch Game Pocket Computer.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Exidy Sorcerer:sorcerer",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Exidy Sorcerer",
        source: "minerva:mame-merged-romsets",
        source_package_name: "sorcerer.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Primary MAME BIOS romset for Exidy Sorcerer.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Exidy Sorcerer:sorcererd",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Exidy Sorcerer",
        source: "minerva:mame-non-merged-romsets",
        source_package_name: "sorcererd.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Secondary MAME BIOS romset for Exidy Sorcerer.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:GamePark GP32",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "GamePark GP32",
        source: "minerva:mame-merged-romsets",
        source_package_name: "gp32.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for GP32.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Memotech MTX512",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Memotech MTX512",
        source: "minerva:mame-merged-romsets",
        source_package_name: "mtx512.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME romset for Memotech MTX512.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:RCA Studio II",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "RCA Studio II",
        source: "minerva:mame-merged-romsets",
        source_package_name: "studio2.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for RCA Studio II.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Sharp X68000",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Sharp X68000",
        source: "minerva:mame-merged-romsets",
        source_package_name: "x68000.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME machine romset for Sharp X68000.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Sord M5:m5",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Sord M5",
        source: "minerva:mame-merged-romsets",
        source_package_name: "m5.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Primary MAME machine romset for Sord M5.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Sord M5:m5p",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Sord M5",
        source: "minerva:mame-non-merged-romsets",
        source_package_name: "m5p.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "European MAME machine romset for Sord M5.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Sony PocketStation",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Sony PocketStation",
        source: "minerva:mame-merged-romsets",
        source_package_name: "pockstat.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Sony PocketStation.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Sega ST-V",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Sega ST-V",
        source: "minerva:mame-merged-romsets",
        source_package_name: "stvbios.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Sega ST-V.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:SNK Neo Geo AES",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "SNK Neo Geo AES",
        source: "minerva:mame-merged-romsets",
        source_package_name: "neogeo.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Neo Geo AES.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:SNK Neo Geo MVS",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "SNK Neo Geo MVS",
        source: "minerva:mame-merged-romsets",
        source_package_name: "neogeo.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Neo Geo MVS.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:SNK Neo Geo CD",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "SNK Neo Geo CD",
        source: "minerva:mame-merged-romsets",
        source_package_name: "neocdz.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Neo Geo CD.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Texas Instruments TI 99/4A",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Texas Instruments TI 99/4A",
        source: "minerva:mame-merged-romsets",
        source_package_name: "ti99_4a.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for TI-99/4A.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:Tomy Tutor",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "Tomy Tutor",
        source: "minerva:mame-merged-romsets",
        source_package_name: "tutor.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for Tomy Tutor.",
    },
    BuiltinFirmwareRule {
        rule_key: "mame:MAME:VTech V.Smile",
        runtime_kind: "mame",
        runtime_name: "MAME",
        platform_name: "VTech V.Smile",
        source: "minerva:mame-merged-romsets",
        source_package_name: "vsmile.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset for VTech V.Smile.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Aamber Pegasus",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Aamber Pegasus",
        source: "minerva:mame-merged-romsets",
        source_package_name: "pegasus.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Casio Loopy",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Casio Loopy",
        source: "minerva:mame-merged-romsets",
        source_package_name: "casloopy.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:TRS-80 Color Computer",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "TRS-80 Color Computer",
        source: "minerva:mame-merged-romsets",
        source_package_name: "coco.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Primary MAME machine romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Coleco ADAM:adam",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Coleco ADAM:adam_ddp",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam_ddp.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME DDP romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Coleco ADAM:adam_fdc",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam_fdc.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME FDC romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Coleco ADAM:adam_kb",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam_kb.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME keyboard romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Coleco ADAM:adam_prn",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Coleco ADAM",
        source: "minerva:mame-merged-romsets",
        source_package_name: "adam_prn.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME printer romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Elektronika BK:bk0010",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Elektronika BK",
        source: "minerva:mame-merged-romsets",
        source_package_name: "bk0010.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Primary MAME machine romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Elektronika BK:bk0011m",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Elektronika BK",
        source: "minerva:mame-non-merged-romsets",
        source_package_name: "bk0011m.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Secondary MAME machine romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Entex Adventure Vision",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Entex Adventure Vision",
        source: "minerva:mame-merged-romsets",
        source_package_name: "advision.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Epoch Game Pocket Computer",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Epoch Game Pocket Computer",
        source: "minerva:mame-merged-romsets",
        source_package_name: "gamepock.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Exidy Sorcerer:sorcerer",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Exidy Sorcerer",
        source: "minerva:mame-merged-romsets",
        source_package_name: "sorcerer.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Primary MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Exidy Sorcerer:sorcererd",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Exidy Sorcerer",
        source: "minerva:mame-non-merged-romsets",
        source_package_name: "sorcererd.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Secondary MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:GamePark GP32",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "GamePark GP32",
        source: "minerva:mame-merged-romsets",
        source_package_name: "gp32.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Memotech MTX512",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Memotech MTX512",
        source: "minerva:mame-merged-romsets",
        source_package_name: "mtx512.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:RCA Studio II",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "RCA Studio II",
        source: "minerva:mame-merged-romsets",
        source_package_name: "studio2.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Sharp X68000",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Sharp X68000",
        source: "minerva:mame-merged-romsets",
        source_package_name: "x68000.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME machine romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Sord M5:m5",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Sord M5",
        source: "minerva:mame-merged-romsets",
        source_package_name: "m5.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "Primary MAME machine romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Sord M5:m5p",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Sord M5",
        source: "minerva:mame-non-merged-romsets",
        source_package_name: "m5p.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "European MAME machine romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Sony PocketStation",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Sony PocketStation",
        source: "minerva:mame-merged-romsets",
        source_package_name: "pockstat.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Sega ST-V",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Sega ST-V",
        source: "minerva:mame-merged-romsets",
        source_package_name: "stvbios.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:SNK Neo Geo AES",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "SNK Neo Geo AES",
        source: "minerva:mame-merged-romsets",
        source_package_name: "neogeo.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:SNK Neo Geo MVS",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "SNK Neo Geo MVS",
        source: "minerva:mame-merged-romsets",
        source_package_name: "neogeo.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:SNK Neo Geo CD",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "SNK Neo Geo CD",
        source: "minerva:mame-merged-romsets",
        source_package_name: "neocdz.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Texas Instruments TI 99/4A",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Texas Instruments TI 99/4A",
        source: "minerva:mame-merged-romsets",
        source_package_name: "ti99_4a.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:Tomy Tutor",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "Tomy Tutor",
        source: "minerva:mame-merged-romsets",
        source_package_name: "tutor.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "retroarch:mame:VTech V.Smile",
        runtime_kind: "retroarch",
        runtime_name: "mame",
        platform_name: "VTech V.Smile",
        source: "minerva:mame-merged-romsets",
        source_package_name: "vsmile.zip",
        target_subdir: "",
        install_mode: "copy_archive",
        required: true,
        notes: "MAME BIOS romset copied beside the ROM for RetroArch MAME.",
    },
    BuiltinFirmwareRule {
        rule_key: "86box:86Box:MS-DOS",
        runtime_kind: "86box",
        runtime_name: "86Box",
        platform_name: "MS-DOS",
        source: "github:86box-romset",
        source_package_name: EIGHTY_SIX_BOX_ROMSET_PACKAGE_NAME,
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Official 86Box machine ROM set for DOS machines.",
    },
    BuiltinFirmwareRule {
        rule_key: "86box:86Box:Windows",
        runtime_kind: "86box",
        runtime_name: "86Box",
        platform_name: "Windows",
        source: "github:86box-romset",
        source_package_name: EIGHTY_SIX_BOX_ROMSET_PACKAGE_NAME,
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Official 86Box machine ROM set for Windows-capable machines.",
    },
    BuiltinFirmwareRule {
        rule_key: "86box:86Box:Windows 3.X",
        runtime_kind: "86box",
        runtime_name: "86Box",
        platform_name: "Windows 3.X",
        source: "github:86box-romset",
        source_package_name: EIGHTY_SIX_BOX_ROMSET_PACKAGE_NAME,
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "Official 86Box machine ROM set for Windows 3.x-capable machines.",
    },
    BuiltinFirmwareRule {
        rule_key: "pcem:PCem:Windows",
        runtime_kind: "pcem",
        runtime_name: "PCem",
        platform_name: "Windows",
        source: "github:pcem-romset",
        source_package_name: PCEM_ROMSET_PACKAGE_NAME,
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PCem machine ROM set for Windows-capable machines.",
    },
    BuiltinFirmwareRule {
        rule_key: "pcem:PCem:Windows 3.X",
        runtime_kind: "pcem",
        runtime_name: "PCem",
        platform_name: "Windows 3.X",
        source: "github:pcem-romset",
        source_package_name: PCEM_ROMSET_PACKAGE_NAME,
        target_subdir: "",
        install_mode: "merge_tree",
        required: true,
        notes: "PCem machine ROM set for Windows 3.x-capable machines.",
    },
];

const RETROARCH_SYSTEM_FILES_TORRENT_FILE: &str =
    "Minerva_Myrient_v0.3/Minerva_Myrient - Internet Archive - chadmaster.torrent";
const RETROARCH_SYSTEM_FILES_PATH_PREFIX: &str =
    "Internet Archive/chadmaster/RetroarchSystemFiles/Retroarch-System/";
const CHADMASTER_MAME_MERGED_PATH_PREFIX: &str =
    "Internet Archive/chadmaster/mame-merged/mame-merged/";
const FLYCAST_BIOS_TORRENT_FILE: &str =
    "Minerva_Myrient_v0.3/Minerva_Myrient - Internet Archive - retro_game_champion.torrent";
const FLYCAST_BIOS_PATH_PREFIX: &str =
    "Internet Archive/retro_game_champion/Game Sets/Arcade (Flycast) Champion Collection/";
const MAME_NONMERGED_TORRENT_FILE: &str =
    "Minerva_Myrient_v0.3/Minerva_Myrient - MAME - ROMs (non-merged).torrent";
const MAME_NONMERGED_PATH_PREFIX: &str = "MAME/ROMs (non-merged)/";
const MAME_HASH_ARCADIA_PACKAGE_NAME: &str = "arcadia.xml";
const MAME_HASH_ARCADIA_RAW_URL: &str =
    "https://raw.githubusercontent.com/mamedev/mame/master/hash/arcadia.xml";
const EIGHTY_SIX_BOX_ROMSET_PACKAGE_NAME: &str = "86Box ROM set.zip";
const EIGHTY_SIX_BOX_RELEASES_API: &str = "https://api.github.com/repos/86Box/roms/releases/latest";
const PCEM_ROMSET_PACKAGE_NAME: &str = "PCem ROM set.zip";
const PCEM_RELEASES_API: &str =
    "https://api.github.com/repos/BaRRaKudaRain/PCem-ROMs/releases/latest";

pub async fn sync_builtin_rules(pool: &SqlitePool) -> Result<(), String> {
    for rule in BUILTIN_FIRMWARE_RULES {
        sqlx::query(
            r#"
            INSERT INTO firmware_rules (
                rule_key, runtime_kind, runtime_name, platform_name, source,
                source_package_name, target_subdir, install_mode, required, notes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(rule_key) DO UPDATE SET
                runtime_kind = excluded.runtime_kind,
                runtime_name = excluded.runtime_name,
                platform_name = excluded.platform_name,
                source = excluded.source,
                source_package_name = excluded.source_package_name,
                target_subdir = excluded.target_subdir,
                install_mode = excluded.install_mode,
                required = excluded.required,
                notes = excluded.notes,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(rule.rule_key)
        .bind(rule.runtime_kind)
        .bind(rule.runtime_name)
        .bind(rule.platform_name)
        .bind(rule.source)
        .bind(rule.source_package_name)
        .bind(rule.target_subdir)
        .bind(rule.install_mode)
        .bind(if rule.required { 1 } else { 0 })
        .bind(rule.notes)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn resolve_runtime_context(
    emulator_info: &EmulatorInfo,
    platform_name: &str,
    as_retroarch_core: bool,
) -> Option<FirmwareRuntimeContext> {
    resolve_runtime_context_for_launch(emulator_info, platform_name, as_retroarch_core, None)
}

fn resolve_runtime_context_for_launch(
    emulator_info: &EmulatorInfo,
    platform_name: &str,
    as_retroarch_core: bool,
    rom_path: Option<&Path>,
) -> Option<FirmwareRuntimeContext> {
    if as_retroarch_core {
        let runtime_name = emulator_info.retroarch_core.as_deref()?;
        if runtime_name == "mame" {
            let runtime_dir = rom_path.and_then(|path| path.parent().map(Path::to_path_buf));
            let runtime_path_display = runtime_dir
                .as_ref()
                .map(|dir| dir.display().to_string())
                .unwrap_or_else(|| "Copied beside the game ROM at launch".to_string());

            return Some(FirmwareRuntimeContext {
                runtime_kind: "retroarch".to_string(),
                runtime_name: runtime_name.to_string(),
                runtime_dir,
                display_name: format!("RetroArch core '{}'", runtime_name),
                runtime_path_display,
                launch_scoped: true,
            });
        }

        let runtime_dir = retroarch_system_dir()?;
        return Some(FirmwareRuntimeContext {
            runtime_kind: "retroarch".to_string(),
            runtime_name: runtime_name.to_string(),
            runtime_dir: Some(runtime_dir.clone()),
            display_name: format!("RetroArch core '{}'", runtime_name),
            runtime_path_display: runtime_dir.display().to_string(),
            launch_scoped: false,
        });
    }

    match emulator_info.name.as_str() {
        "DuckStation"
            if matches!(
                platform_name,
                "Sony Playstation" | "Sony - PlayStation Portable (PSX2PSP)"
            ) =>
        {
            Some(FirmwareRuntimeContext {
                runtime_kind: "duckstation".to_string(),
                runtime_name: "DuckStation".to_string(),
                runtime_dir: Some(duckstation_bios_dir(emulator_info)?),
                display_name: "DuckStation".to_string(),
                runtime_path_display: duckstation_bios_dir(emulator_info)?.display().to_string(),
                launch_scoped: false,
            })
        }
        "PCSX2" if platform_name == "Sony Playstation 2" => Some(FirmwareRuntimeContext {
            runtime_kind: "pcsx2".to_string(),
            runtime_name: "PCSX2".to_string(),
            runtime_dir: Some(pcsx2_bios_dir(emulator_info)?),
            display_name: "PCSX2".to_string(),
            runtime_path_display: pcsx2_bios_dir(emulator_info)?.display().to_string(),
            launch_scoped: false,
        }),
        "melonDS"
            if matches!(
                platform_name,
                "Nintendo DS" | "Nintendo - Nintendo DS (Download Play)"
            ) =>
        {
            Some(FirmwareRuntimeContext {
                runtime_kind: "melonds".to_string(),
                runtime_name: "melonDS".to_string(),
                runtime_dir: Some(melonds_firmware_dir(emulator_info)?),
                display_name: "melonDS".to_string(),
                runtime_path_display: melonds_firmware_dir(emulator_info)?.display().to_string(),
                launch_scoped: false,
            })
        }
        "melonDS" if platform_name == "Nintendo - Nintendo DSi" => Some(FirmwareRuntimeContext {
            runtime_kind: "melonds".to_string(),
            runtime_name: "melonDS".to_string(),
            runtime_dir: None,
            display_name: "melonDS".to_string(),
            runtime_path_display: "Configure DSi BIOS/firmware/NAND paths in melonDS Emu Settings"
                .to_string(),
            launch_scoped: false,
        }),

        "M88kai" if platform_name == "NEC PC-8801" => Some(FirmwareRuntimeContext {
            runtime_kind: "m88kai".to_string(),
            runtime_name: "M88kai".to_string(),
            runtime_dir: None,
            display_name: "M88kai".to_string(),
            runtime_path_display: "Place the required PC-8801 BIOS ROMs in the M88kai emulator directory"
                .to_string(),
            launch_scoped: false,
        }),
        "Tsugaru" if platform_name == "Fujitsu FM Towns Marty" => Some(FirmwareRuntimeContext {
            runtime_kind: "tsugaru".to_string(),
            runtime_name: "Tsugaru".to_string(),
            runtime_dir: None,
            display_name: "Tsugaru".to_string(),
            runtime_path_display: "Configure FM Towns ROM assets and CMOS.BIN for Tsugaru (use -CMOS for settings)"
                .to_string(),
            launch_scoped: false,
        }),
        "UNZ" if platform_name == "Fujitsu FM Towns Marty" => Some(FirmwareRuntimeContext {
            runtime_kind: "unz".to_string(),
            runtime_name: "UNZ".to_string(),
            runtime_dir: None,
            display_name: "UNZ".to_string(),
            runtime_path_display: "Place FM Towns ROM assets and cmos.dat in the UNZ emulator directory"
                .to_string(),
            launch_scoped: false,
        }),
        "Emu5 (Common Source Code Project)" if platform_name == "Sord M5" => Some(FirmwareRuntimeContext {
            runtime_kind: "emu5".to_string(),
            runtime_name: "Emu5 (Common Source Code Project)".to_string(),
            runtime_dir: None,
            display_name: "Emu5 (Common Source Code Project)".to_string(),
            runtime_path_display: "Place the required Sord M5 BIOS ROMs in the Emu5 emulator directory"
                .to_string(),
            launch_scoped: false,
        }),
        "DEmul"
            if matches!(
                platform_name,
                "Sammy Atomiswave" | "Sega Hikaru" | "Sega Naomi" | "Sega Naomi 2"
            ) =>
        {
            Some(FirmwareRuntimeContext {
                runtime_kind: "demul".to_string(),
                runtime_name: "DEmul".to_string(),
                runtime_dir: None,
                display_name: "DEmul".to_string(),
                runtime_path_display: "Configure the appropriate BIOS romset through DEmul's Roms and Bioses Paths settings"
                    .to_string(),
                launch_scoped: false,
            })
        },
        "Dolphin"
            if matches!(
                platform_name,
                "Nintendo GameCube" | "Nintendo Wii" | "Nintendo - Wii (Digital)"
            ) =>
        {
            Some(FirmwareRuntimeContext {
                runtime_kind: "dolphin".to_string(),
                runtime_name: "Dolphin".to_string(),
                runtime_dir: Some(dolphin_user_dir(emulator_info)?),
                display_name: "Dolphin".to_string(),
                runtime_path_display: dolphin_user_dir(emulator_info)?.display().to_string(),
                launch_scoped: false,
            })
        }
        "Flycast"
            if matches!(
                platform_name,
                "Sega Dreamcast" | "Sega Naomi" | "Sega Naomi 2" | "Sammy Atomiswave"
            ) =>
        {
            Some(FirmwareRuntimeContext {
                runtime_kind: "flycast".to_string(),
                runtime_name: "Flycast".to_string(),
                runtime_dir: Some(flycast_runtime_dir(emulator_info)?),
                display_name: "Flycast".to_string(),
                runtime_path_display: flycast_runtime_dir(emulator_info)?.display().to_string(),
                launch_scoped: false,
            })
        }
        "openMSX"
            if matches!(
                platform_name,
                "Microsoft MSX" | "Microsoft MSX2" | "Microsoft MSX2+" | "Spectravideo"
            ) =>
        {
            Some(FirmwareRuntimeContext {
                runtime_kind: "openmsx".to_string(),
                runtime_name: "openMSX".to_string(),
                runtime_dir: Some(openmsx_systemroms_dir(emulator_info)?),
                display_name: "openMSX".to_string(),
                runtime_path_display: openmsx_systemroms_dir(emulator_info)?.display().to_string(),
                launch_scoped: false,
            })
        }
        "Snes9x" if platform_name == "Nintendo Satellaview" => Some(FirmwareRuntimeContext {
            runtime_kind: "snes9x".to_string(),
            runtime_name: "Snes9x".to_string(),
            runtime_dir: Some(snes9x_bios_dir(emulator_info)?),
            display_name: "Snes9x".to_string(),
            runtime_path_display: snes9x_bios_dir(emulator_info)?.display().to_string(),
            launch_scoped: false,
        }),
        "O2EM" if platform_name == "Philips Videopac+" => Some(FirmwareRuntimeContext {
            runtime_kind: "o2em".to_string(),
            runtime_name: "O2EM".to_string(),
            runtime_dir: Some(o2em_bios_dir(emulator_info)?),
            display_name: "O2EM".to_string(),
            runtime_path_display: o2em_bios_dir(emulator_info)?.display().to_string(),
            launch_scoped: false,
        }),
        "LoopyMSE" if platform_name == "Casio Loopy" => Some(FirmwareRuntimeContext {
            runtime_kind: "loopymse".to_string(),
            runtime_name: "LoopyMSE".to_string(),
            runtime_dir: None,
            display_name: "LoopyMSE".to_string(),
            runtime_path_display: "Passed as <BIOS> [sound BIOS] at launch".to_string(),
            launch_scoped: true,
        }),
        "mGBA" if platform_name == "Nintendo - e-Reader" => Some(FirmwareRuntimeContext {
            runtime_kind: "mgba".to_string(),
            runtime_name: "mGBA".to_string(),
            runtime_dir: None,
            display_name: "mGBA".to_string(),
            runtime_path_display: "Passed as --bios at launch".to_string(),
            launch_scoped: true,
        }),
        "MAME" => Some(FirmwareRuntimeContext {
            runtime_kind: "mame".to_string(),
            runtime_name: "MAME".to_string(),
            runtime_dir: Some(mame_roms_dir(emulator_info)?),
            display_name: "MAME".to_string(),
            runtime_path_display: mame_roms_dir(emulator_info)?.display().to_string(),
            launch_scoped: false,
        }),
        "86Box" if matches!(platform_name, "MS-DOS" | "Windows" | "Windows 3.X") => {
            Some(FirmwareRuntimeContext {
                runtime_kind: "86box".to_string(),
                runtime_name: "86Box".to_string(),
                runtime_dir: Some(eighty_six_box_roms_dir(emulator_info)?),
                display_name: "86Box".to_string(),
                runtime_path_display: eighty_six_box_roms_dir(emulator_info)?
                    .display()
                    .to_string(),
                launch_scoped: false,
            })
        }
        "PCem" if matches!(platform_name, "Windows" | "Windows 3.X") => {
            Some(FirmwareRuntimeContext {
                runtime_kind: "pcem".to_string(),
                runtime_name: "PCem".to_string(),
                runtime_dir: Some(pcem_roms_dir(emulator_info)?),
                display_name: "PCem".to_string(),
                runtime_path_display: pcem_roms_dir(emulator_info)?.display().to_string(),
                launch_scoped: false,
            })
        }
        _ => None,
    }
}

pub async fn ensure_runtime_firmware(
    settings: &AppSettings,
    pool: &SqlitePool,
    minerva_pool: Option<&SqlitePool>,
    emulator_info: &EmulatorInfo,
    platform_name: &str,
    as_retroarch_core: bool,
) -> Result<(), String> {
    let Some(runtime) = resolve_runtime_context(emulator_info, platform_name, as_retroarch_core)
    else {
        return Ok(());
    };

    ensure_firmware_for_runtime(settings, pool, minerva_pool, &runtime, platform_name).await
}

pub async fn ensure_runtime_firmware_for_launch(
    settings: &AppSettings,
    pool: &SqlitePool,
    minerva_pool: Option<&SqlitePool>,
    emulator_info: &EmulatorInfo,
    platform_name: &str,
    as_retroarch_core: bool,
    rom_path: &Path,
) -> Result<(), String> {
    let Some(runtime) = resolve_runtime_context_for_launch(
        emulator_info,
        platform_name,
        as_retroarch_core,
        Some(rom_path),
    ) else {
        return Ok(());
    };

    ensure_firmware_for_runtime(settings, pool, minerva_pool, &runtime, platform_name).await
}

pub fn requires_launch_scoped_firmware_staging(
    emulator_info: &EmulatorInfo,
    platform_name: &str,
    as_retroarch_core: bool,
    rom_path: &Path,
) -> bool {
    resolve_runtime_context_for_launch(
        emulator_info,
        platform_name,
        as_retroarch_core,
        Some(rom_path),
    )
    .is_some_and(|runtime| runtime.launch_scoped && runtime.runtime_dir.is_some())
}

pub async fn get_launch_firmware_args(
    pool: &SqlitePool,
    emulator_info: &EmulatorInfo,
    platform_name: &str,
    as_retroarch_core: bool,
) -> Result<Vec<LaunchArg>, String> {
    let Some(runtime) = resolve_runtime_context(emulator_info, platform_name, as_retroarch_core)
    else {
        return Ok(Vec::new());
    };

    match (runtime.runtime_kind.as_str(), platform_name) {
        ("mgba", "Nintendo - e-Reader") => {
            let package_row = get_imported_package_row(
                pool,
                "minerva:retroarch-system-files",
                "Nintendo - Game Boy Advance (mGBA - VBA-M).zip",
            )
            .await?
            .ok_or_else(|| {
                "Required mGBA firmware package has not been imported yet".to_string()
            })?;

            let bios_path = find_mgba_bios_file(Path::new(&package_row.extracted_root))
                .ok_or_else(|| {
                    "mGBA firmware package does not contain a recognizable GBA BIOS file"
                        .to_string()
                })?;

            Ok(vec![
                LaunchArg::Literal("--bios".to_string()),
                LaunchArg::Path(bios_path.display().to_string()),
            ])
        }
        ("loopymse", "Casio Loopy") => {
            let package_row =
                get_imported_package_row(pool, "minerva:mame-merged-romsets", "casloopy.zip")
                    .await?
                    .ok_or_else(|| {
                        "Required LoopyMSE firmware package has not been imported yet".to_string()
                    })?;

            let extracted_root = Path::new(&package_row.extracted_root);
            let main_bios = find_loopymse_main_bios_file(extracted_root).ok_or_else(|| {
                "LoopyMSE firmware package does not contain the required main BIOS file".to_string()
            })?;

            let mut args = vec![LaunchArg::Path(main_bios.display().to_string())];
            if let Some(sound_bios) = find_loopymse_sound_bios_file(extracted_root) {
                args.push(LaunchArg::Path(sound_bios.display().to_string()));
            }

            Ok(args)
        }
        _ => Ok(Vec::new()),
    }
}

fn rule_supports_hle_fallback(rule_key: &str) -> bool {
    matches!(
        rule_key,
        "retroarch:pcsx_rearmed:Sony Playstation"
            | "retroarch:flycast:Sega Dreamcast"
            | "flycast:Flycast:Sega Dreamcast"
            | "openmsx:openMSX:Microsoft MSX"
            | "openmsx:openMSX:Microsoft MSX2"
            | "openmsx:openMSX:Microsoft MSX2+"
            | "retroarch:pokemini:Nintendo Pokemon Mini"
    )
}

fn rule_target_strategy(runtime: &FirmwareRuntimeContext, rule: &FirmwareRuleRow) -> &'static str {
    if rule.source.starts_with("manual:") {
        "manual_import"
    } else if runtime.launch_scoped {
        "launch_scoped"
    } else if runtime.runtime_kind == "mame" {
        "mame_rompath"
    } else {
        "runtime_dir"
    }
}

pub async fn get_firmware_status(
    _settings: &AppSettings,
    pool: &SqlitePool,
    emulator_info: &EmulatorInfo,
    platform_name: &str,
    as_retroarch_core: bool,
) -> Result<Vec<FirmwareStatus>, String> {
    let Some(runtime) = resolve_runtime_context(emulator_info, platform_name, as_retroarch_core)
    else {
        return Ok(Vec::new());
    };

    let rules: Vec<FirmwareRuleRow> = sqlx::query_as(
        r#"
        SELECT rule_key, source, source_package_name, target_subdir, install_mode, required
        FROM firmware_rules
        WHERE runtime_kind = ?
          AND runtime_name = ?
          AND (platform_name = ? OR platform_name IS NULL)
        ORDER BY platform_name DESC
        "#,
    )
    .bind(&runtime.runtime_kind)
    .bind(&runtime.runtime_name)
    .bind(platform_name)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut statuses = Vec::new();
    for rule in rules {
        let imported = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM firmware_packages WHERE source = ? AND package_name = ?",
        )
        .bind(&rule.source)
        .bind(&rule.source_package_name)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
            > 0;

        let runtime_path = match resolve_rule_target_root(&runtime, &rule) {
            Some(target_root) => target_root.display().to_string(),
            None if rule.target_subdir.is_empty() => runtime.runtime_path_display.clone(),
            None => format!("{}/{}", runtime.runtime_path_display, rule.target_subdir),
        };
        let synced = if let Some(runtime_dir) = runtime.runtime_dir.as_ref() {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM firmware_installs WHERE rule_key = ? AND runtime_install_path = ?",
            )
            .bind(&rule.rule_key)
            .bind(runtime_dir.display().to_string())
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?
                > 0
        } else {
            false
        };

        let supports_hle_fallback = rule_supports_hle_fallback(&rule.rule_key);
        let target_strategy = rule_target_strategy(&runtime, &rule).to_string();

        statuses.push(FirmwareStatus {
            rule_key: rule.rule_key,
            source: rule.source,
            package_name: rule.source_package_name,
            required: rule.required != 0,
            supports_hle_fallback,
            target_strategy,
            imported,
            synced,
            launch_scoped: runtime.launch_scoped,
            runtime_path,
        });
    }

    Ok(statuses)
}

pub async fn open_firmware_directory(
    settings: &AppSettings,
    pool: &SqlitePool,
    emulator_info: &EmulatorInfo,
    platform_name: &str,
    as_retroarch_core: bool,
) -> Result<String, String> {
    let Some(runtime) = resolve_runtime_context(emulator_info, platform_name, as_retroarch_core)
    else {
        return Err(format!(
            "{} does not expose a firmware directory for {}",
            emulator_info.name, platform_name
        ));
    };

    let rules: Vec<FirmwareRuleRow> = sqlx::query_as(
        r#"
        SELECT rule_key, source, source_package_name, target_subdir, install_mode, required
        FROM firmware_rules
        WHERE runtime_kind = ?
          AND runtime_name = ?
          AND (platform_name = ? OR platform_name IS NULL)
        ORDER BY platform_name DESC
        "#,
    )
    .bind(&runtime.runtime_kind)
    .bind(&runtime.runtime_name)
    .bind(platform_name)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let open_path = if rules.iter().any(|rule| rule.source.starts_with("manual:")) {
        let manual_dir = manual_firmware_drop_directory(settings, &runtime, platform_name);
        std::fs::create_dir_all(&manual_dir)
            .map_err(|e| format!("Failed to create {}: {}", manual_dir.display(), e))?;
        write_manual_firmware_readme(
            &manual_dir.join("README.txt"),
            &runtime,
            platform_name,
            &rules,
        )?;
        manual_dir
    } else if let Some(runtime_dir) = runtime.runtime_dir.as_ref() {
        std::fs::create_dir_all(runtime_dir)
            .map_err(|e| format!("Failed to create {}: {}", runtime_dir.display(), e))?;
        runtime_dir.clone()
    } else {
        return Err(format!(
            "{} on {} does not expose a stable firmware directory",
            runtime.display_name, platform_name
        ));
    };

    open_path_in_file_manager(&open_path)?;
    Ok(open_path.display().to_string())
}

pub async fn import_firmware_package(
    settings: &AppSettings,
    pool: &SqlitePool,
    archive_path: &Path,
    source: Option<&str>,
) -> Result<ImportedFirmwarePackage, String> {
    let package_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid firmware package path {}", archive_path.display()))?
        .to_string();
    let source = source.unwrap_or("user-import");
    let package_key = sanitize_package_key(&package_name);
    let package_root = settings
        .get_firmware_packages_directory()
        .join(source)
        .join(&package_key);
    let extracted_root = package_root.join("extracted");
    let archive_copy_path = package_root.join(&package_name);

    std::fs::create_dir_all(&package_root)
        .map_err(|e| format!("Failed to create {}: {}", package_root.display(), e))?;

    if archive_path != archive_copy_path {
        std::fs::copy(archive_path, &archive_copy_path).map_err(|e| {
            format!(
                "Failed to copy {} to {}: {}",
                archive_path.display(),
                archive_copy_path.display(),
                e
            )
        })?;
    }

    if extracted_root.exists() {
        std::fs::remove_dir_all(&extracted_root).map_err(|e| {
            format!(
                "Failed to reset extracted firmware directory {}: {}",
                extracted_root.display(),
                e
            )
        })?;
    }
    std::fs::create_dir_all(&extracted_root)
        .map_err(|e| format!("Failed to create {}: {}", extracted_root.display(), e))?;

    extract_zip_archive(&archive_copy_path, &extracted_root)?;
    let archive_sha256 = sha256_file(&archive_copy_path)?;

    sqlx::query(
        r#"
        INSERT INTO firmware_packages (source, package_key, package_name, archive_path, extracted_root, sha256)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(source, package_key) DO UPDATE SET
            package_name = excluded.package_name,
            archive_path = excluded.archive_path,
            extracted_root = excluded.extracted_root,
            sha256 = excluded.sha256,
            imported_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(source)
    .bind(&package_key)
    .bind(&package_name)
    .bind(archive_copy_path.display().to_string())
    .bind(extracted_root.display().to_string())
    .bind(archive_sha256)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    let package_id: i64 =
        sqlx::query_scalar("SELECT id FROM firmware_packages WHERE source = ? AND package_key = ?")
            .bind(source)
            .bind(&package_key)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM firmware_files WHERE package_id = ?")
        .bind(package_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut file_count = 0usize;
    for entry in walkdir::WalkDir::new(&extracted_root) {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let relative = path
            .strip_prefix(&extracted_root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to stat {}: {}", path.display(), e))?;
        let sha256 = sha256_file(&path)?;

        sqlx::query(
            r#"
            INSERT INTO firmware_files (package_id, relative_path, store_path, sha256, file_size)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(package_id)
        .bind(relative)
        .bind(path.display().to_string())
        .bind(sha256)
        .bind(metadata.len() as i64)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
        file_count += 1;
    }

    Ok(ImportedFirmwarePackage {
        source: source.to_string(),
        package_name,
        file_count,
        extracted_root: extracted_root.display().to_string(),
    })
}

async fn ensure_firmware_for_runtime(
    settings: &AppSettings,
    pool: &SqlitePool,
    minerva_pool: Option<&SqlitePool>,
    runtime: &FirmwareRuntimeContext,
    platform_name: &str,
) -> Result<(), String> {
    let system_dir = runtime.runtime_dir.clone();

    let rules: Vec<FirmwareRuleRow> = sqlx::query_as(
        r#"
        SELECT rule_key, source, source_package_name, target_subdir, install_mode, required
        FROM firmware_rules
        WHERE runtime_kind = ?
          AND runtime_name = ?
          AND (platform_name = ? OR platform_name IS NULL)
        ORDER BY platform_name DESC
        "#,
    )
    .bind(&runtime.runtime_kind)
    .bind(&runtime.runtime_name)
    .bind(platform_name)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    if rules.is_empty() {
        return Ok(());
    }

    if let Some(system_dir) = system_dir.as_ref() {
        std::fs::create_dir_all(system_dir)
            .map_err(|e| format!("Failed to create {}: {}", system_dir.display(), e))?;
    }

    let mut missing_packages = Vec::new();
    for rule in rules {
        if !matches!(rule.install_mode.as_str(), "merge_tree" | "copy_archive") {
            let message = format!(
                "{} [{}] uses unsupported install mode '{}'",
                rule.source_package_name, rule.source, rule.install_mode
            );
            tracing::warn!(
                rule_key = %rule.rule_key,
                install_mode = %rule.install_mode,
                "Unsupported firmware install mode"
            );
            if rule.required != 0 {
                missing_packages.push(message);
            }
            continue;
        }

        let mut package_row =
            get_imported_package_row(pool, &rule.source, &rule.source_package_name).await?;

        if package_row.is_none() {
            if let Err(e) = ensure_source_package_imported(
                settings,
                pool,
                minerva_pool,
                &rule.source,
                &rule.source_package_name,
            )
            .await
            {
                if rule.required != 0 {
                    missing_packages.push(format!(
                        "{} [{}]: {}",
                        rule.source_package_name, rule.source, e
                    ));
                }
                continue;
            }
            package_row =
                get_imported_package_row(pool, &rule.source, &rule.source_package_name).await?;
        }

        let Some(package_row) = package_row else {
            let message = format!("{} [{}]", rule.source_package_name, rule.source);
            tracing::info!(rule_key = %rule.rule_key, package = %rule.source_package_name, "Required firmware package has not been imported yet");
            if rule.required != 0 {
                missing_packages.push(message);
            }
            continue;
        };

        let Some(system_dir) = system_dir.as_ref() else {
            continue;
        };

        let target_root = resolve_rule_target_root(runtime, &rule).ok_or_else(|| {
            format!(
                "Could not resolve target path for firmware rule '{}' on {}",
                rule.rule_key, runtime.display_name
            )
        })?;
        let expected_target_path = match rule.install_mode.as_str() {
            "merge_tree" => target_root.clone(),
            "copy_archive" => target_root.join(&rule.source_package_name),
            _ => unreachable!(),
        };
        if expected_target_path.exists()
            && firmware_install_is_synced(pool, &rule.rule_key, system_dir, &expected_target_path)
                .await?
        {
            continue;
        }

        let installed_target_path = match rule.install_mode.as_str() {
            "merge_tree" => {
                let source_root =
                    package_sync_root(Path::new(&package_row.extracted_root), &rule.source)?;
                if runtime.runtime_kind == "openmsx" {
                    copy_openmsx_system_roms(&source_root, &target_root)?;
                } else {
                    copy_tree(&source_root, &target_root)?;
                }
                target_root.clone()
            }
            "copy_archive" => {
                std::fs::create_dir_all(&target_root)
                    .map_err(|e| format!("Failed to create {}: {}", target_root.display(), e))?;
                let source_archive = PathBuf::from(&package_row.archive_path);
                let target_archive = target_root.join(&rule.source_package_name);
                std::fs::copy(&source_archive, &target_archive).map_err(|e| {
                    format!(
                        "Failed to copy {} to {}: {}",
                        source_archive.display(),
                        target_archive.display(),
                        e
                    )
                })?;
                target_archive
            }
            _ => unreachable!(),
        };

        sqlx::query(
            r#"
            INSERT INTO firmware_installs (rule_key, runtime_install_path, package_id, target_path, status)
            VALUES (?, ?, ?, ?, 'synced')
            ON CONFLICT(rule_key, runtime_install_path, target_path) DO UPDATE SET
                package_id = excluded.package_id,
                status = excluded.status,
                synced_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&rule.rule_key)
        .bind(system_dir.display().to_string())
        .bind(package_row.id)
        .bind(installed_target_path.display().to_string())
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    if !missing_packages.is_empty() {
        return Err(format!(
            "Missing required firmware for {} on {}. Import package(s): {}. Expected runtime path: {}",
            runtime.display_name,
            platform_name,
            missing_packages.join(", "),
            runtime.runtime_path_display,
        ));
    }

    let _ = settings;
    Ok(())
}

async fn firmware_install_is_synced(
    pool: &SqlitePool,
    rule_key: &str,
    runtime_install_path: &Path,
    target_path: &Path,
) -> Result<bool, String> {
    Ok(sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM firmware_installs
        WHERE rule_key = ?
          AND runtime_install_path = ?
          AND target_path = ?
          AND status = 'synced'
        "#,
    )
    .bind(rule_key)
    .bind(runtime_install_path.display().to_string())
    .bind(target_path.display().to_string())
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?
        > 0)
}

async fn get_imported_package_row(
    pool: &SqlitePool,
    source: &str,
    package_name: &str,
) -> Result<Option<ImportedPackageRow>, String> {
    sqlx::query_as(
        "SELECT id, archive_path, extracted_root FROM firmware_packages WHERE source = ? AND package_name = ? LIMIT 1",
    )
    .bind(source)
    .bind(package_name)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())
}

async fn ensure_source_package_imported(
    settings: &AppSettings,
    pool: &SqlitePool,
    minerva_pool: Option<&SqlitePool>,
    source: &str,
    package_name: &str,
) -> Result<(), String> {
    match source {
        "minerva:retroarch-system-files" => {
            ensure_minerva_source_package_imported(
                settings,
                pool,
                minerva_pool,
                source,
                package_name,
                RETROARCH_SYSTEM_FILES_TORRENT_FILE,
                RETROARCH_SYSTEM_FILES_PATH_PREFIX,
                "retroarch-system-files",
            )
            .await
        }
        "minerva:flycast-bios-files" => {
            ensure_minerva_source_package_imported(
                settings,
                pool,
                minerva_pool,
                source,
                package_name,
                FLYCAST_BIOS_TORRENT_FILE,
                FLYCAST_BIOS_PATH_PREFIX,
                "flycast-bios-files",
            )
            .await
        }
        "minerva:mame-merged-romsets" => {
            ensure_minerva_source_package_imported(
                settings,
                pool,
                minerva_pool,
                source,
                package_name,
                RETROARCH_SYSTEM_FILES_TORRENT_FILE,
                CHADMASTER_MAME_MERGED_PATH_PREFIX,
                "mame-merged-romsets",
            )
            .await
        }
        "minerva:mame-non-merged-romsets" => {
            ensure_minerva_source_package_imported(
                settings,
                pool,
                minerva_pool,
                source,
                package_name,
                MAME_NONMERGED_TORRENT_FILE,
                MAME_NONMERGED_PATH_PREFIX,
                "mame-non-merged-romsets",
            )
            .await
        }
        "github:mame-hash-files" => {
            ensure_mame_hash_file_imported(settings, pool, package_name).await
        }
        "github:86box-romset" => ensure_86box_romset_imported(settings, pool).await,
        "github:pcem-romset" => ensure_pcem_romset_imported(settings, pool).await,
        "manual:m88kai-bios" => Err(
            "M88kai still expects manually arranged PC-8801 BIOS ROMs in its emulator directory. Lunchbox exposes a managed import folder, but does not auto-install this standalone layout yet.".to_string(),
        ),
        "manual:tsugaru-fmtowns" => Err(
            "Tsugaru still expects FM Towns ROM assets plus CMOS.BIN to be configured manually. Lunchbox exposes a managed import folder, but does not auto-install this standalone layout yet.".to_string(),
        ),
        "manual:unz-fmtowns" => Err(
            "UNZ still expects FM Towns ROM assets plus cmos.dat to be arranged in the emulator directory manually. Lunchbox exposes a managed import folder, but does not auto-install this standalone layout yet.".to_string(),
        ),
        "manual:emu5-sordm5" => Err(
            "The standalone Emu5 Sord M5 path still expects BIOS ROMs to be arranged manually in the emulator directory. Lunchbox exposes a managed import folder, but does not auto-install this standalone layout yet.".to_string(),
        ),
        "manual:demul-awbios" | "manual:demul-hikaru" | "manual:demul-naomi" | "manual:demul-naomi2" => Err(
            "DEmul still expects BIOS romsets to be configured manually through its Roms and Bioses Paths settings. Lunchbox exposes a managed import folder, but does not auto-install this standalone layout yet.".to_string(),
        ),
        "manual:geepee32-firmware" => Err(
            "GeePee32 requires a manually dumped GP32 firmware file such as fw100k.bin or fw157e.bin. Configure it in GeePee32 or geepee32.ini; Lunchbox does not have an auto-download source for it.".to_string(),
        ),
        "manual:melonds-dsi-system" => Err(
            "melonDS DSi requires manually dumped DSi BIOS, firmware, and NAND files. Configure them in melonDS Emu Settings; Lunchbox does not have an auto-download source for them yet.".to_string(),
        ),
        _ => Err(format!("unsupported firmware source '{source}'")),
    }
}

async fn ensure_minerva_source_package_imported(
    settings: &AppSettings,
    pool: &SqlitePool,
    minerva_pool: Option<&SqlitePool>,
    source: &str,
    package_name: &str,
    torrent_file: &str,
    path_prefix: &str,
    download_subdir: &str,
) -> Result<(), String> {
    let minerva_pool = minerva_pool.ok_or_else(|| {
        "Minerva database is not available, so the required firmware package cannot be located"
            .to_string()
    })?;

    let torrent_url: String = sqlx::query_scalar(
        "SELECT torrent_url FROM minerva_torrents WHERE torrent_file = ? LIMIT 1",
    )
    .bind(torrent_file)
    .fetch_optional(minerva_pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| {
        format!(
            "Minerva firmware torrent '{}' is not present in minerva.db",
            torrent_file
        )
    })?;

    let files = crate::torrent::get_torrent_file_listing(&torrent_url)
        .await
        .map_err(|e| format!("failed to fetch Minerva firmware torrent metadata: {e}"))?;
    let file_index =
        find_minerva_source_file_index(&files, path_prefix, package_name).ok_or_else(|| {
            format!(
                "Minerva firmware torrent does not contain '{}'",
                package_name
            )
        })?;
    let expected_size = files
        .iter()
        .find(|file| file.index == file_index)
        .map(|file| file.size as u64)
        .unwrap_or(0);

    let download_dir = settings
        .get_torrent_library_directory()
        .join("_firmware")
        .join(download_subdir);
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create {}: {}", download_dir.display(), e))?;
    if let Some(existing_archive) =
        find_existing_minerva_download(&download_dir, path_prefix, package_name)
    {
        if zip_archive_is_fully_readable(&existing_archive, expected_size) {
            import_firmware_package(settings, pool, &existing_archive, Some(source))
                .await
                .map(|_| ())?;
            return Ok(());
        }
    }

    let client = crate::torrent::create_client(settings)
        .map_err(|e| format!("failed to configure qBittorrent for firmware download: {e}"))?;
    let client_job_id = client
        .add_torrent(&torrent_url, &download_dir, Some(vec![file_index]))
        .await
        .map_err(|e| format!("failed to start Minerva firmware download: {e}"))?;

    let started = Instant::now();
    let timeout = Duration::from_secs(3600);
    let mut last_archive_check = Instant::now() - Duration::from_secs(30);
    loop {
        if started.elapsed() > timeout {
            return Err(format!(
                "timed out downloading firmware package '{}' from Minerva",
                package_name
            ));
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
        if last_archive_check.elapsed() >= Duration::from_secs(10) {
            last_archive_check = Instant::now();
            if let Some(archive_path) = client
                .get_downloaded_file_path(&client_job_id, file_index, &download_dir)
                .await
                .map_err(|e| format!("failed to locate downloaded firmware package: {e}"))?
            {
                if zip_archive_is_fully_readable(&archive_path, expected_size) {
                    import_firmware_package(settings, pool, &archive_path, Some(source))
                        .await
                        .map(|_| ())?;
                    return Ok(());
                }
            }
        }

        let Some(progress) = client
            .get_progress(&client_job_id)
            .await
            .map_err(|e| format!("failed to read firmware download progress: {e}"))?
        else {
            continue;
        };

        match progress.status {
            crate::torrent::DownloadStatus::Completed => break,
            crate::torrent::DownloadStatus::Failed => {
                return Err(progress.status_message);
            }
            crate::torrent::DownloadStatus::Cancelled => {
                return Err("firmware download was cancelled".to_string());
            }
            _ => {}
        }
    }

    let archive_path = client
        .get_downloaded_file_path(&client_job_id, file_index, &download_dir)
        .await
        .map_err(|e| format!("failed to locate downloaded firmware package: {e}"))?
        .ok_or_else(|| {
            format!(
                "firmware package '{}' finished downloading but could not be found on disk",
                package_name
            )
        })?;

    import_firmware_package(settings, pool, &archive_path, Some(source))
        .await
        .map(|_| ())
}

fn normalize_torrent_path(path: &str) -> String {
    path.trim_start_matches("./")
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn find_minerva_source_file_index(
    files: &[crate::torrent::TorrentFileInfo],
    prefix: &str,
    package_name: &str,
) -> Option<usize> {
    let expected_suffix = normalize_torrent_path(&format!("{prefix}{package_name}"));
    let expected_name = package_name.to_ascii_lowercase();

    files
        .iter()
        .find(|file| {
            let normalized = normalize_torrent_path(&file.filename);
            normalized == expected_suffix
                || normalized.ends_with(&format!("/{expected_suffix}"))
                || (normalized.ends_with(&format!("/{expected_name}"))
                    && normalized.contains("/retroarchsystemfiles/retroarch-system/"))
        })
        .map(|file| file.index)
}

fn sanitize_package_key(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn manual_firmware_drop_directory(
    settings: &AppSettings,
    runtime: &FirmwareRuntimeContext,
    platform_name: &str,
) -> PathBuf {
    settings
        .get_manual_firmware_directory()
        .join(sanitize_package_key(&runtime.runtime_kind))
        .join(sanitize_package_key(&runtime.runtime_name))
        .join(sanitize_package_key(platform_name))
}

fn builtin_rule_notes(rule_key: &str) -> Option<&'static str> {
    BUILTIN_FIRMWARE_RULES
        .iter()
        .find(|rule| rule.rule_key == rule_key)
        .map(|rule| rule.notes)
}

fn write_manual_firmware_readme(
    readme_path: &Path,
    runtime: &FirmwareRuntimeContext,
    platform_name: &str,
    rules: &[FirmwareRuleRow],
) -> Result<(), String> {
    let package_lines = rules
        .iter()
        .map(|rule| format!("- {}", rule.source_package_name))
        .collect::<Vec<_>>()
        .join("\n");
    let note_lines = rules
        .iter()
        .filter_map(|rule| builtin_rule_notes(&rule.rule_key))
        .map(|note| format!("- {}", note))
        .collect::<Vec<_>>();

    let mut body = format!(
        "Drop manually acquired firmware here for {display_name} on {platform}.\n\nRuntime: {display_name}\nPlatform: {platform}\n\nExpected files or package contents:\n{packages}\n",
        display_name = runtime.display_name,
        platform = platform_name,
        packages = package_lines,
    );

    if !note_lines.is_empty() {
        body.push_str("\nNotes:\n");
        body.push_str(&note_lines.join("\n"));
        body.push('\n');
    }

    body.push_str(
        "\nLunchbox opens this folder as a managed drop location for manual-only firmware cases. If the emulator still expects paths to be configured manually, point it at the files you place here.\n",
    );

    std::fs::write(readme_path, body)
        .map_err(|e| format!("Failed to write {}: {}", readme_path.display(), e))
}

fn open_path_in_file_manager(path: &Path) -> Result<(), String> {
    let mut command = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("explorer");
        cmd.arg(path);
        cmd
    } else if cfg!(target_os = "macos") {
        let mut cmd = Command::new("open");
        cmd.arg(path);
        cmd
    } else {
        let mut cmd = Command::new("xdg-open");
        cmd.arg(path);
        cmd
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))
}

fn find_mgba_bios_file(extracted_root: &Path) -> Option<PathBuf> {
    let mut fallback = None;

    for entry in walkdir::WalkDir::new(extracted_root) {
        let entry = entry.ok()?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if lower == "gba_bios.bin" {
            return Some(path.to_path_buf());
        }
        if lower.ends_with(".bin") && lower.contains("bios") {
            fallback = Some(path.to_path_buf());
        }
    }

    fallback
}

fn find_loopymse_main_bios_file(extracted_root: &Path) -> Option<PathBuf> {
    let mut fallback = None;

    for entry in walkdir::WalkDir::new(extracted_root) {
        let entry = entry.ok()?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if lower == "hd6437021.lsi302" {
            return Some(path.to_path_buf());
        }
        if fallback.is_none()
            && (lower.contains("lsi302") || (lower.contains("bios") && !lower.contains("printer")))
        {
            fallback = Some(path.to_path_buf());
        }
    }

    fallback
}

fn find_loopymse_sound_bios_file(extracted_root: &Path) -> Option<PathBuf> {
    let mut fallback = None;

    for entry in walkdir::WalkDir::new(extracted_root) {
        let entry = entry.ok()?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if lower == "hn62434fa.lsi352" {
            return Some(path.to_path_buf());
        }
        if fallback.is_none()
            && (lower.contains("lsi352") || lower.contains("printer") || lower.contains("sound"))
        {
            fallback = Some(path.to_path_buf());
        }
    }

    fallback
}

fn find_existing_minerva_download(
    download_dir: &Path,
    prefix: &str,
    package_name: &str,
) -> Option<PathBuf> {
    let direct = download_dir
        .join("Minerva_Myrient")
        .join(prefix)
        .join(package_name);
    direct.exists().then_some(direct)
}

fn resolve_rule_target_root(
    runtime: &FirmwareRuntimeContext,
    rule: &FirmwareRuleRow,
) -> Option<PathBuf> {
    let runtime_dir = runtime.runtime_dir.as_ref()?;

    if rule.target_subdir == "@hash" {
        return match runtime.runtime_kind.as_str() {
            "mame" => runtime_dir.parent().map(|parent| parent.join("hash")),
            _ => None,
        };
    }

    if rule.target_subdir.is_empty() {
        Some(runtime_dir.clone())
    } else {
        Some(runtime_dir.join(&rule.target_subdir))
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let read = file
            .read(&mut buf)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn extract_zip_archive(archive_path: &Path, dest_root: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open {}: {}", archive_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read {}: {}", archive_path.display(), e))?;

    for idx in 0..archive.len() {
        let mut entry = archive
            .by_index(idx)
            .map_err(|e| format!("Failed to read {}: {}", archive_path.display(), e))?;
        let Some(enclosed) = entry.enclosed_name().map(PathBuf::from) else {
            continue;
        };
        let output_path = dest_root.join(enclosed);
        if entry.is_dir() {
            std::fs::create_dir_all(&output_path)
                .map_err(|e| format!("Failed to create {}: {}", output_path.display(), e))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }
        let mut output = File::create(&output_path)
            .map_err(|e| format!("Failed to create {}: {}", output_path.display(), e))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|e| format!("Failed to extract {}: {}", output_path.display(), e))?;
        output
            .flush()
            .map_err(|e| format!("Failed to flush {}: {}", output_path.display(), e))?;
    }

    Ok(())
}

fn zip_archive_is_fully_readable(archive_path: &Path, expected_size: u64) -> bool {
    if expected_size > 0 {
        let Ok(metadata) = std::fs::metadata(archive_path) else {
            return false;
        };
        if metadata.len() < expected_size {
            return false;
        }
    }

    let Ok(file) = File::open(archive_path) else {
        return false;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return false;
    };

    for idx in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(idx) else {
            return false;
        };
        if entry.is_dir() {
            continue;
        }
        if std::io::copy(&mut entry, &mut std::io::sink()).is_err() {
            return false;
        }
    }

    true
}

fn create_single_entry_zip(
    archive_path: &Path,
    entry_name: &str,
    contents: &[u8],
) -> Result<(), String> {
    let file = File::create(archive_path)
        .map_err(|e| format!("Failed to create {}: {}", archive_path.display(), e))?;
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file::<_, ()>(entry_name, zip::write::FileOptions::default())
        .map_err(|e| {
            format!(
                "Failed to start {} in {}: {}",
                entry_name,
                archive_path.display(),
                e
            )
        })?;
    writer.write_all(contents).map_err(|e| {
        format!(
            "Failed to write {} in {}: {}",
            entry_name,
            archive_path.display(),
            e
        )
    })?;
    writer
        .finish()
        .map_err(|e| format!("Failed to finish {}: {}", archive_path.display(), e))?;
    Ok(())
}

fn copy_tree(source_root: &Path, dest_root: &Path) -> Result<(), String> {
    for entry in walkdir::WalkDir::new(source_root) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let relative = path.strip_prefix(source_root).map_err(|e| e.to_string())?;
        let target = dest_root.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)
                .map_err(|e| format!("Failed to create {}: {}", target.display(), e))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }
        std::fs::copy(path, &target).map_err(|e| {
            format!(
                "Failed to copy {} to {}: {}",
                path.display(),
                target.display(),
                e
            )
        })?;
    }
    Ok(())
}

fn package_sync_root(source_root: &Path, source: &str) -> Result<PathBuf, String> {
    if source != "github:pcem-romset" {
        return Ok(source_root.to_path_buf());
    }

    let mut children = std::fs::read_dir(source_root)
        .map_err(|e| format!("Failed to read {}: {}", source_root.display(), e))?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();

    if children.len() == 1 {
        let child = children.remove(0);
        let child_path = child.path();
        if child_path.is_dir() {
            return Ok(child_path);
        }
    }

    Ok(source_root.to_path_buf())
}

fn copy_openmsx_system_roms(source_root: &Path, dest_root: &Path) -> Result<(), String> {
    for entry in walkdir::WalkDir::new(source_root) {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        if matches!(extension.as_deref(), Some("ini" | "xml" | "txt")) {
            continue;
        }

        let relative = path.strip_prefix(source_root).map_err(|e| e.to_string())?;
        let target = dest_root.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }
        std::fs::copy(path, &target).map_err(|e| {
            format!(
                "Failed to copy {} to {}: {}",
                path.display(),
                target.display(),
                e
            )
        })?;
    }

    Ok(())
}

fn retroarch_system_dir() -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_retroarch_installed() {
                Some(home.join(".var/app/org.libretro.RetroArch/config/retroarch/system"))
            } else {
                Some(home.join(".config/retroarch/system"))
            }
        }
        "Windows" => dirs::data_local_dir().map(|dir| dir.join("RetroArch").join("system")),
        "macOS" => {
            dirs::home_dir().map(|home| home.join("Library/Application Support/RetroArch/system"))
        }
        _ => None,
    }
}

fn duckstation_bios_dir(emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_runtime_installed(emulator_info) {
                Some(home.join(".var/app/org.duckstation.DuckStation/data/duckstation/bios"))
            } else {
                dirs::data_local_dir().map(|dir| dir.join("duckstation").join("bios"))
            }
        }
        "Windows" => dirs::data_local_dir().map(|dir| dir.join("duckstation").join("bios")),
        "macOS" => {
            dirs::home_dir().map(|home| home.join("Library/Application Support/DuckStation/bios"))
        }
        _ => None,
    }
}

fn pcsx2_bios_dir(emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_runtime_installed(emulator_info) {
                Some(home.join(".var/app/net.pcsx2.PCSX2/config/PCSX2/bios"))
            } else {
                dirs::config_dir().map(|dir| dir.join("PCSX2").join("bios"))
            }
        }
        "Windows" => dirs::config_dir().map(|dir| dir.join("PCSX2").join("bios")),
        "macOS" => dirs::home_dir().map(|home| home.join("Library/Application Support/PCSX2/bios")),
        _ => None,
    }
}

fn melonds_firmware_dir(emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_runtime_installed(emulator_info) {
                Some(home.join(".var/app/net.kuribo64.melonDS/config/melonDS"))
            } else {
                dirs::config_dir().map(|dir| dir.join("melonDS"))
            }
        }
        "Windows" => dirs::config_dir().map(|dir| dir.join("melonDS")),
        "macOS" => dirs::home_dir().map(|home| home.join("Library/Application Support/melonDS")),
        _ => None,
    }
}

fn dolphin_user_dir(emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_runtime_installed(emulator_info) {
                Some(home.join(".var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu"))
            } else {
                let xdg_path = home.join(".local/share/dolphin-emu");
                if xdg_path.exists() {
                    Some(xdg_path)
                } else {
                    Some(home.join(".dolphin-emu"))
                }
            }
        }
        "Windows" => dirs::document_dir().map(|dir| dir.join("Dolphin Emulator")),
        "macOS" => dirs::home_dir().map(|home| home.join("Library/Application Support/Dolphin")),
        _ => None,
    }
}

fn flycast_runtime_dir(emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_runtime_installed(emulator_info) {
                Some(home.join(".var/app/org.flycast.Flycast/data/flycast"))
            } else {
                dirs::data_local_dir().map(|dir| dir.join("flycast"))
            }
        }
        "Windows" => dirs::data_local_dir().map(|dir| dir.join("Flycast")),
        "macOS" => dirs::home_dir().map(|home| home.join("Library/Application Support/flycast")),
        _ => None,
    }
}

fn openmsx_systemroms_dir(_emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => dirs::home_dir().map(|home| home.join(".openMSX/share/systemroms")),
        "Windows" => dirs::document_dir().map(|dir| dir.join("openMSX/share/systemroms")),
        "macOS" => dirs::home_dir().map(|home| home.join("Library/openMSX/share/systemroms")),
        _ => None,
    }
}

fn pcem_roms_dir(_emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => dirs::home_dir().map(|home| home.join(".pcem/roms")),
        "Windows" => dirs::home_dir().map(|home| home.join(".pcem/roms")),
        "macOS" => dirs::home_dir().map(|home| home.join(".pcem/roms")),
        _ => None,
    }
}

fn snes9x_bios_dir(emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_runtime_installed(emulator_info) {
                Some(home.join(".var/app/com.snes9x.Snes9x/config/snes9x/Bios"))
            } else {
                Some(home.join(".snes9x/Bios"))
            }
        }
        "Windows" => dirs::home_dir().map(|home| home.join(".snes9x/Bios")),
        "macOS" => dirs::home_dir().map(|home| home.join(".snes9x/Bios")),
        _ => None,
    }
}

fn o2em_bios_dir(_emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => dirs::home_dir().map(|home| home.join(".o2em")),
        "Windows" => dirs::home_dir().map(|home| home.join(".o2em")),
        _ => None,
    }
}

fn mame_roms_dir(emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_runtime_installed(emulator_info) {
                Some(home.join(".var/app/org.mamedev.MAME/data/mame/roms"))
            } else {
                Some(home.join(".mame/roms"))
            }
        }
        "Windows" => dirs::data_local_dir().map(|dir| dir.join("mame").join("roms")),
        "macOS" => dirs::home_dir().map(|home| home.join(".mame/roms")),
        _ => None,
    }
}

fn eighty_six_box_roms_dir(emulator_info: &EmulatorInfo) -> Option<PathBuf> {
    match emulator::current_os() {
        "Linux" => {
            let home = dirs::home_dir()?;
            if is_flatpak_runtime_installed(emulator_info) {
                Some(home.join(".var/app/net._86box._86Box/data/86Box/roms"))
            } else {
                dirs::data_local_dir().map(|dir| dir.join("86Box").join("roms"))
            }
        }
        "Windows" => dirs::data_local_dir().map(|dir| dir.join("86Box").join("roms")),
        "macOS" => dirs::home_dir().map(|home| home.join("Library/Application Support/86Box/roms")),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseResponse {
    assets: Vec<GitHubReleaseAsset>,
}

async fn ensure_86box_romset_imported(
    settings: &AppSettings,
    pool: &SqlitePool,
) -> Result<(), String> {
    let package_row = get_imported_package_row(
        pool,
        "github:86box-romset",
        EIGHTY_SIX_BOX_ROMSET_PACKAGE_NAME,
    )
    .await?;
    if package_row.is_some() {
        return Ok(());
    }

    let download_dir = settings
        .get_torrent_library_directory()
        .join("_firmware")
        .join("86box-romset");
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create {}: {}", download_dir.display(), e))?;

    let archive_path = download_dir.join(EIGHTY_SIX_BOX_ROMSET_PACKAGE_NAME);
    if zip_archive_is_fully_readable(&archive_path, 0) {
        import_firmware_package(settings, pool, &archive_path, Some("github:86box-romset"))
            .await
            .map(|_| ())?;
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .user_agent("lunchbox/firmware")
        .build()
        .map_err(|e| format!("Failed to initialize firmware downloader: {}", e))?;
    let release: GitHubReleaseResponse = client
        .get(EIGHTY_SIX_BOX_RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Failed to query 86Box ROM releases: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Failed to query 86Box ROM releases: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse 86Box ROM release metadata: {}", e))?;

    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.to_ascii_lowercase().ends_with(".zip"))
        .ok_or_else(|| "86Box latest release did not expose a ROM set ZIP asset".to_string())?;

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download 86Box ROM set: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Failed to download 86Box ROM set: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read 86Box ROM set download: {}", e))?;

    std::fs::write(&archive_path, &bytes)
        .map_err(|e| format!("Failed to write {}: {}", archive_path.display(), e))?;

    if !zip_archive_is_fully_readable(&archive_path, bytes.len() as u64) {
        return Err(format!(
            "Downloaded 86Box ROM set {} is not a fully readable ZIP archive",
            archive_path.display()
        ));
    }

    import_firmware_package(settings, pool, &archive_path, Some("github:86box-romset"))
        .await
        .map(|_| ())
}

async fn ensure_mame_hash_file_imported(
    settings: &AppSettings,
    pool: &SqlitePool,
    package_name: &str,
) -> Result<(), String> {
    let package_row =
        get_imported_package_row(pool, "github:mame-hash-files", package_name).await?;
    if package_row.is_some() {
        return Ok(());
    }

    match package_name {
        MAME_HASH_ARCADIA_PACKAGE_NAME => {}
        _ => return Err(format!("unsupported MAME hash file '{package_name}'")),
    }

    let download_dir = settings
        .get_torrent_library_directory()
        .join("_firmware")
        .join("mame-hash-files");
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create {}: {}", download_dir.display(), e))?;

    let archive_path = download_dir.join(package_name);
    if zip_archive_is_fully_readable(&archive_path, 0) {
        import_firmware_package(
            settings,
            pool,
            &archive_path,
            Some("github:mame-hash-files"),
        )
        .await
        .map(|_| ())?;
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .user_agent("lunchbox/firmware")
        .build()
        .map_err(|e| format!("Failed to initialize firmware downloader: {}", e))?;
    let contents = client
        .get(MAME_HASH_ARCADIA_RAW_URL)
        .send()
        .await
        .map_err(|e| {
            format!(
                "Failed to download MAME hash file '{}': {}",
                package_name, e
            )
        })?
        .error_for_status()
        .map_err(|e| {
            format!(
                "Failed to download MAME hash file '{}': {}",
                package_name, e
            )
        })?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read MAME hash file '{}': {}", package_name, e))?;

    create_single_entry_zip(&archive_path, package_name, &contents)?;

    if !zip_archive_is_fully_readable(&archive_path, 0) {
        return Err(format!(
            "Downloaded MAME hash package {} is not a fully readable ZIP archive",
            archive_path.display()
        ));
    }

    import_firmware_package(
        settings,
        pool,
        &archive_path,
        Some("github:mame-hash-files"),
    )
    .await
    .map(|_| ())
}

async fn ensure_pcem_romset_imported(
    settings: &AppSettings,
    pool: &SqlitePool,
) -> Result<(), String> {
    let package_row =
        get_imported_package_row(pool, "github:pcem-romset", PCEM_ROMSET_PACKAGE_NAME).await?;
    if package_row.is_some() {
        return Ok(());
    }

    let download_dir = settings
        .get_torrent_library_directory()
        .join("_firmware")
        .join("pcem-romset");
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create {}: {}", download_dir.display(), e))?;

    let archive_path = download_dir.join(PCEM_ROMSET_PACKAGE_NAME);
    if zip_archive_is_fully_readable(&archive_path, 0) {
        import_firmware_package(settings, pool, &archive_path, Some("github:pcem-romset"))
            .await
            .map(|_| ())?;
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .user_agent("lunchbox/firmware")
        .build()
        .map_err(|e| format!("Failed to initialize firmware downloader: {}", e))?;
    let release: GitHubReleaseResponse = client
        .get(PCEM_RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Failed to query PCem ROM releases: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Failed to query PCem ROM releases: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse PCem ROM release metadata: {}", e))?;

    let asset = release
        .assets
        .into_iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".zip") && name.contains("rom")
        })
        .ok_or_else(|| "PCem latest release did not expose a ROM set ZIP asset".to_string())?;

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download PCem ROM set: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Failed to download PCem ROM set: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read PCem ROM set download: {}", e))?;

    std::fs::write(&archive_path, &bytes)
        .map_err(|e| format!("Failed to write {}: {}", archive_path.display(), e))?;

    if !zip_archive_is_fully_readable(&archive_path, bytes.len() as u64) {
        return Err(format!(
            "Downloaded PCem ROM set {} is not a fully readable ZIP archive",
            archive_path.display()
        ));
    }

    import_firmware_package(settings, pool, &archive_path, Some("github:pcem-romset"))
        .await
        .map(|_| ())
}

fn is_flatpak_retroarch_installed() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("flatpak")
            .args(["info", "org.libretro.RetroArch"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn is_flatpak_runtime_installed(emulator_info: &EmulatorInfo) -> bool {
    emulator::check_installation(emulator_info)
        .map(|path| path.to_string_lossy().starts_with("flatpak::"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_zip(archive_path: &Path, entries: &[(&str, &[u8])]) {
        let archive_file = File::create(archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(archive_file);
        for (name, contents) in entries {
            writer
                .start_file::<_, ()>(*name, zip::write::FileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }

    #[tokio::test]
    async fn seeds_builtin_firmware_rules() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();

        sync_builtin_rules(&pool).await.unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM firmware_rules")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(count >= BUILTIN_FIRMWARE_RULES.len() as i64);
    }

    #[tokio::test]
    async fn imports_zip_package_into_canonical_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir.path().join("Sony - PlayStation (SwanStation).zip");
        create_zip(
            &archive_path,
            &[("scph5501.bin", b"bios"), ("notes/readme.txt", b"readme")],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        let imported = import_firmware_package(&settings, &pool, &archive_path, None)
            .await
            .unwrap();
        assert_eq!(
            imported.package_name,
            "Sony - PlayStation (SwanStation).zip"
        );
        assert_eq!(imported.file_count, 2);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM firmware_files")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn syncs_imported_retroarch_package_into_system_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir.path().join("Sharp - X68000 (PX68k).zip");
        create_zip(
            &archive_path,
            &[("keropi/cgrom.dat", b"cg"), ("keropi/iplrom.dat", b"ipl")],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("minerva:retroarch-system-files"),
        )
        .await
        .unwrap();

        let system_dir = temp_dir.path().join("retroarch-system");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "retroarch".to_string(),
            runtime_name: "px68k".to_string(),
            runtime_dir: Some(system_dir.clone()),
            display_name: "RetroArch core 'px68k'".to_string(),
            runtime_path_display: system_dir.display().to_string(),
            launch_scoped: false,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Sharp X68000")
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(system_dir.join("keropi/cgrom.dat")).unwrap(),
            b"cg"
        );
        assert_eq!(
            std::fs::read(system_dir.join("keropi/iplrom.dat")).unwrap(),
            b"ipl"
        );

        let installs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM firmware_installs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(installs, 1);
    }

    #[tokio::test]
    async fn errors_when_required_retroarch_firmware_package_is_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        let system_dir = temp_dir.path().join("retroarch-system");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "retroarch".to_string(),
            runtime_name: "px68k".to_string(),
            runtime_dir: Some(system_dir.clone()),
            display_name: "RetroArch core 'px68k'".to_string(),
            runtime_path_display: system_dir.display().to_string(),
            launch_scoped: false,
        };

        let err = ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Sharp X68000")
            .await
            .unwrap_err();

        assert!(err.contains("Sharp - X68000 (PX68k).zip"));
        assert!(err.contains("Missing required firmware"));
    }

    #[tokio::test]
    async fn seeds_expanded_rule_families() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        async fn package_for(pool: &sqlx::SqlitePool, key: &str) -> Option<String> {
            sqlx::query_scalar::<_, String>(
                "SELECT source_package_name FROM firmware_rules WHERE rule_key = ? LIMIT 1",
            )
            .bind(key)
            .fetch_optional(pool)
            .await
            .unwrap()
        }

        assert_eq!(
            package_for(&pool, "duckstation:DuckStation:Sony Playstation").await,
            Some("Sony - PlayStation (SwanStation).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "pcsx2:PCSX2:Sony Playstation 2").await,
            Some("Sony - PlayStation 2 (LRPS2).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "melonds:melonDS:Nintendo DS").await,
            Some("Nintendo - DS (DeSmuME - melonDS).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "melonds:melonDS:Nintendo - Nintendo DSi").await,
            Some("DSi BIOS, firmware, and NAND image".to_string())
        );
        assert_eq!(
            package_for(&pool, "geepee32:GeePee32:GamePark GP32").await,
            Some("fw100k.bin or fw157e.bin".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:flycast:Sega Dreamcast").await,
            Some("Sega - Dreamcast - NAOMI (Flycast).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:pokemini:Nintendo Pokemon Mini").await,
            Some("Nintendo - Pokemon Mini (PokeMini).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "dolphin:Dolphin:Nintendo GameCube").await,
            Some("Nintendo - GameCube - Wii (Dolphin).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:flycast:Sega Naomi 2").await,
            Some("Sega - Dreamcast - NAOMI (Flycast).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "flycast:Flycast:Sega Dreamcast").await,
            Some("Sega - Dreamcast - NAOMI (Flycast).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "flycast:Flycast:Sega Naomi 2").await,
            Some("Arcade (Flycast) BIOS Files.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:mame:Sega ST-V").await,
            Some("stvbios.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mame:MAME:Sega ST-V").await,
            Some("stvbios.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mame:MAME:Emerson Arcadia 2001:ar_bios").await,
            Some("ar_bios.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mame:MAME:Emerson Arcadia 2001:arcadia_hash").await,
            Some("arcadia.xml".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:mame:TRS-80 Color Computer").await,
            Some("coco.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mame:MAME:TRS-80 Color Computer").await,
            Some("coco.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:mame:Elektronika BK:bk0010").await,
            Some("bk0010.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mame:MAME:Elektronika BK:bk0011m").await,
            Some("bk0011m.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:mame:Sharp X68000").await,
            Some("x68000.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mame:MAME:Sharp X68000").await,
            Some("x68000.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:mame:Texas Instruments TI 99/4A").await,
            Some("ti99_4a.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mame:MAME:Texas Instruments TI 99/4A").await,
            Some("ti99_4a.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:mame:Sony PocketStation").await,
            Some("pockstat.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:mame:Sord M5:m5").await,
            Some("m5.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mame:MAME:Sord M5:m5p").await,
            Some("m5p.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "86box:86Box:MS-DOS").await,
            Some("86Box ROM set.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "pcem:PCem:Windows 3.X").await,
            Some("PCem ROM set.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "retroarch:bluemsx_libretro:Microsoft MSX").await,
            Some("MSX-SVI-ColecoVision-SG1000 (blueMSX).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "openmsx:openMSX:Microsoft MSX2").await,
            Some("MSX-SVI-ColecoVision-SG1000 (blueMSX).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "openmsx:openMSX:Spectravideo").await,
            Some("MSX-SVI-ColecoVision-SG1000 (blueMSX).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "snes9x:Snes9x:Nintendo Satellaview").await,
            Some("Nintendo - SNES - SFC (Snes9x).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "o2em:O2EM:Philips Videopac+").await,
            Some("Magnavox - Odyssey2 - Philips Videopac+ (O2EM).zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "loopymse:LoopyMSE:Casio Loopy").await,
            Some("casloopy.zip".to_string())
        );
        assert_eq!(
            package_for(&pool, "mgba:mGBA:Nintendo - e-Reader").await,
            Some("Nintendo - Game Boy Advance (mGBA - VBA-M).zip".to_string())
        );
    }

    #[test]
    fn resolves_supported_standalone_runtime_contexts() {
        let duckstation = EmulatorInfo {
            id: 1,
            name: "DuckStation".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let pcsx2 = EmulatorInfo {
            id: 2,
            name: "PCSX2".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let melonds = EmulatorInfo {
            id: 3,
            name: "melonDS".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let flycast = EmulatorInfo {
            id: 4,
            name: "Flycast".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: Some("org.flycast.Flycast".to_string()),
            retroarch_core: Some("flycast".to_string()),
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let dolphin = EmulatorInfo {
            id: 5,
            name: "Dolphin".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: Some("dolphin".to_string()),
            flatpak_id: Some("org.DolphinEmu.dolphin-emu".to_string()),
            retroarch_core: Some("dolphin".to_string()),
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let mame = EmulatorInfo {
            id: 6,
            name: "MAME".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: Some("mame".to_string()),
            flatpak_id: Some("org.mamedev.MAME".to_string()),
            retroarch_core: Some("mame".to_string()),
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let eighty_six_box = EmulatorInfo {
            id: 7,
            name: "86Box".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: Some("86box".to_string()),
            flatpak_id: Some("net._86box._86Box".to_string()),
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let pcem = EmulatorInfo {
            id: 8,
            name: "PCem".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: Some("co.uk.pcemulator.PCem".to_string()),
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let openmsx = EmulatorInfo {
            id: 9,
            name: "openMSX".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: Some("org.openmsx.openMSX".to_string()),
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let snes9x = EmulatorInfo {
            id: 10,
            name: "Snes9x".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: Some("com.snes9x.Snes9x".to_string()),
            retroarch_core: Some("snes9x".to_string()),
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let o2em = EmulatorInfo {
            id: 11,
            name: "O2EM".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let mgba = EmulatorInfo {
            id: 12,
            name: "mGBA".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: Some("io.mgba.mGBA".to_string()),
            retroarch_core: Some("mgba".to_string()),
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let loopymse = EmulatorInfo {
            id: 13,
            name: "LoopyMSE".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let geepee32 = EmulatorInfo {
            id: 14,
            name: "GeePee32".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };

        let duckstation_ctx =
            resolve_runtime_context(&duckstation, "Sony Playstation", false).unwrap();
        assert_eq!(duckstation_ctx.runtime_kind, "duckstation");
        assert_eq!(duckstation_ctx.runtime_name, "DuckStation");

        let pcsx2_ctx = resolve_runtime_context(&pcsx2, "Sony Playstation 2", false).unwrap();
        assert_eq!(pcsx2_ctx.runtime_kind, "pcsx2");
        assert_eq!(pcsx2_ctx.runtime_name, "PCSX2");

        let melonds_ctx = resolve_runtime_context(&melonds, "Nintendo DS", false).unwrap();
        assert_eq!(melonds_ctx.runtime_kind, "melonds");
        assert_eq!(melonds_ctx.runtime_name, "melonDS");
        let melonds_dsi_ctx =
            resolve_runtime_context(&melonds, "Nintendo - Nintendo DSi", false).unwrap();
        assert_eq!(melonds_dsi_ctx.runtime_kind, "melonds");
        assert_eq!(melonds_dsi_ctx.runtime_name, "melonDS");
        assert!(!melonds_dsi_ctx.launch_scoped);
        assert!(melonds_dsi_ctx.runtime_dir.is_none());
        assert!(
            melonds_dsi_ctx
                .runtime_path_display
                .contains("Emu Settings")
        );

        let dolphin_ctx = resolve_runtime_context(&dolphin, "Nintendo GameCube", false).unwrap();
        assert_eq!(dolphin_ctx.runtime_kind, "dolphin");
        assert_eq!(dolphin_ctx.runtime_name, "Dolphin");

        let flycast_ctx = resolve_runtime_context(&flycast, "Sega Naomi 2", false).unwrap();
        assert_eq!(flycast_ctx.runtime_kind, "flycast");
        assert_eq!(flycast_ctx.runtime_name, "Flycast");

        let dreamcast_flycast_ctx =
            resolve_runtime_context(&flycast, "Sega Dreamcast", false).unwrap();
        assert_eq!(dreamcast_flycast_ctx.runtime_kind, "flycast");
        assert_eq!(dreamcast_flycast_ctx.runtime_name, "Flycast");

        let mame_ctx = resolve_runtime_context(&mame, "Sega ST-V", false).unwrap();
        assert_eq!(mame_ctx.runtime_kind, "mame");
        assert_eq!(mame_ctx.runtime_name, "MAME");
        assert!(!mame_ctx.launch_scoped);

        let retroarch_mame_ctx = resolve_runtime_context(&mame, "Sega ST-V", true).unwrap();
        assert_eq!(retroarch_mame_ctx.runtime_kind, "retroarch");
        assert_eq!(retroarch_mame_ctx.runtime_name, "mame");
        assert!(retroarch_mame_ctx.launch_scoped);
        assert!(retroarch_mame_ctx.runtime_dir.is_none());
        assert_eq!(
            retroarch_mame_ctx.runtime_path_display,
            "Copied beside the game ROM at launch"
        );

        let eighty_six_box_ctx = resolve_runtime_context(&eighty_six_box, "MS-DOS", false).unwrap();
        assert_eq!(eighty_six_box_ctx.runtime_kind, "86box");
        assert_eq!(eighty_six_box_ctx.runtime_name, "86Box");

        let pcem_ctx = resolve_runtime_context(&pcem, "Windows 3.X", false).unwrap();
        assert_eq!(pcem_ctx.runtime_kind, "pcem");
        assert_eq!(pcem_ctx.runtime_name, "PCem");
        assert!(!pcem_ctx.launch_scoped);
        assert!(pcem_ctx.runtime_path_display.contains(".pcem/roms"));

        let openmsx_ctx = resolve_runtime_context(&openmsx, "Microsoft MSX2", false).unwrap();
        assert_eq!(openmsx_ctx.runtime_kind, "openmsx");
        assert_eq!(openmsx_ctx.runtime_name, "openMSX");
        assert!(!openmsx_ctx.launch_scoped);
        assert!(openmsx_ctx.runtime_path_display.contains("systemroms"));

        let snes9x_ctx = resolve_runtime_context(&snes9x, "Nintendo Satellaview", false).unwrap();
        assert_eq!(snes9x_ctx.runtime_kind, "snes9x");
        assert_eq!(snes9x_ctx.runtime_name, "Snes9x");
        assert!(!snes9x_ctx.launch_scoped);
        assert!(snes9x_ctx.runtime_path_display.contains("Bios"));

        let o2em_ctx = resolve_runtime_context(&o2em, "Philips Videopac+", false).unwrap();
        assert_eq!(o2em_ctx.runtime_kind, "o2em");
        assert_eq!(o2em_ctx.runtime_name, "O2EM");
        assert!(!o2em_ctx.launch_scoped);
        assert!(o2em_ctx.runtime_path_display.contains(".o2em"));

        let mgba_ctx = resolve_runtime_context(&mgba, "Nintendo - e-Reader", false).unwrap();
        assert_eq!(mgba_ctx.runtime_kind, "mgba");
        assert_eq!(mgba_ctx.runtime_name, "mGBA");
        assert!(mgba_ctx.launch_scoped);
        assert!(mgba_ctx.runtime_dir.is_none());
        assert_eq!(mgba_ctx.runtime_path_display, "Passed as --bios at launch");

        let loopymse_ctx = resolve_runtime_context(&loopymse, "Casio Loopy", false).unwrap();
        assert_eq!(loopymse_ctx.runtime_kind, "loopymse");
        assert_eq!(loopymse_ctx.runtime_name, "LoopyMSE");
        assert!(loopymse_ctx.launch_scoped);
        assert!(loopymse_ctx.runtime_dir.is_none());
        assert_eq!(
            loopymse_ctx.runtime_path_display,
            "Passed as <BIOS> [sound BIOS] at launch"
        );

        let geepee32_ctx = resolve_runtime_context(&geepee32, "GamePark GP32", false).unwrap();
        assert_eq!(geepee32_ctx.runtime_kind, "geepee32");
        assert_eq!(geepee32_ctx.runtime_name, "GeePee32");
        assert!(!geepee32_ctx.launch_scoped);
        assert!(geepee32_ctx.runtime_dir.is_none());
        assert!(
            geepee32_ctx
                .runtime_path_display
                .contains("fw100k.bin or fw157e.bin")
        );
    }

    #[test]
    fn finds_minerva_source_file_index_for_retroarch_package() {
        let files = vec![
            crate::torrent::TorrentFileInfo {
                index: 7,
                filename: "Internet Archive/chadmaster/RetroarchSystemFiles/Retroarch-System/Sharp - X68000 (PX68k).zip".to_string(),
                size: 1234,
            },
            crate::torrent::TorrentFileInfo {
                index: 8,
                filename: "something/else.bin".to_string(),
                size: 1,
            },
        ];

        let found = find_minerva_source_file_index(
            &files,
            RETROARCH_SYSTEM_FILES_PATH_PREFIX,
            "Sharp - X68000 (PX68k).zip",
        );

        assert_eq!(found, Some(7));
    }

    #[tokio::test]
    async fn syncs_imported_mame_bios_archive_into_rompath() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir.path().join("stvbios.zip");
        create_zip(&archive_path, &[("dummy.rom", b"stv")]);

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("minerva:mame-merged-romsets"),
        )
        .await
        .unwrap();

        let roms_dir = temp_dir.path().join("mame-roms");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "mame".to_string(),
            runtime_name: "MAME".to_string(),
            runtime_dir: Some(roms_dir.clone()),
            display_name: "MAME".to_string(),
            runtime_path_display: roms_dir.display().to_string(),
            launch_scoped: false,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Sega ST-V")
            .await
            .unwrap();

        let installed_zip = roms_dir.join("stvbios.zip");
        assert!(installed_zip.exists());
        assert_eq!(
            std::fs::read(installed_zip).unwrap(),
            std::fs::read(archive_path).unwrap()
        );
    }

    #[tokio::test]
    async fn syncs_imported_mame_hash_file_into_hash_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let bios_archive_path = temp_dir.path().join("ar_bios.zip");
        create_zip(&bios_archive_path, &[("dummy.rom", b"arcadia")]);

        let archive_path = temp_dir.path().join("arcadia.xml");
        create_zip(
            &archive_path,
            &[(
                "arcadia.xml",
                br#"<softwarelist name="arcadia"></softwarelist>"#,
            )],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &bios_archive_path,
            Some("minerva:mame-merged-romsets"),
        )
        .await
        .unwrap();

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("github:mame-hash-files"),
        )
        .await
        .unwrap();

        let roms_dir = temp_dir.path().join("mame").join("roms");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "mame".to_string(),
            runtime_name: "MAME".to_string(),
            runtime_dir: Some(roms_dir.clone()),
            display_name: "MAME".to_string(),
            runtime_path_display: roms_dir.display().to_string(),
            launch_scoped: false,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Emerson Arcadia 2001")
            .await
            .unwrap();

        let installed_hash = temp_dir
            .path()
            .join("mame")
            .join("hash")
            .join("arcadia.xml");
        assert!(installed_hash.exists());
        assert_eq!(
            std::fs::read_to_string(installed_hash).unwrap(),
            r#"<softwarelist name="arcadia"></softwarelist>"#
        );
    }

    #[tokio::test]
    async fn syncs_imported_retroarch_mame_bios_archive_beside_rom() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir.path().join("stvbios.zip");
        create_zip(&archive_path, &[("dummy.rom", b"stv")]);

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("minerva:mame-merged-romsets"),
        )
        .await
        .unwrap();

        let rom_dir = temp_dir.path().join("rom-dir");
        std::fs::create_dir_all(&rom_dir).unwrap();
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "retroarch".to_string(),
            runtime_name: "mame".to_string(),
            runtime_dir: Some(rom_dir.clone()),
            display_name: "RetroArch core 'mame'".to_string(),
            runtime_path_display: rom_dir.display().to_string(),
            launch_scoped: true,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Sega ST-V")
            .await
            .unwrap();

        let installed_zip = rom_dir.join("stvbios.zip");
        assert!(installed_zip.exists());
        assert_eq!(
            std::fs::read(installed_zip).unwrap(),
            std::fs::read(archive_path).unwrap()
        );
    }

    #[tokio::test]
    async fn syncs_imported_86box_romset_into_roms_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir.path().join(EIGHTY_SIX_BOX_ROMSET_PACKAGE_NAME);
        create_zip(
            &archive_path,
            &[
                ("machines/test/machine.rom", b"rom"),
                ("video/test.rom", b"vga"),
            ],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(&settings, &pool, &archive_path, Some("github:86box-romset"))
            .await
            .unwrap();

        let roms_dir = temp_dir.path().join("86box-roms");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "86box".to_string(),
            runtime_name: "86Box".to_string(),
            runtime_dir: Some(roms_dir.clone()),
            display_name: "86Box".to_string(),
            runtime_path_display: roms_dir.display().to_string(),
            launch_scoped: false,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "MS-DOS")
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(roms_dir.join("machines/test/machine.rom")).unwrap(),
            b"rom"
        );
        assert_eq!(
            std::fs::read(roms_dir.join("video/test.rom")).unwrap(),
            b"vga"
        );
    }

    #[tokio::test]
    async fn syncs_imported_pcem_romset_into_roms_dir_and_strips_archive_root() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir.path().join(PCEM_ROMSET_PACKAGE_NAME);
        create_zip(
            &archive_path,
            &[
                ("PCem-ROMs-v17/machines/test/machine.bin", b"rom"),
                ("PCem-ROMs-v17/video/test.bin", b"vga"),
            ],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(&settings, &pool, &archive_path, Some("github:pcem-romset"))
            .await
            .unwrap();

        let roms_dir = temp_dir.path().join("pcem-roms");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "pcem".to_string(),
            runtime_name: "PCem".to_string(),
            runtime_dir: Some(roms_dir.clone()),
            display_name: "PCem".to_string(),
            runtime_path_display: roms_dir.display().to_string(),
            launch_scoped: false,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Windows 3.X")
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(roms_dir.join("machines/test/machine.bin")).unwrap(),
            b"rom"
        );
        assert_eq!(
            std::fs::read(roms_dir.join("video/test.bin")).unwrap(),
            b"vga"
        );
    }

    #[tokio::test]
    async fn syncs_imported_openmsx_package_into_systemroms_without_metadata_junk() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir
            .path()
            .join("MSX-SVI-ColecoVision-SG1000 (blueMSX).zip");
        create_zip(
            &archive_path,
            &[
                ("Machines/MSX - C-BIOS/cbios_main_msx1.rom", b"romdata"),
                ("Machines/MSX - C-BIOS/config.ini", b"ini"),
                ("Databases/msxromdb.xml", b"xml"),
            ],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("minerva:retroarch-system-files"),
        )
        .await
        .unwrap();

        let systemroms_dir = temp_dir.path().join("openmsx-systemroms");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "openmsx".to_string(),
            runtime_name: "openMSX".to_string(),
            runtime_dir: Some(systemroms_dir.clone()),
            display_name: "openMSX".to_string(),
            runtime_path_display: systemroms_dir.display().to_string(),
            launch_scoped: false,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Microsoft MSX2")
            .await
            .unwrap();

        assert!(
            systemroms_dir
                .join("Machines/MSX - C-BIOS/cbios_main_msx1.rom")
                .exists()
        );
        assert!(
            !systemroms_dir
                .join("Machines/MSX - C-BIOS/config.ini")
                .exists()
        );
        assert!(!systemroms_dir.join("Databases/msxromdb.xml").exists());
    }

    #[tokio::test]
    async fn syncs_imported_snes9x_satellaview_package_into_bios_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir.path().join("Nintendo - SNES - SFC (Snes9x).zip");
        create_zip(
            &archive_path,
            &[("BS-X.bin", b"bsx"), ("STBIOS.bin", b"stbios")],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("minerva:retroarch-system-files"),
        )
        .await
        .unwrap();

        let bios_dir = temp_dir.path().join("snes9x-bios");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "snes9x".to_string(),
            runtime_name: "Snes9x".to_string(),
            runtime_dir: Some(bios_dir.clone()),
            display_name: "Snes9x".to_string(),
            runtime_path_display: bios_dir.display().to_string(),
            launch_scoped: false,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Nintendo Satellaview")
            .await
            .unwrap();

        assert_eq!(std::fs::read(bios_dir.join("BS-X.bin")).unwrap(), b"bsx");
        assert_eq!(
            std::fs::read(bios_dir.join("STBIOS.bin")).unwrap(),
            b"stbios"
        );
    }

    #[tokio::test]
    async fn syncs_imported_o2em_package_into_bios_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir
            .path()
            .join("Magnavox - Odyssey2 - Philips Videopac+ (O2EM).zip");
        create_zip(
            &archive_path,
            &[("o2rom.bin", b"o2"), ("g7400.bin", b"g7400")],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("minerva:retroarch-system-files"),
        )
        .await
        .unwrap();

        let bios_dir = temp_dir.path().join("o2em");
        let runtime = FirmwareRuntimeContext {
            runtime_kind: "o2em".to_string(),
            runtime_name: "O2EM".to_string(),
            runtime_dir: Some(bios_dir.clone()),
            display_name: "O2EM".to_string(),
            runtime_path_display: bios_dir.display().to_string(),
            launch_scoped: false,
        };

        ensure_firmware_for_runtime(&settings, &pool, None, &runtime, "Philips Videopac+")
            .await
            .unwrap();

        assert_eq!(std::fs::read(bios_dir.join("o2rom.bin")).unwrap(), b"o2");
        assert_eq!(std::fs::read(bios_dir.join("g7400.bin")).unwrap(), b"g7400");
    }

    #[tokio::test]
    async fn resolves_mgba_launch_args_from_imported_bios_package() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir
            .path()
            .join("Nintendo - Game Boy Advance (mGBA - VBA-M).zip");
        create_zip(
            &archive_path,
            &[("gba_bios.bin", b"bios"), ("readme.txt", b"ignored")],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("minerva:retroarch-system-files"),
        )
        .await
        .unwrap();

        let emulator = EmulatorInfo {
            id: 1,
            name: "mGBA".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: Some("io.mgba.mGBA".to_string()),
            retroarch_core: Some("mgba".to_string()),
            save_directory: None,
            save_extensions: None,
            notes: None,
        };

        let args = get_launch_firmware_args(&pool, &emulator, "Nintendo - e-Reader", false)
            .await
            .unwrap();

        assert_eq!(args.len(), 2);
        assert_eq!(args[0], LaunchArg::Literal("--bios".to_string()));
        match &args[1] {
            LaunchArg::Path(path) => {
                assert!(path.ends_with("/gba_bios.bin"));
                assert!(path.contains("/firmware/packages/minerva:retroarch-system-files/"));
            }
            other => panic!("expected bios path launch arg, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn resolves_loopymse_launch_args_from_imported_casio_bios_package() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();
        sync_builtin_rules(&pool).await.unwrap();

        let archive_path = temp_dir.path().join("casloopy.zip");
        create_zip(
            &archive_path,
            &[
                ("hd6437021.lsi302", b"main"),
                ("hn62434fa.lsi352", b"sound"),
            ],
        );

        let mut settings = AppSettings::default();
        settings.data_directory = Some(temp_dir.path().join("appdata"));

        import_firmware_package(
            &settings,
            &pool,
            &archive_path,
            Some("minerva:mame-merged-romsets"),
        )
        .await
        .unwrap();

        let emulator = EmulatorInfo {
            id: 1,
            name: "LoopyMSE".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };

        let args = get_launch_firmware_args(&pool, &emulator, "Casio Loopy", false)
            .await
            .unwrap();

        assert_eq!(args.len(), 2);
        match &args[0] {
            LaunchArg::Path(path) => assert!(path.ends_with("/hd6437021.lsi302")),
            other => panic!("expected main bios path arg, got {:?}", other),
        }
        match &args[1] {
            LaunchArg::Path(path) => assert!(path.ends_with("/hn62434fa.lsi352")),
            other => panic!("expected sound bios path arg, got {:?}", other),
        }
    }
}
