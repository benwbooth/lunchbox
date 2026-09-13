//! Calibrated source capture, fixed target pads and private puNES ownership.

use super::{
    configuration,
    flatpak::PreparedFlatpak,
    isolation::PreparedConfig,
    settings::{MAPPING_PROFILE, SavedSetup},
    virtual_pad::Bridge,
};
use crate::{
    controller_catalog::{Calibration, catalog},
    controller_native_process::cancelled,
    controllers::ControllerDevice,
    emulator::LaunchPlan,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::punes_supervisor::Inventory;
use std::{collections::HashMap, path::Path, sync::atomic::AtomicBool};

fn measured_event_path<'a>(device: &'a ControllerDevice) -> Result<(&'a Path, std::path::PathBuf)> {
    let identity = |path: &Path, prefix: &str| -> Result<std::path::PathBuf> {
        ensure!(
            path.parent() == Some(Path::new("/dev/input")),
            "Expected a physical input node"
        );
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Input node name is not UTF-8")?;
        ensure!(
            name.strip_prefix(prefix).is_some_and(|index| {
                !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
            }),
            "Unexpected physical input node"
        );
        Ok(Path::new("/sys/class/input")
            .join(name)
            .join("device")
            .canonicalize()?)
    };
    let expected = identity(&device.device_path, "js")?;
    let mut selected = None;
    for event in &device.event_paths {
        if identity(event, "event")? == expected {
            ensure!(
                selected.is_none(),
                "Ambiguous physical event node for puNES controller"
            );
            selected = Some(event.as_path());
        }
    }
    Ok((
        selected.context("No exact evdev node for the selected puNES controller")?,
        expected,
    ))
}

pub(crate) struct PreparedSession {
    pub(crate) configuration: PreparedConfig,
    bridges: Vec<Bridge>,
    initial: Inventory,
    setup: SavedSetup,
    runtime: PreparedFlatpak,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        devices: &[ControllerDevice],
        runtime: PreparedFlatpak,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.review(calibrations)?;
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == MAPPING_PROFILE)
            .context("Missing puNES standard-pad mapping profile")?;
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matches = devices
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("puNES selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "puNES requires an unambiguous physical controller"
            );
            selected.push(device);
        }
        let mut bridges = Vec::new();
        for (player, device) in setup.players.iter().zip(&selected) {
            let calibration = calibrations
                .get(&player.controller_id)
                .context("puNES controller calibration disappeared")?;
            let (event, identity) = measured_event_path(device)?;
            crate::controller_axis::validate_recorded_axes(
                event,
                calibration.bindings.values().filter_map(|input| {
                    input
                        .native
                        .as_ref()
                        .zip(input.axis.as_ref())
                        .map(|(native, axis)| (native.code, axis))
                }),
            )?;
            bridges.push(Bridge::start(
                player.player,
                calibration,
                profile,
                event,
                &identity,
                cancel,
            )?);
            ensure!(
                measured_event_path(device)?.1 == identity,
                "puNES physical controller changed during bridge startup"
            );
        }
        let initial = runtime.observe(cancel)?;
        initial.validate(bridges.len())?;
        for (index, bridge) in bridges.iter().enumerate() {
            let player = u8::try_from(index + 1)?;
            let target = initial.device_for_player(player)?;
            ensure!(
                target.path == bridge.event_path()
                    && target.guid == bridge.guid()
                    && target.name == bridge.name(),
                "puNES target inventory differs from its owned bridge"
            );
        }
        let pads = bridges
            .iter()
            .enumerate()
            .map(|(index, bridge)| configuration::Pad {
                player: u8::try_from(index + 1).unwrap(),
                guid: bridge.guid().to_owned(),
            })
            .collect();
        let configuration = PreparedConfig::prepare(setup, pads)?;
        let session = Self {
            configuration,
            bridges,
            initial,
            setup: setup.clone(),
            runtime,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn stage_launch(
        &mut self,
        original: &LaunchPlan,
    ) -> Result<Vec<std::ffi::OsString>> {
        self.runtime
            .prepare_launch(&self.setup, &self.configuration, &self.initial, original)
    }

    pub(crate) fn receipt_ready(&self) -> Result<bool> {
        self.runtime.receipt_ready()
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.configuration.verify()?;
        for bridge in &self.bridges {
            bridge.check_health()?;
        }
        self.runtime.verify(cancel)?;
        ensure!(
            self.runtime.observe(cancel)? == self.initial,
            "puNES target routing changed"
        );
        Ok(())
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        for bridge in &self.bridges {
            bridge.check_health()?;
        }
        Ok(())
    }
}
