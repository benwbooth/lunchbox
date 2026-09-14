//! Native SDL2 probe for Yaba Sanshiro 2: enumerates the trusted runtime,
//! disambiguates the saved controller, and translates its calibration into
//! `yabause.ini` host key codes for a private-HOME config.
use crate::controller_bizhawk_guard::InputTopology;
use crate::controller_catalog::Calibration;
use crate::controller_native_process::{cancelled, capture};
use crate::controllers::ControllerDevice;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    duckstation::DigitalInput,
    file_hash,
    linux_classic::AxisEndpoints,
    sdl2::Snapshot,
    sdl2_mapping::{self, Input as MappingInput},
    sdl2_physical::PhysicalMap,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    process::Command,
    sync::atomic::AtomicBool,
};

fn observe(
    setup: &crate::controller_yaba_sanshiro_native::settings::SavedSetup,
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
        serde_json::from_slice(&output).context("Invalid Yaba Sanshiro 2 SDL capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "Yaba Sanshiro 2 helper inspected a different SDL runtime"
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

fn read_mapping_input(input: &MappingInput, measured: DigitalInput, pressed: bool) -> Result<i32> {
    match (input, measured) {
        (MappingInput::Button(index), DigitalInput::Button(actual)) if *index == actual => {
            Ok(i32::from(pressed))
        }
        (
            MappingInput::Hat { index, mask },
            DigitalInput::Hat {
                index: actual,
                direction,
            },
        ) if *index == actual && *mask == direction => {
            Ok(if pressed { i32::from(direction) } else { 0 })
        }
        (
            MappingInput::Axis { index, .. },
            DigitalInput::Axis {
                index: actual,
                released,
                pressed: active,
            },
        ) if *index == actual => Ok(i32::from(if pressed { active } else { released })),
        _ => anyhow::bail!("SDL2 logical output depends on another physical input"),
    }
}

enum LogicalInput {
    Button(u16),
    Axis { index: u16, positive: bool },
}

fn logical_input(mapping: &str, measured: DigitalInput) -> Result<LogicalInput> {
    let bindings = sdl2_mapping::parse(mapping)?;
    let outputs = bindings
        .iter()
        .map(|binding| binding.output.as_str())
        .collect::<BTreeSet<_>>();
    let mut candidates = Vec::new();
    for output in outputs {
        let before = sdl2_mapping::output_value(&bindings, output, |input| {
            read_mapping_input(input, measured, false)
        });
        let after = sdl2_mapping::output_value(&bindings, output, |input| {
            read_mapping_input(input, measured, true)
        });
        let (Ok(before), Ok(after)) = (before, after) else {
            continue;
        };
        if before == after {
            continue;
        }
        let analog = bindings
            .iter()
            .find(|binding| binding.output == output)
            .is_some_and(|binding| binding.output_range.is_some());
        if analog {
            ensure!(
                before.abs() < 10_000 && after.abs() > 10_000,
                "Yaba Sanshiro 2 logical axis does not leave a centered rest"
            );
            let index = [
                "leftx",
                "lefty",
                "rightx",
                "righty",
                "lefttrigger",
                "righttrigger",
            ]
            .iter()
            .position(|candidate| *candidate == output)
            .context("Yaba Sanshiro 2 mapping uses an unknown logical axis")?;
            candidates.push(LogicalInput::Axis {
                index: u16::try_from(index)?,
                positive: after > 0,
            });
        } else if before == 0 && after != 0 {
            let index = [
                "a",
                "b",
                "x",
                "y",
                "back",
                "guide",
                "start",
                "leftstick",
                "rightstick",
                "leftshoulder",
                "rightshoulder",
                "dpup",
                "dpdown",
                "dpleft",
                "dpright",
            ]
            .iter()
            .position(|candidate| *candidate == output)
            .context("Yaba Sanshiro 2 mapping uses an unsupported logical button")?;
            candidates.push(LogicalInput::Button(u16::try_from(index)?));
        }
    }
    ensure!(
        candidates.len() == 1,
        "Yaba Sanshiro 2 physical control has no unique SDL2 logical output"
    );
    Ok(candidates.remove(0))
}

fn host_code(
    device: &lunchbox_controller_probe::sdl2::Device,
    physical: &PhysicalMap,
    device_index: u32,
    input: &crate::controller_catalog::InputBinding,
) -> Result<u32> {
    let native = input
        .native
        .as_ref()
        .context("Yaba Sanshiro 2 needs native controls")?;
    let endpoints = input.axis.as_ref().map(|axis| AxisEndpoints {
        released: axis.released,
        pressed: axis.pressed,
    });
    let measured = physical.digital_input(native.code, endpoints)?;
    let encoded = if device.is_game_controller {
        match logical_input(
            device
                .mapping
                .as_deref()
                .context("Yaba Sanshiro 2 SDL2 game controller mapping is absent")?,
            measured,
        )? {
            LogicalInput::Button(index) => {
                super::super::controller_yaba_sanshiro::gc_button_code(device_index, index)
            }
            LogicalInput::Axis { index, positive } => {
                super::super::controller_yaba_sanshiro::gc_axis_code(device_index, index, positive)
            }
        }
    } else {
        match measured {
            DigitalInput::Button(index) => super::super::controller_yaba_sanshiro::raw_button_code(
                device_index,
                u16::try_from(index)?,
            ),
            DigitalInput::Hat { index, direction } => {
                super::super::controller_yaba_sanshiro::raw_hat_code(
                    device_index,
                    u16::try_from(index)?,
                    direction,
                )
            }
            DigitalInput::Axis {
                index,
                released,
                pressed,
            } => {
                ensure!(
                    released.abs() < 10_000 && pressed.abs() > 10_000,
                    "Yaba Sanshiro 2 raw axis does not leave a centered rest"
                );
                super::super::controller_yaba_sanshiro::raw_axis_code(
                    device_index,
                    u16::try_from(index)?,
                    pressed > 0,
                )
            }
        }
    };
    encoded.map_err(|()| anyhow::anyhow!("Yaba Sanshiro 2 input is outside host encoding"))
}

pub(crate) struct PreparedSession {
    pub(crate) directory: tempfile::TempDir,
    pub(crate) config_path: std::path::PathBuf,
    pub(crate) runtime_path: String,
    pub(crate) device_index: u32,
    pub(crate) topology: InputTopology,
    initial: Snapshot,
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
        let initial = routing(observe(setup, None, cancel)?);
        let runtime_path = topology
            .resolve_runtime_path(
                &selected,
                initial.devices.iter().filter_map(|d| d.path.as_deref()),
            )
            .context("Yaba Sanshiro 2 device not found in SDL enumeration")?;
        let captured = observe(setup, Some(&runtime_path), cancel)?;
        initial.ensure_same_routing(&routing(captured.clone()))?;
        topology.verify()?;
        let dev = captured.device_at_path(&runtime_path)?;
        let physical = PhysicalMap::from_device(dev)?;
        let device_index = dev.device_index;
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
                    .ok_or_else(|| {
                        anyhow::anyhow!("Yaba Sanshiro 2 target {} outside contract", row.target_id)
                    })?;
            ensure!(
                seen.insert(pad_key),
                "Yaba Sanshiro 2 pad key {pad_key} appears twice"
            );
            let input = row
                .input
                .as_ref()
                .context("Yaba Sanshiro 2 control not calibrated")?;
            let host = host_code(dev, &physical, device_index, input)?;
            bindings.push((pad_key, host));
        }
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-yaba-sanshiro-")
            .tempdir()?;
        let config_path = directory.path().join("yabause.ini");
        let baseline = std::fs::read(&setup.config_path)
            .context("Reading the declared Yaba Sanshiro 2 configuration")?;
        std::fs::write(
            &config_path,
            super::super::controller_yaba_sanshiro::patch_ini(
                &baseline,
                setup.port,
                setup.device_id,
                &runtime_path,
                dev.name.as_deref().unwrap_or("SDL controller"),
                &bindings,
            )?,
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
            runtime_path,
            device_index,
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
                "Yaba Sanshiro 2 launch input changed"
            );
        }
        let fresh = routing(observe(&self.setup, None, cancel)?);
        self.initial.ensure_same_routing(&fresh)?;
        ensure!(
            fresh.device_at_path(&self.runtime_path)?.device_index == self.device_index,
            "Yaba Sanshiro 2 controller routing changed"
        );
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
