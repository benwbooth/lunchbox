//! Native SDL launch-time SDL inventory and physical calibration ownership.
use super::{isolation::PreparedConfig, physical::calibrated_bindings, settings::SavedSetup};
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
        serde_json::from_slice(&output).context("Invalid SameBoy SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "SameBoy helper inspected a different SDL runtime"
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
    pub(crate) configuration: PreparedConfig,
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
        abi: super::configuration::Abi,
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
                .context("SameBoy selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "SameBoy requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let mut bindings = None;
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
                "SameBoy players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            super::routing::validate(&captured, &path)?;
            bindings = Some(calibrated_bindings(
                calibrations
                    .get(&player.controller_id)
                    .context("SameBoy calibration disappeared")?,
                &captured,
                &path,
            )?);
            runtime_paths.push(path);
        }
        let configuration = PreparedConfig::prepare(
            &setup.source_config,
            &{
                let bindings = bindings.context("SameBoy mapping absent")?;
                validate_axis_content(&setup.content, &bindings)?;
                bindings
            },
            abi,
        )?;
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
        self.configuration.verify()?;
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        super::routing::validate(&fresh, &self.runtime_paths[0])?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}

/// Core/gb.c routes axes to acceleration iff the cartridge uses MBC7;
/// Core/mbc.c selects MBC7 for header type 0x22. Do not silently report those
/// axes as directional controls. Button/hat gameplay bindings remain usable.
fn validate_axis_content(path: &std::path::Path, bindings: &super::Bindings) -> Result<()> {
    use std::io::{Read, Seek, SeekFrom};
    if bindings.axes.iter().all(|axis| *axis == 255) {
        return Ok(());
    }
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(0x147))?;
    let mut cartridge_type = [0];
    file.read_exact(&mut cartridge_type)
        .context("SameBoy needs a complete raw Game Boy ROM header for axis routing")?;
    ensure!(
        cartridge_type[0] != 0x22,
        "SameBoy MBC7 games route joystick axes to tilt, not directions; use button/hat directions until a separate tilt mapping is configured"
    );
    Ok(())
}
