//! Native mGBA SDL saved intent; settings operations never open input devices.
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::{Component, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Handheld {
    Gba,
    Gameboy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: PathBuf,
    pub handheld: Handheld,
    pub controller_id: String,
    pub source_config: PathBuf,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    pub bubblewrap_program: PathBuf,
    pub executable_sha256: String,
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty() && !self.controller_id.trim().is_empty(),
            "mGBA requires emulator and physical controller identities"
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
                "mGBA setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.source_config
                .file_name()
                .is_some_and(|name| name == "config.ini"),
            "mGBA requires the actual native config.ini location"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "mGBA requires the trusted SDL frontend executable SHA-256"
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
            .context("mGBA controller has no saved calibration")?;
        ensure!(
            calibration.os == "linux",
            "Native mGBA mapping currently requires Linux calibration"
        );
        let profile_id = match self.handheld {
            Handheld::Gba => "mgba:standalone-gba",
            Handheld::Gameboy => "mgba:standalone-gameboy",
        };
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .context("Missing native mGBA mapping profile")?;
        let mapping = calibration.plan_profile(profile)?;
        ensure!(
            mapping.rows.iter().all(|row| row.physical_id.is_some()
                && row
                    .input
                    .as_ref()
                    .is_some_and(|input| input.native.is_some())),
            "mGBA requires saved native calibration for every gameplay control"
        );
        Ok(
            serde_json::json!({"players":[{"pad":1,"controller_id":self.controller_id,
            "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}],
            "launch_ready":false,"detail":"Saved mapping review only; native SDL identity and runtime handoff are checked at launch, not by this review."}),
        )
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many mGBA saved setups");
    let mut seen = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            seen.insert((&setup.emulator_id, &setup.content)),
            "Duplicate mGBA emulator/content setup"
        );
    }
    Ok(())
}
