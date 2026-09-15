//! Saved native Kronos launch setup.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Player {
    pub player: u8,
    pub controller_id: String,
    pub port: u8,
    pub device_id: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: PathBuf,
    /// Existing native QSettings INI. The launch copies and overlays this one
    /// file; it never edits the user's configuration in place.
    pub config_path: PathBuf,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    pub bubblewrap_program: PathBuf,
    pub executable_sha256: String,
    pub players: Vec<Player>,
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many Kronos saved setups");
    for setup in setups {
        setup.validate()?;
    }
    Ok(())
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "Kronos needs an emulator identity"
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
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                "Kronos paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.config_path.file_name().and_then(|name| name.to_str()) == Some("kronos.ini"),
            "Kronos config_path must name kronos.ini"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Kronos needs a trusted executable SHA-256"
        );
        ensure!(
            matches!(self.players.len(), 1..=4),
            "Kronos raw SDL encoding supports one to four players"
        );
        let mut controllers = BTreeSet::new();
        let mut slots = BTreeSet::new();
        for (index, player) in self.players.iter().enumerate() {
            ensure!(
                usize::from(player.player) == index + 1,
                "Kronos players must be contiguous and start at one"
            );
            ensure!(
                !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "Kronos players need distinct controller identities"
            );
            ensure!(
                matches!(player.port, 1 | 2)
                    && matches!(player.device_id, 1..=6)
                    && slots.insert((player.port, player.device_id)),
                "Kronos players need distinct native port/id slots"
            );
        }
        Ok(())
    }

    pub(crate) fn review(
        &self,
        calibrations: &HashMap<String, crate::controller_catalog::Calibration>,
    ) -> Result<serde_json::Value> {
        self.validate()?;
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == crate::controller_kronos::PROFILE_ID)
            .context("Missing native Kronos profile")?;
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations.get(&player.controller_id).with_context(|| {
                format!("Kronos player {} calibration disappeared", player.player)
            })?;
            players.push(serde_json::json!({
                "player": player.player,
                "port": player.port,
                "device_id": player.device_id,
                "mapping": calibration.plan_profile(profile)?,
            }));
        }
        Ok(serde_json::json!({
            "launch_ready": false,
            "launch_integration": "partial",
            "target_layout": profile.target_layout,
            "players": players,
            "detail": "Native launch translates each calibrated physical input into Kronos raw SDL joystick codes, patches complete Saturn pad entries in a copied kronos.ini, and overlays only that file. Real backup RAM, cartridge, state, BIOS, and media paths remain active. Exact SDL2 routing is rechecked at launch; runtime behavior is unverified."
        }))
    }
}
