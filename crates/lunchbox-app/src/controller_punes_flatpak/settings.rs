//! Saved choices for the exact puNES 0.111 Flatpak controller contract.

use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Component, PathBuf},
};

pub(crate) const MAPPING_PROFILE: &str = "punes:flatpak-nes-standard";

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
    pub(crate) executable_sha256: String,
    pub(crate) players: Vec<Player>,
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "puNES needs an emulator identity"
        );
        for path in [
            &self.content,
            &self.source_main_config,
            &self.source_input_config,
            &self.probe_program,
        ] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|component| matches!(component, Component::ParentDir)),
                "puNES setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.source_main_config
                .file_name()
                .is_some_and(|name| name == "puNES.cfg")
                && self
                    .source_input_config
                    .file_name()
                    .is_some_and(|name| name == "input.cfg")
                && self.source_main_config.parent() == self.source_input_config.parent(),
            "puNES needs sibling puNES.cfg and input.cfg files"
        );
        ensure!(
            self.content
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "nes" | "unf" | "unif"
                )),
            "puNES standard-pad mode accepts only NES cartridge content; FDS, NSF and special peripherals need separate setup"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "puNES needs a trusted executable SHA-256"
        );
        ensure!(
            matches!(self.players.len(), 1 | 2),
            "puNES needs one or two players"
        );
        let mut controllers = BTreeSet::new();
        for (index, player) in self.players.iter().enumerate() {
            ensure!(
                usize::from(player.player) == index + 1
                    && !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "puNES players must be contiguous and use distinct controllers"
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
            .find(|profile| profile.id == MAPPING_PROFILE)
            .context("Missing puNES standard-pad mapping profile")?;
        ensure!(
            profile.target_layout == "nes"
                && profile
                    .bindings
                    .keys()
                    .map(String::as_str)
                    .eq(["a", "b", "down", "left", "right", "select", "start", "up"]),
            "puNES standard-pad mapping profile changed"
        );
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("puNES controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "puNES Flatpak needs Linux calibration"
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
                "puNES needs native calibration for all eight NES controls"
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
            "detail": "Saved review only. The exact Flatpak deployment, calibrated bridges, private config, target evdev identities and open puNES descriptors are checked again at launch."
        }))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many puNES saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate puNES emulator/content setup"
        );
    }
    Ok(())
}
