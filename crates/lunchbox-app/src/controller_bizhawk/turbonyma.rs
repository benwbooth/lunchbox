//! TurboNyma input names at BizHawk 8c6b8958bbbe623eaaa36bc82af858b812893628.
//! Native translation/configuration contract used by shared digital sessions.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const BUTTONS: [&str; 14] = [
    "Up", "Down", "Left", "Right", "Select", "Start", "A", "B", "C", "X", "Y", "Z", "Mode2",
    "Mode6",
];
const NATIVE_BUTTONS: [&str; 14] = [
    "Up",
    "Down",
    "Left",
    "Right",
    "Select",
    "Run",
    "I",
    "II",
    "III",
    "IV",
    "V",
    "VI",
    "Mode: Set 2-button",
    "Mode: Set 6-button",
];
const CORE: &str = "BizHawk.Emulation.Cores.Consoles.NEC.PCE.TurboNyma";
const DECK: &str = "PC Engine Controller";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Topology {
    pub ports: [bool; 5],
    pub multitap: bool,
}

pub(crate) use super::digital_session::PlayerRequest;

/// Check prepared configuration, not loaded-runtime provenance.
pub(crate) fn validate_capture_config(
    source: &str,
    topology: Topology,
    system: &str,
) -> Result<()> {
    let sync = super::definition_capture::selected_sync_config(source, system, "TurboNyma", CORE)?;
    ensure!(
        sync["MednafenValues"]["pce.input.multitap"].as_str()
            == Some(if topology.multitap { "1" } else { "0" }),
        "TurboNyma capture config multitap differs from the request"
    );
    for (index, enabled) in topology.ports.into_iter().enumerate() {
        ensure!(
            sync["PortDevices"][index.to_string()].as_str()
                == Some(if enabled { "gamepad" } else { "none" }),
            "TurboNyma capture config port {} differs from the request",
            index + 1
        );
    }
    Ok(())
}

pub(crate) fn validate_definition(
    definition: &super::definition_capture::Definition,
    topology: Topology,
    expected_system: &str,
    expected_rom_hash: &str,
    has_discs: bool,
) -> Result<()> {
    validate_players(
        topology.ports,
        topology.multitap,
        topology
            .ports
            .iter()
            .enumerate()
            .filter(|(_, enabled)| **enabled)
            .map(|(index, _)| index as u8 + 1),
    )?;
    ensure!(
        matches!(expected_system, "PCE" | "SGX" | "PCECD" | "SGXCD"),
        "Unsupported expected TurboNyma system"
    );
    ensure!(
        has_discs == matches!(expected_system, "PCECD" | "SGXCD"),
        "TurboNyma expected system and disc mode disagree"
    );
    ensure!(
        !expected_rom_hash.is_empty()
            && expected_rom_hash.len() <= 256
            && !expected_rom_hash.chars().any(char::is_control),
        "Expected TurboNyma content hash is missing or invalid"
    );
    ensure!(
        definition.system == expected_system && definition.rom_hash == expected_rom_hash,
        "Captured TurboNyma system/content hash differs from the request"
    );
    let mut expected = std::collections::BTreeSet::from(["Power".to_owned(), "Reset".to_owned()]);
    for (index, enabled) in topology.ports.into_iter().enumerate() {
        if enabled {
            expected.extend(
                NATIVE_BUTTONS
                    .iter()
                    .map(|button| format!("P{} {button}", index + 1)),
            );
        }
    }
    if has_discs {
        expected.extend(["Open Tray".to_owned(), "Close Tray".to_owned()]);
    }
    let expected_axes: Vec<String> = if has_discs {
        vec!["Disk Index".to_owned()]
    } else {
        vec![]
    };
    ensure!(
        definition.axes == expected_axes,
        "Captured TurboNyma axes differ from the expected cartridge/CD definition"
    );
    let actual: std::collections::BTreeSet<_> = definition.buttons.iter().cloned().collect();
    ensure!(
        actual.len() == definition.buttons.len(),
        "Captured TurboNyma definition repeats controls"
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
        "Loaded TurboNyma controls differ from the request (first 12 each): missing [{}]; unexpected [{}]",
        missing.join(", "),
        extra.join(", ")
    );
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn capture_definition(
    process: super::definition_capture::CaptureProcess,
    topology: Topology,
    expected_system: &str,
    expected_rom_hash: &str,
    has_discs: bool,
    timeout: std::time::Duration,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<super::definition_capture::Definition> {
    let definition = process.wait_for_definition(timeout, cancel)?;
    validate_definition(
        &definition,
        topology,
        expected_system,
        expected_rom_hash,
        has_discs,
    )?;
    ensure!(
        !cancel.load(std::sync::atomic::Ordering::Relaxed),
        "TurboNyma definition comparison cancelled"
    );
    Ok(definition)
}

pub(crate) struct PreparedConfig {
    pub configuration: String,
    pub warnings: Vec<String>,
}

/// Compose bindings from a caller-supplied snapshot without probing devices.
pub(crate) fn prepare_config(
    source: &str,
    ports: [bool; 5],
    multitap: bool,
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<PreparedConfig> {
    validate_players(
        ports,
        multitap,
        requests.iter().map(|request| request.player),
    )?;
    let mut devices = std::collections::BTreeSet::new();
    for request in requests {
        let device = inventory.device_at_path(request.runtime_path)?;
        ensure!(
            devices.insert(device.device_index),
            "One SDL device cannot supply multiple TurboNyma players"
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
        .with_context(|| format!("Preparing TurboNyma player {}", request.player))?;
        players.insert(request.player, buttons);
        warnings.extend(
            notes
                .into_iter()
                .map(|note| format!("Player {}: {note}", request.player)),
        );
    }
    warnings.push("TurboNyma mode selectors set two-button or six-button mode explicitly; initial mode and content-driven device overrides are not verified by configuration preparation.".into());
    Ok(PreparedConfig {
        configuration: encode_config(source, ports, multitap, &players)?,
        warnings,
    })
}

/// Argument changes remain transactional, with the private config owner returned.
pub(crate) fn prepare_arguments(
    arguments: &mut Vec<std::ffi::OsString>,
    working_directory: &std::path::Path,
    exe_directory: &std::path::Path,
    ports: [bool; 5],
    multitap: bool,
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<(super::PreparedConfig, Vec<String>)> {
    let mut warnings = Vec::new();
    let config =
        super::prepare_arguments_using(arguments, working_directory, exe_directory, |source| {
            super::prepare_config_using(source, |text| {
                let prepared = prepare_config(text, ports, multitap, requests, inventory)?;
                warnings = prepared.warnings;
                Ok(prepared.configuration)
            })
        })?;
    Ok((config, warnings))
}

pub(crate) fn validate_players(
    ports: [bool; 5],
    multitap: bool,
    players: impl IntoIterator<Item = u8>,
) -> Result<()> {
    ensure!(
        multitap || !ports[1..].iter().any(|connected| *connected),
        "TurboNyma ports 2–5 require the multitap"
    );
    super::pcehawk::validate_players(ports, players)
}

/// Encode exact native bindings. This does not establish content overrides or
/// the initial switch state; mode selection remains an explicit input action.
pub(crate) fn encode_config(
    source: &str,
    ports: [bool; 5],
    multitap: bool,
    players: &BTreeMap<u8, BTreeMap<String, String>>,
) -> Result<String> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "BizHawk source config exceeds 16 MiB"
    );
    validate_players(ports, multitap, players.keys().copied())?;
    let mut buttons = Map::new();
    for (player, bindings) in players {
        ensure!(
            bindings.len() == NATIVE_BUTTONS.len()
                && NATIVE_BUTTONS
                    .iter()
                    .all(|name| bindings.contains_key(*name)),
            "TurboNyma player {player} needs all fourteen native controls"
        );
        let mut inputs = std::collections::BTreeSet::new();
        for (name, input) in bindings {
            ensure!(
                !input.trim().is_empty()
                    && input.len() <= 1024
                    && !input.chars().any(char::is_control)
                    && inputs.insert(input),
                "Invalid or duplicate TurboNyma input for player {player} {name}"
            );
            buttons.insert(format!("P{player} {name}"), Value::String(input.clone()));
        }
    }
    let mut config: Value = serde_json::from_str(source.trim_start_matches('\u{feff}'))
        .context("Parsing native BizHawk source config")?;
    let root = config
        .as_object_mut()
        .context("BizHawk config must be an object")?;
    let digital = super::object_child(root, "AllTrollers")?;
    if let Some(previous) = digital.get(DECK).and_then(Value::as_object) {
        for name in ["Power", "Reset", "Open Tray", "Close Tray"] {
            if let Some(binding) = previous.get(name).and_then(Value::as_str) {
                buttons.insert(name.into(), Value::String(binding.into()));
            }
        }
    }
    digital.insert(DECK.into(), Value::Object(buttons));
    let analog = super::object_child(root, "AllTrollersAnalog")?;
    let mut axes = Map::new();
    if let Some(binding) = analog
        .get(DECK)
        .and_then(Value::as_object)
        .and_then(|deck| deck.get("Disk Index"))
    {
        axes.insert("Disk Index".into(), binding.clone());
    }
    analog.insert(DECK.into(), Value::Object(axes));
    for field in ["AllTrollersFeedbacks", "AllTrollersAutoFire"] {
        super::object_child(root, field)?.insert(DECK.into(), Value::Object(Map::new()));
    }
    for system in ["PCE", "PCECD", "SGX", "SGXCD"] {
        super::object_child(root, "PreferredCores")?
            .insert(system.into(), Value::String("TurboNyma".into()));
    }
    root.insert("DontTryOtherCores".into(), Value::Bool(true));
    let sync = super::object_child(super::object_child(root, "CoreSyncSettings")?, CORE)?;
    super::object_child(sync, "MednafenValues")?.insert(
        "pce.input.multitap".into(),
        Value::String(if multitap { "1" } else { "0" }.into()),
    );
    let devices = super::object_child(sync, "PortDevices")?;
    for (port, connected) in ports.into_iter().enumerate() {
        devices.insert(
            port.to_string(),
            Value::String(if connected { "gamepad" } else { "none" }.into()),
        );
    }
    let mut encoded = serde_json::to_string_pretty(&config)?;
    encoded.push('\n');
    Ok(encoded)
}

pub(crate) fn validate_saved_pad(
    calibration: &crate::controller_catalog::Calibration,
) -> Result<()> {
    super::digital_pad::validate_saved_pad("pce-turbonyma", &BUTTONS, calibration)
}

pub(crate) fn translate_pad(
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let (semantic, warnings) = super::digital_pad::translate_pad(
        "pce-turbonyma",
        &BUTTONS,
        calibration,
        logical,
        inventory,
        path,
    )?;
    let native = semantic
        .into_iter()
        .map(|(name, input)| {
            // Nyma preserves Roman numerals, title-cases Run/Select/directions,
            // and appends each switch position's original name without title-casing.
            let name = match name.as_str() {
                "A" => "III".to_owned(),
                "B" => "II".to_owned(),
                "C" => "I".to_owned(),
                "X" => "IV".to_owned(),
                "Y" => "V".to_owned(),
                "Z" => "VI".to_owned(),
                "Start" => "Run".to_owned(),
                "Mode2" => "Mode: Set 2-button".to_owned(),
                "Mode6" => "Mode: Set 6-button".to_owned(),
                _ => name,
            };
            (name, input)
        })
        .collect();
    Ok((native, warnings))
}
