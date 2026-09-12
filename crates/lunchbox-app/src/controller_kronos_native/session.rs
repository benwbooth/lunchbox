//! Native SDL2 probe for Kronos: enumerates the trusted runtime and
//! translates calibrated controls into kronos.ini binding strings.
use crate::controller_bizhawk_guard::InputTopology;
use crate::controller_catalog::Calibration;
use crate::controller_native_process::{cancelled, capture};
use crate::controllers::ControllerDevice;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{Snapshot, file_hash};
use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

fn observe(
    setup: &crate::controller_kronos_native::settings::SavedSetup,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command.arg("--sdl-library").arg(&setup.sdl_library);
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid Kronos SDL capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "Kronos helper inspected a different SDL runtime"
    );
    Ok(snapshot)
}

pub(crate) struct PreparedSession {
    pub(crate) device_index: u32,
    pub(crate) topology: InputTopology,
    pub(crate) bindings: Vec<(String, String)>,
    setup: crate::controller_kronos_native::settings::SavedSetup,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &crate::controller_kronos_native::settings::SavedSetup,
        calibrations: &HashMap<String, crate::controller_catalog::Calibration>,
        inventory: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        let mut matches = inventory
            .iter()
            .filter(|d| d.stable_id == setup.controller_id);
        let device = matches
            .next()
            .context("Kronos selected controller is disconnected")?;
        anyhow::ensure!(
            matches.next().is_none() && !device.is_virtual,
            "Kronos requires an unambiguous physical controller"
        );
        let selected = device.device_path.clone();
        let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
        let snapshot = observe(setup, cancel)?;
        let runtime_path = topology
            .resolve_runtime_path(
                &selected,
                snapshot.devices.iter().filter_map(|d| d.path.as_deref()),
            )
            .context("Kronos device not found in SDL enumeration")?;
        let captured = observe(setup, cancel)?;
        let dev = captured
            .devices
            .iter()
            .find(|d| d.path.as_deref() == Some(runtime_path.as_str()))
            .context("Kronos device not found")?;
        let device_index = snapshot
            .devices
            .iter()
            .position(|d| d.path.as_deref() == Some(runtime_path.as_str()))
            .context("Kronos device position not found")? as u32;
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == "kronos:standalone-genesis")
            .context("Missing Kronos profile")?;
        let calibration = calibrations
            .get(&setup.controller_id)
            .context("Kronos calibration disappeared")?;
        let plan = calibration.plan_profile(profile)?;
        let mut bindings = Vec::new();
        for row in plan.rows {
            let field = super::super::controller_kronos::CONTROLS
                .iter()
                .find(|(control, _)| *control == row.target_id)
                .map(|(_, output)| *output)
                .with_context(|| format!("Kronos target {} outside contract", row.target_id))?;
            let input = row
                .input
                .as_ref()
                .context("Kronos control not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Kronos needs native controls")?;
            let code = native.code & 0xffff;
            let binding = match native.code >> 16 {
                1 => format!("Button {}", code as u16),
                3 => format!(
                    "Axis {} {}",
                    code as u16,
                    if native.direction > 0 {
                        "positive"
                    } else {
                        "negative"
                    }
                ),
                other => anyhow::bail!("Kronos cannot consume input class {}", other),
            };
            bindings.push((field.to_owned(), binding));
        }
        Ok(PreparedSession {
            device_index,
            topology,
            bindings,
            setup: setup.clone(),
        })
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }
}
