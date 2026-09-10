//! Saved native DeSmuME launch setup and visual review; no device I/O.
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
    /// Single physical controller; the GTK frontend maps one joypad to the
    /// DS keys and carries the device digit inside each code.
    pub controller_id: String,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    pub executable_sha256: String,
}

pub(crate) const PROFILE_ID: &str = "desmume:standalone-nds-native-buttons";

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "DeSmuME needs an emulator identity"
        );
        ensure!(
            !self.controller_id.trim().is_empty(),
            "DeSmuME needs a saved physical controller identity"
        );
        for path in [&self.content, &self.probe_program, &self.sdl_library] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "DeSmuME setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "DeSmuME needs a trusted executable SHA-256"
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
            .context("Missing native DeSmuME profile")?;
        let calibration = calibrations
            .get(&self.controller_id)
            .context("DeSmuME controller has no saved calibration")?;
        ensure!(
            calibration.os == "linux",
            "DeSmuME native mapping requires Linux calibration"
        );
        let mapping = calibration.plan_profile(profile)?;
        ensure!(
            mapping.rows.iter().all(|row| row.physical_id.is_some()
                && row
                    .input
                    .as_ref()
                    .is_some_and(|input| input.native.is_some())),
            "DeSmuME needs native calibration for every mapped DS control"
        );
        Ok(
            serde_json::json!({"players":[{"controller_id":self.controller_id,
            "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}],
            "launch_ready":false,
            "launch_integration":"partial",
            "detail":"DeSmuME JOYKEYS native dispatch is connected but untested. Launch isolates XDG_CONFIG_HOME and writes the private [JOYKEYS] section for the first SDL device. The touchscreen remains a mouse input; microphone, lid and Debug/Boost are explicitly disabled. Runtime testing remains deferred."}),
        )
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many DeSmuME saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate DeSmuME emulator/content setup"
        );
    }
    Ok(())
}
