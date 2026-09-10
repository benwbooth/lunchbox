//! Native BizHawk Snes9x config contract at 8c6b8958bbbe623eaaa36bc82af858b812893628.
//! Host strings must already be translated and bound to verified device identity.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const CORE: &str = "BizHawk.Emulation.Cores.Nintendo.SNES9X.Snes9x";
const DECK: &str = "SNES Controller";
pub(crate) const BUTTONS: [&str; 12] = [
    "B", "Y", "Select", "Start", "Up", "Down", "Left", "Right", "A", "X", "L", "R",
];

/// Compare the loaded definition with the selected digital deck, not with
/// whichever controls happened to be pressed. Provenance remains separate.
pub(crate) fn validate_definition(
    definition: &super::definition_capture::Definition,
    ports: [PadPort; 2],
    expected_rom_hash: &str,
) -> Result<()> {
    let count = ports.iter().map(|port| port.players()).sum::<u8>();
    ensure!(count > 0, "No Snes9x controllers requested");
    ensure!(
        !expected_rom_hash.is_empty()
            && expected_rom_hash.len() <= 256
            && !expected_rom_hash.chars().any(char::is_control),
        "Expected Snes9x content hash is missing or invalid"
    );
    ensure!(
        definition.system == "SNES",
        "Captured definition is not SNES"
    );
    ensure!(
        definition.rom_hash == expected_rom_hash,
        "Captured Snes9x content hash differs from the requested content"
    );
    ensure!(
        definition.axes.is_empty(),
        "Captured Snes9x digital deck has unexpected axes"
    );
    let mut expected = std::collections::BTreeSet::from(["Reset".to_owned(), "Power".to_owned()]);
    for player in 1..=count {
        expected.extend(BUTTONS.iter().map(|button| format!("P{player} {button}")));
    }
    let actual: std::collections::BTreeSet<_> = definition.buttons.iter().cloned().collect();
    ensure!(
        actual.len() == definition.buttons.len(),
        "Captured Snes9x definition repeats controls"
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
        "Loaded Snes9x controls differ from the requested mapping (first 12 each): missing [{}]; unexpected [{}]",
        missing.join(", "),
        extra.join(", ")
    );
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn capture_definition(
    process: super::definition_capture::CaptureProcess,
    ports: [PadPort; 2],
    expected_rom_hash: &str,
    timeout: std::time::Duration,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<super::definition_capture::Definition> {
    let definition = process.wait_for_definition(timeout, cancel)?;
    validate_definition(&definition, ports, expected_rom_hash)?;
    ensure!(
        !cancel.load(std::sync::atomic::Ordering::Relaxed),
        "Snes9x definition comparison cancelled"
    );
    Ok(definition)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PadPort {
    None,
    Joypad,
    Multitap,
}

impl PadPort {
    pub(crate) fn players(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Joypad => 1,
            Self::Multitap => 4,
        }
    }
    fn native_value(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Joypad => 1,
            Self::Multitap => 2,
        }
    }
}

pub(crate) use super::digital_session::PlayerRequest;

/// Check prepared configuration, not loaded-runtime provenance.
pub(crate) fn validate_capture_config(source: &str, ports: [PadPort; 2]) -> Result<()> {
    let sync = super::definition_capture::selected_sync_config(source, "SNES", "Snes9x", CORE)?;
    ensure!(
        sync["LeftPort"].as_u64() == Some(u64::from(ports[0].native_value()))
            && sync["RightPort"].as_u64() == Some(u64::from(ports[1].native_value())),
        "Snes9x capture config ports differ from the request"
    );
    Ok(())
}

pub(crate) struct PreparedConfig {
    pub configuration: String,
    pub warnings: Vec<String>,
}

/// Prepare a launch-owned native digital-controller session.
#[cfg(target_os = "linux")]
pub(crate) fn prepare_session(
    plan: &mut crate::emulator::LaunchPlan,
    exe_directory: &std::path::Path,
    probe_program: &std::path::Path,
    sdl_library: &std::path::Path,
    ports: [PadPort; 2],
    requests: &[PlayerRequest<'_>],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<crate::controller_launch::CalibratedLaunch> {
    super::digital_session::prepare_session(
        plan,
        exe_directory,
        probe_program,
        sdl_library,
        super::digital_session::DigitalDeck::Snes9x(ports),
        requests,
        cancel,
    )
}

/// Reuse the native owned-config transaction and argument parsing. On failure
/// arguments remain unchanged; the returned owner keeps the private config alive.
pub(crate) fn prepare_arguments(
    arguments: &mut Vec<std::ffi::OsString>,
    working_directory: &std::path::Path,
    exe_directory: &std::path::Path,
    ports: [PadPort; 2],
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

/// Assemble translated pads and encode one owned configuration. The caller
/// supplies a fresh snapshot from the installation's SDL library and keeps
/// devices/processes alive; this function does no probing or filesystem writes.
pub(crate) fn prepare_config(
    source: &str,
    ports: [PadPort; 2],
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<PreparedConfig> {
    let count = ports.iter().map(|port| port.players()).sum::<u8>();
    ensure!(
        count > 0 && requests.len() == usize::from(count),
        "Snes9x needs one controller for every configured joypad slot"
    );
    let mut slots = std::collections::BTreeSet::new();
    let mut devices = std::collections::BTreeSet::new();
    // Reject topology and duplicate ownership before translating any player.
    for request in requests {
        ensure!(
            (1..=count).contains(&request.player) && slots.insert(request.player),
            "Invalid or duplicate Snes9x logical player {}",
            request.player
        );
        let device = inventory.device_at_path(request.runtime_path)?;
        ensure!(
            devices.insert(device.device_index),
            "One SDL device cannot supply multiple Snes9x players"
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
        .with_context(|| format!("Preparing Snes9x player {}", request.player))?;
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

/// Saved-input completeness, not native SDL eligibility.
pub(crate) fn validate_saved_pad(
    calibration: &crate::controller_catalog::Calibration,
) -> Result<()> {
    super::digital_pad::validate_saved_pad("snes", &BUTTONS, calibration)
}

/// Translate against the actual probed SDL device, never a saved device index.
pub(crate) fn translate_pad(
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    super::digital_pad::translate_pad("snes", &BUTTONS, calibration, logical, inventory, path)
}

/// One-based logical players in native left-deck-then-right-deck order.
/// This encoder neither probes hardware nor configures pointer peripherals.
pub(crate) fn encode_config(
    source: &str,
    ports: [PadPort; 2],
    players: &BTreeMap<u8, BTreeMap<String, String>>,
) -> Result<String> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "BizHawk source config exceeds 16 MiB"
    );
    let count = ports.iter().map(|port| port.players()).sum::<u8>();
    ensure!(
        count > 0
            && !players.is_empty()
            && players.keys().all(|player| (1..=count).contains(player)),
        "Select valid Snes9x logical slots; unassigned slots remain unbound"
    );
    // BindToDefinition only attaches keys present in this replacement map.
    // Unassigned native slots therefore stay unbound, including partial taps.
    let mut buttons = Map::new();
    for (player, bindings) in players {
        ensure!(
            bindings.len() == BUTTONS.len()
                && BUTTONS.iter().all(|button| bindings.contains_key(*button)),
            "Snes9x player {player} needs exactly twelve native button bindings"
        );
        for (button, input) in bindings {
            ensure!(
                !input.trim().is_empty()
                    && input.len() <= 1024
                    && !input.chars().any(char::is_control),
                "Invalid native input for Snes9x player {player} {button}"
            );
            buttons.insert(format!("P{player} {button}"), Value::String(input.clone()));
        }
    }
    let mut config: Value = serde_json::from_str(source.trim_start_matches('\u{feff}'))
        .context("Parsing native BizHawk source config")?;
    let root = config
        .as_object_mut()
        .context("BizHawk config must be an object")?;
    let decks = super::object_child(root, "AllTrollers")?;
    if let Some(previous) = decks.get(DECK) {
        let previous = previous
            .as_object()
            .context("SNES controller bindings must be an object")?;
        for key in ["Reset", "Power"] {
            if let Some(value) = previous.get(key) {
                ensure!(value.is_string(), "SNES {key} binding must be a string");
                buttons.insert(key.into(), value.clone());
            }
        }
    }
    decks.insert(DECK.into(), Value::Object(buttons));
    for field in [
        "AllTrollersAnalog",
        "AllTrollersFeedbacks",
        "AllTrollersAutoFire",
    ] {
        super::object_child(root, field)?.insert(DECK.into(), Value::Object(Map::new()));
    }
    super::object_child(root, "PreferredCores")?
        .insert("SNES".into(), Value::String("Snes9x".into()));
    root.insert("DontTryOtherCores".into(), Value::Bool(true));
    let settings = super::object_child(super::object_child(root, "CoreSyncSettings")?, CORE)?;
    settings.insert("LeftPort".into(), Value::from(ports[0].native_value()));
    settings.insert("RightPort".into(), Value::from(ports[1].native_value()));
    // Preserve existing serializer metadata and unrelated sync settings.
    let mut encoded = serde_json::to_string_pretty(&config)?;
    encoded.push('\n');
    Ok(encoded)
}
