//! Join native settings discovery and supplied same-device calibration data.
use super::{
    configuration, discovery::Discovered, isolation::PreparedConfig, physical::Map,
    settings::SavedSetup,
};
use anyhow::Result;
use std::collections::BTreeMap;

/// The capture owner must resolve FileBase and prove native_id, map and raw
/// state all belong to the saved physical controller. This function performs
/// no device capture and does not treat a supplied ID as identity evidence.
pub(crate) fn prepare(
    setup: &SavedSetup,
    file_base: &str,
    native_id: &str,
    map: &Map,
    released_state: &BTreeMap<u32, i32>,
    calibration: &crate::controller_catalog::Calibration,
) -> Result<PreparedConfig> {
    setup.validate()?;
    anyhow::ensure!(
        setup.gamepad.max_players() == 1,
        "Multi-port Mednafen setup requires the session-wide preparation path"
    );
    let discovered = Discovered::capture(&setup.base_directory, setup.gamepad.system(), file_base)?;
    let threshold = configuration::axis_threshold(&discovered.effective()?)?;
    let controls = map.calibrated(
        calibration,
        setup.gamepad,
        native_id,
        released_state,
        threshold,
    )?;
    // Move the entire discovery owner, not just its existing-file snapshot:
    // missing override guards must live through preparation and child startup.
    PreparedConfig::prepare(discovered, setup.gamepad, native_id, &controls)
}
