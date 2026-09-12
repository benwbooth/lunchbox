//! Saved native Kronos launch setup.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: std::path::PathBuf,
    pub controller_id: String,
    pub probe_program: std::path::PathBuf,
    pub sdl_library: std::path::PathBuf,
    pub executable_sha256: String,
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> anyhow::Result<()> {
    anyhow::ensure!(setups.len() <= 1024, "Too many Kronos saved setups");
    for s in setups {
        s.validate()?;
    }
    Ok(())
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.emulator_id.trim().is_empty(),
            "Kronos needs an emulator identity"
        );
        anyhow::ensure!(
            !self.controller_id.trim().is_empty(),
            "Kronos needs a controller identity"
        );
        for path in [&self.content, &self.probe_program, &self.sdl_library] {
            anyhow::ensure!(path.is_absolute(), "Kronos paths must be absolute");
        }
        anyhow::ensure!(
            self.executable_sha256.len() == 64,
            "Kronos needs a 64-char SHA-256"
        );
        Ok(())
    }
}
