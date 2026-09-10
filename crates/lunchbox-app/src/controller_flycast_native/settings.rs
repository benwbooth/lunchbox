//! Saved standalone Flycast arcade setup, distinct from libretro settings.
use super::arcade::Panel;
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Player {
    pub player: u8,
    pub controller_id: String,
    #[serde(default)]
    pub panel: Panel,
    pub source_controls: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: PathBuf,
    /// Flycast's native game ID, not the library title or filename stem.
    pub game_id: String,
    pub executable_sha256: String,
    pub source_config: PathBuf,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    pub players: Vec<Player>,
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty()
                && !self.game_id.trim().is_empty()
                && !self.game_id.chars().any(char::is_control),
            "Flycast needs emulator identity and native game ID"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Flycast needs a trusted executable SHA-256"
        );
        for path in [
            &self.content,
            &self.source_config,
            &self.probe_program,
            &self.sdl_library,
        ] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                "Flycast paths must be absolute without parent traversal"
            );
        }
        ensure!(
            !self.players.is_empty() && self.players.len() <= 4,
            "Flycast requires one to four panel players"
        );
        let mut ports = BTreeSet::new();
        let mut devices = BTreeSet::new();
        for player in &self.players {
            ensure!(
                (1..=4).contains(&player.player)
                    && ports.insert(player.player)
                    && !player.controller_id.trim().is_empty()
                    && devices.insert(&player.controller_id),
                "Flycast players require distinct ports and controllers"
            );
            let routes = player.panel.routes();
            let mut sources = BTreeSet::new();
            ensure!(
                player.source_controls.len() == routes.len()
                    && routes
                        .keys()
                        .all(|key| player.source_controls.contains_key(key))
                    && player
                        .source_controls
                        .values()
                        .all(|source| !source.is_empty() && sources.insert(source)),
                "Flycast panel requires distinct source controls for every selected action"
            );
        }
        Ok(())
    }

    pub(crate) fn review(
        &self,
        calibrations: &HashMap<String, Calibration>,
    ) -> Result<serde_json::Value> {
        self.validate()?;
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Flycast controller has no saved calibration")?;
            calibration.validate()?;
            ensure!(
                calibration.os == "linux",
                "Flycast native SDL review requires Linux calibration"
            );
            let layout = catalog()
                .layout(&calibration.layout)
                .context("Flycast source layout is missing")?;
            let routes = player.panel.routes();
            let mut rows = Vec::new();
            for (target, source) in &player.source_controls {
                let physical = layout
                    .controls
                    .iter()
                    .find(|control| &control.id == source)
                    .context("Flycast source control is missing from its layout")?;
                let input = calibration
                    .bindings
                    .get(source)
                    .context("Flycast physical source needs calibration")?;
                ensure!(
                    input.native.is_some(),
                    "Flycast source requires native calibration"
                );
                rows.push(serde_json::json!({"target_id":if target == "coin" {"select"} else {target.as_str()}, "target":target,
                    "physical_id":source,"physical":physical.label,"input":input,"output":routes[target],
                    "reason":"Saved physical source; native SDL translation is checked during preparation"}));
            }
            players.push(serde_json::json!({"pad":player.player,"controller_id":player.controller_id,
                "source_layout":calibration.layout,"target_layout":player.panel.layout(),"mapping":{"rows":rows}}));
        }
        Ok(
            serde_json::json!({"players":players,"launch_ready":false,"launch_integration":"partial",
            "detail":"Native Linux standard arcade panel dispatch is connected with private config and startup joystick checks. Game ID, internal routing and runtime compatibility remain unverified. Review opens no devices."}),
        )
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many Flycast saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate Flycast emulator/content setup"
        );
    }
    Ok(())
}
