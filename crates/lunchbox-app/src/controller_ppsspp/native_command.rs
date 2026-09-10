//! Native PPSSPP preparation, invoked only by an explicit user launch.
//! This owns the topology and private configuration, but does not certify that
//! a child loaded the expected SDL library, assets or configuration directory.
use super::{session::PreparedInputs, settings::SavedSetup};
use crate::controller_bizhawk_guard::InputTopology;
use crate::controller_catalog::Calibration;
use crate::controller_native_process::{cancelled, capture};
use crate::controllers::ControllerDevice;
use crate::emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::sdl2::Snapshot;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

mod startup;

pub(crate) struct PreparedLaunch {
    input: PreparedInputs,
    topology: InputTopology,
    runtime_files: BTreeMap<PathBuf, String>,
    setup: SavedSetup,
    runtime_path: String,
    plan: LaunchPlan,
    executable: PathBuf,
    mappings: Vec<String>,
}

fn file_hash(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "PPSSPP runtime input is not a regular file"
    );
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn observe(
    setup: &SavedSetup,
    runtime_path: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command
        .arg("--sdl2-inventory")
        .arg("--sdl-library")
        .arg(&setup.sdl_library)
        .arg("--sdl2-mapping-db")
        .arg(&setup.mapping_database)
        // PPSSPP's SDL frontend sets this hint before initializing SDL.
        .env("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
    if let Some(path) = runtime_path {
        command.arg("--sdl2-controls-for-path").arg(path);
    }
    let (stdout, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&stdout).context("Invalid PPSSPP SDL2 helper response")?;
    ensure!(snapshot.version[0] == 2, "PPSSPP requires an SDL2 runtime");
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "PPSSPP helper inspected a different SDL runtime"
    );
    Ok(snapshot)
}

pub(crate) fn prepare(
    setup: &SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    inventory: &[ControllerDevice],
    option: &RomEmulatorOption,
    original: &LaunchPlan,
    cancel: &AtomicBool,
) -> Result<PreparedLaunch> {
    cancelled(cancel)?;
    setup.review(calibrations)?;
    // The pinned Linux SDL frontend uses this path, before loading any INI.
    // Keep HOME/XDG unchanged so all memory-stick saves remain native.
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .context("PPSSPP requires an explicit native configuration home")?;
    ensure!(
        config_home.is_absolute()
            && config_home.join("ppsspp/PSP/SYSTEM").canonicalize()?
                == setup.source_system.canonicalize()?,
        "PPSSPP saved SYSTEM path differs from the native Linux search directory"
    );
    let EmulatorExecutable::Native(executable) = &option.executable else {
        anyhow::bail!(
            "PPSSPP calibrated launch requires native Linux; container/Wine routing is separate"
        );
    };
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "PPSSPP setup identity differs or custom environment needs resolution"
    );
    ensure!(
        executable.canonicalize()? == original.program.canonicalize()?,
        "PPSSPP launch executable changed"
    );
    // Configuration override flags, multiple content arguments and wrappers
    // cannot silently bypass the saved configuration identity.
    ensure!(
        original.arguments.len() == 1
            && original.arguments[0] == setup.content.as_os_str()
            && setup.content.is_file(),
        "PPSSPP requires the exact saved content without custom arguments"
    );
    let mut runtime_files = BTreeMap::new();
    for path in [
        &original.program,
        &setup.probe_program,
        &setup.sdl_library,
        &setup.mapping_database,
        &setup.bubblewrap_program,
    ] {
        runtime_files.insert(path.clone(), file_hash(path)?);
    }
    ensure!(
        runtime_files[&original.program].eq_ignore_ascii_case(&setup.executable_sha256),
        "PPSSPP executable differs from the saved runtime"
    );
    let mut matches = inventory
        .iter()
        .filter(|device| device.stable_id == setup.controller_id);
    let device = matches
        .next()
        .context("PPSSPP controller is disconnected")?;
    ensure!(
        matches.next().is_none() && !device.is_virtual,
        "PPSSPP requires one unambiguous physical controller identity"
    );
    let topology = InputTopology::capture(std::slice::from_ref(&device.device_path))?;
    // First enumerate without opening a guessed event/js path, then resolve
    // through the kernel identity and capture that exact SDL runtime node.
    let inventory_snapshot = observe(setup, None, cancel)?;
    let runtime_path = topology.resolve_runtime_path(
        &device.device_path,
        inventory_snapshot
            .devices
            .iter()
            .filter_map(|device| device.path.as_deref()),
    )?;
    let snapshot = observe(setup, Some(&runtime_path), cancel)?;
    let mappings = snapshot
        .devices
        .iter()
        .map(|device| {
            ensure!(
                device.is_game_controller,
                "PPSSPP startup cannot confirm SDL fallback mappings"
            );
            let mapping = device
                .mapping
                .clone()
                .context("PPSSPP runtime mapping is absent")?;
            ensure!(
                mapping.len() < 900 && !mapping.contains(['\r', '\n']),
                "PPSSPP mapping exceeds the native startup log contract"
            );
            Ok(mapping)
        })
        .collect::<Result<Vec<_>>>()?;
    // Enumeration intentionally has no physical-control capture. Compare the
    // ordered routing identity here, not the newly populated capture fields.
    let mut routing_snapshot = snapshot.clone();
    for device in &mut routing_snapshot.devices {
        device.controls = None;
        device.linux_classic = None;
        device.linux_evdev = None;
    }
    inventory_snapshot.ensure_same_routing(&routing_snapshot)?;
    topology.verify()?;
    let calibration = calibrations
        .get(&setup.controller_id)
        .context("PPSSPP controller calibration disappeared")?;
    let input = PreparedInputs::prepare(
        calibration,
        &snapshot,
        &runtime_path,
        &setup.source_system,
        &setup.game_id,
    )?;
    let mut plan = original.clone();
    let mut native_arguments = original.arguments.clone();
    native_arguments.insert(0, "--loglevel=4".into());
    plan.arguments = input.configuration.overlay_arguments(
        &executable.canonicalize()?,
        &native_arguments,
        &original.current_directory.canonicalize()?,
    )?;
    plan.program = setup.bubblewrap_program.clone();
    plan.environment
        .push(("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS".into(), "1".into()));
    let prepared = PreparedLaunch {
        input,
        topology,
        runtime_files,
        setup: setup.clone(),
        runtime_path,
        plan,
        executable: executable.canonicalize()?,
        mappings,
    };
    prepared.verify()?;
    cancelled(cancel)?;
    Ok(prepared)
}

impl PreparedLaunch {
    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    pub(crate) fn launch_plan(&self) -> &LaunchPlan {
        &self.plan
    }

    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == &self.plan,
            "PPSSPP launch differs from the prepared plan"
        );
        self.refresh_before_spawn(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan_with_controller_pipes(plan)?;
        if let Err(error) = startup::confirm(self, &mut child, cancel) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.topology.verify()?;
        self.input.configuration.verify_sources()?;
        for (path, expected) in &self.runtime_files {
            ensure!(
                file_hash(path)? == *expected,
                "PPSSPP runtime input changed: {}",
                path.display()
            );
        }
        Ok(())
    }

    /// Run immediately before child creation. The spawning owner must retain
    /// this object and independently confirm the child's runtime/config paths.
    pub(crate) fn refresh_before_spawn(&self, cancel: &AtomicBool) -> Result<&LaunchPlan> {
        self.verify()?;
        let fresh = observe(&self.setup, Some(&self.runtime_path), cancel)?;
        self.input.verify_observation(&fresh)?;
        self.verify()?;
        cancelled(cancel)?;
        Ok(&self.plan)
    }
}
