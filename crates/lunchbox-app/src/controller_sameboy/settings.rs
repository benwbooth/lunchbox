//! Saved native SDL setup and visual review; no device or filesystem I/O.
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
    pub runtime: Runtime,
    pub players: Vec<Player>,
}

/// Explicit build information associated with executable_sha256. These are
/// user declarations, not findings from a runtime probe or a file-size guess.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Runtime {
    pub abi: super::configuration::Abi,
    pub data_directory: DataDirectory,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "path",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub(crate) enum DataDirectory {
    NotCompiled,
    Compiled(PathBuf),
}

impl DataDirectory {
    pub(crate) fn path(&self) -> Option<&std::path::Path> {
        match self {
            Self::NotCompiled => None,
            Self::Compiled(path) => Some(path),
        }
    }
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(path) = self.runtime.data_directory.path() {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "SameBoy compiled data directory must be absolute without parent traversal"
            );
        }
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "SameBoy needs an emulator identity"
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
                "SameBoy setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "SameBoy needs a trusted SDL executable SHA-256"
        );
        ensure!(
            self.players.len() == 1,
            "SameBoy SDL frontend needs exactly one player"
        );
        let mut ports = BTreeSet::new();
        let mut controllers = BTreeSet::new();
        for player in &self.players {
            ensure!(
                player.player == 1
                    && ports.insert(player.player)
                    && !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "SameBoy players and physical controllers must be distinct"
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
            .find(|profile| profile.id == "sameboy:standalone-sdl-gameboy")
            .context("Missing native SameBoy SDL profile")?;
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("SameBoy controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "SameBoy native mapping requires Linux calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "SameBoy needs native calibration for every standard Game Boy control"
            );
            players.push(serde_json::json!({"pad":player.player,"controller_id":player.controller_id,
                "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}));
        }
        Ok(serde_json::json!({"players":players,"launch_ready":false,
            "launch_integration":"partial",
            "detail":"SameBoy SDL native dispatch is connected but untested. SDL device zero must match the selected controller. Runtime ABI and data paths are user-declared. Tilt games can consume directional axes as accelerometer input. Review opens no devices and does not prove readiness."}))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many SameBoy saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate SameBoy emulator/content setup"
        );
    }
    Ok(())
}
