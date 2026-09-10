//! Native Linux launch preparation. All process execution occurs only when the
//! user launches a saved setup; editing/reviewing settings never invokes this.
use super::*;
use crate::emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption};
use std::io::Read;
use std::process::{Child, Command};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;

pub(crate) struct NativeSession {
    input: PreparedSession,
    plan: LaunchPlan,
    executable_hash: String,
}

fn cancelled(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "{}",
        crate::rom_launch_preparation::LAUNCH_CANCELLED_ERROR
    );
    Ok(())
}

use crate::controller_native_process::capture;

pub(crate) fn prepare(
    setup: &SavedSetup,
    calibrations: &std::collections::HashMap<String, Calibration>,
    inventory: &[crate::controllers::ControllerDevice],
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    cancel: &AtomicBool,
) -> Result<NativeSession> {
    setup.validate()?;
    cancelled(cancel)?;
    let runtime = setup
        .runtime
        .as_ref()
        .context("Configure trusted DuckStation helper and SDL runtime paths first")?;
    let EmulatorExecutable::Native(executable) = &option.executable else {
        anyhow::bail!(
            "DuckStation calibrated launch currently requires native Linux; Flatpak/Wine namespace routing is separate"
        );
    };
    ensure!(
        setup.emulator_id == option.emulator_id && plan.environment.is_empty(),
        "DuckStation setup identity differs or custom environment needs resolution"
    );
    ensure!(
        executable.canonicalize()? == plan.program.canonicalize()?,
        "DuckStation launch program changed"
    );
    ensure!(
        file_hash(&plan.program)? == runtime.executable_sha256,
        "DuckStation executable differs from the saved runtime"
    );
    let executable = executable.canonicalize()?;
    let directory = executable
        .parent()
        .context("Missing DuckStation executable directory")?;
    for marker in ["portable.txt", "settings.ini"] {
        ensure!(
            !directory.join(marker).try_exists()?,
            "DuckStation portable configuration overrides private routing"
        );
    }
    let mut content_count = 0;
    for argument in &plan.arguments {
        if argument == setup.content.as_os_str() {
            content_count += 1;
        } else {
            ensure!(
                matches!(
                    argument.to_str(),
                    Some("-batch" | "-fullscreen" | "-nofullscreen")
                ),
                "Custom DuckStation argument needs input/configuration routing resolution"
            );
        }
    }
    ensure!(
        content_count == 1 && setup.content.is_file(),
        "DuckStation launch must select the exact saved content once"
    );
    let (version_out, version_err) = capture(Command::new(&executable).arg("-version"), cancel)?;
    let version = format!(
        "{}\n{}",
        String::from_utf8_lossy(&version_out),
        String::from_utf8_lossy(&version_err)
    );
    ensure!(
        version.contains("0.1-9482-g0a53bc47c"),
        "DuckStation executable revision is outside the supported native input contract"
    );
    let mut staged = LaunchConfig::stage(
        &setup.data_root,
        Some(GameIdentity {
            serial: &setup.serial,
            first_disc_serial: setup.first_disc_serial.as_deref(),
        }),
    )?;
    let hints = staged.configure_classic_sdl()?;
    let mut probe = Command::new(&runtime.probe_program);
    probe
        .arg("--sdl-library")
        .arg(&runtime.sdl_library)
        .arg("--duckstation-player-probe")
        .arg(lunchbox_controller_probe::players::CONTRACT);
    for library in &runtime.runtime_libraries {
        probe.arg("--runtime-library").arg(library);
    }
    for (key, value) in &hints {
        probe.arg("--hint").arg(format!("{key}={value}"));
    }
    let database = setup.data_root.join("gamecontrollerdb.txt");
    if database.try_exists()? {
        probe.arg("--mapping-db").arg(&database);
    }
    for player in &setup.players {
        let mut devices = inventory
            .iter()
            .filter(|device| device.stable_id == player.controller_id);
        let device = devices
            .next()
            .context("DuckStation controller is disconnected")?;
        ensure!(
            devices.next().is_none(),
            "Ambiguous DuckStation physical controller identity"
        );
        probe.arg("--bindings-for-path").arg(&device.device_path);
    }
    let (output, _) = capture(&mut probe, cancel)?;
    staged.verify_originals_unchanged()?;
    let snapshot =
        serde_json::from_slice(&output).context("Invalid DuckStation SDL helper response")?;
    let input = prepare_resolved(setup, calibrations, inventory, &snapshot)?;
    let mut configured = plan.clone();
    configured.environment.push((
        "XDG_CONFIG_HOME".into(),
        input.config_home().into_os_string(),
    ));
    for (key, value) in hints {
        configured.environment.push((key.into(), value.into()));
    }
    configured.arguments.insert(0, "-earlyconsole".into());
    let session = NativeSession {
        input,
        plan: configured.clone(),
        executable_hash: runtime.executable_sha256.clone(),
    };
    session.verify()?;
    *plan = configured;
    Ok(session)
}

impl NativeSession {
    pub(crate) fn verify(&self) -> Result<()> {
        self.input.verify_before_launch()?;
        ensure!(
            file_hash(&self.plan.program)? == self.executable_hash,
            "DuckStation executable changed during preparation"
        );
        Ok(())
    }

    pub(crate) fn spawn(&mut self, plan: &LaunchPlan, cancel: &AtomicBool) -> Result<Child> {
        ensure!(
            &self.plan == plan,
            "DuckStation launch plan changed after input preparation"
        );
        self.verify()?;
        cancelled(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan_with_controller_pipes(plan)?;
        let result = (|| {
            let output = Arc::new(Mutex::new([Vec::<u8>::new(), Vec::<u8>::new()]));
            let collecting = Arc::new(AtomicBool::new(true));
            let overflow = Arc::new(AtomicBool::new(false));
            let readers: Vec<Box<dyn Read + Send>> = vec![
                Box::new(child.stdout.take().context("Missing DuckStation stdout")?),
                Box::new(child.stderr.take().context("Missing DuckStation stderr")?),
            ];
            for (stream, mut reader) in readers.into_iter().enumerate() {
                let output = output.clone();
                let collecting = collecting.clone();
                let overflow = overflow.clone();
                std::thread::Builder::new()
                    .name("duckstation-startup-log".into())
                    .spawn(move || {
                        let mut buffer = [0_u8; 4096];
                        loop {
                            let count = match reader.read(&mut buffer) {
                                Ok(0) | Err(_) => break,
                                Ok(count) => count,
                            };
                            if collecting.load(Ordering::Relaxed) {
                                let Ok(mut output) = output.lock() else {
                                    break;
                                };
                                if output[stream].len() + count <= OUTPUT_LIMIT {
                                    output[stream].extend_from_slice(&buffer[..count]);
                                } else {
                                    overflow.store(true, Ordering::Relaxed);
                                }
                            }
                        }
                    })?;
            }
            let deadline = Instant::now() + Duration::from_secs(20);
            let confirmed = (|| {
                loop {
                    cancelled(cancel)?;
                    ensure!(
                        !overflow.load(Ordering::Relaxed),
                        "DuckStation startup log exceeded limit"
                    );
                    ensure!(
                        child.try_wait()?.is_none(),
                        "DuckStation exited before controller startup was confirmed"
                    );
                    let log = {
                        let output = output
                            .lock()
                            .map_err(|_| anyhow::anyhow!("DuckStation log reader failed"))?;
                        format!(
                            "{}\n{}",
                            String::from_utf8_lossy(&output[0]),
                            String::from_utf8_lossy(&output[1])
                        )
                    };
                    if self.input.confirm_startup(&log).is_ok() {
                        let maps = std::fs::read_to_string(format!("/proc/{}/maps", child.id()))?;
                        let loaded: BTreeSet<_> = maps
                            .lines()
                            .filter_map(|line| {
                                let path = line
                                    .split_whitespace()
                                    .skip(5)
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                path.starts_with('/')
                                    .then(|| std::path::PathBuf::from(path.replace("\\040", " ")))
                            })
                            .collect();
                        for path in self.input.runtime_inputs.keys() {
                            let expected = path.canonicalize()?;
                            ensure!(
                                loaded.contains(&expected),
                                "DuckStation did not load the inspected runtime library: {}",
                                expected.display()
                            );
                        }
                        return Ok(());
                    }
                    ensure!(
                        Instant::now() < deadline,
                        "DuckStation did not confirm private configuration and controller routing before timeout"
                    );
                    std::thread::sleep(Duration::from_millis(25));
                }
            })();
            collecting.store(false, Ordering::Relaxed);
            confirmed
        })();
        if let Err(error) = result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }
}
