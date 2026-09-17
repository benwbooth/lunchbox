//! Native SDL launch-time inventory, calibration ownership and the private
//! settings.bml owner. The user's own settings file is never read or written.
use super::{Binding, Gamepad, settings};
#[cfg(target_os = "linux")]
use crate::controller_bizhawk_guard::InputTopology;
#[cfg(not(target_os = "linux"))]
use crate::controller_native_platform as platform;
use crate::{
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
    // `LD_LIBRARY_PATH` for the probe when its SDL needs sibling runtime
    // libraries (sdl2-compat's SDL3). Native host SDL resolves without it.
    probe_library_path: Option<&std::ffi::OsStr>,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    if let Some(library_path) = probe_library_path {
        command.env("LD_LIBRARY_PATH", library_path);
    }
    command
        .arg("--sdl2-inventory")
        .arg("--sdl-library")
        .arg(&setup.sdl_library);
    if let Some(path) = path {
        command.arg("--sdl2-controls-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid bsnes SDL capture")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "bsnes helper inspected a different SDL runtime"
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

/// Translate one player's resolved raw SDL2 controls into bsnes assignments.
pub(super) fn calibrated_gamepad(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
) -> Result<Gamepad> {
    use lunchbox_controller_probe::{
        linux_classic::{AxisEndpoints, Control},
        sdl2_physical::PhysicalMap,
    };
    ensure!(
        ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
        "bsnes native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let index = device.device_index;
    let counts = device
        .controls
        .as_ref()
        .context("bsnes SDL counts are absent")?;
    let state = device
        .sampled_state
        .as_ref()
        .context("bsnes released state is absent")?;
    state.validate(counts)?;
    let physical = PhysicalMap::from_device(device)?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native bsnes profile")?;
    let mut controls: std::collections::BTreeMap<&'static str, Binding> =
        std::collections::BTreeMap::new();
    for row in calibration.plan_profile(profile)?.rows {
        let input = row
            .input
            .context("bsnes gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("bsnes requires measured native controls")?;
        let binding = match physical.control(native.code)? {
            Control::Button(button) => {
                ensure!(
                    state.buttons.get(&button) == Some(&false),
                    "Release bsnes controller buttons before capture"
                );
                ensure!(
                    u64::from(button) < u64::from(counts.buttons),
                    "bsnes button exceeds the observed SDL count"
                );
                Binding::Button(button)
            }
            Control::Axis(axis) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("bsnes axis measurements are absent")?;
                let code = (native.code & 0xffff) as u8;
                let released = physical.axis_value(code, endpoints.released)?;
                let pressed = physical.axis_value(code, endpoints.pressed)?;
                ensure!(
                    state.axes.get(&axis) == Some(&released),
                    "bsnes axis rest differs from calibration"
                );
                ensure!(
                    (-super::DIGITAL_THRESHOLD..super::DIGITAL_THRESHOLD)
                        .contains(&i32::from(released)),
                    "bsnes axis rest would hold a digital direction pressed; recalibrate the rest position"
                );
                if i32::from(pressed) > super::DIGITAL_THRESHOLD {
                    Binding::AxisHi(axis)
                } else if i32::from(pressed) < -super::DIGITAL_THRESHOLD {
                    Binding::AxisLo(axis)
                } else {
                    anyhow::bail!(
                        "bsnes fixed ±16384 digital threshold cannot represent the measured axis travel"
                    );
                }
            }
            Control::HatAxis { .. } => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("bsnes hat measurements are absent")?;
                let digital = physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: endpoints.released,
                        pressed: endpoints.pressed,
                    }),
                )?;
                match digital {
                    lunchbox_controller_probe::duckstation::DigitalInput::Hat {
                        index: hat,
                        direction,
                    } => {
                        ensure!(
                            state.hats.get(&hat) == Some(&0),
                            "Release bsnes directional hat before capture"
                        );
                        // bsnes hat inputs are 2*hat+0 for X and 2*hat+1 for Y;
                        // Left/Up are the Lo half, Right/Down the Hi half.
                        const LEFT: u8 = 8;
                        const UP: u8 = 1;
                        let negative_half = direction & (LEFT | UP) != 0;
                        let input = hat
                            .checked_mul(2)
                            .context("bsnes hat index overflows the input field")?
                            + u32::from(!negative_half);
                        if negative_half {
                            Binding::HatLo(input)
                        } else {
                            Binding::HatHi(input)
                        }
                    }
                    _ => anyhow::bail!("bsnes hat did not resolve to a native SDL hat"),
                }
            }
        };
        let control = super::CONTROLS
            .iter()
            .find(|(control, _)| *control == row.target_id)
            .with_context(|| {
                format!(
                    "bsnes target {} is outside the gamepad contract",
                    row.target_id
                )
            })?
            .0;
        ensure!(
            controls.insert(control, binding).is_none(),
            "Duplicate bsnes gameplay target"
        );
    }
    Gamepad::build(index, &controls)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) settings_path: std::path::PathBuf,
    runtime_paths: Vec<String>,
    device_indices: Vec<u32>,
    #[cfg(target_os = "linux")]
    topology: InputTopology,
    initial: Snapshot,
    setup: settings::SavedSetup,
    hashes: std::collections::BTreeMap<std::path::PathBuf, String>,
    probe_library_path: Option<std::ffi::OsString>,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        probe_library_path: Option<&std::ffi::OsStr>,
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
                .context("bsnes selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "bsnes requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        #[cfg(target_os = "linux")]
        let topology = InputTopology::capture(&selected)?;
        let initial = routing(observe(setup, None, probe_library_path, cancel)?);
        let mut players = Vec::new();
        let mut runtime_paths = Vec::new();
        let mut device_indices = Vec::new();
        for (player, selected) in setup.players.iter().zip(&selected) {
            // Linux resolves through the sysfs topology; other hosts
            // match the SDL device-interface path and require uniqueness.
            #[cfg(target_os = "linux")]
            let path = topology.resolve_runtime_path(
                selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            #[cfg(not(target_os = "linux"))]
            let path = {
                let selected_string = selected.to_string_lossy().into_owned();
                let candidates = initial
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(selected_string.as_str()))
                    .collect::<Vec<_>>();
                ensure!(
                    candidates.len() == 1,
                    "bsnes physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            ensure!(
                !runtime_paths.contains(&path),
                "bsnes players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), probe_library_path, cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device_index = captured.device_at_path(&path)?.device_index;
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(&captured.devices, &path, device_index)?;
            let gamepad = calibrated_gamepad(
                calibrations
                    .get(&player.controller_id)
                    .context("bsnes calibration disappeared")?,
                &captured,
                &path,
            )?;
            runtime_paths.push(path);
            device_indices.push(device_index);
            players.push(Some(gamepad));
        }
        while players.len() < 2 {
            players.push(None);
        }
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-bsnes-settings-")
            .tempdir()?;
        let settings_path = directory.path().join("settings.bml");
        std::fs::write(&settings_path, super::settings_bml(&players))?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [&setup.probe_program, &setup.sdl_library, &setup.content] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(settings_path.clone(), file_hash(&settings_path)?);
        let session = Self {
            directory,
            settings_path,
            runtime_paths,
            device_indices,
            #[cfg(target_os = "linux")]
            topology,
            initial,
            setup: setup.clone(),
            hashes,
            probe_library_path: probe_library_path.map(|path| path.to_owned()),
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        #[cfg(target_os = "linux")]
        self.topology.verify()?;
        for (path, expected) in &self.hashes {
            ensure!(file_hash(path)? == *expected, "bsnes launch input changed");
        }
        let fresh = routing(observe(
            &self.setup,
            None,
            self.probe_library_path.as_deref(),
            cancel,
        )?);
        self.initial.ensure_same_routing(&fresh)?;
        for (path, index) in self.runtime_paths.iter().zip(&self.device_indices) {
            let device = fresh.device_at_path(path)?;
            ensure!(
                device.device_index == *index,
                "bsnes SDL joystick moved before launch"
            );
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(&fresh.devices, path, *index)?;
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
        let fresh = routing(observe(
            &self.setup,
            None,
            self.probe_library_path.as_deref(),
            &AtomicBool::new(false),
        )?);
        self.initial.ensure_same_routing(&fresh)?;
        for (path, index) in self.runtime_paths.iter().zip(&self.device_indices) {
            platform::require_unique_device_path(&fresh.devices, path, *index)?;
        }
        Ok(())
    }

    /// Insert `--settings=` ahead of the game argument so bsnes reads only the
    /// private file. The user's own configuration is never selected.
    pub(crate) fn overlay_arguments(
        &self,
        arguments: &[std::ffi::OsString],
    ) -> Result<Vec<std::ffi::OsString>> {
        ensure!(
            self.settings_path.is_absolute(),
            "bsnes private settings path must be absolute"
        );
        let mut result = Vec::with_capacity(arguments.len() + 1);
        result.push(format!("--settings={}", self.settings_path.display()).into());
        result.extend_from_slice(arguments);
        Ok(result)
    }
}
