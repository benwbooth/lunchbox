//! Retained native Linux inventory; no SDL GUID substitution.
use super::{enumeration::Enumeration, identity, physical::Map, sysfs::Identity};
use anyhow::{Result, ensure};
use std::{os::unix::fs::MetadataExt, path::PathBuf, sync::atomic::AtomicBool};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Device {
    pub path: PathBuf,
    pub event: PathBuf,
    pub identity: Identity,
    pub map: Map,
    pub native_id: String,
    nodes: [(u64, u64, u64); 2],
}

fn node(path: &std::path::Path) -> Result<(u64, u64, u64)> {
    let metadata = std::fs::metadata(path)?;
    Ok((metadata.dev(), metadata.ino(), metadata.rdev()))
}

pub(crate) struct Inventory {
    enumeration: Enumeration,
    pub devices: Vec<Device>,
}

impl Inventory {
    pub(crate) fn capture(cancel: &AtomicBool) -> Result<Self> {
        let enumeration = Enumeration::capture()?;
        let mut devices = Vec::new();
        let mut base_ids = Vec::new();
        for path in &enumeration.paths {
            crate::controller_native_process::cancelled(cancel)?;
            let identity = Identity::capture(path)?;
            let event = identity.event_path()?;
            let before = [node(path)?, node(&event)?];
            // Reject incomplete capture rather than invent the native cache
            // position after an inaccessible device. Native skip behavior can
            // be added once failed-open outcomes are represented explicitly.
            let map = Map::capture(path)?;
            identity.verify()?;
            ensure!(
                before == [node(path)?, node(&event)?],
                "Mednafen device node changed during capture"
            );
            base_ids.push(identity.base_id(&map)?);
            devices.push(Device {
                path: path.clone(),
                event,
                identity,
                map,
                native_id: String::new(),
                nodes: before,
            });
        }
        for (device, id) in devices.iter_mut().zip(identity::allocate(&base_ids)?) {
            device.native_id = identity::as_hex(&id);
        }
        enumeration.verify()?;
        Ok(Self {
            enumeration,
            devices,
        })
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        self.enumeration.verify()?;
        let fresh = Self::capture(cancel)?;
        ensure!(
            fresh.devices == self.devices,
            "Mednafen native device identity/map/order changed"
        );
        Ok(())
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.enumeration.verify()?;
        for device in &self.devices {
            device.identity.verify()?;
            ensure!(
                device.nodes == [node(&device.path)?, node(&device.event)?],
                "Mednafen controller node changed"
            );
        }
        Ok(())
    }
}
