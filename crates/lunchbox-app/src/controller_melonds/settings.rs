//! Persisted melonDS standard-pad choices; review performs no native I/O.
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
    pub bubblewrap_program: PathBuf,
    pub sdl_library: PathBuf,
    #[serde(default)]
    pub runtime_libraries: Vec<PathBuf>,
    pub players: Vec<Player>,
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "melonDS emulator identity is missing"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "melonDS needs a trusted executable SHA-256"
        );
        for path in [
            &self.content,
            &self.source_config,
            &self.probe_program,
            &self.bubblewrap_program,
            &self.sdl_library,
        ] {
            ensure!(
                path.is_absolute() && !path.components().any(|part| part == Component::ParentDir),
                "melonDS paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.runtime_libraries.len() <= 32
                && self.runtime_libraries.iter().all(|path| path.is_absolute()
                    && !path.components().any(|part| part == Component::ParentDir)),
            "melonDS runtime dependencies must be absolute paths"
        );
        ensure!(
            !self.players.is_empty() && self.players.len() == 1,
            "melonDS currently maps the primary instance controller"
        );
        let mut ports = BTreeSet::new();
        let mut devices = BTreeSet::new();
        let routes = super::visual_routes();
        for player in &self.players {
            ensure!(
                player.player > 0
                    && player.player == 1
                    && ports.insert(player.player)
                    && !player.controller_id.is_empty()
                    && devices.insert(&player.controller_id),
                "melonDS requires distinct valid players and controllers"
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
                "melonDS requires a distinct physical source for each standard pad control"
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
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("melonDS controller needs calibration")?;
            calibration.validate()?;
            ensure!(
                calibration.os == "linux",
                "melonDS native preparation currently requires Linux calibration"
            );
            let layout = catalog()
                .layout(&calibration.layout)
                .context("melonDS physical layout is missing")?;
            let mut rows = Vec::new();
            for (target, source) in &player.source_controls {
                let control = layout
                    .controls
                    .iter()
                    .find(|control| &control.id == source)
                    .context("melonDS source control is missing")?;
                let input = calibration
                    .bindings
                    .get(source)
                    .context("melonDS source control is not calibrated")?;
                ensure!(
                    input.native.is_some(),
                    "melonDS source needs native physical calibration"
                );
                rows.push(serde_json::json!({"target_id":target,"target":routes[target.as_str()],"physical_id":source,
                    "physical":control.label,"input":input,"output":routes[target.as_str()],
                    "reason":"Saved source; native SDL2 translation required at launch"}));
            }
            players.push(serde_json::json!({"pad":player.player,"controller_id":player.controller_id,
                "native_section":"Instance0",
                "source_layout":calibration.layout,"target_layout":"nds-native-buttons","mapping":{"rows":rows}}));
        }
        Ok(
            serde_json::json!({"players":players,"launch_ready":false,"launch_integration":"partial",
            "detail":"melonDS standard pad setup is saved for native SDL translation. Native Linux launch dispatch is connected but untested; this review covers standard buttons only and opens no devices."}),
        )
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many melonDS setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate melonDS emulator/content setup"
        );
    }
    Ok(())
}
