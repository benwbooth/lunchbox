//! Apply guided target/player intent to an existing native runtime setup.
//! This is a launch-local copy. Never rewrite trusted runtime paths, hashes,
//! content identities or the user's separately saved advanced configuration.
use crate::{
    controller_catalog::EmulatorProfile,
    controllers::ControllerDevice,
    emulator::{EmulatorRuntimeKind, LaunchPlan, RomEmulatorOption},
    settings::AppSettings,
};
use anyhow::{Context, Result, ensure};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

mod runtime;

pub(crate) fn supports(profile: &EmulatorProfile) -> bool {
    profile.native_launch.is_some()
        && matches!(
            profile.core.as_str(),
            "ares"
                | "duckstation"
                | "mednafen"
                | "ppsspp"
                | "mgba"
                | "snes9x"
                | "fceux"
                | "sameboy"
                | "dolphin"
                | "pcsx2"
                | "rpcs3"
                | "melonds"
                | "flycast"
                | "mame"
                | "bizhawk"
        )
}

pub(crate) fn settings_for_launch<'a>(
    settings: &'a AppSettings,
    option: &RomEmulatorOption,
    platform: &str,
    plan: &LaunchPlan,
    inventory: &[ControllerDevice],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Cow<'a, AppSettings>> {
    if option.runtime_kind != EmulatorRuntimeKind::Standalone {
        return Ok(Cow::Borrowed(settings));
    }
    let Some(profile) =
        crate::controller_target::selected(&settings.controller_mapping, option, platform)?
    else {
        return Ok(Cow::Borrowed(settings));
    };
    // ares already consumes guided settings directly, without a saved runtime.
    if profile.transport == "ares-settings" {
        return Ok(Cow::Borrowed(settings));
    }
    if !supports(profile) {
        anyhow::bail!(
            "{} still needs its native setup connected to guided target selection. The saved target was not applied; no game was started.",
            option.emulator_name
        );
    }
    let ids = player_ids(settings, profile, inventory)?;
    let matches = |id: &str, content: &Path| {
        id == option.emulator_id && plan.arguments.iter().any(|arg| arg == content.as_os_str())
    };
    let mut adjusted = settings.clone();
    let mapping = &mut adjusted.controller_mapping;
    runtime::reuse(mapping, &profile.core, &option.emulator_id, plan)?;
    #[cfg(target_os = "linux")]
    if profile.core == "mgba"
        && !mapping
            .mgba_launches
            .iter()
            .any(|setup| matches(&setup.emulator_id, &setup.content))
    {
        mapping
            .mgba_launches
            .push(crate::controller_mgba::guided::discover(
                option,
                plan,
                &ids[0],
                profile.target_layout == "gba",
                cancel,
            )?);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = cancel;
    let mut found = 0usize;
    match profile.core.as_str() {
        "bizhawk" => {
            use crate::controller_bizhawk::guided;
            let scope =
                guided::scope(&profile.target_layout).context("Unknown BizHawk guided target")?;
            for id in &ids {
                let choices = source_choices(&mapping.calibrations[id], profile)?;
                mapping
                    .calibrations
                    .get_mut(id)
                    .context("BizHawk calibration disappeared")?
                    .target_mappings
                    .insert(profile.id.clone(), choices);
            }
            for setup in mapping
                .bizhawk_launch
                .iter_mut()
                .chain(&mut mapping.bizhawk_launches)
            {
                if setup.emulator_id != option.emulator_id
                    || !setup.matches_platform(platform)
                    || setup.scope_id() != scope
                {
                    continue;
                }
                found += 1;
                guided::apply(setup, profile, &ids)?;
            }
            // Multiple BizHawk cores may emulate the same system. The selected
            // guided target chooses one in this launch copy, without deleting
            // the user's other persisted core setups.
            let keep = |setup: &crate::controller_bizhawk::NativeLaunchSettings| {
                setup.emulator_id != option.emulator_id
                    || !setup.matches_platform(platform)
                    || setup.scope_id() == scope
            };
            if mapping
                .bizhawk_launch
                .as_ref()
                .is_some_and(|setup| !keep(setup))
            {
                mapping.bizhawk_launch = None;
            }
            mapping.bizhawk_launches.retain(keep);
        }
        "mame" => {
            let panel = match profile.target_layout.as_str() {
                "arcade-six-button" => crate::controller_mame_native::Panel::Six,
                "arcade-eight-button" => crate::controller_mame_native::Panel::Eight,
                _ => anyhow::bail!("Unsupported MAME guided arcade panel"),
            };
            for setup in &mut mapping.mame_native_launches {
                if setup.emulator_id != option.emulator_id {
                    continue;
                }
                found += 1;
                setup.resolve_inputs_at_launch = true;
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| {
                        Ok(crate::controller_mame_native::settings::SavedPlayer {
                            player: (i + 1) as u8,
                            controller_id: id.clone(),
                            panel,
                            native_device_id: String::new(),
                            controls: BTreeMap::new(),
                            source_controls: arcade_sources(&mapping.calibrations[id], profile)?,
                        })
                    })
                    .collect::<Result<_>>()?;
                setup.validate()?;
            }
        }
        "flycast" => {
            let panel = match profile.target_layout.as_str() {
                "arcade-six-button" => crate::controller_flycast_native::arcade::Panel::Six,
                "arcade-eight-button" => crate::controller_flycast_native::arcade::Panel::Eight,
                _ => anyhow::bail!("Unsupported Flycast guided arcade panel"),
            };
            for setup in &mut mapping.flycast_native_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| {
                        Ok(crate::controller_flycast_native::settings::Player {
                            player: (i + 1) as u8,
                            controller_id: id.clone(),
                            panel,
                            source_controls: arcade_sources(&mapping.calibrations[id], profile)?,
                        })
                    })
                    .collect::<Result<_>>()?;
                setup.review(&mapping.calibrations)?;
            }
        }
        "pcsx2" => {
            for setup in &mut mapping.pcsx2_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.multitaps = [ids.len() > 2, ids.len() > 5];
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| {
                        Ok(crate::controller_pcsx2::settings::Player {
                            player: (i + 1) as u8,
                            controller_id: id.clone(),
                            source_controls: source_choices(&mapping.calibrations[id], profile)?,
                        })
                    })
                    .collect::<Result<_>>()?;
                setup.review(&mapping.calibrations)?;
            }
        }
        "rpcs3" => {
            for setup in &mut mapping.rpcs3_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| {
                        Ok(crate::controller_rpcs3::settings::Player {
                            player: (i + 1) as u8,
                            controller_id: id.clone(),
                            source_controls: source_choices(&mapping.calibrations[id], profile)?,
                        })
                    })
                    .collect::<Result<_>>()?;
                setup.review(&mapping.calibrations)?;
            }
        }
        "melonds" => {
            for setup in &mut mapping.melonds_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.players = vec![crate::controller_melonds::settings::Player {
                    player: 1,
                    controller_id: ids[0].clone(),
                    source_controls: source_choices(&mapping.calibrations[&ids[0]], profile)?,
                }];
                setup.review(&mapping.calibrations)?;
            }
        }
        "dolphin" => {
            for setup in &mut mapping.dolphin_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.resolve_devices_at_launch = true;
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(
                        |(i, id)| crate::controller_dolphin::standalone::settings::Player {
                            port: (i + 1) as u8,
                            controller_id: id.clone(),
                            device_qualifier: String::new(),
                        },
                    )
                    .collect();
                setup.review(&mapping.calibrations)?;
            }
        }
        "mgba" => {
            for setup in &mut mapping.mgba_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.controller_id = ids[0].clone();
                setup.handheld = if profile.target_layout == "gba" {
                    crate::controller_mgba::settings::Handheld::Gba
                } else {
                    crate::controller_mgba::settings::Handheld::Gameboy
                };
                setup.review(&mapping.calibrations)?;
            }
        }
        "ppsspp" => {
            for setup in &mut mapping.ppsspp_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.controller_id = ids[0].clone();
                setup.review(&mapping.calibrations)?;
            }
        }
        "snes9x" => {
            for setup in &mut mapping.snes9x_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| crate::controller_snes9x::settings::Player {
                        player: (i + 1) as u8,
                        controller_id: id.clone(),
                    })
                    .collect();
                setup.review(&mapping.calibrations)?;
            }
        }
        "fceux" => {
            for setup in &mut mapping.fceux_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| crate::controller_fceux::settings::Player {
                        player: (i + 1) as u8,
                        controller_id: id.clone(),
                    })
                    .collect();
                setup.review(&mapping.calibrations)?;
            }
        }
        "sameboy" => {
            for setup in &mut mapping.sameboy_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                setup.players = vec![crate::controller_sameboy::settings::Player {
                    player: 1,
                    controller_id: ids[0].clone(),
                }];
                setup.review(&mapping.calibrations)?;
            }
        }
        "duckstation" => {
            for setup in &mut mapping.duckstation_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                let kind = if profile.target_layout == "playstation-digital" {
                    "DigitalController"
                } else {
                    "AnalogController"
                };
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| crate::controller_duckstation::SavedPlayer {
                        pad: (i + 1) as u8,
                        controller_id: id.clone(),
                        controller_type: kind.into(),
                    })
                    .collect();
                setup.apply_selected_ports = true;
                setup.review(&mapping.calibrations)?;
            }
        }
        "mednafen" => {
            for setup in &mut mapping.mednafen_launches {
                if !matches(&setup.emulator_id, &setup.content) {
                    continue;
                }
                found += 1;
                use crate::controller_mednafen::{md::Tap, profiles::Gamepad, settings::Player};
                let mut gamepad = Gamepad::from_profile_id(&profile.id)
                    .context("Unknown Mednafen guided target")?;
                if gamepad == Gamepad::NesTwo && ids.len() > 2 {
                    gamepad = Gamepad::NesFourScore;
                }
                setup.gamepad = gamepad;
                setup.md_tap = (gamepad.system() == "md").then_some(if ids.len() <= 2 {
                    Tap::None
                } else if ids.len() <= 5 {
                    Tap::PortOne
                } else {
                    Tap::Dual
                });
                // Native sequential logical slots are allocated by the existing
                // topology writers; no SDL index becomes an emulated port number.
                setup.psx_multitaps =
                    (gamepad.system() == "psx").then_some([ids.len() > 2, ids.len() > 5]);
                setup.saturn_multitaps =
                    (gamepad.system() == "ss").then_some([ids.len() > 2, ids.len() > 7]);
                setup.players = ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| Player {
                        player: (i + 1) as u8,
                        controller_id: id.clone(),
                        gamepad: None,
                    })
                    .collect();
                setup.review(&mapping.calibrations)?;
            }
        }
        _ => unreachable!(),
    }
    ensure!(
        found == 1,
        if found == 0 {
            "This emulator has no native runtime setup for this game yet. Your guided target and players are saved, but automatic runtime discovery is still required."
        } else {
            "More than one native runtime setup matches this game. Resolve the duplicate before launching."
        }
    );
    Ok(Cow::Owned(adjusted))
}

/// Consume exactly the same resolver and manual overrides as the visual review.
/// These are physical layout IDs, not guessed SDL/native button numbers.
fn source_choices(
    calibration: &crate::controller_catalog::Calibration,
    profile: &EmulatorProfile,
) -> Result<BTreeMap<String, String>> {
    let mut used = BTreeSet::new();
    calibration
        .plan_profile(profile)?
        .rows
        .into_iter()
        .map(|row| {
            let source = row.physical_id.with_context(|| {
                format!(
                    "Assign a physical control to {} in Controller setup",
                    row.target
                )
            })?;
            ensure!(
                row.input
                    .as_ref()
                    .is_some_and(|input| input.native.is_some()),
                "{} needs physical calibration",
                row.target
            );
            ensure!(
                used.insert(source.clone()),
                "The native target cannot reuse a physical control"
            );
            Ok((row.target_id, source))
        })
        .collect()
}

fn arcade_sources(
    calibration: &crate::controller_catalog::Calibration,
    profile: &EmulatorProfile,
) -> Result<BTreeMap<String, String>> {
    let mut choices = source_choices(calibration, profile)?;
    // Generic panel artwork calls the Select-position control "select";
    // the native panel writer calls the same destination action "coin".
    let coin = choices
        .remove("select")
        .context("Arcade target is missing Coin")?;
    ensure!(
        choices.insert("coin".into(), coin).is_none(),
        "Duplicate arcade Coin target"
    );
    Ok(choices)
}

fn player_ids(
    settings: &AppSettings,
    profile: &EmulatorProfile,
    inventory: &[ControllerDevice],
) -> Result<Vec<String>> {
    let mapping = &settings.controller_mapping;
    ensure!(
        mapping.explicit_player_selection,
        "Choose players in Controller setup first"
    );
    let limit = profile
        .native_launch
        .as_ref()
        .context("Missing native player limits")?
        .max_players;
    ensure!(
        !mapping.player_mappings.is_empty() && mapping.player_mappings.len() <= limit,
        "This target supports up to {limit} players. Adjust the players in Controller setup."
    );
    let mut seen = BTreeSet::new();
    mapping.player_mappings.iter().enumerate().map(|(i, player)| {
        let id = player.controller_id.as_ref().context("Select a controller for every player")?;
        ensure!(seen.insert(id) && !mapping.hidden_controller_ids.contains(id), "Player {} has a duplicate or hidden controller", i + 1);
        ensure!(inventory.iter().filter(|device| &device.stable_id == id).count() == 1, "Player {} is disconnected or has an ambiguous device identity", i + 1);
        let calibration = mapping.calibrations.get(id).context("Finish recording this controller first")?;
        ensure!(calibration.os == "linux" && calibration.backend != crate::controller_sdl3::BACKEND,
            "{} currently needs Linux physical button calibration. SDL3 logical-button translation for this native adapter is not implemented yet.", profile.core);
        calibration.plan_profile(profile)?;
        Ok(id.clone())
    }).collect()
}
