//! GPGX digital pad names, BizHawk 8c6b8958bbbe623eaaa36bc82af858b812893628.
//! Ordinary-pad contract used by shared digital-session preparation.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const CORE: &str = "BizHawk.Emulation.Cores.Consoles.Sega.gpgx.GPGX";
const DECK: &str = "GPGX Genesis Controller";
const ACTIVATOR_BUTTONS: [&str; 16] = [
    "1L", "1U", "2L", "2U", "3L", "3U", "4L", "4U", "5L", "5U", "6L", "6U", "7L", "7U", "8L", "8U",
];

/// Consume an owned capture and compare its result with the requested GPGX
/// mapping. This is a definition match, not authenticated core/ROM provenance.
#[cfg(target_os = "linux")]
pub(crate) fn capture_definition(
    process: super::definition_capture::CaptureProcess,
    topology: Topology,
    expected_rom_hash: &str,
    has_discs: bool,
    timeout: std::time::Duration,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<super::definition_capture::Definition> {
    topology.validate()?;
    ensure!(topology.players() > 0, "No GPGX controllers requested");
    validate_expected_hash(expected_rom_hash)?;
    let definition = process.wait_for_definition(timeout, cancel)?;
    validate_definition(&definition, topology, expected_rom_hash, has_discs)?;
    ensure!(
        !cancel.load(std::sync::atomic::Ordering::Relaxed),
        "GPGX definition comparison cancelled"
    );
    Ok(definition)
}

fn validate_expected_hash(expected_rom_hash: &str) -> Result<()> {
    ensure!(
        !expected_rom_hash.is_empty()
            && expected_rom_hash.len() <= 256
            && !expected_rom_hash.chars().any(char::is_control),
        "Expected GPGX content hash is missing or invalid"
    );
    Ok(())
}

/// Compare a parsed definition against the requested digital contract. Callers
/// must separately establish capture provenance and core/runtime identity.
pub(crate) fn validate_definition(
    definition: &super::definition_capture::Definition,
    topology: Topology,
    expected_rom_hash: &str,
    has_discs: bool,
) -> Result<()> {
    topology.validate()?;
    ensure!(topology.players() > 0, "No GPGX controllers requested");
    validate_expected_hash(expected_rom_hash)?;
    ensure!(
        definition.system == "GEN",
        "Captured definition is not Genesis/CD"
    );
    ensure!(
        definition.rom_hash == expected_rom_hash,
        "Captured GPGX content hash does not match the requested content"
    );
    ensure!(
        definition.axes.is_empty(),
        "Captured GPGX definition has unexpected analog controls"
    );
    let mut expected = std::collections::BTreeSet::from(["Power".to_owned(), "Reset".to_owned()]);
    if has_discs {
        expected.extend(["Previous Disk".to_owned(), "Next Disk".to_owned()]);
    }
    for player in 1..=topology.players() {
        for button in topology.buttons(player)? {
            expected.insert(format!("P{player} {button}"));
        }
    }
    let actual: std::collections::BTreeSet<_> = definition.buttons.iter().cloned().collect();
    ensure!(
        actual.len() == definition.buttons.len(),
        "Captured GPGX definition repeats controls"
    );
    let missing: Vec<_> = expected.difference(&actual).take(12).cloned().collect();
    let unexpected: Vec<_> = actual.difference(&expected).take(12).cloned().collect();
    ensure!(
        actual == expected,
        "Loaded GPGX controls differ from the requested mapping (first 12 each): missing [{}]; unexpected [{}]",
        missing.join(", "),
        unexpected.join(", ")
    );
    Ok(())
}

/// Check the encoder's explicit selection fields before an inspection launch.
/// Config agreement does not prove which core or game-selected devices loaded.
pub(crate) fn validate_capture_config(source: &str, topology: Topology) -> Result<()> {
    topology.validate()?;
    ensure!(topology.players() > 0, "No GPGX controllers requested");
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "GPGX capture config exceeds 16 MiB"
    );
    let value: Value = serde_json::from_str(source)?;
    ensure!(
        value.is_object(),
        "GPGX capture config root must be an object"
    );
    ensure!(
        value["PreferredCores"]["GEN"].as_str() == Some("Genplus-gx")
            && value["DontTryOtherCores"].as_bool() == Some(true),
        "GPGX capture config must explicitly select Genplus-gx without fallback"
    );
    let sync = &value["CoreSyncSettings"][CORE];
    let [left, right] = topology.native_port_types();
    ensure!(
        sync["UseSixButton"].as_bool() == Some(topology.pad == Pad::SixButton)
            && sync["ControlTypeLeft"].as_u64() == Some(u64::from(left))
            && sync["ControlTypeRight"].as_u64() == Some(u64::from(right)),
        "GPGX capture config controller topology differs from the request"
    );
    Ok(())
}

pub(crate) fn validate_activator_inputs(
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
) -> Result<()> {
    super::digital_pad::validate_saved_pad("genesis-activator", &ACTIVATOR_BUTTONS, calibration)?;
    let resolution = super::digital_pad::resolve_pad("genesis-activator", calibration, logical)?;
    let mut identities = std::collections::BTreeSet::new();
    let mut outputs = std::collections::BTreeSet::new();
    for name in ACTIVATOR_BUTTONS {
        let assigned = resolution
            .assignments
            .get(&name.to_ascii_lowercase())
            .with_context(|| format!("Activator {name} has no calibrated input"))?;
        let native = calibration
            .bindings
            .get(assigned)
            .and_then(|input| input.native.as_ref())
            .context("Activator source has no native identity")?;
        ensure!(
            identities.insert(native.code),
            "Activator sensors require independent physical inputs; {name} shares a button or axis"
        );
        if let Some(logical) = logical {
            let gesture = logical
                .bindings
                .get(assigned)
                .context("Activator SDL gesture is absent")?;
            ensure!(
                outputs.insert(&gesture.output),
                "Activator sensors require independent SDL outputs; {name} shares an output"
            );
        }
    }
    Ok(())
}

/// Sensor translation only; Activator connector/session selection is unfinished.
pub(crate) fn translate_activator(
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let translated = super::digital_pad::translate_pad(
        "genesis-activator",
        &ACTIVATOR_BUTTONS,
        calibration,
        logical,
        inventory,
        path,
    )?;
    let logical = if inventory.device_at_path(path)?.is_game_controller {
        logical
    } else {
        None
    };
    validate_activator_inputs(calibration, logical)?;
    Ok(translated)
}

pub(crate) use super::digital_session::PlayerRequest;

pub(crate) struct PreparedConfig {
    pub configuration: String,
    pub warnings: Vec<String>,
}

/// Compose a supplied SDL snapshot; previews do not invoke runtime discovery.
pub(crate) fn prepare_config(
    source: &str,
    topology: Topology,
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<PreparedConfig> {
    topology.validate_players(requests.iter().map(|request| request.player))?;
    let mut devices = std::collections::BTreeSet::new();
    for request in requests {
        let device = inventory.device_at_path(request.runtime_path)?;
        ensure!(
            devices.insert(device.device_index),
            "One SDL device cannot supply multiple GPGX players"
        );
    }
    let mut players = BTreeMap::new();
    let mut warnings = Vec::new();
    for request in requests {
        let connector = topology.physical_port(request.player)?;
        let translated = if topology.activators[connector] {
            translate_activator(
                request.calibration,
                request.logical,
                inventory,
                request.runtime_path,
            )
        } else {
            translate_pad(
                topology.pad,
                request.calibration,
                request.logical,
                inventory,
                request.runtime_path,
            )
        };
        let (buttons, notes) = translated.with_context(|| {
            format!(
                "Preparing GPGX player {} on connector {}",
                request.player,
                connector + 1
            )
        })?;
        players.insert(request.player, buttons);
        warnings.extend(
            notes
                .into_iter()
                .map(|note| format!("Player {}: {note}", request.player)),
        );
        warnings.push(format!("Player {} uses requested {} routing with a {} pad; this is not a loaded-core device receipt.",
            request.player, if topology.wayplay { "4-Way Play (both connectors)" } else if connector == 0 { "left connector" } else { "right connector" }, topology.layout_id(request.player)?));
    }
    warnings.push("GPGX may override requested controllers for specific games. Actual loaded devices and compatibility remain unverified.".into());
    Ok(PreparedConfig {
        configuration: encode_config(source, topology, &players)?,
        warnings,
    })
}

/// Retain the private configuration owner and leave arguments unchanged on error.
pub(crate) fn prepare_arguments(
    arguments: &mut Vec<std::ffi::OsString>,
    working_directory: &std::path::Path,
    exe_directory: &std::path::Path,
    topology: Topology,
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<(super::PreparedConfig, Vec<String>)> {
    let mut warnings = Vec::new();
    let config =
        super::prepare_arguments_using(arguments, working_directory, exe_directory, |source| {
            super::prepare_config_using(source, |text| {
                let prepared = prepare_config(text, topology, requests, inventory)?;
                warnings = prepared.warnings;
                Ok(prepared.configuration)
            })
        })?;
    Ok((config, warnings))
}

/// Encode requested normal-pad topology, not a receipt for the loaded game's
/// actual devices. GPGX can apply content-driven controller overrides.
pub(crate) fn encode_config(
    source: &str,
    topology: Topology,
    players: &BTreeMap<u8, BTreeMap<String, String>>,
) -> Result<String> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "BizHawk source config exceeds 16 MiB"
    );
    topology.validate_players(players.keys().copied())?;
    // Replace, do not merge: slots without a physical player remain unbound.
    let mut buttons = Map::new();
    for (player, bindings) in players {
        let required = topology.buttons(*player)?;
        ensure!(
            bindings.len() == required.len()
                && required.iter().all(|name| bindings.contains_key(*name)),
            "GPGX player {player} needs exactly the selected pad's native controls"
        );
        let mut inputs = std::collections::BTreeSet::new();
        for (name, input) in bindings {
            ensure!(
                !input.trim().is_empty()
                    && input.len() <= 1024
                    && !input.chars().any(char::is_control)
                    && inputs.insert(input),
                "Invalid or duplicate GPGX input for player {player} {name}"
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
        for name in ["Power", "Reset", "Previous Disk", "Next Disk"] {
            if let Some(binding) = previous.get(name).and_then(Value::as_str) {
                buttons.insert(name.into(), Value::String(binding.into()));
            }
        }
    }
    digital.insert(DECK.into(), Value::Object(buttons));
    for field in [
        "AllTrollersAnalog",
        "AllTrollersFeedbacks",
        "AllTrollersAutoFire",
    ] {
        super::object_child(root, field)?.insert(DECK.into(), Value::Object(Map::new()));
    }
    // This encoder targets Genesis/CD, not GPGX's SMS/GG/SG controller decks.
    super::object_child(root, "PreferredCores")?
        .insert("GEN".into(), Value::String("Genplus-gx".into()));
    root.insert("DontTryOtherCores".into(), Value::Bool(true));
    let sync = super::object_child(super::object_child(root, "CoreSyncSettings")?, CORE)?;
    sync.insert(
        "UseSixButton".into(),
        Value::Bool(topology.pad == Pad::SixButton),
    );
    let [left, right] = topology.native_port_types();
    sync.insert("ControlTypeLeft".into(), Value::from(left));
    sync.insert("ControlTypeRight".into(), Value::from(right));
    let mut encoded = serde_json::to_string_pretty(&config)?;
    encoded.push('\n');
    Ok(encoded)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pad {
    ThreeButton,
    SixButton,
}

/// GPGX applies one pad mode to all attached normal controllers.
/// Team Player expands one connector; 4-Way Play occupies both connectors.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Topology {
    pub ports: [bool; 2],
    pub pad: Pad,
    #[serde(default)]
    pub team_players: [bool; 2],
    #[serde(default)]
    pub wayplay: bool,
    #[serde(default)]
    pub activators: [bool; 2],
}

/// Post-load gpgx_get_control system/dev fields, not requested settings.
/// Acquisition must bind these values to the loaded content/runtime before
/// treating them as launch evidence. This type alone provides no such binding.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoadedInputs {
    pub systems: [u8; 2],
    pub devices: [u8; 8],
}

impl LoadedInputs {
    /// Return actual native device-array indices in compacted player order.
    /// Never infer these indices from the requested connector selection.
    pub(crate) fn validate_normal_pads(&self, topology: Topology) -> Result<Vec<usize>> {
        topology.validate()?;
        ensure!(topology.players() > 0, "No requested GPGX normal pads");
        ensure!(
            self.systems == topology.native_systems(),
            "Loaded GPGX connector systems differ from the requested normal-pad topology"
        );
        let pad_type = match topology.pad {
            Pad::ThreeButton => 0x00,
            Pad::SixButton => 0x01,
        };
        let mut slots = Vec::new();
        for (slot, device) in self.devices.iter().copied().enumerate() {
            if device == 0xff {
                continue;
            }
            let expected = if topology.activators[slot / 4] {
                0x0a
            } else {
                pad_type
            };
            ensure!(
                device == expected,
                "Loaded GPGX device slot {slot} has type {device:#04x}, expected normal pad {expected:#04x}"
            );
            slots.push(slot);
        }
        ensure!(
            slots == topology.device_slots(),
            "Loaded GPGX pad slots differ from the requested topology"
        );
        Ok(slots)
    }
}

impl Topology {
    pub(crate) fn validate(self) -> Result<()> {
        ensure!(
            self.activators
                .iter()
                .enumerate()
                .all(|(port, active)| !*active
                    || (self.ports[port] && !self.team_players[port] && !self.wayplay)),
            "GPGX Activator requires an enabled connector without Team Player or 4-Way Play"
        );
        ensure!(
            !self.wayplay || (self.ports == [true, true] && self.team_players == [false, false]),
            "GPGX 4-Way Play requires both connectors and cannot share them with Team Player"
        );
        ensure!(
            self.team_players
                .iter()
                .zip(self.ports)
                .all(|(tap, connected)| !*tap || connected),
            "GPGX Team Player requires its connector to be enabled"
        );
        Ok(())
    }

    /// Exact device-array placement for the requested topology, absent content overrides.
    pub(crate) fn device_slots(self) -> Vec<usize> {
        if self.wayplay {
            return (0..4).collect();
        }
        let mut slots = Vec::new();
        for port in 0..2 {
            if self.ports[port] {
                let count = if self.team_players[port] { 4 } else { 1 };
                slots.extend(port * 4..port * 4 + count);
            }
        }
        slots
    }

    pub(crate) fn players(self) -> u8 {
        self.device_slots().len() as u8
    }

    pub(crate) fn layout_id(self, player: u8) -> Result<&'static str> {
        let port = self.physical_port(player)?;
        Ok(if self.activators[port] {
            "genesis-activator"
        } else {
            self.pad.layout_id()
        })
    }

    fn buttons(self, player: u8) -> Result<&'static [&'static str]> {
        let port = self.physical_port(player)?;
        Ok(if self.activators[port] {
            &ACTIVATOR_BUTTONS
        } else {
            self.pad.buttons()
        })
    }

    pub(crate) fn validate_saved_player(
        self,
        player: u8,
        calibration: &crate::controller_catalog::Calibration,
    ) -> Result<()> {
        if self.activators[self.physical_port(player)?] {
            validate_activator_inputs(calibration, None)
        } else {
            validate_saved_pad(self.pad, calibration)
        }
    }

    /// Native player prefixes compact past DEVICE_NONE; a right-only pad is P1.
    pub(crate) fn physical_port(self, player: u8) -> Result<usize> {
        self.validate()?;
        ensure!(
            (1..=self.players()).contains(&player),
            "Invalid GPGX player {player}"
        );
        self.device_slots()
            .get(usize::from(player - 1))
            .map(|slot| slot / 4)
            .ok_or_else(|| anyhow::anyhow!("No connected GPGX port for player {player}"))
    }

    pub(crate) fn validate_players(self, players: impl IntoIterator<Item = u8>) -> Result<()> {
        self.validate()?;
        let mut assigned = std::collections::BTreeSet::new();
        for player in players {
            self.physical_port(player)?;
            ensure!(assigned.insert(player), "Duplicate GPGX player {player}");
        }
        ensure!(
            !assigned.is_empty(),
            "Select at least one connected GPGX pad; unassigned slots remain unbound"
        );
        Ok(())
    }

    /// Serialized GPGX.ControlType discriminants, not libretro device IDs.
    pub(crate) fn native_port_types(self) -> [u8; 2] {
        if self.wayplay {
            return [5, 5];
        }
        std::array::from_fn(|port| {
            if self.team_players[port] {
                4
            } else if self.activators[port] {
                3
            } else {
                u8::from(self.ports[port])
            }
        })
    }

    /// Runtime INPUT_SYSTEM codes differ from serialized ControlType values.
    pub(crate) fn native_systems(self) -> [u8; 2] {
        if self.wayplay {
            return [13, 13];
        }
        std::array::from_fn(|port| {
            if self.team_players[port] {
                12
            } else if self.activators[port] {
                6
            } else {
                u8::from(self.ports[port])
            }
        })
    }
}

impl Pad {
    pub(crate) fn layout_id(self) -> &'static str {
        match self {
            Self::ThreeButton => "genesis-3",
            Self::SixButton => "genesis-6",
        }
    }

    pub(crate) fn buttons(self) -> &'static [&'static str] {
        match self {
            Self::ThreeButton => &["Up", "Down", "Left", "Right", "A", "B", "C", "Start"],
            Self::SixButton => &[
                "Up", "Down", "Left", "Right", "A", "B", "C", "Start", "X", "Y", "Z", "Mode",
            ],
        }
    }
}

pub(crate) fn validate_saved_pad(
    pad: Pad,
    calibration: &crate::controller_catalog::Calibration,
) -> Result<()> {
    super::digital_pad::validate_saved_pad(pad.layout_id(), pad.buttons(), calibration)
}

/// Use native names directly; no libretro button numbering enters this path.
pub(crate) fn translate_pad(
    pad: Pad,
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    super::digital_pad::translate_pad(
        pad.layout_id(),
        pad.buttons(),
        calibration,
        logical,
        inventory,
        path,
    )
}
