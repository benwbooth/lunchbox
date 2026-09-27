//! FCEUX Qt fresh-start port selection, matching GamePad_t::init.
use anyhow::{Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub(crate) struct Device {
    /// FCEUX jsDev slot, not an SDL instance ID.
    pub slot: u8,
    pub guid: String,
    pub path: PathBuf,
    pub game_controller: bool,
}

/// All four native gamepad GUID settings are required, including untouched
/// ports: an earlier unselected port can otherwise consume a selected device.
pub(crate) fn project(devices: &[Device], port_guids: &[String; 4]) -> Result<[Option<u8>; 4]> {
    ensure!(
        devices.len() <= 32,
        "FCEUX inventory exceeds native joystick slots"
    );
    let mut ordered = BTreeMap::new();
    let mut paths = BTreeSet::new();
    for device in devices {
        ensure!(
            device.slot < 32
                && ordered.insert(device.slot, device).is_none()
                && device.path.is_absolute()
                && paths.insert(&device.path),
            "FCEUX inventory needs distinct native slots and physical paths"
        );
        ensure!(
            device.guid.len() == 32 && device.guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "FCEUX native device GUID is invalid"
        );
    }
    let mut used = BTreeSet::new();
    let mut result = [None; 4];
    for (port, guid) in port_guids.iter().enumerate() {
        ensure!(!guid.contains('\0'), "FCEUX configured GUID contains NUL");
        // Native strcmp is case-sensitive. Preserve the exact captured GUID.
        let exact = ordered
            .values()
            .find(|device| !used.contains(&device.slot) && device.guid == *guid);
        let selected = exact.or_else(|| {
            if guid == "keyboard" {
                None
            } else {
                ordered
                    .values()
                    .find(|device| !used.contains(&device.slot) && device.game_controller)
            }
        });
        if let Some(device) = selected {
            used.insert(device.slot);
            result[port] = Some(device.slot);
        }
    }
    Ok(result)
}

/// Require explicit calibrated ports to receive their measured physical paths,
/// never accepting a GUID match or native fallback as proof of identity.
pub(crate) fn validate_selected(
    devices: &[Device],
    port_guids: &[String; 4],
    selected_paths: &BTreeMap<u8, PathBuf>,
) -> Result<()> {
    ensure!(
        !selected_paths.is_empty() && selected_paths.len() <= 4,
        "FCEUX needs one to four selected players"
    );
    let projected = project(devices, port_guids)?;
    let mut paths = BTreeSet::new();
    for (player, path) in selected_paths {
        ensure!(
            (1..=4).contains(player) && paths.insert(path),
            "FCEUX selected players/controllers must be distinct"
        );
        let slot = projected[usize::from(*player - 1)];
        ensure!(
            devices
                .iter()
                .any(|device| Some(device.slot) == slot && device.path == *path),
            "FCEUX GUID/fallback allocation does not select the calibrated controller for player {player}"
        );
    }
    Ok(())
}
