//! SMSHawk digital contract, BizHawk 8c6b8958bbbe623eaaa36bc82af858b812893628.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const CORE: &str = "BizHawk.Emulation.Cores.Sega.MasterSystem.SMS";
const SMS_BUTTONS: [&str; 6] = ["Up", "Down", "Left", "Right", "B1", "B2"];
const GG_BUTTONS: [&str; 7] = ["Up", "Down", "Left", "Right", "B1", "B2", "Start"];

pub(crate) use super::digital_session::PlayerRequest;

/// Check prepared configuration, not loaded-runtime provenance.
pub(crate) fn validate_capture_config(source: &str, system: System) -> Result<()> {
    let sync = super::definition_capture::selected_sync_config(
        source,
        system.system_id(),
        "SMSHawk",
        CORE,
    )?;
    ensure!(
        sync["Port1"].as_u64() == Some(0)
            && sync["Port2"].as_u64() == Some(0)
            && sync["UseKeyboard"].as_bool() == Some(false),
        "SMSHawk capture config does not select the standard digital deck"
    );
    Ok(())
}

/// Validate the whole declared deck, not only the selected mapped players.
pub(crate) fn validate_definition(
    definition: &super::definition_capture::Definition,
    system: System,
    expected_rom_hash: &str,
) -> Result<()> {
    ensure!(
        !expected_rom_hash.is_empty()
            && expected_rom_hash.len() <= 256
            && !expected_rom_hash.chars().any(char::is_control),
        "Expected SMSHawk content hash is missing or invalid"
    );
    ensure!(
        definition.system == system.system_id() && definition.rom_hash == expected_rom_hash,
        "Captured SMSHawk system/content hash differs from the request"
    );
    ensure!(
        definition.axes.is_empty(),
        "SMSHawk digital capture contains unexpected axes"
    );
    let mut expected = std::collections::BTreeSet::from(["Reset".to_owned()]);
    if system != System::GameGear {
        expected.insert("Pause".to_owned());
    }
    // Both SMS/SG ports remain declared even when only P1 or P2 is mapped.
    for player in 1..=system.capacity() {
        expected.extend(
            system
                .buttons()
                .iter()
                .map(|button| format!("P{player} {button}")),
        );
    }
    let actual: std::collections::BTreeSet<_> = definition.buttons.iter().cloned().collect();
    ensure!(
        actual.len() == definition.buttons.len(),
        "Captured SMSHawk definition repeats controls"
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
        "Loaded SMSHawk controls differ from the request (first 12 each): missing [{}]; unexpected [{}]",
        missing.join(", "),
        extra.join(", ")
    );
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn capture_definition(
    process: super::definition_capture::CaptureProcess,
    system: System,
    expected_rom_hash: &str,
    timeout: std::time::Duration,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<super::definition_capture::Definition> {
    let definition = process.wait_for_definition(timeout, cancel)?;
    validate_definition(&definition, system, expected_rom_hash)?;
    ensure!(
        !cancel.load(std::sync::atomic::Ordering::Relaxed),
        "SMSHawk definition comparison cancelled"
    );
    Ok(definition)
}

pub(crate) struct PreparedConfig {
    pub configuration: String,
    pub warnings: Vec<String>,
}

/// Compose a caller-supplied SDL snapshot into native bindings without probing.
pub(crate) fn prepare_config(
    source: &str,
    system: System,
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<PreparedConfig> {
    system.validate_players(requests.iter().map(|request| request.player))?;
    let mut devices = std::collections::BTreeSet::new();
    for request in requests {
        let device = inventory.device_at_path(request.runtime_path)?;
        ensure!(
            devices.insert(device.device_index),
            "One SDL device cannot supply multiple SMSHawk players"
        );
    }
    let mut players = BTreeMap::new();
    let mut warnings = Vec::new();
    for request in requests {
        let (buttons, notes) = translate_pad(
            system,
            request.calibration,
            request.logical,
            inventory,
            request.runtime_path,
        )
        .with_context(|| format!("Preparing SMSHawk player {}", request.player))?;
        players.insert(request.player, buttons);
        warnings.extend(
            notes
                .into_iter()
                .map(|note| format!("Player {}: {note}", request.player)),
        );
    }
    Ok(PreparedConfig {
        configuration: encode_config(source, system, &players)?,
        warnings,
    })
}

/// Keep argument changes transactional and retain the private configuration owner.
pub(crate) fn prepare_arguments(
    arguments: &mut Vec<std::ffi::OsString>,
    working_directory: &std::path::Path,
    exe_directory: &std::path::Path,
    system: System,
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<(super::PreparedConfig, Vec<String>)> {
    let mut warnings = Vec::new();
    let config =
        super::prepare_arguments_using(arguments, working_directory, exe_directory, |source| {
            super::prepare_config_using(source, |text| {
                let prepared = prepare_config(text, system, requests, inventory)?;
                warnings = prepared.warnings;
                Ok(prepared.configuration)
            })
        })?;
    Ok((config, warnings))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum System {
    MasterSystem,
    GameGear,
    Sg1000,
}

impl System {
    pub(crate) fn capacity(self) -> u8 {
        if self == Self::GameGear { 1 } else { 2 }
    }
    pub(crate) fn validate_players(self, players: impl IntoIterator<Item = u8>) -> Result<()> {
        let max = self.capacity();
        let mut selected = std::collections::BTreeSet::new();
        for player in players {
            ensure!(
                (1..=max).contains(&player) && selected.insert(player),
                "Invalid or duplicate SMSHawk player {player}; Game Gear exposes only player one"
            );
        }
        ensure!(
            !selected.is_empty(),
            "Select at least one SMSHawk controller"
        );
        Ok(())
    }
    pub(crate) fn layout_id(self) -> &'static str {
        match self {
            Self::MasterSystem => "master-system",
            Self::GameGear => "gamegear",
            Self::Sg1000 => "sg1000",
        }
    }
    fn semantic_buttons(self) -> &'static [&'static str] {
        match self {
            Self::MasterSystem | Self::Sg1000 => &["Up", "Down", "Left", "Right", "B", "A"],
            Self::GameGear => &["Up", "Down", "Left", "Right", "B", "A", "Start"],
        }
    }
    fn deck(self) -> &'static str {
        match self {
            Self::MasterSystem | Self::Sg1000 => "SMS Controller",
            Self::GameGear => "GG Controller",
        }
    }
    fn system_id(self) -> &'static str {
        match self {
            Self::MasterSystem => "SMS",
            Self::GameGear => "GG",
            Self::Sg1000 => "SG",
        }
    }
    pub(crate) fn buttons(self) -> &'static [&'static str] {
        match self {
            Self::MasterSystem | Self::Sg1000 => &SMS_BUTTONS,
            Self::GameGear => &GG_BUTTONS,
        }
    }
}

pub(crate) fn validate_saved_pad(
    system: System,
    calibration: &crate::controller_catalog::Calibration,
) -> Result<()> {
    super::digital_pad::validate_saved_pad(
        system.layout_id(),
        system.semantic_buttons(),
        calibration,
    )
}

pub(crate) fn translate_pad(
    system: System,
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let (semantic, warnings) = super::digital_pad::translate_pad(
        system.layout_id(),
        system.semantic_buttons(),
        calibration,
        logical,
        inventory,
        path,
    )?;
    // Catalog b/a are printed Sega buttons 1/2, not native B1/B2 identifiers.
    let native = semantic
        .into_iter()
        .map(|(name, input)| {
            let name = match name.as_str() {
                "B" => "B1".to_owned(),
                "A" => "B2".to_owned(),
                _ => name,
            };
            (name, input)
        })
        .collect();
    Ok((native, warnings))
}

/// Inputs must already be translated to native strings for the selected SDL
/// devices. SMS keeps two standard ports; unassigned ports have no host bindings.
pub(crate) fn encode_config(
    source: &str,
    system: System,
    players: &BTreeMap<u8, BTreeMap<String, String>>,
) -> Result<String> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "BizHawk source config exceeds 16 MiB"
    );
    system.validate_players(players.keys().copied())?;
    let mut buttons = Map::new();
    for (player, bindings) in players {
        ensure!(
            bindings.len() == system.buttons().len()
                && system
                    .buttons()
                    .iter()
                    .all(|name| bindings.contains_key(*name)),
            "SMSHawk player {player} needs exactly the selected system's digital controls"
        );
        for (name, input) in bindings {
            ensure!(
                !input.trim().is_empty()
                    && input.len() <= 1024
                    && !input.chars().any(char::is_control),
                "Invalid SMSHawk native binding for player {player} {name}"
            );
            buttons.insert(format!("P{player} {name}"), Value::String(input.clone()));
        }
    }
    let mut config: Value = serde_json::from_str(source.trim_start_matches('\u{feff}'))
        .context("Parsing native BizHawk source config")?;
    let root = config
        .as_object_mut()
        .context("BizHawk config must be an object")?;
    let decks = super::object_child(root, "AllTrollers")?;
    if let Some(previous) = decks.get(system.deck()) {
        let previous = previous
            .as_object()
            .context("SMSHawk controller bindings must be an object")?;
        for name in ["Reset", "Pause"] {
            if name == "Pause" && system == System::GameGear {
                continue;
            }
            if let Some(input) = previous.get(name) {
                ensure!(input.is_string(), "SMSHawk {name} binding must be a string");
                buttons.insert(name.into(), input.clone());
            }
        }
    }
    decks.insert(system.deck().into(), Value::Object(buttons));
    for field in [
        "AllTrollersAnalog",
        "AllTrollersFeedbacks",
        "AllTrollersAutoFire",
    ] {
        super::object_child(root, field)?.insert(system.deck().into(), Value::Object(Map::new()));
    }
    super::object_child(root, "PreferredCores")?
        .insert(system.system_id().into(), Value::String("SMSHawk".into()));
    root.insert("DontTryOtherCores".into(), Value::Bool(true));
    let sync = super::object_child(super::object_child(root, "CoreSyncSettings")?, CORE)?;
    sync.insert("Port1".into(), Value::from(0));
    sync.insert("Port2".into(), Value::from(0));
    sync.insert("UseKeyboard".into(), Value::Bool(false));
    let mut encoded = serde_json::to_string_pretty(&config)?;
    encoded.push('\n');
    Ok(encoded)
}
