//! Fresh-handler SDL device naming at RPCS3 revision 54014a7de4b2ccec98c9c0cb7dbebec0606c5cd6.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

/// One successfully opened gamepad, in native SDL_GetGamepads order.
/// Names are gamepad names after opening, not joystick names or GUIDs.
pub(crate) struct Device<'a> {
    pub instance: u32,
    pub name: &'a str,
    pub path: &'a str,
}

pub(crate) struct Assignment {
    pub instance: u32,
    pub path: String,
    pub native_device: String,
}

/// Project a fresh native handler's naming, including unselected controllers.
/// The caller must verify native enumeration agrees before accepting routing;
/// helper-process enumeration alone is not proof of child-process identity.
pub(crate) fn project(opened: &[Device<'_>]) -> Result<Vec<Assignment>> {
    ensure!(opened.len() <= 256, "Too many RPCS3 SDL devices");
    let mut instances = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut names = BTreeMap::<&str, usize>::new();
    let mut result = Vec::with_capacity(opened.len());
    for device in opened {
        ensure!(
            device.instance != 0 && instances.insert(device.instance),
            "Duplicate or invalid RPCS3 SDL instance"
        );
        ensure!(
            !device.path.is_empty() && paths.insert(device.path),
            "RPCS3 SDL devices need distinct physical paths"
        );
        ensure!(
            device.name.len() <= 4096
                && !device.name.chars().any(char::is_control)
                && device.path.len() <= 32768
                && !device.path.chars().any(char::is_control),
            "Invalid RPCS3 SDL device metadata"
        );
        let count = names.entry(device.name).or_default();
        *count += 1;
        result.push(Assignment {
            instance: device.instance,
            path: device.path.to_owned(),
            native_device: format!("{} {}", device.name, count),
        });
    }
    Ok(result)
}
