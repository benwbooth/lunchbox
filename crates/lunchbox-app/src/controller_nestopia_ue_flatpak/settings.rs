//! Saved choices for the exact Nestopia UE Flatpak controller contract.

use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Component, PathBuf},
};

pub(crate) const MAPPING_PROFILE: &str = "nestopia-ue:flatpak-nes";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Player {
    pub(crate) player: u8,
    pub(crate) controller_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub(crate) emulator_id: String,
    pub(crate) content: PathBuf,
    pub(crate) source_main_config: PathBuf,
    pub(crate) source_input_config: PathBuf,
    pub(crate) probe_program: PathBuf,
    pub(crate) sdl_library: PathBuf,
    pub(crate) executable_sha256: String,
    pub(crate) players: [Player; 2],
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "Nestopia needs an emulator identity"
        );
        for path in [
            &self.content,
            &self.source_main_config,
            &self.source_input_config,
            &self.probe_program,
            &self.sdl_library,
        ] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "Nestopia setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.source_main_config
                .file_name()
                .is_some_and(|name| name == "nestopia.conf")
                && self
                    .source_input_config
                    .file_name()
                    .is_some_and(|name| name == "input.conf")
                && self.source_main_config.parent() == self.source_input_config.parent(),
            "Nestopia needs sibling nestopia.conf and input.conf files"
        );
        ensure!(
            self.content
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "nes" | "unf" | "unif"
                )),
            "Nestopia standard-pad launch needs NES cartridge content"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Nestopia needs a trusted executable SHA-256"
        );
        ensure!(
            self.players[0].player == 1
                && self.players[1].player == 2
                && !self.players[0].controller_id.trim().is_empty()
                && !self.players[1].controller_id.trim().is_empty()
                && self.players[0].controller_id != self.players[1].controller_id,
            "Nestopia needs distinct physical controllers for players one and two"
        );
        Ok(())
    }

    pub(crate) fn review(
        &self,
        calibrations: &HashMap<String, Calibration>,
    ) -> Result<serde_json::Value> {
        self.validate()?;
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == MAPPING_PROFILE)
            .context("Missing NES two-pad mapping profile")?;
        ensure!(
            profile.target_layout == "nes"
                && profile
                    .bindings
                    .keys()
                    .map(String::as_str)
                    .eq(["a", "b", "down", "left", "right", "select", "start", "up",]),
            "NES two-pad mapping profile changed"
        );
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Nestopia controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Nestopia Flatpak needs Linux calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.len() == 8
                    && mapping.rows.iter().all(|row| {
                        row.physical_id.is_some()
                            && row
                                .input
                                .as_ref()
                                .is_some_and(|input| input.native.is_some())
                    }),
                "Nestopia needs native calibration for all eight NES controls"
            );
            players.push(serde_json::json!({
                "player": player.player,
                "controller_id": player.controller_id,
                "source_layout": calibration.layout,
                "target_layout": profile.target_layout,
                "mapping": mapping,
            }));
        }
        Ok(serde_json::json!({
            "players": players,
            "launch_ready": false,
            "detail": "Saved review only. The exact Flatpak deployment, SDL routing, private configuration, and opened controller nodes are checked again at launch."
        }))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many Nestopia saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate Nestopia emulator/content setup"
        );
    }
    Ok(())
}
