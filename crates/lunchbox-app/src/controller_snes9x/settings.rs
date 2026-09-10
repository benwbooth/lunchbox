//! Saved native GTK setup and visual review; no device or filesystem I/O.
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
    pub source_config: PathBuf,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    pub bubblewrap_program: PathBuf,
    pub executable_sha256: String,
    pub players: Vec<Player>,
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "Snes9x needs an emulator identity"
        );
        for path in [
            &self.content,
            &self.source_config,
            &self.probe_program,
            &self.sdl_library,
            &self.bubblewrap_program,
        ] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "Snes9x setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.source_config
                .file_name()
                .is_some_and(|name| name == "snes9x.conf"),
            "Snes9x GTK needs its native snes9x.conf path"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Snes9x needs a trusted GTK executable SHA-256"
        );
        ensure!(
            !self.players.is_empty() && self.players.len() <= 5,
            "Snes9x needs one to five players"
        );
        let mut ports = BTreeSet::new();
        let mut controllers = BTreeSet::new();
        for player in &self.players {
            ensure!(
                (1..=5).contains(&player.player)
                    && ports.insert(player.player)
                    && !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "Snes9x players and physical controllers must be distinct"
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
            .find(|profile| profile.id == "snes9x:standalone-gtk-snes")
            .context("Missing native Snes9x GTK profile")?;
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Snes9x controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Snes9x native mapping requires Linux calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Snes9x needs native calibration for every standard SNES control"
            );
            players.push(serde_json::json!({"pad":player.player,"controller_id":player.controller_id,
                "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}));
        }
        Ok(serde_json::json!({"players":players,"launch_ready":false,
            "detail":"Saved mapping review only. Native Snes9x GTK launch checks runtime and controller routing when starting a game; compatibility remains untested."}))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many Snes9x saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate Snes9x emulator/content setup"
        );
    }
    Ok(())
}
