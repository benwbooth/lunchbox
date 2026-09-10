//! Compose calibrated arcade controls and owned native configuration.
//! The launch session must supply and verify emulator-process identity evidence.
use super::{
    configuration::PreparedConfig, isolation::PreparedMappings, routing::PlayerPort,
    settings::SavedSetup,
};
use crate::controller_catalog::Calibration;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::sdl2::Device;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(crate) struct Controller<'a> {
    pub controller_id: &'a str,
    pub device: &'a Device,
    /// Provisional same-runtime ID; the launch session must confirm it against
    /// the actual child's opened-joystick records before accepting handoff.
    pub native_instance: i32,
}

pub(crate) struct PreparedPanel {
    pub config: PreparedConfig,
    pub mappings: PreparedMappings,
}

impl PreparedPanel {
    pub(crate) fn create(
        setup: &SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        controllers: &[Controller<'_>],
        observed_native_instances: &[i32],
    ) -> Result<Self> {
        setup.review(calibrations)?;
        ensure!(
            controllers.len() == setup.players.len(),
            "Flycast physical controller set differs from saved players"
        );
        let mut identities = BTreeSet::new();
        let mut physical_paths = BTreeSet::new();
        for controller in controllers {
            let path = controller
                .device
                .path
                .as_deref()
                .context("Flycast SDL physical path is missing")?;
            ensure!(
                !path.is_empty()
                    && physical_paths.insert(path)
                    && identities.insert(controller.controller_id),
                "Flycast controllers require distinct physical paths and saved identities"
            );
        }
        let mut entries = Vec::new();
        let mut ports = Vec::new();
        for player in &setup.players {
            let controller = controllers
                .iter()
                .find(|entry| entry.controller_id == player.controller_id)
                .context("Flycast saved controller was not resolved")?;
            let name = controller
                .device
                .name
                .as_deref()
                .context("Flycast SDL name is missing")?;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Flycast calibration is missing")?;
            // Generated mappings are complete, so inherited trigger metadata is
            // not imported. Native SDL fallback classification is used instead.
            let mapping = super::physical::panel_mapping_from_sdl(
                calibration,
                &player.source_controls,
                controller.device,
                player.panel,
                &BTreeMap::new(),
            )?;
            // Own the highest-priority instance filename in both game/global
            // scopes and both arcade/console phases. Model-wide files would
            // collide for identical controllers with different player mappings.
            for arcade in [true, false] {
                for game in [Some(setup.game_id.as_str()), None] {
                    entries.push((
                        super::paths::filename(
                            name,
                            Some(controller.native_instance),
                            game,
                            arcade,
                        )?,
                        mapping.clone(),
                    ));
                }
            }
            ports.push(PlayerPort {
                native_instance: controller.native_instance,
                player: player.player,
            });
        }
        let mappings = PreparedMappings::create(&entries, Vec::new())?;
        let config = PreparedConfig::create(
            &setup.source_config,
            &mappings,
            observed_native_instances,
            &ports,
        )?;
        Ok(Self { config, mappings })
    }

    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        self.mappings.verify_before_launch()?;
        self.config.verify_before_launch()
    }
}
