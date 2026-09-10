//! Compose a fresh SDL2 observation with native PPSSPP configuration ownership.
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{sdl2::Snapshot, sdl2_mapping::MappingContext};
use std::path::Path;

pub(crate) struct PreparedInputs {
    pub(crate) configuration: super::configuration::PreparedConfiguration,
    context: MappingContext,
    device_index: u32,
    runtime_path: String,
}

impl PreparedInputs {
    pub(crate) fn prepare(
        calibration: &crate::controller_catalog::Calibration,
        snapshot: &Snapshot,
        runtime_path: &str,
        source_system: &Path,
        game_id: &str,
    ) -> Result<Self> {
        let device = snapshot.device_at_path(runtime_path)?;
        ensure!(
            device.is_game_controller && device.device_index < 10,
            "PPSSPP requires a resolved SDL GameController in its generic pad index range"
        );
        let released = device
            .sampled_state
            .as_ref()
            .context("PPSSPP helper did not capture physical input state")?;
        // The resolver checks the sampled state against saved released values;
        // an arbitrary held-state observation cannot become a neutral baseline.
        let controls = super::physical::resolve(calibration, snapshot, runtime_path, released)?;
        let configuration = super::configuration::PreparedConfiguration::prepare(
            source_system,
            game_id,
            device.device_index as u8,
            &controls,
        )?;
        Ok(Self {
            configuration,
            context: MappingContext::capture(snapshot, runtime_path)?,
            device_index: device.device_index,
            runtime_path: runtime_path.to_owned(),
        })
    }

    /// A topology guard must span the observations. Path equality by itself is
    /// not evidence that a disconnect/reconnect preserved physical identity.
    pub(crate) fn verify_observation(&self, fresh: &Snapshot) -> Result<()> {
        self.context.ensure_current(fresh, &self.runtime_path)?;
        ensure!(
            fresh.device_at_path(&self.runtime_path)?.device_index == self.device_index,
            "PPSSPP SDL device order changed during launch preparation"
        );
        self.configuration.verify_sources()
    }
}
