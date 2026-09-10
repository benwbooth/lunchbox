//! Compose saved physical calibration with DuckStation's native SDL/INI writers.
//! The caller must resolve the actual runtime, SDL player identity and game serial
//! before preparation. This module does not discover or launch an emulator.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    bindings::ResolvedGamepad,
    duckstation::{AxisSuppression, physical_analog_binding, physical_digital_binding},
    duckstation_config::{GameIdentity, LaunchConfig},
    linux_classic::{AxisEndpoints, ClassicMap},
};

use crate::controller_catalog::{Calibration, catalog};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;

/// Owns the configuration and projection across preparation and actual startup.
/// A prepared projection is not permission to report the controller connected:
/// the spawning layer must call `confirm_startup` with the child's real log.
pub(crate) struct PreparedSession {
    configuration: LaunchConfig,
    projection: lunchbox_controller_probe::players::PlayerProbe,
    runtime_inputs: BTreeMap<std::path::PathBuf, String>,
    startup_confirmed: bool,
}

impl PreparedSession {
    pub(crate) fn config_home(&self) -> std::path::PathBuf {
        self.configuration.config_home()
    }

    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        self.configuration.verify_originals_unchanged()?;
        for (path, expected) in &self.runtime_inputs {
            ensure!(
                file_hash(path)? == *expected,
                "DuckStation runtime input changed: {}",
                path.display()
            );
        }
        Ok(())
    }

    pub(crate) fn confirm_startup(&mut self, log: &str) -> Result<()> {
        self.startup_confirmed = false;
        self.configuration.verify_startup_routing(log)?;
        self.projection.verify_startup_log(log)?;
        self.verify_before_launch()?;
        self.startup_confirmed = true;
        Ok(())
    }

    pub(crate) fn startup_confirmed(&self) -> bool {
        self.startup_confirmed
    }
}

fn file_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Native Linux composition from one fresh target-runtime observation. No
/// process-local SDL instance ID or GUID is used as a saved physical identity.
/// Flatpak paths require a namespace-aware loader and must not enter this path.
pub(crate) fn prepare_resolved(
    setup: &SavedSetup,
    calibrations: &std::collections::HashMap<String, Calibration>,
    inventory: &[crate::controllers::ControllerDevice],
    snapshot: &lunchbox_controller_probe::Snapshot,
) -> Result<PreparedSession> {
    setup.review(calibrations)?;
    ensure!(
        snapshot.schema_version == 4
            && snapshot.host_os == "linux"
            && snapshot.sdl_version == 3_002_020,
        "DuckStation requires the supported target-SDL classic snapshot contract"
    );
    ensure!(
        snapshot
            .effective_hints
            .get("SDL_JOYSTICK_LINUX_CLASSIC")
            .and_then(Option::as_deref)
            == Some("1"),
        "DuckStation runtime did not select the measured Linux classic input backend"
    );
    let projection = snapshot
        .player_probe
        .as_ref()
        .context("Missing DuckStation player projection")?;
    ensure!(
        projection.duckstation_revision == lunchbox_controller_probe::players::CONTRACT,
        "DuckStation player projection revision differs"
    );
    ensure!(
        !snapshot.runtime_libraries.is_empty(),
        "DuckStation target runtime dependencies are unresolved"
    );
    let mut runtime_inputs = BTreeMap::new();
    for (path, hash) in std::iter::once((&snapshot.library, &snapshot.library_sha256)).chain(
        snapshot
            .runtime_libraries
            .iter()
            .map(|library| (&library.path, &library.sha256)),
    ) {
        ensure!(
            path.is_absolute()
                && hash.len() == 64
                && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "Invalid DuckStation runtime dependency identity"
        );
        ensure!(
            file_hash(path)? == *hash,
            "DuckStation target runtime dependency changed"
        );
        if let Some(previous) = runtime_inputs.insert(path.clone(), hash.clone()) {
            ensure!(
                previous == *hash,
                "Conflicting DuckStation runtime dependency hashes"
            );
        }
    }
    let mut requests = Vec::new();
    for player in &setup.players {
        let mut matches = inventory
            .iter()
            .filter(|device| device.stable_id == player.controller_id);
        let physical_device = matches
            .next()
            .context("Saved DuckStation controller is disconnected")?;
        ensure!(
            matches.next().is_none(),
            "Ambiguous DuckStation physical controller identity"
        );
        let path = physical_device
            .device_path
            .to_str()
            .context("DuckStation controller path is not UTF-8")?;
        let device = snapshot.device_at_path(path)?;
        let assignment = projection.at_path(path)?;
        ensure!(
            assignment.instance_id == device.instance_id
                && assignment.is_gamepad
                && device.is_gamepad,
            "DuckStation binding and player observations disagree"
        );
        requests.push(PlayerRequest {
            pad: player.pad,
            sdl_player: assignment.projected_player_id,
            calibration: calibrations
                .get(&player.controller_id)
                .context("Missing DuckStation calibration")?,
            gamepad: device
                .resolved
                .as_ref()
                .context("DuckStation SDL bindings were not captured")?,
            physical: device
                .linux_classic
                .as_ref()
                .context("DuckStation physical input translation was not captured")?,
            suppression: AxisSuppression::DuckStation0a53bc47c,
        });
    }
    let mut configuration = prepare_configuration(
        &setup.data_root,
        GameIdentity {
            serial: &setup.serial,
            first_disc_serial: setup.first_disc_serial.as_deref(),
        },
        &requests,
        setup
            .apply_selected_ports
            .then_some(setup.players.as_slice()),
    )?;
    for player in &setup.players {
        ensure!(
            configuration.controller_type(player.pad)? == player.controller_type,
            "DuckStation pad {} type differs from the saved mapping review",
            player.pad
        );
    }
    let database = configuration.data_root().join("gamecontrollerdb.txt");
    match (&snapshot.mapping_database_sha256, database.try_exists()?) {
        (Some(expected), true) => ensure!(
            file_hash(&database)? == *expected,
            "DuckStation staged SDL mapping database differs from the target observation"
        ),
        (None, false) => (),
        _ => anyhow::bail!(
            "DuckStation mapping database presence differs from the target observation"
        ),
    }
    configuration.enable_startup_diagnostics()?;
    let hints = configuration.configure_classic_sdl()?;
    for (key, value) in hints {
        ensure!(
            snapshot
                .effective_hints
                .get(&key)
                .and_then(Option::as_deref)
                == Some(value.as_str()),
            "DuckStation effective SDL hint changed: {key}"
        );
    }
    let session = PreparedSession {
        configuration,
        projection: projection.clone(),
        runtime_inputs,
        startup_confirmed: false,
    };
    session.verify_before_launch()?;
    Ok(session)
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedPlayer {
    pub pad: u8,
    pub controller_id: String,
    pub controller_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    /// Guided selection explicitly configures pad types and active ports in the
    /// private copy. Legacy advanced setups continue preserving their topology.
    #[serde(default)]
    pub apply_selected_ports: bool,
    #[serde(default)]
    pub runtime: Option<NativeRuntime>,
    pub emulator_id: String,
    pub content: std::path::PathBuf,
    pub data_root: std::path::PathBuf,
    pub serial: String,
    /// None means independently confirmed not part of a multi-disc group.
    #[serde(deserialize_with = "explicit_disc_serial")]
    pub first_disc_serial: Option<String>,
    pub players: Vec<SavedPlayer>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeRuntime {
    pub probe_program: std::path::PathBuf,
    pub sdl_library: std::path::PathBuf,
    pub runtime_libraries: Vec<std::path::PathBuf>,
    pub executable_sha256: String,
}

// Require the key to be present: an omitted identity is unknown, not a confirmed
// absence of a disc group. Explicit JSON null is the single-disc declaration.
fn explicit_disc_serial<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error> {
    <Option<String> as serde::Deserialize>::deserialize(deserializer)
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(runtime) = &self.runtime {
            ensure!(
                runtime.probe_program.is_absolute()
                    && runtime.sdl_library.is_absolute()
                    && !runtime.runtime_libraries.is_empty()
                    && runtime
                        .runtime_libraries
                        .iter()
                        .all(|path| path.is_absolute()),
                "DuckStation runtime requires absolute trusted helper, SDL and dependency paths"
            );
            ensure!(
                runtime.executable_sha256.len() == 64
                    && runtime
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "DuckStation runtime requires an executable SHA-256"
            );
        }
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "DuckStation emulator ID is required"
        );
        for path in [&self.content, &self.data_root] {
            ensure!(
                path.is_absolute()
                    && !path.components().any(|part| matches!(
                        part,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )),
                "DuckStation paths must be absolute without parent traversal"
            );
        }
        for serial in std::iter::once(&self.serial).chain(self.first_disc_serial.iter()) {
            ensure!(
                !serial.is_empty()
                    && serial.trim() == serial
                    && serial != "."
                    && serial != ".."
                    && !serial.ends_with('.')
                    && !serial
                        .chars()
                        .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c)),
                "DuckStation needs an exact safe game serial, not a display title"
            );
        }
        ensure!(
            !self.players.is_empty() && self.players.len() <= 8,
            "DuckStation needs one to eight players"
        );
        let mut pads = BTreeSet::new();
        let mut controllers = BTreeSet::new();
        for player in &self.players {
            ensure!(
                (1..=8).contains(&player.pad) && pads.insert(player.pad),
                "Invalid or duplicate DuckStation pad slot"
            );
            ensure!(
                !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "DuckStation requires distinct saved controller identities"
            );
            ensure!(
                matches!(
                    player.controller_type.as_str(),
                    "DigitalController" | "AnalogController"
                ),
                "Unsupported DuckStation controller type"
            );
        }
        Ok(())
    }

    /// Source-only visual review. SDL and launch readiness are deliberately not
    /// inferred from a successful physical-layout plan.
    pub(crate) fn review(
        &self,
        calibrations: &std::collections::HashMap<String, Calibration>,
    ) -> Result<serde_json::Value> {
        self.validate()?;
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("DuckStation player has no saved calibration")?;
            let profile_id = if player.controller_type == "DigitalController" {
                "duckstation:digital-controller"
            } else {
                "duckstation:analog-controller"
            };
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == profile_id)
                .context("Missing DuckStation profile")?;
            let target = catalog()
                .layout(&profile.target_layout)
                .context("Missing DuckStation layout")?;
            let plan = calibration.plan_profile(profile)?;
            for row in &plan.rows {
                if target
                    .controls
                    .iter()
                    .any(|control| control.id == row.target_id && !control.optional)
                {
                    ensure!(
                        row.input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some()),
                        "DuckStation pad {} is missing measured {}",
                        player.pad,
                        row.target
                    );
                }
            }
            players.push(serde_json::json!({"pad":player.pad, "controller_id":player.controller_id,
                "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":plan}));
        }
        Ok(serde_json::json!({"players":players,"launch_ready":false,
            "detail":"Physical mapping review only. Native runtime, SDL identity, game serial and effective configuration require launch-time verification."}))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many saved DuckStation setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate DuckStation emulator/content setup"
        );
    }
    Ok(())
}

pub(crate) struct PlayerRequest<'a> {
    /// DuckStation settings slot, not the SDL controller index.
    pub pad: u8,
    /// Resolved by the target runtime's player projection, never device order.
    pub sdl_player: u32,
    pub calibration: &'a Calibration,
    pub gamepad: &'a ResolvedGamepad,
    /// Target SDL classic backend indices and kernel corrections for this device.
    pub physical: &'a ClassicMap,
    pub suppression: AxisSuppression,
}

/// Pure translation: no files, processes or input devices are opened.
pub(crate) fn bindings(
    request: &PlayerRequest<'_>,
    controller_type: &str,
) -> Result<BTreeMap<String, String>> {
    ensure!(
        request.calibration.os == "linux",
        "DuckStation classic translation requires a Linux calibration"
    );
    let profile_id = if controller_type.eq_ignore_ascii_case("DigitalController") {
        "duckstation:digital-controller"
    } else {
        ensure!(
            controller_type.eq_ignore_ascii_case("AnalogController"),
            "DuckStation controller type {controller_type} requires a different mapping adapter"
        );
        "duckstation:analog-controller"
    };
    let profile = catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == profile_id)
        .context("Missing DuckStation gameplay profile")?;
    let target = catalog()
        .layout(&profile.target_layout)
        .context("Missing DuckStation target layout")?;
    let plan = request.calibration.plan_profile(profile)?;
    request.physical.validate_counts(request.gamepad)?;
    let mut translated = BTreeMap::new();
    let mut tokens = BTreeSet::new();
    let mut axes = BTreeMap::new();
    for row in plan.rows {
        let control = target
            .controls
            .iter()
            .find(|control| control.id == row.target_id)
            .context("Unknown DuckStation target control")?;
        let Some(input) = row.input else {
            ensure!(
                control.optional,
                "Missing calibrated DuckStation control: {}",
                row.target
            );
            continue;
        };
        let native = input
            .native
            .as_ref()
            .context("DuckStation requires measured native input identities")?;
        let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
            released: axis.released,
            pressed: axis.pressed,
        });
        let suffix = if control.analog {
            let measured =
                measured.context("DuckStation analog direction has no measured endpoints")?;
            axes.insert(
                row.output.clone(),
                (native.code, measured.released, measured.pressed),
            );
            physical_analog_binding(
                request.gamepad,
                request.physical,
                native.code,
                measured,
                request.suppression,
            )?
        } else {
            physical_digital_binding(
                request.gamepad,
                request.physical,
                native.code,
                measured,
                request.suppression,
            )?
        };
        ensure!(
            tokens.insert(suffix.clone()),
            "Two DuckStation controls resolve to the same SDL input: {suffix}"
        );
        ensure!(
            translated
                .insert(row.output, format!("SDL-{}/{suffix}", request.sdl_player))
                .is_none(),
            "Duplicate DuckStation setting output"
        );
    }
    // Preserve bipolar sticks. Two unrelated axes must not masquerade as the
    // two halves of one native analog stick axis.
    let mut native_axes = BTreeSet::new();
    for (negative, positive) in [
        ("LLeft", "LRight"),
        ("LUp", "LDown"),
        ("RLeft", "RRight"),
        ("RUp", "RDown"),
    ] {
        match (axes.get(negative), axes.get(positive)) {
            (None, None) => (),
            (Some(&(a, center, low)), Some(&(b, other_center, high))) => {
                ensure!(
                    a == b
                        && center == other_center
                        && ((low < center && high > center) || (low > center && high < center)),
                    "DuckStation {negative}/{positive} requires opposite measured halves of one centered axis"
                );
                ensure!(
                    native_axes.insert(a),
                    "DuckStation stick axes share one physical axis"
                );
            }
            _ => anyhow::bail!("Incomplete DuckStation analog axis {negative}/{positive}"),
        }
    }
    Ok(translated)
}

/// Build a session-owned native configuration; persistent user files are never
/// edited. The returned owner must survive until the launched emulator exits.
/// Game identity must be established even when no per-game settings file exists.
pub(crate) fn prepare_configuration(
    source_root: &Path,
    game: GameIdentity<'_>,
    players: &[PlayerRequest<'_>],
    selected_ports: Option<&[SavedPlayer]>,
) -> Result<LaunchConfig> {
    ensure!(
        !players.is_empty() && players.len() <= 8,
        "DuckStation needs one to eight configured players"
    );
    let mut pads = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for player in players {
        ensure!(
            (1..=8).contains(&player.pad) && pads.insert(player.pad),
            "Invalid or duplicate DuckStation pad slot"
        );
        ensure!(
            identities.insert(player.sdl_player),
            "DuckStation players share one SDL controller identity"
        );
    }
    let mut configuration = LaunchConfig::stage(source_root, Some(game))?;
    if let Some(selected) = selected_ports {
        let ports = selected
            .iter()
            .map(|p| (p.pad, p.controller_type.as_str()))
            .collect::<Vec<_>>();
        ensure!(
            ports.iter().map(|(pad, _)| *pad).collect::<BTreeSet<_>>() == pads,
            "DuckStation guided ports differ from translated players"
        );
        configuration.configure_controller_ports(&ports)?;
    }
    let mut changes = Vec::new();
    for player in players {
        let controller_type = configuration.controller_type(player.pad)?;
        changes.push((
            player.pad,
            controller_type.clone(),
            bindings(player, &controller_type)?,
        ));
    }
    // Translate every requested player before patching any staged controller.
    for (pad, controller_type, bindings) in changes {
        if controller_type.eq_ignore_ascii_case("DigitalController") {
            configuration.apply_digital(pad, &bindings)?;
        } else {
            configuration.apply_analog(pad, &bindings)?;
        }
    }
    configuration.verify_originals_unchanged()?;
    Ok(configuration)
}
