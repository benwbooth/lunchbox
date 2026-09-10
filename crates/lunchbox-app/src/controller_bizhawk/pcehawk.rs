//! PCEHawk at BizHawk 8c6b8958bbbe623eaaa36bc82af858b812893628.
//! This core exposes two-button pads only, not the separate TurboNyma contract.
use anyhow::{Context, Result, ensure};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const CORE: &str = "BizHawk.Emulation.Cores.PCEngine.PCEngine";
const DECK: &str = "PC Engine Controller";
pub(crate) const BUTTONS: [&str; 8] = ["Up", "Down", "Left", "Right", "Select", "Run", "B2", "B1"];
const SEMANTIC_BUTTONS: [&str; 8] = ["Up", "Down", "Left", "Right", "Select", "Start", "B", "A"];

pub(crate) use super::digital_session::PlayerRequest;

/// Check prepared configuration, not loaded-runtime provenance.
pub(crate) fn validate_capture_config(source: &str, ports: [bool; 5], system: &str) -> Result<()> {
    let sync = super::definition_capture::selected_sync_config(source, system, "PCEHawk", CORE)?;
    for (index, enabled) in ports.into_iter().enumerate() {
        ensure!(
            sync[format!("Port{}", index + 1)].as_u64() == Some(u64::from(enabled)),
            "PCEHawk capture config port {} differs from the request",
            index + 1
        );
    }
    Ok(())
}

pub(crate) fn validate_definition(
    definition: &super::definition_capture::Definition,
    ports: [bool; 5],
    expected_system: &str,
    expected_rom_hash: &str,
) -> Result<()> {
    ensure!(
        ports.iter().any(|enabled| *enabled),
        "No PCEHawk controllers requested"
    );
    ensure!(
        matches!(expected_system, "PCE" | "SGX" | "PCECD" | "SGXCD"),
        "Unsupported expected PCEHawk system"
    );
    ensure!(
        !expected_rom_hash.is_empty()
            && expected_rom_hash.len() <= 256
            && !expected_rom_hash.chars().any(char::is_control),
        "Expected PCEHawk content hash is missing or invalid"
    );
    ensure!(
        definition.system == expected_system && definition.rom_hash == expected_rom_hash,
        "Captured PCEHawk system/content hash differs from the request"
    );
    ensure!(
        definition.axes.is_empty(),
        "Captured PCEHawk deck has unexpected axes"
    );
    let mut expected = std::collections::BTreeSet::new();
    for (index, enabled) in ports.into_iter().enumerate() {
        if enabled {
            expected.extend(
                BUTTONS
                    .iter()
                    .map(|button| format!("P{} {button}", index + 1)),
            );
        }
    }
    let actual: std::collections::BTreeSet<_> = definition.buttons.iter().cloned().collect();
    ensure!(
        actual.len() == definition.buttons.len(),
        "Captured PCEHawk definition repeats controls"
    );
    let missing = expected
        .difference(&actual)
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    let extra = actual
        .difference(&expected)
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    ensure!(
        actual == expected,
        "Loaded PCEHawk controls differ from the request (first 12 each): missing [{}]; unexpected [{}]",
        missing.join(", "),
        extra.join(", ")
    );
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn capture_definition(
    process: super::definition_capture::CaptureProcess,
    ports: [bool; 5],
    expected_system: &str,
    expected_rom_hash: &str,
    timeout: std::time::Duration,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<super::definition_capture::Definition> {
    let definition = process.wait_for_definition(timeout, cancel)?;
    validate_definition(&definition, ports, expected_system, expected_rom_hash)?;
    ensure!(
        !cancel.load(std::sync::atomic::Ordering::Relaxed),
        "PCEHawk definition comparison cancelled"
    );
    Ok(definition)
}

pub(crate) struct PreparedConfig {
    pub configuration: String,
    pub warnings: Vec<String>,
}

pub(crate) fn validate_saved_pad(
    calibration: &crate::controller_catalog::Calibration,
) -> Result<()> {
    super::digital_pad::validate_saved_pad("pce-2", &SEMANTIC_BUTTONS, calibration)
}

pub(crate) fn translate_pad(
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let (semantic, warnings) = super::digital_pad::translate_pad(
        "pce-2",
        &SEMANTIC_BUTTONS,
        calibration,
        logical,
        inventory,
        path,
    )?;
    // Catalog A/B are printed I/II; Start is printed Run. Do not reuse
    // SMSHawk's B-to-B1 conversion, which has the opposite button numbering.
    let native = semantic
        .into_iter()
        .map(|(name, input)| {
            let name = match name.as_str() {
                "A" => "B1".to_owned(),
                "B" => "B2".to_owned(),
                "Start" => "Run".to_owned(),
                _ => name,
            };
            (name, input)
        })
        .collect();
    Ok((native, warnings))
}

/// Translate a supplied snapshot only; device discovery belongs to the session.
pub(crate) fn prepare_config(
    source: &str,
    ports: [bool; 5],
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<PreparedConfig> {
    validate_players(ports, requests.iter().map(|request| request.player))?;
    let mut devices = std::collections::BTreeSet::new();
    for request in requests {
        let device = inventory.device_at_path(request.runtime_path)?;
        ensure!(
            devices.insert(device.device_index),
            "One SDL device cannot supply multiple PCEHawk players"
        );
    }
    let mut players = BTreeMap::new();
    let mut warnings = Vec::new();
    for request in requests {
        let (buttons, notes) = translate_pad(
            request.calibration,
            request.logical,
            inventory,
            request.runtime_path,
        )
        .with_context(|| format!("Preparing PCEHawk player {}", request.player))?;
        players.insert(request.player, buttons);
        warnings.extend(
            notes
                .into_iter()
                .map(|note| format!("Player {}: {note}", request.player)),
        );
    }
    Ok(PreparedConfig {
        configuration: encode_config(source, ports, &players)?,
        warnings,
    })
}

/// Publish argument changes only after the private configuration is prepared.
pub(crate) fn prepare_arguments(
    arguments: &mut Vec<std::ffi::OsString>,
    working_directory: &std::path::Path,
    exe_directory: &std::path::Path,
    ports: [bool; 5],
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<(super::PreparedConfig, Vec<String>)> {
    let mut warnings = Vec::new();
    let config =
        super::prepare_arguments_using(arguments, working_directory, exe_directory, |source| {
            super::prepare_config_using(source, |text| {
                let prepared = prepare_config(text, ports, requests, inventory)?;
                warnings = prepared.warnings;
                Ok(prepared.configuration)
            })
        })?;
    Ok((config, warnings))
}

/// Native P1–P5 remain fixed when earlier ports are disconnected.
pub(crate) fn validate_players(
    ports: [bool; 5],
    players: impl IntoIterator<Item = u8>,
) -> Result<()> {
    let mut assigned = std::collections::BTreeSet::new();
    for player in players {
        ensure!(
            (1..=5).contains(&player) && ports[usize::from(player - 1)] && assigned.insert(player),
            "Invalid, disconnected or duplicate PCEHawk player {player}"
        );
    }
    ensure!(
        !assigned.is_empty()
            && assigned.len() == ports.iter().filter(|connected| **connected).count(),
        "Map every connected PCEHawk port"
    );
    Ok(())
}

pub(crate) fn encode_config(
    source: &str,
    ports: [bool; 5],
    players: &BTreeMap<u8, BTreeMap<String, String>>,
) -> Result<String> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "BizHawk source config exceeds 16 MiB"
    );
    validate_players(ports, players.keys().copied())?;
    let mut buttons = Map::new();
    for (player, bindings) in players {
        ensure!(
            bindings.len() == BUTTONS.len()
                && BUTTONS.iter().all(|name| bindings.contains_key(*name)),
            "PCEHawk player {player} needs exactly its eight native controls"
        );
        for (name, input) in bindings {
            ensure!(
                !input.trim().is_empty()
                    && input.len() <= 1024
                    && !input.chars().any(char::is_control),
                "Invalid PCEHawk native input for player {player} {name}"
            );
            buttons.insert(format!("P{player} {name}"), Value::String(input.clone()));
        }
    }
    let mut config: Value = serde_json::from_str(source.trim_start_matches('\u{feff}'))
        .context("Parsing native BizHawk source config")?;
    let root = config
        .as_object_mut()
        .context("BizHawk config must be an object")?;
    super::object_child(root, "AllTrollers")?.insert(DECK.into(), Value::Object(buttons));
    for field in [
        "AllTrollersAnalog",
        "AllTrollersFeedbacks",
        "AllTrollersAutoFire",
    ] {
        super::object_child(root, field)?.insert(DECK.into(), Value::Object(Map::new()));
    }
    // Every system handled by this native core shares the same controller deck.
    for system in ["PCE", "PCECD", "SGX", "SGXCD"] {
        super::object_child(root, "PreferredCores")?
            .insert(system.into(), Value::String("PCEHawk".into()));
    }
    root.insert("DontTryOtherCores".into(), Value::Bool(true));
    let sync = super::object_child(super::object_child(root, "CoreSyncSettings")?, CORE)?;
    for (index, connected) in ports.into_iter().enumerate() {
        sync.insert(
            format!("Port{}", index + 1),
            Value::from(u8::from(connected)),
        );
    }
    let mut encoded = serde_json::to_string_pretty(&config)?;
    encoded.push('\n');
    Ok(encoded)
}
