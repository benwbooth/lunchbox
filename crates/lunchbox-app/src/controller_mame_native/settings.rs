//! Saved standard-panel declarations. No native discovery is inferred from JSON.
use super::{DeviceSlot, Panel, Player, tokens::Item};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedPlayer {
    pub player: u8,
    pub controller_id: String,
    pub native_device_id: String,
    pub panel: Panel,
    pub controls: BTreeMap<String, Item>,
    /// Target control to saved physical-layout control. This is a declaration,
    /// not proof that the native MAME item identifies the same physical input.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub source_controls: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedSetup {
    pub emulator_id: String,
    /// Guided launch-local declarations contain only physical source choices.
    /// Native IDs and item numbers are resolved from the captured SDL devices.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resolve_inputs_at_launch: bool,
    pub executable: PathBuf,
    pub executable_sha256: String,
    /// Exact MAME provider name; not an inferred SDL backend.
    pub joystick_provider: String,
    /// Native joystick_threshold, stored in ten-thousandths to retain Eq.
    #[serde(default = "default_threshold")]
    pub threshold_basis_points: u16,
    /// Explicit source cfg directory, before session-local input overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfg_directory: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<Runtime>,
    pub players: Vec<SavedPlayer>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Runtime {
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
}

impl SavedSetup {
    pub(crate) fn review(
        &self,
        calibrations: &std::collections::HashMap<String, crate::controller_catalog::Calibration>,
    ) -> Result<serde_json::Value> {
        self.validate()?;
        ensure!(
            !self.resolve_inputs_at_launch,
            "MAME native inputs have not been resolved yet"
        );
        let maps = self.token_maps()?;
        let players: Vec<_> = self.players.iter().zip(maps).map(|(player, tokens)| -> Result<_> {
            let calibration = calibrations.get(&player.controller_id);
            let layout = calibration.and_then(|saved| crate::controller_catalog::catalog().layout(&saved.layout));
            let mut labels = BTreeMap::new();
            if !player.source_controls.is_empty() {
                let saved = calibration.context("MAME source links require saved physical calibration")?;
                saved.validate()?;
                let layout = layout.context("MAME source layout is absent")?;
                for physical in player.source_controls.values() {
                    let control = layout.controls.iter().find(|control| &control.id == physical)
                        .context("MAME source control is absent from the saved layout")?;
                    ensure!(saved.bindings.contains_key(physical), "MAME source control has no saved calibration");
                    labels.insert(physical, &control.label);
                }
            }
            Ok(serde_json::json!({"player":player.player,"controller_id":player.controller_id,
                "native_device_id":player.native_device_id,
                "source_layout":layout.map(|layout| &layout.id),"source_controls":player.source_controls,"source_labels":labels,
                "target_layout":if player.panel == Panel::Six {"arcade-six-button"} else {"arcade-eight-button"},
                "bindings":tokens}))
        }).collect::<Result<_>>()?;
        Ok(
            serde_json::json!({"emulator_id":self.emulator_id,"players":players,
            "launch_ready":false,"launch_integration":"partial",
            "detail":"Partial native Linux raw SDL dispatch is connected. Review only checks saved declarations; launch verifies device paths, GUIDs and physical item correspondence. Runtime compatibility and internal MAME routing remain unverified. No devices were opened."}),
        )
    }

    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.runtime
                .as_ref()
                .is_none_or(|runtime| runtime.probe_program.is_absolute()
                    && runtime.sdl_library.is_absolute()),
            "MAME probe and SDL library paths must be absolute"
        );
        ensure!(
            self.cfg_directory
                .as_ref()
                .is_none_or(|path| path.is_absolute()),
            "MAME source cfg directory must be absolute"
        );
        ensure!(
            self.threshold_basis_points <= 10_000,
            "MAME threshold must be between 0 and 10000 basis points"
        );
        ensure!(
            !self.emulator_id.trim().is_empty() && self.executable.is_absolute(),
            "MAME setup requires emulator identity and absolute executable path"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "MAME setup requires a trusted executable SHA-256"
        );
        ensure!(
            !self.joystick_provider.is_empty()
                && self.joystick_provider.len() <= 64
                && self
                    .joystick_provider
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                && self.joystick_provider != "auto"
                && self.joystick_provider != "none",
            "MAME setup requires an explicit joystick provider"
        );
        let mut physical = BTreeSet::new();
        let mut native = BTreeSet::new();
        let mut ports = BTreeSet::new();
        ensure!(
            !self.players.is_empty() && self.players.len() <= 8,
            "Invalid MAME player count"
        );
        for player in &self.players {
            ensure!(
                (1..=8).contains(&player.player) && ports.insert(player.player),
                "Invalid or duplicate MAME player"
            );
            ensure!(
                !player.controller_id.trim().is_empty() && physical.insert(&player.controller_id),
                "MAME players require distinct physical controllers"
            );
            let mut source_owners = BTreeSet::new();
            if self.resolve_inputs_at_launch {
                let routes = player.panel.routes(player.player);
                ensure!(
                    self.joystick_provider == "sdl"
                        && player.native_device_id.is_empty()
                        && player.controls.is_empty(),
                    "Guided MAME sources cannot contain predeclared native identities or items"
                );
                ensure!(
                    player.source_controls.len() == routes.len()
                        && routes
                            .keys()
                            .all(|key| player.source_controls.contains_key(key))
                        && player
                            .source_controls
                            .values()
                            .all(|source| !source.is_empty() && source_owners.insert(source)),
                    "Guided MAME panel needs distinct sources for directions, Start, Coin and all buttons"
                );
                continue;
            }
            ensure!(
                player.source_controls.iter().all(|(target, source)| player
                    .controls
                    .contains_key(target)
                    && !source.is_empty()
                    && source_owners.insert(source)),
                "MAME source links contain unknown targets or duplicate physical controls"
            );
            ensure!(
                !player.native_device_id.trim().is_empty()
                    && player.native_device_id.len() <= 4096
                    && player
                        .native_device_id
                        .chars()
                        .all(|ch| !ch.is_control() && ch != '\u{fffe}' && ch != '\u{ffff}')
                    && native.insert(&player.native_device_id),
                "MAME players require distinct full native device IDs"
            );
        }
        if self.resolve_inputs_at_launch {
            return Ok(());
        }
        let maps = self.token_maps()?;
        super::controller_xml(&self.player_refs(&maps))?;
        Ok(())
    }

    fn token_maps(&self) -> Result<Vec<BTreeMap<String, String>>> {
        self.players
            .iter()
            .map(|player| super::joystick_tokens(player.player, &player.controls))
            .collect()
    }

    fn player_refs<'a>(&self, maps: &'a [BTreeMap<String, String>]) -> Vec<Player<'a>> {
        self.players
            .iter()
            .zip(maps)
            .map(|(player, tokens)| Player {
                number: player.player,
                panel: player.panel,
                tokens,
            })
            .collect()
    }

    /// Requires externally captured native IDs. Saved declarations must never
    /// be substituted for this inventory by a launch caller.
    pub(crate) fn render(&self, observed_joystick_ids: &[String]) -> Result<String> {
        self.validate()?;
        ensure!(
            !self.resolve_inputs_at_launch,
            "Resolve MAME native inputs before rendering"
        );
        let maps = self.token_maps()?;
        let devices: Vec<_> = self
            .players
            .iter()
            .map(|player| DeviceSlot {
                native_id: &player.native_device_id,
                number: player.player,
            })
            .collect();
        super::mapped_controller_xml(&self.player_refs(&maps), &devices, observed_joystick_ids)
    }
}

fn default_threshold() -> u16 {
    3000
}

pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
    ensure!(setups.len() <= 1024, "Too many standalone MAME setups");
    let mut identities = BTreeSet::new();
    for setup in setups {
        setup.validate()?;
        ensure!(
            identities.insert(&setup.emulator_id),
            "Duplicate standalone MAME emulator setup"
        );
    }
    Ok(())
}
