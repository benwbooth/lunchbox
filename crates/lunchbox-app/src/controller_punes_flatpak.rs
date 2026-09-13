//! Production controller launch path for the exact audited puNES 0.111 Flatpak.
//!
//! Source contract: `punesemu/puNES` v0.111 at
//! `613b7b44baddbe7bf2ff79e72348c3ee35f3e70b`. The Flatpak app,
//! KDE runtime, executable, loader and libudev are pinned independently.

pub(crate) mod configuration;
mod flatpak;
mod isolation;
mod session;
pub(crate) mod settings;
mod virtual_pad;

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
    time::{Duration, Instant},
};

pub(crate) const SOURCE_COMMIT: &str = "613b7b44baddbe7bf2ff79e72348c3ee35f3e70b";

pub(crate) struct PreparedLaunch {
    pub(crate) inputs: session::PreparedSession,
    pub(crate) executable: PathBuf,
    pub(crate) setup: settings::SavedSetup,
    pub(crate) plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
}

impl PreparedLaunch {
    pub(crate) fn check_health(&self) -> Result<()> {
        self.inputs.check_health()
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        for (path, expected) in &self.files {
            ensure!(file_hash(path)? == *expected, "puNES launch input changed");
        }
        self.inputs.verify(cancel)
    }

    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == &self.plan,
            "puNES launch plan changed after preparation"
        );
        self.verify(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan(plan)?;
        let result = (|| {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                cancelled(cancel)?;
                ensure!(
                    child.try_wait()?.is_none(),
                    "puNES Flatpak supervisor exited before controller handoff"
                );
                if self.inputs.receipt_ready()? {
                    self.verify(cancel)?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "puNES Flatpak supervisor did not prove controller routing before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        })();
        if let Err(error) = result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }
}

pub(crate) fn prepare(
    setup: &settings::SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    inventory: &[ControllerDevice],
    option: &RomEmulatorOption,
    original: &LaunchPlan,
    cancel: &AtomicBool,
) -> Result<PreparedLaunch> {
    cancelled(cancel)?;
    setup.validate()?;
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "puNES identity differs or custom environment needs resolution"
    );
    let EmulatorExecutable::Flatpak { command, app_id } = &option.executable else {
        anyhow::bail!("puNES production adapter supports only its audited Flatpak");
    };
    let runtime =
        flatpak::PreparedFlatpak::prepare(setup, option, original, command, app_id, cancel)?;
    let executable = runtime.executable().to_path_buf();
    let mut files = BTreeMap::new();
    for path in [
        &setup.content,
        &setup.probe_program,
        &original.program,
        &executable,
    ] {
        files.insert(path.clone(), file_hash(path)?);
    }
    let mut inputs =
        session::PreparedSession::prepare(setup, calibrations, inventory, runtime, cancel)?;
    let mut plan = original.clone();
    plan.arguments = inputs.stage_launch(original)?;
    let prepared = PreparedLaunch {
        inputs,
        executable,
        setup: setup.clone(),
        plan,
        files,
    };
    prepared.verify(cancel)?;
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controller_catalog::{Calibration, InputBinding, NativeInput, catalog},
        controllers::ControllerDevice,
    };
    use anyhow::Context;
    use std::{
        collections::BTreeMap,
        io::{BufRead, BufReader, Write},
        path::Path,
        process::{Child, Command, Stdio},
    };

    struct ChildGuard(Child);

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    struct OracleOutputs {
        battery: PathBuf,
        state: PathBuf,
    }

    impl Drop for OracleOutputs {
        fn drop(&mut self) {
            for path in [&self.battery, &self.state] {
                if path.is_file() {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    #[test]
    fn source_identity_is_a_full_commit() {
        assert_eq!(SOURCE_COMMIT.len(), 40);
        assert!(SOURCE_COMMIT.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    fn oracle_calibration() -> Calibration {
        let codes = BTreeMap::from([
            ("a", 0x130_u32),
            ("b", 0x131),
            ("select", 0x13a),
            ("start", 0x13b),
            ("up", 0x220),
            ("down", 0x221),
            ("left", 0x222),
            ("right", 0x223),
        ]);
        let layout = catalog().layout("nes").unwrap();
        Calibration {
            target_mappings: BTreeMap::new(),
            layout: "nes".into(),
            os: "linux".into(),
            backend: "gilrs-0.11".into(),
            bindings: layout
                .controls
                .iter()
                .map(|control| {
                    let code = codes[control.id.as_str()];
                    (
                        control.id.clone(),
                        InputBinding {
                            code,
                            kind: "button".into(),
                            direction: 0,
                            logical: control.label.clone(),
                            native: Some(NativeInput {
                                code: 0x1_0000 | code,
                                direction: 0,
                            }),
                            axis: None,
                        },
                    )
                })
                .collect(),
        }
    }

    fn pulse(
        driver: &mut ChildGuard,
        lines: &mut impl Iterator<Item = std::io::Result<String>>,
        player: usize,
        button: usize,
    ) -> Result<()> {
        writeln!(
            driver.0.stdin.as_mut().context("Pad stdin disappeared")?,
            "pulse {player} {button}"
        )?;
        ensure!(
            lines.next().context("Pad driver exited during pulse")??
                == format!("PULSED {player} {button}"),
            "Unexpected puNES pad-driver response"
        );
        Ok(())
    }

    fn exact_window(window_tool: &Path, deadline: Instant) -> Result<(String, String)> {
        loop {
            let output = Command::new(window_tool)
                .args(["search", "--all", "--limit", "2", "--name", "^puNES \\("])
                .output()?;
            ensure!(
                output.status.success()
                    || output.status.code() == Some(1)
                        && output.stdout.is_empty()
                        && output.stderr.is_empty(),
                "Could not inspect the puNES oracle window: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let windows: Vec<_> = std::str::from_utf8(&output.stdout)?
                .lines()
                .filter(|line| !line.is_empty())
                .collect();
            ensure!(windows.len() <= 1, "Found multiple puNES oracle windows");
            if let Some(window) = windows.first() {
                let name = Command::new(window_tool)
                    .args(["getwindowname", window])
                    .output()?;
                ensure!(name.status.success(), "Could not read puNES window title");
                let name = std::str::from_utf8(&name.stdout)?.trim().to_owned();
                ensure!(
                    name.starts_with("puNES (")
                        && name.ends_with(')')
                        && name.len() <= 254
                        && !name.chars().any(char::is_control),
                    "Unexpected puNES oracle window title"
                );
                return Ok(((*window).to_owned(), name));
            }
            ensure!(
                Instant::now() < deadline,
                "The puNES oracle window did not become ready"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn focus_exact_window(window_tool: &Path, window: &str, title: &str) -> Result<()> {
        ensure!(
            Command::new(window_tool)
                .args(["windowactivate", "--sync", window])
                .status()?
                .success(),
            "Could not activate exact puNES window"
        );
        ensure!(
            Command::new(window_tool)
                .args(["windowfocus", "--sync", window])
                .status()?
                .success(),
            "Could not focus exact puNES window"
        );
        let active = Command::new(window_tool).arg("getactivewindow").output()?;
        let focused = Command::new(window_tool).arg("getwindowfocus").output()?;
        let name = Command::new(window_tool)
            .args(["getwindowname", window])
            .output()?;
        ensure!(
            active.status.success()
                && focused.status.success()
                && name.status.success()
                && std::str::from_utf8(&active.stdout)?.trim() == window
                && std::str::from_utf8(&focused.stdout)?.trim() == window
                && std::str::from_utf8(&name.stdout)?.trim() == title,
            "Exact puNES window did not retain activation, focus and identity"
        );
        Ok(())
    }

    fn activate_state_menu_action(
        window_tool: &Path,
        window: &str,
        title: &str,
        action: &str,
    ) -> Result<()> {
        focus_exact_window(window_tool, window, title)?;
        let keys: &[&str] = match action {
            // Exact v0.111 menu topology: State is Alt+T, Save state is the
            // first item, and Load state is the second item. Home prevents
            // menu-selection history from affecting which QAction is used.
            "save" => &["alt+t", "Home", "Return"],
            "load" => &["alt+t", "Home", "Down", "Return"],
            _ => anyhow::bail!("Unknown puNES State-menu oracle action"),
        };
        ensure!(
            Command::new(window_tool)
                .arg("key")
                .args(["--clearmodifiers", "--delay", "100"])
                .args(keys)
                .status()?
                .success(),
            "Could not activate exact puNES State/{action} menu action"
        );
        Ok(())
    }

    fn quit_and_wait(
        window_tool: &Path,
        window: &str,
        title: &str,
        emulator: &mut ChildGuard,
    ) -> Result<()> {
        focus_exact_window(window_tool, window, title)?;
        ensure!(
            Command::new(window_tool)
                // Exact v0.111 menu topology: File is Alt+F and Quit is the
                // final item. End prevents menu-selection history from
                // changing the QAction selected here.
                .args([
                    "key",
                    "--clearmodifiers",
                    "--delay",
                    "100",
                    "alt+f",
                    "End",
                    "Return",
                ])
                .status()?
                .success(),
            "Could not activate exact puNES File/Quit menu action"
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = emulator.0.try_wait()? {
                ensure!(status.success(), "puNES oracle failed: {status}");
                return Ok(());
            }
            ensure!(Instant::now() < deadline, "puNES did not exit cleanly");
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn assert_sram(bytes: &[u8], generation: u8, expected: &[(u8, u8)]) -> Result<()> {
        ensure!(bytes.len() == 8192, "puNES oracle SRAM size differs");
        let observed_count = usize::from(bytes[5]);
        let observed_end = 0x10 + observed_count.saturating_mul(2).min(bytes.len() - 0x10);
        eprintln!(
            "puNES oracle: SRAM header={:02x?} event_bytes={:02x?}",
            &bytes[..6],
            &bytes[0x10..observed_end]
        );
        ensure!(
            &bytes[..4] == b"LBPU"
                && bytes[4] == generation
                && usize::from(bytes[5]) == expected.len(),
            "puNES oracle SRAM header differs: observed={:02x?} expected_generation={} expected_events={}",
            &bytes[..6],
            generation,
            expected.len()
        );
        let observed: Vec<_> = bytes[0x10..0x10 + expected.len() * 2]
            .chunks_exact(2)
            .map(|pair| (pair[0], pair[1]))
            .collect();
        ensure!(observed == expected, "puNES oracle input log differs");
        Ok(())
    }

    /// Opt-in exact installed-runtime proof through the ordinary production
    /// prepare/spawn path. Run under an isolated X11 server with no other puNES.
    #[test]
    #[ignore = "needs exact puNES Flatpak, /dev/uinput, X11/xdotool and generated oracle artifacts"]
    fn production_two_pad_sram_and_state_oracle() -> Result<()> {
        let rom = PathBuf::from(
            std::env::var_os("LUNCHBOX_PUNES_FLATPAK_ORACLE_ROM")
                .context("Missing puNES oracle ROM")?,
        )
        .canonicalize()?;
        let probe = PathBuf::from(
            std::env::var_os("LUNCHBOX_PUNES_FLATPAK_ORACLE_PROBE")
                .context("Missing puNES oracle probe")?,
        )
        .canonicalize()?;
        let pad_driver = PathBuf::from(
            std::env::var_os("LUNCHBOX_PUNES_FLATPAK_ORACLE_PAD_DRIVER")
                .context("Missing puNES oracle pad driver")?,
        )
        .canonicalize()?;
        let window_tool = PathBuf::from(
            std::env::var_os("LUNCHBOX_PUNES_FLATPAK_ORACLE_WINDOW_TOOL")
                .context("Missing puNES exact-window tool")?,
        )
        .canonicalize()?;
        ensure!(
            window_tool
                .file_name()
                .is_some_and(|name| name == "xdotool"),
            "puNES oracle currently proves only exact X11 window input"
        );
        let flatpak = PathBuf::from("/run/current-system/sw/bin/flatpak").canonicalize()?;
        let running = Command::new(&flatpak).arg("ps").output()?;
        ensure!(
            running.status.success()
                && !String::from_utf8_lossy(&running.stdout).contains(flatpak::APP_ID),
            "Refusing to mix the oracle with an existing puNES process"
        );
        let stem = rom
            .file_stem()
            .and_then(|stem| stem.to_str())
            .context("puNES oracle ROM has no UTF-8 stem")?;
        ensure!(
            !stem.is_empty()
                && stem
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "puNES oracle ROM needs a unique ASCII-word stem"
        );
        let profile = directories::BaseDirs::new()
            .context("Finding user home")?
            .home_dir()
            .join(".var/app")
            .join(flatpak::APP_ID);
        let source_main_config = profile.join("config/puNES/puNES.cfg");
        let source_input_config = profile.join("config/puNES/input.cfg");
        let source_hashes = [
            file_hash(&source_main_config)?,
            file_hash(&source_input_config)?,
        ];
        let outputs = OracleOutputs {
            battery: profile.join("data/puNES/prb").join(format!("{stem}.prb")),
            state: profile.join("data/puNES/save").join(format!("{stem}.p00")),
        };
        ensure!(
            !outputs.battery.try_exists()? && !outputs.state.try_exists()?,
            "puNES oracle needs a unique ROM stem without existing outputs"
        );

        let mut driver = ChildGuard(
            Command::new(pad_driver)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?,
        );
        let mut lines =
            BufReader::new(driver.0.stdout.take().context("Pad stdout disappeared")?).lines();
        let mut pads = Vec::new();
        for player in 1..=2 {
            let line = lines.next().context("Pad driver exited early")??;
            let fields: Vec<_> = line.split('\t').collect();
            ensure!(
                fields.len() == 4 && fields[0] == format!("P{player}"),
                "Unexpected puNES pad-driver identity"
            );
            pads.push((fields[1].to_owned(), fields[2].to_owned()));
        }
        ensure!(
            lines.next().context("Pad driver omitted readiness")?? == "READY",
            "puNES pad driver did not become ready"
        );

        let setup = settings::SavedSetup {
            emulator_id: "punes".into(),
            content: rom.clone(),
            source_main_config,
            source_input_config,
            probe_program: probe,
            executable_sha256: flatpak::APP_EXECUTABLE_SHA256.into(),
            players: vec![
                settings::Player {
                    player: 1,
                    controller_id: "punes-flatpak-oracle-p1".into(),
                },
                settings::Player {
                    player: 2,
                    controller_id: "punes-flatpak-oracle-p2".into(),
                },
            ],
        };
        let calibration = oracle_calibration();
        let calibrations = HashMap::from([
            (setup.players[0].controller_id.clone(), calibration.clone()),
            (setup.players[1].controller_id.clone(), calibration),
        ]);
        let inventory: Vec<_> = pads
            .iter()
            .zip(&setup.players)
            .map(|((joystick, event), player)| ControllerDevice {
                stable_id: player.controller_id.clone(),
                name: format!("puNES oracle source P{}", player.player),
                device_path: joystick.into(),
                event_paths: vec![event.into()],
                vendor_id: Some("1209".into()),
                product_id: Some(format!("4c6{}", player.player)),
                version: Some("0001".into()),
                bus_type: Some("0006".into()),
                physical_path: None,
                unique_id: None,
                is_virtual: false,
            })
            .collect();
        let option = RomEmulatorOption::standalone(
            "punes".into(),
            "puNES".into(),
            EmulatorExecutable::Flatpak {
                command: flatpak.clone(),
                app_id: flatpak::APP_ID.into(),
            },
        );
        let directory = rom
            .parent()
            .context("Oracle ROM has no parent")?
            .to_path_buf();
        let plan = LaunchPlan {
            emulator_name: "puNES".into(),
            program: flatpak.clone(),
            arguments: vec![
                "run".into(),
                format!("--filesystem={}", directory.display()).into(),
                flatpak::APP_ID.into(),
                rom.as_os_str().to_owned(),
            ],
            current_directory: directory,
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };
        let cancel = AtomicBool::new(false);
        let oracle_started = Instant::now();

        let mut first = prepare(&setup, &calibrations, &inventory, &option, &plan, &cancel)?;
        let launch = first.plan.clone();
        let mut emulator = ChildGuard(first.spawn(&launch, &cancel)?);
        let (window, title) = exact_window(&window_tool, Instant::now() + Duration::from_secs(20))?;
        std::thread::sleep(Duration::from_secs(2));
        pulse(&mut driver, &mut lines, 1, 0)?;
        activate_state_menu_action(&window_tool, &window, &title, "save")?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while !outputs.state.is_file() {
            ensure!(Instant::now() < deadline, "puNES state did not appear");
            std::thread::sleep(Duration::from_millis(25));
        }
        let state_hash = file_hash(&outputs.state)?;
        eprintln!(
            "puNES oracle: exact State/Save state created slot 0 elapsed_ms={}",
            oracle_started.elapsed().as_millis()
        );
        pulse(&mut driver, &mut lines, 2, 0)?;
        std::thread::sleep(Duration::from_millis(500));
        activate_state_menu_action(&window_tool, &window, &title, "load")?;
        eprintln!(
            "puNES oracle: exact State/Load state activated slot 0 elapsed_ms={}",
            oracle_started.elapsed().as_millis()
        );
        std::thread::sleep(Duration::from_millis(500));
        pulse(&mut driver, &mut lines, 1, 2)?;
        quit_and_wait(&window_tool, &window, &title, &mut emulator)?;
        drop(first);
        assert_sram(
            &std::fs::read(&outputs.battery)?,
            1,
            &[(0x01, 0), (0x04, 0)],
        )?;
        eprintln!(
            "puNES oracle: generation 1 proves post-save mutation was restored elapsed_ms={}",
            oracle_started.elapsed().as_millis()
        );

        let mut second = prepare(&setup, &calibrations, &inventory, &option, &plan, &cancel)?;
        let launch = second.plan.clone();
        let mut emulator = ChildGuard(second.spawn(&launch, &cancel)?);
        let (window, title) = exact_window(&window_tool, Instant::now() + Duration::from_secs(20))?;
        std::thread::sleep(Duration::from_secs(2));
        driver
            .0
            .stdin
            .as_mut()
            .context("Pad stdin disappeared")?
            .write_all(b"pulse-all\n")?;
        loop {
            if lines
                .next()
                .context("Pad driver exited during pulse-all")??
                == "PULSE-ALL-DONE"
            {
                break;
            }
        }
        quit_and_wait(&window_tool, &window, &title, &mut emulator)?;
        drop(second);
        driver
            .0
            .stdin
            .as_mut()
            .context("Pad stdin disappeared")?
            .write_all(b"quit\n")?;
        ensure!(driver.0.wait()?.success(), "puNES pad driver failed");

        let masks = [0x01_u8, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80];
        let mut expected = vec![(0x01, 0), (0x04, 0)];
        expected.extend(masks.map(|mask| (mask, 0)));
        expected.extend(masks.map(|mask| (0, mask)));
        assert_sram(&std::fs::read(&outputs.battery)?, 2, &expected)?;
        ensure!(
            file_hash(&outputs.state)? == state_hash,
            "puNES state changed without another save-state action"
        );
        ensure!(
            [
                file_hash(&setup.source_main_config)?,
                file_hash(&setup.source_input_config)?,
            ] == source_hashes,
            "puNES source configuration changed during oracle"
        );
        eprintln!(
            "puNES oracle passed: battery_sha256={} state_sha256={} generations=1,2 events={} elapsed_ms={}",
            file_hash(&outputs.battery)?,
            state_hash,
            expected.len(),
            oracle_started.elapsed().as_millis()
        );
        Ok(())
    }
}
