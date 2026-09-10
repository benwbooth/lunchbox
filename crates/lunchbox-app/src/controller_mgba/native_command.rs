//! Native SDL frontend launch ownership. Invoked only for an explicit launch.
use super::{configuration::PreparedConfig, settings::SavedSetup};
use crate::controller_bizhawk_guard::InputTopology;
use crate::controller_catalog::Calibration;
use crate::controller_native_process::{cancelled, capture};
use crate::controllers::ControllerDevice;
use crate::emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{file_hash, sdl2::Snapshot};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::AtomicBool;

mod startup;

pub(crate) struct NativeSession {
    configuration: PreparedConfig,
    topology: InputTopology,
    snapshot: Snapshot,
    runtime_path: String,
    setup: SavedSetup,
    runtime_files: BTreeMap<PathBuf, String>,
    executable: PathBuf,
    cwd: PathBuf,
    plan: LaunchPlan,
}

pub(crate) fn native_config(cwd: &Path) -> Result<PathBuf> {
    let marker = cwd.join("portable.ini");
    if marker.try_exists()? {
        // Native mGBA detects this marker by opening it. Require the same
        // readable regular marker, not just a directory with that name.
        ensure!(
            std::fs::File::open(&marker)?.metadata()?.is_file(),
            "mGBA portable marker is not a regular file"
        );
        return Ok(cwd.join("config.ini"));
    }
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .context("mGBA native configuration home is unavailable")?;
    ensure!(
        root.is_absolute(),
        "mGBA native configuration home must be absolute"
    );
    Ok(root.join("mgba/config.ini"))
}

fn observe(setup: &SavedSetup, path: Option<&str>, cancel: &AtomicBool) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command
        .arg("--sdl2-inventory")
        .arg("--sdl-library")
        .arg(&setup.sdl_library)
        .env("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1")
        .env("SDL_NO_SIGNAL_HANDLERS", "1");
    if let Some(path) = path {
        command.arg("--sdl2-controls-for-path").arg(path);
    }
    let (out, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&out).context("Invalid mGBA SDL inventory response")?;
    ensure!(
        snapshot.version[0] == 2
            && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
        "mGBA helper inspected a different SDL2 runtime"
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
) -> Result<NativeSession> {
    cancelled(cancel)?;
    setup.review(calibrations)?;
    let EmulatorExecutable::Native(executable) = &option.executable else {
        anyhow::bail!(
            "mGBA calibrated launch requires native Linux SDL; Qt/Wine/Flatpak routing is separate"
        );
    };
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "mGBA setup identity differs or custom environment needs resolution"
    );
    let executable = executable.canonicalize()?;
    ensure!(
        executable == original.program.canonicalize()?,
        "mGBA launch executable changed"
    );
    let mut content_count = 0;
    for arg in &original.arguments {
        if arg == setup.content.as_os_str() {
            content_count += 1;
        } else {
            ensure!(
                matches!(arg.to_str(), Some("-f" | "--fullscreen")),
                "mGBA custom arguments need input/config routing resolution"
            );
        }
    }
    ensure!(
        content_count == 1 && setup.content.is_file(),
        "mGBA launch must select exact saved content once"
    );
    let cwd = original.current_directory.canonicalize()?;
    ensure!(
        native_config(&cwd)?.canonicalize()? == setup.source_config.canonicalize()?,
        "mGBA saved config differs from native portable/XDG selection"
    );
    let mut runtime_files = BTreeMap::new();
    for path in [
        &original.program,
        &setup.probe_program,
        &setup.sdl_library,
        &setup.bubblewrap_program,
    ] {
        runtime_files.insert(path.clone(), file_hash(path)?);
    }
    ensure!(
        runtime_files[&original.program].eq_ignore_ascii_case(&setup.executable_sha256),
        "mGBA executable differs from the saved trusted runtime"
    );
    // These CLI-only queries run during user launch, never settings review.
    let (version, _) = capture(Command::new(&executable).arg("--version"), cancel)?;
    ensure!(
        String::from_utf8_lossy(&version)
            .split_whitespace()
            .any(|part| part == "0.10.5"),
        "mGBA version differs from the native SDL input contract"
    );
    let (help, _) = capture(Command::new(&executable).arg("--help"), cancel)?;
    ensure!(
        String::from_utf8_lossy(&help).contains("Graphics options:")
            && String::from_utf8_lossy(&help).contains("Scale viewport by 1-8 times"),
        "mGBA executable does not expose the expected SDL frontend command contract"
    );
    let mut devices = inventory
        .iter()
        .filter(|device| device.stable_id == setup.controller_id);
    let device = devices.next().context("mGBA controller is disconnected")?;
    ensure!(
        devices.next().is_none() && !device.is_virtual,
        "mGBA requires one unambiguous physical controller"
    );
    let topology = InputTopology::capture(std::slice::from_ref(&device.device_path))?;
    let initial = observe(setup, None, cancel)?;
    let runtime_path = topology.resolve_runtime_path(
        &device.device_path,
        initial
            .devices
            .iter()
            .filter_map(|device| device.path.as_deref()),
    )?;
    let snapshot = observe(setup, Some(&runtime_path), cancel)?;
    let mut routing = snapshot.clone();
    for device in &mut routing.devices {
        device.controls = None;
        device.linux_classic = None;
        device.linux_evdev = None;
    }
    initial.ensure_same_routing(&routing)?;
    topology.verify()?;
    let calibration = calibrations
        .get(&setup.controller_id)
        .context("mGBA calibration disappeared")?;
    let configuration = PreparedConfig::prepare(setup, calibration, &snapshot, &runtime_path)?;
    let mut plan = original.clone();
    plan.arguments = configuration.overlay_arguments(&executable, &original.arguments, &cwd)?;
    plan.program = setup.bubblewrap_program.clone();
    plan.environment.extend([
        ("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS".into(), "1".into()),
        ("SDL_NO_SIGNAL_HANDLERS".into(), "1".into()),
    ]);
    let session = NativeSession {
        configuration,
        topology,
        snapshot,
        runtime_path,
        setup: setup.clone(),
        runtime_files,
        executable,
        cwd,
        plan,
    };
    session.verify()?;
    cancelled(cancel)?;
    Ok(session)
}

impl NativeSession {
    pub(crate) fn launch_plan(&self) -> &LaunchPlan {
        &self.plan
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.check_health()?;
        self.configuration.verify()?;
        ensure!(
            native_config(&self.cwd)?.canonicalize()? == self.setup.source_config.canonicalize()?,
            "mGBA portable/native configuration selection changed"
        );
        for (path, expected) in &self.runtime_files {
            ensure!(
                file_hash(path)? == *expected,
                "mGBA runtime input changed: {}",
                path.display()
            );
        }
        Ok(())
    }

    pub(crate) fn spawn(&mut self, plan: &LaunchPlan, cancel: &AtomicBool) -> Result<Child> {
        ensure!(
            plan == &self.plan,
            "mGBA launch differs from the prepared plan"
        );
        self.verify()?;
        let fresh = observe(&self.setup, Some(&self.runtime_path), cancel)?;
        self.snapshot.ensure_same_routing(&fresh)?;
        self.verify()?;
        cancelled(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan(plan)?;
        if let Err(error) = startup::confirm(self, &mut child, cancel) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }
}
