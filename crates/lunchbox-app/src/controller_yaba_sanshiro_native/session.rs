//! Native SDL2 probe for Yaba Sanshiro 2: enumerates the trusted runtime,
//! disambiguates the saved controller, and translates its calibration into
//! `yabause.ini` host key codes for a private-HOME config.
use crate::controller_bizhawk_guard::InputTopology;
use crate::controller_catalog::Calibration;
use crate::controller_native_process::{cancelled, capture};
use crate::controllers::ControllerDevice;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{Snapshot, file_hash};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(
    setup: &crate::controller_yaba_sanshiro_native::settings::SavedSetup,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command.arg("--sdl-library").arg(&setup.sdl_library);
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid Yaba Sanshiro 2 SDL capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "Yaba Sanshiro 2 helper inspected a different SDL runtime"
    );
    Ok(snapshot)
}

pub(crate) struct PreparedSession {
    pub(crate) directory: tempfile::TempDir,
    pub(crate) config_path: std::path::PathBuf,
    pub(crate) runtime_path: String,
    pub(crate) device_index: u32,
    pub(crate) topology: InputTopology,
    setup: crate::controller_yaba_sanshiro_native::settings::SavedSetup,
    hashes: std::collections::BTreeMap<std::path::PathBuf, String>,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &crate::controller_yaba_sanshiro_native::settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.validate().map_err(|e| anyhow::anyhow!("{e:#}"))?;
        let mut matches = inventory
            .iter()
            .filter(|d| d.stable_id == setup.controller_id);
        let device = matches
            .next()
            .context("Yaba Sanshiro 2 selected controller is disconnected")?;
        ensure!(
            matches.next().is_none() && !device.is_virtual,
            "Yaba Sanshiro 2 requires an unambiguous physical controller"
        );
        let selected = device.device_path.clone();
        let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
        let snapshot = observe(setup, cancel)?;
        let runtime_path = topology
            .resolve_runtime_path(
                &selected,
                snapshot.devices.iter().filter_map(|d| d.path.as_deref()),
            )
            .context("Yaba Sanshiro 2 device not found in SDL enumeration")?;
        let dev = snapshot
            .devices
            .iter()
            .find(|d| d.path.as_deref() == Some(runtime_path.as_str()))
            .context("Yaba Sanshiro 2 device not found")?;
        let is_gamepad = dev.is_gamepad;
        let device_index = snapshot
            .devices
            .iter()
            .position(|d| d.path.as_deref() == Some(runtime_path.as_str()))
            .context("Yaba Sanshiro 2 device position not found")?
            as u32;
        ensure!(
            device_index < super::super::controller_yaba_sanshiro::PERSDL_MAX_DEVICES,
            "Yaba Sanshiro 2 host codes address at most four devices"
        );
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == "yaba-sanshiro:standalone-saturn-digital")
            .context("Missing Yaba Sanshiro 2 profile")?;
        let calibration = calibrations
            .get(&setup.controller_id)
            .context("Yaba Sanshiro 2 calibration disappeared")?;
        let plan = calibration.plan_profile(profile)?;
        let mut bindings = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for row in plan.rows {
            let pad_key =
                super::super::controller_yaba_sanshiro::pad_key_for_target(&row.target_id)
                    .with_context(|| {
                        format!("Yaba Sanshiro 2 target {} outside contract", row.target_id)
                    })?;
            ensure!(
                seen.insert(pad_key),
                "Yaba Sanshiro 2 pad key {pad_key} appears twice"
            );
            let input = row
                .input
                .as_ref()
                .context("Yaba Sanshiro 2 control not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Yaba Sanshiro 2 needs native controls")?;
            let code = (native.code & 0xffff) as u16;
            let host = match native.code >> 16 {
                1 => if is_gamepad {
                    super::super::controller_yaba_sanshiro::gc_button_code(device_index, code)
                } else {
                    super::super::controller_yaba_sanshiro::raw_button_code(device_index, code)
                }
                .with_context(|| format!("Yaba Sanshiro 2 button {code} outside host encoding"))?,
                3 => {
                    ensure!(
                        !(0x10..=0x17).contains(&u32::from(code)),
                        "Yaba Sanshiro 2 hat switches need SDL-hat identity mapping"
                    );
                    let positive = native.direction > 0;
                    if is_gamepad {
                        super::super::controller_yaba_sanshiro::gc_axis_code(
                            device_index,
                            code,
                            positive,
                        )
                    } else {
                        super::super::controller_yaba_sanshiro::raw_axis_code(
                            device_index,
                            code,
                            positive,
                        )
                    }
                    .with_context(|| format!("Yaba Sanshiro 2 axis {code} outside host encoding"))?
                }
                other => anyhow::bail!("Yaba Sanshiro 2 cannot consume input class {other}"),
            };
            bindings.push((pad_key, host));
        }
        let body = super::super::controller_yaba_sanshiro::ini_body(
            setup.port,
            setup.device_id,
            &bindings,
        )?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-yaba-sanshiro-")
            .tempdir()?;
        let config_path = directory.path().join("yabause.ini");
        std::fs::write(&config_path, body)?;
        let mut hashes = std::collections::BTreeMap::new();
        for path in [&setup.probe_program, &setup.sdl_library, &setup.content] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(config_path.clone(), file_hash(&config_path)?);
        let session = Self {
            directory,
            config_path,
            runtime_path,
            device_index,
            topology,
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
                "Yaba Sanshiro 2 launch input changed"
            );
        }
        let fresh = observe(&self.setup, cancel)?;
        ensure!(
            fresh
                .devices
                .iter()
                .any(|device| device.path.as_deref() == Some(self.runtime_path.as_str())),
            "Yaba Sanshiro 2 controller disappeared"
        );
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
