//! Saved native VICE launch setup and visual review; no device I/O.
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

pub(crate) const PROFILE_ID: &str = "vice:standalone-vice-joystick-panel";

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "VICE needs an emulator identity"
        );
        for path in [&self.content, &self.probe_program, &self.sdl_library] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "VICE setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "VICE needs a trusted executable SHA-256"
        );
        ensure!(
            (1..=2).contains(&self.players.len()),
            "VICE supports one or two joystick ports in this contract"
        );
        let mut ports = BTreeSet::new();
        let mut controllers = BTreeSet::new();
        for player in &self.players {
            ensure!(
                (1..=2).contains(&player.player)
                    && ports.insert(player.player)
                    && !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "VICE players and physical controllers must be distinct"
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
            .context("Missing native VICE profile")?;
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("VICE controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "VICE native mapping requires Linux calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "VICE needs native calibration for every required joystick control"
            );
            players.push(serde_json::json!({"port":player.player,"controller_id":player.controller_id,
                "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}));
        }
        Ok(serde_json::json!({"players":players,"launch_ready":false,
            "launch_integration":"partial",
            "detail":"VICE .vjm native dispatch is connected but untested. Launch passes a private -config/-joymap pair and the SDL2 numbering is probed with the trusted runtime library; the user's own vicerc is never touched. Digital joystick pins plus fire2/fire3 are covered; keysets, paddles, potentiometer axes, mouse, light pens and extra userport adapters are not."}))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many VICE saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate VICE emulator/content setup"
        );
    }
    Ok(())
}
