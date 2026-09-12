//! Saved native Yaba Sanshiro 2 launch setup.
//! Config target: `YabaSanshiro/qt/yabause.ini` under the XDG config home
//! (see `crate::controller_yaba_sanshiro` for the evidence-backed paths and
//! the `Input/Port/<port>/Id/<id>/...` key grammar). `port`/`device_id`
//! select the key-path identity the saved controller binds; their value
//! tables are source-unverified and stay caller-resolved.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: std::path::PathBuf,
    pub controller_id: String,
    pub port: u32,
    pub device_id: u32,
    pub probe_program: std::path::PathBuf,
    pub sdl_library: std::path::PathBuf,
    pub executable_sha256: String,
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> anyhow::Result<()> {
    anyhow::ensure!(
        setups.len() <= 1024,
        "Too many Yaba Sanshiro 2 saved setups"
    );
    for s in setups {
        s.validate()?;
    }
    Ok(())
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.emulator_id.trim().is_empty(),
            "Yaba Sanshiro 2 needs an emulator identity"
        );
        anyhow::ensure!(
            !self.controller_id.trim().is_empty(),
            "Yaba Sanshiro 2 needs a controller identity"
        );
        for path in [&self.content, &self.probe_program, &self.sdl_library] {
            anyhow::ensure!(path.is_absolute(), "Yaba Sanshiro 2 paths must be absolute");
        }
        anyhow::ensure!(
            self.executable_sha256.len() == 64,
            "Yaba Sanshiro 2 needs a 64-char SHA-256"
        );
        Ok(())
    }

    pub(crate) fn review(
        &self,
        calibrations: &std::collections::HashMap<String, crate::controller_catalog::Calibration>,
    ) -> anyhow::Result<serde_json::Value> {
        self.validate()?;
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == "yaba-sanshiro:standalone-saturn-digital")
            .ok_or_else(|| anyhow::anyhow!("Missing native Yaba Sanshiro 2 profile"))?;
        let calibration = calibrations
            .get(&self.controller_id)
            .ok_or_else(|| anyhow::anyhow!("Yaba Sanshiro 2 calibration disappeared"))?;
        let mapping = calibration.plan_profile(profile)?;
        Ok(serde_json::json!({"launch_ready":false,
            "launch_integration":"partial",
            "target_layout":profile.target_layout,
            "mapping":mapping}))
    }
}
