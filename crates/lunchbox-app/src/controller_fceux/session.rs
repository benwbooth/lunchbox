//! Native Qt launch-time SDL inventory and physical calibration ownership.
use super::{
    configuration::Selection, physical::calibrated_profile, prepared::PreparedMapping,
    routing::Device, settings::SavedSetup,
};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{file_hash, sdl2::Snapshot};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(setup: &SavedSetup, path: Option<&str>, cancel: &AtomicBool) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command
        .arg("--sdl2-inventory")
        .arg("--sdl-library")
        .arg(&setup.sdl_library);
    if let Some(path) = path {
        command.arg("--sdl2-controls-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid FCEUX SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "FCEUX helper inspected a different SDL runtime"
    );
    // FCEUX AddJoystick(which) stores directly in jsDev[which].
    ensure!(
        snapshot.devices.len() <= 32
            && snapshot
                .devices
                .iter()
                .enumerate()
                .all(|(index, device)| device.device_index as usize == index),
        "FCEUX needs a complete contiguous native joystick inventory within 32 slots"
    );
    Ok(snapshot)
}

fn routing(mut snapshot: Snapshot) -> Snapshot {
    for device in &mut snapshot.devices {
        device.controls = None;
        device.linux_classic = None;
        device.linux_evdev = None;
        device.sampled_state = None;
    }
    snapshot
}

pub(crate) struct PreparedSession {
    pub(crate) configuration: PreparedMapping,
    pub(crate) runtime_paths: Vec<String>,
    topology: InputTopology,
    initial: Snapshot,
    setup: SavedSetup,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.review(calibrations)?;
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("FCEUX selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "FCEUX requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let mut profiles = Vec::new();
        let mut paths = std::collections::BTreeMap::new();
        let mut runtime_paths = Vec::new();
        for (player, selected) in setup.players.iter().zip(&selected) {
            let path = topology.resolve_runtime_path(
                selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            ensure!(
                !runtime_paths.contains(&path),
                "FCEUX players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            let name = format!("lunchbox-p{}", player.player);
            let guid = captured.device_at_path(&path)?.guid.clone();
            let text = calibrated_profile(
                calibrations
                    .get(&player.controller_id)
                    .context("FCEUX calibration disappeared")?,
                &captured,
                &path,
                &name,
            )?;
            profiles.push((
                Selection {
                    player: player.player,
                    guid,
                    profile: name,
                },
                text,
            ));
            paths.insert(player.player, std::path::PathBuf::from(&path));
            runtime_paths.push(path);
        }
        let configuration =
            PreparedMapping::prepare(&setup.base_directory, profiles, &devices(&initial)?, paths)?;
        let session = Self {
            configuration,
            runtime_paths,
            topology,
            initial,
            setup: setup.clone(),
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        self.configuration.verify(&devices(&fresh)?)?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    pub(crate) fn append_mounts(&self, arguments: &mut Vec<std::ffi::OsString>) -> Result<()> {
        self.configuration
            .append_mounts(arguments, &devices(&self.initial)?)
    }
}

fn devices(snapshot: &Snapshot) -> Result<Vec<Device>> {
    snapshot
        .devices
        .iter()
        .map(|device| {
            Ok(Device {
                slot: device.device_index.try_into()?,
                guid: device.guid.clone(),
                path: device
                    .path
                    .as_ref()
                    .context("FCEUX SDL device has no physical path")?
                    .into(),
                game_controller: device.is_game_controller,
            })
        })
        .collect()
}
