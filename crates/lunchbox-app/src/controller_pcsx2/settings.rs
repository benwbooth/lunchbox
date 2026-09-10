//! Persisted PCSX2 standard-pad choices; review performs no native I/O.
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Component, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Player {
    pub player: u8,
    pub controller_id: String,
    /// Destination visual ID -> calibrated physical-layout control ID.
    pub source_controls: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: PathBuf,
    pub executable_sha256: String,
    pub source_config: PathBuf,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    #[serde(default)]
    pub runtime_libraries: Vec<PathBuf>,
    #[serde(default)]
    pub native: Option<NativeContent>,
    #[serde(default)]
    pub multitaps: [bool; 2],
    pub players: Vec<Player>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeContent {
    pub data_root: PathBuf,
    /// Native disc serial, not a library title or filename stem.
    pub serial: String,
    /// PCSX2 disc CRC (integer), not a hash of the entire ISO file.
    pub crc: u32,
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "PCSX2 emulator identity is missing"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "PCSX2 needs a trusted executable SHA-256"
        );
        for path in [
            &self.content,
            &self.source_config,
            &self.probe_program,
            &self.sdl_library,
        ] {
            ensure!(
                path.is_absolute() && !path.components().any(|part| part == Component::ParentDir),
                "PCSX2 paths must be absolute without parent traversal"
            );
        }
        let slots = super::profile::native_slots(self.multitaps);
        if let Some(native) = &self.native {
            ensure!(
                native.data_root.is_absolute()
                    && !native
                        .data_root
                        .components()
                        .any(|part| part == Component::ParentDir)
                    && native.crc != 0
                    && native.serial.len() <= 128
                    && native
                        .serial
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')),
                "PCSX2 native content needs an absolute data root, nonzero disc CRC and supported serial"
            );
        }
        ensure!(
            self.runtime_libraries.len() <= 32
                && self.runtime_libraries.iter().all(|path| path.is_absolute()
                    && !path.components().any(|part| part == Component::ParentDir)),
            "PCSX2 runtime dependencies must be absolute paths"
        );
        ensure!(
            !self.players.is_empty() && self.players.len() <= slots.len(),
            "PCSX2 player count exceeds multitap capacity"
        );
        let mut ports = BTreeSet::new();
        let mut devices = BTreeSet::new();
        let routes = super::visual_routes();
        for player in &self.players {
            ensure!(
                player.player > 0
                    && usize::from(player.player) <= slots.len()
                    && ports.insert(player.player)
                    && !player.controller_id.is_empty()
                    && devices.insert(&player.controller_id),
                "PCSX2 requires distinct valid players and controllers"
            );
            let mut sources = BTreeSet::new();
            ensure!(
                player.source_controls.len() == routes.len()
                    && routes
                        .keys()
                        .all(|key| player.source_controls.contains_key(*key))
                    && player
                        .source_controls
                        .values()
                        .all(|value| !value.is_empty() && sources.insert(value)),
                "PCSX2 requires a distinct physical source for each standard pad control"
            );
        }
        Ok(())
    }

    pub(crate) fn review(
        &self,
        calibrations: &HashMap<String, Calibration>,
    ) -> Result<serde_json::Value> {
        self.validate()?;
        let routes = super::visual_routes();
        let slots = super::profile::native_slots(self.multitaps);
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("PCSX2 controller needs calibration")?;
            calibration.validate()?;
            ensure!(
                calibration.os == "linux",
                "PCSX2 native preparation currently requires Linux calibration"
            );
            let layout = catalog()
                .layout(&calibration.layout)
                .context("PCSX2 physical layout is missing")?;
            let mut rows = Vec::new();
            for (target, source) in &player.source_controls {
                let control = layout
                    .controls
                    .iter()
                    .find(|control| &control.id == source)
                    .context("PCSX2 source control is missing")?;
                let input = calibration
                    .bindings
                    .get(source)
                    .context("PCSX2 source control is not calibrated")?;
                ensure!(
                    input.native.is_some(),
                    "PCSX2 source needs native physical calibration"
                );
                rows.push(serde_json::json!({"target_id":target,"target":routes[target.as_str()],"physical_id":source,
                    "physical":control.label,"input":input,"output":routes[target.as_str()],
                    "reason":"Saved source; native SDL3 translation required at launch"}));
            }
            players.push(serde_json::json!({"pad":player.player,"controller_id":player.controller_id,
                "native_section":super::section(slots[usize::from(player.player - 1)])?,
                "source_layout":calibration.layout,"target_layout":"dualshock","mapping":{"rows":rows}}));
        }
        Ok(
            serde_json::json!({"players":players,"launch_ready":false,"launch_integration":"partial",
            "detail":"PCSX2 native Linux DualShock2 dispatch is connected. Native content identity and SDL dependencies are required. Runtime/internal routing remain unverified; review opens no devices."}),
        )
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many PCSX2 setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate PCSX2 emulator/content setup"
        );
    }
    Ok(())
}
