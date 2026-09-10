//! Saved native native setup and visual review; no device or filesystem I/O.
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
    /// Omitted in older setups: inherit the setup-wide pad choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gamepad: Option<super::profiles::Gamepad>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    pub content: PathBuf,
    pub base_directory: PathBuf,
    pub bubblewrap_program: PathBuf,
    pub executable_sha256: String,
    pub gamepad: super::profiles::Gamepad,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md_tap: Option<super::md::Tap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saturn_multitaps: Option<[bool; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub psx_multitaps: Option<[bool; 2]>,
    pub players: Vec<Player>,
}

impl SavedSetup {
    pub(crate) fn player_gamepad(&self, player: &Player) -> Result<super::profiles::Gamepad> {
        let selected = player.gamepad.unwrap_or(self.gamepad);
        ensure!(
            self.gamepad.system() != "nes" || selected == self.gamepad,
            "NES adapter choice applies to the whole setup, not individual players"
        );
        ensure!(
            selected.system() == self.gamepad.system(),
            "Mednafen per-player pad must belong to the setup's native system"
        );
        Ok(selected)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.psx_multitaps.is_none() || self.gamepad.system() == "psx",
            "psx_multitaps only applies to PlayStation setups"
        );
        ensure!(
            self.saturn_multitaps.is_none() || self.gamepad.system() == "ss",
            "saturn_multitaps only applies to Saturn setups"
        );
        ensure!(
            self.md_tap.is_none() || self.gamepad.system() == "md",
            "md_tap only applies to Genesis setups"
        );
        let max_players = if self.gamepad.system() == "psx" {
            super::psx::capacity(self.psx_multitaps.unwrap_or([false; 2])) as u8
        } else if self.gamepad.system() == "ss" {
            super::saturn::capacity(self.saturn_multitaps.unwrap_or([false; 2])) as u8
        } else if self.gamepad.system() == "md" {
            self.md_tap.unwrap_or(super::md::Tap::None).max_players()
        } else {
            self.gamepad.max_players()
        };
        ensure!(
            !self.emulator_id.trim().is_empty(),
            "Mednafen needs an emulator identity"
        );
        for path in [
            &self.content,
            &self.base_directory,
            &self.bubblewrap_program,
        ] {
            ensure!(
                path.is_absolute()
                    && !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir)),
                "Mednafen setup paths must be absolute without parent traversal"
            );
        }
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Mednafen needs a trusted native executable SHA-256"
        );
        ensure!(
            !self.players.is_empty() && self.players.len() <= usize::from(max_players),
            "Mednafen player count exceeds the selected system's native ports"
        );
        let mut ports = BTreeSet::new();
        let mut controllers = BTreeSet::new();
        for player in &self.players {
            let gamepad = self.player_gamepad(player)?;
            ensure!(
                (1..=max_players.min(gamepad.max_players())).contains(&player.player)
                    && ports.insert(player.player)
                    && !player.controller_id.trim().is_empty()
                    && controllers.insert(&player.controller_id),
                "Mednafen players and physical controllers must be distinct"
            );
        }
        Ok(())
    }

    pub(crate) fn review(
        &self,
        calibrations: &HashMap<String, Calibration>,
    ) -> Result<serde_json::Value> {
        self.validate()?;
        let mut players = Vec::new();
        for player in &self.players {
            let gamepad = self.player_gamepad(player)?;
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == gamepad.profile_id())
                .context("Missing native Mednafen profile")?;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Mednafen controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Mednafen native mapping requires Linux calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Mednafen needs native calibration for every selected controller control"
            );
            if gamepad == super::profiles::Gamepad::PlayStationDualAnalog {
                let mut axes = BTreeSet::new();
                for (negative, positive) in super::psx::STICK_PAIRS {
                    let get = |output: &str| -> Result<_> {
                        let input = mapping
                            .rows
                            .iter()
                            .find(|row| row.output == output)
                            .and_then(|row| row.input.as_ref())
                            .context("Dual Analog direction has no saved input")?;
                        let native = input
                            .native
                            .as_ref()
                            .context("Dual Analog direction has no native identity")?;
                        let axis = input
                            .axis
                            .as_ref()
                            .context("Dual Analog direction needs an axis measurement")?;
                        axis.validate()?;
                        ensure!(
                            native.code >> 16 == 3
                                && !(0x10..=0x17).contains(&(native.code & 0xffff)),
                            "Dual Analog sticks cannot use buttons or hat switches"
                        );
                        ensure!(
                            (i64::from(axis.pressed) - i64::from(axis.released)).abs() > 2,
                            "Dual Analog stick measurement is discrete"
                        );
                        Ok((native.code, axis))
                    };
                    let (a_code, a) = get(negative)?;
                    let (b_code, b) = get(positive)?;
                    ensure!(
                        a_code == b_code
                            && axes.insert(a_code)
                            && a.direction() != b.direction()
                            && a.minimum == b.minimum
                            && a.maximum == b.maximum
                            && a.flat == b.flat
                            && a.fuzz == b.fuzz
                            && a.resolution == b.resolution,
                        "Dual Analog requires four distinct physical axes with consistent opposite-direction measurements"
                    );
                }
            }
            players.push(serde_json::json!({"pad":player.player,"controller_id":player.controller_id,"gamepad":gamepad,
                "source_layout":calibration.layout,"target_layout":profile.target_layout,"mapping":mapping}));
        }
        if self.gamepad.system() == "psx" {
            return Ok(serde_json::json!({"players":players,"launch_ready":false,
                "launch_integration":"partial",
                "detail":"PlayStation digital and forced Dual Analog CCD/CUE dispatch is connected, including mixed pads and multitaps. Review checks saved analog measurements; native corrected neutral and device ownership are checked at launch. DualShock, firmware identity and runtime behavior remain unverified. Review opens no devices."}));
        }
        if self.gamepad.system() == "ss" {
            return Ok(serde_json::json!({"players":players,"launch_ready":false,
                "launch_integration":"partial",
                "detail":"Saturn digital pad CCD/CUE launch dispatch is connected, including six-way taps and disc dependency tracking. TOC/M3U, firmware identity and runtime behavior remain unverified. Review opens no devices."}));
        }
        Ok(serde_json::json!({"players":players,"launch_ready":false,
            "launch_integration":"partial",
            "detail":"Native Linux GB/GBA/Lynx/Neo Geo Pocket/WonderSwan/Virtual Boy/Game Gear/Master System/PC Engine saved-setup dispatch is implemented. Launch captures joydev identity, corrected axis state and effective config thresholds. Child internal IDs, other native input drivers and runtime behavior remain unverified. Review opens no devices and does not establish launch readiness."}))
    }
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many Mednafen saved setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert((&setup.emulator_id, &setup.content)),
            "Duplicate Mednafen emulator/content setup"
        );
    }
    Ok(())
}
