//! SDL/gui.c connect_joypad opens device index zero on a fresh start.
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::sdl2::Snapshot;

pub(crate) fn validate(snapshot: &Snapshot, selected_path: &str) -> Result<()> {
    ensure!(
        snapshot.version[0] == 2,
        "SameBoy SDL frontend requires an SDL2 inventory"
    );
    ensure!(
        snapshot
            .devices
            .iter()
            .enumerate()
            .all(|(index, device)| device.device_index as usize == index),
        "SameBoy requires a complete ordered SDL inventory"
    );
    let first = snapshot
        .devices
        .first()
        .context("SameBoy has no native joystick")?;
    ensure!(
        first.path.as_deref() == Some(selected_path),
        "SameBoy opens SDL device zero; the calibrated controller is not first in native order"
    );
    // Native has no saved GUID selector. Never rewrite a numeric index or
    // accept a same-name device as if it changed connect_joypad's behavior.
    snapshot.device_at_path(selected_path)?;
    Ok(())
}
