//! Saved native BlastEm launch setup and visual review; no device I/O.
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Component, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Player {
    pub player: u8,
    pub controller_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: PathBuf,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    pub executable_sha256: String,
    pub players: Vec<Player>,
}

pub(crate) const PROFILE_ID: &str = "blastem:standalone-genesis-6";

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "BlastEm needs an emulator identity"
        );
        for path in [&self.content, &self.probe_program, &self.sdl_library] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "BlastEm setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "BlastEm needs a trusted executable SHA-256"
        );
        ensure!(
            (1..=2).contains(&self.players.len()),
            "BlastEm supports one or two gamepad ports in this contract"
        );
        let mut ports = BTreeSet::new();
        let mut controllers = BTreeSet::new();
        for player in &self.players {
            ensure!(
                (1..=2).contains(&player.player)
                    && ports.insert(player.player)
                    && !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "BlastEm players and physical controllers must be distinct"
            );
        }
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
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing native BlastEm profile")?;
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("BlastEm controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "BlastEm native mapping requires Linux calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "BlastEm needs native calibration for every six-button control"
            );
            players.push(serde_json::json!({"port":player.player,"controller_id":player.controller_id,
                "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}));
        }
        Ok(serde_json::json!({"players":players,"launch_ready":false,
            "launch_integration":"partial",
            "detail":"BlastEm tern-config dispatch is connected but untested. Launch isolates HOME so the user's own blastem.cfg is never touched and binds the selected SDL devices to gamepad ports one and two; the six-button Genesis pad is covered. Mice, the tee-input adapter, hotkeys and analog stick translation are not. Runtime testing remains deferred."}))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many BlastEm saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate BlastEm emulator/content setup"
        );
    }
    Ok(())
}
