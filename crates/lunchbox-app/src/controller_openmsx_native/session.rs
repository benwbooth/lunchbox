//! Native SDL2 launch-time inventory, calibration ownership and the private
//! OPENMSX_HOME/-setting pair. The user's own settings.xml is never opened.
use super::{Binding, settings};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{file_hash, sdl2::Snapshot};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(
    setup: &settings::SavedSetup,
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
        serde_json::from_slice(&output).context("Invalid openMSX SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "openMSX helper inspected a different SDL runtime"
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

/// Translate one player's calibrated controls into dict bindings.
pub(super) fn calibrated_bindings(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
) -> Result<Vec<(String, Binding)>> {
    use lunchbox_controller_probe::{
        duckstation::DigitalInput, linux_classic::AxisEndpoints, sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "openMSX native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let counts = device
        .controls
        .as_ref()
        .context("openMSX SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("openMSX released state is absent")?;
    state.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native openMSX profile")?;
    let mut bindings = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let key = super::CONTROLS
            .iter()
            .find(|(control, _, _)| *control == row.target_id)
            .map(|(_, key, _)| (*key).to_owned())
            .with_context(|| {
                format!(
                    "openMSX target {} is outside the joystick contract",
                    row.target_id
                )
            })?;
        let input = row
            .input
            .context("openMSX gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("openMSX requires measured native controls")?;
        let binding = match physical.control(native.code)? {
            lunchbox_controller_probe::linux_classic::Control::Button(button) => {
                ensure!(
                    state.buttons.get(&button) == Some(&false),
                    "Release openMSX controller buttons before capture"
                );
                Binding::Button(button)
            }
            lunchbox_controller_probe::linux_classic::Control::Axis(axis) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("openMSX axis measurements are absent")?;
                let code = (native.code & 0xffff) as u8;
                let released = physical.axis_value(code, endpoints.released)?;
                let pressed = physical.axis_value(code, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&axis) == Some(&released),
                    "openMSX axis rest differs from calibration"
                );
                ensure!(
                    (-super::DIGITAL_THRESHOLD..super::DIGITAL_THRESHOLD)
                        .contains(&i32::from(released)),
                    "openMSX axis rest would hold a direction pressed; recalibrate the rest position"
                );
                if i32::from(pressed) > super::DIGITAL_THRESHOLD {
                    Binding::Axis {
                        index: axis,
                        negative: false,
                    }
                } else if i32::from(pressed) < -super::DIGITAL_THRESHOLD {
                    Binding::Axis {
                        index: axis,
                        negative: true,
                    }
                } else {
                    anyhow::bail!(
                        "openMSX's default 25% dead zone cannot represent the measured axis travel"
                    );
                }
            }
            lunchbox_controller_probe::linux_classic::Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("openMSX hat measurements are absent")?;
                match physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )? {
                    DigitalInput::Hat { index, direction } => {
                        ensure!(
                            state.hats.get(&index) == Some(&0),
                            "Release openMSX directional hat before capture"
                        );
                        Binding::Hat { index, direction }
                    }
                    _ => anyhow::bail!("openMSX hat did not resolve to a native SDL hat"),
                }
            }
        };
        bindings.push((key, binding));
    }
    Ok(bindings)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) settings_path: std::path::PathBuf,
    pub(crate) home_path: std::path::PathBuf,
    pub(crate) second_port_command: Option<String>,
    runtime_paths: Vec<String>,
    topology: InputTopology,
    initial: Snapshot,
    setup: settings::SavedSetup,
    hashes: std::collections::BTreeMap<std::path::PathBuf, String>,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
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
                .context("openMSX selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "openMSX requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let mut dicts = Vec::new();
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
                "openMSX players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            let bindings = calibrated_bindings(
                calibrations
                    .get(&player.controller_id)
                    .context("openMSX calibration disappeared")?,
                &captured,
                &path,
            )?;
            // joyN numbers the host joysticks from one in enumeration order.
            let joystick = captured.device_at_path(&path)?.device_index + 1;
            dicts.push((
                usize::from(player.player),
                super::dict(&bindings, joystick)?,
            ));
            runtime_paths.push(path);
        }
        let references: Vec<_> = dicts.iter().map(|(p, d)| (*p, d.as_str())).collect();
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-openmsx-")
            .tempdir()?;
        let home_path = directory.path().join("home");
        std::fs::create_dir(&home_path)?;
        let settings_path = directory.path().join("settings.xml");
        std::fs::write(&settings_path, super::settings_xml(&references))?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [&setup.probe_program, &setup.sdl_library, &setup.content] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(settings_path.clone(), file_hash(&settings_path)?);
        let session = Self {
            directory,
            settings_path,
            home_path,
            second_port_command: if setup.players.len() > 1 {
                Some("plug joyportb msxjoystick2".into())
            } else {
                None
            },
            runtime_paths,
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
        self.topology.verify()?;
        for (path, expected) in &self.hashes {
            ensure!(
                file_hash(path)? == *expected,
                "openMSX launch input changed"
            );
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    /// Launch arguments select the private settings file (and the second
    /// joystick port) ahead of the game.
    pub(crate) fn overlay_arguments(
        &self,
        arguments: &[std::ffi::OsString],
    ) -> Result<Vec<std::ffi::OsString>> {
        ensure!(
            self.settings_path.is_absolute(),
            "openMSX private settings path must be absolute"
        );
        let mut result = Vec::with_capacity(arguments.len() + 2);
        result.push("-setting".into());
        result.push(self.settings_path.as_os_str().to_owned());
        if let Some(command) = &self.second_port_command {
            result.push("-command".into());
            result.push(command.into());
        }
        result.extend_from_slice(arguments);
        Ok(result)
    }
}
