//! NesHawk contract at BizHawk 8c6b8958bbbe623eaaa36bc82af858b812893628.
//! Host bindings must be translated against session-owned SDL device identity.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const CORE: &str = "BizHawk.Emulation.Cores.Nintendo.NES.NES";
const DECK: &str = "NES Controller";
pub(crate) const BUTTONS: [&str; 8] = ["A", "B", "Select", "Start", "Up", "Down", "Left", "Right"];
const POWER_BUTTONS: [&str; 12] = [
    "PP1", "PP2", "PP3", "PP4", "PP5", "PP6", "PP7", "PP8", "PP9", "PP10", "PP11", "PP12",
];

pub(crate) use super::digital_session::PlayerRequest;

/// Check prepared configuration, not loaded-runtime provenance.
pub(crate) fn validate_capture_config(source: &str, ports: [PadPort; 2]) -> Result<()> {
    let sync = super::definition_capture::selected_sync_config(source, "NES", "NesHawk", CORE)?;
    let controls = &sync["Controls"];
    ensure!(
        controls["Famicom"].as_bool() == Some(false)
            && controls["FamicomExpPort"].as_str() == Some("UnpluggedFam")
            && controls["NesLeftPort"].as_str() == Some(ports[0].native_name())
            && controls["NesRightPort"].as_str() == Some(ports[1].native_name()),
        "NesHawk capture config ports/expansion differ from the request"
    );
    Ok(())
}

/// Content expectations supplied independently of the captured response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ConsoleControls {
    pub fds_sides: Option<u16>,
    pub vs_system: bool,
}

pub(crate) fn validate_definition(
    definition: &super::definition_capture::Definition,
    ports: [PadPort; 2],
    console: ConsoleControls,
    expected_rom_hash: &str,
) -> Result<()> {
    let count = ports.iter().map(|port| port.players()).sum::<u8>();
    ensure!(count > 0, "No NesHawk controllers requested");
    ensure!(
        !expected_rom_hash.is_empty()
            && expected_rom_hash.len() <= 256
            && !expected_rom_hash.chars().any(char::is_control),
        "Expected NesHawk content hash is missing or invalid"
    );
    ensure!(
        definition.system == "NES" && definition.rom_hash == expected_rom_hash,
        "Captured NesHawk system/content hash differs from the request"
    );
    ensure!(
        definition.axes.is_empty(),
        "NesHawk digital capture contains unexpected axes"
    );
    let mut expected = std::collections::BTreeSet::from(["Power".to_owned(), "Reset".to_owned()]);
    for player in 1..=count {
        expected.extend(
            player_port(ports, player)?
                .buttons()
                .iter()
                .map(|button| format!("P{player} {button}")),
        );
    }
    if let Some(sides) = console.fds_sides {
        ensure!(
            (1..=256).contains(&sides),
            "FDS side count exceeds the capture contract"
        );
        expected.insert("FDS Eject".to_owned());
        expected.extend((0..sides).map(|side| format!("FDS Insert {side}")));
    }
    if console.vs_system {
        expected.extend(["Insert Coin P1", "Insert Coin P2", "Service Switch"].map(str::to_owned));
    }
    let actual: std::collections::BTreeSet<_> = definition.buttons.iter().cloned().collect();
    ensure!(
        actual.len() == definition.buttons.len(),
        "Captured NesHawk definition repeats controls"
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
        "Loaded NesHawk controls differ from the request (first 12 each): missing [{}]; unexpected [{}]",
        missing.join(", "),
        extra.join(", ")
    );
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn capture_definition(
    process: super::definition_capture::CaptureProcess,
    ports: [PadPort; 2],
    console: ConsoleControls,
    expected_rom_hash: &str,
    timeout: std::time::Duration,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<super::definition_capture::Definition> {
    let definition = process.wait_for_definition(timeout, cancel)?;
    validate_definition(&definition, ports, console, expected_rom_hash)?;
    ensure!(
        !cancel.load(std::sync::atomic::Ordering::Relaxed),
        "NesHawk definition comparison cancelled"
    );
    Ok(definition)
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
        super::digital_session::DigitalDeck::NesHawk(ports),
        requests,
        cancel,
    )
}

pub(crate) struct PreparedConfig {
    pub configuration: String,
    pub warnings: Vec<String>,
}

/// Translate all selected players from a caller-supplied installation snapshot.
/// This function neither probes devices nor writes files.
pub(crate) fn prepare_config(
    source: &str,
    ports: [PadPort; 2],
    requests: &[PlayerRequest<'_>],
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<PreparedConfig> {
    let count = ports.iter().map(|port| port.players()).sum::<u8>();
    ensure!(
        count > 0 && requests.len() == usize::from(count),
        "NesHawk needs one controller for every configured joypad slot"
    );
    let mut slots = std::collections::BTreeSet::new();
    let mut devices = std::collections::BTreeSet::new();
    for request in requests {
        ensure!(
            (1..=count).contains(&request.player) && slots.insert(request.player),
            "Invalid or duplicate NesHawk logical player {}",
            request.player
        );
        let device = inventory.device_at_path(request.runtime_path)?;
        ensure!(
            devices.insert(device.device_index),
            "One SDL device cannot supply multiple NesHawk players"
        );
    }
    let mut players = BTreeMap::new();
    let mut warnings = Vec::new();
    for request in requests {
        let (buttons, notes) = translate_pad(
            player_port(ports, request.player)?,
            request.calibration,
            request.logical,
            inventory,
            request.runtime_path,
        )
        .with_context(|| format!("Preparing NesHawk player {}", request.player))?;
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

/// The returned owner retains the private configuration and guards its source.
/// Argument parsing and temporary-file lifetime use the existing native path.
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PadPort {
    None,
    Joypad,
    FourScore,
    SnesJoypad,
    PowerPad,
}

impl PadPort {
    pub(crate) fn players(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Joypad | Self::SnesJoypad | Self::PowerPad => 1,
            Self::FourScore => 2,
        }
    }
    fn native_name(self) -> &'static str {
        match self {
            Self::None => "UnpluggedNES",
            Self::Joypad => "ControllerNES",
            Self::FourScore => "FourScore",
            Self::SnesJoypad => "ControllerSNES",
            Self::PowerPad => "PowerPad",
        }
    }
    pub(crate) fn layout_id(self) -> &'static str {
        if self == Self::SnesJoypad {
            "snes"
        } else if self == Self::PowerPad {
            "nes-power-pad"
        } else {
            "nes"
        }
    }
    fn buttons(self) -> &'static [&'static str] {
        if self == Self::SnesJoypad {
            &super::snes9x::BUTTONS
        } else if self == Self::PowerPad {
            &POWER_BUTTONS
        } else {
            &BUTTONS
        }
    }
}

pub(crate) fn player_port(ports: [PadPort; 2], player: u8) -> Result<PadPort> {
    ensure!(player > 0, "Native NES player numbers start at one");
    let mut end = 0;
    for port in ports {
        end += port.players();
        if player <= end {
            return Ok(port);
        }
    }
    anyhow::bail!("Native NES player {player} is outside the configured deck")
}

pub(crate) fn validate_saved_pad(
    port: PadPort,
    calibration: &crate::controller_catalog::Calibration,
) -> Result<()> {
    super::digital_pad::validate_saved_pad(port.layout_id(), port.buttons(), calibration)?;
    if port == PadPort::PowerPad {
        validate_power_inputs(calibration, None)?;
    }
    Ok(())
}

pub(crate) fn validate_power_inputs(
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
) -> Result<()> {
    let resolution = super::digital_pad::resolve_pad("nes-power-pad", calibration, logical)?;
    let mut identities = std::collections::BTreeSet::new();
    let mut outputs = std::collections::BTreeSet::new();
    for name in POWER_BUTTONS {
        let assigned = resolution
            .assignments
            .get(&name.to_ascii_lowercase())
            .with_context(|| format!("Power Pad {name} has no calibrated input"))?;
        let native = calibration
            .bindings
            .get(assigned)
            .and_then(|input| input.native.as_ref())
            .context("Power Pad source has no native identity")?;
        ensure!(
            identities.insert(native.code),
            "Power Pad requires independent inputs; {name} shares a button or axis with another pad"
        );
        if let Some(logical) = logical {
            let gesture = logical
                .bindings
                .get(assigned)
                .context("Power Pad SDL gesture is absent")?;
            ensure!(
                outputs.insert(&gesture.output),
                "Power Pad requires independent SDL outputs; {name} shares output {} with another pad",
                gesture.output
            );
        }
    }
    Ok(())
}

pub(crate) fn translate_pad(
    port: PadPort,
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    if port == PadPort::PowerPad {
        let selected = if inventory.device_at_path(path)?.is_game_controller {
            logical
        } else {
            None
        };
        validate_power_inputs(calibration, selected)?;
    }
    super::digital_pad::translate_pad(
        port.layout_id(),
        port.buttons(),
        calibration,
        logical,
        inventory,
        path,
    )
}

fn system_button(name: &str) -> bool {
    matches!(
        name,
        "Power" | "Reset" | "FDS Eject" | "Insert Coin P1" | "Insert Coin P2" | "Service Switch"
    ) || name.strip_prefix("FDS Insert ").is_some_and(|index| {
        index
            .parse::<u32>()
            .is_ok_and(|value| value.to_string() == index)
    })
}

/// Logical players follow merged left-deck then right-deck order. Four Score
/// numbering is the core's ordering, not an assumed physical socket numbering.
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
        "Select valid NesHawk logical slots; unassigned slots remain unbound"
    );
    // Replacing the whole deck removes prior bindings on unassigned slots.
    let mut buttons = Map::new();
    for (player, bindings) in players {
        let required = player_port(ports, *player)?.buttons();
        ensure!(
            bindings.len() == required.len()
                && required.iter().all(|name| bindings.contains_key(*name)),
            "NesHawk player {player} needs exactly the selected controller's native buttons"
        );
        for (name, input) in bindings {
            ensure!(
                !input.trim().is_empty()
                    && input.len() <= 1024
                    && !input.chars().any(char::is_control),
                "Invalid native input for NesHawk player {player} {name}"
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
    if let Some(previous) = decks.get(DECK) {
        let previous = previous
            .as_object()
            .context("NES controller bindings must be an object")?;
        for (name, input) in previous.iter().filter(|(name, _)| system_button(name)) {
            ensure!(
                input.is_string(),
                "NES system binding {name} must be a string"
            );
            buttons.insert(name.clone(), input.clone());
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
        .insert("NES".into(), Value::String("NesHawk".into()));
    root.insert("DontTryOtherCores".into(), Value::Bool(true));
    let sync = super::object_child(super::object_child(root, "CoreSyncSettings")?, CORE)?;
    let controls = super::object_child(sync, "Controls")?;
    controls.insert("Famicom".into(), Value::Bool(false));
    controls.insert(
        "FamicomExpPort".into(),
        Value::String("UnpluggedFam".into()),
    );
    controls.insert(
        "NesLeftPort".into(),
        Value::String(ports[0].native_name().into()),
    );
    controls.insert(
        "NesRightPort".into(),
        Value::String(ports[1].native_name().into()),
    );
    let mut encoded = serde_json::to_string_pretty(&config)?;
    encoded.push('\n');
    Ok(encoded)
}
