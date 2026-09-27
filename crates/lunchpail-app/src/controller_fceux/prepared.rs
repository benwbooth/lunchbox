//! Join effective routing and private files using one config snapshot.
use super::{
    configuration::{self, Selection},
    isolation::PreparedConfig,
    layers::Snapshot,
    profile_mount::PreparedProfiles,
    routing::{self, Device},
};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) struct PreparedMapping {
    config: PreparedConfig,
    profiles: PreparedProfiles,
    port_guids: [String; 4],
    selected_paths: BTreeMap<u8, PathBuf>,
}

impl PreparedMapping {
    /// Inventory and profile bytes are produced by the native capture owner.
    /// This does not probe devices or start the emulator.
    pub(crate) fn prepare(
        root: &Path,
        profiles: Vec<(Selection, String)>,
        devices: &[Device],
        selected_paths: BTreeMap<u8, PathBuf>,
    ) -> Result<Self> {
        ensure!(
            profiles.len() == selected_paths.len()
                && profiles
                    .iter()
                    .all(|(selection, _)| selected_paths.contains_key(&selection.player)),
            "FCEUX selected paths must match generated profile players"
        );
        let snapshot = Snapshot::capture(root)?;
        let mut effective = snapshot.effective()?;
        // Every layer receives these same selected overrides; retain all
        // untouched port GUIDs from the captured native merge for allocation.
        for (selection, _) in &profiles {
            effective.extend(configuration::assignments(&configuration::render(
                "",
                std::slice::from_ref(selection),
            )?)?);
        }
        let port_guids = configuration::port_guids(&effective);
        routing::validate_selected(devices, &port_guids, &selected_paths)?;
        let mounted_profiles = PreparedProfiles::prepare(root, &profiles)?;
        let selections: Vec<_> = profiles
            .into_iter()
            .map(|(selection, _)| selection)
            .collect();
        let config = PreparedConfig::from_snapshot(snapshot, &selections)?;
        let prepared = Self {
            config,
            profiles: mounted_profiles,
            port_guids,
            selected_paths,
        };
        prepared.verify(devices)?;
        Ok(prepared)
    }

    pub(crate) fn verify(&self, devices: &[Device]) -> Result<()> {
        self.config.verify()?;
        self.profiles.verify()?;
        routing::validate_selected(devices, &self.port_guids, &self.selected_paths)
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn verify_child_mounts(&self, pid: u32) -> Result<()> {
        self.config.verify_child_mounts(pid)?;
        self.profiles.verify_child_mounts(pid)
    }

    pub(crate) fn append_mounts(
        &self,
        arguments: &mut Vec<OsString>,
        devices: &[Device],
    ) -> Result<()> {
        self.verify(devices)?;
        self.config.append_mounts(arguments)?;
        self.profiles.append_mounts(arguments)
    }
}
