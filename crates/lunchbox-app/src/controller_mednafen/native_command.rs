//! Native Linux command preparation and owned child startup.
use super::{session::PreparedSession, settings::SavedSetup};
use crate::{
    controller_catalog::Calibration,
    controller_native_process::cancelled,
    controllers::ControllerDevice,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::atomic::AtomicBool,
};

pub(crate) struct NativeSession {
    pub(crate) inputs: PreparedSession,
    pub(crate) executable: PathBuf,
    pub(crate) setup: SavedSetup,
    pub(crate) plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
    disc: Option<super::disc::Snapshot>,
}

mod startup;

impl NativeSession {
    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == &self.plan,
            "Mednafen launch plan changed after preparation"
        );
        self.verify(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan(plan)?;
        if let Err(error) = startup::confirm(self, &mut child, cancel) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        if let Some(disc) = &self.disc {
            disc.verify()?;
        }
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "Mednafen launch input/runtime changed"
            );
        }
        self.inputs.verify(cancel)
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.inputs.check_health()
    }
}

pub(crate) fn prepare(
    setup: &SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    inventory: &[ControllerDevice],
    option: &RomEmulatorOption,
    original: &LaunchPlan,
    platform: &str,
    cancel: &AtomicBool,
) -> Result<NativeSession> {
    cancelled(cancel)?;
    setup.validate()?;
    let extension = setup
        .content
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let is_disc = ["ccd", "cue"]
        .iter()
        .any(|value| extension.eq_ignore_ascii_case(value));
    let disc = if matches!(setup.gamepad.system(), "ss" | "psx")
        || (matches!(setup.gamepad.system(), "pce" | "pce_fast") && is_disc)
    {
        Some(super::disc::Snapshot::capture(&setup.content)?)
    } else {
        None
    };
    let EmulatorExecutable::Native(executable) = &option.executable else {
        anyhow::bail!("Mednafen native calibrated launch requires native Linux");
    };
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "Mednafen identity differs or custom environment needs resolution"
    );
    let executable = executable.canonicalize()?;
    ensure!(
        executable == original.program.canonicalize()?,
        "Mednafen executable differs from selection"
    );
    ensure!(
        original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
        "Mednafen calibrated launch requires exactly the saved ROM argument"
    );
    let cwd = original.current_directory.canonicalize()?;
    let mut files = BTreeMap::new();
    for path in [
        &original.program,
        &executable,
        &setup.bubblewrap_program,
        &setup.content,
    ] {
        files.insert(path.clone(), file_hash(path)?);
    }
    ensure!(
        files[&executable].eq_ignore_ascii_case(&setup.executable_sha256),
        "Mednafen executable differs from saved trusted native runtime"
    );
    ensure!(
        match setup.gamepad {
            super::profiles::Gamepad::SaturnDigital
            | super::profiles::Gamepad::PlayStationDigital
            | super::profiles::Gamepad::PlayStationDualAnalog => ["ccd", "cue"]
                .iter()
                .any(|value| extension.eq_ignore_ascii_case(value)),
            super::profiles::Gamepad::MdThree | super::profiles::Gamepad::MdSix => matches!(
                extension.to_ascii_lowercase().as_str(),
                "bin" | "md" | "smd"
            ),
            super::profiles::Gamepad::Snes | super::profiles::Gamepad::SnesFaust => matches!(
                extension.to_ascii_lowercase().as_str(),
                "sfc" | "smc" | "swc" | "fig"
            ),
            super::profiles::Gamepad::NesTwo
            | super::profiles::Gamepad::NesFourScore
            | super::profiles::Gamepad::NesFamicomFour => matches!(
                extension.to_ascii_lowercase().as_str(),
                "nes" | "unf" | "unif" | "fds"
            ),
            super::profiles::Gamepad::PceTwo
            | super::profiles::Gamepad::PceSix
            | super::profiles::Gamepad::PceFastTwo
            | super::profiles::Gamepad::PceFastSix => matches!(
                extension.to_ascii_lowercase().as_str(),
                "pce" | "sgx" | "ccd" | "cue"
            ),
            super::profiles::Gamepad::MasterSystem => extension.eq_ignore_ascii_case("sms"),
            super::profiles::Gamepad::GameGear => extension.eq_ignore_ascii_case("gg"),
            super::profiles::Gamepad::VirtualBoy =>
                matches!(extension.to_ascii_lowercase().as_str(), "vb" | "vboy"),
            super::profiles::Gamepad::WonderSwan =>
                matches!(extension.to_ascii_lowercase().as_str(), "ws" | "wsc"),
            super::profiles::Gamepad::NeoGeoPocket =>
                matches!(extension.to_ascii_lowercase().as_str(), "ngp" | "ngc"),
            super::profiles::Gamepad::Lynx => extension.eq_ignore_ascii_case("lnx"),
            super::profiles::Gamepad::GameBoy => matches!(
                extension.to_ascii_lowercase().as_str(),
                "gb" | "gbc" | "cgb"
            ),
            super::profiles::Gamepad::GameBoyAdvance => matches!(
                extension.to_ascii_lowercase().as_str(),
                "gba" | "agb" | "bin"
            ),
        },
        "Mednafen calibrated launch currently requires a raw handheld ROM; compressed-content naming remains unimplemented"
    );
    let file_base = setup
        .content
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("Mednafen content basename is not UTF-8"))?;
    let inputs = PreparedSession::prepare(setup, file_base, calibrations, inventory, cancel)?;
    let mut plan = original.clone();
    plan.program = setup.bubblewrap_program.clone();
    // Keep the saved native base and saves; overlay only configuration files.
    plan.arguments = vec![
        "--die-with-parent".into(),
        "--bind".into(),
        "/".into(),
        "/".into(),
        "--setenv".into(),
        "MEDNAFEN_HOME".into(),
        setup.base_directory.as_os_str().to_owned(),
    ];
    inputs.configuration.append_mounts(&mut plan.arguments)?;
    // Firmware: bind the Lunchbox-managed manual-import directory over the
    // base dir's firmware/ subpath so Mednafen's default
    // filesys.path_firmware resolution finds user-dropped BIOS files.
    let firmware_host_dir = crate::firmware::manual_firmware_dir("mednafen", "mednafen", platform)?;
    std::fs::create_dir_all(&firmware_host_dir)?;
    plan.arguments.extend([
        "--bind".into(),
        firmware_host_dir.as_os_str().to_owned(),
        setup.base_directory.join("firmware").into_os_string(),
    ]);
    plan.arguments.extend([
        "--chdir".into(),
        cwd.into_os_string(),
        "--".into(),
        executable.as_os_str().to_owned(),
    ]);
    plan.arguments
        .extend(["-force_module".into(), setup.gamepad.system().into()]);
    plan.arguments.extend_from_slice(&original.arguments);
    let session = NativeSession {
        inputs,
        executable,
        setup: setup.clone(),
        plan,
        files,
        disc,
    };
    session.verify(cancel)?;
    Ok(session)
}
