//! Unified database import from multiple sources
//!
//! Import priority (best quality first):
//! 1. LaunchBox - Richest metadata (descriptions, ratings, etc.)
//! 2. LibRetro/No-Intro - Authoritative ROM checksums
//! 3. OpenVGDB - Additional metadata enrichment
//!
//! Key principles:
//! - Import best sources first
//! - Never overwrite non-empty fields
//! - Create new records for missing games/platforms
//! - Keep all source IDs for cross-referencing

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

use crate::enrich::{extract_variant_tags, normalize_title, similarity_ratio};

/// Platform name aliases for matching across different databases
/// Maps various names to a canonical name (using LaunchBox Platforms.xml names as canonical)
fn get_platform_aliases() -> HashMap<&'static str, &'static str> {
    let mut aliases = HashMap::new();

    // =========================================================================
    // MS-DOS / PC
    // =========================================================================
    aliases.insert("ms-dos", "MS-DOS");
    aliases.insert("dos", "MS-DOS");
    aliases.insert("microsoft - ms-dos", "MS-DOS");
    aliases.insert("pc - dos", "MS-DOS");
    aliases.insert("ibm pc", "MS-DOS");

    // =========================================================================
    // Nintendo - NES / Famicom
    // =========================================================================
    aliases.insert("nes", "Nintendo Entertainment System");
    aliases.insert("nintendo entertainment system", "Nintendo Entertainment System");
    aliases.insert("nintendo - nintendo entertainment system", "Nintendo Entertainment System");
    aliases.insert("famicom", "Nintendo Entertainment System");
    aliases.insert("nintendo - family computer", "Nintendo Entertainment System");

    // Famicom Disk System
    aliases.insert("fds", "Nintendo Famicom Disk System");
    aliases.insert("famicom disk system", "Nintendo Famicom Disk System");
    aliases.insert("nintendo famicom disk system", "Nintendo Famicom Disk System");
    aliases.insert("nintendo - famicom disk system", "Nintendo Famicom Disk System");

    // =========================================================================
    // Nintendo - SNES / Super Famicom
    // =========================================================================
    aliases.insert("snes", "Super Nintendo Entertainment System");
    aliases.insert("super nes", "Super Nintendo Entertainment System");
    aliases.insert("super famicom", "Super Nintendo Entertainment System");
    aliases.insert("super nintendo entertainment system", "Super Nintendo Entertainment System");
    aliases.insert("nintendo - super nintendo entertainment system", "Super Nintendo Entertainment System");
    aliases.insert("nintendo super nintendo entertainment system", "Super Nintendo Entertainment System");

    // =========================================================================
    // Nintendo - Game Boy family
    // =========================================================================
    aliases.insert("gb", "Nintendo Game Boy");
    aliases.insert("game boy", "Nintendo Game Boy");
    aliases.insert("nintendo game boy", "Nintendo Game Boy");
    aliases.insert("nintendo - game boy", "Nintendo Game Boy");

    aliases.insert("gbc", "Nintendo Game Boy Color");
    aliases.insert("game boy color", "Nintendo Game Boy Color");
    aliases.insert("nintendo game boy color", "Nintendo Game Boy Color");
    aliases.insert("nintendo - game boy color", "Nintendo Game Boy Color");

    aliases.insert("gba", "Nintendo Game Boy Advance");
    aliases.insert("game boy advance", "Nintendo Game Boy Advance");
    aliases.insert("nintendo game boy advance", "Nintendo Game Boy Advance");
    aliases.insert("nintendo - game boy advance", "Nintendo Game Boy Advance");

    // =========================================================================
    // Nintendo - N64
    // =========================================================================
    aliases.insert("n64", "Nintendo 64");
    aliases.insert("nintendo 64", "Nintendo 64");
    aliases.insert("nintendo - nintendo 64", "Nintendo 64");

    // =========================================================================
    // Nintendo - GameCube
    // =========================================================================
    aliases.insert("gamecube", "Nintendo GameCube");
    aliases.insert("gc", "Nintendo GameCube");
    aliases.insert("ngc", "Nintendo GameCube");
    aliases.insert("nintendo gamecube", "Nintendo GameCube");
    aliases.insert("nintendo - gamecube", "Nintendo GameCube");

    // =========================================================================
    // Nintendo - Wii / Wii U
    // =========================================================================
    aliases.insert("wii", "Nintendo Wii");
    aliases.insert("nintendo wii", "Nintendo Wii");
    aliases.insert("nintendo - wii", "Nintendo Wii");

    aliases.insert("wii u", "Nintendo Wii U");
    aliases.insert("wiiu", "Nintendo Wii U");
    aliases.insert("nintendo wii u", "Nintendo Wii U");
    aliases.insert("nintendo - wii u", "Nintendo Wii U");

    // =========================================================================
    // Nintendo - DS / 3DS
    // =========================================================================
    aliases.insert("ds", "Nintendo DS");
    aliases.insert("nds", "Nintendo DS");
    aliases.insert("nintendo ds", "Nintendo DS");
    aliases.insert("nintendo - nintendo ds", "Nintendo DS");

    aliases.insert("3ds", "Nintendo 3DS");
    aliases.insert("nintendo 3ds", "Nintendo 3DS");
    aliases.insert("nintendo - nintendo 3ds", "Nintendo 3DS");

    // =========================================================================
    // Nintendo - Virtual Boy
    // =========================================================================
    aliases.insert("vb", "Nintendo Virtual Boy");
    aliases.insert("virtual boy", "Nintendo Virtual Boy");
    aliases.insert("nintendo virtual boy", "Nintendo Virtual Boy");
    aliases.insert("nintendo - virtual boy", "Nintendo Virtual Boy");

    // =========================================================================
    // Nintendo - Switch
    // =========================================================================
    aliases.insert("switch", "Nintendo Switch");
    aliases.insert("nintendo switch", "Nintendo Switch");
    aliases.insert("nintendo - switch", "Nintendo Switch");

    // =========================================================================
    // Nintendo - Pokemon Mini
    // =========================================================================
    aliases.insert("pokemon mini", "Nintendo Pokemon Mini");
    aliases.insert("nintendo pokemon mini", "Nintendo Pokemon Mini");
    aliases.insert("nintendo - pokemon mini", "Nintendo Pokemon Mini");

    // =========================================================================
    // Sony - PlayStation
    // =========================================================================
    aliases.insert("ps1", "Sony Playstation");
    aliases.insert("psx", "Sony Playstation");
    aliases.insert("playstation", "Sony Playstation");
    aliases.insert("sony playstation", "Sony Playstation");
    aliases.insert("sony - playstation", "Sony Playstation");

    aliases.insert("ps2", "Sony Playstation 2");
    aliases.insert("playstation 2", "Sony Playstation 2");
    aliases.insert("sony playstation 2", "Sony Playstation 2");
    aliases.insert("sony - playstation 2", "Sony Playstation 2");

    aliases.insert("ps3", "Sony Playstation 3");
    aliases.insert("playstation 3", "Sony Playstation 3");
    aliases.insert("sony playstation 3", "Sony Playstation 3");
    aliases.insert("sony - playstation 3", "Sony Playstation 3");

    aliases.insert("ps4", "Sony Playstation 4");
    aliases.insert("playstation 4", "Sony Playstation 4");
    aliases.insert("sony playstation 4", "Sony Playstation 4");
    aliases.insert("sony - playstation 4", "Sony Playstation 4");

    aliases.insert("ps5", "Sony Playstation 5");
    aliases.insert("playstation 5", "Sony Playstation 5");
    aliases.insert("sony playstation 5", "Sony Playstation 5");
    aliases.insert("sony - playstation 5", "Sony Playstation 5");

    // =========================================================================
    // Sony - PSP / Vita
    // =========================================================================
    aliases.insert("psp", "Sony PSP");
    aliases.insert("sony psp", "Sony PSP");
    aliases.insert("playstation portable", "Sony PSP");
    aliases.insert("sony playstation portable", "Sony PSP");
    aliases.insert("sony - playstation portable", "Sony PSP");

    aliases.insert("vita", "Sony Playstation Vita");
    aliases.insert("ps vita", "Sony Playstation Vita");
    aliases.insert("playstation vita", "Sony Playstation Vita");
    aliases.insert("sony playstation vita", "Sony Playstation Vita");
    aliases.insert("sony - playstation vita", "Sony Playstation Vita");

    // =========================================================================
    // Sega - Genesis / Mega Drive
    // =========================================================================
    aliases.insert("genesis", "Sega Genesis");
    aliases.insert("md", "Sega Genesis");
    aliases.insert("mega drive", "Sega Genesis");
    aliases.insert("sega genesis", "Sega Genesis");
    aliases.insert("sega mega drive", "Sega Genesis");
    aliases.insert("sega - mega drive - genesis", "Sega Genesis");
    aliases.insert("sega genesis/mega drive", "Sega Genesis");

    // =========================================================================
    // Sega - Master System
    // =========================================================================
    aliases.insert("sms", "Sega Master System");
    aliases.insert("master system", "Sega Master System");
    aliases.insert("sega master system", "Sega Master System");
    aliases.insert("sega - master system - mark iii", "Sega Master System");

    // =========================================================================
    // Sega - Game Gear
    // =========================================================================
    aliases.insert("gg", "Sega Game Gear");
    aliases.insert("game gear", "Sega Game Gear");
    aliases.insert("sega game gear", "Sega Game Gear");
    aliases.insert("sega - game gear", "Sega Game Gear");

    // =========================================================================
    // Sega - Saturn
    // =========================================================================
    aliases.insert("saturn", "Sega Saturn");
    aliases.insert("sega saturn", "Sega Saturn");
    aliases.insert("sega - saturn", "Sega Saturn");

    // =========================================================================
    // Sega - Dreamcast
    // =========================================================================
    aliases.insert("dreamcast", "Sega Dreamcast");
    aliases.insert("dc", "Sega Dreamcast");
    aliases.insert("sega dreamcast", "Sega Dreamcast");
    aliases.insert("sega - dreamcast", "Sega Dreamcast");

    // =========================================================================
    // Sega - 32X
    // =========================================================================
    aliases.insert("32x", "Sega 32X");
    aliases.insert("sega 32x", "Sega 32X");
    aliases.insert("sega - 32x", "Sega 32X");

    // =========================================================================
    // Sega - CD / Mega-CD
    // =========================================================================
    aliases.insert("scd", "Sega CD");
    aliases.insert("mega cd", "Sega CD");
    aliases.insert("mega-cd", "Sega CD");
    aliases.insert("sega cd", "Sega CD");
    aliases.insert("sega - cd", "Sega CD");
    aliases.insert("sega cd/mega-cd", "Sega CD");

    // =========================================================================
    // Sega - SG-1000
    // =========================================================================
    aliases.insert("sg1000", "Sega SG-1000");
    aliases.insert("sg-1000", "Sega SG-1000");
    aliases.insert("sega sg-1000", "Sega SG-1000");
    aliases.insert("sega - sg-1000", "Sega SG-1000");

    // =========================================================================
    // Atari - 2600 / 5200 / 7800
    // =========================================================================
    aliases.insert("2600", "Atari 2600");
    aliases.insert("atari - 2600", "Atari 2600");

    aliases.insert("5200", "Atari 5200");
    aliases.insert("atari - 5200", "Atari 5200");

    aliases.insert("7800", "Atari 7800");
    aliases.insert("atari - 7800", "Atari 7800");

    // =========================================================================
    // Atari - Lynx
    // =========================================================================
    aliases.insert("lynx", "Atari Lynx");
    aliases.insert("atari - lynx", "Atari Lynx");

    // =========================================================================
    // Atari - Jaguar
    // =========================================================================
    aliases.insert("jaguar", "Atari Jaguar");
    aliases.insert("atari - jaguar", "Atari Jaguar");

    aliases.insert("jaguar cd", "Atari Jaguar CD");
    aliases.insert("atari - jaguar cd", "Atari Jaguar CD");

    // =========================================================================
    // Atari - ST
    // =========================================================================
    aliases.insert("st", "Atari ST");
    aliases.insert("atari st", "Atari ST");
    aliases.insert("atari - st", "Atari ST");

    // =========================================================================
    // Atari - 800 / 8-bit
    // =========================================================================
    aliases.insert("atari 8-bit", "Atari 800");
    aliases.insert("atari - 800", "Atari 800");

    // =========================================================================
    // NEC - TurboGrafx-16 / PC Engine
    // =========================================================================
    aliases.insert("pce", "NEC TurboGrafx-16");
    aliases.insert("turbografx-16", "NEC TurboGrafx-16");
    aliases.insert("turbografx 16", "NEC TurboGrafx-16");
    aliases.insert("pc engine", "NEC TurboGrafx-16");
    aliases.insert("nec - pc engine - turbografx 16", "NEC TurboGrafx-16");
    aliases.insert("nec pc engine/turbografx-16", "NEC TurboGrafx-16");

    // =========================================================================
    // NEC - TurboGrafx-CD / PC Engine CD
    // =========================================================================
    aliases.insert("pcecd", "NEC TurboGrafx-CD");
    aliases.insert("turbografx-cd", "NEC TurboGrafx-CD");
    aliases.insert("pc engine cd", "NEC TurboGrafx-CD");
    aliases.insert("nec - pc engine cd - turbografx-cd", "NEC TurboGrafx-CD");
    aliases.insert("nec pc engine cd/turbografx-cd", "NEC TurboGrafx-CD");

    // =========================================================================
    // NEC - SuperGrafx
    // =========================================================================
    aliases.insert("supergrafx", "PC Engine SuperGrafx");
    aliases.insert("nec supergrafx", "PC Engine SuperGrafx");
    aliases.insert("nec - supergrafx", "PC Engine SuperGrafx");

    // =========================================================================
    // NEC - PC-FX
    // =========================================================================
    aliases.insert("pcfx", "NEC PC-FX");
    aliases.insert("pc-fx", "NEC PC-FX");
    aliases.insert("nec - pc-fx", "NEC PC-FX");

    // =========================================================================
    // SNK - Neo Geo
    // =========================================================================
    aliases.insert("neo geo", "SNK Neo Geo AES");
    aliases.insert("neogeo", "SNK Neo Geo AES");
    aliases.insert("snk - neo geo", "SNK Neo Geo AES");
    aliases.insert("neo geo aes", "SNK Neo Geo AES");

    aliases.insert("neo geo mvs", "SNK Neo Geo MVS");
    aliases.insert("snk - neo geo mvs", "SNK Neo Geo MVS");

    aliases.insert("neo geo cd", "SNK Neo Geo CD");
    aliases.insert("snk - neo geo cd", "SNK Neo Geo CD");

    // =========================================================================
    // SNK - Neo Geo Pocket
    // =========================================================================
    aliases.insert("ngp", "SNK Neo Geo Pocket");
    aliases.insert("neo geo pocket", "SNK Neo Geo Pocket");
    aliases.insert("snk - neo geo pocket", "SNK Neo Geo Pocket");

    aliases.insert("ngpc", "SNK Neo Geo Pocket Color");
    aliases.insert("neo geo pocket color", "SNK Neo Geo Pocket Color");
    aliases.insert("snk - neo geo pocket color", "SNK Neo Geo Pocket Color");

    // =========================================================================
    // Arcade
    // =========================================================================
    aliases.insert("arcade", "Arcade");
    aliases.insert("mame", "Arcade");
    aliases.insert("fbneo", "Arcade");

    // =========================================================================
    // Commodore - 64
    // =========================================================================
    aliases.insert("c64", "Commodore 64");
    aliases.insert("commodore - 64", "Commodore 64");

    // =========================================================================
    // Commodore - Amiga
    // =========================================================================
    aliases.insert("amiga", "Commodore Amiga");
    aliases.insert("commodore - amiga", "Commodore Amiga");

    aliases.insert("amiga cd32", "Commodore Amiga CD32");
    aliases.insert("commodore - amiga cd32", "Commodore Amiga CD32");

    // =========================================================================
    // Commodore - VIC-20
    // =========================================================================
    aliases.insert("vic-20", "Commodore VIC-20");
    aliases.insert("vic20", "Commodore VIC-20");
    aliases.insert("commodore - vic-20", "Commodore VIC-20");

    // =========================================================================
    // Commodore - 128
    // =========================================================================
    aliases.insert("c128", "Commodore 128");
    aliases.insert("commodore - 128", "Commodore 128");

    // =========================================================================
    // Sinclair - ZX Spectrum
    // =========================================================================
    aliases.insert("zx spectrum", "Sinclair ZX Spectrum");
    aliases.insert("spectrum", "Sinclair ZX Spectrum");
    aliases.insert("sinclair - zx spectrum", "Sinclair ZX Spectrum");

    // =========================================================================
    // Sinclair - ZX-81
    // =========================================================================
    aliases.insert("zx81", "Sinclair ZX-81");
    aliases.insert("zx-81", "Sinclair ZX-81");
    aliases.insert("sinclair - zx81", "Sinclair ZX-81");

    // =========================================================================
    // Microsoft - Xbox
    // =========================================================================
    aliases.insert("xbox", "Microsoft Xbox");
    aliases.insert("microsoft - xbox", "Microsoft Xbox");

    aliases.insert("xbox 360", "Microsoft Xbox 360");
    aliases.insert("microsoft - xbox 360", "Microsoft Xbox 360");

    aliases.insert("xbox one", "Microsoft Xbox One");
    aliases.insert("microsoft - xbox one", "Microsoft Xbox One");

    aliases.insert("xbox series x", "Microsoft Xbox Series X/S");
    aliases.insert("xbox series s", "Microsoft Xbox Series X/S");
    aliases.insert("microsoft - xbox series x/s", "Microsoft Xbox Series X/S");

    // =========================================================================
    // Microsoft - MSX
    // =========================================================================
    aliases.insert("msx", "Microsoft MSX");
    aliases.insert("microsoft - msx", "Microsoft MSX");

    aliases.insert("msx2", "Microsoft MSX2");
    aliases.insert("microsoft - msx2", "Microsoft MSX2");

    aliases.insert("msx2+", "Microsoft MSX2+");
    aliases.insert("microsoft - msx2+", "Microsoft MSX2+");

    // =========================================================================
    // Bandai - WonderSwan
    // =========================================================================
    aliases.insert("wonderswan", "WonderSwan");
    aliases.insert("ws", "WonderSwan");
    aliases.insert("bandai wonderswan", "WonderSwan");
    aliases.insert("bandai - wonderswan", "WonderSwan");

    aliases.insert("wonderswan color", "WonderSwan Color");
    aliases.insert("wsc", "WonderSwan Color");
    aliases.insert("bandai wonderswan color", "WonderSwan Color");
    aliases.insert("bandai - wonderswan color", "WonderSwan Color");

    // =========================================================================
    // Coleco - ColecoVision
    // =========================================================================
    aliases.insert("colecovision", "ColecoVision");
    aliases.insert("coleco colecovision", "ColecoVision");
    aliases.insert("coleco - colecovision", "ColecoVision");

    // =========================================================================
    // Mattel - Intellivision
    // =========================================================================
    aliases.insert("intellivision", "Mattel Intellivision");
    aliases.insert("mattel - intellivision", "Mattel Intellivision");

    // =========================================================================
    // Magnavox - Odyssey
    // =========================================================================
    aliases.insert("odyssey", "Magnavox Odyssey");
    aliases.insert("magnavox - odyssey", "Magnavox Odyssey");

    aliases.insert("odyssey2", "Magnavox Odyssey 2");
    aliases.insert("odyssey 2", "Magnavox Odyssey 2");
    aliases.insert("magnavox odyssey2", "Magnavox Odyssey 2");
    aliases.insert("magnavox - odyssey 2", "Magnavox Odyssey 2");

    // =========================================================================
    // GCE - Vectrex
    // =========================================================================
    aliases.insert("vectrex", "GCE Vectrex");
    aliases.insert("gce - vectrex", "GCE Vectrex");

    // =========================================================================
    // 3DO
    // =========================================================================
    aliases.insert("3do", "3DO Interactive Multiplayer");
    aliases.insert("3do interactive multiplayer", "3DO Interactive Multiplayer");

    // =========================================================================
    // Amstrad - CPC
    // =========================================================================
    aliases.insert("cpc", "Amstrad CPC");
    aliases.insert("amstrad cpc", "Amstrad CPC");
    aliases.insert("amstrad - cpc", "Amstrad CPC");

    aliases.insert("gx4000", "Amstrad GX4000");
    aliases.insert("amstrad gx4000", "Amstrad GX4000");
    aliases.insert("amstrad - gx4000", "Amstrad GX4000");

    // =========================================================================
    // Apple
    // =========================================================================
    aliases.insert("apple ii", "Apple II");
    aliases.insert("apple 2", "Apple II");
    aliases.insert("apple - ii", "Apple II");

    aliases.insert("apple iigs", "Apple IIGS");
    aliases.insert("apple - iigs", "Apple IIGS");

    // =========================================================================
    // Philips - CD-i
    // =========================================================================
    aliases.insert("cd-i", "Philips CD-i");
    aliases.insert("cdi", "Philips CD-i");
    aliases.insert("philips - cd-i", "Philips CD-i");

    // =========================================================================
    // Fairchild - Channel F
    // =========================================================================
    aliases.insert("channel f", "Fairchild Channel F");
    aliases.insert("fairchild - channel f", "Fairchild Channel F");

    // =========================================================================
    // Nokia - N-Gage
    // =========================================================================
    aliases.insert("n-gage", "Nokia N-Gage");
    aliases.insert("ngage", "Nokia N-Gage");
    aliases.insert("nokia - n-gage", "Nokia N-Gage");

    // =========================================================================
    // Tandy - TRS-80
    // =========================================================================
    aliases.insert("trs-80", "Tandy TRS-80");
    aliases.insert("trs80", "Tandy TRS-80");
    aliases.insert("tandy - trs-80", "Tandy TRS-80");

    aliases.insert("coco", "TRS-80 Color Computer");
    aliases.insert("trs-80 coco", "TRS-80 Color Computer");

    // =========================================================================
    // BBC Microcomputer
    // =========================================================================
    aliases.insert("bbc micro", "BBC Microcomputer System");
    aliases.insert("bbc microcomputer", "BBC Microcomputer System");
    aliases.insert("bbc - microcomputer", "BBC Microcomputer System");

    // =========================================================================
    // Texas Instruments - TI-99/4A
    // =========================================================================
    aliases.insert("ti-99/4a", "Texas Instruments TI 99/4A");
    aliases.insert("ti99", "Texas Instruments TI 99/4A");
    aliases.insert("ti - 99/4a", "Texas Instruments TI 99/4A");

    // =========================================================================
    // Sharp - X68000
    // =========================================================================
    aliases.insert("x68000", "Sharp X68000");
    aliases.insert("sharp - x68000", "Sharp X68000");

    // =========================================================================
    // Sharp - X1
    // =========================================================================
    aliases.insert("x1", "Sharp X1");
    aliases.insert("sharp - x1", "Sharp X1");

    // =========================================================================
    // NEC - PC-8801 / PC-9801
    // =========================================================================
    aliases.insert("pc-8801", "NEC PC-8801");
    aliases.insert("pc8801", "NEC PC-8801");
    aliases.insert("nec - pc-8801", "NEC PC-8801");

    aliases.insert("pc-9801", "NEC PC-9801");
    aliases.insert("pc9801", "NEC PC-9801");
    aliases.insert("nec - pc-9801", "NEC PC-9801");

    // =========================================================================
    // Fujitsu - FM Towns
    // =========================================================================
    aliases.insert("fm towns", "Fujitsu FM Towns Marty");
    aliases.insert("fm towns marty", "Fujitsu FM Towns Marty");
    aliases.insert("fujitsu - fm towns", "Fujitsu FM Towns Marty");

    // =========================================================================
    // Fujitsu - FM-7
    // =========================================================================
    aliases.insert("fm-7", "Fujitsu FM-7");
    aliases.insert("fm7", "Fujitsu FM-7");
    aliases.insert("fujitsu - fm-7", "Fujitsu FM-7");

    // =========================================================================
    // Watara - Supervision
    // =========================================================================
    aliases.insert("supervision", "Watara Supervision");
    aliases.insert("watara - supervision", "Watara Supervision");

    // =========================================================================
    // Emerson - Arcadia 2001
    // =========================================================================
    aliases.insert("arcadia 2001", "Emerson Arcadia 2001");
    aliases.insert("emerson - arcadia 2001", "Emerson Arcadia 2001");

    // =========================================================================
    // Bally - Astrocade
    // =========================================================================
    aliases.insert("astrocade", "Bally Astrocade");
    aliases.insert("bally - astrocade", "Bally Astrocade");

    // =========================================================================
    // SAM Coupé
    // =========================================================================
    aliases.insert("sam coupe", "SAM Coupé");
    aliases.insert("sam - coupe", "SAM Coupé");

    // =========================================================================
    // Dragon 32/64
    // =========================================================================
    aliases.insert("dragon 32", "Dragon 32/64");
    aliases.insert("dragon 64", "Dragon 32/64");
    aliases.insert("dragon - 32/64", "Dragon 32/64");

    // =========================================================================
    // Acorn - Archimedes / Electron
    // =========================================================================
    aliases.insert("archimedes", "Acorn Archimedes");
    aliases.insert("acorn - archimedes", "Acorn Archimedes");

    aliases.insert("electron", "Acorn Electron");
    aliases.insert("acorn - electron", "Acorn Electron");

    aliases.insert("atom", "Acorn Atom");
    aliases.insert("acorn - atom", "Acorn Atom");

    // =========================================================================
    // Enterprise
    // =========================================================================
    aliases.insert("enterprise 64", "Enterprise");
    aliases.insert("enterprise 128", "Enterprise");

    // =========================================================================
    // Oric - Atmos
    // =========================================================================
    aliases.insert("oric", "Oric Atmos");
    aliases.insert("oric atmos", "Oric Atmos");
    aliases.insert("oric - atmos", "Oric Atmos");

    // =========================================================================
    // Casio - PV-1000 / Loopy
    // =========================================================================
    aliases.insert("pv-1000", "Casio PV-1000");
    aliases.insert("casio - pv-1000", "Casio PV-1000");

    aliases.insert("loopy", "Casio Loopy");
    aliases.insert("casio - loopy", "Casio Loopy");

    // =========================================================================
    // Epoch - Super Cassette Vision
    // =========================================================================
    aliases.insert("super cassette vision", "Epoch Super Cassette Vision");
    aliases.insert("epoch - super cassette vision", "Epoch Super Cassette Vision");

    // =========================================================================
    // VTech - CreatiVision
    // =========================================================================
    aliases.insert("creativision", "VTech CreatiVision");
    aliases.insert("vtech - creativision", "VTech CreatiVision");

    // =========================================================================
    // Spectravideo
    // =========================================================================
    aliases.insert("sv-318", "Spectravideo");
    aliases.insert("sv-328", "Spectravideo");
    aliases.insert("spectravideo - sv-318", "Spectravideo");

    // =========================================================================
    // Sord - M5
    // =========================================================================
    aliases.insert("sord m5", "Sord M5");
    aliases.insert("m5", "Sord M5");
    aliases.insert("sord - m5", "Sord M5");

    // =========================================================================
    // Mattel - Aquarius
    // =========================================================================
    aliases.insert("aquarius", "Mattel Aquarius");
    aliases.insert("mattel - aquarius", "Mattel Aquarius");

    // =========================================================================
    // Jupiter Ace
    // =========================================================================
    aliases.insert("jupiter ace", "Jupiter Ace");

    // =========================================================================
    // Exidy - Sorcerer
    // =========================================================================
    aliases.insert("sorcerer", "Exidy Sorcerer");
    aliases.insert("exidy - sorcerer", "Exidy Sorcerer");

    // =========================================================================
    // Camputers - Lynx (not Atari)
    // =========================================================================
    aliases.insert("camputers lynx", "Camputers Lynx");
    aliases.insert("camputers - lynx", "Camputers Lynx");

    // =========================================================================
    // Mega Duck
    // =========================================================================
    aliases.insert("mega duck", "Mega Duck");
    aliases.insert("megaduck", "Mega Duck");

    // =========================================================================
    // Entex - Adventure Vision
    // =========================================================================
    aliases.insert("adventure vision", "Entex Adventure Vision");
    aliases.insert("entex - adventure vision", "Entex Adventure Vision");

    // =========================================================================
    // GamePark - GP32
    // =========================================================================
    aliases.insert("gp32", "GamePark GP32");
    aliases.insert("gamepark - gp32", "GamePark GP32");

    // =========================================================================
    // Hartung - Game Master
    // =========================================================================
    aliases.insert("game master", "Hartung Game Master");
    aliases.insert("hartung - game master", "Hartung Game Master");

    // =========================================================================
    // RCA - Studio II
    // =========================================================================
    aliases.insert("studio ii", "RCA Studio II");
    aliases.insert("rca - studio ii", "RCA Studio II");

    // =========================================================================
    // Tiger - Game.com
    // =========================================================================
    aliases.insert("game.com", "Tiger Game.com");
    aliases.insert("tiger - game.com", "Tiger Game.com");

    // =========================================================================
    // Tomy - Tutor
    // =========================================================================
    aliases.insert("tutor", "Tomy Tutor");
    aliases.insert("tomy - tutor", "Tomy Tutor");

    // =========================================================================
    // Interton - VC 4000
    // =========================================================================
    aliases.insert("vc 4000", "Interton VC 4000");
    aliases.insert("interton - vc 4000", "Interton VC 4000");

    // =========================================================================
    // Memotech - MTX512
    // =========================================================================
    aliases.insert("mtx512", "Memotech MTX512");
    aliases.insert("mtx 512", "Memotech MTX512");
    aliases.insert("memotech - mtx512", "Memotech MTX512");

    // =========================================================================
    // Epoch - Game Pocket Computer
    // =========================================================================
    aliases.insert("game pocket computer", "Epoch Game Pocket Computer");
    aliases.insert("epoch - game pocket computer", "Epoch Game Pocket Computer");

    // =========================================================================
    // Bandai - Playdia / Super Vision 8000
    // =========================================================================
    aliases.insert("playdia", "Bandai Playdia");
    aliases.insert("bandai - playdia", "Bandai Playdia");

    aliases.insert("super vision 8000", "Bandai Super Vision 8000");
    aliases.insert("bandai - super vision 8000", "Bandai Super Vision 8000");

    // =========================================================================
    // ScummVM / OpenBOR / MUGEN (game engines)
    // =========================================================================
    aliases.insert("scummvm", "ScummVM");
    aliases.insert("openbor", "OpenBOR");
    aliases.insert("mugen", "MUGEN");

    // =========================================================================
    // PICO-8 / Uzebox / Arduboy / WASM-4
    // =========================================================================
    aliases.insert("pico-8", "PICO-8");
    aliases.insert("pico8", "PICO-8");
    aliases.insert("uzebox", "Uzebox");
    aliases.insert("arduboy", "Arduboy");
    aliases.insert("wasm-4", "WASM-4");
    aliases.insert("wasm4", "WASM-4");

    // =========================================================================
    // Windows / Linux
    // =========================================================================
    aliases.insert("windows", "Windows");
    aliases.insert("pc", "Windows");
    aliases.insert("linux", "Linux");

    aliases
}

/// Normalize a platform name to canonical form
pub fn normalize_platform_name(name: &str) -> String {
    let aliases = get_platform_aliases();
    let lower = name.to_lowercase().trim().to_string();

    // Check direct alias match only - contains() check was too greedy
    // (e.g., "amstrad cpc" contains "pc" which would incorrectly match Windows)
    if let Some(&canonical) = aliases.get(lower.as_str()) {
        return canonical.to_string();
    }

    // No match - return original name cleaned up
    name.trim().to_string()
}

/// Create the unified games database schema
pub async fn create_schema(pool: &SqlitePool) -> Result<()> {
    // Platforms table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS platforms (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            -- Source-specific IDs
            launchbox_name TEXT,
            libretro_name TEXT,
            screenscraper_id INTEGER,
            openvgdb_system_id INTEGER,
            -- Metadata
            manufacturer TEXT,
            release_date TEXT,
            category TEXT,
            -- Emulator config
            retroarch_core TEXT,
            file_extensions TEXT,
            -- Search aliases (comma-separated short names like NES, SNES, etc.)
            aliases TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
    "#)
    .execute(pool)
    .await?;

    // Games table with all source IDs
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS games (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            platform_id INTEGER REFERENCES platforms(id),

            -- Source-specific IDs (keep all for cross-referencing)
            launchbox_db_id INTEGER,
            libretro_crc32 TEXT,
            libretro_md5 TEXT,
            libretro_sha1 TEXT,
            libretro_serial TEXT,
            libretro_title TEXT,  -- Original No-Intro title for libretro thumbnail lookups
            screenscraper_id INTEGER,
            igdb_id INTEGER,
            steamgriddb_id INTEGER,
            openvgdb_release_id INTEGER,
            steam_app_id INTEGER,

            -- Core metadata
            description TEXT,
            release_date TEXT,
            release_year INTEGER,
            developer TEXT,
            publisher TEXT,
            genre TEXT,

            -- Extended metadata
            players TEXT,
            rating REAL,
            rating_count INTEGER,
            esrb TEXT,
            cooperative INTEGER,
            video_url TEXT,
            wikipedia_url TEXT,
            release_type TEXT,  -- Released, Homebrew, Unlicensed, Unreleased, DLC, ROM Hack, Early Access
            notes TEXT,

            -- Platform XML extended metadata
            sort_title TEXT,
            series TEXT,
            region TEXT,
            play_mode TEXT,
            version TEXT,
            status TEXT,

            -- Import tracking
            metadata_source TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
    "#)
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_platform ON games(platform_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_title ON games(title)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_launchbox_id ON games(launchbox_db_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_crc32 ON games(libretro_crc32)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_openvgdb_id ON games(openvgdb_release_id)").execute(pool).await?;

    // Game alternate names table (for regional/alternate titles)
    // Note: No foreign key since launchbox_db_id in games is not unique
    // (multiple game variants can share the same launchbox_db_id)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS game_alternate_names (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            launchbox_db_id INTEGER NOT NULL,
            alternate_name TEXT NOT NULL,
            region TEXT
        )
    "#)
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_alt_names_db_id ON game_alternate_names(launchbox_db_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_alt_names_name ON game_alternate_names(alternate_name)").execute(pool).await?;

    Ok(())
}

/// Game record for import
#[derive(Debug, Clone, Default)]
pub struct GameRecord {
    pub title: String,
    pub platform: String,

    // Source IDs
    pub launchbox_db_id: Option<i64>,
    pub libretro_crc32: Option<String>,
    pub libretro_md5: Option<String>,
    pub libretro_sha1: Option<String>,
    pub libretro_serial: Option<String>,
    pub screenscraper_id: Option<i64>,
    pub igdb_id: Option<i64>,
    pub openvgdb_release_id: Option<i64>,
    pub steam_app_id: Option<i64>,

    // Core metadata
    pub description: Option<String>,
    pub release_date: Option<String>,
    pub release_year: Option<i32>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub players: Option<String>,
    pub rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub esrb: Option<String>,
    pub cooperative: Option<bool>,
    pub video_url: Option<String>,
    pub wikipedia_url: Option<String>,
    pub release_type: Option<String>,
    pub notes: Option<String>,

    // Platform XML extended metadata
    pub sort_title: Option<String>,
    pub series: Option<String>,
    pub region: Option<String>,
    pub play_mode: Option<String>,
    pub version: Option<String>,
    pub status: Option<String>,
}

/// Platform record for import
#[derive(Debug, Clone, Default)]
pub struct PlatformRecord {
    pub name: String,
    pub launchbox_name: Option<String>,
    pub libretro_name: Option<String>,
    pub screenscraper_id: Option<i64>,
    pub openvgdb_system_id: Option<i64>,
    pub manufacturer: Option<String>,
    pub release_date: Option<String>,
    pub category: Option<String>,
    pub retroarch_core: Option<String>,
    pub file_extensions: Option<String>,
    pub aliases: Option<String>,
}

/// Generate search aliases for a platform name
/// Returns comma-separated short names (e.g., "NES, Famicom, FC")
pub fn get_platform_search_aliases(name: &str) -> Option<String> {
    let aliases = match name {
        // Nintendo
        "Nintendo Entertainment System" => "NES, Famicom, FC, nes, famicom",
        "Super Nintendo Entertainment System" => "SNES, Super Famicom, SFC, snes, snesna",
        "Nintendo 64" => "N64, n64",
        "Nintendo GameCube" => "GC, NGC, GameCube, gc, gamecube",
        "Nintendo Game Boy" => "GB, Game Boy, gb",
        "Nintendo Game Boy Color" => "GBC, Game Boy Color, gbc",
        "Nintendo Game Boy Advance" => "GBA, Game Boy Advance, gba",
        "Nintendo DS" => "NDS, DS, nds",
        "Nintendo 3DS" => "3DS, n3ds, 3ds",
        "Nintendo Wii" => "Wii, wii",
        "Nintendo Wii U" => "Wii U, WiiU, wiiu",
        "Nintendo Switch" => "Switch, NS, switch",
        "Nintendo Virtual Boy" => "VB, Virtual Boy, virtualboy",

        // Sega
        "Sega Master System" => "SMS, Master System, mastersystem",
        "Sega Genesis" => "MD, Mega Drive, Genesis, genesis, megadrive",
        "Sega CD" => "SCD, Mega CD, Sega CD, segacd, megacd",
        "Sega 32X" => "32X, sega32x",
        "Sega Saturn" => "SS, Saturn, saturn",
        "Sega Dreamcast" => "DC, Dreamcast, dreamcast",
        "Sega Game Gear" => "GG, Game Gear, gamegear",

        // Sony
        "Sony Playstation" => "PS1, PSX, PS, PlayStation, psx",
        "Sony Playstation 2" => "PS2, PlayStation 2, ps2",
        "Sony Playstation 3" => "PS3, PlayStation 3, ps3",
        "Sony PSP" => "PSP, PlayStation Portable, psp",
        "Sony Playstation Vita" => "PSV, Vita, PS Vita, psvita",

        // NEC
        "NEC TurboGrafx-16" => "PCE, PC Engine, TG16, TurboGrafx-16, tg16, pcengine",
        "NEC TurboGrafx-CD" => "PCECD, PC Engine CD, TG-CD, TurboGrafx-CD, tg-cd, pcenginecd",
        "NEC PC-98" => "PC98, PC-98, pc98",

        // SNK
        "SNK Neo Geo Pocket" => "NGP, Neo Geo Pocket, ngp",
        "SNK Neo Geo Pocket Color" => "NGPC, Neo Geo Pocket Color, ngpc",
        "SNK Neo Geo AES" => "AES, MVS, Neo Geo, neogeo",
        "SNK Neo Geo CD" => "Neo Geo CD, neogeocd, neogeocdjp",

        // Atari
        "Atari 2600" => "2600, VCS, atari2600",
        "Atari 5200" => "5200, atari5200",
        "Atari 7800" => "7800, atari7800",
        "Atari Lynx" => "Lynx, lynx",
        "Atari Jaguar" => "Jaguar, Jag, atarijaguar",
        "Atari Jaguar CD" => "Jaguar CD, atarijaguarcd",

        // Commodore
        "Commodore 64" => "C64, c64",
        "Commodore Amiga" => "Amiga, amiga",
        "Commodore VIC-20" => "VIC-20, VIC20, vic20",
        "Commodore 16" => "C16, c16",

        // Other
        "MS-DOS" => "DOS, dos",
        "Microsoft MSX" => "MSX, msx",
        "Microsoft MSX2" => "MSX2, msx2",
        "Microsoft Xbox" => "Xbox, xbox",
        "Microsoft Xbox 360" => "X360, 360, Xbox 360, xbox360",
        "Sinclair ZX Spectrum" => "ZX, ZX Spectrum, zxspectrum",
        "Amstrad CPC" => "CPC, amstradcpc",
        "Arcade" => "MAME, arcade, fbneo",
        "Panasonic 3DO" => "3DO, 3do",
        "Philips CD-i" => "CD-i, CDi, cdimono1",
        "Bandai WonderSwan" => "WS, WonderSwan, wonderswan",
        "Bandai WonderSwan Color" => "WSC, WonderSwan Color, wonderswancolor",
        "Coleco ColecoVision" => "Coleco, ColecoVision, colecovision",
        "Mattel Intellivision" => "Intellivision, intellivision",
        "GCE Vectrex" => "Vectrex, vectrex",
        "Sharp X68000" => "X68000, x68000",
        "ScummVM" => "ScummVM, scummvm",

        _ => return None,
    };
    Some(aliases.to_string())
}

/// Unified importer that handles multiple sources
pub struct UnifiedImporter {
    pool: SqlitePool,
    platform_cache: HashMap<String, i64>, // canonical name -> id
}

impl UnifiedImporter {
    /// Create a new importer with an output database
    pub async fn new(output_path: &Path) -> Result<Self> {
        // Remove existing DB if present
        if output_path.exists() {
            std::fs::remove_file(output_path)?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", output_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        create_schema(&pool).await?;

        Ok(Self {
            pool,
            platform_cache: HashMap::new(),
        })
    }

    /// Open an existing database for enrichment
    pub async fn open(db_path: &Path) -> Result<Self> {
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&db_url).await?;

        // Load existing platforms into cache
        let mut platform_cache = HashMap::new();
        let rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM platforms")
            .fetch_all(&pool)
            .await?;
        for (id, name) in rows {
            let canonical = normalize_platform_name(&name);
            platform_cache.insert(canonical, id);
        }

        Ok(Self { pool, platform_cache })
    }

    /// Get or create a platform, returns platform ID
    pub async fn get_or_create_platform(&mut self, record: &PlatformRecord) -> Result<i64> {
        let canonical = normalize_platform_name(&record.name);

        // Generate aliases if not provided
        let aliases = record.aliases.clone().or_else(|| get_platform_search_aliases(&canonical));

        // Check cache first
        if let Some(&id) = self.platform_cache.get(&canonical) {
            // Update with any new source IDs (don't overwrite existing)
            sqlx::query(r#"
                UPDATE platforms SET
                    launchbox_name = COALESCE(launchbox_name, ?),
                    libretro_name = COALESCE(libretro_name, ?),
                    screenscraper_id = COALESCE(screenscraper_id, ?),
                    openvgdb_system_id = COALESCE(openvgdb_system_id, ?),
                    manufacturer = COALESCE(manufacturer, ?),
                    category = COALESCE(category, ?),
                    retroarch_core = COALESCE(retroarch_core, ?),
                    file_extensions = COALESCE(file_extensions, ?),
                    aliases = COALESCE(aliases, ?)
                WHERE id = ?
            "#)
            .bind(&record.launchbox_name)
            .bind(&record.libretro_name)
            .bind(record.screenscraper_id)
            .bind(record.openvgdb_system_id)
            .bind(&record.manufacturer)
            .bind(&record.category)
            .bind(&record.retroarch_core)
            .bind(&record.file_extensions)
            .bind(&aliases)
            .bind(id)
            .execute(&self.pool)
            .await?;

            return Ok(id);
        }

        // Create new platform
        let id: i64 = sqlx::query_scalar(r#"
            INSERT INTO platforms (name, launchbox_name, libretro_name, screenscraper_id,
                                   openvgdb_system_id, manufacturer, category, retroarch_core, file_extensions, aliases)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
        "#)
        .bind(&canonical)
        .bind(&record.launchbox_name)
        .bind(&record.libretro_name)
        .bind(record.screenscraper_id)
        .bind(record.openvgdb_system_id)
        .bind(&record.manufacturer)
        .bind(&record.category)
        .bind(&record.retroarch_core)
        .bind(&record.file_extensions)
        .bind(&aliases)
        .fetch_one(&self.pool)
        .await?;

        self.platform_cache.insert(canonical, id);
        Ok(id)
    }

    /// Try to find an existing game that matches
    /// Match priority: launchbox_db_id > CRC > normalized title + platform
    async fn find_existing_game(&self, record: &GameRecord, platform_id: i64) -> Result<Option<String>> {
        // Match by LaunchBox ID
        if let Some(lb_id) = record.launchbox_db_id {
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM games WHERE launchbox_db_id = ?"
            )
            .bind(lb_id)
            .fetch_optional(&self.pool)
            .await?;
            if let Some((id,)) = existing {
                return Ok(Some(id));
            }
        }

        // Match by CRC32
        if let Some(ref crc) = record.libretro_crc32 {
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM games WHERE libretro_crc32 = ? AND platform_id = ?"
            )
            .bind(crc.to_uppercase())
            .bind(platform_id)
            .fetch_optional(&self.pool)
            .await?;
            if let Some((id,)) = existing {
                return Ok(Some(id));
            }
        }

        // Match by normalized title + platform
        let normalized = normalize_title(&record.title);
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, title FROM games WHERE platform_id = ?"
        )
        .bind(platform_id)
        .fetch_all(&self.pool)
        .await?;

        for (id, existing_title) in rows {
            let existing_normalized = normalize_title(&existing_title);
            let sim = similarity_ratio(&normalized, &existing_normalized);
            if sim >= 0.95 {
                return Ok(Some(id));
            }
        }

        Ok(None)
    }

    /// Import a game record - creates new or updates existing
    /// Only fills empty fields (preserves earlier, higher-quality data)
    pub async fn import_game(&mut self, record: GameRecord, source: &str) -> Result<String> {
        let platform_record = PlatformRecord {
            name: record.platform.clone(),
            ..Default::default()
        };
        let platform_id = self.get_or_create_platform(&platform_record).await?;

        // Check for existing game
        if let Some(existing_id) = self.find_existing_game(&record, platform_id).await? {
            // Update existing - only fill empty fields
            sqlx::query(r#"
                UPDATE games SET
                    launchbox_db_id = COALESCE(launchbox_db_id, ?),
                    libretro_crc32 = COALESCE(libretro_crc32, ?),
                    libretro_md5 = COALESCE(libretro_md5, ?),
                    libretro_sha1 = COALESCE(libretro_sha1, ?),
                    libretro_serial = COALESCE(libretro_serial, ?),
                    screenscraper_id = COALESCE(screenscraper_id, ?),
                    igdb_id = COALESCE(igdb_id, ?),
                    openvgdb_release_id = COALESCE(openvgdb_release_id, ?),
                    steam_app_id = COALESCE(steam_app_id, ?),
                    description = COALESCE(description, ?),
                    release_date = COALESCE(release_date, ?),
                    release_year = COALESCE(release_year, ?),
                    developer = COALESCE(developer, ?),
                    publisher = COALESCE(publisher, ?),
                    genre = COALESCE(genre, ?),
                    players = COALESCE(players, ?),
                    rating = COALESCE(rating, ?),
                    rating_count = COALESCE(rating_count, ?),
                    esrb = COALESCE(esrb, ?),
                    cooperative = COALESCE(cooperative, ?),
                    video_url = COALESCE(video_url, ?),
                    wikipedia_url = COALESCE(wikipedia_url, ?),
                    release_type = COALESCE(release_type, ?),
                    notes = COALESCE(notes, ?),
                    sort_title = COALESCE(sort_title, ?),
                    series = COALESCE(series, ?),
                    region = COALESCE(region, ?),
                    play_mode = COALESCE(play_mode, ?),
                    version = COALESCE(version, ?),
                    status = COALESCE(status, ?),
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = ?
            "#)
            .bind(record.launchbox_db_id)
            .bind(record.libretro_crc32.as_ref().map(|s| s.to_uppercase()))
            .bind(&record.libretro_md5)
            .bind(&record.libretro_sha1)
            .bind(&record.libretro_serial)
            .bind(record.screenscraper_id)
            .bind(record.igdb_id)
            .bind(record.openvgdb_release_id)
            .bind(record.steam_app_id)
            .bind(&record.description)
            .bind(&record.release_date)
            .bind(record.release_year)
            .bind(&record.developer)
            .bind(&record.publisher)
            .bind(&record.genre)
            .bind(&record.players)
            .bind(record.rating)
            .bind(record.rating_count)
            .bind(&record.esrb)
            .bind(record.cooperative.map(|b| if b { 1 } else { 0 }))
            .bind(&record.video_url)
            .bind(&record.wikipedia_url)
            .bind(&record.release_type)
            .bind(&record.notes)
            .bind(&record.sort_title)
            .bind(&record.series)
            .bind(&record.region)
            .bind(&record.play_mode)
            .bind(&record.version)
            .bind(&record.status)
            .bind(&existing_id)
            .execute(&self.pool)
            .await?;

            Ok(existing_id)
        } else {
            // Create new game
            let game_id = Uuid::new_v4().to_string();

            sqlx::query(r#"
                INSERT INTO games (
                    id, title, platform_id,
                    launchbox_db_id, libretro_crc32, libretro_md5, libretro_sha1, libretro_serial,
                    screenscraper_id, igdb_id, openvgdb_release_id, steam_app_id,
                    description, release_date, release_year, developer, publisher, genre,
                    players, rating, rating_count, esrb, cooperative, video_url, wikipedia_url,
                    release_type, notes, sort_title, series, region, play_mode, version, status,
                    metadata_source
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&game_id)
            .bind(&record.title)
            .bind(platform_id)
            .bind(record.launchbox_db_id)
            .bind(record.libretro_crc32.as_ref().map(|s| s.to_uppercase()))
            .bind(&record.libretro_md5)
            .bind(&record.libretro_sha1)
            .bind(&record.libretro_serial)
            .bind(record.screenscraper_id)
            .bind(record.igdb_id)
            .bind(record.openvgdb_release_id)
            .bind(record.steam_app_id)
            .bind(&record.description)
            .bind(&record.release_date)
            .bind(record.release_year)
            .bind(&record.developer)
            .bind(&record.publisher)
            .bind(&record.genre)
            .bind(&record.players)
            .bind(record.rating)
            .bind(record.rating_count)
            .bind(&record.esrb)
            .bind(record.cooperative.map(|b| if b { 1 } else { 0 }))
            .bind(&record.video_url)
            .bind(&record.wikipedia_url)
            .bind(&record.release_type)
            .bind(&record.notes)
            .bind(&record.sort_title)
            .bind(&record.series)
            .bind(&record.region)
            .bind(&record.play_mode)
            .bind(&record.version)
            .bind(&record.status)
            .bind(source)
            .execute(&self.pool)
            .await?;

            Ok(game_id)
        }
    }

    /// Batch import LaunchBox games (fast path for initial import)
    /// Uses transactions for much better performance
    pub async fn import_launchbox_games_batch(
        &mut self,
        games: &[crate::launchbox::LaunchBoxGame],
        pb: &ProgressBar,
    ) -> Result<usize> {
        let mut imported = 0;
        let mut skipped_dupes = 0;
        let batch_size = 1000;

        // Build dedup cache: (platform_id, normalized_title) -> game_id
        // This prevents inserting games that only differ by punctuation
        let mut dedup_cache: HashMap<(i64, String), String> = HashMap::new();

        for chunk in games.chunks(batch_size) {
            let mut tx = self.pool.begin().await?;

            for game in chunk {
                // Get or create platform
                let platform_canonical = normalize_platform_name(&game.platform);
                let platform_id = if let Some(&id) = self.platform_cache.get(&platform_canonical) {
                    // Update launchbox_name if not already set (in case platform was created by another source)
                    sqlx::query(
                        "UPDATE platforms SET launchbox_name = COALESCE(launchbox_name, ?) WHERE id = ?"
                    )
                    .bind(&game.platform)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                    id
                } else {
                    let id: i64 = sqlx::query_scalar(
                        "INSERT INTO platforms (name, launchbox_name) VALUES (?, ?) ON CONFLICT(name) DO UPDATE SET launchbox_name = COALESCE(platforms.launchbox_name, excluded.launchbox_name) RETURNING id"
                    )
                    .bind(&platform_canonical)
                    .bind(&game.platform)
                    .fetch_one(&mut *tx)
                    .await?;
                    self.platform_cache.insert(platform_canonical.clone(), id);
                    id
                };

                // Normalize title for deduplication (removes punctuation, lowercases)
                let normalized = normalize_title(&game.name);
                let dedup_key = (platform_id, normalized.clone());

                // Check if we already have this game (by normalized title + platform)
                if dedup_cache.contains_key(&dedup_key) {
                    skipped_dupes += 1;
                    continue;
                }

                let game_id = uuid::Uuid::new_v4().to_string();
                dedup_cache.insert(dedup_key, game_id.clone());

                sqlx::query(r#"
                    INSERT INTO games (
                        id, title, platform_id, launchbox_db_id,
                        description, release_date, release_year, developer, publisher, genre,
                        players, rating, rating_count, esrb, cooperative, video_url, wikipedia_url,
                        release_type, steam_app_id, notes, metadata_source
                    ) VALUES (
                        ?, ?, ?, ?,
                        ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?
                    )
                "#)
                .bind(&game_id)
                .bind(&game.name)
                .bind(platform_id)
                .bind(game.database_id)
                .bind(&game.overview)
                .bind(&game.release_date)
                .bind(game.release_year)
                .bind(&game.developer)
                .bind(&game.publisher)
                .bind(&game.genres)
                .bind(&game.max_players)
                .bind(game.rating)
                .bind(game.rating_count)
                .bind(&game.esrb)
                .bind(game.cooperative.map(|b| if b { 1 } else { 0 }))
                .bind(&game.video_url)
                .bind(&game.wikipedia_url)
                .bind(&game.release_type)
                .bind(game.steam_app_id)
                .bind(&game.notes)
                .bind("launchbox")
                .execute(&mut *tx)
                .await?;

                imported += 1;
            }

            tx.commit().await?;
            pb.inc(chunk.len() as u64);
        }

        if skipped_dupes > 0 {
            println!("  Skipped {} duplicates (same normalized title + platform)", skipped_dupes);
        }

        Ok(imported)
    }

    /// Batch import alternate names (fast path)
    pub async fn import_alternate_names_batch(
        &self,
        alt_names: &[crate::launchbox::GameAlternateName],
        pb: &ProgressBar,
    ) -> Result<usize> {
        let mut imported = 0;
        let batch_size = 5000;

        for chunk in alt_names.chunks(batch_size) {
            let mut tx = self.pool.begin().await?;

            for alt in chunk {
                sqlx::query(r#"
                    INSERT INTO game_alternate_names (launchbox_db_id, alternate_name, region)
                    VALUES (?, ?, ?)
                "#)
                .bind(alt.database_id)
                .bind(&alt.alternate_name)
                .bind(&alt.region)
                .execute(&mut *tx)
                .await?;

                imported += 1;
            }

            tx.commit().await?;
            pb.inc(chunk.len() as u64);
        }

        Ok(imported)
    }

    /// Batch import LibRetro games for a single platform
    /// Uses transaction for speed, matches by CRC or normalized title, or inserts new
    pub async fn import_libretro_games_batch(
        &mut self,
        platform_name: &str,
        games: &[lunchbox_core::import::DatGame],
    ) -> Result<usize> {
        let platform_canonical = normalize_platform_name(platform_name);

        // Get or create platform in transaction
        let mut tx = self.pool.begin().await?;

        let platform_id = if let Some(&id) = self.platform_cache.get(&platform_canonical) {
            id
        } else {
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO platforms (name, libretro_name) VALUES (?, ?) ON CONFLICT(name) DO UPDATE SET libretro_name=COALESCE(libretro_name, ?) RETURNING id"
            )
            .bind(&platform_canonical)
            .bind(platform_name)
            .bind(platform_name)
            .fetch_one(&mut *tx)
            .await?;
            self.platform_cache.insert(platform_canonical.clone(), id);
            id
        };

        let mut imported = 0;
        let mut skipped_dupes = 0;
        let mut merged_with_launchbox = 0;

        // Pre-load existing LaunchBox games for this platform: normalized_title -> (id, title)
        let existing_games: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, title FROM games WHERE platform_id = ? AND metadata_source = 'launchbox'"
        )
        .bind(platform_id)
        .fetch_all(&mut *tx)
        .await?;

        let launchbox_by_normalized: HashMap<String, (String, String)> = existing_games
            .into_iter()
            .map(|(id, title)| (normalize_title(&title), (id, title)))
            .collect();

        // Group LibRetro games by normalized title to detect multiple variants
        let mut libretro_by_normalized: HashMap<String, Vec<&lunchbox_core::import::DatGame>> = HashMap::new();
        for game in games {
            let normalized = normalize_title(&game.name);
            libretro_by_normalized.entry(normalized).or_default().push(game);
        }

        // Dedup cache for this import run
        let mut dedup_cache: HashMap<String, String> = HashMap::new();

        for game in games {
            let primary_rom = game.roms.first();
            let crc = primary_rom.and_then(|r| r.crc.as_ref()).map(|c| c.to_uppercase());
            let normalized = normalize_title(&game.name);

            // Try to find existing game by CRC first
            let existing_by_crc: Option<(String,)> = if let Some(ref crc_val) = crc {
                sqlx::query_as("SELECT id FROM games WHERE libretro_crc32 = ? AND platform_id = ?")
                    .bind(crc_val)
                    .bind(platform_id)
                    .fetch_optional(&mut *tx)
                    .await?
            } else {
                None
            };

            if let Some((id,)) = existing_by_crc {
                // Update existing with any new data (CRC match)
                sqlx::query(r#"
                    UPDATE games SET
                        libretro_md5 = COALESCE(libretro_md5, ?),
                        libretro_sha1 = COALESCE(libretro_sha1, ?),
                        libretro_serial = COALESCE(libretro_serial, ?),
                        libretro_title = COALESCE(libretro_title, ?),
                        release_year = COALESCE(release_year, ?),
                        developer = COALESCE(developer, ?),
                        publisher = COALESCE(publisher, ?),
                        genre = COALESCE(genre, ?),
                        updated_at = CURRENT_TIMESTAMP
                    WHERE id = ?
                "#)
                .bind(primary_rom.and_then(|r| r.md5.as_ref()))
                .bind(primary_rom.and_then(|r| r.sha1.as_ref()))
                .bind(&game.serial)
                .bind(&game.name)
                .bind(game.release_year.map(|y| y as i32))
                .bind(&game.developer)
                .bind(&game.publisher)
                .bind(&game.genre)
                .bind(&id)
                .execute(&mut *tx)
                .await?;
                imported += 1;
                continue;
            }

            // Check if LaunchBox has this game and if this is the ONLY LibRetro variant
            let libretro_variants = libretro_by_normalized.get(&normalized).map(|v| v.len()).unwrap_or(0);

            if let Some((launchbox_id, _)) = launchbox_by_normalized.get(&normalized) {
                if libretro_variants == 1 {
                    // Single LibRetro entry matching a LaunchBox entry = same game, merge
                    sqlx::query(r#"
                        UPDATE games SET
                            libretro_crc32 = COALESCE(?, libretro_crc32),
                            libretro_md5 = COALESCE(?, libretro_md5),
                            libretro_sha1 = COALESCE(?, libretro_sha1),
                            libretro_serial = COALESCE(?, libretro_serial),
                            libretro_title = COALESCE(libretro_title, ?),
                            release_year = COALESCE(release_year, ?),
                            developer = COALESCE(developer, ?),
                            publisher = COALESCE(publisher, ?),
                            genre = COALESCE(genre, ?),
                            updated_at = CURRENT_TIMESTAMP
                        WHERE id = ?
                    "#)
                    .bind(&crc)
                    .bind(primary_rom.and_then(|r| r.md5.as_ref()))
                    .bind(primary_rom.and_then(|r| r.sha1.as_ref()))
                    .bind(&game.serial)
                    .bind(&game.name)
                    .bind(game.release_year.map(|y| y as i32))
                    .bind(&game.developer)
                    .bind(&game.publisher)
                    .bind(&game.genre)
                    .bind(launchbox_id)
                    .execute(&mut *tx)
                    .await?;
                    merged_with_launchbox += 1;
                    continue;
                }
                // Multiple LibRetro variants = real regional variants, keep them separate
            }

            // Check dedup cache by full title (within this LibRetro import only)
            // Use full title so regional variants like (USA), (Europe) are kept separate
            if dedup_cache.contains_key(&game.name) {
                skipped_dupes += 1;
                continue;
            }

            // Insert new game
            let game_id = uuid::Uuid::new_v4().to_string();
            dedup_cache.insert(game.name.clone(), game_id.clone());

            sqlx::query(r#"
                INSERT INTO games (
                    id, title, platform_id,
                    libretro_crc32, libretro_md5, libretro_sha1, libretro_serial, libretro_title,
                    release_year, developer, publisher, genre, metadata_source
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&game_id)
            .bind(&game.name)
            .bind(platform_id)
            .bind(&crc)
            .bind(primary_rom.and_then(|r| r.md5.as_ref()))
            .bind(primary_rom.and_then(|r| r.sha1.as_ref()))
            .bind(&game.serial)
            .bind(&game.name)  // libretro_title = original libretro name
            .bind(game.release_year.map(|y| y as i32))
            .bind(&game.developer)
            .bind(&game.publisher)
            .bind(&game.genre)
            .bind("libretro")
            .execute(&mut *tx)
            .await?;
            imported += 1;
        }

        tx.commit().await?;

        if skipped_dupes > 0 || merged_with_launchbox > 0 {
            if skipped_dupes > 0 {
                println!("    Skipped {} duplicates", skipped_dupes);
            }
            if merged_with_launchbox > 0 {
                println!("    Merged {} with LaunchBox entries", merged_with_launchbox);
            }
        }

        Ok(imported)
    }

    /// Get statistics about the database
    pub async fn get_stats(&self) -> Result<(i64, i64, i64)> {
        let (platforms,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM platforms")
            .fetch_one(&self.pool)
            .await?;
        let (games,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM games")
            .fetch_one(&self.pool)
            .await?;
        let (alt_names,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM game_alternate_names")
            .fetch_one(&self.pool)
            .await?;
        Ok((platforms, games, alt_names))
    }

    /// Close the database connection
    pub async fn close(self) {
        self.pool.close().await;
    }
}

/// Progress bar helper
fn create_progress_bar(len: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(msg.to_string());
    pb
}

/// Build unified database from multiple sources
/// Import order: LaunchBox (best) -> LibRetro (checksums) -> OpenVGDB (enrichment)
pub async fn build_unified_database(
    output: &Path,
    launchbox_xml: Option<&Path>,
    libretro_path: Option<&Path>,
    openvgdb_path: Option<&Path>,
    threshold: f64,
) -> Result<()> {
    use crate::launchbox;
    use crate::enrich;
    use lunchbox_core::import::{parse_dat_file, merge_dat_files, DatFile};

    println!("Unified Game Database Builder");
    println!("==============================");
    println!("Output: {}", output.display());
    println!();
    println!("Import order (best quality first):");
    println!("  1. LaunchBox: {}", launchbox_xml.map(|p| p.display().to_string()).unwrap_or_else(|| "not provided".into()));
    println!("  2. LibRetro:  {}", libretro_path.map(|p| p.display().to_string()).unwrap_or_else(|| "not provided".into()));
    println!("  3. OpenVGDB:  {}", openvgdb_path.map(|p| p.display().to_string()).unwrap_or_else(|| "not provided".into()));
    println!();

    let mut importer = UnifiedImporter::new(output).await?;

    // Phase 1: Import LaunchBox (best metadata)
    if let Some(xml_path) = launchbox_xml {
        if !xml_path.exists() {
            println!("Warning: LaunchBox XML not found: {}", xml_path.display());
        } else {
            println!("Phase 1: Importing LaunchBox metadata...");
            let games = launchbox::parse_launchbox_metadata(xml_path)?;
            println!("  Parsed {} games", games.len());

            let pb = create_progress_bar(games.len() as u64, "Importing games");
            let imported = importer.import_launchbox_games_batch(&games, &pb).await?;
            pb.finish_with_message("Done");
            println!("  Imported {} games from LaunchBox", imported);

            // Import alternate names
            println!("  Parsing alternate names...");
            let alt_names = launchbox::parse_alternate_names(xml_path)?;
            println!("  Parsed {} alternate names", alt_names.len());

            let pb = create_progress_bar(alt_names.len() as u64, "Importing alternate names");
            let alt_imported = importer.import_alternate_names_batch(&alt_names, &pb).await?;
            pb.finish_with_message("Done");
            println!("  Imported {} alternate names", alt_imported);
            println!();
        }
    }

    // Phase 2: Import LibRetro (adds checksums, fills gaps)
    if let Some(lr_path) = libretro_path {
        if !lr_path.exists() {
            println!("Warning: LibRetro database not found: {}", lr_path.display());
        } else {
            println!("Phase 2: Importing LibRetro DAT files...");

            let metadat_path = lr_path.join("metadat");
            if !metadat_path.exists() {
                println!("Warning: metadat directory not found in LibRetro path");
            } else {
                let mut dat_files: Vec<std::path::PathBuf> = Vec::new();

                for subdir in &["no-intro", "redump"] {
                    let subdir_path = metadat_path.join(subdir);
                    if subdir_path.exists() {
                        for entry in walkdir::WalkDir::new(&subdir_path)
                            .max_depth(1)
                            .into_iter()
                            .filter_map(|e| e.ok())
                        {
                            let path = entry.path();
                            if path.extension().map(|e| e == "dat").unwrap_or(false) {
                                dat_files.push(path.to_path_buf());
                            }
                        }
                    }
                }

                println!("  Found {} DAT files", dat_files.len());

                let developer_path = metadat_path.join("developer");
                let publisher_path = metadat_path.join("publisher");
                let genre_path = metadat_path.join("genre");
                let releaseyear_path = metadat_path.join("releaseyear");

                let pb = create_progress_bar(dat_files.len() as u64, "Processing DAT files");
                let mut total_imported = 0;

                for dat_path in &dat_files {
                    let platform_name = dat_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown");
                    pb.set_message(platform_name.to_string());

                    let base_dat = match parse_dat_file(dat_path) {
                        Ok(dat) => dat,
                        Err(e) => {
                            pb.println(format!("  Error parsing {}: {}", platform_name, e));
                            pb.inc(1);
                            continue;
                        }
                    };

                    let mut supplements: Vec<DatFile> = Vec::new();
                    for supp_path in [&developer_path, &publisher_path, &genre_path, &releaseyear_path] {
                        let supp_file = supp_path.join(format!("{}.dat", platform_name));
                        if supp_file.exists() {
                            if let Ok(supp_dat) = parse_dat_file(&supp_file) {
                                supplements.push(supp_dat);
                            }
                        }
                    }

                    let merged = if supplements.is_empty() { base_dat } else { merge_dat_files(base_dat, supplements) };

                    // Batch import all games for this platform
                    let imported = importer.import_libretro_games_batch(platform_name, &merged.games).await?;
                    total_imported += imported;
                    pb.inc(1);
                }

                pb.finish_with_message("Done");
                println!("  Processed {} games from LibRetro", total_imported);
                println!();
            }
        }
    }

    // Phase 3: Enrich with OpenVGDB
    if let Some(ovgdb_path) = openvgdb_path {
        if !ovgdb_path.exists() {
            println!("Warning: OpenVGDB not found: {}", ovgdb_path.display());
        } else {
            println!("Phase 3: Enriching with OpenVGDB...");
            println!("  (Using threshold: {:.0}%)", threshold * 100.0);
            importer.close().await;
            enrich::enrich_database(output, ovgdb_path, threshold, false).await?;
            println!();

            // Reopen for stats
            let db_url = format!("sqlite:{}?mode=ro", output.display());
            let pool = SqlitePool::connect(&db_url).await?;
            let (platforms,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM platforms").fetch_one(&pool).await?;
            let (games,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM games").fetch_one(&pool).await?;
            let (alt_names,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM game_alternate_names").fetch_one(&pool).await?;
            pool.close().await;

            println!("Build complete!");
            println!("  Platforms:       {}", platforms);
            println!("  Games:           {}", games);
            println!("  Alternate names: {}", alt_names);

            // Compress the database
            compress_database(output)?;

            return Ok(());
        }
    }

    let (platforms, games, alt_names) = importer.get_stats().await?;
    importer.close().await;

    println!("Build complete!");
    println!("  Platforms:       {}", platforms);
    println!("  Games:           {}", games);
    println!("  Alternate names: {}", alt_names);
    println!("  Output:          {}", output.display());

    // Compress the database
    compress_database(output)?;

    Ok(())
}

/// Compress the database with zstd ultra compression
fn compress_database(db_path: &Path) -> Result<()> {
    use std::fs::File;
    use std::io::{BufReader, BufWriter};

    let compressed_path = db_path.with_extension("db.zst");

    println!();
    println!("Compressing database...");

    let input_file = File::open(db_path)?;
    let input_size = input_file.metadata()?.len();
    let reader = BufReader::new(input_file);

    let output_file = File::create(&compressed_path)?;
    let writer = BufWriter::new(output_file);

    // Use compression level 22 (ultra) for maximum compression
    let mut encoder = zstd::Encoder::new(writer, 22)?;
    encoder.long_distance_matching(true)?;
    std::io::copy(&mut BufReader::new(reader), &mut encoder)?;
    encoder.finish()?;

    let compressed_size = std::fs::metadata(&compressed_path)?.len();
    let ratio = (compressed_size as f64 / input_size as f64) * 100.0;

    println!("  Original:   {} MB", input_size / 1024 / 1024);
    println!("  Compressed: {} MB ({:.1}%)", compressed_size / 1024 / 1024, ratio);
    println!("  Output:     {}", compressed_path.display());

    Ok(())
}
