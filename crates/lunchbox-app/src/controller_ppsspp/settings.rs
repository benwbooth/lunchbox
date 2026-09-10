//! Explicit native PPSSPP setup identity; validation does not open hardware.
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::{Component, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: PathBuf,
    pub game_id: String,
    pub source_system: PathBuf,
    pub controller_id: String,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    pub mapping_database: PathBuf,
    pub bubblewrap_program: PathBuf,
    pub executable_sha256: String,
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty() && !self.controller_id.trim().is_empty(),
            "PPSSPP requires emulator and saved controller identities"
        );
        ensure!(
            !self.game_id.is_empty()
                && self.game_id.len() <= 128
                && self
                    .game_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
            "PPSSPP requires an exact filename-safe game ID"
        );
        for path in [
            &self.content,
            &self.source_system,
            &self.probe_program,
            &self.sdl_library,
            &self.mapping_database,
            &self.bubblewrap_program,
        ] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "PPSSPP setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.source_system
                .file_name()
                .is_some_and(|name| name == "SYSTEM")
                && self
                    .source_system
                    .parent()
                    .and_then(|path| path.file_name())
                    .is_some_and(|name| name == "PSP"),
            "PPSSPP native setup requires the resolved PSP/SYSTEM configuration directory"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "PPSSPP requires the trusted native executable SHA-256"
        );
        Ok(())
    }

    pub(crate) fn review(
        &self,
        calibrations: &HashMap<String, Calibration>,
    ) -> Result<serde_json::Value> {
        self.validate()?;
        let calibration = calibrations
            .get(&self.controller_id)
            .context("PPSSPP controller has no saved calibration")?;
        ensure!(
            calibration.os == "linux",
            "Native PPSSPP mapping currently requires Linux calibration"
        );
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == "ppsspp:standalone-psp")
            .context("Missing standalone PPSSPP profile")?;
        let plan = calibration.plan_profile(profile)?;
        ensure!(
            plan.rows.iter().all(|row| row
                .input
                .as_ref()
                .is_some_and(|input| input.native.is_some())),
            "PPSSPP requires all PSP gameplay controls with measured native inputs"
        );
        Ok(
            serde_json::json!({"players":[{"pad":1,"controller_id":self.controller_id,
            "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":plan}],
            "launch_ready":false,"detail":"Physical mapping review only; native SDL runtime/order and launch routing remain unverified."}),
        )
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many PPSSPP saved setups");
    let mut seen = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            seen.insert((&setup.emulator_id, &setup.content)),
            "Duplicate PPSSPP emulator/content setup"
        );
    }
    Ok(())
}
