//! Exact raw-SDL2 preparation for native Kronos.
#[cfg(target_os = "linux")]
use crate::controller_bizhawk_guard::InputTopology;
use crate::controller_catalog::{Calibration, InputBinding};
#[cfg(not(target_os = "linux"))]
use crate::controller_native_platform as platform;
use crate::controller_native_process::{cancelled, capture};
use crate::controllers::ControllerDevice;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    duckstation::DigitalInput, file_hash, linux_classic::AxisEndpoints, sdl2::Snapshot,
    sdl2_physical::PhysicalMap,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    process::Command,
    sync::atomic::AtomicBool,
};

fn observe(
    setup: &super::settings::SavedSetup,
    path: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
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
        serde_json::from_slice(&output).context("Invalid Kronos SDL2 capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "Kronos helper inspected a different SDL2 runtime"
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

fn host_code(physical: &PhysicalMap<'_>, device_index: u32, input: &InputBinding) -> Result<u32> {
    let native = input
        .native
        .as_ref()
        .context("Kronos needs native physical controls")?;
    let endpoints = input.axis.as_ref().map(|axis| AxisEndpoints {
        released: axis.released,
        pressed: axis.pressed,
    });
    let encoded = match physical.digital_input(native.code, endpoints)? {
        DigitalInput::Button(index) => {
            crate::controller_kronos::raw_button_code(device_index, u16::try_from(index)?)
        }
        DigitalInput::Axis {
            index,
            released,
            pressed,
        } => {
            ensure!(
                released.abs() < 10_000 && pressed.abs() > 10_000,
                "Kronos raw axis does not leave a centered rest"
            );
            crate::controller_kronos::raw_axis_code(
                device_index,
                u16::try_from(index)?,
                pressed > 0,
            )
        }
        DigitalInput::Hat { index, direction } => {
            crate::controller_kronos::raw_hat_code(device_index, u16::try_from(index)?, direction)
        }
    };
    encoded.map_err(|()| anyhow::anyhow!("Kronos input is outside its raw SDL encoding"))
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedPlayer {
    pub(crate) player: u8,
    pub(crate) runtime_path: String,
    pub(crate) device_index: u32,
}

pub(crate) struct PreparedSession {
    pub(crate) directory: tempfile::TempDir,
    pub(crate) config_path: std::path::PathBuf,
    pub(crate) players: Vec<PreparedPlayer>,
    #[cfg(target_os = "linux")]
    pub(crate) topology: InputTopology,
    initial: Snapshot,
    setup: super::settings::SavedSetup,
    hashes: BTreeMap<std::path::PathBuf, String>,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &super::settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.validate()?;
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches.next().with_context(|| {
                format!("Kronos player {} controller is disconnected", player.player)
            })?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "Kronos requires unambiguous physical controllers"
            );
            selected.push(device.device_path.clone());
        }
        #[cfg(target_os = "linux")]
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == crate::controller_kronos::PROFILE_ID)
            .context("Missing native Kronos profile")?;
        let mut prepared = Vec::new();
        let mut pads = Vec::new();
        let mut indices = BTreeSet::new();
        for (player, selected_path) in setup.players.iter().zip(selected.iter()) {
            // Linux resolves through the sysfs topology; other hosts
            // match the SDL device-interface path and require uniqueness.
            #[cfg(target_os = "linux")]
            let runtime_path = topology
                .resolve_runtime_path(
                    selected_path,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )
                .with_context(|| {
                    format!("Kronos player {} device not found in SDL2", player.player)
                })?;
            #[cfg(not(target_os = "linux"))]
            let runtime_path = {
                let selected_string = selected_path.to_string_lossy().into_owned();
                let candidates = initial
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(selected_string.as_str()))
                    .collect::<Vec<_>>();
                ensure!(
                    candidates.len() == 1,
                    "Kronos player {} controller is missing or ambiguous in SDL",
                    player.player
                );
                selected_string
            };
            let captured = observe(setup, Some(&runtime_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&runtime_path)?;
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(
                &captured.devices,
                &runtime_path,
                device.device_index,
            )?;
            ensure!(
                device.device_index < crate::controller_kronos::PERSDL_MAX_DEVICES,
                "Kronos raw SDL codes address only the first four enumerated devices"
            );
            ensure!(
                indices.insert(device.device_index),
                "Kronos SDL2 device index is duplicated"
            );
            let physical = PhysicalMap::from_device(device)?;
            let calibration = calibrations.get(&player.controller_id).with_context(|| {
                format!("Kronos player {} calibration disappeared", player.player)
            })?;
            let plan = calibration.plan_profile(profile)?;
            let mut bindings = Vec::new();
            let mut keys = BTreeSet::new();
            for row in plan.rows {
                let key = crate::controller_kronos::pad_key_for_target(&row.target_id)
                    .with_context(|| {
                        format!("Kronos target {} is outside its contract", row.target_id)
                    })?;
                ensure!(keys.insert(key), "Kronos pad key {key} appears twice");
                let input = row
                    .input
                    .as_ref()
                    .context("Kronos control is not calibrated")?;
                bindings.push((key, host_code(&physical, device.device_index, input)?));
            }
            ensure!(
                keys.len() == crate::controller_kronos::PAD_BUTTONS.len(),
                "Kronos standard pad mapping is incomplete"
            );
            pads.push(crate::controller_kronos::PadBinding {
                port: player.port,
                id: player.device_id,
                bindings,
            });
            prepared.push(PreparedPlayer {
                player: player.player,
                runtime_path,
                device_index: device.device_index,
            });
        }
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-kronos-")
            .tempdir()?;
        let config_path = directory.path().join("kronos.ini");
        let baseline = std::fs::read(&setup.config_path)
            .context("Reading the declared Kronos configuration")?;
        std::fs::write(
            &config_path,
            crate::controller_kronos::patch_ini(&baseline, &pads)?,
        )?;
        let mut hashes = BTreeMap::new();
        for path in [
            &setup.probe_program,
            &setup.sdl_library,
            &setup.content,
            &setup.config_path,
            &config_path,
        ] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        let session = Self {
            directory,
            config_path,
            players: prepared,
            #[cfg(target_os = "linux")]
            topology,
            initial,
            setup: setup.clone(),
            hashes,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        #[cfg(target_os = "linux")]
        self.topology.verify()?;
        for (path, expected) in &self.hashes {
            ensure!(file_hash(path)? == *expected, "Kronos launch input changed");
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        for player in &self.players {
            ensure!(
                fresh.device_at_path(&player.runtime_path)?.device_index == player.device_index,
                "Kronos player {} SDL2 routing changed",
                player.player
            );
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(
                &fresh.devices,
                &player.runtime_path,
                player.device_index,
            )?;
        }
        #[cfg(target_os = "linux")]
        {
            return self.topology.verify();
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Ok(());
        }
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        return self.topology.verify();
        #[cfg(not(target_os = "linux"))]
        return self.verify_health_probe();
    }

    #[cfg(not(target_os = "linux"))]
    fn verify_health_probe(&self) -> Result<()> {
        let fresh = routing(observe(&self.setup, None, &AtomicBool::new(false))?);
        self.initial.ensure_same_routing(&fresh)?;
        for player in &self.players {
            platform::require_unique_device_path(
                &fresh.devices,
                &player.runtime_path,
                player.device_index,
            )?;
        }
        Ok(())
    }
}
