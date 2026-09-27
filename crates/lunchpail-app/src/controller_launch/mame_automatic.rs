//! Automatic digital arcade launch, deliberately narrower than all MAME systems.
//! Complete self-contained archives can bootstrap without a hand-written setup.
//! Parent/BIOS/disk manifests and unusual controls use the reviewed per-game path.
use super::*;
use crate::controller_mame::{DigitalLayout, InspectionInput, PersistentPaths};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

pub(super) fn prepare(
    settings: &AppSettings,
    platform: &str,
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    inventory: &[ControllerDevice],
    layout: DigitalLayout,
    cancel: &AtomicBool,
) -> Result<CalibratedLaunch> {
    ensure!(
        platform == "Arcade",
        "Automatic MAME arcade layouts apply only to the Arcade platform; use a reviewed per-game setup for other MAME systems"
    );
    ensure!(
        matches!(&option.executable, EmulatorExecutable::Native(_)),
        "Automatic MAME inspection requires native RetroArch; other runtime adapters need a per-game contract"
    );
    ensure!(
        !plan
            .environment
            .iter()
            .any(|(key, _)| key == "HOME" || key == "XDG_CONFIG_HOME"),
        "Resolve custom MAME configuration roots before automatic setup"
    );
    check_preparation_cancel(cancel)?;
    let content = plan
        .retroarch_content
        .as_ref()
        .context("Missing original MAME content")?;
    crate::controller_launch_modes::validate_arguments(
        emulator_arguments(plan, &option.executable)?,
        content,
    )?;
    let original = &content.content;
    ensure!(
        matches!(
            original.extension().and_then(|ext| ext.to_str()),
            Some("zip" | "7z")
        ),
        "Automatic MAME arcade setup accepts self-contained .zip/.7z sets; command files and external dependencies require a reviewed per-game setup"
    );
    let machine = original
        .file_stem()
        .and_then(|name| name.to_str())
        .context("MAME set name is not UTF-8")?
        .to_owned();
    ensure!(
        !machine.is_empty()
            && machine.len() <= 64
            && machine != "default"
            && machine
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "MAME archive filename must be the exact machine short name; display titles are not accepted as identities"
    );
    let mut devices = selected_devices(settings, inventory, platform);
    ensure!(
        !devices.is_empty(),
        "No connected calibrated controller is available for automatic MAME setup"
    );
    devices.truncate(8);
    let mut configuration =
        mame_configuration::Snapshot::read(&option.executable, "MAME", original, true)?;
    for key in ["mame_read_config", "mame_mame_paths_enable"] {
        ensure!(
            cfg_value(&configuration.options, key)?
                .as_deref()
                .unwrap_or("disabled")
                == "disabled",
            "Automatic MAME setup cannot reproduce enabled {key}; use a per-game setup with resolved native configuration and paths"
        );
    }
    // Resolve persistence against the original content, before private .cmd
    // substitution. Never choose an inspection/session directory as a save root.
    let save_path = frontend_directory(
        &configuration.base_path,
        &configuration.base,
        original,
        false,
    )?;
    let state_path = frontend_directory(
        &configuration.base_path,
        &configuration.base,
        original,
        true,
    )?;
    configuration.verify()?;
    let frontend_save = configuration.create_persistent_directory(&save_path)?;
    let frontend_state = configuration.create_persistent_directory(&state_path)?;
    // Pinned libretro retro_init.cpp Set_Path_Option uses save/mame/<category>.
    let native_root = frontend_save.join("mame");
    let persistent = PersistentPaths {
        nvram: configuration.create_persistent_directory(&native_root.join("nvram"))?,
        diff: configuration.create_persistent_directory(&native_root.join("diff"))?,
        states: configuration.create_persistent_directory(&native_root.join("states"))?,
        snapshots: configuration.create_persistent_directory(&native_root.join("snaps"))?,
        recordings: configuration.create_persistent_directory(&native_root.join("input"))?,
    };
    let mut inputs = vec![InspectionInput {
        source: original.canonicalize()?,
        destination: Path::new("roms").join(
            original
                .file_name()
                .context("Missing MAME archive filename")?,
        ),
    }];
    for name in ["default.cfg".to_owned(), format!("{machine}.cfg")] {
        let source = native_root.join("cfg").join(&name);
        if configuration.track_native_config(&source)? {
            inputs.push(InspectionInput {
                source: source.canonicalize()?,
                destination: Path::new("cfg").join(name),
            });
        }
    }
    inputs.extend(crate::controller_mame::persistent_inputs(
        &persistent,
        cancel,
    )?);
    let dependency_roots = if settings
        .controller_mapping
        .mame_discover_sibling_dependencies
    {
        vec![
            original
                .parent()
                .context("MAME archive has no parent directory")?
                .canonicalize()?,
        ]
    } else {
        Vec::new()
    };
    let mut inspection = prepare_mame_inspection(option, plan, &machine, "MAME", inputs, &(1..=8).collect(), Some(configuration), dependency_roots, &[], &[], Some(devices.len()), &[], &[], false, cancel)
        .context("Automatic MAME inspection could not establish this set's inputs. Enable sibling dependency discovery for split/nonmerged sets or declare exact files in a per-game setup")?;
    ensure!(
        inspection.mapping.unhandled_fields.is_empty(),
        "MAME has {} unsupported active fields ({}); use a reviewed special-control contract, not a generic arcade layout",
        inspection.mapping.unhandled_fields.len(),
        inspection
            .mapping
            .unhandled_fields
            .iter()
            .take(8)
            .map(|field| field.input_type.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    // The complete snapshot was inspected, but allocation is already limited
    // to actual digital players for the available controllers.
    let mut profiles = Vec::new();
    for &port in &inspection.selected_players {
        if let Some(profile) = crate::controller_mame::explicit_profile(
            &inspection.fields,
            &inspection.selected_players,
            &[],
            &[],
            port,
            layout,
        )
        .with_context(|| format!("Resolving automatic MAME player {port}"))?
        {
            profiles.push((port, profile));
        }
    }
    ensure!(
        !profiles.is_empty(),
        "MAME reported no standard digital arcade players"
    );
    devices.truncate(profiles.len());
    let mut players = BTreeMap::new();
    for ((port, profile), device) in profiles.iter().zip(devices) {
        check_preparation_cancel(cancel)?;
        let calibration = settings
            .controller_mapping
            .calibrations
            .get(&device.stable_id)
            .context("MAME controller calibration disappeared")?;
        ensure!(
            compatible(calibration, profile),
            "MAME player {port} lacks the calibrated controls required by {}; review the controller or save a per-game setup",
            profile.name
        );
        players.insert(*port, (calibration, device));
    }
    let selected: BTreeSet<_> = players.keys().copied().collect();
    inspection.mapping = crate::controller_mame::plan_digital_fields(
        &inspection.fields,
        inspection.original_controller_config.as_deref(),
        inspection.original_game_config.as_deref(),
        &selected,
    )?;
    inspection.selected_players = selected;
    let input = stage_mame_input(inspection, persistent)?;
    prepare_mame_calibrated_session(
        input,
        &players,
        plan,
        &frontend_save,
        &frontend_state,
        layout,
        &BTreeMap::new(),
        &[],
        &[],
        cancel,
    )
}

fn frontend_directory(
    base_path: &Path,
    config: &str,
    original: &Path,
    state: bool,
) -> Result<PathBuf> {
    let (directory, in_content, sort_content, sort_core, fallback) = if state {
        (
            "savestate_directory",
            "savestates_in_content_dir",
            "sort_savestates_by_content_enable",
            "sort_savestates_enable",
            "states",
        )
    } else {
        (
            "savefile_directory",
            "savefiles_in_content_dir",
            "sort_savefiles_by_content_enable",
            "sort_savefiles_enable",
            "saves",
        )
    };
    let parent = original
        .parent()
        .context("MAME content has no parent directory")?;
    let mut path = if config_bool(config, in_content, false)? {
        parent.to_path_buf()
    } else if let Some(value) =
        cfg_value(config, directory)?.filter(|value| !value.is_empty() && value != "default")
    {
        let path = configured_path(&value)?;
        ensure!(
            path.is_dir(),
            "Configured MAME {directory} is not an existing directory; resolve it before automatic setup"
        );
        path
    } else {
        base_path.join(fallback)
    };
    // RetroArch 69a4f0ea configuration.c handles save/state paths separately
    // from rgui_config_directory: empty values leave desktop defaults intact.
    // runloop_path_set_redirect appends content-folder then core sorting.
    if config_bool(config, sort_content, false)? {
        path.push(
            parent
                .file_name()
                .context("MAME content sorting requires a named directory")?,
        );
    }
    if config_bool(config, sort_core, true)? {
        path.push("MAME");
    }
    Ok(path)
}
