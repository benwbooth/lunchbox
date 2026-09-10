//! Explicit saved native Dolphin GameCube setup; no hardware or file I/O.
use crate::controller_catalog::{Calibration, catalog};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::{Component, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Player {
    pub port: u8,
    pub controller_id: String,
    pub device_qualifier: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    /// Guided players select physical identities, not volatile evdev ordinals.
    /// Resolve qualifiers from the launch inventory; legacy setups retain their
    /// explicit qualifier check. Empty qualifiers are allowed only in this mode.
    #[serde(default)]
    pub resolve_devices_at_launch: bool,
    pub emulator_id: String,
    pub content: PathBuf,
    pub game_id: String,
    pub revision: u8,
    pub user_directory: PathBuf,
    pub system_directory: PathBuf,
    pub bubblewrap_program: PathBuf,
    pub executable_sha256: String,
    pub players: Vec<Player>,
}

impl SavedSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "Dolphin emulator identity is absent"
        );
        ensure!(
            self.game_id.len() == 6
                && self
                    .game_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric()),
            "Dolphin needs the exact six-character GameCube disc ID"
        );
        for path in [
            &self.content,
            &self.user_directory,
            &self.system_directory,
            &self.bubblewrap_program,
        ] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "Dolphin setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Dolphin needs a trusted native executable SHA-256"
        );
        ensure!(
            !self.players.is_empty() && self.players.len() <= 4,
            "Dolphin needs one to four selected GameCube pads"
        );
        let mut ports = BTreeSet::new();
        let mut controllers = BTreeSet::new();
        let mut qualifiers = BTreeSet::new();
        for player in &self.players {
            ensure!(
                (1..=4).contains(&player.port) && ports.insert(player.port),
                "Duplicate or invalid Dolphin pad port"
            );
            ensure!(
                !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "Dolphin needs distinct physical controllers for selected players"
            );
            if self.resolve_devices_at_launch {
                ensure!(
                    player.device_qualifier.is_empty(),
                    "Automatically resolved Dolphin players must not carry a stale native qualifier"
                );
                continue;
            }
            let parts: Vec<_> = player.device_qualifier.splitn(3, '/').collect();
            ensure!(
                parts.len() == 3
                    && parts[0] == "evdev"
                    && parts[1].parse::<u32>().is_ok()
                    && !parts[2].trim().is_empty()
                    && player.device_qualifier.len() <= 512
                    && !player
                        .device_qualifier
                        .chars()
                        .any(|ch| ch.is_control() || matches!(ch, '`' | '"'))
                    && qualifiers.insert(&player.device_qualifier),
                "Dolphin needs distinct native evdev/id/name qualifiers; names alone are not device identity"
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
            .find(|profile| profile.id == "dolphin:standalone-gamecube")
            .context("Missing native Dolphin controller profile")?;
        let mut players = Vec::new();
        for player in &self.players {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Dolphin controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Dolphin evdev mapping requires Linux calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            for row in &mapping.rows {
                let input = row
                    .input
                    .as_ref()
                    .context("Dolphin gameplay target is not calibrated")?;
                ensure!(
                    row.physical_id.is_some() && input.native.is_some(),
                    "Dolphin requires native measurements for all gameplay controls"
                );
                ensure!(
                    !super::analog_target(&row.output) || input.axis.is_some(),
                    "Dolphin analog target has no measured axis endpoints"
                );
            }
            players.push(serde_json::json!({"pad":player.port,"controller_id":player.controller_id,
                "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}));
        }
        Ok(serde_json::json!({"players":players,"launch_ready":false,
            "detail":"Saved mapping review only; native Linux Dolphin 2606 ISO/GCM dispatch requires launch-time device and runtime checks. System-data directory is user-declared; runtime compatibility remains untested."}))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many Dolphin saved setups");
    let mut seen = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            seen.insert((&setup.emulator_id, &setup.content)),
            "Duplicate Dolphin emulator/content setup"
        );
    }
    Ok(())
}
