//! Native SDL2 launch-time inventory, calibration ownership and the private
//! ini file. The user's own scummvm.ini is never opened.
use super::{Raw, settings};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{Snapshot, file_hash};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(
    setup: &settings::SavedSetup,
    path: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command.arg("--sdl-library").arg(&setup.sdl_library);
    if let Some(path) = path {
        command.arg("--bindings-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid ScummVM SDL capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "ScummVM helper inspected a different SDL runtime"
    );
    Ok(snapshot)
}

fn routing(mut snapshot: Snapshot) -> Snapshot {
    for device in &mut snapshot.devices {
        device.resolved = None;
        device.linux_classic = None;
    }
    snapshot
}

/// Translate one control's raw backing into the matching SDL standard
/// field from the mapping string.
fn standard_backing(fields: &std::collections::BTreeMap<String, Raw>, raw: &Raw) -> Option<String> {
    fields
        .iter()
        .find(|(_, backing)| *backing == raw)
        .map(|(standard, _)| standard.clone())
}

/// Translate the calibrated controls into (action, hardware input id)
/// bindings for the engine-default keymap.
pub(super) fn calibrated_bindings(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
) -> Result<Vec<(String, String)>> {
    use lunchbox_controller_probe::{
        linux_classic::{AxisEndpoints, Control},
        sdl2_physical::PhysicalMap,
    };
    ensure!(
        calibration.os == "linux",
        "ScummVM native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    ensure!(
        snapshot
            .devices
            .first()
            .is_some_and(|first| first.path.as_deref() == device.path.as_deref()),
        "ScummVM consumes SDL device zero (joystick_num); the selected controller must be the first device"
    );
    let mapping = device
        .mapping
        .clone()
        .context("ScummVM device has no SDL gamecontroller mapping")?;
    let fields = super::parse_mapping(&mapping)?;
    let classic = device.linux_classic.as_ref().context(
        "ScummVM physical translation is absent; the classic backend did not capture it",
    )?;
    let physical = PhysicalMap::Classic(classic);
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native ScummVM profile")?;
    let mut bindings = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let action = super::CONTROLS
            .iter()
            .find(|(control, _)| *control == row.target_id)
            .map(|(_, action)| (*action).to_owned())
            .with_context(|| {
                format!(
                    "ScummVM target {} is outside the action contract",
                    row.target_id
                )
            })?;
        let input = row
            .input
            .context("ScummVM action control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("ScummVM requires measured native controls")?;
        let raw = match physical.control(native.code)? {
            Control::Button(button) => Raw::Button(button),
            Control::Axis(axis) => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("ScummVM axis measurements are absent")?;
                let code = (native.code & 0xffff) as u8;
                let released = physical.axis_value(code, endpoints.released)?;
                let pressed = physical.axis_value(code, endpoints.pressed)?;
                ensure!(
                    released != pressed,
                    "ScummVM axis gesture is lost in the kernel dead zone"
                );
                Raw::Axis {
                    index: axis,
                    positive: pressed > released,
                }
            }
            Control::HatAxis { index, horizontal } => {
                let direction = input
                    .axis
                    .as_ref()
                    .map(|axis| axis.pressed.saturating_sub(axis.released).signum())
                    .unwrap_or(1);
                let mask: u8 = match (horizontal, direction) {
                    (true, -1) => 8,
                    (true, _) => 2,
                    (false, -1) => 1,
                    (false, _) => 4,
                };
                Raw::Hat { hat: index, mask }
            }
        };
        let standard = standard_backing(&fields, &raw).with_context(|| {
            format!("ScummVM calibration input does not back any standard gamepad field")
        })?;
        let hw = super::joy_id(&standard)
            .with_context(|| format!("ScummVM has no joystick id for standard field {standard}"))?;
        bindings.push((action, hw.to_owned()));
    }
    Ok(bindings)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    pub(crate) config_root: std::path::PathBuf,
    pub(crate) config_path: std::path::PathBuf,
    pub(crate) target: String,
    runtime_path: String,
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
            setup.content.is_dir(),
            "ScummVM content must be the game directory: {}",
            setup.content.display()
        );
        let mut matches = inventory
            .iter()
            .filter(|device| device.stable_id == setup.controller_id);
        let device = matches
            .next()
            .context("ScummVM selected controller is disconnected")?;
        ensure!(
            matches.next().is_none() && !device.is_virtual,
            "ScummVM requires an unambiguous physical controller"
        );
        let selected = device.device_path.clone();
        let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
        let initial = routing(observe(setup, None, cancel)?);
        let runtime_path = topology.resolve_runtime_path(
            &selected,
            initial
                .devices
                .iter()
                .filter_map(|device| device.path.as_deref()),
        )?;
        let captured = observe(setup, Some(&runtime_path), cancel)?;
        let bindings = calibrated_bindings(
            calibrations
                .get(&setup.controller_id)
                .context("ScummVM calibration disappeared")?,
            &captured,
            &runtime_path,
        )?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-scummvm-")
            .tempdir()?;
        let config_root = directory.path().join("config");
        std::fs::create_dir_all(config_root.join("scummvm"))?;
        let config_path = config_root.join("scummvm").join("scummvm.ini");
        let target = "lunchbox-game".to_owned();
        std::fs::write(
            &config_path,
            format!(
                "{}{}",
                super::app_section(),
                super::target_section(&target, &setup.content, &bindings)?
            ),
        )?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [&setup.probe_program, &setup.sdl_library, &setup.content] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(config_path.clone(), file_hash(&config_path)?);
        let session = Self {
            directory,
            config_root,
            config_path,
            target,
            runtime_path,
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
                "ScummVM launch input changed"
            );
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        ensure!(
            fresh
                .devices
                .iter()
                .any(|device| device.path.as_deref() == Some(self.runtime_path.as_str())),
            "ScummVM controller disappeared"
        );
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    /// Launch arguments select the private ini and the prepared target.
    pub(crate) fn overlay_arguments(
        &self,
        arguments: &[std::ffi::OsString],
    ) -> Result<Vec<std::ffi::OsString>> {
        ensure!(
            self.config_path.is_absolute(),
            "ScummVM private configuration path must be absolute"
        );
        let mut result = Vec::with_capacity(arguments.len() + 2);
        result.push("-c".into());
        result.push(self.config_path.as_os_str().to_owned());
        // The game comes from the private target, not a bare argument.
        let _ = arguments;
        result.push(self.target.clone().into());
        Ok(result)
    }
}
