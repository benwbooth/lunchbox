//! Native SDL2 launch-time inventory, calibration ownership and the private
//! HOME/-c configuration pair. The user's own hatari.cfg is never opened.
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
        serde_json::from_slice(&output).context("Invalid Hatari SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "Hatari helper inspected a different SDL runtime"
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

/// Translate one player's calibrated controls into Hatari's fixed vocabulary.
/// Returns the direction bindings per control plus the fire-button slots.
pub(super) fn calibrated_bindings(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
) -> Result<(Vec<(&'static str, Binding)>, Vec<(usize, u32)>)> {
    use lunchbox_controller_probe::{
        duckstation::DigitalInput, linux_classic::AxisEndpoints, sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "Hatari native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let counts = device
        .controls
        .as_ref()
        .context("Hatari SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("Hatari released state is absent")?;
    state.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native Hatari profile")?;
    let mut directions = Vec::new();
    let mut fires = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let control = super::CONTROLS
            .iter()
            .find(|(control, _, _)| *control == row.target_id)
            .with_context(|| {
                format!(
                    "Hatari target {} is outside the joystick contract",
                    row.target_id
                )
            })?;
        let input = row
            .input
            .context("Hatari gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("Hatari requires measured native controls")?;
        let slot = match control.0 {
            "fire" => Some(1),
            "fire2" => Some(2),
            "fire3" => Some(3),
            _ => None,
        };
        if let Some(slot) = slot {
            let button = match physical.control(native.code)? {
                lunchbox_controller_probe::linux_classic::Control::Button(button) => {
                    ensure!(
                        state.buttons.get(&button) == Some(&false),
                        "Release Hatari controller buttons before capture"
                    );
                    button
                }
                _ => anyhow::bail!("Hatari fire slot {slot} needs a physical button"),
            };
            fires.push((slot, button));
            continue;
        }
        let binding = match physical.control(native.code)? {
            lunchbox_controller_probe::linux_classic::Control::Button(_) => anyhow::bail!(
                "Hatari directions are hardcoded to axes 0/1 and hat 0; a button cannot drive {}",
                control.0
            ),
            lunchbox_controller_probe::linux_classic::Control::Axis(axis) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("Hatari axis measurements are absent")?;
                let code = (native.code & 0xffff) as u8;
                let released = physical.axis_value(code, endpoints.released)?;
                let pressed = physical.axis_value(code, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&axis) == Some(&released),
                    "Hatari axis rest differs from calibration"
                );
                ensure!(
                    (-super::AXIS_NEGATIVE_THRESHOLD..=super::AXIS_POSITIVE_THRESHOLD)
                        .contains(&i32::from(released)),
                    "Hatari axis rest would hold a direction pressed; recalibrate the rest position"
                );
                if i32::from(pressed) > super::AXIS_POSITIVE_THRESHOLD {
                    Binding::AxisHalf {
                        axis,
                        negative: false,
                    }
                } else if i32::from(pressed) < super::AXIS_NEGATIVE_THRESHOLD {
                    Binding::AxisHalf {
                        axis,
                        negative: true,
                    }
                } else {
                    anyhow::bail!(
                        "Hatari's fixed ±16384 direction thresholds cannot represent the measured axis travel"
                    );
                }
            }
            lunchbox_controller_probe::linux_classic::Control::HatAxis { index, .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("Hatari hat measurements are absent")?;
                match physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )? {
                    DigitalInput::Hat {
                        index: hat,
                        direction,
                    } => {
                        ensure!(
                            state.hats.get(&hat) == Some(&0),
                            "Release Hatari directional hat before capture"
                        );
                        let _ = index;
                        ensure!(
                            hat == 0,
                            "Hatari only reads hat 0 for directions; hat {hat} cannot drive {}",
                            control.0
                        );
                        Binding::Hat { direction }
                    }
                    _ => anyhow::bail!("Hatari hat did not resolve to a native SDL hat"),
                }
            }
        };
        binding.valid()?;
        directions.push((control.0, binding));
    }
    for (control, _, required) in super::CONTROLS {
        let present = directions.iter().any(|(id, _)| *id == control)
            || fires.iter().any(|(slot, _)| {
                matches!((control, *slot), ("fire", 1) | ("fire2", 2) | ("fire3", 3))
            });
        ensure!(
            !required || present,
            "Hatari needs the required control {control}"
        );
    }
    Ok((directions, fires))
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) config_path: std::path::PathBuf,
    pub(crate) home_path: std::path::PathBuf,
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
        ensure!(
            setup.tos_image.is_file(),
            "Hatari TOS image does not exist: {}",
            setup.tos_image.display()
        );
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("Hatari selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "Hatari requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, cancel)?);
        let mut players = Vec::new();
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
                "Hatari players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            let (_, fires) = calibrated_bindings(
                calibrations
                    .get(&player.controller_id)
                    .context("Hatari calibration disappeared")?,
                &captured,
                &path,
            )?;
            let device = i32::try_from(captured.device_at_path(&path)?.device_index)
                .context("Hatari SDL device index overflows the config field")?;
            players.push((usize::from(player.player), device, fires));
            runtime_paths.push(path);
        }
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-hatari-")
            .tempdir()?;
        let home_path = directory.path().join("home");
        std::fs::create_dir(&home_path)?;
        let config_path = directory.path().join("hatari.cfg");
        let references: Vec<_> = players
            .iter()
            .map(|(port, device, fires)| (*port, *device, fires.as_slice()))
            .collect();
        std::fs::write(&config_path, super::config(&references, &setup.tos_image)?)?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [
            &setup.probe_program,
            &setup.sdl_library,
            &setup.content,
            &setup.tos_image,
        ] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(config_path.clone(), file_hash(&config_path)?);
        let session = Self {
            directory,
            config_path,
            home_path,
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
            ensure!(file_hash(path)? == *expected, "Hatari launch input changed");
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    /// Launch arguments pass the additional configuration ahead of the game.
    pub(crate) fn overlay_arguments(
        &self,
        arguments: &[std::ffi::OsString],
    ) -> Result<Vec<std::ffi::OsString>> {
        ensure!(
            self.config_path.is_absolute(),
            "Hatari private configuration path must be absolute"
        );
        let mut result = Vec::with_capacity(arguments.len() + 2);
        result.push("-c".into());
        result.push(self.config_path.as_os_str().to_owned());
        result.extend_from_slice(arguments);
        Ok(result)
    }
}
