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
    /// Existing native QSettings INI. It is copied, never edited in place.
    pub config_path: std::path::PathBuf,
    pub probe_program: std::path::PathBuf,
    pub sdl_library: std::path::PathBuf,
    pub bubblewrap_program: std::path::PathBuf,
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
        let mut path_vec = vec![
            &self.content,
            &self.config_path,
            &self.probe_program,
            &self.sdl_library,
        ];
        // bubblewrap is required only when a launch can actually sandbox;
        // elsewhere the field is accepted and ignored.
        #[cfg(target_os = "linux")]
        path_vec.push(&self.bubblewrap_program);
        for path in path_vec {
            anyhow::ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                "Yaba Sanshiro 2 paths must be absolute without parent traversal"
            );
        }
        anyhow::ensure!(
            self.config_path.file_name().and_then(|name| name.to_str()) == Some("yabause.ini"),
            "Yaba Sanshiro 2 config_path must name yabause.ini"
        );
        anyhow::ensure!(
            matches!(self.port, 1 | 2) && matches!(self.device_id, 1..=6),
            "Yaba Sanshiro 2 pad port/id is outside the standard Saturn topology"
        );
        anyhow::ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Yaba Sanshiro 2 needs a trusted executable SHA-256"
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
            "mapping":mapping,
            "detail":"Native Linux launch patches one Saturn pad entry in a copied yabause.ini and overlays only that file. The real backup-RAM and state roots remain active. Exact SDL2 device identity, mapping, and order are rechecked at launch; runtime behavior is unverified."}))
    }
}
