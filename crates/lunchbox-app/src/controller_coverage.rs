//! Coverage is derived from discovery relationships and implemented contracts.
//! It is not an installation, runtime-verification, or all-modes readiness claim.
use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::controller_catalog::{Catalog, catalog};
use crate::emulator::canonical_retroarch_core_name;

// Source dispatch presence only. Each entry still has host/mode restrictions;
// neither this list nor its percentage is a runtime acceptance result.
const NATIVE_ADAPTERS: &[&str] = &[
    "ares",
    "BizHawk",
    "DuckStation",
    "PPSSPP",
    "mGBA",
    "Dolphin",
    "Snes9x",
    "FCEUX",
    "SameBoy",
    "Mednafen",
    "MAME",
    "Flycast",
    "PCSX2",
    "RPCS3",
    "melonDS",
    "bsnes",
    "Stella",
    "VICE",
    "Hatari",
    "DeSmuME",
    "openMSX",
    "Mesen",
    "BlastEm",
    "xemu",
];

pub fn report(selections: &HashMap<String, String>) -> Result<Value> {
    let path = crate::catalog::requested_database_path().context("No emulator database found")?;
    let connection = crate::catalog::open_read_only(&path, "Controller coverage")?;
    let mut statement = connection.prepare(
        "SELECT e.id, e.name, p.canonical_name, ep.core_name
         FROM emulator_platforms ep
         JOIN emulators e ON e.id=ep.emulator_id
         JOIN platforms p ON p.id=ep.platform_id
         ORDER BY e.name COLLATE NOCASE, p.canonical_name COLLATE NOCASE",
    )?;
    let relationships = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let catalog = catalog();
    let mut cores = BTreeMap::<String, BTreeSet<String>>::new();
    let mut core_owners = BTreeMap::<(String, String), BTreeSet<String>>::new();
    let mut standalone = BTreeMap::<String, (String, BTreeSet<String>)>::new();
    let mut platforms = BTreeSet::new();
    let mut emulators = BTreeSet::new();
    for (id, name, platform, names) in &relationships {
        platforms.insert(platform);
        emulators.insert(id);
        // A libretro alternative does not remove the native emulator candidate.
        // Discovery considers both independently; mirror that inventory here.
        standalone
            .entry(id.clone())
            .or_insert_with(|| (name.clone(), BTreeSet::new()))
            .1
            .insert(platform.clone());
        if !names.trim().is_empty() {
            for core in names
                .split(';')
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                core_owners
                    .entry((
                        canonical_retroarch_core_name(core).to_owned(),
                        platform.clone(),
                    ))
                    .or_default()
                    .insert(name.clone());
                cores
                    .entry(canonical_retroarch_core_name(core).to_owned())
                    .or_default()
                    .insert(platform.clone());
            }
        }
    }
    // BizHawk-owned core names are not libretro contracts. Keep their rows,
    // but never put them in a RetroArch completion denominator.
    let retroarch_cores: BTreeSet<_> = core_owners
        .iter()
        .filter(|(_, owners)| {
            owners
                .iter()
                .any(|name| !name.eq_ignore_ascii_case("BizHawk"))
        })
        .map(|((core, _), _)| core.as_str())
        .collect();
    let dynamic_cores: Vec<_> = ["mame", "fbneo"]
        .into_iter()
        .filter(|core| retroarch_cores.contains(core))
        .collect();
    let enabled_cores: BTreeSet<_> = catalog
        .emulator_profiles
        .iter()
        .filter(|p| p.retroarch_launch.is_some())
        .map(|p| canonical_retroarch_core_name(&p.core))
        .collect();
    let covered = cores
        .keys()
        .filter(|core| {
            enabled_cores.contains(core.as_str())
                && core_owners.iter().any(|((owner_core, _), names)| {
                    owner_core == *core
                        && names
                            .iter()
                            .any(|name| !name.eq_ignore_ascii_case("BizHawk"))
                })
        })
        .count();
    let automatic_cores: BTreeSet<_> = catalog
        .emulator_profiles
        .iter()
        .filter(|profile| profile.retroarch_launch.is_some() && !profile.explicit_selection)
        .map(|profile| canonical_retroarch_core_name(&profile.core))
        .collect();
    let automatic_covered = cores
        .keys()
        .filter(|core| {
            automatic_cores.contains(core.as_str())
                && core_owners.iter().any(|((owner_core, _), names)| {
                    owner_core == *core
                        && names
                            .iter()
                            .any(|name| !name.eq_ignore_ascii_case("BizHawk"))
                })
        })
        .count();
    let mut rows = Vec::new();
    let mut core_platform_count = 0usize;
    let mut core_platforms_with_contracts = 0usize;
    for (core, platforms) in &cores {
        let profiles: Vec<_> = catalog
            .emulator_profiles
            .iter()
            .filter(|p| {
                canonical_retroarch_core_name(&p.core) == core.as_str() && p.transport == "retropad"
            })
            .collect();
        for platform in platforms {
            let owners = core_owners.get(&(core.clone(), platform.clone()));
            let bizhawk_only = owners.is_some_and(|names| {
                !names.is_empty()
                    && names
                        .iter()
                        .all(|name| name.eq_ignore_ascii_case("BizHawk"))
            });
            if bizhawk_only {
                // A nonempty core_name is not proof of a libretro ABI. Keep the
                // discovered core in the backlog without offering RetroArch modes.
                if core == "nymashock"
                    && [
                        "Sony Playstation",
                        "PlayStation",
                        "Sony PlayStation 1",
                        "PSX",
                    ]
                    .iter()
                    .any(|name| platform.trim().eq_ignore_ascii_case(name))
                {
                    rows.push(json!({"name":core,"platform":platform,"kind":"BizHawk core",
                        "status":"in_progress","profiles":[],"choices":[],"selected":null,
                        "native_setup":true,
                        "detail":"Native configuration and launch code is connected, but the full Nymashock contract is not yet accepted or counted. This is not a RetroArch profile or a tested runtime result.",
                        "implemented":["Digital pad, DualShock, Dual Analog, Analog Joystick, neGcon and rhythm modes",
                            "Controller-driven mouse, GunCon and Justifier modes with explicit offscreen controls",
                            "Explicit multitaps, mode toggles and independent measured pressure axes",
                            "Classic/evdev physical translation and measured SDL2 logical bindings",
                            "Session-normalized sticks, owned virtual-device lifetime and health checks",
                            "Native runtime/player editor and opt-in direct Mono environment"],
                        "remaining":["Physical mouse-delta and lightgun capture, pointer calibration and runtime fidelity",
                            "HID identity and ambiguous SDL2 mapping routes",
                            "Force-feedback forwarding for normalized virtual devices",
                            "Custom launcher and Nix split-output runtime contracts",
                            "Build, tests and runtime verification deferred by request"]}));
                    continue;
                }
                rows.push(json!({"name":core,"platform":platform,"kind":"BizHawk core",
                    "status":"missing","profiles":[],"choices":[],"selected":null,
                    "detail":"This database relationship belongs to BizHawk, not RetroArch. It needs a BizHawk controller configuration and launch adapter; libretro profiles cannot configure it."}));
                continue;
            }
            let modes = catalog.platform_profiles(core, platform);
            core_platform_count += 1;
            if modes.is_empty() && matches!(core.as_str(), "mame" | "fbneo") {
                let (detail, implemented, remaining) = if core == "mame" {
                    (
                        "Per-game mapping code is connected. Automatic six/eight-button discovery applies to Arcade; other systems require an explicit inspected setup. This is not universal game support or a runtime-verified result.",
                        vec![
                            "Inspected digital arcade mapping, six/eight-button and Neo Geo presets with per-player inheritance/overrides",
                            "Sparse button-channel placement, capacity-based Automatic geometry and shared-pressure composition",
                            "Source/destination diagrams, exact action selection, button swaps and calibration-gap navigation",
                            "Read-only shared-preset comparison and readable setup summaries with optional JSON editing",
                            "Complete resolved-output checks shared by automatic setup, saved validation, review and per-game launch",
                            "Selected-player routing and explicit analog/directional-switch assignments",
                        ],
                        vec![
                            "Unresolved game-specific inputs, channel-capacity limits and peripheral contracts; wheels remain paused",
                            "A complete game/mode inventory and matching evidence before any MAME completion percentage can be established",
                            "Native runtime, content dependency and calibration requirements",
                            "Build, tests and runtime verification deferred by request",
                        ],
                    )
                } else {
                    (
                        "Per-game descriptor/query inspection and mapping code is connected. Supported inputs depend on the selected game and device mode; no universal core profile is claimed.",
                        vec![
                            "Exact native input review and saved-measurement mapping",
                            "Visual RetroPad/lightgun-button mapping and native mouse target diagrams",
                            "Alias handling, calibration checks and launch-owned gamepad transport",
                            "Exact relative-device selection, mixed/relative-only port preparation and owned frontend routing",
                            "Source/destination relative mapping diagrams and private hotkey/menu isolation configuration",
                            "Explicit gamepad-to-key mapping plus native keyboard passthrough, preserving required keys absent from the Linux key table",
                            "Saved rectangular absolute calibration, Arcade Gun aim selection and owned virtual-gamepad launch composition",
                        ],
                        vec![
                            "Runtime verification of connected gamepad, relative mouse, keyboard and Arcade Gun aim paths",
                            "Hardware offscreen/reload protocols, other pointer/touch modes and independent simultaneous keyboard ownership",
                            "Game/device-specific inputs outside supported binding parts",
                            "Build, tests and runtime verification deferred by request",
                        ],
                    )
                };
                rows.push(json!({"name":core,"platform":platform,"kind":"RetroArch",
                    "status":"per_game","detail":detail,"implemented":implemented,"remaining":remaining,
                    "per_game_setup":core,"profiles":[],"choices":[],"selected":null}));
                continue;
            }
            if !modes.is_empty() {
                core_platforms_with_contracts += 1;
            }
            let status = if modes.iter().any(|profile| !profile.explicit_selection) {
                "limited"
            } else if !modes.is_empty() {
                "explicit"
            } else if !profiles.is_empty() {
                "preview"
            } else {
                "missing"
            };
            let detail = if status == "explicit" {
                "Choose a target input/topology mode below. It is not inferred from attached controllers. Review the contract's content/peripheral limitations before selecting a default for this core/platform."
            } else if !modes.is_empty() {
                "Listed modes only. Requires a compatible saved calibration, supported device/options and Linux RetroArch. Other peripherals/modes are not implied."
            } else if !profiles.is_empty() {
                "Core has mapping profiles, but no launch contract matches this platform, including explicit modes."
            } else if core == "steemsse" {
                "Controller mapping requires a Linux core port or a supported Windows-runtime adapter. The existing embedded-ROM guard is not an input adapter."
            } else {
                "Missing core output contract and launch-mode integration. Reuse shared layout rules; do not add controller-specific pair mappings."
            };
            let mut choices =
                vec![json!({"id":"","name":"Automatic mode resolution (no explicit choice)"})];
            choices.extend(
                modes
                    .iter()
                    .filter(|profile| profile.explicit_selection)
                    .map(|profile| json!({"id":profile.id,"name":profile.name})),
            );
            rows.push(json!({"name":core,"platform":platform,"kind":"RetroArch",
                "status":status,"detail":detail,
                "choices":choices,"selected":selections.get(&crate::controller_launch::selection_key(core, platform)),
                "profiles":modes.iter().map(|p| p.id.as_str()).collect::<Vec<_>>()}));
        }
    }
    let standalone_covered = standalone
        .values()
        .filter(|(name, _)| {
            NATIVE_ADAPTERS
                .iter()
                .any(|adapter| name.eq_ignore_ascii_case(adapter))
        })
        .count();
    for (name, platforms) in standalone.values() {
        for platform in platforms {
            let bizhawk = name.eq_ignore_ascii_case("BizHawk");
            let duckstation = name.eq_ignore_ascii_case("DuckStation");
            let ppsspp = name.eq_ignore_ascii_case("PPSSPP");
            let mgba = name.eq_ignore_ascii_case("mGBA");
            let dolphin = name.eq_ignore_ascii_case("Dolphin");
            let snes9x = name.eq_ignore_ascii_case("Snes9x");
            let fceux = name.eq_ignore_ascii_case("FCEUX");
            let sameboy = name.eq_ignore_ascii_case("SameBoy");
            let mednafen = name.eq_ignore_ascii_case("Mednafen");
            let mame_native = name.eq_ignore_ascii_case("MAME");
            let flycast_native = name.eq_ignore_ascii_case("Flycast");
            let pcsx2_native = name.eq_ignore_ascii_case("PCSX2");
            let rpcs3_native = name.eq_ignore_ascii_case("RPCS3");
            let melonds_native = name.eq_ignore_ascii_case("melonDS");
            let ares = name.eq_ignore_ascii_case("ares");
            rows.push(json!({"name":name,"platform":platform,"kind":"Standalone / non-core catalog entry",
                "status":if NATIVE_ADAPTERS.iter().any(|adapter| name.eq_ignore_ascii_case(adapter)) { "partial" } else { "missing" },"profiles":[],"choices":[],"selected":null,
                "detail":if ares {
                    "Native ares 148+ guided player/target mapping with private settings. Default gamepad modes and ordinary six/eight-button arcade panels are implemented. Alternate attachments, target-runtime device availability and cross-platform runtime verification remain incomplete."
                } else if melonds_native {
                    "Native Linux melonDS standard-button dispatch is connected with a private TOML mount and SDL2 physical capture. Touch, microphone, other hosts and native input behavior remain unverified or incomplete."
                } else if rpcs3_native {
                    "Native Linux RPCS3 standard-pad saved-setup dispatch is connected for file boot targets, with temporary profile ownership and child log/device checks. Directory boot targets, other backends, mapping database parity and runtime behavior remain unverified."
                } else if bizhawk {
                    "Native BizHawk launch dispatch is implemented for explicitly saved controller setups, including supported digital decks and PlayStation modes. Individual platform/mode coverage and runtime verification remain incomplete."
                } else if duckstation {
                    "Native Linux DuckStation calibrated digital/analog launch dispatch is implemented for saved setups and the pinned executable/SDL contract. The child must confirm configuration/player routing. Flatpak/Wine and runtime verification remain incomplete."
                } else if ppsspp {
                    "Native Linux SDL2 PPSSPP PSP gamepad launch dispatch is implemented for saved setups with a private SYSTEM overlay and child mapping/runtime confirmation. Flatpak/Wine, other frontend variants, unrecognized SDL fallback devices and runtime verification remain incomplete."
                } else if snes9x {
                    "Native Linux Snes9x GTK 1.63 dispatch is implemented for saved SNES controller setups, with private config, SDL identity and child mount/device checks. Qt, Wine/Flatpak, custom launch arguments and runtime testing remain incomplete."
                } else if fceux {
                    "Native Linux FCEUX Qt 2.6.6 dispatch is implemented for saved NES pad setups with private config/profile mounts and SDL routing checks. ROM-selected device overrides remain unresolved; this is partial, untested support."
                } else if sameboy {
                    "Native Linux SameBoy SDL v1.0.3 saved-setup dispatch is implemented for device-zero Game Boy controls. Runtime ABI and DATA_DIR are user-declared; tilt-game axis behavior and runtime testing remain unresolved."
                } else if mame_native {
                    "Native Linux MAME six/eight-button saved-panel dispatch is connected for plain machine arguments, raw SDL and unique GUIDs. Private configuration copies preserve originals. Internal item routing, identical controllers, other providers and runtime compatibility remain unverified."
                } else if flycast_native {
                    "Native Linux Flycast six/eight-button saved-panel dispatch is connected with private mappings/config and startup joystick reports. Internal routing, native game ID, companion content and runtime compatibility remain unverified."
                } else if pcsx2_native {
                    "Native Linux PCSX2 DualShock2 saved-setup dispatch is connected with private data/config, SDL3 player and disc-identity startup checks. SDL 3.2.20 classic backend and explicit dependencies are required. Internal routing, additional input actions, portable installs and runtime compatibility remain unverified."
                } else if mednafen {
                    "Native Linux Mednafen GB/GBA/Lynx/Neo Geo Pocket/WonderSwan/Virtual Boy/Game Gear/Master System/PC Engine saved-setup dispatch is implemented with joydev mapping and private config layers. Child internal IDs, additional native drivers, other systems, compressed content and runtime testing remain incomplete."
                } else if dolphin {
                    "Native Linux Dolphin 2606 GameCube ISO/GCM dispatch is implemented for explicitly saved evdev setups, with private settings and child mount/device checks. System-data path is user-declared; runtime verification, compressed media, Wii, other backends and host variants remain incomplete."
                } else if mgba {
                    "Native Linux mGBA SDL 0.10.5 guided player/target mapping now discovers runtime paths at launch and uses a private config.ini. Requires an existing native config and physical calibration. Qt, Wine/Flatpak, identical-GUID ambiguity and runtime verification remain incomplete."
                } else {
                    "Standalone candidate: no calibrated native launch dispatch is implemented for this emulator. A RetroArch alternative does not configure the standalone executable. Catalog presence does not establish installation, supported launch media or host compatibility."
                }}));
        }
    }
    Ok(
        json!({"database":path.display().to_string(),"host_os":std::env::consts::OS,
        "layout_count":catalog.layouts.len(),"profile_count":catalog.emulator_profiles.len(),
        "launch_profile_count":catalog.emulator_profiles.iter().filter(|p| p.retroarch_launch.is_some()).count(),
        "database_core_count":cores.len(),
        "non_retroarch_core_count":cores.len()-retroarch_cores.len(),
        "core_count":retroarch_cores.len(),"cores_with_contracts":covered,"cores_without_contracts":retroarch_cores.len()-covered,
        "dynamic_core_adapters":dynamic_cores,
        "overall_completion_percent":null,
        "automatic_core_count":automatic_covered,"explicit_only_core_count":covered-automatic_covered,
        "core_entry_percent":if retroarch_cores.is_empty() { 0.0 } else { covered as f64 * 100.0 / retroarch_cores.len() as f64 },
        "core_platform_count":core_platform_count,
        "core_platforms_with_contracts":core_platforms_with_contracts,
        "core_platforms_without_contracts":core_platform_count-core_platforms_with_contracts,
        "core_platform_percent":if core_platform_count == 0 { 0.0 } else { core_platforms_with_contracts as f64 * 100.0 / core_platform_count as f64 },
        "standalone_count":standalone.len(),"standalone_adapters":NATIVE_ADAPTERS,
        "standalone_with_dispatch":standalone_covered,
        "standalone_entry_percent":if standalone.is_empty() { 0.0 } else { standalone_covered as f64 * 100.0 / standalone.len() as f64 },
        "source_entries_with_dispatch":covered+standalone_covered,
        "source_entry_count":retroarch_cores.len()+standalone.len(),
        "source_entry_percent":if retroarch_cores.is_empty() && standalone.is_empty() { 0.0 } else { (covered+standalone_covered) as f64 * 100.0 / (retroarch_cores.len()+standalone.len()) as f64 },
        "standalone_inventory_status":"source_dispatch_audited_runtime_unverified",
        "emulator_count":emulators.len(),"platform_count":platforms.len(),
        "relationship_count":relationships.len(),"rows":rows,
        "profiles":profile_coverage(catalog)}),
    )
}

fn profile_coverage(catalog: &Catalog) -> Vec<Value> {
    catalog
        .emulator_profiles
        .iter()
        .flat_map(|base_profile| {
            let count = if base_profile.port_controls.is_empty() && base_profile.port_layouts.is_empty() && base_profile.port_bindings.is_empty() {
                1
            } else {
                base_profile
                    .retroarch_launch
                    .as_ref()
                    .expect("validated port controls")
                    .max_players
            };
            (1..=count).map(move |port| {
                let profile = base_profile.for_port(port);
                let target = catalog
                    .layout(&profile.target_layout)
                    .expect("validated layout");
                let requested = profile.bindings.keys().map(String::as_str).collect();
                let layouts: Vec<_> = catalog
                .layouts
                .iter()
                .map(|source| {
                    let available = source
                        .controls
                        .iter()
                        .map(|control| control.id.as_str())
                        .collect();
                    let resolved =
                        crate::controller_layout::resolve(source, target, &available, &requested);
                    let missing: Vec<_> = target
                        .controls
                        .iter()
                        .filter(|control| {
                            !control.optional && profile.bindings.contains_key(&control.id)
                        })
                        .filter_map(|control| {
                            resolved.missing.get(&control.id).map(|reason| {
                                format!("{}: {}", control.label, reason.description())
                            })
                        })
                        .collect();
                    let mapping: Vec<_> = target
                        .controls
                        .iter()
                        .filter_map(|control| {
                            let output = profile.bindings.get(&control.id)?;
                            let physical = resolved.assignments.get(&control.id)?;
                            let physical =
                                source.controls.iter().find(|input| input.id == *physical)?;
                            Some(format!(
                                "{} → {} → {}",
                                physical.label, control.label, output
                            ))
                        })
                        .collect();
                    json!({"id":source.id,"name":source.name,"missing":missing,"mapping":mapping})
                })
                .collect();
                let name = if count > 1 {
                    format!("{} · port {port}", profile.name)
                } else {
                    profile.name.clone()
                };
                json!({"id":profile.id,"name":name,"core":profile.core,
            "target":target.name,"transport":profile.transport,"source":profile.source,
            "conditions":profile.conditions,"launch":profile.retroarch_launch,
            "native_launch":profile.native_launch,
            "guided_native":crate::controller_guided_native::supports(&profile),
            "frontend_ports":profile.frontend_port_count(),"explicit_selection":profile.explicit_selection,
            "port_devices":profile.port_devices,
            "requires_fresh_start":profile.requires_fresh_start,
            "content_guard":profile.content_guard,
            "content_extensions":profile.content_extensions,
            "layouts":layouts})
            })
        })
        .collect()
}
