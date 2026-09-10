//! Saved native Mesen2 launch setup and visual review; no device I/O.
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Component, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: PathBuf,
    /// Single physical controller; the pad slot is verified at launch by
    /// requiring the sole qualifying /dev/input/event device.
    pub controller_id: String,
    pub probe_program: PathBuf,
    pub executable_sha256: String,
}

pub(crate) const PROFILE_ID: &str = "mesen2:standalone-nes";

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "Mesen2 needs an emulator identity"
        );
        ensure!(
            !self.controller_id.trim().is_empty(),
            "Mesen2 needs a saved physical controller identity"
        );
        for path in [&self.content, &self.probe_program] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "Mesen2 setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Mesen2 needs a trusted executable SHA-256"
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
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing native Mesen2 profile")?;
        let calibration = calibrations
            .get(&self.controller_id)
            .context("Mesen2 controller has no saved calibration")?;
        ensure!(
            calibration.os == "linux",
            "Mesen2 native mapping requires Linux calibration"
        );
        let mapping = calibration.plan_profile(profile)?;
        ensure!(
            mapping.rows.iter().all(|row| row.physical_id.is_some()
                && row
                    .input
                    .as_ref()
                    .is_some_and(|input| input.native.is_some())),
            "Mesen2 needs native calibration for every standard NES control"
        );
        Ok(
            serde_json::json!({"players":[{"controller_id":self.controller_id,
            "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}],
            "launch_ready":false,
            "launch_integration":"partial",
            "detail":"Mesen2 settings.json native dispatch is connected but untested. Launch isolates XDG_DATA_HOME and writes the NES Port1 mapping for the single qualifying event device (pad slot 0). Zapper, Power Pad, Four Score, multitaps, SNES and every other system's controllers are not covered; runtime testing remains deferred."}),
        )
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many Mesen2 saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate Mesen2 emulator/content setup"
        );
    }
    Ok(())
}
